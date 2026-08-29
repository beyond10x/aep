//! `story:attested-approver`: an `operator` step is answered by whoever the record says answered
//! it, and the run's own cursor says who — or that nobody did.
//!
//! The loop is driven against the same fake harness `tests/driving.rs` uses, and the approvals
//! are written the way a person or an agent writes one between a pause and a resume: into the
//! run's snapshot, through the engine, while nothing of the run is executing. The tests construct
//! `Evidence::Approval` freely — the scan that refuses that is over the crate's shipped sources,
//! and what it protects is exactly what these tests read: the driver never wrote any of them.
//!
//! Each test reaches the state where its rule is load-bearing before asserting the outcome: a run
//! that named the approver *and* got an approval from it, a run that got one from somebody else,
//! a run whose own actor tried, and a run under the unchanged human route.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use aep_backend_markdown::MarkdownStore;
use aep_domain::artifact::ArtifactGraph;
use aep_domain::entity::ActorRef;
use aep_domain::evidence::{ApprovalDecision, ApprovalRecord, ChangeSet, Evidence, Producer};
use aep_domain::ids::ApprovalId;
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_driver::attest::{admit, session_actor, Admission};
use aep_driver::executor::{
    CommandStepExecutor, LlmStepExecutor, OperatorStepExecutor, StepAuthorizer, StepContext,
    StepOutcome,
};
use aep_driver::run::{drive, resume, DriverOptions, RunDirectory, RunReport};
use aep_driver_spec::cursor::RunStatus;
use aep_driver_spec::map::{CommandStep, LlmStep, OperatorStep, StepMap};
use aep_engine::clock::SteppingClock;
use aep_engine::engine::{Engine, EvidenceSubmission, ProtocolEngine};
use aep_engine::registry::Registry;

const PROTOCOL: &str = r"
id: aep
version: 1
title: Test protocol
capabilities: [repository.read, repository.write, tests.execute]
evidence_kinds: [test_result, diff, approval]
verifiers: [test-runner, compiler, human-approval]
artifact_kinds: [specification, story]
phases: [implementation, verification, completion]
observables:
  - 'task.**'
  - 'tests.**'
  - 'diff.**'
  - 'artifact.**'
  - 'evidence.**'
  - 'state.**'
  - 'workflow.**'
  - 'approval.**'
  - 'approvals.**'
";

const WORKFLOW: &str = r"
id: test/linear
version: 1
title: Linear
initial: implement
states:
  implement:
    title: Implement
    phases: [implementation]
  verify:
    title: Verify
    phases: [verification]
  complete:
    title: Complete
    terminal: true
    phases: [completion]
transitions:
  - from: implement
    to: verify
    when: diff.exists
  - from: verify
    to: complete
    when: tests.unit.failed == 0
";

const PROFILE: &str = r"
id: test.standard
title: Test standard
protocol: aep/1
workflow: test/linear
capabilities:
  allow: [repository.read, repository.write, tests.execute]
completion:
  - tests.unit.failed == 0
";

const TASK: &str = r"
id: T-1
kind: feature
objective: drive something
protocol: aep/1
profile: test.standard
";

/// An `operator` step first, then a step whose evidence moves the run: the shape
/// `establish_verifiers` has, and the shape `a_resumed_run_does_not_ask_the_person_again` pins.
const MAP: &str = r"
format: aep.driver-steps/1
id: test/attested
workflow: test/linear/1
states:
  implement:
    steps:
      - kind: operator
        prompt: approve the specification before implementation begins
      - kind: command
        run: [git, diff]
        evidence:
          kind: diff
          verifier: compiler
";

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .insert_protocol(aep_schema::parse::protocol(PROTOCOL, None).expect("the protocol parses"))
        .expect("the protocol is unique");
    registry
        .insert_workflow(aep_schema::parse::workflow(WORKFLOW, None).expect("the workflow parses"))
        .expect("the workflow is unique");
    registry
        .insert_profile(aep_schema::parse::profile(PROFILE, None).expect("the profile parses"))
        .expect("the profile is unique");
    registry
}

fn engine() -> Engine<SteppingClock> {
    Engine::with_clock(registry(), SteppingClock::new(1_000, 10))
}

fn task() -> Task {
    aep_schema::parse::task(TASK, None).expect("the task parses")
}

fn map() -> StepMap {
    aep_schema::parse::step_map(MAP, Some("test/attested.yaml")).expect("the map validates")
}

fn actor(text: &str) -> ActorRef {
    ActorRef::parse(text).expect("a legal actor")
}

