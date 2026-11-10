//! `aep plan store unpack`: packed artifacts and their evidence, written back as the loose files
//! they were, byte for byte.
//!
//! Everything is decided before anything is written: every record path is held to the pack path
//! grammar, every target to "inside `.engineering`, through no symbolic link", every restored record
//! to its digest, and every path a merge left in two places to the rule (loose copy kept; two packs
//! with different bytes and no loose copy are refused unless `--from` chooses). The writes are
//! ordered so that each record is held by a loose file or a pack on disk at every point: restored
//! files first, rewritten packs second, the old pack files last.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use aep_backend_markdown::pack::{self, Pack};
use aep_domain::project::{ProjectVersion, GIT_EVIDENCE_DIRECTORY, GIT_PLANNING_DIRECTORY};
use anyhow::{Context, Result};
use clap::Args;

use super::pack::{
    checkout_bytes, ensure_attributes_removed, open_repo, print_counts, write_atomic, write_pack,
    Store,
};

/// The arguments of `aep plan store unpack`.
#[derive(Debug, Clone, Args)]
pub(crate) struct UnpackArgs {
    /// Artifact ids (`story:name`) to restore.
    #[arg(
        value_name = "ID",
        required_unless_present = "all",
        conflicts_with = "all"
    )]
    pub(super) ids: Vec<String>,
    /// Restore everything every pack holds.
    #[arg(long)]
    pub(super) all: bool,
    /// Where two packs hold a path with different bytes and no loose copy exists: the pack whose
    /// copy to restore.
    #[arg(long, value_name = "PACK-NAME")]
    pub(super) from: Option<String>,
    /// Print what would be restored and rewritten, and write nothing.
    #[arg(long)]
    pub(super) dry_run: bool,
    /// The project's `.engineering/` directory; discovered from the working directory when
    /// omitted.
    #[arg(long)]
    pub(super) engineering: Option<PathBuf>,
}

/// What `unpack` would do.
#[derive(Default)]
struct Plan {
    /// Loose files written: path to bytes.
    restore: BTreeMap<String, Vec<u8>>,
    /// Paths with a loose copy already, which stays.
    loose: BTreeSet<String>,
    /// Records removed from each pack, by pack name.
    drops: BTreeMap<String, BTreeSet<String>>,
}

/// A path that passes the grammar and reaches its target through no symbolic link.
fn check_target(store: &Store, path: &str) -> Result<(), String> {
    pack::check_path(path).map_err(|error| error.to_string())?;
    let mut current = store.engineering.clone();
    let segments: Vec<&str> = path.split('/').collect();
    for (index, segment) in segments.iter().enumerate() {
        current.push(segment);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "`{path}` would be written through the symbolic link {}",
                    current.display()
                ));
            }
            Ok(metadata) if index + 1 == segments.len() && metadata.is_dir() => {
                return Err(format!("`{path}` is a directory"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(format!("`{path}`: {}: {error}", current.display())),
        }
    }
    Ok(())
}

/// The paths to restore: every record for `--all`, else the artifact's file and its evidence.
fn select(args: &UnpackArgs, packs: &[(String, std::sync::Arc<Pack>)]) -> Result<BTreeSet<String>> {
    let mut selected = BTreeSet::new();
    if args.all {
        for (_, pack) in packs {
            selected.extend(pack.records().iter().map(|record| record.path().to_owned()));
        }
        return Ok(selected);
    }
    let mut refused = Vec::new();
    for id in &args.ids {
        let Some((kind, name)) = id.split_once(':').filter(|(k, n)| !k.is_empty() && !n.is_empty())
        else {
            refused.push(format!("`{id}` is not an artifact id (`<kind>:<name>`)"));
            continue;
        };
        let document = format!("{GIT_PLANNING_DIRECTORY}/{kind}/{name}.md");
        let evidence = format!("{GIT_EVIDENCE_DIRECTORY}/{kind}/{name}/");
        if let Err(error) = pack::check_path(&document) {
            refused.push(format!("`{id}`: {error}"));
            continue;
        }
        let before = selected.len();
        for (_, pack) in packs {
            selected.extend(
                pack.records()
                    .iter()
                    .map(pack::PackRecord::path)
                    .filter(|path| *path == document || path.starts_with(&evidence))
                    .map(str::to_owned),
            );
        }
        if selected.len() == before {
            refused.push(format!("`{id}` is not packed in any pack"));
        }
    }
    if !refused.is_empty() {
        anyhow::bail!("nothing was written:\n  {}", refused.join("\n  "));
    }
    Ok(selected)
}

