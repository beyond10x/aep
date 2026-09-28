//! `aep plan store migrate git`: an `aep.project/1` Markdown store rewritten as a Git-native one
//! (`aep.project/5`, git-native design § 8).
//!
//! The old store is its documents plus the store-wide `journal.jsonl`. Both are read once and the
//! whole new store is computed in memory before anything is written. A document the Git layout
//! would refuse — a `status` its last journalled move did not go to, a `revision` no higher than
//! its move count — or a journal entry about an artifact that has no document refuses the whole
//! migration, naming every such artifact, and writes nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_markdown::journal::{self, Change, Entry, Transition};
use aep_backend_markdown::{MarkdownStore, PlanningDocument, PlanningFormat};
use aep_domain::artifact::{ArtifactId, ArtifactRelation, ArtifactStatus};
use aep_domain::evidence::EvidenceKind;
use aep_domain::project::{
    ProjectVersion, StoreConfig, GIT_EVIDENCE_DIRECTORY, GIT_PLANNING_DIRECTORY, PROJECT_FILE,
    PROJECT_VERSION, PROJECT_VERSION_V5,
};
use anyhow::{Context, Result};
use clap::Args;

/// The arguments of `aep plan store migrate git`.
#[derive(Debug, Clone, Args)]
pub(crate) struct GitArgs {
    /// The project's `.engineering/` directory; discovered from the working directory when
    /// omitted.
    #[arg(long)]
    engineering: Option<PathBuf>,
    /// Print what the migration would write and refuse, and write nothing.
    #[arg(long, conflicts_with = "verify")]
    dry_run: bool,
    /// After writing, read the new store back and compare it, artifact by artifact, with what the
    /// old store answered; any difference exits non-zero.
    #[arg(long)]
    verify: bool,
    /// For a plan with no `project.yaml`: where its governing documents come from, a path or a
    /// pinned `git+…#<40-hex>` locator. Refused when a project file exists.
    #[arg(long, requires = "profile")]
    protocols: Option<String>,
    /// For a plan with no `project.yaml`: the profile whose rules apply.
    #[arg(long, requires = "protocols")]
    profile: Option<String>,
    /// For a plan with no `project.yaml`: the protocol it runs under.
    #[arg(long, default_value = "adp/1")]
    protocol: String,
}

/// The project file a plan with none is migrated from: `aep.project/1`, from the flags.
fn unconfigured_selector(args: &GitArgs, selector_path: &Path) -> Result<String> {
    let (Some(protocols), Some(profile)) = (&args.protocols, &args.profile) else {
        anyhow::bail!(
            "{} is not there; `{PROJECT_VERSION_V5}` records the protocol source and profile a \
             project file names, so pass `--protocols <source> --profile <profile>`",
            selector_path.display()
        );
    };
    let quote = |value: &str| serde_json::to_string(value).unwrap_or_default();
    Ok(format!(
        "version: {PROJECT_VERSION}\nprotocol: {}\nprofile: {}\nprotocols: {}\n",
        quote(&args.protocol),
        quote(profile),
        quote(protocols)
    ))
}

/// What the old store answered about one artifact, kept to compare the new store against.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answered {
    status: ArtifactStatus,
    revision: u64,
    title: Option<String>,
    relations: BTreeSet<ArtifactRelation>,
    body: String,
    moves: Vec<Transition>,
    evidence: BTreeMap<EvidenceKind, usize>,
}

/// One artifact, planned: the document to write and where.
struct Planned {
    relative_path: String,
    document: PlanningDocument,
}

/// The whole migration, computed before anything is written.
#[derive(Default)]
struct Plan {
    documents: BTreeMap<ArtifactId, Planned>,
    answered: BTreeMap<ArtifactId, Answered>,
    evidence: Vec<Entry>,
    transitions: usize,
    dropped: BTreeMap<&'static str, usize>,
    unreadable: usize,
    violations: Vec<String>,
}

