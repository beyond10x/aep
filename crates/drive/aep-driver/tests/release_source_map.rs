//! `story:release-step-map`: this repository's release is cut by the shipped step map
//! `drivers/releases/source.yaml`, which has no model step.
//!
//! The map is driven against the real document tree (`release.source` under `aop/1`, workflow
//! `release/source/1`) with a stub for every command: it records each argv, answers an exit
//! status, mints a `test_result` or a `diff` from it the way `aep-cli` does, and writes the `verification`
//! record a claim-writer step's `record:` names — the record the later
//! `story:release-verification-claims` command writes for real. Nothing here runs Git, a gate, a
//! push or a forge call.

use std::path::{Path, PathBuf};

use aep_backend_markdown::MarkdownStore;
use aep_domain::artifact::ArtifactGraph;
use aep_domain::evidence::{
    ApprovalDecision, ApprovalRecord, ChangeSet, Evidence, EvidenceKind, Producer, TestResult,
    TestSuite,
};
use aep_domain::ids::{ApprovalId, StateId};
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_driver::executor::{
    CommandStepExecutor, LlmStepExecutor, OperatorStepExecutor, StepAuthorizer, StepContext,
    StepOutcome,
};
use aep_driver::run::{drive, resume, DriveError, DriverOptions, RunDirectory, RunReport};
use aep_driver_spec::cursor::RunStatus;
use aep_driver_spec::map::{
    CommandStep, LlmStep, OperatorStep, Step, StepKind, StepMap, DEFAULT_COMMAND_RETRIES,
};
use aep_engine::clock::SteppingClock;
use aep_engine::engine::{Engine, EvidenceSubmission, ProtocolEngine};
use aep_engine::registry::Registry;
use aep_project::load_tree_report;

const VERSION: &str = "9.9.9";
const DATE: &str = "2026-01-02";

const TASK: &str = "id: R-1\nkind: release\nobjective: cut this repository's release\n\
                    protocol: aop/1\nprofile: release.source\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn registry() -> Registry {
    load_tree_report(&root())
        .into_result()
        .expect("the document tree, the release map included, is valid")
}

fn engine() -> Engine<SteppingClock> {
    Engine::with_clock(registry(), SteppingClock::new(1_000, 10))
}

fn task() -> Task {
    aep_schema::parse::task(TASK, None).expect("the release task parses")
}

fn map() -> StepMap {
    let path = root().join("drivers/releases/source.yaml");
    let text = std::fs::read_to_string(&path).expect("the release map is shipped");
    aep_schema::parse::step_map(&text, Some("drivers/releases/source.yaml"))
        .expect("the release map validates")
}

fn inputs(given: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    given
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn options(given: &[(&str, &str)]) -> DriverOptions {
    DriverOptions {
        pause_on_approval: true,
        inputs: inputs(given),
        ..DriverOptions::default()
    }
}

fn both() -> DriverOptions {
    options(&[("version", VERSION), ("date", DATE)])
}

fn scratch(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("the previous run's directory is removable");
    }
    std::fs::create_dir_all(&path).expect("a writable scratch directory");
    path
}

/// The argv of every command step the story's table lists, in table order, numbered as there.
///
/// `{v}` is the version, `{d}` the date and `{run}` the run directory. Step 25 is the one the
/// story's table does not number: the release's diff against the previous tag, which
/// `provenance-tracking` owes a `diff` record for. It runs in `verify`, between steps 16 and 17.
const TABLE: &[(u32, &str)] = &[
    (1, "b10x-gates bot -- fetch origin main --tags"),
    (2, "cargo xtask release check release-on-default-branch --version {v} --out {run}/claim-release-on-default-branch.yaml"),
    (3, "git switch -c release/{v} origin/main"),
    (4, "cargo xtask release prepare {v} --date {d}"),
    (5, "b10x-gates bot -- commit -a -m|chore: release {v}|-m|Release {v}."),
    (6, "cargo xtask release tag"),
    (7, "cargo xtask release pin"),
    (8, "cargo xtask release check release-inputs --version {v} --date {d} --out {run}/claim-release-inputs.yaml"),
    (9, "cargo build --release -p aep-cli"),
    (10, "env AEP_BIN=target/release/aep task check"),
    (12, "b10x-gates bot -- push origin release/{v} {v}"),
    (14, "cargo xtask release check release-tag-pushed --version {v} --out {run}/claim-release-tag-pushed.yaml"),
    (15, "b10x-gates bot -- fetch origin main --tags"),
    (16, "sh -c|gh run watch --exit-status \"$(gh run list --workflow release.yml --branch {v} --limit 1 --json databaseId --jq \".[0].databaseId\")\""),
    (17, "git switch -c plan/release-{v}-record origin/main"),
    (18, "cargo xtask release record"),
    (19, "b10x-gates bot -- commit -a -m|plan: record the {v} release|-m|Release {v}."),
    (20, "cargo xtask status"),
    (21, "b10x-gates bot -- commit -a -m|docs: move the release pins to {v}|-m|Release {v}."),
    (22, "b10x-gates bot -- push origin plan/release-{v}-record"),
    (24, "cargo xtask release check source-release --version {v} --out {run}/claim-source-release.yaml"),
    (25, "sh -c|git --no-pager diff --stat \"$(git describe --tags --abbrev=0 {v}^)\" {v}"),
];

