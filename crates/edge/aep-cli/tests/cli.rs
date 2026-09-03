//! CLI integration tests.
//!
//! These drive the real binary, because the interface is the product here: a harness shells out to
//! `protocol` and reads its exit code. Testing the library instead would not catch an argument that
//! never reaches it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args`, always against the repository's own document tree.
fn protocol(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

/// Standard output as a string.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Standard error as a string.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The exit code, which is part of the contract with a calling harness.
fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

/// A fixture path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to build a fixture in.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

const TASK: &str = "examples/development-passkeys/task.yaml";
const ARTIFACTS: &str = "examples/development-passkeys/artifacts.yaml";

#[test]
fn validate_accepts_the_repositorys_own_documents() {
    let output = protocol(&["validate"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("valid"), "{text}");
    assert!(text.contains("protocol(s)"), "{text}");
}

#[test]
fn validate_reports_a_broken_document_with_its_path_and_fails() {
    let directory = std::env::temp_dir().join("aep-cli-broken-tree/workflows");
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    let file = directory.join("broken.yaml");
    std::fs::write(
        &file,
        "id: broken\ntitle: Broken\ninitial: nowhere\nstates:\n  a:\n    title: A\n    terminal: true\n",
    )
    .expect("the fixture is writable");

    let output = protocol(&[
        "validate",
        "--root",
        directory
            .parent()
            .expect("the tree root")
            .to_str()
            .expect("a printable path"),
    ]);
    assert_eq!(code(&output), 1);
    let text = stdout(&output);
    assert!(
        text.contains("broken.yaml"),
        "the path must be in the report: {text}"
    );
    assert!(text.contains("unknown_initial_state"), "{text}");

    std::fs::remove_dir_all(directory.parent().expect("the tree root")).ok();
}

#[test]
fn validate_checks_an_artifact_manifest_against_the_lifecycles() {
    let output = protocol(&["validate", "--artifacts", ARTIFACTS]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(stdout(&output).contains("valid"));
}

#[test]
fn resolve_prints_the_plan() {
    let output = protocol(&["resolve", "--task", TASK]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("development.standard"), "{text}");
    assert!(text.contains("adp/default"), "{text}");
    assert!(text.contains("test-driven"), "{text}");
    assert!(
        text.contains("requires_approval"),
        "the capability summary must show what is gated: {text}"
    );
}

#[test]
fn resolve_fails_when_the_task_names_a_profile_that_does_not_exist() {
    let path = std::env::temp_dir().join("aep-cli-bad-task.yaml");
    std::fs::write(
        &path,
        "id: T-1\nkind: feature\nobjective: nothing\nprotocol: adp/1\nprofile: development.imaginary\n",
    )
    .expect("the fixture is writable");

    let output = protocol(&[
        "resolve",
        "--task",
        path.to_str().expect("a printable path"),
    ]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("development.imaginary"),
        "{}",
        stderr(&output)
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn evaluate_reports_the_state_and_why_a_transition_is_blocked() {
    let output = protocol(&["evaluate", "--task", TASK]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("state       receive"), "{text}");
    assert!(text.contains("Task incomplete"), "{text}");
}

#[test]
fn evaluate_advances_with_the_examples_evidence() {
    let output = protocol(&[
        "evaluate",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--evidence",
        "examples/development-passkeys/evidence/01-red-test.yaml",
        "--advance",
    ]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.contains("state       implement"),
        "a failing test and an approved specification are enough to reach implementation: {text}"
    );
}

#[test]
fn evaluate_reads_every_evidence_file_in_the_example() {
    let output = protocol(&[
        "evaluate",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--evidence",
        "examples/development-passkeys/evidence/01-red-test.yaml",
        "--evidence",
        "examples/development-passkeys/evidence/02-implementation.yaml",
        "--evidence",
        "examples/development-passkeys/evidence/03-verification.yaml",
        "--evidence",
        "examples/development-passkeys/evidence/04-review.yaml",
        "--advance",
    ]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.contains("adversarial_verify") || text.contains("review"),
        "the example's evidence should carry the work well past implementation: {text}"
    );
}

#[test]
fn explain_refuses_a_production_change_and_names_the_rule() {
    let output = protocol(&[
        "explain",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--action",
        "production.write",
    ]);
    assert_eq!(code(&output), 1, "a refusal is a non-zero exit");
    let text = stdout(&output);
    assert!(text.contains("production.write denied"), "{text}");
    assert!(text.contains("requires-approval"), "{text}");
    assert!(
        text.contains("approval for capability production.write"),
        "{text}"
    );
}

#[test]
fn explain_allows_what_the_profile_grants() {
    let output = protocol(&["explain", "--task", TASK, "--action", "tests.execute"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("is allowed"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn inspect_lists_documents_and_shows_one() {
    let listing = protocol(&["inspect"]);
    assert_eq!(code(&listing), 0, "{}", stderr(&listing));
    let text = stdout(&listing);
    assert!(text.contains("principle  test-driven"), "{text}");
    assert!(text.contains("workflow   adp/default"), "{text}");

    let single = protocol(&["inspect", "test-driven"]);
    assert_eq!(code(&single), 0, "{}", stderr(&single));
    let document = stdout(&single);
    assert!(document.contains("id: test-driven"), "{document}");
    assert!(document.contains("obligations"), "{document}");
}

#[test]
fn schema_lists_and_prints_generated_schemas() {
    let listing = protocol(&["schema"]);
    assert_eq!(code(&listing), 0);
    let text = stdout(&listing);
    // One line per published schema, checked against what the library publishes rather than
    // against a number: a count only ever fails with "the number changed".
    assert_eq!(
        text.lines().count(),
        aep_schema::generated_schemas().len(),
        "{text}"
    );
    for entry in aep_schema::generated_schemas() {
        assert!(
            text.contains(&entry.filename),
            "{} is not listed: {text}",
            entry.filename
        );
    }

    let single = protocol(&["schema", "workflow"]);
    assert_eq!(code(&single), 0);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout(&single)).expect("the schema is valid JSON");
    assert_eq!(parsed["title"], "RawWorkflow");
}

#[test]
fn json_output_is_machine_readable() {
    let output = protocol(&["evaluate", "--task", TASK, "--format", "json"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("the evaluation is valid JSON");
    assert_eq!(parsed["state"], "receive");
    assert!(parsed["transitions"].is_array());
    assert_eq!(parsed["is_complete"], false);
}

#[test]
fn conformance_runs_the_suites_against_the_reference_backend() {
    let output = protocol(&["conformance", "--level", "full"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("conformance full"), "{text}");
    assert!(text.contains("properties hold"), "{text}");
}

#[test]
fn conformance_fails_when_a_property_is_deliberately_broken() {
    // The point of shipping a faulty backend: a suite that passes everything tells you nothing, and
    // this is how a reader checks that for themselves in one command.
    let output = protocol(&[
        "conformance",
        "--suite",
        "idempotency",
        "--inject",
        "replay-applies",
    ]);
    assert_eq!(code(&output), 1, "a broken property is a non-zero exit");
    let text = stdout(&output);
    assert!(text.contains("do not hold"), "{text}");
    assert!(
        text.contains("expected to be caught by the `idempotency` suite"),
        "{text}"
    );
}

#[test]
fn a_project_is_discovered_so_no_arguments_are_needed() {
    // The first command an adopting team types should not need four paths.
    let project = std::env::temp_dir().join("aep-cli-project");
    std::fs::remove_dir_all(&project).ok();
    std::fs::create_dir_all(project.join(".engineering")).expect("writable");
    // The tree is named by the climb to it and not absolutely: a project file is committed, so
    // `ProtocolSource::parse` refuses an absolute path. This is also what a real adopter with a
    // sibling checkout writes.
    let engineering = project.join(".engineering");
    let base: Vec<_> = engineering.components().collect();
    let tree = root();
    let tree: Vec<_> = tree.components().collect();
    let shared = base
        .iter()
        .zip(&tree)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; base.len() - shared];
    parts.extend(
        tree[shared..]
            .iter()
            .map(|component| component.as_os_str().to_str().expect("a printable path")),
    );
    std::fs::write(
        project.join(".engineering/project.yaml"),
        format!(
            "protocol: adp/1\nprofile: development.standard\nprotocols: {}\n",
            parts.join("/")
        ),
    )
    .expect("writable");
    std::fs::write(
        project.join(".engineering/task.yaml"),
        "id: LOCAL-1\nkind: feature\nobjective: prove discovery works\nprotocol: adp/1\n\
         profile: development.standard\n",
    )
    .expect("writable");

    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .arg("resolve")
        .current_dir(&project)
        .output()
        .expect("the protocol binary runs");

    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("LOCAL-1"), "{text}");
    assert!(text.contains("development.standard"), "{text}");

    // From a subdirectory too: discovery walks up.
    let nested = project.join("src/deep");
    std::fs::create_dir_all(&nested).expect("writable");
    let nested_output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .arg("resolve")
        .current_dir(&nested)
        .output()
        .expect("the protocol binary runs");
    assert_eq!(code(&nested_output), 0, "{}", stderr(&nested_output));

    std::fs::remove_dir_all(&project).ok();
}

#[test]
fn outside_a_project_the_missing_task_is_explained() {
    let elsewhere = std::env::temp_dir().join("aep-cli-not-a-project");
    std::fs::create_dir_all(&elsewhere).expect("writable");

    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .arg("resolve")
        .current_dir(&elsewhere)
        .output()
        .expect("the protocol binary runs");

    assert_eq!(code(&output), 1);
    let errors = stderr(&output);
    assert!(errors.contains(".engineering/project.yaml"), "{errors}");
    assert!(errors.contains("no --task was given"), "{errors}");

    std::fs::remove_dir_all(&elsewhere).ok();
}

#[test]
fn output_survives_a_reader_that_stops_reading() {
    // `protocol inspect | head -3` must produce three lines, not a stack trace. Rust's `println!`
    // panics on a closed pipe, which turns an ordinary shell idiom into a crash report.
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["conformance", "--level", "full"])
        .current_dir(root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the protocol binary runs");

    // Read one line, then drop the pipe while the child is still writing.
    {
        use std::io::{BufRead, BufReader};
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut reader = BufReader::new(stdout);
        let mut first = String::new();
        reader
            .read_line(&mut first)
            .expect("the first line arrives");
        assert!(!first.is_empty());
    }

    let output = child.wait_with_output().expect("the child finishes");
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        !errors.contains("panicked"),
        "a reader that stopped reading is not a crash: {errors}"
    );
}

#[test]
fn conformance_runs_against_the_backend_the_caller_names_and_the_report_says_which() {
    // `story:conformance-verb-takes-a-backend`: the verb was hard-coded to the reference backend
    // while a story ticked "runs against the markdown store". Now the caller names the backend and
    // the first line of the report names it back.
    let sqlite = protocol(&["conformance", "--backend", "sqlite"]);
    assert_eq!(code(&sqlite), 0, "{}", stderr(&sqlite));
    let text = stdout(&sqlite);
    assert!(
        text.starts_with("ran against: sqlite (in-memory database)\n"),
        "{text}"
    );
    assert!(text.contains("conformance full"), "{text}");
    assert!(text.contains("properties hold"), "{text}");

    let dir = std::env::temp_dir().join(format!(
        "aep-cli-conformance-markdown-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch store");
    let markdown = protocol(&[
        "conformance",
        "--backend",
        "markdown",
        "--store",
        printable(&dir),
    ]);
    assert_eq!(code(&markdown), 0, "{}", stderr(&markdown));
    assert!(
        stdout(&markdown).starts_with(&format!("ran against: markdown ({})\n", dir.display())),
        "{}",
        stdout(&markdown)
    );
    let _ = std::fs::remove_dir_all(&dir);

    // The default did not move, and it says so too.
    let memory = protocol(&["conformance", "--level", "core"]);
    assert_eq!(code(&memory), 0, "{}", stderr(&memory));
    assert!(
        stdout(&memory).starts_with("ran against: memory\n"),
        "{}",
        stdout(&memory)
    );

    // Machine formats carry the same answer as a field.
    let json = protocol(&["conformance", "--backend", "sqlite", "--format", "json"]);
    assert_eq!(code(&json), 0, "{}", stderr(&json));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&json)).expect("JSON");
    assert_eq!(parsed["ran_against"], "sqlite (in-memory database)");
}

#[test]
fn conformance_refuses_a_store_for_the_backend_that_keeps_nothing() {
    // A flag that does nothing is a lie the next reader believes.
    let output = protocol(&["conformance", "--backend", "memory", "--store", "somewhere"]);
    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("would have no effect"),
        "{}",
        stderr(&output)
    );

    let unknown = protocol(&["conformance", "--backend", "oracle"]);
    assert_eq!(code(&unknown), 2, "an unknown backend is a usage error");
    assert!(
        stderr(&unknown).contains("memory") && stderr(&unknown).contains("sqlite"),
        "and the usage error names the backends that exist: {}",
        stderr(&unknown)
    );

    // The one backend with no scratch to invent: a server has to be named.
    let unaddressed = protocol(&["conformance", "--backend", "postgres"]);
    assert_eq!(code(&unaddressed), 1);
    assert!(
        stderr(&unaddressed).contains("needs `--store <url>`"),
        "{}",
        stderr(&unaddressed)
    );
}

#[test]
fn conformance_rejects_an_unknown_level_or_fault() {
    let level = protocol(&["conformance", "--level", "thorough"]);
    assert_eq!(code(&level), 1);
    assert!(
        stderr(&level).contains("is not a conformance level"),
        "{}",
        stderr(&level)
    );

    let fault = protocol(&["conformance", "--inject", "nonsense"]);
    assert_eq!(code(&fault), 1);
    assert!(
        stderr(&fault).contains("is not a fault"),
        "{}",
        stderr(&fault)
    );
}

/// The vendored corpus, whose expectations are the regression target for the scanner.
const CORPUS: &str = "examples/evidence-horizons-corpus/corpus";

/// The corpus's own reference date.
const REFERENCE_DATE: &str = "2026-09-01";

#[test]
fn the_scan_finds_every_annotation_the_corpus_holds_and_says_so_in_one_line() {
    // 43 raw occurrences, 43 records, zero unparsed — `expected.json`'s own summary, which is
    // ground truth since the corpus revision of 2026-08-21: the reference implementation was fixed
    // against this corpus and finds all 43, and the fenced example counts for neither side.
    let output = protocol(&[
        "evidence",
        "scan",
        CORPUS,
        "--at",
        REFERENCE_DATE,
        "--warn-days",
        "2",
        "--strict",
    ]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let report = stdout(&output);
    assert!(
        report.contains("43 occurrence(s), 43 record(s), 0 unparsed"),
        "the coverage line is the point of the verb: {report}"
    );
    assert!(
        report.contains("16 ok, 17 expiring, 10 expired, 8 malformed"),
        "the classification, at the corpus's own reference date: {report}"
    );
}

#[test]
fn a_scan_that_is_blind_to_an_annotation_fails_strict_and_says_which_file() {
    // The guard verified by breaking it: a document with an annotation this parser refuses would
    // report a divergence. The hyphen separator is the corpus's deliberate negative — it is not an
    // annotation and is not counted, so a file of them diverges by zero and `--strict` passes.
    let directory = scratch("aep-cli-evidence-scan");
    let file = directory.join("notes.md");
    write(
        &file,
        "Verify: 2026-08-30 - a hyphen is not an annotation. (horizon: 7d)\n",
    );
    let output = protocol(&[
        "evidence",
        "scan",
        printable(&file),
        "--at",
        REFERENCE_DATE,
        "--strict",
    ]);
    assert_eq!(
        code(&output),
        0,
        "a deliberate near-miss is not a coverage gap: {}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("0 occurrence(s), 0 record(s), 0 unparsed"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_expired_claim_fails_only_the_flag_that_exists_to_judge_it() {
    // Two flags, two questions: `--strict` asks whether the gate is blind, `--fail-on-expired`
    // whether a claim is stale. The corpus deliberately carries nine expired records, so a verb
    // that conflated them could never be run over it as a pass condition.
    let strict = protocol(&[
        "evidence",
        "scan",
        CORPUS,
        "--at",
        REFERENCE_DATE,
        "--warn-days",
        "2",
        "--strict",
    ]);
    assert_eq!(code(&strict), 0, "coverage is complete");

    let stale = protocol(&[
        "evidence",
        "scan",
        CORPUS,
        "--at",
        REFERENCE_DATE,
        "--warn-days",
        "2",
        "--fail-on-expired",
    ]);
    assert_eq!(code(&stale), 1, "and nine claims are past their horizon");
}

#[test]
fn inspect_reports_when_each_submitted_record_was_observed() {
    let output = protocol(&[
        "evidence",
        "inspect",
        "examples/development-passkeys/evidence/03-verification.yaml",
        "--at",
        "2023-11-20",
        "--horizon",
        "7d",
        "--format",
        "json",
    ]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let records: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("the report is JSON");
    let records = records.as_array().expect("a list of records");
    assert_eq!(records.len(), 6, "the document holds six records");
    assert_eq!(records[0]["observed_at"], "2023-11-13");
    assert_eq!(records[0]["age_days"], 7);
    assert_eq!(
        records[0]["state"], "ok",
        "seven days old against a seven-day horizon is the boundary, and the boundary is not expired"
    );
}

#[test]
fn inspect_refuses_an_observation_that_has_not_happened_yet() {
    // The same one comparison the engine applies at submission, available before anything is
    // submitted. A scheduled re-check stored as a performed one is the freshest record in the log.
    let directory = scratch("aep-cli-evidence-inspect");
    let file = directory.join("scheduled.yaml");
    write(
        &file,
        "- kind: test_result\n  observed_at: 2026-12-24\n  suite: unit\n  passed: 12\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    let output = protocol(&[
        "evidence",
        "inspect",
        printable(&file),
        "--at",
        REFERENCE_DATE,
    ]);
    assert_eq!(code(&output), 1, "a planned check is not an observation");
    assert!(
        stderr(&output).contains("has not happened yet"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn inspect_accepts_a_record_observed_on_the_reference_date_itself() {
    // The reference is a civil date and the check runs at civil granularity. A record stamped with
    // wall-clock milliseconds — `now_observed`, the default for a record the writer produced in
    // this process — is dated *inside* the reference day, and `--at <that day>` used to refuse it
    // as future because the day's own timestamp is its first millisecond. The primary use of the
    // verb is reading a record the day it was written; that must not be the failing case.
    let directory = scratch("aep-cli-evidence-inspect-today");
    let file = directory.join("today.yaml");
    // 2026-09-01T14:07 as epoch milliseconds: inside REFERENCE_DATE, after its first millisecond.
    write(
        &file,
        "- kind: test_result\n  observed_at: 1788271620000\n  suite: unit\n  passed: 12\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    let output = protocol(&[
        "evidence",
        "inspect",
        printable(&file),
        "--at",
        REFERENCE_DATE,
    ]);
    assert_eq!(
        code(&output),
        0,
        "an observation made during the reference day has happened: {}",
        stderr(&output)
    );
    assert!(
        !stderr(&output).contains("has not happened yet"),
        "{}",
        stderr(&output)
    );
}

/// A document with one good record, one dated after any clock will read, and one more good record.
///
/// `2099-01-01` rather than a date computed from the clock: the property is *this record is in the
/// future*, and a fixture that has to be regenerated to stay true is a fixture that goes stale.
fn mixed_evidence_document(name: &str) -> PathBuf {
    let directory = scratch(name);
    let file = directory.join("verify.yaml");
    write(
        &file,
        "- kind: test_result\n  observed_at: 2023-11-13\n  suite: unit\n  passed: 34\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n\n- kind: test_result\n  observed_at: 2099-01-01\n  suite: regression\n  passed: 812\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n\n- kind: test_result\n  observed_at: 2023-11-13\n  suite: integration\n  passed: 7\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    file
}

#[test]
fn one_future_record_refuses_itself_by_position_and_the_document_is_still_evaluated() {
    // The adopter's report: one record two hours ahead discarded the other 214, and the run
    // produced no evaluation at all. The refusal stands — it is invariant 7 and nothing here
    // downgrades it — but it is now about that record, and the rest of the file is submitted.
    let file = mixed_evidence_document("aep-cli-evaluate-future-record");
    let output = protocol(&[
        "evaluate",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--evidence",
        printable(&file),
    ]);

    assert_eq!(
        code(&output),
        1,
        "a refused record is not a warning: {}",
        stdout(&output)
    );
    let report = stdout(&output);
    assert!(
        report.contains("state       "),
        "the evaluation is still produced — an evaluation missing one fact beats no evaluation: {report}"
    );
    assert!(
        report.contains("\u{2713} tests.unit.failed == 0"),
        "record 1 reached the engine, which is what `the rest of the document is submitted` means: {report}"
    );
    assert!(
        report.contains("unobserved: regression_suite.result"),
        "and record 2 did not: a refused record contributes no fact, only a refusal: {report}"
    );
    let refusals = stderr(&output);
    assert!(
        refusals.contains("record 2"),
        "the refusal names which of the three records it is about: {refusals}"
    );
    assert!(
        refusals.contains("2099-01-01"),
        "and the date as the writer wrote it, not only an epoch pair: {refusals}"
    );
    assert!(
        refusals.contains("verify.yaml"),
        "and the file it is in: {refusals}"
    );
    assert_eq!(
        refusals
            .lines()
            .filter(|line| line.contains("record "))
            .count(),
        1,
        "the two dated records around it were submitted, not refused: {refusals}"
    );
}

#[test]
fn a_document_whose_every_record_is_future_dated_still_fails() {
    // Nothing is downgraded to a warning by being common. A file of scheduled re-checks submits
    // nothing and says so, with one line per record.
    let directory = scratch("aep-cli-evaluate-all-future");
    let file = directory.join("scheduled.yaml");
    write(
        &file,
        "- kind: test_result\n  observed_at: 2099-01-01\n  suite: unit\n  passed: 1\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n\n- kind: test_result\n  observed_at: 2099-01-02\n  suite: regression\n  passed: 2\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    let output = protocol(&[
        "evaluate",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--evidence",
        printable(&file),
    ]);

    assert_eq!(code(&output), 1, "{}", stdout(&output));
    let refusals = stderr(&output);
    assert!(refusals.contains("record 1"), "{refusals}");
    assert!(refusals.contains("record 2"), "{refusals}");
}

#[test]
fn evaluate_and_inspect_answer_identically_about_one_file() {
    // `evidence inspect`'s help says the two verbs apply one refusal to a future observation time.
    // They did not: one refused an instant and the whole file, the other reported a day and the
    // record. Both now put every record to the engine's own comparison and name the same one.
    let file = mixed_evidence_document("aep-cli-two-verbs-one-answer");

    let evaluated = protocol(&[
        "evaluate",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--evidence",
        printable(&file),
    ]);
    let inspected = protocol(&["evidence", "inspect", printable(&file)]);

    assert_eq!(code(&evaluated), 1, "{}", stdout(&evaluated));
    assert_eq!(code(&inspected), 1, "{}", stdout(&inspected));
    for refusals in [stderr(&evaluated), stderr(&inspected)] {
        assert!(refusals.contains("record 2"), "{refusals}");
        assert!(refusals.contains("2099-01-01"), "{refusals}");
        assert!(
            !refusals.contains("record 1") && !refusals.contains("record 3"),
            "neither verb refuses a record the other admits: {refusals}"
        );
    }
}

#[test]
fn inspect_admits_a_date_that_is_today_at_utc_plus_fourteen_and_refuses_one_nowhere_yet() {
    // The adopter's store sits at UTC+2 and writes local calendar dates, so between 22:00 and
    // midnight local — the last two hours of every UTC day — the correct date parses to tomorrow
    // at Greenwich. Measured in their tree at 22:27 UTC on 2026-08-28: 20 of 215 records refused.
    // Pinned here with `--at`, whose reference is the end of the named day.
    let directory = scratch("aep-cli-inspect-writers-day");
    let file = directory.join("local-dates.yaml");
    write(
        &file,
        "- kind: test_result\n  observed_at: 2026-09-02\n  suite: unit\n  passed: 12\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    let admitted = protocol(&[
        "evidence",
        "inspect",
        printable(&file),
        "--at",
        REFERENCE_DATE,
    ]);
    assert_eq!(
        code(&admitted),
        0,
        "2026-09-02 had begun at UTC+14 while Greenwich was still on 2026-09-01: {}",
        stderr(&admitted)
    );

    let nowhere_yet = directory.join("day-after.yaml");
    write(
        &nowhere_yet,
        "- kind: test_result\n  observed_at: 2026-09-03\n  suite: unit\n  passed: 12\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    let refused = protocol(&[
        "evidence",
        "inspect",
        printable(&nowhere_yet),
        "--at",
        REFERENCE_DATE,
    ]);
    assert_eq!(
        code(&refused),
        1,
        "and a day that has begun in no timezone is still a plan, not an observation"
    );
    assert!(
        stderr(&refused).contains("2026-09-03"),
        "{}",
        stderr(&refused)
    );
}

/// The corpus's own case for *the writer's day is not the engine's day*.
const WRITERS_DAY: &str = "examples/evidence-horizons-corpus/writers-day.yaml";

#[test]
fn the_corpus_case_admits_two_spellings_and_refuses_the_two_it_names() {
    // Records 2 and 3 of the fixture name one instant — midnight UTC on 2026-09-02 — written as a
    // day and as an epoch value. One is admitted and the other refused, which is the whole rule:
    // the granularity a document was written in survives to the comparison. A fixture where the
    // two spellings agreed would pass whether or not the rule held.
    let output = protocol(&["evidence", "inspect", WRITERS_DAY, "--at", REFERENCE_DATE]);

    assert_eq!(
        code(&output),
        1,
        "two of the five have not happened yet: {}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("5 record(s), aged at 2026-09-01"),
        "the whole table is printed anyway: {}",
        stdout(&output)
    );
    let refusals = stderr(&output);
    assert!(
        refusals.contains("record 3: the observation time 1788307200000ms"),
        "the instant spelling of 2026-09-02 midnight is refused exactly: {refusals}"
    );
    assert!(
        refusals.contains("record 4: the observation time 2026-09-03"),
        "and a day that has begun in no timezone, named as the date it was written: {refusals}"
    );
    assert_eq!(
        refusals
            .lines()
            .filter(|line| line.contains("record "))
            .count(),
        2,
        "records 1, 2 and 5 are admitted — including 2026-09-02 written as a day: {refusals}"
    );
}

#[test]
fn an_evidence_document_without_an_observation_time_is_refused_by_name() {
    // The field is required, and the refusal has to say which field — a harness author who omits it
    // is the person the rule exists for, and "invalid document" would send them reading YAML.
    let directory = scratch("aep-cli-evidence-undated");
    let file = directory.join("undated.yaml");
    write(
        &file,
        "- kind: test_result\n  suite: unit\n  passed: 12\n  failed: 0\n  producer:\n    producer: verifier\n    verifier: test-runner\n",
    );
    let output = protocol(&["evidence", "inspect", printable(&file)]);
    assert_eq!(code(&output), 1, "{}", stdout(&output));
    assert!(
        stderr(&output).contains("observed_at"),
        "the refusal names the field: {}",
        stderr(&output)
    );
}

/// `protocol workspace members` reports what it found and does not fail on a member nobody has.
///
/// The state worth pinning is `absent`: a workspace is read on machines that checked out different
/// subsets of it, and a command that failed because a colleague's repository is missing from your
/// disk is a command nobody could put in a script.
#[test]
fn a_workspace_member_nobody_checked_out_is_reported_rather_than_fatal() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("workspace-members");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".engineering")).expect("scratch root");
    std::fs::create_dir_all(root.join("here/.engineering/planning")).expect("a store that exists");
    std::fs::write(
        root.join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: here\n    source: ../here\n  - name: gone\n    source: ../gone\n",
    )
    .expect("workspace file");

    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["workspace", "members", "--root"])
        .arg(&root)
        .args(["--format", "json"])
        .output()
        .expect("the protocol binary runs");

    assert!(
        output.status.success(),
        "an absent member must not fail the command: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the report is JSON");
    let members = report["members"].as_array().expect("members");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0]["name"], "here");
    assert_eq!(members[0]["state"], "ok");
    assert_eq!(members[1]["name"], "gone");
    assert_eq!(members[1]["state"], "absent");
}

/// A repository without a workspace file answers only for itself, and says so rather than failing.
#[test]
fn a_repository_with_no_workspace_file_is_not_an_error() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("workspace-none");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".engineering")).expect("scratch root");

    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["workspace", "members", "--root"])
        .arg(&root)
        .output()
        .expect("the protocol binary runs");

    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("no workspace"),
        "it should say there is no workspace rather than print an empty list"
    );
}