/// The `.engineering/` directory `--engineering` names, or the discovered project's.
fn engineering_of(args: &GitArgs) -> Result<PathBuf> {
    if let Some(path) = &args.engineering {
        return Ok(path.clone());
    }
    let here = std::env::current_dir().context("reading the working directory")?;
    let directory = aep_project::project::project_directory();
    if let Some(project) = aep_project::project::discover(&here) {
        return Ok(project.join(directory));
    }
    // A plan with no project file is found where every planning verb finds it: here.
    let unconfigured = here.join(directory);
    if unconfigured.join(GIT_PLANNING_DIRECTORY).is_dir() {
        return Ok(unconfigured);
    }
    anyhow::bail!("no project found; pass `--engineering <dir>`")
}

pub(crate) fn run(args: &GitArgs) -> Result<ExitCode> {
    let engineering = engineering_of(args)?;
    let selector_path = engineering.join(PROJECT_FILE);
    let selector_text = if selector_path.is_file() {
        if args.protocols.is_some() {
            anyhow::bail!(
                "{} exists; `--protocols` and `--profile` are only for a plan with no project file",
                selector_path.display()
            );
        }
        fs::read_to_string(&selector_path)
            .with_context(|| format!("reading {}", selector_path.display()))?
    } else {
        unconfigured_selector(args, &selector_path)?
    };
    // `/2`–`/4` are refused here with the instruction that names the build which migrates them.
    let config =
        aep_schema::parse::project(&selector_text, Some(&selector_path.display().to_string()))
            .map_err(|error| anyhow::anyhow!("{error}"))?;
    if config.version == ProjectVersion::V5 {
        anyhow::bail!(
            "{} already selects `{PROJECT_VERSION_V5}`; there is nothing to migrate",
            selector_path.display()
        );
    }
    if config.store != StoreConfig::Markdown {
        anyhow::bail!(
            "{} keeps its plan in a `store:` other than markdown; only a Markdown \
             `{PROJECT_VERSION}` store migrates to `{PROJECT_VERSION_V5}`",
            selector_path.display()
        );
    }
    let project_root = engineering
        .parent()
        .context("the `.engineering` directory has no parent")?;
    let scope = crate::store_command::default_planning_scope(project_root)?;

    let _fence = crate::planning_writer_fence::PlanningWriterFence::acquire(&engineering)
        .context("holding the planning writer fence")?;
    let clean = git_clean(&engineering)?;
    if !clean.is_empty() {
        anyhow::bail!(
            "{} has uncommitted changes; the migration runs on a clean tree so Git holds the \
             store it replaces:\n{clean}",
            engineering.display()
        );
    }

    let planning = engineering.join(GIT_PLANNING_DIRECTORY);
    let evidence = engineering.join(GIT_EVIDENCE_DIRECTORY);
    if !planning.is_dir() {
        anyhow::bail!("there is no planning store at {}", planning.display());
    }
    if fs::read_dir(&evidence).is_ok_and(|mut entries| entries.next().is_some()) {
        anyhow::bail!(
            "{} is not empty; the migration writes a new evidence directory",
            evidence.display()
        );
    }

    let plan = compute(&planning);
    print_plan(&plan, &scope, args.dry_run);
    if !plan.violations.is_empty() {
        eprintln!(
            "refused: {} artifact(s) would not read back as `{}` documents; nothing was written",
            plan.violations.len(),
            PlanningFormat::V3.as_str()
        );
        return Ok(ExitCode::FAILURE);
    }
    if args.dry_run {
        outln!("dry run: nothing was written");
        return Ok(ExitCode::SUCCESS);
    }

    let selector = selector_v5(&selector_text, &scope)?;
    write(&plan, &planning, &evidence)?;
    fs::write(&selector_path, selector)
        .with_context(|| format!("writing {}", selector_path.display()))?;
    let removed = remove_all(
        [
            journal::JOURNAL,
            aep_backend_markdown::provider::PENDING_BATCH,
            aep_backend_markdown::chain::LOCK,
        ]
        .into_iter()
        .map(|name| planning.join(name)),
    )?;
    outln!(
        "{} now selects `{PROJECT_VERSION_V5}` with `planning_scope: {scope}`: {} document(s), \
         {} transition(s), {} evidence file(s) written",
        selector_path.display(),
        plan.documents.len(),
        plan.transitions,
        plan.evidence.len()
    );
    for path in &removed {
        outln!("removed {path}; Git history holds it");
    }
    if args.verify {
        return Ok(verified(&plan, &engineering, &planning, &evidence));
    }
    Ok(ExitCode::SUCCESS)
}

