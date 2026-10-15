//! Adversarial cases for `concurrent: true` states: where a batch's decisions differ from the ones a
//! serial run of the same steps makes, and where a resume after a batch repeats or misnames work.
//!
//! Driven against a fake harness, as `tests/concurrent.rs` is, so each case is about the
//! deterministic core and not about threads.

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use aep_backend_markdown::MarkdownStore;
use aep_domain::evidence::{Evidence, Producer, TestResult, TestSuite};
use aep_domain::ids::StateId;
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_driver::executor::{
    CommandJob, CommandStepExecutor, LlmStepExecutor, OperatorStepExecutor, StepAuthorizer,
    StepContext, StepOutcome,
};
use aep_driver::run::{drive, DriverOptions, InFlightResolution, RunDirectory, RunReport};
use aep_driver_spec::cursor::RunStatus;
use aep_driver_spec::map::{CommandStep, LlmStep, OperatorStep, StepMap};
use aep_engine::clock::SteppingClock;
use aep_engine::engine::{Engine, EvidenceSubmission};
use aep_engine::registry::Registry;

const PROTOCOL: &str = r"
id: aep
version: 1
title: Test protocol
capabilities: [repository.read, tests.execute]
evidence_kinds: [test_result]
verifiers: [test-runner]
artifact_kinds: [story]
phases: [verification, completion]
observables:
  - 'tests.**'
  - 'evidence.**'
  - 'state.**'
";

const WORKFLOW: &str = r"
id: test/diagnose
version: 1
title: Diagnose
initial: diagnose
states:
  diagnose:
    title: Diagnose
    phases: [verification]
  complete:
    title: Complete
    terminal: true
    phases: [completion]
transitions:
  - from: diagnose
    to: complete
    when: tests.unit.failed == 0
";

const PROFILE: &str = r"
id: test.diagnose
title: Test diagnose
protocol: aep/1
workflow: test/diagnose
capabilities:
  allow: [repository.read, tests.execute]
completion:
  - tests.unit.failed == 0
";

const TASK: &str = r"
id: T-1
kind: feature
objective: collect state at once
protocol: aep/1
profile: test.diagnose
";

fn engine() -> Engine<SteppingClock> {
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
    Engine::with_clock(registry, SteppingClock::new(1_000, 10))
}

fn task() -> Task {
    aep_schema::parse::task(TASK, None).expect("the task parses")
}

fn map_text(concurrent: bool, steps: &str) -> String {
    format!(
        "format: aep.driver-steps/2\nid: test/concurrent\nworkflow: test/diagnose/1\nstates:\n  \
         diagnose:\n    concurrent: {concurrent}\n    steps:\n{steps}"
    )
}

fn map(concurrent: bool, steps: &str) -> StepMap {
    aep_schema::parse::step_map(&map_text(concurrent, steps), Some("test/concurrent.yaml"))
        .expect("the fixture map validates")
}

fn command(suite: &str, extra: &str) -> String {
    format!(
        "      - kind: command\n        run: [{suite}]\n{extra}        evidence:\n          \
         kind: test_result\n          verifier: test-runner\n          suite: {suite}\n"
    )
}

fn diagnose() -> StateId {
    StateId::new("diagnose").expect("a state id")
}

/// A fresh scratch root for one case.
fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary_concurrent")
        .join(name);
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("the previous run's directory is removable");
    }
    std::fs::create_dir_all(&path).expect("a writable scratch directory");
    path
}

#[derive(Debug, Clone, Copy)]
enum Act {
    Pass,
    Crash,
    /// The host dies while the step runs: the driver process goes with it.
    Die,
}

struct Fake {
    script: BTreeMap<(usize, u32), Act>,
    calls: Vec<(usize, u32)>,
}

impl Fake {
    fn new(script: &[((usize, u32), Act)]) -> Self {
        Self {
            script: script.iter().copied().collect(),
            calls: Vec::new(),
        }
    }

    fn outcome(&mut self, step: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        self.calls.push((context.index, context.attempt));
        let suite = step.run[0].as_str().to_owned();
        match self
            .script
            .get(&(context.index, context.attempt))
            .copied()
            .unwrap_or(Act::Pass)
        {
            Act::Pass => StepOutcome::Observed(Box::new(EvidenceSubmission::new(
                Evidence::TestResult(TestResult::passing(TestSuite::Named(suite), 1)),
                Producer::Verifier {
                    verifier: Verifier::TestRunner,
                },
                ObservedAt::new(Timestamp::EPOCH),
            ))),
            Act::Crash => StepOutcome::NoVerdict {
                reason: "killed".to_owned(),
            },
            Act::Die => panic!("the driver was killed mid-batch"),
        }
    }
}