fn plan(args: &UnpackArgs, store: &Store, packs: &[(String, std::sync::Arc<Pack>)]) -> Result<Plan> {
    if let Some(from) = &args.from {
        if !packs.iter().any(|(name, _)| name == from) {
            anyhow::bail!("`--from {from}` names no pack in .engineering/packs; nothing was written");
        }
    }
    let selected = select(args, packs)?;
    let mut plan = Plan::default();
    let mut refused: Vec<String> = Vec::new();
    for path in selected {
        if let Err(reason) = check_target(store, &path) {
            refused.push(reason);
            continue;
        }
        let holders: Vec<(&str, &pack::PackRecord)> = packs
            .iter()
            .filter_map(|(name, pack)| pack.get(&path).map(|record| (name.as_str(), record)))
            .collect();
        for (name, _) in &holders {
            plan.drops
                .entry((*name).to_owned())
                .or_default()
                .insert(path.clone());
        }
        if fs::symlink_metadata(store.overlay.loose_path(&path)).is_ok() {
            plan.loose.insert(path);
            continue;
        }
        if let Some((name, record)) = holders.iter().find(|(_, record)| !record.digest_matches()) {
            refused.push(format!(
                "`{path}` in pack {name} does not match its digest {}",
                record.sha256()
            ));
            continue;
        }
        let distinct: BTreeSet<&[u8]> = holders.iter().map(|(_, record)| record.bytes()).collect();
        let chosen = if distinct.len() == 1 {
            holders.first().map(|(_, record)| record.bytes())
        } else if let Some(from) = &args.from {
            holders
                .iter()
                .find(|(name, _)| *name == from.as_str())
                .map(|(_, record)| record.bytes())
        } else {
            None
        };
        if let Some(bytes) = chosen {
            plan.restore.insert(path, bytes.to_vec());
        } else {
            let names: Vec<&str> = holders.iter().map(|(name, _)| *name).collect();
            refused.push(format!(
                "`{path}` is held with different bytes by packs {} and no loose copy exists; \
                 choose one with `--from <pack-name>`",
                names.join(" and ")
            ));
        }
    }
    if !refused.is_empty() {
        anyhow::bail!("nothing was written:\n  {}", refused.join("\n  "));
    }
    Ok(plan)
}

/// What becomes of one affected pack: its new bytes, or `None` when nothing is left.
fn rewritten(
    packs: &[(String, std::sync::Arc<Pack>)],
    plan: &Plan,
) -> Result<BTreeMap<String, Option<Vec<u8>>>> {
    let mut found = BTreeMap::new();
    for (name, pack) in packs {
        if let Some(dropped) = plan.drops.get(name) {
            let bytes = pack::render_without(pack, dropped)
                .map_err(|error| anyhow::anyhow!("pack {name} does not render without them: {error}"))?;
            found.insert(name.clone(), bytes);
        }
    }
    Ok(found)
}

