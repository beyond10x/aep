//! `aep plan store pack`: closed artifacts and their evidence, moved into one `aep.pack/1` file.
//!
//! The plan is computed first, from the loose files, the packs already there and what Git holds at
//! `HEAD`, and printed; `--dry-run` stops there. Only bytes Git has committed unchanged are packed
//! or removed, so every byte this verb removes is held by the new pack *and* by Git. The writes are
//! ordered so that each record is held by a loose file or a pack on disk at every point: the pack is
//! written and read back first, the project file is flipped second, the loose files go last.
//!
//! The module also holds what `unpack` shares: opening the store, the project file's version swap
//! and the atomic write.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use aep_backend_markdown::pack::{self, Overlay, Pack, Place};
use aep_backend_markdown::PlanningDocument;
use aep_domain::artifact::LifecycleRegistry;
use aep_domain::project::{
    ProjectConfig, ProjectVersion, GIT_EVIDENCE_DIRECTORY, GIT_PLANNING_DIRECTORY, PROJECT_FILE,
    PROJECT_VERSION_V5, PROJECT_VERSION_V6,
};
use anyhow::{Context, Result};
use clap::Args;

/// What `.engineering/packs/.gitattributes` holds: a pack is bytes, never line-ending converted.
pub(super) const PACK_ATTRIBUTES: &str = "*.pack -text\n";

/// The arguments of `aep plan store pack`.
#[derive(Debug, Clone, Args)]
pub(crate) struct PackArgs {
    /// The project's `.engineering/` directory; discovered from the working directory when
    /// omitted.
    #[arg(long)]
    pub(super) engineering: Option<PathBuf>,
    /// Print what would be packed, kept and removed, and write nothing.
    #[arg(long)]
    pub(super) dry_run: bool,
}

/// A Git-native store, opened.
pub(super) struct Store {
    /// The `.engineering` directory.
    pub(super) engineering: PathBuf,
    /// The loose files and the packs, read as one tree.
    pub(super) overlay: Overlay,
    /// The project's configuration.
    pub(super) config: ProjectConfig,
    selector_path: PathBuf,
    selector_text: String,
}

impl Store {
    /// Opens the store `explicit` (or the discovered project) names: an `aep.project/5` or `/6`
    /// store.
    pub(super) fn open(explicit: Option<&Path>) -> Result<Self> {
        let engineering = super::migrate_git::discover_engineering(explicit)?;
        let selector_path = engineering.join(PROJECT_FILE);
        let selector_text = fs::read_to_string(&selector_path)
            .with_context(|| format!("reading {}", selector_path.display()))?;
        let config = aep_schema::parse::project(&selector_text, None)
            .map_err(|error| anyhow::anyhow!("{}: {error}", selector_path.display()))?;
        let overlay = Overlay::new(
            engineering.join(GIT_PLANNING_DIRECTORY),
            engineering.join(GIT_EVIDENCE_DIRECTORY),
        );
        Ok(Self {
            engineering,
            overlay,
            config,
            selector_path,
            selector_text,
        })
    }

    /// The lifecycles the project's protocol tree declares.
    pub(super) fn lifecycles(&self) -> Result<LifecycleRegistry> {
        super::migrate_git::lifecycles_of(&self.config, &self.engineering)
    }

    /// The packs, every one parsed, in name order; a pack that does not parse refuses the verb.
    pub(super) fn parsed_packs(&self) -> Result<Vec<(String, std::sync::Arc<Pack>)>> {
        let mut parsed = Vec::new();
        let mut refused = Vec::new();
        for file in self.overlay.packs() {
            match file.pack() {
                Ok(pack) => parsed.push((file.name().to_owned(), std::sync::Arc::clone(pack))),
                Err(error) => refused.push(format!("{}: {error}", file.file().display())),
            }
        }
        if !refused.is_empty() {
            anyhow::bail!(
                "a pack does not read as `{}`; nothing was written:\n  {}",
                pack::FORMAT,
                refused.join("\n  ")
            );
        }
        Ok(parsed)
    }

    /// Writes the project file with its `version:` set to `to`, when it is not already.
    pub(super) fn set_version(&self, to: ProjectVersion) -> Result<bool> {
        if self.config.version == to {
            return Ok(false);
        }
        let from = self.config.version.as_str();
        let text = swap_version(&self.selector_text, from, to.as_str())?;
        write_atomic(&self.selector_path, text.as_bytes())?;
        Ok(true)
    }

