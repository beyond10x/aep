//! `aep plan store migrate git`: an event-log store (`aep.project/2`, `/3` or `/4`) rewritten as a
//! Git-native one (`aep.project/5`, git-native design § 8).
//!
//! The old store is read once, through the backend every other verb reads it through, and the whole
//! new store is computed in memory before anything is written. A document the markdown backend
//! would refuse — a `status` the last transition did not move to, a `revision` no higher than its
//! transition count — refuses the whole migration, naming every such artifact, and writes nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_markdown::journal::{Change, Entry, Transition};
use aep_backend_markdown::{MarkdownStore, PlanningDocument, PlanningFormat};
use aep_domain::artifact::{ArtifactId, ArtifactRelation, ArtifactStatus};
use aep_domain::evidence::EvidenceKind;
use anyhow::{Context, Result};
use clap::Args;

/// The projection ownership record an event-log store keeps beside its documents.
const OWNERSHIP: &str = ".aep-projection-ownership.json";
/// What a Git-native store keeps out of version control, relative to the repository.

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
    Ok(aep_project::project::discover(&here)
        .context("no project found; pass `--engineering <dir>`")?
        .join(aep_project::project::project_directory()))
}

pub(crate) fn run(args: &GitArgs) -> Result<ExitCode> {
    let engineering = engineering_of(args)?;
    let selector_path = engineering.join(aep_domain::project::PROJECT_FILE);
    let selector_text = fs::read_to_string(&selector_path)
        .with_context(|| format!("reading {}", selector_path.display()))?;
    let config = aep_schema::parse::project(&selector_text, Some(&selector_path.display().to_string()))
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    if config.version == aep_domain::project::ProjectVersion::V5 {
        anyhow::bail!(
            "{} already selects `{}`; there is no event-log store to migrate",
            selector_path.display(),
            aep_domain::project::PROJECT_VERSION_V5
        );
    }

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

    let plan_selected = crate::planning::Plan::for_project(&engineering)?;
    let crate::planning::Plan::Eventlog {
        authority_root,
        projection_root,
        blobs,
        ..
    } = plan_selected.clone()
    else {
        anyhow::bail!(
            "{} selects {}, not an event-log store; only `aep.project/2`, `/3` or `/4` migrates \
             to `{}`",
            selector_path.display(),
            plan_selected.describe(),
            aep_domain::project::PROJECT_VERSION_V5
        );
    };
    let planning = engineering.join(aep_domain::project::GIT_PLANNING_DIRECTORY);
    let evidence = engineering.join(aep_domain::project::GIT_EVIDENCE_DIRECTORY);
    if !same_path(&projection_root, &planning) {
        anyhow::bail!(
            "the projection is {}, and an `{}` store keeps its documents at {}; move the \
             projection there first",
            projection_root.display(),
            aep_domain::project::PROJECT_VERSION_V5,
            planning.display()
        );
    }
    if fs::read_dir(&evidence).is_ok_and(|mut entries| entries.next().is_some()) {
        anyhow::bail!(
            "{} is not empty; the migration writes a new evidence directory",
            evidence.display()
        );
    }

    let plan = compute(plan_selected, &projection_root)?;
    print_plan(&plan, args.dry_run);
    if !plan.violations.is_empty() {
        eprintln!(
            "refused: {} artifact(s) would not read back as `{}` documents; nothing was written",
            plan.violations.len(),
            PlanningFormat::V3.as_str()
        );
        return Ok(ExitCode::FAILURE);
    }
    if args.dry_run {
        println!("dry run: nothing was written");
        return Ok(ExitCode::SUCCESS);
    }

    write(&plan, &planning, &evidence)?;
    let selector = selector_v5(&selector_text)?;
    fs::write(&selector_path, selector)
        .with_context(|| format!("writing {}", selector_path.display()))?;
    let removed = remove_all(
        [
            Some(authority_root),
            blobs,
            Some(planning.join(OWNERSHIP)),
            Some(planning.join(aep_backend_markdown::journal::JOURNAL)),
        ]
        .into_iter()
        .flatten(),
    )?;
    println!(
        "{} now selects `{}`: {} document(s), {} transition(s), {} evidence file(s) written",
        selector_path.display(),
        aep_domain::project::PROJECT_VERSION_V5,
        plan.documents.len(),
        plan.transitions,
        plan.evidence.len()
    );
    for path in &removed {
        println!("removed {path}; Git history holds it");
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
            println!("  {difference}");
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
    println!(
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

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// Reads the old store once and plans the new one.
#[allow(clippy::too_many_lines)] // One pass maps every entry kind; it is removed with the event-log reader.
fn compute(selected: crate::planning::Plan, projection_root: &Path) -> Result<Plan> {
    let opened = crate::planning::open_plan(
        selected,
        &crate::planning::StoreLocation::at(None, None),
        true,
    )?;
    let projection = MarkdownStore::open(projection_root.to_owned()).load();
    let mut plan = Plan::default();
    for failure in &projection.failures {
        plan.violations
            .push(format!("{}: the projection does not read: {}", failure.path.display(), failure.detail));
    }
    for id in projection.documents.keys() {
        if !opened.report.documents.contains_key(id) {
            plan.violations
                .push(format!("{id}: in the projection, but the store holds no such artifact"));
        }
    }

    for (id, stored) in &opened.report.documents {
        let (entries, unreadable) = crate::planning::entries_from_the_contract(&opened, id)?;
        plan.unreadable += unreadable;
        let mut moves = Vec::new();
        let mut evidence = BTreeMap::new();
        for entry in entries {
            match &entry.change {
                Change::Moved {
                    from,
                    to,
                    decided_on,
                } => moves.push(Transition {
                    at: entry.at.clone(),
                    actor: entry.actor.clone(),
                    revision: entry.revision,
                    from: from.clone(),
                    to: to.clone(),
                    decided_on: decided_on.clone(),
                    imported: true,
                }),
                Change::Evidence { kind, .. } => {
                    *evidence.entry(*kind).or_default() += 1;
                    plan.evidence.push(entry);
                }
                Change::Created { .. } => *plan.dropped.entry("created").or_default() += 1,
                Change::Related { .. } => *plan.dropped.entry("related").or_default() += 1,
                Change::Unrelated { .. } => *plan.dropped.entry("unrelated").or_default() += 1,
                Change::BodyReplaced => *plan.dropped.entry("body_replaced").or_default() += 1,
            }
        }
        let answer = &stored.document.frontmatter;
        plan.answered.insert(
            id.clone(),
            Answered {
                status: answer.status.clone(),
                revision: answer.revision,
                title: answer.title.clone(),
                relations: answer.relations.iter().cloned().collect(),
                body: stored.document.body.clone(),
                moves: moves.clone(),
                evidence,
            },
        );

        let Some(projected) = projection.documents.get(id) else {
            plan.violations
                .push(format!("{id}: the store holds it, but the projection has no document"));
            continue;
        };
        let mut document = projected.document.clone();
        let front = &document.frontmatter;
        if front.status != answer.status || front.revision != answer.revision {
            plan.violations.push(format!(
                "{id}: the projection says `{}` at revision {} and the store `{}` at revision {}; \
                 the projection is stale",
                front.status, front.revision, answer.status, answer.revision
            ));
        }
        if let Some(last) = moves.last() {
            if last.to != front.status {
                plan.violations.push(format!(
                    "{id}: `status: {}`, but the last recorded move went to `{}`",
                    front.status, last.to
                ));
            }
        }
        let count = u64::try_from(moves.len()).unwrap_or(u64::MAX);
        if front.revision <= count {
            plan.violations.push(format!(
                "{id}: `revision: {}` is not above its {count} recorded move(s)",
                front.revision
            ));
        }
        plan.transitions += moves.len();
        document.frontmatter.format = PlanningFormat::V3;
        document.frontmatter.transitions = moves;
        plan.documents.insert(
            id.clone(),
            Planned {
                relative_path: projected.relative_path.clone(),
                document,
            },
        );
    }
    Ok(plan)
}

fn print_plan(plan: &Plan, dry_run: bool) {
    let verb = if dry_run { "would write" } else { "writes" };
    println!(
        "{verb} {} document(s) as `{}` carrying {} transition(s), and {} evidence file(s)",
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
    println!(
        "not carried (Git history holds them): {}",
        if dropped.is_empty() {
            "nothing".to_owned()
        } else {
            dropped.join(", ")
        }
    );
    if plan.unreadable > 0 {
        println!("{} history entries did not read and are not carried", plan.unreadable);
    }
    for violation in &plan.violations {
        println!("  refused: {violation}");
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
    let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for entry in &plan.evidence {
        let key = serde_json::to_string(entry).context("serialising an evidence record")?;
        let occurrence = seen.entry(key).or_default();
        aep_backend_markdown::journal::write_evidence_occurrence(evidence, entry, *occurrence)
            .with_context(|| format!("writing evidence about {}", entry.artifact))?;
        *occurrence += 1;
    }
    Ok(())
}

/// The selector as `aep.project/5`: every key kept but the Eventlog's.
fn selector_v5(text: &str) -> Result<String> {
    let json = text.trim_start().starts_with('{');
    let mut value: serde_json::Value =
        serde_yaml::from_str(text).context("the project selector does not read as YAML")?;
    let map = value
        .as_object_mut()
        .context("the project selector is not a mapping")?;
    map.remove("planning_identity");
    map.remove("planning_tenant");
    map.insert(
        "version".to_owned(),
        aep_domain::project::PROJECT_VERSION_V5.into(),
    );
    map.insert("store".to_owned(), serde_json::json!({ "git": {} }));
    let mut written = if json {
        serde_json::to_string(&value)?
    } else {
        serde_yaml::to_string(&value)?
    };
    if !written.ends_with('\n') && text.ends_with('\n') {
        written.push('\n');
    }
    Ok(written)
}


/// Removes each path, answering those that were there.
fn remove_all(paths: impl Iterator<Item = PathBuf>) -> Result<Vec<String>> {
    let mut removed = Vec::new();
    for path in paths {
        if remove(&path)? {
            removed.push(path.display().to_string());
        }
    }
    Ok(removed)
}

/// Removes a file or a directory; `false` when there was nothing there.
fn remove(path: &Path) -> Result<bool> {
    let removed = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    match removed {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("removing {}", path.display())),
    }
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
        let (history, _) = aep_backend_markdown::journal::history_git(planning, evidence, id);
        let moves: Vec<Transition> = history
            .iter()
            .filter_map(|entry| match &entry.change {
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
            })
            .collect();
        let new = Answered {
            status: front.status.clone(),
            revision: front.revision,
            title: front.title.clone(),
            relations: front.relations.iter().cloned().collect(),
            body: stored.document.body.clone(),
            moves,
            evidence: aep_backend_markdown::journal::evidence_on_hand_git(planning, evidence, id),
        };
        let mut differ = |field: &str, left: String, right: String| {
            if left != right {
                differences.push(format!("{id}: {field} was {left} and is {right}"));
            }
        };
        differ("status", old.status.to_string(), new.status.to_string());
        differ("revision", old.revision.to_string(), new.revision.to_string());
        differ("title", format!("{:?}", old.title), format!("{:?}", new.title));
        differ(
            "relations",
            format!("{:?}", old.relations),
            format!("{:?}", new.relations),
        );
        differ("body", format!("{:?}", old.body), format!("{:?}", new.body));
        differ("transitions", format!("{:?}", old.moves), format!("{:?}", new.moves));
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