/// `--verify`: the new store read back and compared, with the differences printed.
fn verified(plan: &Plan, engineering: &Path, planning: &Path, evidence: &Path) -> ExitCode {
    let differences = verify(&plan.answered, planning, evidence);
    if !differences.is_empty() {
        for difference in &differences {
            outln!("  {difference}");
        }
        eprintln!(
            "verification failed: the new store differs from the old in {} place(s); \
             `git checkout -- {} && git clean -fd -- {}` restores the old store",
            differences.len(),
            engineering.display(),
            engineering.display()
        );
        return ExitCode::FAILURE;
    }
    outln!(
        "verified {} artifact(s): status, revision, title, relations, body, transitions and \
         evidence equal what the old store answered",
        plan.answered.len()
    );
    ExitCode::SUCCESS
}

/// `git status --porcelain` over the `.engineering` directory: empty when it is clean.
fn git_clean(engineering: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(engineering)
        .args(["status", "--porcelain", "--", "."])
        .output()
        .context("running git")?;
    if !output.status.success() {
        anyhow::bail!(
            "`git status` refused in {}: {}; the migration needs the store in a Git repository",
            engineering.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The journalled move an entry records, as the transition the Git layout carries.
fn transition_of(entry: &Entry) -> Option<Transition> {
    match &entry.change {
        Change::Moved {
            from,
            to,
            decided_on,
        } => Some(Transition {
            at: entry.at.clone(),
            actor: entry.actor.clone(),
            revision: entry.revision,
            from: from.clone(),
            to: to.clone(),
            decided_on: decided_on.clone(),
            imported: true,
        }),
        _ => None,
    }
}

/// Reads the old store once — its documents and its journal — and plans the new one.
fn compute(planning: &Path) -> Plan {
    let store = MarkdownStore::open(planning.to_owned()).load();
    let (entries, unreadable) = journal::read(planning);
    let mut plan = Plan {
        unreadable,
        ..Plan::default()
    };
    for failure in &store.failures {
        plan.violations.push(format!(
            "{}: the document does not read: {}",
            failure.path.display(),
            failure.detail
        ));
    }

    // Journal order is kept per artifact: it is the order the moves happened in.
    let mut by_artifact: BTreeMap<ArtifactId, Vec<Entry>> = BTreeMap::new();
    for entry in entries {
        by_artifact
            .entry(entry.artifact.clone())
            .or_default()
            .push(entry);
    }
    for id in by_artifact.keys() {
        if !store.documents.contains_key(id) {
            plan.violations.push(format!(
                "{id}: the journal records it, but the store has no document for it"
            ));
        }
    }

    for (id, stored) in &store.documents {
        let mut moves = Vec::new();
        let mut evidence = BTreeMap::new();
        for entry in by_artifact.remove(id).unwrap_or_default() {
            if let Some(transition) = transition_of(&entry) {
                moves.push(transition);
                continue;
            }
            let dropped = match &entry.change {
                Change::Evidence { kind, .. } => {
                    *evidence.entry(*kind).or_default() += 1;
                    plan.evidence.push(entry);
                    continue;
                }
                Change::Moved { .. } => unreachable!("a move is a transition"),
                Change::Created { .. } => "created",
                Change::Related { .. } => "related",
                Change::Unrelated { .. } => "unrelated",
                Change::BodyReplaced => "body_replaced",
            };
            *plan.dropped.entry(dropped).or_default() += 1;
        }
        let front = &stored.document.frontmatter;
        if let Some(last) = moves.last() {
            if last.to != front.status {
                plan.violations.push(format!(
                    "{id}: `status: {}`, but the last journalled move went to `{}`",
                    front.status, last.to
                ));
            }
        }
        let count = u64::try_from(moves.len()).unwrap_or(u64::MAX);
        if front.revision <= count {
            plan.violations.push(format!(
                "{id}: `revision: {}` is not above its {count} journalled move(s)",
                front.revision
            ));
        }
        if !front.transitions.is_empty() {
            plan.violations.push(format!(
                "{id}: the document already carries `transitions`, which an `{PROJECT_VERSION}` \
                 document does not have"
            ));
        }
        plan.answered.insert(
            id.clone(),
            Answered {
                status: front.status.clone(),
                revision: front.revision,
                title: front.title.clone(),
                relations: front.relations.iter().cloned().collect(),
                body: stored.document.body.clone(),
                moves: moves.clone(),
                evidence,
            },
        );
        plan.transitions += moves.len();
        let mut document = stored.document.clone();
        document.frontmatter.format = PlanningFormat::V3;
        document.frontmatter.transitions = moves;
        plan.documents.insert(
            id.clone(),
            Planned {
                relative_path: stored.relative_path.clone(),
                document,
            },
        );
    }
    plan
}

fn print_plan(plan: &Plan, scope: &str, dry_run: bool) {
    let verb = if dry_run { "would write" } else { "writes" };
    outln!(
        "{verb} {} document(s) as `{}` carrying {} transition(s), and {} evidence file(s); \
         `planning_scope: {scope}` (the repository directory's name)",
        plan.documents.len(),
        PlanningFormat::V3.as_str(),
        plan.transitions,
        plan.evidence.len()
    );
    let dropped: Vec<String> = plan
        .dropped
        .iter()
        .map(|(change, count)| format!("{count} {change}"))
        .collect();
    outln!(
        "not carried (Git history holds them): {}",
        if dropped.is_empty() {
            "nothing".to_owned()
        } else {
            dropped.join(", ")
        }
    );
    if plan.unreadable > 0 {
        outln!(
            "{} journal line(s) did not read and are not carried",
            plan.unreadable
        );
    }
    for violation in &plan.violations {
        outln!("  refused: {violation}");
    }
}

/// Every document through a temporary file and a rename, then every evidence record.
fn write(plan: &Plan, planning: &Path, evidence: &Path) -> Result<()> {
    for planned in plan.documents.values() {
        let path = planning.join(&planned.relative_path);
        let directory = path.parent().context("a document path has no directory")?;
        let name = path
            .file_name()
            .context("a document path has no file name")?
            .to_string_lossy();
        let temporary = directory.join(format!(".{name}.{}.migrate.tmp", std::process::id()));
        fs::write(&temporary, planned.document.render())
            .with_context(|| format!("writing {}", temporary.display()))?;
        fs::rename(&temporary, &path).with_context(|| format!("replacing {}", path.display()))?;
    }
    // Identical records are kept as separate files: each one counts as one piece of evidence.
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &plan.evidence {
        let key = serde_json::to_string(entry).context("serialising an evidence record")?;
        let occurrence = seen.entry(key).or_default();
        journal::write_evidence_occurrence(evidence, entry, *occurrence)
            .with_context(|| format!("writing evidence about {}", entry.artifact))?;
        *occurrence += 1;
    }
    Ok(())
}

/// The selector as `aep.project/5`: every key kept, `store` replaced, `planning_scope` added.
fn selector_v5(text: &str, scope: &str) -> Result<String> {
    let json = text.trim_start().starts_with('{');
    let mut value: serde_json::Value =
        serde_yaml::from_str(text).context("the project selector does not read as YAML")?;
    let map = value
        .as_object_mut()
        .context("the project selector is not a mapping")?;
    map.insert("version".to_owned(), PROJECT_VERSION_V5.into());
    map.insert("planning_scope".to_owned(), scope.into());
    map.insert("store".to_owned(), serde_json::json!({ "git": {} }));
    let mut written = if json {
        serde_json::to_string(&value)?
    } else {
        serde_yaml::to_string(&value)?
    };
    if !written.ends_with('\n') && text.ends_with('\n') {
        written.push('\n');
    }
    // The file is written only if this build reads it back as the store it just planned.
    aep_schema::parse::project(&written, None)
        .map_err(|error| anyhow::anyhow!("the rewritten project file does not read: {error}"))?;
    Ok(written)
}

/// Removes each file, answering those that were there.
fn remove_all(paths: impl Iterator<Item = PathBuf>) -> Result<Vec<String>> {
    let mut removed = Vec::new();
    for path in paths {
        match fs::remove_file(&path) {
            Ok(()) => removed.push(path.display().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("removing {}", path.display()))
            }
        }
    }
    Ok(removed)
}

/// The new store read back through the Git-native layout, compared with what the old one answered.
fn verify(
    answered: &BTreeMap<ArtifactId, Answered>,
    planning: &Path,
    evidence: &Path,
) -> Vec<String> {
    let mut differences = Vec::new();
    let report = MarkdownStore::open(planning.to_owned()).load();
    for failure in &report.failures {
        differences.push(format!("{failure}"));
    }
    if planning.join(journal::JOURNAL).exists() {
        differences.push(format!("{} is still there", journal::JOURNAL));
    }
    for id in report.documents.keys() {
        if !answered.contains_key(id) {
            differences.push(format!("{id}: in the new store only"));
        }
    }
    for (id, old) in answered {
        let Some(stored) = report.documents.get(id) else {
            differences.push(format!("{id}: missing from the new store"));
            continue;
        };
        let front = &stored.document.frontmatter;
        let (history, _) = journal::history_git(planning, evidence, id);
        let new = Answered {
            status: front.status.clone(),
            revision: front.revision,
            title: front.title.clone(),
            relations: front.relations.iter().cloned().collect(),
            body: stored.document.body.clone(),
            moves: history.iter().filter_map(transition_of).collect(),
            evidence: journal::evidence_on_hand_git(planning, evidence, id),
        };
        let mut differ = |field: &str, left: String, right: String| {
            if left != right {
                differences.push(format!("{id}: {field} was {left} and is {right}"));
            }
        };
        differ("status", old.status.to_string(), new.status.to_string());
        differ(
            "revision",
            old.revision.to_string(),
            new.revision.to_string(),
        );
        differ(
            "title",
            format!("{:?}", old.title),
            format!("{:?}", new.title),
        );
        differ(
            "relations",
            format!("{:?}", old.relations),
            format!("{:?}", new.relations),
        );
        differ("body", format!("{:?}", old.body), format!("{:?}", new.body));
        differ(
            "transitions",
            format!("{:?}", old.moves),
            format!("{:?}", new.moves),
        );
        differ(
            "evidence",
            format!("{:?}", old.evidence),
            format!("{:?}", new.evidence),
        );
        if front.format != PlanningFormat::V3 {
            differences.push(format!("{id}: written as `{}`", front.format.as_str()));
        }
    }
    differences
}

#[cfg(test)]
mod tests {
    use super::selector_v5;

    #[test]
    fn a_v1_selector_becomes_v5_keeping_every_other_key() {
        let written = selector_v5(
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\n\
             protocols: ..\nsummary: kept\n",
            "demo",
        )
        .expect("the selector rewrites");
        let value: serde_json::Value = serde_yaml::from_str(&written).expect("YAML");
        assert_eq!(value["version"], "aep.project/5");
        assert_eq!(value["planning_scope"], "demo");
        assert_eq!(value["store"], serde_json::json!({"git": {}}));
        assert_eq!(value["summary"], "kept", "an unrelated key is kept");
        assert_eq!(value["protocols"], "..");
    }

    #[test]
    fn an_explicit_markdown_store_key_is_replaced_by_git() {
        let written = selector_v5(
            "{\"version\":\"aep.project/1\",\"protocol\":\"adp/1\",\
             \"profile\":\"development.standard\",\"store\":\"markdown\"}",
            "demo",
        )
        .expect("the selector rewrites");
        assert!(written.starts_with('{'), "a JSON selector stays JSON: {written}");
        assert!(written.contains("\"store\":{\"git\":{}}"), "{written}");
    }
}