/// Every step of the map, in its order: the table's 24 and step 25.
const STEPS: usize = 25;

/// The table's steps that change Git state, a ref or the planning store, and so declare
/// `retries: 0`.
const NO_RETRY: &[u32] = &[1, 3, 4, 5, 6, 7, 12, 15, 17, 18, 19, 20, 21, 22];

/// One table row as an argv: words split on spaces, except that a row holding `|` is split on
/// `|` after its first segment's spaces, so a word with spaces in it stays one word.
fn argv(row: &str, run: &Path) -> Vec<String> {
    let filled = row
        .replace("{v}", VERSION)
        .replace("{d}", DATE)
        .replace("{run}", &run.display().to_string());
    let mut segments = filled.split('|');
    let mut words: Vec<String> = segments
        .next()
        .unwrap_or_default()
        .split(' ')
        .map(str::to_owned)
        .collect();
    words.extend(segments.map(str::to_owned));
    words
}

fn table(steps: &[u32], run: &Path) -> Vec<Vec<String>> {
    steps
        .iter()
        .map(|number| {
            let (_, row) = TABLE
                .iter()
                .find(|(n, _)| n == number)
                .expect("a command step of the table");
            argv(row, run)
        })
        .collect()
}

/// The table's order of states, and how many of its numbered steps each holds.
const STATES: &[(&str, &[u32])] = &[
    ("inspect", &[1, 2]),
    ("qualify", &[3, 4, 5, 6, 7, 8, 9, 10]),
    ("await_approval", &[11]),
    ("publish", &[12, 13, 14]),
    ("verify", &[15, 16, 25, 17, 18, 19, 20, 21, 22, 23, 24]),
];

/// The stub: records every argv, answers exit statuses, and writes what a claim writer writes.
#[derive(Default)]
struct Stub {
    /// Every argv a command step was run with, placeholders filled.
    ran: Vec<Vec<String>>,
    /// Every operator step the run stopped at, as `(state, index)`.
    stopped: Vec<(String, usize)>,
    /// The argv answered with exit 1; every other exits 0.
    failing: Option<Vec<String>>,
}

impl Stub {
    fn filled(step: &CommandStep, context: &StepContext<'_>) -> Vec<String> {
        step.run
            .iter()
            .map(|word| {
                word.fill(|name| match name {
                    "run_directory" => Ok(Some(context.run_directory.display().to_string())),
                    other => Err(format!("the release map names no `{{{other}}}`")),
                })
                .expect("the release map names only the run directory")
                .as_str()
                .to_owned()
            })
            .collect()
    }
}

