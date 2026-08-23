//! Seven checked runs, three arms, two harnesses, one table — asserted on the bytes.
//!
//! # What the committed pairs are, and what they are not
//!
//! No three-arm evaluation has been run yet. These pairs are **constructed**, and saying so here is
//! the point of the file: a fixture set that implied a measurement nobody made would be the same
//! defect as a matrix that implied a score nobody can compute.
//!
//! What is real in them is everything the assembler reads. Every record under
//! `crates/protocol-cli/fixtures/eval-matrix/runs/` is the output of this repository's own checker
//! over a committed transcript, minted with
//!
//! ```console
//! protocol trace check --spec crates/protocol-cli/fixtures/eval-matrix/expectations.development-story.trace.yaml \
//!     --transcript <the transcript> --format json --redact
//! ```
//!
//! and `--redact` because a report that quotes a transcript is not a thing to commit to a public
//! repository. Four transcripts are recorded runs already in this tree
//! (`crates/trace-spec/tests/fixtures/`: the plugin eval's session, the driven honest step and the
//! driven denial step); four are written for this fixture set under `fixtures/eval-matrix/transcripts/`,
//! three of them for a harness whose transcripts this repository has never held — which is why the
//! `unobservable` column has anything in it at all: that wire carries `thinking_tokens` and writes
//! `null` into it.
//!
//! The manifests are this story's own document kind, and their numbers are read out of the
//! transcripts beside them. One of the seven states no cost, no tokens and no wall time, on purpose:
//! a resource total over some of a cell's runs must say so, and a fixture set where every run
//! answers cannot show that it does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args`, always from the repository root.
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

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to build a mutated pair in.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// The committed runs.
const RUNS: &str = "crates/protocol-cli/fixtures/eval-matrix/runs";

/// The committed matrix, as JSON.
const MATRIX_JSON: &str = include_str!("../fixtures/eval-matrix/matrix.json");

/// The committed matrix, as a person reads it.
const MATRIX_TEXT: &str = include_str!("../fixtures/eval-matrix/matrix.txt");

/// One committed pair, for the mutations below.
const CLAUDE_PLUGIN_MANIFEST: &str =
    include_str!("../fixtures/eval-matrix/runs/claude-plugin-create-a-story.manifest.yaml");

/// Its record.
const CLAUDE_PLUGIN_RECORD: &str =
    include_str!("../fixtures/eval-matrix/runs/claude-plugin-create-a-story.report.json");

/// Writes a pair into a scratch directory under one name.
fn write_pair(directory: &Path, name: &str, manifest: &str, record: &str) {
    std::fs::write(directory.join(format!("{name}.manifest.yaml")), manifest)
        .expect("the scratch tree is writable");
    std::fs::write(directory.join(format!("{name}.report.json")), record)
        .expect("the scratch tree is writable");
}

/// Assembles the matrix over a directory, as JSON.
fn assemble(directory: &Path) -> Output {
    protocol(&["eval", "matrix", printable(directory), "--format", "json"])
}

