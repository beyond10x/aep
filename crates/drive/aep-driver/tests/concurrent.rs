//! A `concurrent: true` state, driven against a fake harness: which steps the driver hands over as
//! one batch, which attempts it charges, and in which order it submits what came back.
//!
//! The CLI host runs a batch on threads, and `aep-cli`'s `drive_cli` tests time it. These tests
//! hold the deterministic half: the core decides the batch, charges its attempts, applies the
//! circuit breaker and the retry budget as a serial run would, and submits in map order whatever
//! order the host finished in.

use std::collections::BTreeMap;
use std::path::PathBuf;

use aep_backend_markdown::MarkdownStore;
use aep_domain::evidence::{Evidence, Producer, TestResult, TestSuite};
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_driver::executor::{
    CommandJob, CommandStepExecutor, LlmStepExecutor, OperatorStepExecutor, StepAuthorizer,
    StepContext, StepOutcome,
};
use aep_driver::run::{drive, DriverOptions, RunDirectory, RunReport};
use aep_driver_spec::cursor::{InFlightAttempt, RunStatus};
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

/// A map whose `diagnose` state holds `steps`, each a YAML list item indented for the list.
fn map(concurrent: bool, steps: &str) -> StepMap {
    let text = format!(
        "format: aep.driver-steps/2\nid: test/concurrent\nworkflow: test/diagnose/1\nstates:\n  \
         diagnose:\n    concurrent: {concurrent}\n    steps:\n{steps}"
    );
    aep_schema::parse::step_map(&text, Some("test/concurrent.yaml"))
        .expect("the fixture map validates")
}

/// One command step submitting a `test_result` for `suite`.
fn command(suite: &str, extra: &str) -> String {
    format!(
        "      - kind: command\n        run: [{suite}]\n{extra}        evidence:\n          \
         kind: test_result\n          verifier: test-runner\n          suite: {suite}\n"
    )
}

/// A scratch directory under this test binary's target directory.
fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("concurrent")
        .join(name);
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("the previous run's directory is removable");
    }
    std::fs::create_dir_all(&path).expect("a writable scratch directory");
    path
}

/// What the fake does with one attempt.
#[derive(Debug, Clone, Copy)]
enum Act {
    Pass,
    Fail,
    Crash,
}

/// A harness that records every call the driver made, batch by batch.
struct Fake {
    run: RunDirectory,
    /// What each `(index, attempt)` does; anything unlisted passes.
    script: BTreeMap<(usize, u32), Act>,
    /// Every call, as the `(index, attempt)` pairs it was handed: one entry per batch, and one per
    /// serial `run_command` or `run_llm` call.
    calls: Vec<Vec<(usize, u32)>>,
    /// The cursor's in-flight marker, read from disk as each batch was handed over.
    in_flight: Vec<Option<InFlightAttempt>>,
}

impl Fake {
    fn new(run: &RunDirectory, script: &[((usize, u32), Act)]) -> Self {
        Self {
            run: run.clone(),
            script: script.iter().copied().collect(),
            calls: Vec::new(),
            in_flight: Vec::new(),
        }
    }

    fn outcome(&self, step: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        let suite = step.run[0].as_str().to_owned();
        let act = self
            .script
            .get(&(context.index, context.attempt))
            .copied()
            .unwrap_or(Act::Pass);
        let result = match act {
            Act::Pass => TestResult::passing(TestSuite::Named(suite), 1),
            Act::Fail => TestResult::failing(TestSuite::Named(suite), 0, 1),
            Act::Crash => {
                return StepOutcome::NoVerdict {
                    reason: "killed".to_owned(),
                }
            }
        };
        StepOutcome::Observed(Box::new(EvidenceSubmission::new(
            Evidence::TestResult(result),
            Producer::Verifier {
                verifier: Verifier::TestRunner,
            },
            ObservedAt::new(Timestamp::EPOCH),
        )))
    }
}

impl CommandStepExecutor for Fake {
    fn run_command(&mut self, step: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        self.calls.push(vec![(context.index, context.attempt)]);
        self.outcome(step, context)
    }

