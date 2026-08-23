//! The record metaharness mints, read by the loop this repository decides on.
//!
//! The mirror of `metaharness_frame_contract.rs`. That file is *what this repository mints, read by
//! the rules the other side refuses it with*; this one is the other direction — **what they mint,
//! read by ours** — and it is the direction the adapter-contract design was written for:
//!
//! > An EP-driven eval, or any consumer, reads an adapter's conformance as a `contract_result`
//! > without knowing anything about metaharness's internals.
//! >
//! > — metaharness `docs/design/adapter-contract-v0.1.md`, read 2026-08-23
//!
//! Until this file existed, that sentence described an intention. The two repositories shared the
//! `contract_result` vocabulary and no code, and nothing here had ever read a byte the other side
//! produced — so a change to what `metaharness conformance <kind> --contract` prints was silent on
//! this side until somebody's evaluation quietly stopped seeing a contract record.
//!
//! # The fixtures
//!
//! Both are the provider's own bytes, captured 2026-08-23 from the live build — `metaharness
//! conformance claude --contract` and `metaharness conformance codex --contract`, exit 0, one JSON
//! object on standard output each. They are committed at
//! `crates/protocol-cli/fixtures/metaharness-contract-result-{claude,codex}.json` and hold nothing
//! account-level: two provider strings, one consumer string and four counts. A metaharness-side wave
//! pins the same bytes as goldens on its side, so the two repositories disagree loudly or not at all.
//!
//! # What is asserted here and not in the unit tests
//!
//! `crates/protocol-cli/src/contract.rs` tests the reading rules directly. What only a test through
//! the binary can show is the loop closing: the runner's bytes become a document, and
//! `protocol evaluate --evidence` reads that document and moves a principle's predicates. The two
//! halves run in different processes and the only thing joining them is a file.
//!
//! Both ways in are exercised here, because they are different code paths and only one of them can
//! be checked afterwards. `--record <file>` is the form to reach for; `--record -` is the pipe the
//! runner is already at the end of, and the two must produce the same record — asserted by minting
//! the same bytes both ways and comparing everything except the lines that say where they came from.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
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

/// The same, with `input` on standard input — the pipe the runner is at the end of.
///
/// Spawned rather than run, because the bytes have to be written after the child exists and its
/// standard input closed after they are: a verb reading to end of file on a pipe nobody closes waits
/// forever, and that failure would show up as a hung gate rather than as a red test.
fn protocol_with_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the protocol binary runs");
    child
        .stdin
        .take()
        .expect("standard input is a pipe")
        .write_all(input.as_bytes())
        .expect("the record is written to the pipe");
    child.wait_with_output().expect("the process is waited on")
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

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to build a mutated record in.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// `metaharness conformance claude --contract`, captured: 20 vectors, all green.
const CLAUDE_RECORD: &str = "crates/protocol-cli/fixtures/metaharness-contract-result-claude.json";

/// `metaharness conformance codex --contract`, captured: 10 vectors, all green.
const CODEX_RECORD: &str = "crates/protocol-cli/fixtures/metaharness-contract-result-codex.json";

/// The captured claude record's bytes, for the mutations below.
const CLAUDE_BYTES: &str = include_str!("../fixtures/metaharness-contract-result-claude.json");

/// A development task, so the protocol in force declares `contract_result` and the profile carries
/// `contract-testing`.
const TASK: &str = "examples/billing-conformance/task.yaml";

/// Its artifact graph.
const ARTIFACTS: &str = "examples/billing-conformance/artifacts.yaml";

/// The day the records were captured. Pinned, never `today`: a test whose answer depends on the day
/// it runs is a test that cannot be checked into a repository — and this verb requires the caller to
/// state the time anyway, because it did not watch the run.
const CAPTURED: &str = "2026-08-23";

/// Mints the evidence document for a record, and returns where it was written.
fn mint(record: &str, into: &Path, format: &str) -> Output {
    protocol(&[
        "contract",
        "evidence",
        "--record",
        record,
        "--observed-at",
        CAPTURED,
        "--format",
        format,
        "--out",
        printable(into),
    ])
}

/// Evaluates the billing task with one evidence document, and returns what the engine printed.
fn evaluate_with(evidence: &Path) -> String {
    let evaluated = protocol(&[
        "evaluate",
        "--task",
        TASK,
        "--artifacts",
        ARTIFACTS,
        "--evidence",
        printable(evidence),
    ]);
    assert_eq!(
        code(&evaluated),
        0,
        "the engine must read the document: {}",
        stderr(&evaluated)
    );
    stdout(&evaluated)
}

