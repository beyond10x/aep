//! `aep plan store migrate git`: an `aep.project/1` Markdown store rewritten as a Git-native one
//! (`aep.project/5`, git-native design § 8).
//!
//! The old store is its documents plus the store-wide `journal.jsonl`. This build opens no such
//! store any more; this command is the one reader of it left, through [`legacy`]. Both are read
//! once and the whole new store is computed in memory before anything is written. A document the
//! Git layout would refuse — a `status` its last journalled move did not go to, a `revision` no
//! higher than its move count — or a journal entry about an artifact that has no document refuses
//! the whole migration, naming every such artifact, and writes nothing.
//!
//! A document whose journal records no move but whose `status` is not its kind's initial state is
//! carried with one imported transition from that initial state to its status, so `validate` holds
//! it to its transitions like every other artifact rather than to the format of its first commit.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_markdown::journal::{self, Change, Entry, Transition};
use aep_backend_markdown::{MarkdownStore, PlanningDocument, PlanningFormat};
use aep_domain::artifact::{ArtifactId, ArtifactRelation, ArtifactStatus, LifecycleRegistry};
use aep_domain::evidence::EvidenceKind;
use aep_domain::project::{
    ProjectConfig, EVENT_LOG_PROJECT_VERSIONS, GIT_EVIDENCE_DIRECTORY, GIT_PLANNING_DIRECTORY,
    PROJECT_FILE, PROJECT_VERSION_V1, PROJECT_VERSION_V5,
};
use anyhow::{Context, Result};
use clap::Args;

/// The read-only reader of an `aep.project/1` store's journal: all this build keeps of that layout.
pub(crate) mod legacy {
    use std::path::Path;

    use aep_backend_markdown::journal::Entry;

    /// The store-wide log, relative to the planning directory.
    pub(crate) const JOURNAL: &str = aep_backend_markdown::journal::LEGACY_JOURNAL;
    /// The lock a `/1` writer held while appending to the journal.
    pub(crate) const LOCK: &str = "journal.lock";

    /// Every entry of the journal under `root`, oldest first, and how many lines did not read.
    ///
    /// A line is an [`Entry`] (written before 0.19.0's provider) or an `entity_core::DomainEvent`
    /// carrying one under `payload.change`; any sealing keys a line carries are ignored. A line that
    /// is neither is skipped and counted rather than fatal: a half-written line from a killed
    /// process must not make the whole history unreadable.
    pub(crate) fn read(root: &Path) -> (Vec<Entry>, usize) {
        let Ok(text) = std::fs::read_to_string(root.join(JOURNAL)) else {
            return (Vec::new(), 0);
        };
        let mut entries = Vec::new();
        let mut unreadable = 0;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            if let Ok(entry) = serde_json::from_str::<Entry>(line) {
                entries.push(entry);
                continue;
            }
            match serde_json::from_str::<entity_core::DomainEvent>(line)
                .ok()
                .and_then(|event| aep_backend_markdown::journal::entry_of(&event))
            {
                Some(entry) => entries.push(entry),
                None => unreadable += 1,
            }
        }
        (entries, unreadable)
    }
}

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
    /// The `planning_scope` to write. When omitted it is derived, in order, from the `origin`
    /// remote's repository name, the primary checkout's directory name, or, outside Git, the
    /// directory holding `.engineering/`.
    #[arg(long, value_name = "NAME")]
    planning_scope: Option<String>,
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
        "version: {PROJECT_VERSION_V1}\nprotocol: {}\nprofile: {}\nprotocols: {}\n",
        quote(&args.protocol),
        quote(profile),
        quote(protocols)
    ))
}

/// Refuses a selector this command does not migrate: anything but a Markdown `aep.project/1`.
///
/// Read as plain YAML, because the project reader of this build refuses `/1` outright.
fn require_a_v1_markdown_selector(text: &str, selector_path: &Path) -> Result<()> {
    let value: serde_json::Value =
        serde_yaml::from_str(text).context("the project selector does not read as YAML")?;
    let map = value
        .as_object()
        .context("the project selector is not a mapping")?;
    let version = match map.get("version") {
        None => PROJECT_VERSION_V1,
        Some(version) => version
            .as_str()
            .context("the project selector's `version` is not text")?,
    };
    if EVENT_LOG_PROJECT_VERSIONS.contains(&version) {
        anyhow::bail!("{}", aep_domain::project::event_log_store_refusal(version));
    }
    if version == PROJECT_VERSION_V5 {
        anyhow::bail!(
            "{} already selects `{PROJECT_VERSION_V5}`; there is nothing to migrate",
            selector_path.display()
        );
    }
    if version != PROJECT_VERSION_V1 {
        anyhow::bail!(
            "{} is `{version}`; this build migrates `{PROJECT_VERSION_V1}` to \
             `{PROJECT_VERSION_V5}`",
            selector_path.display()
        );
    }
    match map.get("store") {
        None => Ok(()),
        Some(serde_json::Value::String(word)) if word == "markdown" => Ok(()),
        Some(_) => anyhow::bail!(
            "{} keeps its plan in a `store:` other than markdown; only a Markdown \
             `{PROJECT_VERSION_V1}` store migrates to `{PROJECT_VERSION_V5}` — a SQLite or \
             Postgres project is rewritten by hand as `{PROJECT_VERSION_V5}` with a \
             `planning_scope` and `store: {{ sqlite: {{ path: <path> }} }}` or \
             `store: {{ postgres: {{ url: <url> }} }}`",
            selector_path.display()
        ),
    }
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
    /// Every evidence record about it, complete, in the order the Git-native store answers them
    /// ([`journal::history_order`]); records that order cannot tell apart keep journal order,
    /// duplicates included.
    records: Vec<Entry>,
    /// How many of those records there are of each kind: a diagnostic beside them.
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
    /// Artifacts the journal never moved that stand past their initial state, each carried with
    /// one imported transition.
    carried: usize,
    dropped: BTreeMap<&'static str, usize>,
    unreadable: usize,
    violations: Vec<String>,
}

