//! A verb's grouped path and its flat spelling are the same command.
//!
//! The first level of `aep` is the four area names plus `doctor`, and every spelling that reached
//! 0.51.0 still answers as a hidden top-level alias. `command_tree` in the binary proves the two
//! paths exist in the clap tree and reach the same subtree; that is a claim about the parser.
//!
//! This is the claim about the *answer*: the same bytes on standard output, the same bytes on
//! standard error, the same exit status — no banner, no deprecation line, no reordering. Callers of
//! the flat spellings are counted in the hundreds outside this repository, and several of them
//! compare output rather than read it: `metaharness` diffs a recorded transcript, the driver's
//! `checks` map compares a report, and an evaluation matrix is committed byte for byte. A single
//! extra line on standard error would be invisible in a manual check and would break all three.
//!
//! Run against both binaries, because `aep` and `protocol` are one interface (invariant 10) and a
//! grouping that reached only one of them would be the first thing to break it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root, which every case below is run from.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs one binary with `arguments` against the repository's own tree.
fn run(binary: &str, arguments: &[&str]) -> Output {
    Command::new(binary)
        .args(arguments)
        .current_dir(root())
        .output()
        .expect("the CLI starts")
}

/// The transcript specification this repository ships, for `trace check`.
const SPEC: &str = "conformance/trace/expectations.trace.yaml";

/// A committed real run, checked against it.
const TRANSCRIPT: &str = "crates/observe/trace-spec/tests/fixtures/plugin-eval-7hTYjT.jsonl";

/// The committed evaluation runs `eval matrix` assembles.
const RUNS: &str = "crates/edge/aep-cli/fixtures/eval-matrix/runs";

/// One leaf, spelled both ways, with arguments that make it do its work rather than refuse.
///
/// Every case here has a fixture behind it and exercises the leaf's real output. A usage error is
/// deliberately not among them: clap prints the path it was invoked by, so `Usage: protocol
/// validate` and `Usage: protocol govern validate` are *supposed* to differ, and a case asserting
/// otherwise would be asserting that the grouped spelling lies about itself.
struct Case {
    /// What it is, for the failure message.
    what: &'static str,
    /// The grouped path.
    grouped: &'static [&'static str],
    /// The spelling that worked before the grouping.
    flat: &'static [&'static str],
}

/// The representative leaves: one from each area, both `artifact` verbs a plan is read and checked
/// with, the verb that moved area (`eval`), and the one that belongs to no area (`doctor`).
const CASES: &[Case] = &[
    Case {
        what: "govern validate",
        grouped: &["govern", "validate", "--root", "."],
        flat: &["validate", "--root", "."],
    },
    Case {
        what: "govern inspect",
        grouped: &["govern", "inspect", "aep/1"],
        flat: &["inspect", "aep/1"],
    },
    Case {
        what: "govern workflow render",
        grouped: &["govern", "workflow", "render", "--id", "adp/default"],
        flat: &["workflow", "render", "--id", "adp/default"],
    },
    Case {
        what: "plan artifact list",
        grouped: &["plan", "artifact", "list"],
        flat: &["artifact", "list"],
    },
    Case {
        what: "plan artifact validate",
        grouped: &["plan", "artifact", "validate"],
        flat: &["artifact", "validate"],
    },
    Case {
        what: "observe trace check",
        grouped: &[
            "observe",
            "trace",
            "check",
            "--spec",
            SPEC,
            "--transcript",
            TRANSCRIPT,
        ],
        flat: &["trace", "check", "--spec", SPEC, "--transcript", TRANSCRIPT],
    },
    Case {
        what: "drive eval matrix",
        grouped: &["drive", "eval", "matrix", RUNS, "--format", "json"],
        flat: &["eval", "matrix", RUNS, "--format", "json"],
    },
    // `doctor` belongs to no area, so both spellings are the same word. The row is here so the
    // representative set is whole, and so moving it under an area later cannot pass quietly: the
    // flat spelling would stop resolving and this case would say so.
    Case {
        what: "doctor",
        grouped: &["doctor"],
        flat: &["doctor"],
    },
];

/// Asserts every case answers identically under `binary`.
fn every_case_answers_identically(binary: &str, name: &str) {
    for case in CASES {
        let grouped = run(binary, case.grouped);
        let flat = run(binary, case.flat);
        assert_eq!(
            grouped.status.code(),
            flat.status.code(),
            "{name} {}: exit status differs between {:?} and {:?}",
            case.what,
            case.grouped,
            case.flat
        );
        assert_eq!(
            String::from_utf8_lossy(&grouped.stdout),
            String::from_utf8_lossy(&flat.stdout),
            "{name} {}: standard output differs between the grouped and the flat spelling",
            case.what
        );
        assert_eq!(
            String::from_utf8_lossy(&grouped.stderr),
            String::from_utf8_lossy(&flat.stderr),
            "{name} {}: standard error differs between the grouped and the flat spelling — a \
             deprecation line here is exactly what the story refused",
            case.what
        );
    }
}

#[test]
fn the_grouped_path_and_the_flat_spelling_answer_identically_under_aep() {
    every_case_answers_identically(env!("CARGO_BIN_EXE_aep"), "aep");
}

#[test]
fn the_grouped_path_and_the_flat_spelling_answer_identically_under_protocol() {
    every_case_answers_identically(env!("CARGO_BIN_EXE_protocol"), "protocol");
}

/// The cases are only evidence while they actually run the verb.
///
/// Without this, a fixture that moved would make every case above compare one usage error with
/// another usage error and pass — the shape the `--help` carve-out in [`Case`] already warns
/// about, reached by accident instead of on purpose.
#[test]
fn every_case_reaches_its_verb_rather_than_a_usage_error() {
    for case in CASES {
        for arguments in [case.grouped, case.flat] {
            let output = run(env!("CARGO_BIN_EXE_aep"), arguments);
            assert_ne!(
                output.status.code(),
                Some(2),
                "{}: {arguments:?} exited 2, so this case compares two usage errors and asserts \
                 nothing about the verb:\n{}",
                case.what,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// The first level is what a reader is offered, and it is the four areas and the preflight.
///
/// The clap tree already says so in `command_tree`; this says it about the bytes a person sees, and
/// it is the one place the retained spellings are shown to be *absent* from `--help` rather than
/// merely marked hidden in a builder.
#[test]
fn the_help_a_reader_sees_offers_the_four_areas_and_doctor_and_no_flat_spelling() {
    for binary in [env!("CARGO_BIN_EXE_aep"), env!("CARGO_BIN_EXE_protocol")] {
        let help = run(binary, &["--help"]);
        assert_eq!(help.status.code(), Some(0));
        let text = String::from_utf8_lossy(&help.stdout).into_owned();
        let commands = text
            .lines()
            .skip_while(|line| !line.starts_with("Commands:"))
            .skip(1)
            .take_while(|line| !line.trim().is_empty())
            .filter_map(|line| line.split_whitespace().next())
            .collect::<Vec<_>>();
        assert_eq!(
            commands,
            ["govern", "plan", "drive", "observe", "doctor"],
            "the first level a reader is shown, in {binary}:\n{text}"
        );
    }
}
