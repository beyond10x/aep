//! `protocol specification evidence` — the requirement-by-requirement verdict on a specification,
//! decided against the evidence a run has admitted.
//!
//! # The gap this closes
//!
//! `principles/development/spec-driven.yaml` owes `specification.satisfied` before completion, and
//! only a `specification` record projects that fact. Until this verb existed no step of any step map
//! could produce one, so a driven run walked every state and stopped at the guard that wanted it —
//! which is what `W4-2/1` measured, at $31.46 and 76 minutes, and what arm c of pilot 1 was refused
//! at launch for.
//!
//! `EvidenceMapping::MINTABLE` does not admit the kind and must not: the payload names *which
//! requirements are unmet*, and an exit status names none of them. So the check runs here and the
//! driver reads the document, through `EvidenceMapping::record`.
//!
//! # The design decision: what counts as a requirement in a markdown artifact
//!
//! A specification artifact is `aep.planning-md/1` — validated frontmatter and a body somebody
//! wrote. Nothing in that format marks a requirement, so this verb defines it, and the definition
//! has to be one a person can satisfy on purpose and cannot satisfy by accident:
//!
//! * **A requirement is a list item under a `Requirements` or an `Acceptance` heading.** Any
//!   heading depth, matched without regard to case. Prose outside those sections is context; a
//!   specification whose requirements are only in prose states none, which this verb reports rather
//!   than guesses at.
//! * **A requirement is satisfied when the predicate it names is `True`.** The predicate is written
//!   in the item as an inline code span, in the same language every profile and workflow guard is
//!   written in — `` `tests.unit.failed == 0` `` — and it is parsed by the same parser and
//!   evaluated against the facts the run's evidence projects.
//! * **A requirement that names no predicate is not satisfied**, and is reported saying so. This is
//!   the decision that makes the whole thing worth having: the alternative — treating an
//!   unmeasurable requirement as met — would let a specification discharge `spec-driven` by being
//!   vague, and vagueness is what a specification exists to remove.
//! * **`False` and `Unknown` both fail to satisfy**, and they are reported differently. Invariant 5:
//!   nobody looked is not the same finding as it is broken, and only one of them is fixed by
//!   changing code.
//!
//! Considered and refused: a ticked task-list item (`- [x]`). It reads well and is worthless — the
//! party that writes the specification is the party being checked, so a record built from ticks is
//! an agent marking its own homework wearing `producer: verifier`.
//!
//! # Which specification, and why the verb refuses rather than chooses
//!
//! The one the run is being held to, decided by **the guard's own rule**: an approved
//! `specification` artifact whose `specifies` edge lands on the work this task declares —
//! `spec-driven.before_implementation`'s `{kind: specification, status: approved, relation: {kind:
//! specifies, target: task}}`, evaluated here by [`ArtifactRequirement::matches`] over
//! [`Task::declared_work`], which is the same function and the same set the engine answers
//! `RequirementContext::task_artifacts` with.
//!
//! That it is the same rule is the point. Until `story:task-scoped-artifact-requirements`' follow-up
//! this verb took *any* in-force specification in the store, which after the guard was bound was
//! **looser than the guard it serves**: a run could write a `specification` record about a document
//! `before_implementation` would refuse, and the record's `satisfied` would then be about somebody
//! else's story.
//!
//! With none, or with more than one, this verb **writes nothing and says why** — a step establishes
//! one thing, and picking one of several would be the tool deciding what the run is about. The
//! driver reads that as D5's `Unknown`: nothing observed, and the run stops at the guard instead of
//! moving on a record about the wrong document.
//!
//! **`--artifact` names which specification, never whether the binding applies.** An id given on
//! the command line that does not specify this task's work is refused, saying so: an escape hatch
//! here would be a way to mint the record the guard exists to withhold. What it does lift is the
//! *status* half — a person may legitimately ask this verb whether a `draft` specification is
//! written so that anything could ever decide it.
//!
//! **The task is known from `--task <file>`, or from the project this was run inside** (the task
//! `project.yaml` names), and from nothing else. With neither — a store handed to the verb from
//! outside a project — the selection is unbound and falls back to what it always did: the store's
//! one in-force specification, and a refusal naming them all when there is more than one.
//!
//! # What this cannot see, stated rather than implied
//!
//! The facts come from the **evidence records in the snapshot**, and from nothing else. The engine
//! derives more than that — `evidence.count.*`, `evidence.first_seq.*`, `test.first_result`, the
//! horizon decay that withholds a lapsed record's facts — and none of it is reproduced here,
//! because a second implementation of fact derivation is how a fact and a requirement come to
//! disagree. A requirement naming a derived fact therefore reads `Unknown` and is reported unmet.
//! That is the fail-closed direction: this verb can call a met requirement unmet, and cannot call an
//! unmet one met.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_markdown::document::PlanningDocument;
use aep_domain::artifact::{
    Artifact, ArtifactGraph, ArtifactKind, ArtifactRef, ArtifactStatus, RelationKind,
};
use aep_domain::evidence::{Evidence, SpecificationRecord};
use aep_domain::facts::FactStore;
use aep_domain::predicate::{Predicate, Truth};
use aep_domain::requirement::{ArtifactRequirement, RelationRequirement, RelationTarget};
use aep_domain::task::Task;
use aep_domain::verification::Verifier;
use aep_engine::Snapshot;
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::evidence_doc::{emit, MintedEvidence};
use crate::Format;

/// The verifier class this verb signs as.
///
/// An external tool named `protocol`, the same spelling `drivers/development/checks.yaml` already
/// uses for this binary's own verbs. Not `artifact-validator`, however well that describes the
/// method: `EvidenceKind::Specification::default_verifiers` names `test-runner` and `human-review`,
/// and `StepMap::check_run` refuses a **named** verifier the protocol does not join to the kind, so
/// a map declaring `artifact-validator` for a `specification` does not load. An external tool is
/// admitted and checked at run start instead, which is the right shape here: what produced this is
/// a tool, and naming it is more use to a later reader than borrowing a class that does not fit.
///
/// `spec-driven` pins no verifier for the kind, so this is a statement about method rather than a
/// requirement — and it is a function of no arguments, not a caller's choice, because a caller that
/// could name itself the verifier would make the record's independence an input to the record.
fn producer() -> Verifier {
    Verifier::ExternalTool(
        aep_domain::ids::ToolRef::new("protocol").expect("`protocol` is a tool reference"),
    )
}