/// Who and when an imported transition that no journalled move backs is attributed to.
struct Carrier {
    at: String,
    actor: String,
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

/// The lifecycles the migrated project's protocol tree declares, for each kind's initial state.
fn lifecycles_of(config: &ProjectConfig, engineering: &Path) -> Result<LifecycleRegistry> {
    let tree = aep_project::project::resolve_protocols(&config.protocols, engineering)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .context("resolving the project's protocol source")?;
    Ok(crate::load(&tree)
        .with_context(|| format!("loading the lifecycles under {}", tree.display()))?
        .lifecycles()
        .clone())
}

#[allow(clippy::too_many_lines)] // One ordered sequence: every refusal before the first write.
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
    require_a_v1_markdown_selector(&selector_text, &selector_path)?;
    // A relative `--engineering .engineering` has the empty path as its parent, which names no
    // directory; made absolute first, its parent is the checkout it was run in.
    let engineering_absolute = std::path::absolute(&engineering)
        .with_context(|| format!("making {} absolute", engineering.display()))?;
    let project_root = engineering_absolute
        .parent()
        .context("the `.engineering` directory has no parent")?;
    let scope =
        crate::store_command::planning_scope(project_root, args.planning_scope.as_deref())?;

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

    // The new selector is built, and read back as the store it plans, before anything is written;
    // its protocol source supplies each kind's initial state.
    let (selector, config) = selector_v5(&selector_text, &scope.value)?;
    let lifecycles = lifecycles_of(&config, &engineering)?;
    let carrier = Carrier {
        at: crate::planning::clock_at_the_edge().iso_8601(),
        actor: crate::planning::command_actor()?.to_string(),
    };

    let plan = compute(&planning, &lifecycles, &carrier);
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

    write(&plan, &planning, &evidence)?;
    fs::write(&selector_path, selector)
        .with_context(|| format!("writing {}", selector_path.display()))?;
    let removed = remove_all(
        [
            legacy::JOURNAL,
            aep_backend_markdown::provider::PENDING_BATCH,
            legacy::LOCK,
        ]
        .into_iter()
        .map(|name| planning.join(name)),
    )?;
    outln!(
        "{} now selects `{PROJECT_VERSION_V5}` with {scope}: {} document(s), {} transition(s), \
         {} evidence file(s) written",
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
        "verified {} artifact(s) and {} evidence record(s): status, revision, title, relations, \
         body, transitions and every evidence record, field by field and in order, equal what the \
         old store answered",
        plan.answered.len(),
        plan.answered
            .values()
            .map(|answered| answered.records.len())
            .sum::<usize>()
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
            ..
        } => Some(Transition {
            at: entry.at.clone(),
            actor: entry.actor.clone(),
            revision: entry.revision,
            from: from.clone(),
            to: to.clone(),
            decided_on: decided_on.clone(),
            imported: true,
            executor: None,
            correlation: None,
        }),
        _ => None,
    }
}

