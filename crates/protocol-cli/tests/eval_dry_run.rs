//! The whole three-arm pipeline, end to end, for nothing — in the gate.
//!
//! # What this proves, and what it deliberately does not
//!
//! Four recorded streams go in one end and a matrix comes out the other: the runner assembles a
//! `eval.run-manifest/1` per run out of what each stream states, judges each transcript with the
//! case's own `trace-spec/1` document through the same checker `protocol trace check` calls, lays
//! the pair out where `protocol eval matrix` looks for it, and the matrix is asserted **byte for
//! byte**. Every step of the programme except the spawn is exercised, and the spawn is the only
//! step that costs money.
//!
//! What it does not prove is anything about a model. Two of the four streams are the eval corpus's
//! own committed transcripts and two are derived from them
//! (`crates/protocol-cli/fixtures/eval-run/README.md` says exactly how): they are structurally
//! faithful and **not observed**, so a failure here is a change in this repository's code or
//! documents and never a finding about Claude Code or Codex. When a paid sweep produces real
//! streams they replace these in place — same verb, same flags, same layout, no test change.
//!
//! # Why the golden is the whole document
//!
//! The same reason `eval_matrix.rs` gives for its own: the deliverable of this programme is a
//! document somebody commits and diffs between waves, so a matrix whose row order, key order or
//! column set moved must show up as a failure rather than as a diff nobody chose. It is also the
//! drift check between the runner and the corpus — an edit to a case's expectations, its
//! transcript, or the manifest's field list lands here with the row that changed.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args`, always from the repository root, and never with a live flag.
fn protocol(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        // Nothing in this file may spawn anything, whatever the developer running it exported.
        .env_remove("METAHARNESS_LIVE")
        .env_remove("METAHARNESS_BIN")
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

/// The exit code.
fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// The committed matrix these four runs assemble into, as JSON.
const MATRIX_JSON: &str = include_str!("../fixtures/eval-run/dry-run.matrix.json");

/// The same, as a person reads it.
const MATRIX_TEXT: &str = include_str!("../fixtures/eval-run/dry-run.matrix.txt");

/// The date every run in the dry run is observed at.
///
/// The caller's, and required by the verb rather than defaulted to now: a manifest is a committed
/// document that must assemble to the same bytes twice, and a clock in it would make every
/// re-ingest a diff. This constant is that decision paying for itself — without it there is no
/// golden to assert.
const OBSERVED_AT: &str = "2026-08-23";

/// One arm of one case, as the dry run replays it.
struct Replay {
    /// The case directory.
    case: &'static str,
    /// The arm.
    arm: &'static str,
    /// The harness.
    harness: &'static str,
    /// The recorded stream standing in for the spawn.
    stream: &'static str,
}

/// The four runs: two harnesses, all three arms, one workflow, one honest case and one that is not.
const REPLAYS: [Replay; 4] = [
    // Arm a, over the corpus's own violating transcript — which attests no plugin, as arm a must.
    Replay {
        case: "conformance/eval/development-tests-after-the-code",
        arm: "raw",
        harness: "claude",
        stream: "conformance/eval/development-tests-after-the-code/transcript.jsonl",
    },
    // Arm b, over a stream whose `session.started` attests the injected plugin with its digest.
    Replay {
        case: "conformance/eval/development-honest",
        arm: "plugin",
        harness: "claude",
        stream: "crates/protocol-cli/fixtures/eval-run/claude-plugin-attested.jsonl",
    },
    // Arm c, which this verb reads and does not launch: `protocol drive run` wrote it.
    Replay {
        case: "conformance/eval/development-honest",
        arm: "driven",
        harness: "claude",
        stream: "crates/protocol-cli/fixtures/eval-run/claude-driven-attested.jsonl",
    },
    // The second harness, on the wire that prices nothing.
    Replay {
        case: "conformance/eval/development-honest",
        arm: "plugin",
        harness: "codex",
        stream: "crates/protocol-cli/fixtures/eval-run/codex-plugin-attested.jsonl",
    },
];

/// Replays all four runs into one directory and answers with it.
fn ingest_every_arm(name: &str) -> PathBuf {
    let out = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&out).ok();
    std::fs::create_dir_all(&out).expect("the temporary tree is writable");

    for replay in &REPLAYS {
        let ingested = protocol(&[
            "eval",
            "run",
            "--case",
            replay.case,
            "--arm",
            replay.arm,
            "--harness",
            replay.harness,
            "--stream",
            replay.stream,
            "--observed-at",
            OBSERVED_AT,
            "--out",
            printable(&out),
            // Every record committed to a public repository is redacted, and the pairs this
            // produces are the shape the committed ones have.
            "--redact",
        ]);
        assert_eq!(
            code(&ingested),
            0,
            "ingesting {} / {} / {} — {}",
            replay.harness,
            replay.arm,
            replay.case,
            stderr(&ingested)
        );
    }
    out
}