/// The headings whose list items are requirements, matched without regard to case.
const REQUIREMENT_HEADINGS: [&str; 2] = ["requirements", "acceptance"];

/// What can be done with a specification artifact.
#[derive(Debug, Subcommand)]
pub(crate) enum SpecificationCommand {
    /// Decide every requirement against a run's evidence and write the `specification` document.
    ///
    /// Exits `0` whatever the verdict. A specification with unmet requirements is exactly the case
    /// the record exists for: `SpecificationRecord::unsatisfied` names them, and the engine is what
    /// decides on it.
    Evidence(EvidenceArgs),
}

/// The arguments of `protocol specification evidence`.
#[derive(Debug, Args)]
pub(crate) struct EvidenceArgs {
    /// The planning store, as a markdown directory.
    #[arg(long, default_value = ".engineering/planning")]
    store: PathBuf,
    /// The run's snapshot — `snapshot.json` in its run directory — holding the evidence it admitted.
    ///
    /// Without it every requirement reads `Unknown`, which is a legitimate thing to ask for: it
    /// answers *is this specification written so that anything could ever decide it?* before a run
    /// has produced a single record.
    #[arg(long)]
    snapshot: Option<PathBuf>,
    /// Which specification to decide. Selected from the store by the guard's own rule when omitted.
    ///
    /// It names *which*, not *whether*: an id that does not specify the task's work is refused —
    /// see [`select`]. What it does lift is the status requirement, so a `draft` specification can
    /// be asked whether it states anything decidable.
    #[arg(long, value_name = "ID")]
    artifact: Option<String>,
    /// The task document whose declared work the selection is bound to.
    ///
    /// Discovered from the project when omitted — the task `project.yaml` names. It is what makes
    /// the selection mean *this run's specification*: without a task in reach the verb falls back
    /// to the store's one in-force specification, which is looser than the guard this record
    /// serves. See [`select`].
    #[arg(long)]
    task: Option<PathBuf>,
    /// Where to write the document. Without it, it goes to standard output.
    #[arg(long)]
    out: Option<PathBuf>,
    /// How to write it. Both are read by `protocol evaluate --evidence`.
    #[arg(long, value_enum, default_value_t = Format::Yaml)]
    format: Format,
}

/// The `specification` verb family, one arm per subcommand.
pub(crate) fn run(command: SpecificationCommand) -> Result<ExitCode> {
    match command {
        SpecificationCommand::Evidence(args) => mint_evidence(&args),
    }
}

/// **The rule this verb selects by**, as a value: the artifact requirement
/// `principles/development/spec-driven.yaml`'s `before_implementation` states, which
/// `principles/development/clean-room.yaml` restates word for word.
///
/// A value rather than a description, because the selection is then made by
/// [`ArtifactRequirement::matches`] — the engine's own function, over
/// [`Task::declared_work`] — and there is no second reading of *whose specification is this* to
/// drift from the first. `status: approved` accepts `accepted`, `active` and `implemented` too
/// (`ArtifactStatus::satisfies`), so a specification already in force does not have to be
/// re-approved for each task that implements it, and `fresh` refuses a superseded, rejected or
/// archived one.
///
/// The two shipped declarations are pinned to this value by
/// `crates/protocol-cli/tests/specification_task_binding.rs`, which parses them and compares: an
/// edit to either principle that this verb has not followed fails a test rather than quietly
/// making the verb looser than the guard again.
fn specification_of_this_task() -> ArtifactRequirement {
    ArtifactRequirement {
        kind: ArtifactKind::Specification,
        status: Some(ArtifactStatus::Approved),
        at_least: 1,
        relation: Some(RelationRequirement {
            kind: RelationKind::Specifies,
            target_kind: None,
            target: Some(RelationTarget::Task),
        }),
        fresh: true,
    }
}

/// The rule with its **relation** dropped: what the verb may still ask with no task in reach.
///
/// Not a weaker binding — no binding at all, and it is reached only when nothing named a task.
/// `matches` with an empty work set refuses every artifact, which is the engine's fail-closed
/// polarity and the wrong answer for a person pointing this verb at a store from outside a project.
fn unbound(rule: &ArtifactRequirement) -> ArtifactRequirement {
    ArtifactRequirement {
        relation: None,
        ..rule.clone()
    }
}

/// The rule with its **status** dropped: what an id given as `--artifact` is checked against.
///
/// The binding half is kept, deliberately: naming a document does not make it this task's. The
/// status half is lifted because asking whether a `draft` specification states requirements
/// anything could ever decide is a legitimate question, and the answer is a record about a
/// specification the caller named rather than one this verb chose.
fn named(rule: &ArtifactRequirement) -> ArtifactRequirement {
    ArtifactRequirement {
        status: None,
        ..rule.clone()
    }
}

/// Every specification artifact in the store, with the file it came from.
///
/// Reads the markdown directly through the backend's own parser rather than through the artifact
/// graph, because what this verb needs is the **body** — the graph carries frontmatter and edges,
/// and the requirements are in the prose.
fn specifications(store: &Path) -> Result<Vec<(PathBuf, PlanningDocument)>> {
    let directory = store.join(ArtifactKind::Specification.as_str());
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => bail!(
            "no specification artifacts at {}: {error}. `spec-driven` asks for one before \
             implementation, so a run with none has nothing to be decided against",
            directory.display()
        ),
    };

    // Collected and sorted rather than taken in directory order: two machines must read the same
    // store the same way, and `read_dir` promises no order at all (invariant 9).
    let mut paths: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    paths.sort();

    let mut found = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let document = PlanningDocument::parse(&text, Some(&path.display().to_string()))
            .with_context(|| format!("{} is not a planning document", path.display()))?;
        if document.frontmatter.kind == ArtifactKind::Specification {
            found.push((path, document));
        }
    }
    Ok(found)
}

