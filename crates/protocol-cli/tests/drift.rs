//! Wave G, story 4: an out-of-band edit, and a deleted document, are reported by `validate`.
//!
//! The fixture is the golden plan after one move made through this binary. Since the fixture was
//! re-recorded (2026-08-28, `golden_plan.rs`) the three stories carry events from their own
//! recording and the epic alone predates the log.
//!
//! Since 2026-08-29 a fourth case is here: a **forged revision**, a number higher than any event
//! for the document records. The `native-arm-store-integrity` design § 4 found one written into a
//! store by a model's whole-file rewrite, and § 8 OQ5 asked what catches it. This does; it does not
//! refuse it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn protocol(store: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .arg("--store")
        .arg(store)
        .arg("--root")
        .arg(root())
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the target exists");
    for entry in std::fs::read_dir(from).expect("readable") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copied");
        }
    }
}

/// The golden plan, with `story:golden-one` moved once through this binary.
fn a_plan_with_one_event(name: &str) -> PathBuf {
    let store = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("drift-{name}"));
    let _ = std::fs::remove_dir_all(&store);
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-plan/expected"),
        &store,
    );
    let moved = protocol(
        &store,
        &["artifact", "move", "--to", "active", "story:golden-one"],
    );
    assert!(
        moved.status.success(),
        "{}",
        String::from_utf8_lossy(&moved.stderr)
    );
    store
}