/// Builds a two-member workspace on disk and returns its root.
fn two_member_workspace(name: &str, docs: &[(&str, &str, &str)]) -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".engineering")).expect("scratch root");
    std::fs::write(
        root.join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: one\n    source: ../one\n  - name: two\n    source: ../two\n",
    )
    .expect("workspace file");

    for (member, id, title) in docs {
        let (kind, artifact) = id.split_once(':').expect("kind:name");
        let directory = root.join(member).join(".engineering/planning").join(kind);
        std::fs::create_dir_all(&directory).expect("kind directory");
        std::fs::write(
            directory.join(format!("{artifact}.md")),
            format!(
                "---\nformat: aep.planning-md/1\nid: {id}\nkind: {kind}\nstatus: draft\ntitle: {title}\nrevision: 1\n---\n\n# {title}\n"
            ),
        )
        .expect("a planning document");
    }
    root
}

/// `workspace list` answers across members, and every row says which member it came from.
#[test]
fn a_workspace_list_names_the_member_every_artifact_came_from() {
    let root = two_member_workspace(
        "workspace-list",
        &[
            ("one", "story:alpha", "Alpha"),
            ("two", "story:beta", "Beta"),
        ],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["workspace", "list", "--root"])
        .arg(&root)
        .args(["--format", "json"])
        .output()
        .expect("the protocol binary runs");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    let rows = report["artifacts"].as_array().expect("artifacts");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["reference"], "one/story:alpha");
    assert_eq!(rows[1]["reference"], "two/story:beta");
}

