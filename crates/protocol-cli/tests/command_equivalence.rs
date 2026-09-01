//! The canonical `aep` command and the `protocol` compatibility alias are one interface.

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

#[test]
fn the_canonical_command_and_alias_match_on_a_usage_error() {
    let output = assert_equivalent(&["not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
}