/// What the fake harness does next.
#[derive(Debug, Clone, Copy)]
enum Act {
    Pause,
    Diff,
}

/// A harness that runs a script, and panics if the driver runs a step the test did not expect.
struct Fake {
    script: VecDeque<Act>,
    asked: Vec<usize>,
    /// The execution each step was told it belongs to, so a test can compare what a step is
    /// handed against what the cursor records.
    executions: Vec<String>,
}

impl Fake {
    fn new(script: &[Act]) -> Self {
        Self {
            script: script.iter().copied().collect(),
            asked: Vec::new(),
            executions: Vec::new(),
        }
    }

    fn act(&mut self, context: &StepContext<'_>) -> StepOutcome {
        self.asked.push(context.index);
        self.executions.push(context.execution.to_string());
        match self.script.pop_front().expect(
            "the script has an act for every step the driver runs; an empty script means the \
             driver ran a step the test did not expect",
        ) {
            Act::Pause => StepOutcome::Paused {
                reason: "an operator step is owed an answer".to_owned(),
            },
            Act::Diff => StepOutcome::Observed(Box::new(EvidenceSubmission::new(
                Evidence::Diff(ChangeSet {
                    files_changed: 1,
                    lines_added: 4,
                    lines_removed: 0,
                    revision_before: None,
                    revision_after: None,
                    paths: vec!["src/lib.rs".to_owned()],
                }),
                Producer::Verifier {
                    verifier: Verifier::Compiler,
                },
                ObservedAt::new(Timestamp::EPOCH),
            ))),
        }
    }
}

impl LlmStepExecutor for Fake {
    fn run_llm(
        &mut self,
        _: &LlmStep,
        context: &StepContext<'_>,
        _: StepAuthorizer<'_>,
    ) -> StepOutcome {
        self.act(context)
    }
}

impl CommandStepExecutor for Fake {
    fn run_command(&mut self, _: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        self.act(context)
    }
}

impl OperatorStepExecutor for Fake {
    fn run_operator(&mut self, _: &OperatorStep, context: &StepContext<'_>) -> StepOutcome {
        self.act(context)
    }
}

fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("the previous run's directory is removable");
    }
    std::fs::create_dir_all(&path).expect("a writable scratch directory");
    path
}

/// A run under `root`, paused at the operator step, with `approver` admitted if named.
fn paused(root: &Path, approver: Option<&str>) -> (RunDirectory, DriverOptions, RunReport) {
    let run = RunDirectory::at(root.join("runs").join("T-1").join("1"));
    let options = DriverOptions {
        pause_on_approval: true,
        approver: approver.map(actor),
        ..DriverOptions::default()
    };
    let store = MarkdownStore::open(root.join("planning"));
    let mut fake = Fake::new(&[Act::Pause]);
    let report = drive(
        &engine(),
        &task(),
        &store,
        &map(),
        &run,
        &mut fake,
        &options,
    )
    .expect("a pause is a report");
    assert_eq!(report.status(), RunStatus::AwaitingOperator);
    assert!(
        report.cursor.owed.is_some(),
        "the fixture is a run that owes an answer, or nothing below is about the rule"
    );
    (run, options, report)
}

/// Records one granted approval into the paused run's snapshot, as the approver would.
///
/// Through the engine and into the snapshot on disk — `protocol evaluate --evidence` with the
/// run's snapshot as its state — while the run is stopped. The driver never sees the submission
/// happen; it sees the record on resume.
fn approve(run: &RunDirectory, approver: Producer, approval: &str) {
    let engine = engine();
    let snapshot = run.read_snapshot().expect("the paused run has a snapshot");
    let cursor = run.read_cursor().expect("and a cursor");
    let mut execution = engine
        .restore(task(), ArtifactGraph::new(), snapshot)
        .expect("the snapshot restores");
    engine
        .submit_evidence(
            &mut execution,
            EvidenceSubmission::new(
                Evidence::Approval(ApprovalRecord {
                    approval: ApprovalId::new(approval).expect("an approval id"),
                    approver: approver.clone(),
                    decision: ApprovalDecision::Granted,
                    subject: None,
                    note: None,
                }),
                approver,
                ObservedAt::new(Timestamp::EPOCH),
            ),
        )
        .expect("the protocol declares approvals");
    run.persist(&execution.snapshot(), &cursor)
        .expect("the approval is on disk before the resume");
}