/// A name two members both hold is refused, and the refusal is the two things to retype.
///
/// The failure being bought off here is the quiet one: answering with whichever store was read
/// first tells somebody a fact about the other repository and says nothing about having chosen.
#[test]
fn a_reference_two_members_both_hold_is_refused_with_both_spellings() {
    let root = two_member_workspace(
        "workspace-ambiguous",
        &[
            ("one", "story:passkey-login", "Passkey login, here"),
            ("two", "story:passkey-login", "Passkey login, elsewhere"),
        ],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["workspace", "show", "story:passkey-login", "--root"])
        .arg(&root)
        .args(["--format", "json"])
        .output()
        .expect("the protocol binary runs");

    assert_eq!(
        output.status.code(),
        Some(1),
        "an ambiguous reference is a refusal"
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    let mut retypeable: Vec<&str> = report["try"]
        .as_array()
        .expect("the refusal offers what to type")
        .iter()
        .map(|value| value.as_str().expect("a string"))
        .collect();
    retypeable.sort_unstable();
    assert_eq!(
        retypeable,
        vec!["one/story:passkey-login", "two/story:passkey-login"]
    );

    // And each spelling resolves on its own.
    let qualified = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["workspace", "show", "two/story:passkey-login", "--root"])
        .arg(&root)
        .args(["--format", "json"])
        .output()
        .expect("the protocol binary runs");
    assert!(qualified.status.success());
    let report: serde_json::Value = serde_json::from_slice(&qualified.stdout).expect("JSON");
    assert_eq!(report["member"], "two");
    assert_eq!(report["title"], "Passkey login, elsewhere");
}