impl CommandStepExecutor for Stub {
    fn run_command(&mut self, step: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        let argv = Self::filled(step, context);
        self.ran.push(argv.clone());
        let passed = self.failing.as_ref() != Some(&argv);
        let Some(mapping) = &step.evidence else {
            return if passed {
                StepOutcome::Nothing
            } else {
                StepOutcome::NoVerdict {
                    reason: format!("`{}` exited 1", argv.join(" ")),
                }
            };
        };
        if let Some(record) = &mapping.record {
            // The claim writer's part: the claim is the word after `check`, and the record goes
            // where `record:` says, which the driver then reads as it reads any written record.
            let claim = argv
                .iter()
                .skip_while(|word| *word != "check")
                .nth(1)
                .expect("a claim-writer step names its claim after `check`");
            let path = record
                .fill(|name| match name {
                    "run_directory" => Ok(Some(context.run_directory.display().to_string())),
                    other => Err(other.to_owned()),
                })
                .expect("the record path names only the run directory")
                .as_str()
                .to_owned();
            let status = if passed { "passed" } else { "failed" };
            let document = format!(
                "- kind: verification\n  observed_at: 1970-01-01\n  claim: {claim}\n  verifier: {}\n  \
                 status: {status}\n  producer:\n    producer: verifier\n    verifier: {}\n",
                mapping.verifier.as_str(),
                mapping.verifier.as_str(),
            );
            std::fs::write(&path, document).expect("the run directory is writable");
            let text = std::fs::read_to_string(&path).expect("the record was written");
            let mut read = aep_schema::parse::evidence_list(&text, Some(&path))
                .expect("the claim record reads");
            assert_eq!(read.len(), 1, "one record per claim");
            let input = read.remove(0);
            assert_eq!(input.evidence.kind(), EvidenceKind::Verification);
            let mut submission =
                EvidenceSubmission::new(input.evidence, input.producer, input.observed_at);
            submission.subject = input.about;
            return StepOutcome::Observed(Box::new(submission));
        }
        // Minted as `aep-cli`'s `mint` does: a `diff` only from a command that exited 0, with no
        // counts, because an exit status carries none.
        let evidence = match mapping.kind {
            EvidenceKind::TestResult => {
                let suite = mapping.suite.clone().unwrap_or(TestSuite::Unit);
                Evidence::TestResult(if passed {
                    TestResult::passing(suite, 1)
                } else {
                    TestResult::failing(suite, 0, 1)
                })
            }
            EvidenceKind::Diff if passed => Evidence::Diff(ChangeSet {
                files_changed: 0,
                lines_added: 0,
                lines_removed: 0,
                revision_before: None,
                revision_after: None,
                paths: Vec::new(),
            }),
            EvidenceKind::Diff => {
                return StepOutcome::NoVerdict {
                    reason: format!("`{}` exited 1", argv.join(" ")),
                }
            }
            other => panic!("the release map mints only `test_result` and `diff`, not {other:?}"),
        };
        StepOutcome::Observed(Box::new(EvidenceSubmission::new(
            evidence,
            Producer::Verifier {
                verifier: mapping.verifier.clone(),
            },
            ObservedAt::new(Timestamp::EPOCH),
        )))
    }
}

impl OperatorStepExecutor for Stub {
    fn run_operator(&mut self, _: &OperatorStep, context: &StepContext<'_>) -> StepOutcome {
        self.stopped
            .push((context.state.to_string(), context.index));
        StepOutcome::Paused {
            reason: "a person is owed an answer".to_owned(),
        }
    }
}

impl LlmStepExecutor for Stub {
    fn run_llm(&mut self, _: &LlmStep, _: &StepContext<'_>, _: StepAuthorizer<'_>) -> StepOutcome {
        panic!("the release map has no model step, and the driver ran one")
    }
}