    fn packs_directory(&self) -> &Path {
        self.overlay.packs_directory()
    }

    /// Files under `.engineering/{planning,evidence,packs}` that are not dot-names.
    pub(super) fn counts(&self) -> [usize; 3] {
        [
            count_files(&self.engineering.join(GIT_PLANNING_DIRECTORY)),
            count_files(&self.engineering.join(GIT_EVIDENCE_DIRECTORY)),
            count_files(self.packs_directory()),
        ]
    }
}

/// `text` with the `version:` value `from` replaced by `to`, on the line that carries it and
/// nowhere else, so the opposite swap restores the file byte for byte.
fn swap_version(text: &str, from: &str, to: &str) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut done = false;
    for line in text.split_inclusive('\n') {
        let key = line.trim_start();
        let is_version = !done
            && (key.starts_with("version:")
                || key.starts_with("\"version\":")
                || key.starts_with("{\"version\":"));
        if is_version && line.contains(from) {
            out.push_str(&line.replacen(from, to, 1));
            done = true;
        } else {
            out.push_str(line);
        }
    }
    if !done {
        anyhow::bail!("the project file has no `version: {from}` line to change");
    }
    let config = aep_schema::parse::project(&out, None)
        .map_err(|error| anyhow::anyhow!("the rewritten project file does not read: {error}"))?;
    if config.version.as_str() != to {
        anyhow::bail!("the rewritten project file does not select `{to}`");
    }
    Ok(out)
}

/// Writes `bytes` to `path` through a dot-named temporary file in the same directory.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path.parent().context("a path with no directory")?;
    fs::create_dir_all(directory).with_context(|| format!("creating {}", directory.display()))?;
    let name = path
        .file_name()
        .context("a path with no file name")?
        .to_string_lossy();
    let temporary = directory.join(format!(".{name}.{}.pack.tmp", std::process::id()));
    fs::write(&temporary, bytes).with_context(|| format!("writing {}", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

/// Writes a pack under its name and reads it back: it must parse, carry the name its bytes earn
/// and every record must match its digest. An existing file of that name is the same pack.
pub(super) fn write_pack(packs: &Path, bytes: &[u8]) -> Result<String> {
    let name = pack::name(bytes);
    let file = packs.join(pack::file_name(&name));
    if fs::read(&file).ok().as_deref() != Some(bytes) {
        write_atomic(&file, bytes)?;
    }
    let written = fs::read(&file).with_context(|| format!("reading back {}", file.display()))?;
    let parsed = Pack::parse(&written)
        .map_err(|error| anyhow::anyhow!("{} does not read back: {error}", file.display()))?;
    if written != bytes
        || parsed.name() != name
        || parsed.records().iter().any(|record| !record.digest_matches())
    {
        anyhow::bail!("{} does not read back as written", file.display());
    }
    Ok(name)
}

/// Writes `.gitattributes` into the packs directory unless it is there.
pub(super) fn ensure_attributes(packs: &Path) -> Result<()> {
    let file = packs.join(".gitattributes");
    if !file.exists() {
        write_atomic(&file, PACK_ATTRIBUTES.as_bytes())?;
    }
    Ok(())
}

/// Removes the `.gitattributes` this verb wrote, and the packs directory when that leaves it empty.
pub(super) fn ensure_attributes_removed(packs: &Path) -> Result<()> {
    let file = packs.join(".gitattributes");
    if fs::read_to_string(&file).is_ok_and(|text| text == PACK_ATTRIBUTES) {
        fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))?;
    }
    // Non-empty (a file this verb did not write) stays.
    let _ = fs::remove_dir(packs);
    Ok(())
}

/// Prints the file counts under `planning`, `evidence` and `packs`, before and after.
pub(super) fn print_counts(before: [usize; 3], after: [usize; 3]) {
    for (index, directory) in [
        GIT_PLANNING_DIRECTORY,
        GIT_EVIDENCE_DIRECTORY,
        pack::PACKS_DIRECTORY,
    ]
    .into_iter()
    .enumerate()
    {
        outln!(
            "files under .engineering/{directory}: {} -> {}",
            before[index],
            after[index]
        );
    }
}