#[test]
fn the_committed_pairs_assemble_into_the_matrix_byte_for_byte() {
    // The golden. Byte equality and not a field-by-field comparison, because the deliverable of
    // this story *is* a document somebody commits and diffs between waves: a matrix whose key
    // order, row order or column set moved would produce a diff nobody chose, and a test that
    // compared values would not see it.
    let json = protocol(&["eval", "matrix", RUNS, "--format", "json"]);
    assert_eq!(code(&json), 0, "{}", stderr(&json));
    assert_eq!(
        stdout(&json),
        MATRIX_JSON,
        "the assembled matrix is the committed one, byte for byte"
    );

    let text = protocol(&["eval", "matrix", RUNS, "--format", "text"]);
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
fn the_matrix_reports_every_arm_of_every_harness_and_all_three_answers() {
    // What makes the golden above worth having: the fixture set reaches every state the assembler
    // has a column for. Asserted through the text rendering, which is what a person reads.
    let text = protocol(&["eval", "matrix", RUNS]);
    assert_eq!(code(&text), 0, "{}", stderr(&text));
    let table = stdout(&text);

    for harness in ["claude", "codex"] {
        for arm in ["raw", "plugin", "driven"] {
            assert!(
                table
                    .lines()
                    .any(|line| line.contains(harness) && line.contains(arm)),
                "the matrix has a cell for {harness} × {arm}: {table}"
            );
        }
    }
    assert!(
        table.contains("23 fact(s) held, 9 contradicted, 3 nobody found out, over 7 run(s)"),
        "and all three answers occur, so no column is untested: {table}"
    );
    assert!(
        table.contains("(1/2)"),
        "including a resource total that covers some of a cell's runs: {table}"
    );
}

#[test]
fn a_null_verdict_is_counted_unobservable_and_never_held() {
    // The polarity, verified by mutation through the binary. The committed claude/plugin run is the
    // only one whose five expectations all held, so a single `"verdict": null` in it must move one
    // fact out of `held` and into `unobservable` — and never leave the counts where they were.
    let directory = scratch("aep-eval-matrix-null-verdict");
    write_pair(
        &directory,
        "mutated",
        CLAUDE_PLUGIN_MANIFEST,
        CLAUDE_PLUGIN_RECORD,
    );

    let honest = assemble(&directory);
    assert_eq!(code(&honest), 0, "{}", stderr(&honest));
    let honest = stdout(&honest);
    assert!(
        honest.contains("\"held\": 5,\n  \"violated\": 0,\n  \"unobservable\": 0"),
        "the unmutated run holds all five: {honest}"
    );

    // The row's verdict, at the indent a row's fields have — not the report's own top-level
    // verdict, which the matrix does not read and which would make this mutation a no-op.
    let silenced = CLAUDE_PLUGIN_RECORD.replacen(
        "\n      \"verdict\": \"ok\"",
        "\n      \"verdict\": null",
        1,
    );
    assert_ne!(
        silenced, CLAUDE_PLUGIN_RECORD,
        "the mutation reached the record"
    );
    write_pair(&directory, "mutated", CLAUDE_PLUGIN_MANIFEST, &silenced);

    let mutated = assemble(&directory);
    assert_eq!(code(&mutated), 0, "{}", stderr(&mutated));
    let mutated = stdout(&mutated);
    assert!(
        mutated.contains("\"held\": 4,\n  \"violated\": 0,\n  \"unobservable\": 1"),
        "a row the checker recorded no verdict for is unobservable, never held: {mutated}"
    );
}

#[test]
fn a_row_the_record_does_not_mention_at_all_is_the_same_answer_as_a_null_one() {
    // The other spelling of *nothing was recorded*: the key removed rather than nulled. Both are
    // read as `unobservable`, because a checker that dropped a field and one that wrote `null`
    // established exactly as much about the run.
    let directory = scratch("aep-eval-matrix-absent-verdict");
    let stripped = CLAUDE_PLUGIN_RECORD.replacen("      \"verdict\": \"ok\"\n", "", 1);
    assert_ne!(
        stripped, CLAUDE_PLUGIN_RECORD,
        "the mutation reached the record"
    );
    // The row above the removed line ends in a comma that now dangles; take it out too.
    let stripped = stripped.replacen("      },\n    }", "      }\n    }", 1);
    write_pair(&directory, "mutated", CLAUDE_PLUGIN_MANIFEST, &stripped);

    let mutated = assemble(&directory);
    assert_eq!(code(&mutated), 0, "{}", stderr(&mutated));
    assert!(
        stdout(&mutated).contains("\"held\": 4,\n  \"violated\": 0,\n  \"unobservable\": 1"),
        "an absent verdict is unobservable too: {}",
        stdout(&mutated)
    );
}

#[test]
fn an_incomplete_manifest_is_refused_by_name_and_no_matrix_is_written() {
    // Three fields removed, so the refusal also has to show that validation accumulates: a reader
    // who has to run the verb three times to find three typos stops running it.
    let directory = scratch("aep-eval-matrix-incomplete");
    let kept: Vec<&str> = CLAUDE_PLUGIN_MANIFEST
        .lines()
        .filter(|line| {
            !line.starts_with("model:")
                && !line.starts_with("harness_version:")
                && !line.starts_with("case:")
        })
        .collect();
    let stripped = kept.join("\n");
    write_pair(&directory, "incomplete", &stripped, CLAUDE_PLUGIN_RECORD);
    let out = directory.join("never-written.json");

    let refused = protocol(&[
        "eval",
        "matrix",
        printable(&directory),
        "--format",
        "json",
        "--out",
        printable(&out),
    ]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("3 refusal(s)"),
        "every refusal is reported at once: {reason}"
    );
    for field in ["case", "model", "harness_version"] {
        assert!(
            reason.contains("EVAL-MANIFEST-003") && reason.contains(&format!("`{field}`")),
            "and each names its own field: {reason}"
        );
    }
    assert!(
        !out.exists(),
        "and no matrix exists for anyone to read as a measurement"
    );
}

#[test]
fn an_arm_this_evaluation_does_not_have_is_refused_by_name() {
    let directory = scratch("aep-eval-matrix-fourth-arm");
    write_pair(
        &directory,
        "hybrid",
        &CLAUDE_PLUGIN_MANIFEST.replace("arm: plugin", "arm: hybrid"),
        CLAUDE_PLUGIN_RECORD,
    );

    let refused = assemble(&directory);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-MANIFEST-002") && reason.contains("`hybrid`"),
        "the refusal names the code and the word it was handed: {reason}"
    );
}