/// What the verb knows about the task, and where it learned it.
///
/// Absent means *no task document is in reach* — not *a task that declares nothing*, which is a
/// different finding with a different refusal.
struct Binding {
    /// The task document, as the refusals name it.
    ///
    /// Printed because the wrong task is the failure a reader cannot otherwise see: a run driven
    /// with `--task .engineering/task-native-1.yaml` reaches this verb through the project, which
    /// names `.engineering/task.yaml`, and the refusal is only actionable if it says which one it
    /// bound to.
    source: String,
    /// The work the task declares — [`Task::declared_work`], the same set the engine answers
    /// `RequirementContext::task_artifacts` with, and the same call.
    work: Vec<ArtifactRef>,
}

impl Binding {
    /// What this task is about, in the words the refusals use.
    ///
    /// The clause `ArtifactRequirement::evaluate` puts on an unmet bound row, so a reader meets
    /// one sentence whether the guard refused or the verb did.
    fn work_line(&self) -> String {
        if self.work.is_empty() {
            "the task declares no work".to_owned()
        } else {
            format!(
                "this task's work is {}",
                self.work
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// The task the verb is bound to: the one `--task` names, or the one the project does.
///
/// A named path that cannot be read is a refusal — a caller who said which task must not be
/// answered as though they had not. A *discovery* that finds nothing is not: this verb is also run
/// by hand against a store outside any project, and the unbound path below is what answers that.
fn binding(task: Option<&Path>) -> Result<Option<Binding>> {
    if let Some(path) = task {
        let task = crate::read_task(path)?;
        return Ok(Some(Binding {
            source: path.display().to_string(),
            work: task.declared_work(),
        }));
    }
    Ok(discovered())
}

/// The task the project names, when this was run inside one.
fn discovered() -> Option<Binding> {
    let here = std::env::current_dir().ok()?;
    let root = aep_project::project::discover(&here)?;
    let project = aep_project::project::load(&root).ok()?;
    let task: Task = project.task?;
    Some(Binding {
        source: project.paths.task.display().to_string(),
        work: task.declared_work(),
    })
}

/// The store's specifications as the artifacts a requirement is evaluated against.
///
/// The graph holds only these, which is enough for this rule and honest about it: the relation
/// declares no `target_kind`, so nothing here has to resolve the far end of an edge — the binding
/// compares ids, exactly as `ArtifactRequirement::matches` does for the engine.
fn described(found: &[(PathBuf, PlanningDocument)]) -> (Vec<Artifact>, ArtifactGraph) {
    let artifacts: Vec<Artifact> = found
        .iter()
        .map(|(path, document)| {
            document
                .frontmatter
                .to_artifact(&path.display().to_string())
        })
        .collect();
    let mut graph = ArtifactGraph::new();
    for artifact in artifacts.iter().cloned() {
        graph.insert(artifact);
    }
    (artifacts, graph)
}

/// Every specification in the store, with its status, for a refusal that names what was there.
fn declared(artifacts: &[Artifact]) -> String {
    artifacts
        .iter()
        .map(|artifact| format!("{} ({})", artifact.id, artifact.status))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The one specification this run is being held to, or a refusal naming what it found instead.
fn select(store: &Path, wanted: Option<&str>, task: Option<&Path>) -> Result<PlanningDocument> {
    let mut found = specifications(store)?;
    let (artifacts, graph) = described(&found);
    let binding = binding(task)?;
    let chosen = choose(store, &artifacts, &graph, binding.as_ref(), wanted)?;
    Ok(found.swap_remove(chosen).1)
}

/// **The selection, decided** — everything `select` does that is not reading a directory.
///
/// Separate so the rule can be tested at every boundary it has without a store on disk, and so
/// that what decides which document a record is about is one function of its arguments.
fn choose(
    store: &Path,
    artifacts: &[Artifact],
    graph: &ArtifactGraph,
    binding: Option<&Binding>,
    wanted: Option<&str>,
) -> Result<usize> {
    let rule = specification_of_this_task();

    if let Some(id) = wanted {
        let Some(index) = artifacts
            .iter()
            .position(|artifact| artifact.id.to_string() == id)
        else {
            bail!(
                "no specification artifact `{id}` in {}. `--artifact` names one exactly; without \
                 it the specification of this task's work is used",
                store.display()
            );
        };
        if let Some(binding) = binding {
            // `--artifact` says *which* document to decide, never *whether* the binding applies.
            // A record about a specification `spec-driven.before_implementation` would refuse is a
            // record about the wrong work, and a flag that produced one would be the way around
            // the guard rather than the way through it.
            if !named(&rule).matches(&artifacts[index], graph, &binding.work) {
                bail!(
                    "`{id}` is not a specification which {}, so a record about it would be a \
                     record about work nobody said this run was about. {} (from {}). \
                     `--artifact` names which specification to decide and does not lift the \
                     binding; point `--task` at the task this specification is about, or give it \
                     the edge — `protocol artifact relate {id} specifies <the artifact the task \
                     is derived from>`",
                    rule.relation.as_ref().expect("the rule binds"),
                    binding.work_line(),
                    binding.source
                );
            }
        }
        return Ok(index);
    }

    let work: &[ArtifactRef] = binding.map_or(&[], |binding| &binding.work);
    let requirement = if binding.is_some() {
        rule
    } else {
        unbound(&rule)
    };
    let chosen: Vec<usize> = (0..artifacts.len())
        .filter(|index| requirement.matches(&artifacts[*index], graph, work))
        .collect();

    match chosen.len() {
        1 => Ok(chosen[0]),
        0 => bail!("{}", nothing_selected(store, artifacts, binding)),
        held => bail!(
            "{held} specifications in {} {} — {} — so a step here would establish something about \
             one of several documents. {} `--artifact` names one exactly; it does not lift the \
             binding",
            store.display(),
            match binding {
                Some(_) => "are this task's".to_owned(),
                None => format!("satisfy `{requirement}`"),
            },
            chosen
                .iter()
                .map(|index| artifacts[*index].id.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            match binding {
                Some(binding) => format!("{} (from {}).", binding.work_line(), binding.source),
                None => "No task document is in reach, so the selection is not bound to any work; \
                         `--task <file>` names one."
                    .to_owned(),
            }
        ),
    }
}

/// Why nothing was selected, saying **which end is missing**.
///
/// Three different findings, and a reader acts on each differently: there is no specification here
/// at all; there are specifications and none is in force; there are specifications in force and
/// none of them is this task's. The last is the one the binding added, and it names both ends —
/// what is declared, and what the task said it was about — for the reason the engine's own unmet
/// row does: *two approved specifications are present and the rule is still unmet* reads as a
/// defect in the tool until the sentence says whose they are.
fn nothing_selected(store: &Path, artifacts: &[Artifact], binding: Option<&Binding>) -> String {
    if artifacts.is_empty() {
        return format!(
            "no specification artifact is declared in {}. `spec-driven.before_implementation` \
             asks for one before implementation, so a run with none has nothing to be decided \
             against",
            store.display()
        );
    }
    let Some(binding) = binding else {
        return format!(
            "no specification in {} is `approved`, `accepted`, `active` or `implemented`, so \
             nothing here is a specification a task may be implemented against — declared: {}. \
             `spec-driven.before_implementation` is the guard that says so, and an `operator` step \
             is where a person approves one",
            store.display(),
            declared(artifacts)
        );
    };
    format!(
        "no specification in {} is `{}` — declared: {}; {} (from {}). This is the guard's own \
         rule, so a record about any of these would be one `spec-driven.before_implementation` \
         refuses. A specification of this task carries `specifies:` the artifact the task is \
         derived from; `protocol artifact relate <id> specifies <target>` writes that edge, and an \
         `operator` step is where a person approves the document",
        store.display(),
        specification_of_this_task(),
        declared(artifacts),
        binding.work_line(),
        binding.source
    )
}

/// One requirement, as written, with everything in it that might decide it.
#[derive(Debug, PartialEq, Eq)]
struct Requirement {
    /// The item's text, as the specification wrote it, wrapped lines folded into one.
    text: String,
    /// Every inline code span in the item, in the order they appear.
    ///
    /// All of them and not the first, because a requirement legitimately quotes a command, a path
    /// or a field name beside the predicate that decides it — the shipped specifications in this
    /// repository do exactly that. Which one is the predicate is decided by parsing, not by
    /// position; see [`decide`].
    spans: Vec<String>,
}

/// `true` when a line opens a markdown list item.
fn list_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            return Some(rest);
        }
    }
    // An ordered item: digits, then `.` or `)`, then a space.
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &trimmed[digits.len()..];
    for marker in [". ", ") "] {
        if let Some(rest) = rest.strip_prefix(marker) {
            return Some(rest);
        }
    }
    None
}

/// The heading a `#`-prefixed line declares, lowercased, or `None`.
fn heading(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    Some(
        trimmed
            .trim_start_matches('#')
            .trim()
            .trim_end_matches(':')
            .to_lowercase(),
    )
}

/// Every inline code span in `text`, in order.
///
/// A backtick pair and nothing cleverer — no fenced blocks, no escapes, no nesting. Markdown's real
/// grammar is larger than this and the difference does not matter here: what is being looked for is
/// a predicate somebody wrote between backticks, and a span this misreads cannot parse as one.
fn code_spans(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some((_, after)) = rest.split_once('`') {
        let Some((inside, tail)) = after.split_once('`') else {
            break;
        };
        let inside = inside.trim();
        if !inside.is_empty() {
            found.push(inside.to_owned());
        }
        rest = tail;
    }
    found
}

/// How much of a requirement's text a record repeats.
///
/// A record is read by a person deciding what to fix, and a requirement written as a paragraph
/// would otherwise put the paragraph in the evidence document. The predicate — which is what makes
/// the line actionable — is appended after the text by [`unmet`], so a truncation never hides it.
const TEXT_LIMIT: usize = 160;

/// `text`, cut to [`TEXT_LIMIT`] characters on a character boundary.
fn shortened(text: &str) -> String {
    if text.chars().count() <= TEXT_LIMIT {
        return text.to_owned();
    }
    let kept: String = text.chars().take(TEXT_LIMIT).collect();
    format!("{}…", kept.trim_end())
}

/// Every requirement the specification states, in the order it states them.
///
/// See the [module documentation](self) for what counts as one and why. A nested list item is one
/// requirement like any other: a specification that indents a clause still stated it.
fn requirements(body: &str) -> Vec<Requirement> {
    let mut found = Vec::new();
    let mut inside = false;
    let mut fenced = false;

    for line in body.lines() {
        // A fenced block holds examples, not requirements — a bullet inside one is somebody showing
        // what a document looks like.
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(heading) = heading(line) {
            inside = REQUIREMENT_HEADINGS
                .iter()
                .any(|wanted| heading.starts_with(wanted));
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(item) = list_item(line) {
            let text = item.trim().to_owned();
            if text.is_empty() {
                continue;
            }
            found.push(Requirement {
                spans: code_spans(&text),
                text,
            });
            continue;
        }
        // A wrapped line: markdown folds it into the item above, and a requirement read as its
        // first line only reads as a sentence somebody cut in half — which is what the record then
        // hands back to whoever has to satisfy it.
        if let Some(last) = found.last_mut() {
            let continuation = line.trim();
            if !continuation.is_empty() && line.starts_with(char::is_whitespace) {
                last.text.push(' ');
                last.text.push_str(continuation);
                last.spans = code_spans(&last.text);
            }
        }
    }
    found
}

/// The facts the run's admitted evidence projects, in submission order.
///
/// Later bindings win, which is the same rule `Evidence::facts` states for its own list: a second
/// test result supersedes the first for `tests.unit.failed`. The engine's derived facts are
/// deliberately absent — see the [module documentation](self).
fn observed(snapshot: Option<&Snapshot>) -> FactStore {
    let mut facts = FactStore::new();
    let Some(snapshot) = snapshot else {
        return facts;
    };
    for recorded in &snapshot.evidence {
        facts.extend_facts(recorded.record.facts());
    }
    facts
}

/// Reads the run's snapshot, or refuses naming the file.
fn read_snapshot(path: &Path) -> Result<Snapshot> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading the run's snapshot at {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("{} is not a run snapshot this build reads", path.display()))
}

/// Why a requirement is not satisfied, in the words the record hands back.
///
/// Three reasons and they are not interchangeable: `Unknown` is not `False` (invariant 5), and a
/// requirement nothing can decide is a different defect from one a run failed.
fn unmet(requirement: &Requirement, decided_by: Option<&str>, verdict: Truth) -> Option<String> {
    let text = shortened(&requirement.text);
    match (decided_by, verdict) {
        (_, Truth::True) => None,
        (Some(written), Truth::False) => Some(format!("{text} — `{written}` is false")),
        (Some(written), Truth::Unknown) => {
            Some(format!("{text} — nothing has observed `{written}`"))
        }
        (None, _) if requirement.spans.is_empty() => Some(format!(
            "{text} — states no predicate a verifier could decide"
        )),
        (None, _) => Some(format!(
            "{text} — states no predicate a verifier could decide; `{}` {}",
            requirement.spans.join("`, `"),
            if requirement.spans.len() == 1 {
                "is not one"
            } else {
                "are not predicates"
            }
        )),
    }
}

/// The comparison operators a written predicate can carry.
///
/// Used only to rank candidate spans, never to parse one — the parser is
/// [`Predicate::parse_expression`] and stays the single reader of the language.
const COMPARISONS: [&str; 6] = ["==", "!=", ">=", "<=", ">", "<"];

/// The span in a requirement that decides it, with the predicate it parses to.
///
/// A comparison is preferred over a bare fact path, and the reason is a false positive worth
/// avoiding: `scan-declarations.sh` is a filename and also a syntactically valid fact path, so an
/// item that quotes a script beside a real comparison would otherwise be decided by the filename
/// and report *nothing has observed `scan-declarations.sh`* — true, useless, and misleading about
/// what the specification asked for. Both readings are unmet, so this changes no verdict; it
/// changes what the record tells the person who has to act on it.
fn decided_by(requirement: &Requirement) -> Option<(&str, Predicate)> {
    fn parsed(span: &str) -> Option<(&str, Predicate)> {
        Predicate::parse_expression(span)
            .ok()
            .map(|predicate| (span, predicate))
    }
    requirement
        .spans
        .iter()
        .map(std::string::String::as_str)
        .filter(|span| COMPARISONS.iter().any(|operator| span.contains(operator)))
        .find_map(parsed)
        .or_else(|| {
            requirement
                .spans
                .iter()
                .map(std::string::String::as_str)
                .find_map(parsed)
        })
}

/// Every requirement, decided.
///
/// The predicate is the code span in the item that **parses** as one, rather than the first code
/// span. A requirement legitimately quotes a command or a path beside the fact that decides it, and
/// position is not a reliable way to tell them apart; parsing is. An item whose spans are all prose
/// states no predicate, and the reason names them so the author can see what was tried.
fn decide(found: &[Requirement], facts: &FactStore) -> Vec<String> {
    let mut unsatisfied = Vec::new();
    for requirement in found {
        let decided_by = decided_by(requirement);
        let (written, verdict) = match decided_by {
            Some((written, predicate)) => (Some(written), predicate.evaluate(facts)),
            None => (None, Truth::Unknown),
        };
        if let Some(reason) = unmet(requirement, written, verdict) {
            unsatisfied.push(reason);
        }
    }
    unsatisfied
}

/// `protocol specification evidence`
fn mint_evidence(args: &EvidenceArgs) -> Result<ExitCode> {
    let document = select(&args.store, args.artifact.as_deref(), args.task.as_deref())?;
    let snapshot = args.snapshot.as_deref().map(read_snapshot).transpose()?;
    let facts = observed(snapshot.as_ref());

    let found = requirements(&document.body);
    let unsatisfied = decide(&found, &facts);

    let record = SpecificationRecord {
        artifact: Some(ArtifactRef::unpinned(document.frontmatter.id.clone())),
        // Every requirement met, and a specification that states none is **not** satisfied: an
        // empty obligation discharged vacuously is the shape `contract-testing` refuses `checked: 0`
        // for, one principle over.
        satisfied: !found.is_empty() && unsatisfied.is_empty(),
        requirements_total: Some(found.len()),
        requirements_satisfied: Some(found.len().saturating_sub(unsatisfied.len())),
        unsatisfied,
    };

    let mut minted = MintedEvidence::new(
        Evidence::Specification(record),
        producer(),
        // The reading happened in this process, in this second.
        crate::now_observed(),
    )
    .obtained_by(invocation(args))
    .reading(args.store.display().to_string());
    if let Some(path) = &args.snapshot {
        minted = minted.reading(path.display().to_string());
    }

    emit(&minted, args.format, args.out.as_deref())?;
    // Exit 0 for an unsatisfied specification as well. The verdict is in the record.
    Ok(ExitCode::SUCCESS)
}

/// The command line, as the record's provenance reports it.
fn invocation(args: &EvidenceArgs) -> String {
    use std::fmt::Write as _;

    let mut line = format!(
        "protocol specification evidence --store {}",
        args.store.display()
    );
    if let Some(path) = &args.snapshot {
        let _ = write!(line, " --snapshot {}", path.display());
    }
    if let Some(id) = &args.artifact {
        let _ = write!(line, " --artifact {id}");
    }
    if let Some(path) = &args.task {
        let _ = write!(line, " --task {}", path.display());
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::facts::FactValue;

    /// A fact store built from a list of bindings, for a fixture.
    fn facts_from(pairs: &[(&str, FactValue)]) -> FactStore {
        let mut facts = FactStore::new();
        for (path, value) in pairs {
            facts.set_path(path, value.clone());
        }
        facts
    }

    /// A specification body with one of every case the extractor has to tell apart.
    const BODY: &str = "\
# Specification\n\
\n\
Some prose that states nothing checkable.\n\
\n\
## Acceptance\n\
\n\
- The unit suite is green: `tests.unit.failed == 0`\n\
- Static analysis reports nothing: `static_analysis.errors == 0`\n\
- The change is reviewed carefully\n\
\n\
## Notes\n\
\n\
- Not a requirement, because this is not a requirements section.\n\
\n\
### Requirements (further)\n\
\n\
1. Contracts hold: `contracts.breaking_changes == 0`\n\
\n\
```\n\
- A bullet in a fenced block is an example, not a requirement.\n\
```\n\
";

    /// The extraction rule, at the boundary of every clause it has: sections in and out, list
    /// markers of both shapes, a fenced example, and an item that names no predicate.
    #[test]
    fn a_requirement_is_a_list_item_under_a_requirements_or_acceptance_heading() {
        let found = requirements(BODY);
        assert_eq!(
            found
                .iter()
                .map(|requirement| requirement.spans.clone())
                .collect::<Vec<_>>(),
            vec![
                vec!["tests.unit.failed == 0".to_owned()],
                vec!["static_analysis.errors == 0".to_owned()],
                Vec::new(),
                vec!["contracts.breaking_changes == 0".to_owned()],
            ],
            "three from `## Acceptance`, one from `### Requirements`, none from `## Notes` and \
             none from the fenced block: {found:#?}"
        );
    }

    /// A requirement whose predicate is `True` is satisfied, and the other three roads are all
    /// unmet — each saying which road it took.
    #[test]
    fn a_requirement_is_unmet_unless_its_own_predicate_is_observed_true() {
        let facts = facts_from(&[
            ("tests.unit.failed", FactValue::count(0)),
            ("static_analysis.errors", FactValue::count(3)),
        ]);
        let unsatisfied = decide(&requirements(BODY), &facts);

        assert_eq!(unsatisfied.len(), 3, "{unsatisfied:#?}");
        assert!(
            unsatisfied[0].contains("is false"),
            "an observed contradiction says so: {unsatisfied:#?}"
        );
        assert!(
            unsatisfied[1].contains("states no predicate"),
            "a requirement nothing can decide is unmet and says why: {unsatisfied:#?}"
        );
        assert!(
            unsatisfied[2].contains("nothing has observed"),
            "invariant 5: an unobserved fact is not a failure, and the two read differently: \
             {unsatisfied:#?}"
        );
    }

    /// Invariant 5 at the layer that would most like to collapse it: an empty fact store makes
    /// every checkable requirement `Unknown`, and `Unknown` does not satisfy.
    #[test]
    fn a_run_that_has_observed_nothing_satisfies_no_requirement() {
        let unsatisfied = decide(&requirements(BODY), &FactStore::new());
        assert_eq!(
            unsatisfied.len(),
            4,
            "every requirement is unmet when nothing has been observed: {unsatisfied:#?}"
        );
    }

    /// A specification that states no requirement is not satisfied.
    ///
    /// Without this rule the easiest way to discharge `spec-driven` would be to write a
    /// specification with no `## Acceptance` section at all — the vacuous pass `contract-testing`
    /// refuses `checked: 0` for.
    #[test]
    fn a_specification_that_states_no_requirement_does_not_satisfy_the_principle() {
        let found = requirements("# Specification\n\nIt should be good.\n");
        assert!(found.is_empty(), "{found:#?}");
        let satisfied = !found.is_empty() && decide(&found, &FactStore::new()).is_empty();
        assert!(
            !satisfied,
            "a specification nobody can disagree with has not been satisfied; it has been avoided"
        );
    }

    /// A requirement whose code spans are all prose states no predicate, and the reason names the
    /// spans that were tried — so an author who meant one of them to be a predicate can see why it
    /// was not read as one.
    #[test]
    fn a_requirement_whose_spans_are_all_prose_names_what_was_tried() {
        let unsatisfied = decide(
            &requirements("## Acceptance\n\n- Broken: `== 0` and `bash run.sh`\n"),
            &FactStore::new(),
        );
        assert_eq!(unsatisfied.len(), 1, "{unsatisfied:#?}");
        assert!(
            unsatisfied[0].contains("states no predicate") && unsatisfied[0].contains("`== 0`"),
            "the reason names the spans it could not read: {unsatisfied:#?}"
        );
    }

    /// A requirement that quotes a command beside the fact that decides it is decided by the fact.
    ///
    /// The shipped specifications in this repository are written that way, and a rule that took the
    /// *first* span would read every one of them as undecidable while the fact it needed was two
    /// words later.
    #[test]
    fn the_predicate_is_the_span_that_parses_rather_than_the_first_one() {
        let body = "## Acceptance\n\n- `bash run.sh` exits 0, so `tests.unit.failed == 0`\n";
        assert!(
            decide(
                &requirements(body),
                &facts_from(&[("tests.unit.failed", FactValue::count(0))])
            )
            .is_empty(),
            "the fact decides it, not the command quoted beside it"
        );
    }

    /// A quoted filename parses as a fact path, so a comparison in the same item wins.
    ///
    /// Without the preference the record would report *nothing has observed
    /// `scan-declarations.sh`* — an unmet verdict that is true and tells the author nothing about
    /// the requirement they actually wrote.
    #[test]
    fn a_quoted_filename_does_not_outrank_the_comparison_beside_it() {
        let found = requirements(
            "## Acceptance\n\n- `scan-declarations.sh` is clean: `tests.unit.failed == 0`\n",
        );
        assert_eq!(
            decided_by(&found[0]).map(|(span, _)| span),
            Some("tests.unit.failed == 0"),
            "{found:#?}"
        );
    }

    /// A wrapped list item is one requirement, folded, so the record hands back a sentence rather
    /// than the half of it that fitted on the first line.
    #[test]
    fn a_requirement_wrapped_over_two_lines_is_one_requirement() {
        let found = requirements(
            "## Acceptance\n\n- The suite is green and stays green,\n  which is `tests.unit.failed == 0`\n",
        );
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(
            found[0].text.ends_with("`tests.unit.failed == 0`"),
            "the continuation line is folded in: {found:#?}"
        );
        assert!(
            decide(
                &found,
                &facts_from(&[("tests.unit.failed", FactValue::count(0))])
            )
            .is_empty(),
            "and the predicate on the second line still decides it"
        );
    }

    /// The store a selection is decided over, as a path a refusal can name.
    fn store() -> &'static Path {
        Path::new(".engineering/planning")
    }

    /// A specification artifact, with the edges a driven run's `specify` state writes.
    fn specification(id: &str, status: ArtifactStatus, specifies: &[&str]) -> Artifact {
        let mut artifact = Artifact::new(
            aep_domain::artifact::ArtifactId::new(id).expect("an artifact id"),
            ArtifactKind::Specification,
            status,
            aep_domain::artifact::ArtifactLocation::Inline,
        );
        for target in specifies {
            artifact = artifact.with_relation(
                RelationKind::Specifies,
                ArtifactRef::parse(target).expect("an artifact reference"),
            );
        }
        artifact
    }

    /// The store, as `choose` reads it.
    fn graph_of(artifacts: &[Artifact]) -> ArtifactGraph {
        let mut graph = ArtifactGraph::new();
        for artifact in artifacts.iter().cloned() {
            graph.insert(artifact);
        }
        graph
    }

    /// A binding from a task document, through the same [`Task::declared_work`] the engine calls.
    ///
    /// Parsed rather than constructed: what a task declares is what the document says, and a
    /// hand-built work list would test this function against itself.
    fn binding(task: &str) -> Binding {
        Binding {
            source: ".engineering/task.yaml".to_owned(),
            work: aep_schema::parse::task(task, None)
                .expect("the task parses")
                .declared_work(),
        }
    }

    /// The task a driven run walks: decomposed from a story, which is the edge the binding reads.
    const TASK: &str = "id: AUTH-142\n\
         kind: feature\n\
         objective: add-passkey-support\n\
         protocol: adp/1\n\
         profile: development.fast\n\
         derived_from:\n  - story:AUTH-141\n";

    /// The selected specification is the one whose `specifies` edge lands on the task's work.
    #[test]
    fn the_specification_decided_is_the_one_that_specifies_this_tasks_work() {
        let artifacts = [
            specification("specification:sessions", ArtifactStatus::Approved, &[]),
            specification(
                "specification:passkeys",
                ArtifactStatus::Approved,
                &["story:AUTH-141"],
            ),
        ];
        let chosen = choose(
            store(),
            &artifacts,
            &graph_of(&artifacts),
            Some(&binding(TASK)),
            None,
        )
        .expect("one specification is this task's");
        assert_eq!(
            artifacts[chosen].id.to_string(),
            "specification:passkeys",
            "the unrelated approved specification beside it does not decide anything"
        );
    }

    /// Another story's approved specification is not this task's, and the refusal says which end
    /// is missing rather than leaving the reader to guess.
    ///
    /// This is the defect the whole binding exists for: before it, the verb wrote a `specification`
    /// record about exactly this document — a verdict about somebody else's story, carrying this
    /// run's `specification.satisfied`.
    #[test]
    fn another_storys_approved_specification_is_not_this_tasks() {
        let artifacts = [specification(
            "specification:sessions",
            ArtifactStatus::Approved,
            &["story:AUTH-9"],
        )];
        let refusal = choose(
            store(),
            &artifacts,
            &graph_of(&artifacts),
            Some(&binding(TASK)),
            None,
        )
        .expect_err("an approved specification of another story decides nothing here")
        .to_string();

        for named in [
            "specification:sessions (approved)",
            "this task's work is story:AUTH-141, task:AUTH-142",
            "specifies this task",
        ] {
            assert!(
                refusal.contains(named),
                "the refusal names both ends of the missing edge; `{named}` is not in:\n{refusal}"
            );
        }
    }

    /// Two specifications of this task's work are a refusal listing both, not a coin toss.
    #[test]
    fn two_specifications_of_this_tasks_work_are_refused_and_both_are_named() {
        let artifacts = [
            specification(
                "specification:passkeys",
                ArtifactStatus::Approved,
                &["story:AUTH-141"],
            ),
            specification(
                "specification:passkeys-v2",
                ArtifactStatus::Active,
                &["task:AUTH-142"],
            ),
            // A third, approved, of another story: it is not one of the candidates, so the
            // ambiguity being reported is an ambiguity inside this task's own work.
            specification(
                "specification:sessions",
                ArtifactStatus::Approved,
                &["story:AUTH-9"],
            ),
        ];
        let refusal = choose(
            store(),
            &artifacts,
            &graph_of(&artifacts),
            Some(&binding(TASK)),
            None,
        )
        .expect_err("a step establishes one thing, and this store offers two")
        .to_string();

        for named in [
            "specification:passkeys",
            "specification:passkeys-v2",
            "--artifact",
        ] {
            assert!(
                refusal.contains(named),
                "the refusal names every candidate and the way to say which; `{named}` is not \
                 in:\n{refusal}"
            );
        }
        assert!(
            !refusal.contains("specification:sessions"),
            "another story's approved specification is not a candidate to be ambiguous with:\n\
             {refusal}"
        );
    }

    /// A task that was decomposed from nothing is still matched by a specification of the task
    /// itself — `Task::declared_work` contributes `task:<id>`, and a rule that refused it would be
    /// disagreeing with its own name.
    #[test]
    fn a_task_that_declares_no_story_is_matched_by_a_specification_of_the_task_itself() {
        let bare = "id: AUTH-142\n\
             kind: feature\n\
             objective: add-passkey-support\n\
             protocol: adp/1\n\
             profile: development.fast\n";
        let artifacts = [
            // Approved, of another story, and beside it in the store — so a selection that had
            // stopped consulting the task at all would be ambiguous here rather than right.
            specification(
                "specification:sessions",
                ArtifactStatus::Approved,
                &["story:AUTH-9"],
            ),
            specification(
                "specification:passkeys",
                ArtifactStatus::Approved,
                &["task:AUTH-142"],
            ),
        ];
        let chosen = choose(
            store(),
            &artifacts,
            &graph_of(&artifacts),
            Some(&binding(bare)),
            None,
        )
        .expect("`specifies: task:AUTH-142` is exactly the relationship the rule asks about");
        assert_eq!(artifacts[chosen].id.to_string(), "specification:passkeys");
    }

    /// `--artifact` names which specification, never whether the binding applies.
    ///
    /// The one that matters for the safety envelope: an escape hatch here would be a way to mint
    /// the record `spec-driven.before_implementation` exists to withhold.
    #[test]
    fn an_artifact_named_on_the_command_line_still_has_to_specify_this_task() {
        let artifacts = [specification(
            "specification:sessions",
            ArtifactStatus::Approved,
            &["story:AUTH-9"],
        )];
        let refusal = choose(
            store(),
            &artifacts,
            &graph_of(&artifacts),
            Some(&binding(TASK)),
            Some("specification:sessions"),
        )
        .expect_err("naming a document does not make it this task's")
        .to_string();
        assert!(
            refusal.contains("is not a specification which specifies this task")
                && refusal.contains("does not lift the binding"),
            "{refusal}"
        );
    }

    /// The status half is what `--artifact` does lift: asking whether a `draft` specification
    /// states anything decidable is a legitimate question, and its edge still has to land here.
    #[test]
    fn an_artifact_named_on_the_command_line_may_be_a_draft_of_this_tasks_work() {
        let artifacts = [specification(
            "specification:passkeys",
            ArtifactStatus::Draft,
            &["story:AUTH-141"],
        )];
        let chosen = choose(
            store(),
            &artifacts,
            &graph_of(&artifacts),
            Some(&binding(TASK)),
            Some("specification:passkeys"),
        )
        .expect("a draft of this task's work may be asked whether it states anything decidable");
        assert_eq!(artifacts[chosen].id.to_string(), "specification:passkeys");
    }

    /// Which statuses count as a specification a task may be implemented against — the same four
    /// `spec-driven` accepts, and no more. Asked of the rule this verb selects by, so a change to
    /// the rule is a change to this answer.
    #[test]
    fn only_a_specification_in_force_is_one_a_task_may_be_implemented_against() {
        let rule = unbound(&specification_of_this_task());
        for status in [
            ArtifactStatus::Approved,
            ArtifactStatus::Accepted,
            ArtifactStatus::Active,
            ArtifactStatus::Implemented,
        ] {
            let artifact = specification("specification:passkeys", status.clone(), &[]);
            assert!(
                rule.matches(&artifact, &graph_of(std::slice::from_ref(&artifact)), &[]),
                "{status:?}"
            );
        }
        for status in [
            ArtifactStatus::Draft,
            ArtifactStatus::Proposed,
            ArtifactStatus::InReview,
            ArtifactStatus::Rejected,
            ArtifactStatus::Superseded,
            ArtifactStatus::Archived,
        ] {
            let artifact = specification("specification:passkeys", status.clone(), &[]);
            assert!(
                !rule.matches(&artifact, &graph_of(std::slice::from_ref(&artifact)), &[]),
                "`{status:?}` is not a specification anything may be built against"
            );
        }
    }

    /// **The rule this verb selects by is the rule the shipped principles declare** — parsed from
    /// the documents rather than described here.
    ///
    /// The verb is only as tight as the guard it serves while the two agree, and they are two
    /// files. This is the check that makes the agreement mechanical: an edit to either principle
    /// that this verb has not followed fails here, rather than turning up as a record about a
    /// document `before_implementation` refuses.
    #[test]
    fn the_rule_this_verb_selects_by_is_the_one_the_shipped_principles_declare() {
        let rule = specification_of_this_task();
        for principle in ["spec-driven", "clean-room"] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../principles/development")
                .join(format!("{principle}.yaml"));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
            let parsed = aep_schema::parse::principle(&text, Some(&path.display().to_string()))
                .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()));
            let declared: Vec<&ArtifactRequirement> = parsed
                .obligations
                .iter()
                .flat_map(|obligation| &obligation.requires.artifacts)
                .filter(|requirement| requirement.kind == ArtifactKind::Specification)
                .collect();
            assert_eq!(
                declared,
                vec![&rule],
                "`{principle}` is what asks for a specification of this task, and this verb writes \
                 the record that guard reads; the two must state one rule: {declared:#?}"
            );
        }
    }

    /// With no task in reach the selection is unbound, and the store's one in-force specification
    /// is still decided — a person pointing this verb at a store from outside a project is not
    /// refused by a binding nothing could have answered.
    #[test]
    fn with_no_task_in_reach_the_stores_one_in_force_specification_is_still_decided() {
        let artifacts = [specification(
            "specification:sessions",
            ArtifactStatus::Approved,
            &["story:AUTH-9"],
        )];
        let chosen = choose(store(), &artifacts, &graph_of(&artifacts), None, None)
            .expect("nothing said what this run is about, and one document is in force");
        assert_eq!(artifacts[chosen].id.to_string(), "specification:sessions");
    }
}