impl CommandStepExecutor for Fake {
    fn run_command(&mut self, step: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        self.outcome(step, context)
    }

    fn run_commands(&mut self, jobs: &[CommandJob<'_>]) -> Vec<StepOutcome> {
        jobs.iter()
            .map(|job| self.outcome(job.step, &job.context))
            .collect()
    }
}

impl LlmStepExecutor for Fake {
    fn run_llm(&mut self, _: &LlmStep, _: &StepContext<'_>, _: StepAuthorizer<'_>) -> StepOutcome {
        StepOutcome::Nothing
    }
}

impl OperatorStepExecutor for Fake {
    fn run_operator(&mut self, _: &OperatorStep, _: &StepContext<'_>) -> StepOutcome {
        StepOutcome::Paused {
            reason: "a person is owed".to_owned(),
        }
    }
}

fn directory(root: &std::path::Path) -> RunDirectory {
    RunDirectory::at(root.join("runs").join("T-1").join("1"))
}

fn go(
    root: &std::path::Path,
    map: &StepMap,
    fake: &mut Fake,
    options: &DriverOptions,
) -> RunReport {
    let store = MarkdownStore::open(root.join("planning"));
    drive(
        &engine(),
        &task(),
        &store,
        map,
        &directory(root),
        fake,
        options,
    )
    .expect("the run ends with a report")
}

fn options() -> DriverOptions {
    DriverOptions {
        pause_on_approval: true,
        ..DriverOptions::default()
    }
}

fn submitted(root: &std::path::Path) -> Vec<String> {
    directory(root)
        .read_snapshot()
        .expect("a snapshot")
        .evidence
        .iter()
        .filter_map(|recorded| match &recorded.record.value {
            Evidence::TestResult(result) => Some(result.suite.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

/// The story's shared-dependency fixture: "This is the result a serial run gives." A serial run
/// reads the breaker *before* the attempt (`Session::step`), so a skipped step is never charged.
/// The batch charges every step up front, so `cursor.json` records an attempt at step 1 that was
/// never made.
#[test]
fn a_step_skipped_by_an_open_circuit_in_a_batch_is_charged_no_attempt_as_in_a_serial_run() {
    let store = "        retries: 1\n        depends_on: store\n        circuit_breaker: 1\n";
    let steps = [
        command("s0", store),
        command("s1", store),
        command("s2", ""),
    ]
    .concat();
    let script = [((0, 1), Act::Crash), ((1, 1), Act::Crash)];

    let serial_root = scratch("circuit-serial");
    let serial = go(
        &serial_root,
        &map(false, &steps),
        &mut Fake::new(&script),
        &options(),
    );
    let batch_root = scratch("circuit-batch");
    let batch = go(
        &batch_root,
        &map(true, &steps),
        &mut Fake::new(&script),
        &options(),
    );

    assert_eq!(
        serial.cursor.attempts_at(&diagnose(), 1),
        0,
        "serially, step 1 is skipped before it is charged"
    );
    assert_eq!(
        batch.cursor.attempts, serial.cursor.attempts,
        "the batch's attempt record differs from the serial one"
    );
}

/// `retries: 0` on step 0 of a shared-dependency lane: step 0 spends its budget and the lane is
/// cleared, so step 1 never runs in this call. It was still charged an attempt, so a resume runs it
/// as attempt 2, writing `diagnose-1-2.log` with no `diagnose-1-1.log` ever existing.
#[test]
fn a_step_left_behind_a_spent_sibling_on_its_lane_is_charged_no_attempt() {
    let store = "        retries: 0\n        depends_on: store\n";
    let steps = [command("s0", store), command("s1", store)].concat();
    let root = scratch("lane-spent");
    let mut fake = Fake::new(&[((0, 1), Act::Crash)]);
    let report = go(&root, &map(true, &steps), &mut fake, &options());
    assert_eq!(report.status(), RunStatus::BudgetExhausted);
    assert_eq!(
        fake.calls,
        [(0, 1)],
        "step 1 waited behind step 0 and never ran"
    );
    assert_eq!(
        report.cursor.attempts_at(&diagnose(), 1),
        0,
        "step 1 never ran, yet the cursor records an attempt at it"
    );
}

/// The story's "Budget spent in a batch" fixture, then `aep drive resume` (which continues a
/// budget-exhausted run). The batch already submitted step 1's verdict and left the cursor at step
/// 0, so the resume reruns the whole batch: step 1 runs a second time and its evidence is
/// submitted twice. A serial run submits each step's verdict once over the same two calls.
#[test]
fn resuming_after_a_batch_spent_a_budget_does_not_rerun_a_sibling_already_submitted() {
    let steps = [command("s0", "        retries: 0\n"), command("s1", "")].concat();
    let root = scratch("resume-after-budget");
    let concurrent = map(true, &steps);
    let mut first = Fake::new(&[((0, 1), Act::Crash)]);
    let report = go(&root, &concurrent, &mut first, &options());
    assert_eq!(report.status(), RunStatus::BudgetExhausted);
    assert_eq!(submitted(&root), ["s1"]);

    let mut second = Fake::new(&[]);
    let _resumed = go(&root, &concurrent, &mut second, &options());
    let s1_runs = first
        .calls
        .iter()
        .chain(&second.calls)
        .filter(|(index, _)| *index == 1)
        .count();
    assert_eq!(
        (s1_runs, submitted(&root)),
        (1, vec!["s1".to_owned(), "s0".to_owned()]),
        "step 1 ran and was submitted again on resume"
    );
}

/// A retry of the batch's first step, then the driver dies during that retry. The in-flight marker
/// still names attempt 1, which had already ended with no verdict; attempt 2 is the one that was in
/// flight. `--retry-in-flight <id of attempt 1>` then reruns attempt 2: the id the operator
/// authorises does not name the attempt that runs, which a serial retry guarantees
/// (`Session::begin_attempt` reuses `in_flight.attempt`).
#[test]
fn the_in_flight_marker_names_the_attempt_of_the_first_step_that_was_actually_in_flight() {
    let steps = [command("s0", "        retries: 1\n"), command("s1", "")].concat();
    let root = scratch("marker-after-retry");
    let concurrent = map(true, &steps);
    let mut dying = Fake::new(&[((0, 1), Act::Crash), ((0, 2), Act::Die)]);
    let died = catch_unwind(AssertUnwindSafe(|| {
        go(&root, &concurrent, &mut dying, &options())
    }));
    assert!(died.is_err(), "the fake host killed the driver");
    assert_eq!(dying.calls, [(0, 1), (1, 1), (0, 2)]);

    let cursor = directory(&root).read_cursor().expect("a cursor");
    let marker = cursor.in_flight.clone().expect("the batch is in flight");
    assert_eq!(marker.step, 0);
    assert_eq!(
        marker.attempt,
        cursor.attempts_at(&diagnose(), 0),
        "the marker `{}` names an attempt that had already ended",
        marker.id
    );

    let mut retry = Fake::new(&[]);
    let _resumed = go(
        &root,
        &concurrent,
        &mut retry,
        &DriverOptions {
            in_flight_resolution: Some(InFlightResolution::Retry(marker.id.clone())),
            ..options()
        },
    );
    assert_eq!(
        retry.calls[0],
        (0, marker.attempt),
        "the retried attempt is the one the id names"
    );
}

/// Guard (passes today): the shared-output refusal compares in any ASCII case, as the loader
/// documents, so `a.txt` and `A.txt` in one concurrent state are refused. A mutant comparing with
/// `==` passes every case the unit wrote.
#[test]
fn two_outputs_differing_only_in_case_are_one_file_in_a_concurrent_state() {
    let steps = "      - kind: command\n        run: [one]\n        output: a.txt\n      \
                 - kind: command\n        run: [two]\n        output: A.txt\n";
    let errors = aep_schema::parse::step_map(&map_text(true, steps), Some("test/concurrent.yaml"))
        .expect_err("a.txt and A.txt are one file on a case-insensitive file system");
    let text = format!("{errors} {errors:?}");
    assert!(
        text.contains("unsupported_construct") || text.contains("UnsupportedConstruct"),
        "{text}"
    );
    assert!(
        text.contains("steps[0]") && text.contains("steps[1]"),
        "{text}"
    );
    aep_schema::parse::step_map(&map_text(false, steps), Some("test/concurrent.yaml"))
        .expect("serially the later write wins");
}