#[test]
fn both_captured_records_become_evidence_the_engine_reads() {
    // The whole loop, through the binary, for both adapters and both renderings: the provider's
    // bytes on disk, this repository's verb, a document, and the engine reading it back. Anything
    // that made the document unreadable — a field the payload does not have, a producer class the
    // principle does not accept, a shape that is not a list — surfaces here and nowhere else.
    let directory = scratch("aep-contract-evidence-roundtrip");

    for (name, record, format, suffix, checked) in [
        ("claude", CLAUDE_RECORD, "yaml", "yaml", "24 checked"),
        ("codex", CODEX_RECORD, "json", "json", "17 checked"),
    ] {
        let out = directory.join(format!("{name}.{suffix}"));
        let minted = mint(record, &out, format);
        assert_eq!(code(&minted), 0, "{}", stderr(&minted));
        assert!(
            stdout(&minted).contains(checked),
            "the verb reports the counts it wrote down: {}",
            stdout(&minted)
        );

        let document = std::fs::read_to_string(&out).expect("the record was written");
        assert!(
            document.contains("verifier: contract-runner")
                || document.contains("\"verifier\": \"contract-runner\""),
            "the producer is the one class this evidence kind names: {document}"
        );
        assert!(
            !document.contains("producer: agent") && !document.contains("\"producer\": \"agent\""),
            "an adapter's own claim about itself never mints this kind: {document}"
        );
        assert!(
            document.contains("metaharness.event/1"),
            "the consumer the adapter maps onto survives into the record: {document}"
        );

        let text = evaluate_with(&out);
        assert!(
            text.contains("✓ evidence contract_result from contract-runner (independent)"),
            "the {name} record discharges the obligation contract-testing places: {text}"
        );
        assert!(
            text.contains("✓ contracts.checked > 0")
                && text.contains("✓ contracts.failed == 0")
                && text.contains("✓ contracts.breaking_changes == 0"),
            "and satisfies all three of its predicates: {text}"
        );
    }
}

#[test]
fn a_breaking_change_is_the_number_the_evaluation_turns_on() {
    // The gate, isolated. Three records differing only in two counts:
    //
    //   | failed | breaking | what it means                        |
    //   |---|---|---|
    //   | 0 | 0 | the captured run: green                           |
    //   | 1 | 0 | red, and it is metaharness's own machinery (C3)   |
    //   | 1 | 1 | red, and the vendor moved (C1/C2)                 |
    //
    // The middle row is the control, and it is what makes this a test of `breaking_changes` rather
    // than of "something went red": rows two and three agree on `contracts.failed == 0` failing and
    // disagree on exactly one line. That is the provider's own decision 2 — `failed` is *the
    // contract is red*, `breaking_changes` is *and it is the vendor's fault* — asserted from this
    // side of the seam.
    let directory = scratch("aep-contract-evidence-gate");

    let green = directory.join("green.yaml");
    assert_eq!(code(&mint(CLAUDE_RECORD, &green, "yaml")), 0);
    let green_text = evaluate_with(&green);

    let ours = directory.join("ours.json");
    std::fs::write(&ours, CLAUDE_BYTES.replace("\"failed\":0", "\"failed\":1"))
        .expect("the scratch tree is writable");
    let ours_evidence = directory.join("ours-evidence.yaml");
    assert_eq!(code(&mint(printable(&ours), &ours_evidence, "yaml")), 0);
    let ours_text = evaluate_with(&ours_evidence);

    let vendor = directory.join("vendor.json");
    std::fs::write(
        &vendor,
        CLAUDE_BYTES
            .replace("\"failed\":0", "\"failed\":1")
            .replace("\"breaking_changes\":0", "\"breaking_changes\":1"),
    )
    .expect("the scratch tree is writable");
    let vendor_evidence = directory.join("vendor-evidence.yaml");
    assert_eq!(code(&mint(printable(&vendor), &vendor_evidence, "yaml")), 0);
    let vendor_text = evaluate_with(&vendor_evidence);

    assert!(
        green_text.contains("✓ contracts.breaking_changes == 0"),
        "the captured record satisfies the gate: {green_text}"
    );
    assert!(
        ours_text.contains("✗ contracts.failed == 0")
            && ours_text.contains("✓ contracts.breaking_changes == 0"),
        "a red run that is metaharness's own does not read as the vendor moving: {ours_text}"
    );
    assert!(
        vendor_text.contains("✗ contracts.breaking_changes == 0")
            && vendor_text.contains("contracts.breaking_changes = 1"),
        "and one breaking change blocks, naming the number that did it: {vendor_text}"
    );
}