#[test]
fn a_document_edited_in_an_editor_is_drift_naming_the_field_and_the_event() {
    let store = a_plan_with_one_event("edited");
    // A second event, so the fold has a field the last event did not write: the move wrote
    // `status`, the edge writes `relations`.
    let related = protocol(
        &store,
        &[
            "artifact",
            "relate",
            "story:golden-one",
            "depends_on",
            "story:golden-two",
        ],
    );
    assert!(
        related.status.success(),
        "{}",
        String::from_utf8_lossy(&related.stderr)
    );
    let path = store.join("story/golden-one.md");
    let text = std::fs::read_to_string(&path).expect("the document");
    assert!(text.contains("status: active"), "the move landed:\n{text}");
    assert!(
        text.contains("- depends_on: story:golden-two"),
        "the edge landed:\n{text}"
    );
    std::fs::write(
        &path,
        text.replace("status: active", "status: implemented")
            .replace("- depends_on: story:golden-two\n", ""),
    )
    .expect("the edit");

    let output = protocol(&store, &["artifact", "validate"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "drift is a problem: {}",
        stdout(&output)
    );
    let text = stdout(&output);
    assert!(
        text.contains("story:golden-one drifted from its log"),
        "{text}"
    );
    assert!(text.contains("status"), "names the status field: {text}");
    assert!(
        text.contains("relations"),
        "and the edge the fold knows: {text}"
    );
    assert!(
        text.contains("event story:golden-one@"),
        "and the event: {text}"
    );

    let json = protocol(&store, &["artifact", "validate", "--format", "json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&json)).expect("JSON");
    assert_eq!(parsed["drift"].as_array().map(Vec::len), Some(1));
    assert!(parsed.get("deleted").is_none(), "nothing was deleted");
    assert_eq!(parsed["pre_provider"], 1, "the epic alone predates the log");
}

#[test]
fn a_document_removed_with_rm_is_reported_as_deleted() {
    let store = a_plan_with_one_event("removed");
    std::fs::remove_file(store.join("story/golden-one.md")).expect("rm");

    let output = protocol(&store, &["artifact", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let text = stdout(&output);
    assert!(text.contains("story:golden-one was deleted"), "{text}");
    assert!(
        text.contains("event story:golden-one@"),
        "names the last event: {text}"
    );
    assert!(
        !text.contains("the journal holds"),
        "said once, as a deletion, not also as the journal's orphan: {text}"
    );
}

#[test]
fn a_document_that_matches_its_log_is_not_drift_and_a_plan_before_the_log_is_not_either() {
    let store = a_plan_with_one_event("clean");
    let output = protocol(&store, &["artifact", "validate", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    assert!(parsed.get("drift").is_none());
    assert!(parsed.get("forged").is_none());
    assert!(parsed.get("deleted").is_none());
    assert_eq!(parsed["pre_provider"], 1);

    // The store as 0.28.0 left it: four documents, no events, exit 0, and the count says so.
    let untouched = Path::new(env!("CARGO_TARGET_TMPDIR")).join("drift-untouched");
    let _ = std::fs::remove_dir_all(&untouched);
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-plan/expected"),
        &untouched,
    );
    let output = protocol(&untouched, &["artifact", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("1 document(s) predate the event log"),
        "{}",
        stdout(&output)
    );
}

/// A revision no write produced is its own finding, and it is not called an editor's edit.
///
/// The case the `native-arm-store-integrity` design § 4 recorded: `revision: 99` written straight
/// into a document. Ordinary drift already reported the disagreement, but as *somebody edited a
/// field*, which sends the reader to the log for a state that was never in it.
#[test]
fn a_revision_no_write_produced_is_reported_as_forged_and_not_as_an_edit() {
    let store = a_plan_with_one_event("forged");
    let path = store.join("story/golden-one.md");
    let text = std::fs::read_to_string(&path).expect("the document");
    assert!(
        text.contains("revision: 4"),
        "the move left it at revision 4:\n{text}"
    );
    std::fs::write(&path, text.replace("revision: 4", "revision: 99")).expect("the edit");

    let output = protocol(&store, &["artifact", "validate"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a forged revision is a problem: {}",
        stdout(&output)
    );
    let text = stdout(&output);
    assert!(
        text.contains("story:golden-one claims revision 99, and no write produced it"),
        "{text}"
    );
    assert!(
        text.contains("3 event(s) logged"),
        "and says how thin the record is: {text}"
    );
    assert!(
        text.contains("the highest at revision 4"),
        "and what the log actually left behind: {text}"
    );
    assert!(
        !text.contains("drifted from its log"),
        "said once, as the finding it is: {text}"
    );
    assert!(
        !text.contains("the journal holds"),
        "and not a second time in the journal's older words: {text}"
    );

    let json = protocol(&store, &["artifact", "validate", "--format", "json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&json)).expect("JSON");
    assert_eq!(parsed["forged"].as_array().map(Vec::len), Some(1));
    assert!(parsed.get("drift").is_none(), "not also ordinary drift");
    assert_eq!(parsed["problems"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        parsed["pre_provider"], 1,
        "and the count of documents predating the log is unchanged"
    );
}

/// A document at the revision its events record is not forged, however many events there are.
///
/// `story:golden-three` sits at revision 1 under three events, and an evidence record adds a fourth
/// that writes nothing and moves no revision. Counting events rather than reading the revision they
/// record would call every one of these a forgery.
#[test]
fn a_document_with_more_events_than_its_revision_is_not_forged() {
    let store = a_plan_with_one_event("more-events-than-revision");
    let recorded = protocol(
        &store,
        &[
            "artifact",
            "evidence",
            "story:golden-three",
            "--kind",
            "test_result",
            "--source",
            "golden",
        ],
    );
    assert!(
        recorded.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded.stderr)
    );
    let text = std::fs::read_to_string(store.join("story/golden-three.md")).expect("the document");
    assert!(
        text.contains("revision: 1"),
        "an observation writes nothing and moves no revision:\n{text}"
    );

    let output = protocol(&store, &["artifact", "validate", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    assert!(parsed.get("forged").is_none(), "{parsed}");
    assert!(parsed.get("drift").is_none(), "{parsed}");
}

/// The forged check does not reach a document the log has never heard of.
///
/// `pre_provider` keeps its meaning — the store as it was before the provider is not a defect — and
/// the count still prints. Whether a driven run's scratch store should read it differently is open
/// (`docs/design/native-arm-store-integrity-design-v0.1.md` § 8, OQ5) and is not decided here.
#[test]
fn a_forged_revision_on_a_document_with_no_events_is_still_only_a_document_predating_the_log() {
    let store = Path::new(env!("CARGO_TARGET_TMPDIR")).join("drift-forged-pre-provider");
    let _ = std::fs::remove_dir_all(&store);
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-plan/expected"),
        &store,
    );
    std::fs::remove_file(store.join("journal.jsonl")).expect("no log at all");
    let path = store.join("story/golden-one.md");
    let text = std::fs::read_to_string(&path).expect("the document");
    std::fs::write(&path, text.replace("revision: 3", "revision: 99")).expect("the edit");

    let output = protocol(&store, &["artifact", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("4 document(s) predate the event log"),
        "{}",
        stdout(&output)
    );
}
