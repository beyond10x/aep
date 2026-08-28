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
//! The one the run is being held to: a `specification` artifact in the store whose status is
//! `approved` or later, which is exactly what `spec-driven.before_implementation` asks for. With
//! none, or with more than one, this verb **writes nothing and says why** — a step establishes one
//! thing, and picking one of several would be the tool deciding what the run is about. The driver
//! reads that as D5's `Unknown`: nothing observed, and the run stops at the guard instead of moving
//! on a record about the wrong document.
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
use aep_domain::artifact::{ArtifactKind, ArtifactRef, ArtifactStatus};
use aep_domain::evidence::{Evidence, SpecificationRecord};
use aep_domain::facts::FactStore;
use aep_domain::predicate::{Predicate, Truth};
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
    /// Which specification to decide. Discovered from the store when omitted.
    #[arg(long, value_name = "ID")]
    artifact: Option<String>,
    /// The task document, whose referenced artifacts say which specification the run is about.
    ///
    /// Discovered from the project when omitted. It is what makes discovery work in a store that
    /// holds more than one in-force specification — see [`narrow`].
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

/// `true` when a specification in this status is one a task may be implemented against.
///
/// The same four `spec-driven` accepts: *"`approved` is also satisfied by `accepted`, `active` and
/// `implemented`, so a specification already in force does not have to be re-approved for each task
/// that implements it."* Read off that comment rather than invented here, so the two cannot drift.
fn is_in_force(status: &ArtifactStatus) -> bool {
    matches!(
        status,
        ArtifactStatus::Approved
            | ArtifactStatus::Accepted
            | ArtifactStatus::Active
            | ArtifactStatus::Implemented
    )
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

/// Every artifact the task points at, as written.
///
/// `derived_from` and `context` both, because a task may name the story it decomposes *and* the
/// specification it is written against, and either is a legitimate way to say which document this
/// run is about.
fn task_artifacts(task: &Path) -> Result<Vec<String>> {
    let task = crate::read_task(task)?;
    Ok(task
        .artifacts
        .all()
        .into_iter()
        .map(|reference| reference.id().to_string())
        .collect())
}

/// The task the project names, when this was run inside one.
///
/// A convenience and never a requirement: a discovery that fails leaves the selection unnarrowed
/// and the refusal below says the whole in-force set, which is a better failure than silently
/// deciding the wrong document.
fn discovered_task() -> Option<Vec<String>> {
    let here = std::env::current_dir().ok()?;
    let root = aep_engine::project::discover(&here)?;
    let project = aep_engine::project::load(&root).ok()?;
    let task = project.task?;
    Some(
        task.artifacts
            .all()
            .into_iter()
            .map(|reference| reference.id().to_string())
            .collect(),
    )
}

/// The specifications that are about what the task is about.
///
/// A specification created by a run under `adp/default` relates to the work it specifies —
/// `specifies: story:…`, `derived_from: task:…` — and the task document names the same artifacts in
/// its `derived_from`. That shared reference is the join, and it is the only non-arbitrary one
/// available: the run's specification is created *during* the run, so no map can name it by id and
/// the store's file order says nothing.
///
/// Returns the wider set unchanged when the narrowing finds nothing, so a task that references no
/// artifact is no worse off than before — and the refusal that follows names every candidate rather
/// than an empty list.
fn narrow(candidates: Vec<PlanningDocument>, referenced: &[String]) -> Vec<PlanningDocument> {
    if referenced.is_empty() {
        return candidates;
    }
    let about: Vec<PlanningDocument> = candidates
        .iter()
        .filter(|document| {
            document.frontmatter.relations.iter().any(|relation| {
                referenced
                    .iter()
                    .any(|wanted| relation.target.id().to_string() == *wanted)
            }) || referenced.contains(&document.frontmatter.id.to_string())
        })
        .cloned()
        .collect();
    if about.is_empty() {
        candidates
    } else {
        about
    }
}

/// The one specification this run is being held to, or a refusal naming what it found instead.
fn select(store: &Path, wanted: Option<&str>, task: Option<&Path>) -> Result<PlanningDocument> {
    let found = specifications(store)?;

    if let Some(id) = wanted {
        let Some((_, document)) = found
            .into_iter()
            .find(|(_, document)| document.frontmatter.id.to_string() == id)
        else {
            bail!(
                "no specification artifact `{id}` in {}. `--artifact` names one exactly; without \
                 it the store's one in-force specification is used",
                store.display()
            );
        };
        return Ok(document);
    }

    let referenced = match task {
        Some(path) => task_artifacts(path)?,
        None => discovered_task().unwrap_or_default(),
    };
    let mut in_force = narrow(
        found
            .into_iter()
            .map(|(_, document)| document)
            .filter(|document| is_in_force(&document.frontmatter.status))
            .collect(),
        &referenced,
    );

    match in_force.len() {
        1 => Ok(in_force.remove(0)),
        0 => bail!(
            "no specification in {} is `approved`, `accepted`, `active` or `implemented`, so \
             nothing here is a specification a task may be implemented against. \
             `spec-driven.before_implementation` is the guard that says so, and an `operator` step \
             is where a person approves one",
            store.display()
        ),
        held => bail!(
            "{held} specifications in {} are in force — {} — and none of them is the only one the \
             task references, so a step here would establish something about a document nobody \
             said this run was about. `--artifact` names one exactly, and a specification that \
             relates to what the task's `derived_from` names is found without it",
            store.display(),
            in_force
                .iter()
                .map(|document| document.frontmatter.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
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

    /// Which statuses count as a specification a task may be implemented against — the same four
    /// `spec-driven` accepts, and no more.
    #[test]
    fn only_a_specification_in_force_is_one_a_task_may_be_implemented_against() {
        for status in [
            ArtifactStatus::Approved,
            ArtifactStatus::Accepted,
            ArtifactStatus::Active,
            ArtifactStatus::Implemented,
        ] {
            assert!(is_in_force(&status), "{status:?}");
        }
        for status in [
            ArtifactStatus::Draft,
            ArtifactStatus::Proposed,
            ArtifactStatus::InReview,
            ArtifactStatus::Rejected,
            ArtifactStatus::Superseded,
            ArtifactStatus::Archived,
        ] {
            assert!(
                !is_in_force(&status),
                "`{status:?}` is not a specification anything may be built against"
            );
        }
    }
}