#[test]
fn a_plugin_digest_on_arm_raw_is_refused_where_the_manifest_enters() {
    // The manifest rule that is a statement about the experiment rather than about a document:
    // arm a *is* the arm with no plugin in it, so a run that names one is not a run of arm a.
    let directory = scratch("aep-eval-matrix-raw-with-a-plugin");
    write_pair(
        &directory,
        "raw",
        &CLAUDE_PLUGIN_MANIFEST.replace("arm: plugin", "arm: raw"),
        CLAUDE_PLUGIN_RECORD,
    );

    let refused = assemble(&directory);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-MANIFEST-005"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn a_manifest_whose_transcript_is_not_the_records_transcript_is_refused() {
    // The one contradiction between the two documents that is checkable from what they both carry.
    // Without it a manifest could describe a Codex run under arm a and be counted with the
    // outcomes of somebody else's Claude run, and nothing in either document would object.
    let directory = scratch("aep-eval-matrix-wrong-run");
    write_pair(
        &directory,
        "swapped",
        &CLAUDE_PLUGIN_MANIFEST.replace(
            "transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b",
            "transcript_digest: 4730e11b5bc692a9115f0c00c333a7c630698669d9d97d9421cf46649aed6b47",
        ),
        CLAUDE_PLUGIN_RECORD,
    );

    let refused = assemble(&directory);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-PAIR-003") && reason.contains("belongs to another run"),
        "{reason}"
    );
}

#[test]
fn a_manifest_with_no_record_beside_it_is_refused_rather_than_skipped() {
    let directory = scratch("aep-eval-matrix-lonely-manifest");
    std::fs::write(
        directory.join("lonely.manifest.yaml"),
        CLAUDE_PLUGIN_MANIFEST,
    )
    .expect("the scratch tree is writable");

    let refused = assemble(&directory);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-PAIR-001"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn a_record_with_no_manifest_beside_it_is_refused_rather_than_dropped() {
    // The direction that matters more, and the one a scanner gets wrong: a record nobody claimed
    // would leave the matrix silently, and the matrix would report fewer runs than were run.
    let directory = scratch("aep-eval-matrix-lonely-record");
    std::fs::write(directory.join("lonely.report.json"), CLAUDE_PLUGIN_RECORD)
        .expect("the scratch tree is writable");
    write_pair(
        &directory,
        "claimed",
        CLAUDE_PLUGIN_MANIFEST,
        CLAUDE_PLUGIN_RECORD,
    );

    let refused = assemble(&directory);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-PAIR-002"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn a_directory_with_no_runs_in_it_is_refused_rather_than_rendered_as_a_clean_sheet() {
    let directory = scratch("aep-eval-matrix-empty");

    let refused = assemble(&directory);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-PAIR-006") && stderr(&refused).contains("clean sheet"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn the_matrix_is_a_report_and_not_a_gate() {
    // Exit 0 over a fixture set holding nine contradicted facts. An exit code that moved with the
    // counts would be the scalar this programme refuses to compute, wearing a different name.
    let assembled = protocol(&["eval", "matrix", RUNS]);
    assert_eq!(code(&assembled), 0);
    assert!(
        stdout(&assembled).contains("9 contradicted"),
        "and the contradictions are in the report where a reader decides on them: {}",
        stdout(&assembled)
    );
}
