//! `protocol reverse openapi` drafts a `relations:` block, and the draft is compared byte for byte.
//!
//! `story:reverse-openapi-emits-relations`. Every other fixture under `reverse_cli.rs` is written by
//! the test that reads it, for a reason that file states: a scanner is the tool that would carry a
//! real repository's domain language into a public one. This test keeps the same rule — the document
//! under `fixtures/reverse-openapi/` is invented — and breaks the other half of it deliberately, the
//! way `golden_plan.rs` does: the expected draft is recorded on disk, so a change to a single
//! rendered byte shows up as a diff a reviewer reads rather than as a predicate that stopped
//! holding.
//!
//! The document is built so that each schema exercises exactly one arm of the projection: a `$ref`,
//! an array of `$ref`, an `<x>_id`, an `<x>Id`, a composition that states one shape and one that
//! states two, and five properties that carry no signal at all and must therefore produce nothing.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace root, which is the directory the draft's header path is relative to.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The fixture document, named the way the recorded draft's header names it.
const DOCUMENT: &str = "crates/edge/aep-cli/tests/fixtures/reverse-openapi/order-service.yaml";

/// The recorded draft.
const RECORDED: &str = "crates/edge/aep-cli/tests/fixtures/reverse-openapi/expected/draft.yaml";

#[test]
fn the_drafted_domain_is_the_recorded_bytes() {
    let root = root();
    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["reverse", "openapi", DOCUMENT, "--domain", "acme.order"])
        .current_dir(&root)
        .output()
        .expect("the protocol binary runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let drafted = String::from_utf8_lossy(&output.stdout).into_owned();
    let recorded = std::fs::read_to_string(root.join(RECORDED)).expect("the recorded draft exists");

    if drafted != recorded {
        let mut report = String::new();
        for (index, (left, right)) in recorded.lines().zip(drafted.lines()).enumerate() {
            if left != right {
                let _ = writeln!(report, "line {}", index + 1);
                let _ = writeln!(report, "  recorded: {left}");
                let _ = writeln!(report, "  drafted:  {right}");
            }
        }
        if recorded.lines().count() != drafted.lines().count() {
            let _ = writeln!(
                report,
                "recorded {} lines, drafted {}",
                recorded.lines().count(),
                drafted.lines().count()
            );
        }
        panic!("the draft is not the recorded bytes:\n{report}");
    }
}

#[test]
fn a_schema_with_no_relation_signal_carries_no_relations_block() {
    // The half of the contract that is an absence. `Customer`, `Warehouse` and `Reference` hold
    // nothing but scalars, so a `relations:` key on any of them would be a guess.
    let root = root();
    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["reverse", "openapi", DOCUMENT, "--domain", "acme.order"])
        .current_dir(&root)
        .output()
        .expect("the protocol binary runs");
    let drafted = String::from_utf8_lossy(&output.stdout).into_owned();

    let blocks = drafted.matches("    relations:").count();
    assert_eq!(
        blocks, 1,
        "only `acme.order.Order` has a relation signal, so only it may carry a block:\n{drafted}"
    );
}