/// Records a person's granted approval into the stopped run's snapshot, as `tests/attested.rs`
/// `approve` does.
fn approve(run: &RunDirectory, person: &str, approval: &str) {
    let engine = engine();
    let snapshot = run.read_snapshot().expect("the stopped run has a snapshot");
    let cursor = run.read_cursor().expect("and a cursor");
    let mut execution = engine
        .restore(task(), ArtifactGraph::new(), snapshot)
        .expect("the snapshot restores");
    let approver = Producer::Human {
        id: person.to_owned(),
    };
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

fn start(root: &Path, stub: &mut Stub, options: &DriverOptions) -> (RunDirectory, RunReport) {
    let run = RunDirectory::at(root.join("runs").join("R-1").join("1"));
    let store = MarkdownStore::open(root.join("planning"));
    let report = drive(&engine(), &task(), &store, &map(), &run, stub, options)
        .expect("the run reports where it stopped");
    (run, report)
}

fn again(root: &Path, run: &RunDirectory, stub: &mut Stub) -> RunReport {
    let store = MarkdownStore::open(root.join("planning"));
    resume(&engine(), &task(), &store, &map(), run, stub, &both()).expect("the run resumes")
}

fn state(name: &str) -> StateId {
    StateId::new(name).expect("a state id")
}

fn moves(report: &RunReport) -> Vec<(String, String)> {
    report
        .transitions
        .iter()
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect()
}

#[test]
fn the_release_map_validates_against_release_source_and_has_no_model_step() {
    let outcome = load_tree_report(&root());
    assert!(
        outcome.failures.is_empty(),
        "the tree, the release map cross-validated in it, loads clean: {:#?}",
        outcome.failures
    );
    let shipped = outcome
        .drivers
        .iter()
        .find(|map| map.id.as_str() == "releases/source")
        .expect("the tree ships `releases/source`");
    assert_eq!(shipped.workflow.to_string(), "release/source/1");

    let plan = aep_engine::resolve(&task(), &outcome.registry).expect("release.source resolves");
    let errors = shipped.check_run(&plan.protocol, &plan.workflow);
    assert!(errors.is_empty(), "the map fits the run's plan: {errors}");

    let kinds: Vec<StepKind> = shipped
        .states
        .values()
        .flat_map(|state| state.steps.iter().map(Step::kind))
        .collect();
    assert_eq!(
        kinds.len(),
        STEPS,
        "the table's 24 steps and the diff: {kinds:?}"
    );
    assert!(
        !kinds.contains(&StepKind::Llm),
        "no `kind: llm` step: {kinds:?}"
    );

    // `provenance-tracking` owes one `diff` before `complete`; the map mints it from a command.
    let diffs: Vec<&CommandStep> = shipped
        .states
        .values()
        .flat_map(|state| state.steps.iter())
        .filter_map(|step| match step {
            Step::Command(command) => Some(command),
            _ => None,
        })
        .filter(|command| {
            command
                .evidence
                .as_ref()
                .is_some_and(|mapping| mapping.kind == EvidenceKind::Diff)
        })
        .collect();
    assert_eq!(diffs.len(), 1, "one step mints the release's diff");
    assert_eq!(
        diffs[0]
            .evidence
            .as_ref()
            .map(|mapping| mapping.verifier.as_str()),
        Some("git"),
        "and git, not a model, produces it"
    );
}

#[test]
fn every_step_that_changes_git_a_ref_or_the_store_declares_no_retry_and_the_rest_keep_the_default()
{
    let map = map();
    let mut count = 0;
    for (name, numbers) in STATES {
        let steps = map.steps_for(&state(name));
        assert_eq!(
            steps.len(),
            numbers.len(),
            "`{name}` holds the table's steps {numbers:?}"
        );
        for (&number, step) in numbers.iter().zip(steps) {
            count += 1;
            let Step::Command(command) = step else {
                continue;
            };
            let expected = if NO_RETRY.contains(&number) {
                0
            } else {
                DEFAULT_COMMAND_RETRIES
            };
            assert_eq!(
                command.retries,
                expected,
                "step {number} ({}) declares retries {expected}",
                command.command_line()
            );
        }
    }
    assert_eq!(count, STEPS, "the table's 24 steps and the diff");
}

#[test]
fn a_run_missing_its_version_or_its_date_is_refused_before_any_step_naming_the_input() {
    for (given, missing) in [
        (&[("date", DATE)][..], "{input.version}"),
        (&[("version", VERSION)][..], "{input.date}"),
    ] {
        let root = scratch(&format!("release-missing-{}", given[0].0));
        let mut stub = Stub::default();
        let run = RunDirectory::at(root.join("runs").join("R-1").join("1"));
        let store = MarkdownStore::open(root.join("planning"));
        let refused = drive(
            &engine(),
            &task(),
            &store,
            &map(),
            &run,
            &mut stub,
            &options(given),
        )
        .expect_err("the run is missing an input");
        let DriveError::Validation(errors) = &refused else {
            panic!("a missing input is a refusal of the map against the run: {refused}");
        };
        assert!(
            errors
                .as_slice()
                .iter()
                .all(|error| error.to_string().contains(missing)),
            "every refusal names {missing}: {errors}"
        );
        assert!(!errors.is_empty());
        assert!(
            stub.ran.is_empty() && stub.stopped.is_empty(),
            "no step ran"
        );
        assert!(!run.has_cursor(), "nothing was persisted");
    }
}

#[test]
fn a_release_runs_the_tables_commands_in_order_and_stops_for_a_person_at_each_operator_step() {
    let root = scratch("release-whole");
    let mut stub = Stub::default();

    // Up to the approval: steps 1-10, and nothing of step 12.
    let (run, report) = start(&root, &mut stub, &both());
    let dir = run.path().to_path_buf();
    assert_eq!(
        report.status(),
        RunStatus::AwaitingOperator,
        "{:?}",
        report.notes
    );
    assert_eq!(report.cursor.state, state("await_approval"));
    assert_eq!(stub.ran, table(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], &dir));
    assert_eq!(stub.stopped, [("await_approval".to_owned(), 0)]);

    // A person grants `release-create` while the run is stopped; the resume moves to `publish`
    // and stops again at the PR step, without asking the approval twice.
    approve(&run, "release-manager", "release-create");
    let report = again(&root, &run, &mut stub);
    assert!(
        moves(&report).contains(&("await_approval".to_owned(), "publish".to_owned())),
        "the approval moved the run: {:?} {:?}",
        moves(&report),
        report.reasons
    );
    let answer = report
        .cursor
        .answers
        .iter()
        .find(|answer| answer.approval == "release-create")
        .expect("the cursor says who answered the approval");
    assert_eq!(answer.by, "human release-manager");
    assert_eq!(report.status(), RunStatus::AwaitingOperator);
    assert_eq!(report.cursor.state, state("publish"));
    assert_eq!(
        stub.stopped.last(),
        Some(&("publish".to_owned(), 1)),
        "step 13 stops the run"
    );
    assert_eq!(stub.ran, table(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12], &dir));

    // Resumed after the release PR: step 14 to step 22, the diff between 16 and 17, then the
    // record PR stops it.
    let report = again(&root, &run, &mut stub);
    assert_eq!(
        report.status(),
        RunStatus::AwaitingOperator,
        "{:?}",
        report.notes
    );
    assert_eq!(report.cursor.state, state("verify"));
    assert_eq!(
        stub.stopped.last(),
        Some(&("verify".to_owned(), 9)),
        "step 23"
    );

    // Resumed after the record PR: step 24, and the run is complete.
    let report = again(&root, &run, &mut stub);
    assert_eq!(
        stub.ran,
        table(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 15, 16, 25, 17, 18, 19, 20, 21, 22, 24],
            &dir
        ),
        "one argv per command step, in the map's order, inputs filled in"
    );
    assert_eq!(
        stub.stopped,
        [
            ("await_approval".to_owned(), 0),
            ("publish".to_owned(), 1),
            ("verify".to_owned(), 9),
        ]
    );
    assert!(
        stub.ran
            .iter()
            .flatten()
            .all(|word| !word.contains("/pulls")),
        "the driver issues no forge write: {:?}",
        stub.ran
    );
    assert_eq!(
        report.cursor.llm_steps_run,
        Some(0),
        "no model step attempt"
    );
    assert_eq!(report.llm_steps_run, 0);
    assert_eq!(
        report.status(),
        RunStatus::Completed,
        "{:?} {:?}",
        report.notes,
        report.reasons
    );
    assert_eq!(report.cursor.state, state("complete"));
}