/// Reads the old store once — its documents and its journal — and plans the new one.
#[allow(clippy::too_many_lines)] // One pass over the old store: every rule the new one must hold.
fn compute(planning: &Path, lifecycles: &LifecycleRegistry, carrier: &Carrier) -> Plan {
    let store = MarkdownStore::open(planning.to_owned()).load();
    let (entries, unreadable) = legacy::read(planning);
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
        let mut records = Vec::new();
        let mut evidence = BTreeMap::new();
        for entry in by_artifact.remove(id).unwrap_or_default() {
            if let Some(transition) = transition_of(&entry) {
                moves.push(transition);
                continue;
            }
            let dropped = match &entry.change {
                Change::Evidence { kind, .. } => {
                    *evidence.entry(*kind).or_default() += 1;
                    records.push(entry.clone());
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
        // The files are written in journal order, which only the per-second sequence keeps; the
        // new store answers them by instant first, so a backdated record (`--at`) moves ahead.
        records.sort_by(journal::history_order);
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
                "{id}: the document already carries `transitions`, which an \
                 `{PROJECT_VERSION_V1}` document does not have"
            ));
        }
        // Never moved by the journal, and not where its kind starts: the status is carried as one
        // imported move, so the new store's own record accounts for it. A move is a write, so a
        // document still at its first revision is raised to its second.
        let mut revision = front.revision;
        if moves.is_empty() {
            if let Some(ladder) = lifecycles
                .for_kind(&front.kind)
                .filter(|ladder| ladder.initial != front.status)
            {
                revision = revision.max(2);
                moves.push(Transition {
                    at: carrier.at.clone(),
                    actor: carrier.actor.clone(),
                    revision,
                    from: ladder.initial.clone(),
                    to: front.status.clone(),
                    decided_on: journal::Provenance::default(),
                    imported: true,
                    executor: None,
                    correlation: None,
                });
                plan.carried += 1;
            }
        }
        plan.answered.insert(
            id.clone(),
            Answered {
                status: front.status.clone(),
                revision,
                title: front.title.clone(),
                relations: front.relations.iter().cloned().collect(),
                body: stored.document.body.clone(),
                moves: moves.clone(),
                records,
                evidence,
            },
        );
        plan.transitions += moves.len();
        let mut document = stored.document.clone();
        document.frontmatter.format = PlanningFormat::V3;
        document.frontmatter.revision = revision;
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

fn print_plan(plan: &Plan, scope: &crate::store_command::PlanningScope, dry_run: bool) {
    let verb = if dry_run { "would write" } else { "writes" };
    outln!(
        "{verb} {} document(s) as `{}` carrying {} transition(s), and {} evidence file(s); \
         {scope}",
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
    if plan.carried > 0 {
        outln!(
            "{} artifact(s) the journal never moved stand past their initial state; each carries one \
             imported transition to its status, at its second revision or later",
            plan.carried
        );
    }
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

/// The selector as `aep.project/5`: every key kept, `store` replaced, `planning_scope` added —
/// with the configuration this build reads it back as.
fn selector_v5(text: &str, scope: &str) -> Result<(String, ProjectConfig)> {
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
    let config = aep_schema::parse::project(&written, None)
        .map_err(|error| anyhow::anyhow!("the rewritten project file does not read: {error}"))?;
    Ok((written, config))
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
    if planning.join(legacy::JOURNAL).exists() {
        differences.push(format!("{} is still there", legacy::JOURNAL));
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
        let (files, records): (Vec<PathBuf>, Vec<Entry>) =
            journal::evidence_records_git(planning, evidence, id)
                .0
                .into_iter()
                .unzip();
        let new = Answered {
            status: front.status.clone(),
            revision: front.revision,
            title: front.title.clone(),
            relations: front.relations.iter().cloned().collect(),
            body: stored.document.body.clone(),
            moves: history.iter().filter_map(transition_of).collect(),
            records,
            evidence: journal::evidence_on_hand_git(planning, evidence, id),
        };
        // Both values, for what is vocabulary, a number or an artifact id. Free text — a title, a
        // body, an actor, a record's source or reference — is named and never quoted: a
        // difference is printed, and what a person wrote there is not for a report.
        let differ = |field: &str, left: String, right: String| {
            (left != right).then(|| format!("{id}: {field} was {left} and is {right}"))
        };
        differences.extend(differ(
            "status",
            old.status.to_string(),
            new.status.to_string(),
        ));
        differences.extend(differ(
            "revision",
            old.revision.to_string(),
            new.revision.to_string(),
        ));
        differences.extend(text_difference(
            id,
            "title",
            old.title.as_deref(),
            new.title.as_deref(),
        ));
        differences.extend(differ(
            "relations",
            format!("{:?}", old.relations),
            format!("{:?}", new.relations),
        ));
        differences.extend(text_difference(
            id,
            "body",
            Some(&old.body),
            Some(&new.body),
        ));
        differences.extend(sequence_differences(
            id,
            "transition",
            &old.moves,
            &new.moves,
            |_| None,
        ));
        differences.extend(differ(
            "evidence count by kind",
            counts(&old.evidence),
            counts(&new.evidence),
        ));
        differences.extend(sequence_differences(
            id,
            "evidence record",
            &old.records,
            &new.records,
            |position| {
                files.get(position).map(|path| {
                    path.strip_prefix(evidence)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                })
            },
        ));
        if front.format != PlanningFormat::V3 {
            differences.push(format!("{id}: written as `{}`", front.format.as_str()));
        }
    }
    differences
}

/// Per-kind evidence counts as `{test_result: 2, review_outcome: 1}`.
fn counts(counted: &BTreeMap<EvidenceKind, usize>) -> String {
    let listed: Vec<String> = counted
        .iter()
        .map(|(kind, count)| format!("{}: {count}", kind.as_str()))
        .collect();
    format!("{{{}}}", listed.join(", "))
}

/// That free-text `field` of `id` differs, with each side's length and never its text.
fn text_difference(
    id: &ArtifactId,
    field: &str,
    was: Option<&str>,
    is: Option<&str>,
) -> Option<String> {
    let size = |text: Option<&str>| {
        text.map_or_else(
            || "absent".to_owned(),
            |text| format!("{} byte(s)", text.len()),
        )
    };
    (was != is).then(|| format!("{id}: {field} differs: was {}, is {}", size(was), size(is)))
}

/// Where the new store's `what`s about `id` differ from the old store's, each list in the order
/// its store answers it, duplicates included.
///
/// Equal items are paired first, in order, so one lost or added item is named at its own position
/// rather than shifting every later one onto a neighbour. What is left is reported by 1-based
/// position: an old and a new item at the same position as differing in the fields named, any
/// other old item as missing from the new store, any other new item as the new store's only.
/// `file` names where the new item at a position was read from, when there is such a file. A
/// difference never carries a field's value.
fn sequence_differences<T: serde::Serialize + PartialEq>(
    id: &ArtifactId,
    what: &str,
    old: &[T],
    new: &[T],
    file: impl Fn(usize) -> Option<String>,
) -> Vec<String> {
    let read_from =
        |position: usize| file(position).map_or_else(String::new, |file| format!(" ({file})"));
    let (old_left, new_left) = unpaired(old, new);
    let mut new_left: BTreeSet<usize> = new_left.into_iter().collect();
    let mut differences: Vec<(usize, String)> = Vec::new();
    for position in old_left {
        let number = position + 1;
        let difference = if new_left.remove(&position) {
            format!(
                "{id}: {what} {number}{} differs in {}",
                read_from(position),
                differing_fields(&old[position], &new[position]).join(", ")
            )
        } else {
            format!(
                "{id}: {what} {number} of {} is missing from the new store",
                old.len()
            )
        };
        differences.push((position, difference));
    }
    for position in new_left {
        differences.push((
            position,
            format!(
                "{id}: {what} {}{} is in the new store only",
                position + 1,
                read_from(position)
            ),
        ));
    }
    differences.sort_by_key(|(position, _)| *position);
    differences
        .into_iter()
        .map(|(_, difference)| difference)
        .collect()
}

/// The most cells [`unpaired`] aligns in one table (16 MiB of `u32`). Between a longer old and
/// new run that share no prefix or suffix, every item is compared with the one at its position.
const ALIGNED_CELLS: usize = 1 << 22;

/// The positions in `old` and in `new` left over when equal items are paired in order: the shared
/// prefix and suffix, then the longest common subsequence of what lies between, earliest first.
fn unpaired<T: PartialEq>(old: &[T], new: &[T]) -> (Vec<usize>, Vec<usize>) {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(was, is)| was == is)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(was, is)| was == is)
        .count();
    let old_middle = &old[prefix..old.len() - suffix];
    let new_middle = &new[prefix..new.len() - suffix];
    let (rows, columns) = (old_middle.len(), new_middle.len());
    if rows.saturating_mul(columns) > ALIGNED_CELLS {
        // Each item against the one at its position: an equal pair is paired, the rest and the
        // longer side's overhang are left over.
        let differing = |position: &usize| old_middle[*position] != new_middle[*position];
        let shared = rows.min(columns);
        return (
            (0..shared)
                .filter(differing)
                .chain(shared..rows)
                .map(|position| prefix + position)
                .collect(),
            (0..shared)
                .filter(differing)
                .chain(shared..columns)
                .map(|position| prefix + position)
                .collect(),
        );
    }
    // `longest[row * width + column]`: the longest common subsequence of `old_middle[row..]` and
    // `new_middle[column..]`.
    let width = columns + 1;
    let mut longest = vec![0_u32; (rows + 1) * width];
    for row in (0..rows).rev() {
        for column in (0..columns).rev() {
            longest[row * width + column] = if old_middle[row] == new_middle[column] {
                longest[(row + 1) * width + column + 1] + 1
            } else {
                longest[(row + 1) * width + column].max(longest[row * width + column + 1])
            };
        }
    }
    let (mut old_left, mut new_left) = (Vec::new(), Vec::new());
    let (mut row, mut column) = (0, 0);
    while row < rows && column < columns {
        if old_middle[row] == new_middle[column] {
            row += 1;
            column += 1;
        } else if longest[(row + 1) * width + column] >= longest[row * width + column + 1] {
            old_left.push(prefix + row);
            row += 1;
        } else {
            new_left.push(prefix + column);
            column += 1;
        }
    }
    old_left.extend((row..rows).map(|row| prefix + row));
    new_left.extend((column..columns).map(|column| prefix + column));
    (old_left, new_left)
}

/// The fields two records differ in, as paths into their JSON (`change.reference`), sorted; a
/// field one record has and the other lacks differs.
///
/// Read from the records' JSON, so a field the type gains is compared without a list to extend.
fn differing_fields<T: serde::Serialize>(was: &T, is: &T) -> Vec<String> {
    fn leaves(
        path: &str,
        value: &serde_json::Value,
        into: &mut BTreeMap<String, serde_json::Value>,
    ) {
        match value {
            serde_json::Value::Object(map) if !map.is_empty() => {
                for (key, value) in map {
                    let path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    leaves(&path, value, into);
                }
            }
            leaf => {
                into.insert(path.to_owned(), leaf.clone());
            }
        }
    }
    let flat = |entry: &T| {
        let mut found = BTreeMap::new();
        if let Ok(value) = serde_json::to_value(entry) {
            leaves("", &value, &mut found);
        }
        found
    };
    let (was, is) = (flat(was), flat(is));
    let fields: BTreeSet<&String> = was
        .keys()
        .chain(is.keys())
        .filter(|field| was.get(*field) != is.get(*field))
        .collect();
    if fields.is_empty() {
        // Unequal records whose JSON agrees: nothing to name but the record itself.
        return vec!["the record".to_owned()];
    }
    fields.into_iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use aep_backend_markdown::journal::{Change, Entry};
    use aep_domain::artifact::{ArtifactId, ArtifactKind, LifecycleRegistry};
    use aep_domain::evidence::EvidenceKind;
    use aep_domain::project::{GIT_EVIDENCE_DIRECTORY, GIT_PLANNING_DIRECTORY};
    use aep_domain::review::ReviewOutcome;

    use super::{compute, legacy, selector_v5, verify, write, Carrier, Plan};

    // `verify` runs in the same process as the write it checks, straight after it, so no
    // integration test can change a record in between: these drive the two halves directly.

    const ACTOR: &str = "human:observer";
    const SAME_SECOND: &str = "2026-09-28T10:00:00Z";

    /// An empty directory for one test under the test binary's own target directory, which a
    /// unit test reaches without `CARGO_TARGET_TMPDIR`: `<target>/<profile>/deps/<binary>`.
    fn scratch(name: &str) -> PathBuf {
        let binary = std::env::current_exe().expect("the test binary has a path");
        let target = binary
            .ancestors()
            .nth(3)
            .expect("the test binary sits under <target>/<profile>/deps");
        let directory = target.join("tmp").join(format!("migrate-git-unit-{name}"));
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("the previous scratch store is removable");
        }
        fs::create_dir_all(&directory).expect("the scratch is writable");
        directory
    }

    fn observed() -> ArtifactId {
        ArtifactId::new("story:observed").expect("a valid id")
    }

    /// One evidence record about `story:observed`, as a `/1` recorder journalled it.
    fn record(at: &str, change: Change) -> Entry {
        Entry {
            at: at.to_owned(),
            actor: ACTOR.to_owned(),
            artifact: observed(),
            kind: ArtifactKind::Story,
            revision: 1,
            change,
        }
    }

    fn test_result(source: &str, reference: &str) -> Change {
        Change::Evidence {
            kind: EvidenceKind::TestResult,
            source: source.to_owned(),
            reference: Some(reference.to_owned()),
            review: None,
            outcome: None,
        }
    }

    /// Two distinct test results in one second, a review outcome, and one test result recorded
    /// twice, byte for byte.
    fn records() -> Vec<Entry> {
        let duplicate = record(
            "2026-09-28T10:05:00Z",
            test_result("task check", "run-dup-2020"),
        );
        vec![
            record(SAME_SECOND, test_result("nightly suite", "run-first-1717")),
            record(SAME_SECOND, test_result("nightly suite", "run-second-1718")),
            record(
                "2026-09-28T10:02:00Z",
                Change::Evidence {
                    kind: EvidenceKind::ReviewOutcome,
                    source: "review panel".to_owned(),
                    reference: None,
                    review: Some(ArtifactId::new("review:first-look").expect("a valid id")),
                    outcome: Some(ReviewOutcome::Fixed),
                },
            ),
            duplicate.clone(),
            duplicate,
        ]
    }

    /// A `/1` store holding `story:observed` and its journalled `records`, migrated as `run`
    /// does up to `--verify`: the plan, the planning directory and the evidence directory.
    fn migrated(name: &str, records: &[Entry]) -> (Plan, PathBuf, PathBuf) {
        migrated_at(name, "draft", 1, records)
    }

    /// [`migrated`], with the document at `status` and `revision` and any `journal`.
    fn migrated_at(
        name: &str,
        status: &str,
        revision: u64,
        journal: &[Entry],
    ) -> (Plan, PathBuf, PathBuf) {
        let engineering = scratch(name);
        let planning = engineering.join(GIT_PLANNING_DIRECTORY);
        let evidence = engineering.join(GIT_EVIDENCE_DIRECTORY);
        fs::create_dir_all(planning.join("story")).expect("the store is writable");
        fs::write(
            planning.join("story/observed.md"),
            format!(
                "---\nformat: aep.planning-md/1\nid: story:observed\nkind: story\n\
                 status: {status}\ntitle: observed\nrevision: {revision}\n---\n\n# observed\n"
            ),
        )
        .expect("the document is writable");
        let journal: Vec<String> = journal
            .iter()
            .map(|entry| serde_json::to_string(entry).expect("an entry serialises"))
            .collect();
        fs::write(planning.join(legacy::JOURNAL), journal.join("\n") + "\n")
            .expect("the journal is writable");
        let carrier = Carrier {
            at: "2026-10-01T00:00:00Z".to_owned(),
            actor: "human:migrator".to_owned(),
        };
        let plan = compute(&planning, &LifecycleRegistry::new(), &carrier);
        assert!(plan.violations.is_empty(), "{:?}", plan.violations);
        write(&plan, &planning, &evidence).expect("the migration writes");
        fs::remove_file(planning.join(legacy::JOURNAL)).expect("the journal is removable");
        (plan, planning, evidence)
    }

    /// The evidence files written about `story:observed` whose text holds `needle`, sorted.
    fn files_holding(evidence: &Path, needle: &str) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = fs::read_dir(evidence.join("story/observed"))
            .expect("the evidence directory lists")
            .map(|entry| entry.expect("a directory entry").path())
            .filter(|path| {
                fs::read_to_string(path).is_ok_and(|text| text.contains(needle))
            })
            .collect();
        found.sort();
        found
    }

    /// Rewrites the one file holding `from` so it holds `to` instead.
    fn alter(evidence: &Path, from: &str, to: &str) -> String {
        let found = files_holding(evidence, from);
        let [path] = found.as_slice() else {
            panic!("exactly one evidence file holds {from}: {found:?}");
        };
        let text = fs::read_to_string(path).expect("the record reads");
        fs::write(path, text.replace(from, to)).expect("the record is writable");
        path.file_name()
            .and_then(|name| name.to_str())
            .expect("a printable file name")
            .to_owned()
    }

    /// Every value a record carries that a difference must never print.
    const VALUES: &[&str] = &[
        "run-first-1717",
        "run-second-1718",
        "run-dup-2020",
        "nightly suite",
        "review panel",
        "task check",
        "review:first-look",
        ACTOR,
    ];

    fn assert_no_value(differences: &[String], altered: &[&str]) {
        for difference in differences {
            for value in VALUES.iter().chain(altered) {
                assert!(
                    !difference.contains(value),
                    "a difference printed the value {value:?}: {difference}"
                );
            }
        }
    }

    #[test]
    fn an_unmodified_migration_with_same_second_and_duplicate_records_verifies_clean() {
        let (plan, planning, evidence) = migrated("clean", &records());
        assert_eq!(
            files_holding(&evidence, "\"evidence\"").len(),
            5,
            "every record is its own file, the duplicate twice"
        );
        assert_eq!(
            verify(&plan.answered, &planning, &evidence),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_record_whose_reference_changed_is_reported_by_position_file_and_field_never_value() {
        let (plan, planning, evidence) = migrated("reference", &records());
        let file = alter(&evidence, "run-second-1718", "run-forged-9999");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            [format!(
                "story:observed: evidence record 2 (story/observed/{file}) differs in \
                 change.reference"
            )],
            "only the second record's reference changed, and the per-kind counts did not"
        );
        assert_no_value(&differences, &["run-forged-9999"]);
    }

    #[test]
    fn a_record_whose_review_outcome_changed_is_reported_by_position_file_and_field() {
        let (plan, planning, evidence) = migrated("outcome", &records());
        let file = alter(&evidence, "\"fixed\"", "\"escalated\"");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            [format!(
                "story:observed: evidence record 3 (story/observed/{file}) differs in \
                 change.outcome"
            )],
            "only the review outcome changed"
        );
        assert_no_value(&differences, &["fixed", "escalated"]);
    }

    #[test]
    fn dropping_one_of_two_identical_records_is_reported_beside_the_per_kind_counts() {
        let (plan, planning, evidence) = migrated("duplicate", &records());
        let copies = files_holding(&evidence, "run-dup-2020");
        assert_eq!(copies.len(), 2, "the duplicate was written twice");
        fs::remove_file(&copies[0]).expect("the copy is removable");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            [
                "story:observed: evidence count by kind was {test_result: 4, review_outcome: 1} \
                 and is {test_result: 3, review_outcome: 1}",
                "story:observed: evidence record 5 of 5 is missing from the new store",
            ],
            "the lost copy is named by position, and the counts stay as a diagnostic"
        );
        assert_no_value(&differences, &[]);
    }

    /// Rewrites the one evidence file holding `holding` through `edit` on its JSON, answering its
    /// file name.
    fn rewrite(evidence: &Path, holding: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let found = files_holding(evidence, holding);
        let [path] = found.as_slice() else {
            panic!("exactly one evidence file holds {holding}: {found:?}");
        };
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).expect("the record reads"))
                .expect("the record is JSON");
        edit(&mut value);
        fs::write(
            path,
            serde_json::to_string_pretty(&value).expect("the record serialises") + "\n",
        )
        .expect("the record is writable");
        path.file_name()
            .and_then(|name| name.to_str())
            .expect("a printable file name")
            .to_owned()
    }

    /// The acceptance names body text among the values a difference must never print, and
    /// `--verify` compares the body: a body that does not read back must be named, not quoted.
    #[test]
    fn a_body_difference_names_the_field_never_the_body_text() {
        let (plan, planning, evidence) = migrated("adversary-body", &records());
        let path = planning.join("story/observed.md");
        let text = fs::read_to_string(&path).expect("the migrated document reads");
        assert!(text.contains("# observed"), "{text}");
        fs::write(
            &path,
            text.replace("# observed", "# observed\n\nconfidential-body-text-4242"),
        )
        .expect("the document is writable");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert!(
            differences.iter().any(|difference| difference.contains("body")),
            "the changed body is reported: {differences:?}"
        );
        assert_no_value(&differences, &["confidential-body-text-4242"]);
    }

    /// Every field of a record is compared: a change to any one of them, or an optional field
    /// removed or added, is reported naming exactly that field and the record's position.
    #[test]
    fn a_change_to_any_one_field_of_a_record_is_reported_naming_that_field() {
        type Edit = fn(&mut serde_json::Value);
        let cases: [(&str, &str, usize, Edit, &str); 7] = [
            ("actor", "review panel", 3, |value| value["actor"] = "human:forger".into(), "actor"),
            (
                "at",
                "review panel",
                3,
                |value| value["at"] = "2026-09-28T10:02:30Z".into(),
                "at",
            ),
            ("revision", "review panel", 3, |value| value["revision"] = 7.into(), "revision"),
            (
                "source",
                "run-first-1717",
                1,
                |value| value["change"]["source"] = "forged suite".into(),
                "change.source",
            ),
            (
                "reference-removed",
                "run-second-1718",
                2,
                |value| {
                    value["change"]
                        .as_object_mut()
                        .expect("the change is an object")
                        .remove("reference");
                },
                "change.reference",
            ),
            (
                "review-added",
                "run-first-1717",
                1,
                |value| value["change"]["review"] = "review:planted".into(),
                "change.review",
            ),
            (
                "review-target",
                "review panel",
                3,
                |value| value["change"]["review"] = "review:other-look".into(),
                "change.review",
            ),
        ];
        for (name, holding, position, edit, field) in cases {
            let (plan, planning, evidence) =
                migrated(&format!("adversary-field-{name}"), &records());
            let file = rewrite(&evidence, holding, edit);
            let differences = verify(&plan.answered, &planning, &evidence);
            assert_eq!(
                differences,
                [format!(
                    "story:observed: evidence record {position} (story/observed/{file}) differs \
                     in {field}"
                )],
                "case {name}: only {field} of record {position} changed"
            );
            assert_no_value(
                &differences,
                &["human:forger", "forged suite", "review:planted", "review:other-look"],
            );
        }
    }

    /// A record the old store never answered, written into the artifact's own evidence directory
    /// after the migration, is reported as the new store's only.
    #[test]
    fn an_extra_record_in_the_new_store_is_reported() {
        let (plan, planning, evidence) = migrated("adversary-extra", &records());
        let planted = record(
            "2026-09-28T11:00:00Z",
            test_result("planted suite", "run-planted-5150"),
        );
        let path = aep_backend_markdown::journal::write_evidence(&evidence, &planted)
            .expect("the planted record writes");
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a printable file name")
            .to_owned();
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            [
                "story:observed: evidence count by kind was {test_result: 4, review_outcome: 1} \
                 and is {test_result: 5, review_outcome: 1}"
                    .to_owned(),
                format!("story:observed: evidence record 6 (story/observed/{file}) is in the new store only"),
            ],
            "the planted record is named by position and file"
        );
        assert_no_value(&differences, &["planted suite", "run-planted-5150"]);
    }

    /// A record moved into another artifact's evidence directory is no longer the observed
    /// artifact's, and its absence is reported.
    #[test]
    fn a_record_moved_to_another_artifacts_directory_is_reported_missing() {
        let (plan, planning, evidence) = migrated("adversary-moved", &records());
        let [path] = files_holding(&evidence, "review panel").try_into().expect("one file");
        let elsewhere = evidence.join("story/other");
        fs::create_dir_all(&elsewhere).expect("the other directory is writable");
        fs::rename(&path, elsewhere.join(path.file_name().expect("a file name")))
            .expect("the record moves");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert!(
            differences.contains(
                &"story:observed: evidence count by kind was {test_result: 4, review_outcome: 1} \
                  and is {test_result: 4}"
                    .to_owned()
            ),
            "the count line names the lost kind: {differences:?}"
        );
        // The lost record is the third; the two after it are intact and are not blamed.
        assert_eq!(
            differences,
            [
                "story:observed: evidence count by kind was {test_result: 4, review_outcome: 1} \
                 and is {test_result: 4}",
                "story:observed: evidence record 3 of 5 is missing from the new store",
            ],
            "the lost record is reported at its own position: {differences:?}"
        );
        assert_no_value(&differences, &[]);
    }

    /// Order is compared, not only membership: the same-second pair answered the other way round
    /// is a difference, though every record is still there.
    #[test]
    fn a_same_second_pair_answered_in_the_other_order_is_reported() {
        let (plan, planning, evidence) = migrated("reordered", &records());
        let [first] = files_holding(&evidence, "run-first-1717")
            .try_into()
            .expect("one file holds the first record");
        let name = first
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a printable file name");
        assert!(name.starts_with("20260928T100000Z-000-"), "{name}");
        let renamed = name.replace("-000-", "-002-");
        fs::rename(&first, first.with_file_name(&renamed)).expect("the record renames");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            [
                "story:observed: evidence record 1 of 5 is missing from the new store".to_owned(),
                format!(
                    "story:observed: evidence record 2 (story/observed/{renamed}) is in the new \
                     store only"
                ),
            ],
            "the first record now answers second"
        );
        assert_no_value(&differences, &[]);
    }

    /// A transition is compared field by field like a record, and its actor is never printed.
    #[test]
    fn a_transition_difference_names_its_position_and_field_never_the_actor() {
        let moved: Entry = serde_json::from_value(serde_json::json!({
            "at": "2026-09-28T09:10:00Z",
            "actor": "human:mover",
            "artifact": "story:observed",
            "kind": "story",
            "revision": 2,
            "change": {"change": "moved", "from": "draft", "to": "proposed"}
        }))
        .expect("a journalled move reads");
        let mut journal = vec![moved];
        journal.extend(records());
        let (plan, planning, evidence) = migrated_at("transition", "proposed", 2, &journal);
        let path = planning.join("story/observed.md");
        let text = fs::read_to_string(&path).expect("the migrated document reads");
        assert!(text.contains("\"human:mover\""), "{text}");
        fs::write(&path, text.replace("\"human:mover\"", "\"human:forger\""))
            .expect("the document is writable");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            ["story:observed: transition 1 differs in actor"],
            "only the move's actor changed"
        );
        assert_no_value(&differences, &["human:mover", "human:forger"]);
    }

    /// A title is free text too: named with its length, never quoted.
    #[test]
    fn a_title_difference_names_the_field_never_the_title() {
        let (plan, planning, evidence) = migrated("title", &records());
        let path = planning.join("story/observed.md");
        let text = fs::read_to_string(&path).expect("the migrated document reads");
        let title = "confidential-title-77";
        let start = text.find("\ntitle: ").expect("the document has a title line") + 1;
        let end = start + text[start..].find('\n').expect("the title line ends");
        fs::write(
            &path,
            format!("{}title: {title}{}", &text[..start], &text[end..]),
        )
        .expect("the document is writable");
        let differences = verify(&plan.answered, &planning, &evidence);
        assert_eq!(
            differences,
            [format!(
                "story:observed: title differs: was {} byte(s), is {} byte(s)",
                "observed".len(),
                title.len()
            )],
            "only the title changed"
        );
        assert_no_value(&differences, &[title]);
    }

    /// `ALIGNED_CELLS` promises that beyond the table "every item is compared with the one at its
    /// position": an equal pair at one position is not a difference there, however long the run.
    #[test]
    fn beyond_the_aligned_table_equal_records_at_their_positions_are_not_reported() {
        let inner = 2048;
        let item = |source: &str| {
            record(SAME_SECOND, test_result(source, "run-aligned-0001"))
        };
        let shared: Vec<Entry> = (0..inner)
            .map(|index| item(&format!("shared suite {index}")))
            .collect();
        let old: Vec<Entry> = std::iter::once(item("old head"))
            .chain(shared.iter().cloned())
            .chain(std::iter::once(item("old tail")))
            .collect();
        let new: Vec<Entry> = std::iter::once(item("new head"))
            .chain(shared.iter().cloned())
            .chain(std::iter::once(item("new tail")))
            .collect();
        assert!(
            old.len() * new.len() > super::ALIGNED_CELLS,
            "the run is beyond the aligned table"
        );
        let differences =
            super::sequence_differences(&observed(), "evidence record", &old, &new, |_| None);
        assert_eq!(
            differences,
            [
                "story:observed: evidence record 1 differs in change.source".to_owned(),
                format!(
                    "story:observed: evidence record {} differs in change.source",
                    inner + 2
                ),
            ],
            "only the first and last records differ; {} lines were reported",
            differences.len()
        );
    }

    /// Two records in one second at falling revisions, and a byte-identical copy of the first:
    /// the new store answers them by revision, then file sequence, and the old store's records
    /// are sorted the same way, so the faithful migration verifies clean.
    #[test]
    fn same_second_records_at_falling_revisions_verify_clean_in_revision_order() {
        let at_revision = |revision: u64, source: &str| Entry {
            revision,
            ..record(SAME_SECOND, test_result(source, "run-revision-0002"))
        };
        let journal = [
            at_revision(2, "revision two"),
            at_revision(1, "revision one"),
            at_revision(2, "revision two"),
        ];
        let (plan, planning, evidence) = migrated("adversary-falling-revisions", &journal);
        let (records, unreadable) =
            aep_backend_markdown::journal::evidence_records_git(&planning, &evidence, &observed());
        assert_eq!(unreadable, 0, "every record reads");
        let sources: Vec<String> = records
            .iter()
            .filter_map(|(_, entry)| match &entry.change {
                Change::Evidence { source, .. } => Some(source.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            sources,
            ["revision one", "revision two", "revision two"],
            "the new store answers revision first, then file sequence"
        );
        assert_eq!(
            verify(&plan.answered, &planning, &evidence),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_v1_selector_becomes_v5_keeping_every_other_key() {
        let (written, _) = selector_v5(
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
        let (written, _) = selector_v5(
            "{\"version\":\"aep.project/1\",\"protocol\":\"adp/1\",\
             \"profile\":\"development.standard\",\"store\":\"markdown\"}",
            "demo",
        )
        .expect("the selector rewrites");
        assert!(written.starts_with('{'), "a JSON selector stays JSON: {written}");
        assert!(written.contains("\"store\":{\"git\":{}}"), "{written}");
    }
}