/// Every group below `root` by id, and every leaf as `(id, run.state)`, in document order.
fn sections_and_leaves(
    node: &serde_yaml::Value,
    sections: &mut Vec<String>,
    leaves: &mut Vec<(String, String)>,
) {
    let id = node["id"].as_str().unwrap_or_default().to_owned();
    if let Some(nodes) = node["nodes"].as_sequence() {
        if id != "root" {
            sections.push(id);
        }
        for inner in nodes {
            sections_and_leaves(inner, sections, leaves);
        }
    } else {
        let state = node["run"]["state"].as_str().unwrap_or_default().to_owned();
        leaves.push((id, state));
    }
}

/// The shape the b10x loop is governed by: it asks its `transition` hook at a group boundary and
/// nowhere else, so a state that is not a group is a state the governor is never asked about. With
/// and without a map, every non-terminal state of `adp/default` comes out as a group named for
/// it, and the leaves inside are numbered from the state.
#[test]
fn workflow_flow_makes_every_state_a_section() {
    for extra in [&[][..], &["--map", "development/default"][..]] {
        let mut args = vec!["workflow", "flow", "--id", "adp/default"];
        args.extend_from_slice(extra);
        let output = protocol(&args);
        assert_eq!(code(&output), 0, "{extra:?}: {}", stderr(&output));
        let text = stdout(&output);
        assert!(
            text.contains("# Every state is a section"),
            "{extra:?}: the header says so:\n{text}"
        );
        let parsed: serde_yaml::Value = serde_yaml::from_str(&text).expect("valid YAML");

        let mut sections: Vec<String> = Vec::new();
        let mut leaves: Vec<(String, String)> = Vec::new();
        sections_and_leaves(&parsed["root"], &mut sections, &mut leaves);

        for state in [
            "receive",
            "specify",
            "decompose",
            "establish_verifiers",
            "implement",
            "verify",
            "adversarial_verify",
            "review",
        ] {
            assert!(
                sections.iter().any(|id| id == state),
                "{extra:?}: `{state}` is not a section: {sections:?}"
            );
        }
        assert!(
            sections.iter().any(|id| id == "implement-to-review"),
            "{extra:?}: the retreat is still a section of sections: {sections:?}"
        );
        for (id, state) in &leaves {
            assert!(
                !state.is_empty() && id.starts_with(&format!("{state}-")),
                "{extra:?}: leaf `{id}` is not numbered from the state it says it is in"
            );
        }
    }
}