#[test]
fn the_whole_pipeline_runs_on_committed_streams_and_assembles_the_matrix_byte_for_byte() {
    // The story's acceptance in one test: spawn (recorded upstream), record, judge, assemble —
    // with no vendor binary, no credential and no network anywhere in it.
    let out = ingest_every_arm("aep-eval-dry-run-golden");

    let json = protocol(&["eval", "matrix", printable(&out), "--format", "json"]);
    assert_eq!(code(&json), 0, "{}", stderr(&json));
    assert_eq!(
        stdout(&json),
        MATRIX_JSON,
        "the pipeline assembles the committed matrix, byte for byte"
    );

    let text = protocol(&["eval", "matrix", printable(&out)]);
    assert_eq!(code(&text), 0, "{}", stderr(&text));
    assert_eq!(stdout(&text), MATRIX_TEXT);

    for rendering in [MATRIX_JSON, MATRIX_TEXT] {
        assert!(
            !rendering.contains('%'),
            "no percentage reaches the committed bytes"
        );
        assert_eq!(
            rendering.matches("score").count(),
            rendering.matches("no score is computed").count(),
            "and the only place the word occurs is the sentence saying there is none"
        );
    }
}

#[test]
fn the_dry_run_reaches_both_harnesses_all_three_arms_and_a_contradiction() {
    // What makes the golden above worth having. A pipeline test whose every row held would be
    // green against a checker that had stopped checking — the arm-a run is the corpus's declared
    // violation, so two facts are contradicted here on purpose.
    let out = ingest_every_arm("aep-eval-dry-run-coverage");
    let table = stdout(&protocol(&["eval", "matrix", printable(&out)]));

    for (harness, arm) in [
        ("claude", "raw"),
        ("claude", "plugin"),
        ("claude", "driven"),
        ("codex", "plugin"),
    ] {
        assert!(
            table
                .lines()
                .any(|line| line.contains(harness) && line.contains(arm)),
            "the matrix has a cell for {harness} × {arm}: {table}"
        );
    }
    assert!(
        table.contains("38 fact(s) held, 2 contradicted, 0 nobody found out, over 4 run(s)"),
        "and the arm-a run contradicts the two ordering rows its case declares: {table}"
    );
    // The third column is zero here and that is a property of the *document*, not of the pipeline:
    // this case's expectations read tool calls and orderings, and none of them reads a field a wire
    // writes `null` into. `eval_matrix.rs`'s golden is where the column is exercised — the Codex
    // runs there carry `thinking_tokens: null`. Written down so a later reader does not take the
    // zero for evidence that nothing can be undecidable.
    assert!(
        table.contains("adp/default               codex      plugin    1      10     0          0"),
        "the codex cell decided all ten of its rows: {table}"
    );
}

#[test]
fn the_manifests_the_runner_wrote_are_the_documents_the_matrix_refuses_to_guess_at() {
    // The R3.2 decision, asserted on the products rather than on prose: the three fields that
    // describe the run are read out of each stream, and the four the runner knows are its own. A
    // manifest is refused for a hole in either half, so a green matrix over these four is the
    // evidence that both halves arrived.
    let out = ingest_every_arm("aep-eval-dry-run-manifests");

    let raw = std::fs::read_to_string(
        out.join("claude-raw-development-tests-after-the-code.manifest.yaml"),
    )
    .expect("arm a left a manifest");
    assert!(
        raw.contains("plugin_digest: null"),
        "arm a writes the key and writes it null, never omitting it:\n{raw}"
    );
    assert!(
        raw.contains("harness_version: claude 2.1.239") && raw.contains("model: claude-sonnet-5"),
        "and the two the stream stated are the stream's:\n{raw}"
    );
    assert!(
        raw.contains(&format!("observed_at: {OBSERVED_AT}")),
        "and the one nothing in a stream could know is the caller's:\n{raw}"
    );

    let codex = std::fs::read_to_string(out.join("codex-plugin-development-honest.manifest.yaml"))
        .expect("the codex run left a manifest");
    assert!(
        codex.contains("harness_version: codex 0.145.0"),
        "the version pin says whose it is — two harnesses at 0.145.0 are not one pin:\n{codex}"
    );
    assert!(
        !codex.contains("cost_micro_usd"),
        "and a run its wire priced at null states no cost rather than stating zero:\n{codex}"
    );
    assert!(
        codex.contains("model: null"),
        "and a wire that names no model at session start says so, rather than being given the \
         model somebody assumed it resolved — which is what the first live pilot run taught this \
         fixture:\n{codex}"
    );
}

#[test]
fn the_digest_every_manifest_carries_comes_from_the_instruments_row() {
    // Crossing #4 after the correction. `session.started` carries the vendor's own `plugins` echo
    // *and* `hermetic.installed_plugins`, and only the second is written on every adapter — Codex's
    // vendor states nothing, so its echo is `null` while its instrument row is an honest `[]`. The
    // dry run's own Codex leg is the proof that the manifest still gets a digest out of a stream
    // whose vendor half says nothing at all.
    let out = ingest_every_arm("aep-eval-dry-run-attestation");

    let codex_stream = std::fs::read_to_string(
        root().join("crates/protocol-cli/fixtures/eval-run/codex-plugin-attested.jsonl"),
    )
    .expect("the codex fixture is readable");
    let started: serde_json::Value =
        serde_json::from_str(codex_stream.lines().next().expect("a session")).expect("JSON");
    let attested = started["hermetic"]["installed_plugins"][0]["digest"]
        .as_str()
        .expect("the instrument attests a digest");

    let manifest =
        std::fs::read_to_string(out.join("codex-plugin-development-honest.manifest.yaml"))
            .expect("the codex run left a manifest");
    assert!(
        manifest.contains(&format!("plugin_digest: {attested}")),
        "the manifest carries the instrument's digest verbatim:\n{manifest}"
    );
}