fn resumed(
    root: &Path,
    run: &RunDirectory,
    options: &DriverOptions,
    script: &[Act],
) -> (RunReport, Vec<usize>) {
    let store = MarkdownStore::open(root.join("planning"));
    let mut fake = Fake::new(script);
    let report = resume(&engine(), &task(), &store, &map(), run, &mut fake, options)
        .expect("the run resumes");
    (report, fake.asked)
}

fn agent(name: &str) -> Producer {
    Producer::Agent {
        id: name.to_owned(),
    }
}

#[test]
fn the_named_agents_approval_recorded_while_the_run_was_stopped_answers_the_step_and_the_cursor_says_so(
) {
    let root = scratch("attested-named");
    let (run, options, _) = paused(&root, Some("agent:orchestrator"));
    approve(&run, agent("orchestrator"), "specification");

    let (report, asked) = resumed(&root, &run, &options, &[Act::Diff]);

    assert_eq!(
        asked,
        vec![1],
        "the step after the pause ran, and nobody was asked again"
    );
    assert_eq!(
        report
            .transitions
            .iter()
            .map(|(from, to)| (from.to_string(), to.to_string()))
            .collect::<Vec<_>>(),
        vec![("implement".to_owned(), "verify".to_owned())],
        "the run walked on"
    );
    let answer = report
        .cursor
        .answers
        .first()
        .expect("the cursor says who answered");
    assert_eq!(answer.by, "agent orchestrator");
    assert_eq!(answer.approval, "specification");
    assert_eq!(answer.state.to_string(), "implement");
    assert_eq!(answer.step, 0);
    assert_eq!(report.cursor.owed, None, "nothing is owed any more");
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("answered by agent orchestrator")),
        "the report says so in one line: {:?}",
        report.notes
    );
    let written = run.read_cursor().expect("the cursor is on disk");
    assert_eq!(
        written.answers, report.cursor.answers,
        "and the record on disk carries it"
    );
}

#[test]
fn an_approval_by_an_agent_nobody_named_stops_the_run_again_and_names_who_would_be_admissible() {
    let root = scratch("attested-unnamed");
    let (run, options, _) = paused(&root, Some("agent:orchestrator"));
    approve(&run, agent("someone-else"), "specification");

    let (report, asked) = resumed(&root, &run, &options, &[]);

    assert!(
        asked.is_empty(),
        "no step ran on an answer the run may not count"
    );
    assert_eq!(report.status(), RunStatus::AwaitingOperator);
    assert!(report.cursor.owed.is_some(), "the step is still owed");
    assert!(report.cursor.answers.is_empty());
    let note = report
        .notes
        .iter()
        .find(|note| note.contains("still owed"))
        .expect("the report says the step is still owed");
    assert!(
        note.contains("agent:someone-else"),
        "names who tried: {note}"
    );
    assert!(
        note.contains("agent:orchestrator"),
        "and who would count: {note}"
    );
    assert!(
        note.contains("a person"),
        "and that a person always does: {note}"
    );
}

#[test]
fn the_runs_own_actor_cannot_answer_its_own_operator_step_even_when_it_was_named() {
    let root = scratch("attested-self");
    let (run, _, paused_report) = paused(&root, Some("agent:orchestrator"));
    let own = paused_report.cursor.execution.to_string();
    approve(&run, agent(&own), "specification");

    // Named as the approver, which is the case that must buy nothing.
    let options = DriverOptions {
        pause_on_approval: true,
        approver: Some(actor(&format!("agent:{own}"))),
        ..DriverOptions::default()
    };
    let (report, asked) = resumed(&root, &run, &options, &[]);

    assert!(asked.is_empty());
    assert_eq!(report.status(), RunStatus::AwaitingOperator);
    let note = report
        .notes
        .iter()
        .find(|note| note.contains("still owed"))
        .expect("the step is still owed");
    assert!(note.contains("own actor"), "the refusal says why: {note}");
}

#[test]
fn a_person_who_recorded_nothing_still_walks_on_and_the_report_says_the_record_holds_nobodys_answer(
) {
    // The route as it was: a person moves the artifact the prompt names, resumes, and the guard
    // on the way out decides. Unchanged — except that the report now says the record holds no
    // answer, which is what tells a run that was approved from one that merely walked on.
    let root = scratch("attested-human-silent");
    let (run, options, _) = paused(&root, None);

    let (report, asked) = resumed(&root, &run, &options, &[Act::Diff]);

    assert_eq!(asked, vec![1]);
    assert_eq!(report.transitions.len(), 1, "the run walked on as before");
    assert!(report.cursor.answers.is_empty());
    assert_eq!(report.cursor.owed, None);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("holds nobody's")),
        "the absence is in the report: {:?}",
        report.notes
    );
}

