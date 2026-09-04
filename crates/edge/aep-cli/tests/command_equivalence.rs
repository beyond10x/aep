//! The canonical `aep` command and the `protocol` compatibility alias are one interface.
//!
//! Invariant 10, and it is checked over **both spellings of every case**. The first level of the
//! command line is now the four area names, with every verb that reached 0.51.0 kept as a hidden
//! top-level alias — which is a second surface, and a second surface is where an alias quietly
//! stops being exact. `aep govern schema` and `aep schema` reach one dispatch arm carrying one
//! value, and this file is what says so from outside the process, for the accepted, the refused
//! and the mistyped.

use std::process::{Command, Output};

fn run(binary: &str, arguments: &[&str]) -> Output {
    Command::new(binary)
        .args(arguments)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the CLI starts")
}

fn assert_equivalent(arguments: &[&str]) -> Output {
    let canonical = run(env!("CARGO_BIN_EXE_aep"), arguments);
    let compatibility = run(env!("CARGO_BIN_EXE_protocol"), arguments);
    assert_eq!(
        canonical.status.code(),
        compatibility.status.code(),
        "exit status differs for {arguments:?}"
    );
    assert_eq!(
        canonical.stdout, compatibility.stdout,
        "stdout differs for {arguments:?}"
    );
    assert_eq!(
        canonical.stderr, compatibility.stderr,
        "stderr differs for {arguments:?}"
    );
    canonical
}

#[test]
fn the_canonical_command_and_alias_match_on_an_accepted_operation() {
    let output = assert_equivalent(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn the_canonical_command_and_alias_match_on_a_domain_refusal() {
    let output = assert_equivalent(&[
        "artifact",
        "move",
        "story:no-such-artifact",
        "--to",
        "active",
    ]);
    assert_eq!(output.status.code(), Some(1));
}

/// `doctor` is the one verb whose whole output is a report about the installation, so the two names
/// disagreeing here would be the alias reporting on a different binary than the one that answered.
/// Run at this crate's own directory, which holds no project file: the report has failures in it,
/// which exercises the exit status as well as the bytes.
#[test]
fn the_canonical_command_and_alias_match_on_a_preflight_that_reports_failures() {
    let output = assert_equivalent(&["doctor"]);
    assert_eq!(output.status.code(), Some(1));
}

/// The same, in the rendering a script reads.
#[test]
fn the_canonical_command_and_alias_match_on_a_preflight_rendered_as_json() {
    assert_equivalent(&["doctor", "--format", "json"]);
}

#[test]
fn the_canonical_command_and_alias_match_on_a_usage_error() {
    let output = assert_equivalent(&["not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
}

// ------------------------------------------------------------------------------------------
// The same three shapes at the grouped spelling, and inside an area.
//
// A usage error is included at both levels on purpose. Its text names the path it was invoked by,
// so the grouped and the flat spelling are *supposed* to print different usage lines — but `aep`
// and `protocol` must still print the same one as each other, which is the thing this file is for.
// ------------------------------------------------------------------------------------------

/// The accepted case at both spellings, and the same bytes out of both.
///
/// `schema` prints the built-in schema names and needs no project, so it is the cheapest verb that
/// actually produces output rather than a version string.
#[test]
fn the_canonical_command_and_alias_match_on_an_accepted_operation_at_both_spellings() {
    let grouped = assert_equivalent(&["govern", "schema"]);
    assert_eq!(grouped.status.code(), Some(0));
    let flat = assert_equivalent(&["schema"]);
    assert_eq!(flat.status.code(), Some(0));
    assert_eq!(
        grouped.stdout, flat.stdout,
        "the grouped path and its retained flat spelling print the same schema list"
    );
}

/// The refused case at the grouped spelling.
#[test]
fn the_canonical_command_and_alias_match_on_a_domain_refusal_at_the_grouped_spelling() {
    let output = assert_equivalent(&[
        "plan",
        "artifact",
        "move",
        "story:no-such-artifact",
        "--to",
        "active",
    ]);
    assert_eq!(output.status.code(), Some(1));
}

/// A verb added after the grouping, at both spellings: `unrelate` is reached through the `artifact`
/// group, so its hidden flat alias is built rather than written, and invariant 10 covers it the day
/// it ships rather than the day somebody notices.
///
/// A refusal, because it needs no store to be prepared and the refusal path is where two binaries
/// most easily print different bytes.
#[test]
fn the_canonical_command_and_alias_match_on_unrelate_at_both_spellings() {
    let grouped = assert_equivalent(&[
        "plan",
        "artifact",
        "unrelate",
        "story:no-such-artifact",
        "decomposes",
        "epic:no-such-epic",
    ]);
    assert_eq!(grouped.status.code(), Some(1));
    let flat = assert_equivalent(&[
        "artifact",
        "unrelate",
        "story:no-such-artifact",
        "decomposes",
        "epic:no-such-epic",
    ]);
    assert_eq!(flat.status.code(), Some(1));
    assert_eq!(
        grouped.stderr, flat.stderr,
        "the grouped path and its hidden flat spelling refuse in the same words"
    );
}

/// A mistyped verb inside an area.
#[test]
fn the_canonical_command_and_alias_match_on_a_usage_error_inside_an_area() {
    let output = assert_equivalent(&["govern", "not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
}

/// A mistyped verb under a retained flat spelling.
#[test]
fn the_canonical_command_and_alias_match_on_a_usage_error_under_a_flat_spelling() {
    let output = assert_equivalent(&["artifact", "not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
}

/// The first level itself, which is the part of the surface this grouping changed.
///
/// `--help` is where a rendering difference between the two names would show first, and it is the
/// one page a reader of either binary is sent to.
#[test]
fn the_canonical_command_and_alias_match_on_the_first_level_help() {
    let output = assert_equivalent(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let listed = String::from_utf8(output.stdout).expect("the help is text");
    for area in ["govern", "plan", "drive", "observe", "doctor"] {
        assert!(
            listed.contains(area),
            "both names offer the same first level; `{area}` is missing from:\n{listed}"
        );
    }
}