/// Files below `directory` whose own name is not a dot-name.
fn count_files(directory: &Path) -> usize {
    let Ok(entries) = fs::read_dir(directory) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                0
            } else if path.is_dir() {
                count_files(&path)
            } else {
                1
            }
        })
        .sum()
}

/// A `git` process for the repository around `engineering`, isolated from the caller's repository
/// selection.
fn git(engineering: &Path) -> Command {
    let mut command = Command::new("git");
    for name in super::REDIRECTING_GIT_VARIABLES {
        command.env_remove(name);
    }
    command.arg("-C").arg(engineering);
    command
}

/// Whether `engineering` is inside a Git work tree, and if not why not.
fn require_work_tree(engineering: &Path) -> Result<()> {
    let output = git(engineering)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .context("running git")?;
    if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true" {
        return Ok(());
    }
    anyhow::bail!(
        "{} is not inside a Git work tree; `pack` packs only bytes Git has committed, so it needs \
         one; nothing was written",
        engineering.display()
    )
}

/// The blob at each path (relative to `engineering`) under `planning` and `evidence` in `HEAD`;
/// empty when the repository has no commit.
fn head_blobs(engineering: &Path) -> Result<BTreeMap<String, String>> {
    let has_head = git(engineering)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .context("running git")?
        .status
        .success();
    let mut found = BTreeMap::new();
    if !has_head {
        return Ok(found);
    }
    let output = git(engineering)
        .args(["ls-tree", "-r", "-z", "HEAD", "--"])
        .args([GIT_PLANNING_DIRECTORY, GIT_EVIDENCE_DIRECTORY])
        .output()
        .context("running git")?;
    if !output.status.success() {
        anyhow::bail!(
            "`git ls-tree HEAD` refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    for entry in output.stdout.split(|byte| *byte == 0) {
        let text = String::from_utf8_lossy(entry);
        if let Some((meta, path)) = text.split_once('\t') {
            let mut fields = meta.split(' ');
            if let (Some(_mode), Some("blob"), Some(oid)) =
                (fields.next(), fields.next(), fields.next())
            {
                found.insert(path.to_owned(), oid.to_owned());
            }
        }
    }
    Ok(found)
}

/// The Git blob id of each file's bytes as they are on disk, no filter applied.
fn blob_ids(engineering: &Path, paths: &[String]) -> Result<Vec<String>> {
    let mut found = Vec::with_capacity(paths.len());
    for chunk in paths.chunks(256) {
        let output = git(engineering)
            .args(["hash-object", "--no-filters", "--"])
            .args(chunk)
            .output()
            .context("running git")?;
        if !output.status.success() {
            anyhow::bail!(
                "`git hash-object` refused: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        found.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::to_owned),
        );
    }
    if found.len() != paths.len() {
        anyhow::bail!("`git hash-object` answered for {} of {} files", found.len(), paths.len());
    }
    Ok(found)
}

/// One artifact's two kinds of loose file.
#[derive(Default)]
struct Loose {
    planning: Option<String>,
    evidence: Vec<String>,
}

/// Why a loose file is not moved, or `None` when it is a regular file.
fn irregular(file: &Path) -> Option<&'static str> {
    match fs::symlink_metadata(file) {
        Ok(metadata) if metadata.file_type().is_file() => None,
        Ok(metadata) if metadata.file_type().is_symlink() => Some("a symbolic link"),
        Ok(_) => Some("not a regular file"),
        Err(_) => Some("unreadable"),
    }
}

/// Sorted entries of a directory that are not dot-names, as `(name, path)`.
fn entries(directory: &Path) -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = fs::read_dir(directory)
        .map(|listed| {
            listed
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().to_str()?.to_owned();
                    (!name.starts_with('.')).then(|| (name, entry.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// What `pack` would do.
#[derive(Default)]
struct Plan {
    /// Records the new pack holds: path to bytes.
    packed: BTreeMap<String, Vec<u8>>,
    /// Loose files removed: the packed ones and copies a pack already holds.
    removals: BTreeSet<String>,
    /// How many removals are copies a pack already holds.
    copies: usize,
    /// Candidates kept, with why.
    kept: BTreeMap<String, String>,
    /// Per kind and status: artifacts and evidence records packed.
    counts: BTreeMap<(String, String), (usize, usize)>,
    /// Kinds present whose lifecycle declares no `closed:`.
    terminal_only: BTreeSet<String>,
    /// The new pack's bytes, when there is one.
    bytes: Option<Vec<u8>>,
}

/// A loose file that is a candidate: its bytes, and whether Git holds exactly those.
struct Checked {
    bytes: Vec<u8>,
    verdict: Option<&'static str>,
}

/// Every loose planning and evidence file, grouped by artifact (kind, name). What can never move
/// (a symbolic link, a file that is not `.json` evidence) goes to `plan.kept` with the reason.
fn loose_files(engineering: &Path, plan: &mut Plan) -> BTreeMap<(String, String), Loose> {
    let mut by_artifact: BTreeMap<(String, String), Loose> = BTreeMap::new();
    let planning = engineering.join(GIT_PLANNING_DIRECTORY);
    for (kind, directory) in entries(&planning) {
        if irregular(&directory) == Some("a symbolic link") {
            plan.kept.insert(
                format!("{GIT_PLANNING_DIRECTORY}/{kind}"),
                "a symbolic link".to_owned(),
            );
            continue;
        }
        for (file, path) in entries(&directory) {
            let Some(name) = file.strip_suffix(".md") else {
                continue;
            };
            let relative = format!("{GIT_PLANNING_DIRECTORY}/{kind}/{file}");
            if let Some(why) = irregular(&path) {
                plan.kept.insert(relative, why.to_owned());
                continue;
            }
            by_artifact
                .entry((kind.clone(), name.to_owned()))
                .or_default()
                .planning = Some(relative);
        }
    }
    let evidence = engineering.join(GIT_EVIDENCE_DIRECTORY);
    for (kind, kind_directory) in entries(&evidence) {
        if irregular(&kind_directory) == Some("a symbolic link") {
            plan.kept.insert(
                format!("{GIT_EVIDENCE_DIRECTORY}/{kind}"),
                "a symbolic link".to_owned(),
            );
            continue;
        }
        for (name, directory) in entries(&kind_directory) {
            if irregular(&directory) == Some("a symbolic link") {
                plan.kept.insert(
                    format!("{GIT_EVIDENCE_DIRECTORY}/{kind}/{name}"),
                    "a symbolic link".to_owned(),
                );
                continue;
            }
            for (file, path) in entries(&directory) {
                let relative = format!("{GIT_EVIDENCE_DIRECTORY}/{kind}/{name}/{file}");
                if let Some(why) = irregular(&path) {
                    plan.kept.insert(relative, why.to_owned());
                    continue;
                }
                if Path::new(&file).extension().is_none_or(|extension| extension != "json") {
                    plan.kept
                        .insert(relative, "not a `.json` evidence file".to_owned());
                    continue;
                }
                by_artifact
                    .entry((kind.clone(), name.clone()))
                    .or_default()
                    .evidence
                    .push(relative);
            }
        }
    }
    by_artifact
}

/// A closed artifact and the loose files that could move with it.
struct Artifact {
    kind: String,
    status: String,
    planning: Option<String>,
    evidence: Vec<String>,
    packed_already: bool,
}

#[allow(clippy::too_many_lines)] // One ordered sequence: decide every artifact, then every file.
fn plan(store: &Store) -> Result<Plan> {
    let engineering = &store.engineering;
    let lifecycles = store.lifecycles()?;
    let committed = head_blobs(engineering)?;

    let mut plan = Plan::default();
    let by_artifact = loose_files(engineering, &mut plan);

    // Decide each artifact. Reading what Git holds is batched below, so first collect the loose
    // files that could move.
    let mut artifacts: Vec<Artifact> = Vec::new();
    let mut candidates: BTreeSet<String> = BTreeSet::new();
    for ((kind, name), loose) in by_artifact {
        let planning_path = format!("{GIT_PLANNING_DIRECTORY}/{kind}/{name}.md");
        let packed_already = !store.overlay.packed_in(&planning_path)?.is_empty();
        let document_bytes = match &loose.planning {
            Some(_) => fs::read(engineering.join(&planning_path))
                .with_context(|| format!("reading {planning_path}"))?,
            None if packed_already => store
                .overlay
                .read(&planning_path)
                .with_context(|| format!("reading {planning_path}"))?
                .unwrap_or_default(),
            None => continue,
        };
        let Some(document) = std::str::from_utf8(&document_bytes)
            .ok()
            .and_then(|text| PlanningDocument::parse(text, Some(&planning_path)).ok())
        else {
            if loose.planning.is_some() {
                plan.kept
                    .insert(planning_path, "does not read as a planning document".to_owned());
            }
            continue;
        };
        let (doc_kind, doc_status) = (
            document.frontmatter.kind.to_string(),
            document.frontmatter.status.to_string(),
        );
        if document.frontmatter.id.to_string() != format!("{kind}:{name}") || doc_kind != kind {
            if loose.planning.is_some() {
                plan.kept
                    .insert(planning_path, "its path does not match its id".to_owned());
            }
            continue;
        }
        let lifecycle = lifecycles.for_kind(&document.frontmatter.kind);
        if let Some(lifecycle) = lifecycle {
            if lifecycle.closed.is_empty() {
                plan.terminal_only.insert(doc_kind.clone());
            }
        }
        let closed = lifecycle.is_some_and(|lifecycle| lifecycle.is_closed(&document.frontmatter.status));
        if !closed {
            continue;
        }
        candidates.extend(loose.planning.iter().cloned());
        candidates.extend(loose.evidence.iter().cloned());
        artifacts.push(Artifact {
            kind: doc_kind,
            status: doc_status,
            planning: loose.planning,
            evidence: loose.evidence,
            packed_already,
        });
    }

    // What Git holds: one `hash-object` over every candidate, compared with `HEAD`.
    let paths: Vec<String> = candidates.into_iter().collect();
    let ids = blob_ids(engineering, &paths)?;
    let mut checked: BTreeMap<String, Checked> = BTreeMap::new();
    for (path, id) in paths.into_iter().zip(ids) {
        let verdict = match committed.get(&path) {
            None => Some("untracked"),
            Some(blob) if *blob != id => Some("differs from HEAD"),
            Some(_) => None,
        };
        let bytes = fs::read(engineering.join(&path)).with_context(|| format!("reading {path}"))?;
        checked.insert(path, Checked { bytes, verdict });
    }

    for artifact in artifacts {
        let moves = |path: &String, is_planning: bool, artifact_moves: bool, plan: &mut Plan| -> bool {
            let loose = &checked[path];
            let holders = store.overlay.holders(path).unwrap_or_default();
            let packed: Vec<&pack::Holder> = holders
                .iter()
                .filter(|holder| matches!(holder.place(), Place::Packed(_)))
                .collect();
            if let Some(why) = loose.verdict {
                plan.kept.insert(path.clone(), why.to_owned());
                return false;
            }
            if let Some(holder) = packed.first() {
                if packed.iter().any(|holder| holder.bytes() == loose.bytes) {
                    plan.removals.insert(path.clone());
                    plan.copies += 1;
                    return is_planning || artifact_moves;
                }
                if let Place::Packed(name) = holder.place() {
                    plan.kept
                        .insert(path.clone(), format!("differs from the copy packed in {name}"));
                }
                return false;
            }
            if !is_planning && !artifact_moves {
                plan.kept
                    .insert(path.clone(), "its artifact is not packed".to_owned());
                return false;
            }
            plan.packed.insert(path.clone(), loose.bytes.clone());
            plan.removals.insert(path.clone());
            let entry = plan
                .counts
                .entry((artifact.kind.clone(), artifact.status.clone()))
                .or_default();
            if is_planning {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
            true
        };
        let mut artifact_moves = artifact.packed_already;
        if let Some(path) = &artifact.planning {
            artifact_moves = moves(path, true, false, &mut plan) || artifact_moves;
        }
        for path in &artifact.evidence {
            moves(path, false, artifact_moves, &mut plan);
        }
    }

    if !plan.packed.is_empty() {
        plan.bytes = Some(
            pack::render(&plan.packed.iter().collect::<Vec<_>>())
                .map_err(|error| anyhow::anyhow!("the pack does not render: {error}"))?,
        );
    }
    Ok(plan)
}

/// The counts after the plan is carried out, for the dry run's and the run's report.
fn counts_after(before: [usize; 3], plan: &Plan) -> [usize; 3] {
    let under = |directory: &str| {
        plan.removals
            .iter()
            .filter(|path| path.starts_with(&format!("{directory}/")))
            .count()
    };
    [
        before[0] - under(GIT_PLANNING_DIRECTORY),
        before[1] - under(GIT_EVIDENCE_DIRECTORY),
        before[2] + usize::from(plan.bytes.is_some()),
    ]
}

fn print_plan(plan: &Plan, store: &Store, before: [usize; 3], after: [usize; 3]) {
    let name = plan.bytes.as_deref().map(pack::name);
    match &name {
        Some(name) => outln!(
            "pack: {name} (.engineering/{}/{})",
            pack::PACKS_DIRECTORY,
            pack::file_name(name)
        ),
        None => outln!("pack: none, no new pack would be written"),
    }
    for ((kind, status), (artifacts, evidence)) in &plan.counts {
        outln!("{kind} {status}: {artifacts} artifact(s), {evidence} evidence record(s)");
    }
    for kind in &plan.terminal_only {
        outln!("  {kind}: declares no `closed:`; packs its terminal states only");
    }
    for path in plan.packed.keys() {
        outln!("  pack {path}");
    }
    for path in &plan.removals {
        outln!("  remove {path}");
    }
    for (path, why) in &plan.kept {
        outln!("  keep {path}: {why}");
    }
    if plan.copies > 0 {
        outln!(
            "{} loose cop{} a pack already holds byte for byte",
            plan.copies,
            if plan.copies == 1 { "y" } else { "ies" }
        );
    }
    print_counts(before, after);
    if name.is_some() && store.config.version != ProjectVersion::V6 {
        outln!("project.yaml: {PROJECT_VERSION_V5} -> {PROJECT_VERSION_V6}");
    }
}

pub(crate) fn run(args: &PackArgs) -> Result<ExitCode> {
    let store = Store::open(args.engineering.as_deref())?;
    require_work_tree(&store.engineering)?;
    let _fence = if args.dry_run {
        None
    } else {
        Some(
            crate::planning_writer_fence::PlanningWriterFence::acquire(&store.engineering)
                .context("holding the planning writer fence")?,
        )
    };
    // Every pack must read before anything is decided about what a pack holds.
    store.parsed_packs()?;
    let plan = plan(&store)?;
    let before = store.counts();
    if plan.removals.is_empty() && plan.bytes.is_none() {
        for (path, why) in &plan.kept {
            outln!("  keep {path}: {why}");
        }
        outln!("nothing to pack: no closed artifact has bytes Git has committed unchanged");
        return Ok(ExitCode::SUCCESS);
    }
    let after = counts_after(before, &plan);
    print_plan(&plan, &store, before, after);
    if args.dry_run {
        outln!("dry run: nothing was written");
        return Ok(ExitCode::SUCCESS);
    }

    let packs = store.packs_directory().to_path_buf();
    if let Some(bytes) = &plan.bytes {
        fs::create_dir_all(&packs).with_context(|| format!("creating {}", packs.display()))?;
        ensure_attributes(&packs)?;
        write_pack(&packs, bytes)?;
    }
    if !store.parsed_packs()?.is_empty() {
        store.set_version(ProjectVersion::V6)?;
    }
    let mut removed = 0_usize;
    for path in &plan.removals {
        let file = store.overlay.loose_path(path);
        // A file changed since the plan was read is not the file that was packed: keep it.
        let held = plan.packed.get(path).cloned().or_else(|| {
            store
                .overlay
                .holders(path)
                .ok()
                .and_then(|holders| holders.into_iter().find(|h| matches!(h.place(), Place::Packed(_))))
                .map(|holder| holder.bytes().to_vec())
        });
        if fs::read(&file).ok() != held {
            outln!("kept {path}: it changed while the pack was written");
            continue;
        }
        fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))?;
        removed += 1;
        if let Some(directory) = file.parent() {
            if path.starts_with(&format!("{GIT_EVIDENCE_DIRECTORY}/")) {
                // An emptied evidence directory goes with its last file; a non-empty one stays.
                let _ = fs::remove_dir(directory);
            }
        }
    }
    let after = store.counts();
    match &plan.bytes {
        Some(bytes) => outln!(
            "packed {} record(s) into {}; removed {removed} loose file(s)",
            plan.packed.len(),
            pack::file_name(&pack::name(bytes))
        ),
        None => outln!("no new pack written; removed {removed} loose cop(ies) a pack already holds"),
    }
    print_counts(before, after);
    Ok(ExitCode::SUCCESS)
}