#[test]
fn a_persons_recorded_approval_answers_the_step_without_being_named() {
    let root = scratch("attested-human-recorded");
    let (run, options, _) = paused(&root, None);
    approve(
        &run,
        Producer::Human {
            id: "alice".to_owned(),
        },
        "specification",
    );

    let (report, asked) = resumed(&root, &run, &options, &[Act::Diff]);

    assert_eq!(asked, vec![1]);
    assert_eq!(
        report
            .cursor
            .answers
            .first()
            .map(|answer| answer.by.as_str()),
        Some("human alice")
    );
}

#[test]
fn with_an_approver_named_a_resume_that_found_nothing_recorded_stops_again() {
    // Naming an approver is asking for a recorded answer: an agent's approval exists only as a
    // record, so a resume that finds none has nothing to count and says so.
    let root = scratch("attested-named-silent");
    let (run, options, _) = paused(&root, Some("agent:orchestrator"));

    let (report, asked) = resumed(&root, &run, &options, &[]);

    assert!(asked.is_empty());
    assert_eq!(report.status(), RunStatus::AwaitingOperator);
    let note = report
        .notes
        .iter()
        .find(|note| note.contains("still owed"))
        .expect("still owed");
    assert!(note.contains("nothing was recorded"), "{note}");
    assert!(note.contains("agent:orchestrator"), "{note}");
}

/// The run's two names for itself are one name, end to end.
///
/// A step is told which execution it belongs to; `protocol-cli` turns that into the `AEP_ACTOR`
/// its session writes to the planning store under; and `own_actors` turns the *same* value into
/// the actor an approval may not come from. If any link spelled it differently, a driven run could
/// approve its own specification under the very identity it wrote it with — which is the one
/// outcome the `operator` step exists to prevent, and the reason the chain is asserted here rather
/// than trusted to three matching `format!` calls.
#[test]
fn the_execution_a_step_is_told_it_is_is_the_actor_whose_approval_the_run_refuses() {
    let root = scratch("attested-agreement");
    let run = RunDirectory::at(root.join("runs").join("T-1").join("1"));
    let store = MarkdownStore::open(root.join("planning"));
    let mut fake = Fake::new(&[Act::Pause]);
    let report = drive(
        &engine(),
        &task(),
        &store,
        &map(),
        &run,
        &mut fake,
        &DriverOptions {
            pause_on_approval: true,
            ..DriverOptions::default()
        },
    )
    .expect("a pause is a report");

    let told = fake
        .executions
        .first()
        .cloned()
        .expect("the operator step ran, and was told which execution it belongs to");
    assert_eq!(
        told,
        report.cursor.execution.to_string(),
        "a step is told the execution the cursor records, or the actor a session declares is not \
         this run's"
    );
    let session = session_actor(&report.cursor.execution).expect("the execution spells an actor");
    assert_eq!(session.to_string(), format!("agent:{told}"));

    // The state the rule is load-bearing in: the session's own actor granted the approval, and it
    // is named as the approver besides, so nothing but the self-approval rule can refuse it.
    approve(&run, agent(session.name()), "specification");
    let options = DriverOptions {
        pause_on_approval: true,
        approver: Some(session.clone()),
        ..DriverOptions::default()
    };
    let (resumed_report, asked) = resumed(&root, &run, &options, &[]);

    assert!(asked.is_empty(), "no step ran on a self-approval");
    assert_eq!(resumed_report.status(), RunStatus::AwaitingOperator);
    let note = resumed_report
        .notes
        .iter()
        .find(|note| note.contains("still owed"))
        .expect("the step is still owed");
    assert!(
        note.contains(&session.to_string()) && note.contains("own actor"),
        "the refusal names the same actor the session writes under: {note}"
    );
}

#[test]
fn a_person_is_admitted_without_being_named_and_whoever_else_was_named() {
    // The rule's pure form, asserted here rather than beside it: `attest.rs` is shipped driver
    // code and the evidence scan refuses a `Producer::Human` constructed anywhere in it.
    let person = Producer::Human {
        id: "alice".to_owned(),
    };
    assert_eq!(admit(&person, None, &[]), Admission::Admitted);
    assert_eq!(
        admit(
            &person,
            Some(&actor("agent:orchestrator")),
            &[actor("agent:T-1.1")]
        ),
        Admission::Admitted
    );
}