    fn run_commands(&mut self, jobs: &[CommandJob<'_>]) -> Vec<StepOutcome> {
        self.calls.push(
            jobs.iter()
                .map(|job| (job.context.index, job.context.attempt))
                .collect(),
        );
        self.in_flight
            .push(self.run.read_cursor().expect("a cursor").in_flight);
        // Answered last to first, as a host whose first step was slowest would: the driver must
        // not care.
        let mut outcomes: Vec<StepOutcome> = jobs
            .iter()
            .rev()
            .map(|job| self.outcome(job.step, &job.context))
            .collect();
        outcomes.reverse();
        outcomes
    }
}

impl LlmStepExecutor for Fake {
    fn run_llm(
        &mut self,
        _: &LlmStep,
        context: &StepContext<'_>,
        _: StepAuthorizer<'_>,
    ) -> StepOutcome {
        self.calls.push(vec![(context.index, context.attempt)]);
        StepOutcome::Nothing
    }
}

impl OperatorStepExecutor for Fake {
    fn run_operator(&mut self, _: &OperatorStep, context: &StepContext<'_>) -> StepOutcome {
        self.calls.push(vec![(context.index, context.attempt)]);
        StepOutcome::Paused {
            reason: "a person is owed".to_owned(),
        }
    }
}

/// Drives `map` with `fake`, returning the report.
fn run(name: &str, map: &StepMap, script: &[((usize, u32), Act)]) -> (RunReport, Fake) {
    let root = scratch(name);
    let store = MarkdownStore::open(root.join("planning"));
    let directory = RunDirectory::at(root.join("runs").join("T-1").join("1"));
    let mut fake = Fake::new(&directory, script);
    let report = drive(
        &engine(),
        &task(),
        &store,
        map,
        &directory,
        &mut fake,
        &DriverOptions {
            pause_on_approval: true,
            ..DriverOptions::default()
        },
    )
    .expect("the run ends with a report");
    (report, fake)
}

/// The suites the run submitted, in the order the engine recorded them.
fn submitted(name: &str) -> Vec<String> {
    let directory = RunDirectory::at(
        PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("concurrent")
            .join(name)
            .join("runs/T-1/1"),
    );
    let snapshot = directory.read_snapshot().expect("a snapshot");
    snapshot
        .evidence
        .iter()
        .filter_map(|recorded| match &recorded.record.value {
            Evidence::TestResult(result) => Some(format!(
                "{}:{}",
                result.suite.as_str(),
                if result.failed == 0 { "pass" } else { "fail" }
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn a_concurrent_state_hands_its_command_steps_over_as_one_batch_and_submits_in_map_order() {
    let steps = [command("s3", ""), command("s1", ""), command("s2", "")].concat();
    let (report, fake) = run("batch", &map(true, &steps), &[]);
    assert_eq!(
        fake.calls,
        [vec![(0, 1), (1, 1), (2, 1)]],
        "one batch, one attempt charged to each step"
    );
    let in_flight = fake.in_flight[0].as_ref().expect("the batch is in flight");
    assert_eq!(
        (in_flight.step, in_flight.attempt),
        (0, 1),
        "named by its first step"
    );
    assert_eq!(submitted("batch"), ["s3:pass", "s1:pass", "s2:pass"]);
    assert_eq!(report.evidence_submitted, 3);
    assert_eq!(report.steps_run, 3);
    assert!(report.cursor.in_flight.is_none(), "the batch ended");

    let (_, serial) = run("serial", &map(false, &steps), &[]);
    assert_eq!(
        serial.calls,
        [vec![(0, 1)], vec![(1, 1)], vec![(2, 1)]],
        "a state without the key runs one step at a time"
    );
    assert_eq!(submitted("serial"), submitted("batch"));
}

#[test]
fn an_llm_or_operator_step_is_a_barrier_between_batches() {
    let llm = "      - kind: llm\n        prompt: read what was collected\n        judgement: \
               which line of the logs matters is a reading no command performs\n";
    let steps = [
        command("a", ""),
        command("b", ""),
        llm.to_owned(),
        command("c", ""),
        command("d", ""),
    ]
    .concat();
    let (_, fake) = run("llm-barrier", &map(true, &steps), &[]);
    assert_eq!(
        fake.calls,
        [vec![(0, 1), (1, 1)], vec![(2, 1)], vec![(3, 1), (4, 1)]]
    );

    let operator = "      - kind: operator\n        prompt: look at it\n";
    let steps = [command("a", ""), operator.to_owned(), command("c", "")].concat();
    let (report, fake) = run("operator-barrier", &map(true, &steps), &[]);
    assert_eq!(report.status(), RunStatus::AwaitingOperator);
    assert_eq!(
        fake.calls,
        [vec![(0, 1)], vec![(1, 1)]],
        "c waits for the person"
    );
}

#[test]
fn a_retry_inside_a_batch_advances_only_that_steps_attempt() {
    let steps = [command("s0", "        retries: 1\n"), command("s1", "")].concat();
    let (report, fake) = run("retry", &map(true, &steps), &[((0, 1), Act::Crash)]);
    assert_eq!(fake.calls, [vec![(0, 1), (1, 1)], vec![(0, 2)]]);
    assert_eq!(submitted("retry"), ["s0:pass", "s1:pass"]);
    assert_eq!(report.steps_run, 3);
}

#[test]
fn a_step_that_spends_its_retries_stops_the_run_after_its_siblings_are_submitted() {
    let steps = [command("s0", "        retries: 0\n"), command("s1", "")].concat();
    let (report, _) = run("budget", &map(true, &steps), &[((0, 1), Act::Crash)]);
    assert_eq!(report.status(), RunStatus::BudgetExhausted);
    assert_eq!(submitted("budget"), ["s1:pass"]);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("step 0 of `diagnose` has spent its command retry budget")),
        "{:?}",
        report.notes
    );
}

#[test]
fn a_failing_step_does_not_cancel_its_siblings() {
    let steps = [command("s0", ""), command("s1", "")].concat();
    let (_, fake) = run("no-cancel", &map(true, &steps), &[((0, 1), Act::Fail)]);
    assert_eq!(fake.calls, [vec![(0, 1), (1, 1)]]);
    assert_eq!(submitted("no-cancel"), ["s0:fail", "s1:pass"]);
}

#[test]
fn steps_sharing_a_dependency_run_one_after_another_and_the_breaker_reads_a_serial_count() {
    let store = "        retries: 1\n        depends_on: store\n        circuit_breaker: 1\n";
    let steps = [
        command("s0", store),
        command("s1", store),
        command("s2", ""),
    ]
    .concat();
    let (report, fake) = run(
        "shared-dependency",
        &map(true, &steps),
        &[((0, 1), Act::Crash), ((1, 1), Act::Crash)],
    );
    assert_eq!(
        fake.calls,
        [vec![(0, 1), (2, 1)]],
        "step 1 waits behind step 0, and the breaker step 0 opened skips both after it"
    );
    assert_eq!(submitted("shared-dependency"), ["s2:pass"]);
    assert!(
        report.notes.iter().any(|note| note
            .contains("step 1 of `diagnose` (s1) was not attempted: `store` has failed 1 time(s)")),
        "{:?}",
        report.notes
    );
}