/// No pack under `.engineering/packs`: whether the last one just went (an interrupted unpack) or
/// there never was one, the store must not stay `/6`, and the packs directory goes.
fn no_pack(args: &UnpackArgs, store: &Store) -> Result<ExitCode> {
    let directory = store.overlay.packs_directory().to_path_buf();
    let stale =
        store.config.version != ProjectVersion::V5 || directory.join(".gitattributes").exists();
    if !stale {
        outln!("nothing to unpack: no pack under .engineering/packs");
    } else if args.dry_run {
        outln!("no pack under .engineering/packs; would return project.yaml to aep.project/5");
    } else {
        ensure_attributes_removed(&directory)?;
        store.set_version(ProjectVersion::V5)?;
        outln!("no pack remains; project.yaml selects aep.project/5");
    }
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn run(args: &UnpackArgs) -> Result<ExitCode> {
    let store = Store::open(args.engineering.as_deref())?;
    let repo = open_repo(&store.engineering, store.overlay.packs_directory())?;
    let _fence = if args.dry_run {
        None
    } else {
        Some(
            crate::planning_writer_fence::PlanningWriterFence::acquire(&store.engineering)
                .context("holding the planning writer fence")?,
        )
    };
    let packs = store.parsed_packs()?;
    if packs.is_empty() {
        return no_pack(args, &store);
    }
    let plan = plan(args, &store, &packs)?;
    let rewrites = rewritten(&packs, &plan)?;
    let existing: BTreeSet<&str> = packs.iter().map(|(name, _)| name.as_str()).collect();
    let mut remaining: BTreeSet<String> = existing
        .iter()
        .filter(|name| !rewrites.contains_key(**name))
        .map(|name| (*name).to_owned())
        .collect();
    remaining.extend(
        rewrites
            .values()
            .flatten()
            .map(|bytes| pack::name(bytes)),
    );

    let before = store.counts();
    let mut after = before;
    after[0] += plan.restore.keys().filter(|p| p.starts_with("planning/")).count();
    after[1] += plan.restore.keys().filter(|p| p.starts_with("evidence/")).count();
    after[2] = remaining.len();
    for path in plan.restore.keys() {
        outln!("  restore {path}");
    }
    for path in &plan.loose {
        outln!("  keep {path}: a loose copy exists");
    }
    for (name, dropped) in &plan.drops {
        for path in dropped {
            outln!("  drop {path} from {name}");
        }
    }
    for (name, bytes) in &rewrites {
        match bytes {
            Some(bytes) => outln!("  rewrite {name} -> {}", pack::name(bytes)),
            None => outln!("  delete {name}"),
        }
    }
    print_counts(before, after);
    if remaining.is_empty() && store.config.version != ProjectVersion::V5 {
        outln!("project.yaml: aep.project/6 -> aep.project/5");
    } else if !remaining.is_empty() && store.config.version != ProjectVersion::V6 {
        outln!("project.yaml: aep.project/5 -> aep.project/6");
    }
    if args.dry_run {
        outln!("dry run: nothing was written");
        return Ok(ExitCode::SUCCESS);
    }

    for (path, bytes) in &plan.restore {
        let file = store.overlay.loose_path(path);
        let written = match &repo {
            Some(repo) => checkout_bytes(repo, path, bytes)?,
            None => bytes.clone(),
        };
        write_atomic(&file, &written)?;
        if fs::read(&file).ok().as_deref() != Some(written.as_slice()) {
            anyhow::bail!("{} does not read back as written", file.display());
        }
    }
    let directory = store.overlay.packs_directory().to_path_buf();
    for bytes in rewrites.values().flatten() {
        write_pack(&directory, bytes)?;
    }
    for (name, bytes) in &rewrites {
        let new_name = bytes.as_deref().map(pack::name);
        if new_name.as_deref() == Some(name.as_str()) {
            continue;
        }
        let file = directory.join(pack::file_name(name));
        fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))?;
    }
    if remaining.is_empty() {
        ensure_attributes_removed(&directory)?;
        store.set_version(ProjectVersion::V5)?;
    } else {
        store.set_version(ProjectVersion::V6)?;
    }
    outln!(
        "restored {} file(s); rewrote {} pack(s); {} pack(s) remain",
        plan.restore.len(),
        rewrites.len(),
        remaining.len()
    );
    let after = store.counts();
    print_counts(before, after);
    Ok(ExitCode::SUCCESS)
}