#[test]
fn the_record_can_arrive_on_a_pipe_and_the_loop_still_closes() {
    // `--record -`, the whole way through: the runner's bytes never touch a file on the way in, and
    // what comes back is still a document `protocol evaluate --evidence` reads. This is the form
    // `metaharness conformance claude --contract | protocol contract evidence --record -` takes,
    // and it is the one thing the file form could not be made to prove.
    let directory = scratch("aep-contract-evidence-stdin");
    let out = directory.join("piped.yaml");

    let minted = protocol_with_stdin(
        &[
            "contract",
            "evidence",
            "--record",
            "-",
            "--observed-at",
            CAPTURED,
            "--out",
            printable(&out),
        ],
        CLAUDE_BYTES,
    );
    assert_eq!(code(&minted), 0, "{}", stderr(&minted));
    assert!(
        stdout(&minted).contains("24 checked"),
        "the verb reports the counts it was piped: {}",
        stdout(&minted)
    );

    let document = std::fs::read_to_string(&out).expect("the record was written");
    assert!(
        document.contains("standard input"),
        "the provenance says where the bytes came from, because nothing else holds them now: \
         {document}"
    );
    let piped = evaluate_with(&out);
    assert!(
        piped.contains("✓ evidence contract_result from contract-runner (independent)"),
        "a piped record discharges the same obligation a file one does: {piped}"
    );

    // The two input forms differ in provenance and in nothing else. Minting the committed file is
    // the comparison: same bytes, same record, same digest — a pipe that dropped a byte would
    // otherwise read as a success, because a shorter record is still a valid one.
    let from_file = directory.join("file.yaml");
    assert_eq!(code(&mint(CLAUDE_RECORD, &from_file, "yaml")), 0);
    let filed = std::fs::read_to_string(&from_file).expect("the record was written");
    assert_eq!(
        without_source(&document),
        without_source(&filed),
        "the same bytes must produce the same record whichever way they arrived"
    );
    assert!(
        filed.contains(CLAUDE_RECORD),
        "and the file form still names the file, which is the check the pipe form gives up: {filed}"
    );
}

/// A minted document with the two lines that name where the bytes came from removed.
///
/// Everything else — the counts, the producer, the observation time and the digest — is what the
/// two input forms have to agree on exactly.
fn without_source(document: &str) -> Vec<&str> {
    document
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("command:")
                && !line.starts_with("- crates/")
                && line != "- standard input"
        })
        .collect()
}

#[test]
fn a_record_that_checked_nothing_is_refused_before_a_document_exists() {
    // The discipline `principles/development/contract-testing.yaml` states, enforced where the
    // record enters rather than one layer later. Measured, a `checked: 0` record submitted to the
    // engine reads `✓ evidence contract_result from contract-runner (independent)` and passes two
    // of the principle's three predicates vacuously — so it would discharge the obligation on the
    // strength of a run that checked nothing.
    let directory = scratch("aep-contract-evidence-empty");
    let empty = directory.join("empty.json");
    std::fs::write(
        &empty,
        CLAUDE_BYTES.replace("\"checked\":24", "\"checked\":0"),
    )
    .expect("the scratch tree is writable");
    let out = directory.join("never-written.yaml");

    let refused = mint(printable(&empty), &out, "yaml");
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("asserts nothing"),
        "the refusal names the reason: {}",
        stderr(&refused)
    );
    assert!(
        stderr(&refused).contains("contract-testing"),
        "and cites the principle that states the discipline: {}",
        stderr(&refused)
    );
    assert!(
        !out.exists(),
        "and no document exists for anyone to submit later"
    );
}

#[test]
fn a_record_whose_breaking_changes_exceed_its_failures_is_refused_before_a_document_exists() {
    // A breaking change is one of the failures, so this pair describes no run. Refused because the
    // engine would read both counts happily and the record would contradict itself about one run:
    // `contracts.failed == 0` passing beside `contracts.breaking_changes == 0` failing.
    let directory = scratch("aep-contract-evidence-impossible");
    let impossible = directory.join("impossible.json");
    std::fs::write(
        &impossible,
        CLAUDE_BYTES.replace("\"breaking_changes\":0", "\"breaking_changes\":2"),
    )
    .expect("the scratch tree is writable");
    let out = directory.join("never-written.yaml");

    let refused = mint(printable(&impossible), &out, "yaml");
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("2 breaking change(s) out of 0 failure(s)"),
        "the refusal quotes both counts, so the record can be fixed: {}",
        stderr(&refused)
    );
    assert!(
        !out.exists(),
        "and no document exists for anyone to submit later"
    );
}

#[test]
fn the_observation_time_is_required_because_this_process_did_not_watch_the_run() {
    // The one place this verb is stricter than `protocol trace evidence`, which defaults to now
    // because the check runs in its own process. The contract run happened elsewhere and the record
    // carries no time of its own, so a default would be a freshness claim nobody made — and a stale
    // record reading as fresh is precisely what evidence horizons exist to catch.
    let refused = protocol(&["contract", "evidence", "--record", CLAUDE_RECORD]);
    assert_eq!(
        code(&refused),
        2,
        "clap refuses the invocation: {}",
        stderr(&refused)
    );
    assert!(
        stderr(&refused).contains("--observed-at"),
        "and names the argument that is missing: {}",
        stderr(&refused)
    );
}