#[test]
fn a_failing_gate_blocks_the_run_in_qualify_before_anything_reaches_the_remote() {
    let root = scratch("release-gate-red");
    let gate = table(&[10], Path::new("")).remove(0);
    let mut stub = Stub {
        failing: Some(gate.clone()),
        ..Stub::default()
    };

    let (run, report) = start(&root, &mut stub, &both());

    assert_eq!(report.status(), RunStatus::Blocked, "{:?}", report.notes);
    assert_eq!(report.cursor.state, state("qualify"));
    assert!(
        report
            .transitions
            .iter()
            .all(|(_, to)| to.as_str() != "publish" && to.as_str() != "await_approval"),
        "`publish` is never entered: {:?}",
        moves(&report)
    );
    assert!(
        stub.stopped.is_empty(),
        "nobody is asked to approve a red gate"
    );
    assert_eq!(
        stub.ran.last(),
        Some(&gate),
        "the gate is the last thing run: {:?}",
        stub.ran
    );
    assert!(
        stub.ran.contains(&table(&[6], run.path()).remove(0)),
        "the local tag was made before the gate"
    );
    assert!(
        stub.ran
            .iter()
            .all(|argv| !argv.contains(&"push".to_owned())),
        "no push of the branch or the tag: {:?}",
        stub.ran
    );
}
