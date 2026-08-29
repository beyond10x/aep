//! `protocol specification evidence` decides **this task's** specification, in a store that holds
//! somebody else's.
//!
//! The defect, measured: run `NATIVE-1/1` (2026-08-29) satisfied
//! `spec-driven.before_implementation` on two approved specifications belonging to two other
//! stories, because the rule was a query over the whole store.
//! `story:task-scoped-artifact-requirements` bound the guard; this file holds the other half of
//! it. Until that follow-up the verb still picked any in-force specification, which left it
//! **looser than the guard it serves** — a run could write a `specification` record about a
//! document `before_implementation` would refuse, and `specification.satisfied` would then be a
//! verdict about somebody else's story.
//!
//! # Why through the binary
//!
//! `crates/protocol-cli/src/specification.rs` tests the selection directly, over artifacts, which
//! is where a rule belongs. What only a test through the binary can show is the property a driven
//! step depends on: **one store answers two tasks differently**, and the document the verb wrote
//! its record about is named in the record a driver reads back. Both runs below are the same store
//! and the same command line but for `--task`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use aep_domain::evidence::Evidence;

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args` from the repository root.
fn protocol(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

/// A scratch directory of this test's own, emptied first so a rerun is a fresh run.
///
/// Under `CARGO_TARGET_TMPDIR`, which is this workspace's own `target/`: a fixture store written
/// into a shared temporary directory is one two checkouts can collide over.
fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("spec-binding-{name}"));
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

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An approved specification of one story, carrying the `specifies` edge a driven run's `specify`
/// state writes.
fn specification(directory: &Path, name: &str, story: &str) {
    write(
        &directory.join(format!("specification/{name}.md")),
        &format!(
            "---\nformat: aep.planning-md/1\nid: specification:{name}\nkind: specification\n\
             status: approved\ntitle: {name}\nsummary: What {name} must do.\n\
             relations:\n- specifies: story:{story}\n---\n# Specification\n\n## Acceptance\n\n\
             - The unit suite is green: `tests.unit.failed == 0`\n"
        ),
    );
}

/// A task document decomposed from one story.
fn task(directory: &Path, id: &str, story: &str) -> PathBuf {
    let path = directory.join(format!("task-{id}.yaml"));
    write(
        &path,
        &format!(
            "id: {id}\n\
             kind: feature\n\
             objective: {story}\n\
             protocol: adp/1\n\
             profile: development.fast\n\
             derived_from:\n  - story:{story}\n"
        ),
    );
    path
}

/// **The store both tasks are decided against**: two approved specifications, of two stories.
///
/// Both are `approved`, both state a requirement, and both are perfectly good documents. Nothing
/// about either one is wrong — which is the point: the only thing that tells them apart is whose
/// work each is about, and before the binding the verb had no way to ask.
fn store(name: &str) -> PathBuf {
    let directory = scratch(name);
    specification(&directory, "passkeys", "passkeys");
    specification(&directory, "sessions", "sessions");
    directory
}

/// The specification a written record is about, or a panic saying what the document was instead.
///
/// Read through `aep_schema::parse::evidence_list`, the reader `protocol evaluate --evidence` and
/// the driver both use, so a document this accepts is one a driven step can submit.
fn subject_of(path: &Path) -> String {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let mut records = aep_schema::parse::evidence_list(&text, Some(printable(path)))
        .unwrap_or_else(|error| panic!("{} is not an evidence document: {error}", path.display()));
    assert_eq!(
        records.len(),
        1,
        "a step establishes one thing, and the driver refuses a record file holding several: {text}"
    );
    let Evidence::Specification(payload) = records.remove(0).evidence else {
        panic!("the payload is a specification record");
    };
    payload
        .artifact
        .expect("the record names the specification it is about")
        .id()
        .to_string()
}

#[test]
fn one_store_decides_two_tasks_differently_and_each_record_names_its_own_specification() {
    let directory = store("two-tasks");
    let passkeys = task(&directory, "PASSKEYS-1", "passkeys");
    let sessions = task(&directory, "SESSIONS-1", "sessions");

    for (task, expected) in [
        (&passkeys, "specification:passkeys"),
        (&sessions, "specification:sessions"),
    ] {
        let out = directory.join("specification.yaml");
        std::fs::remove_file(&out).ok();
        let output = protocol(&[
            "specification",
            "evidence",
            "--store",
            printable(&directory),
            "--task",
            printable(task),
            "--out",
            printable(&out),
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "one of the two specifications is this task's: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            subject_of(&out),
            expected,
            "the record is about the specification that specifies this task's work, and the store \
             holds another story's approved one beside it"
        );
    }
}

#[test]
fn a_task_no_specification_in_the_store_is_about_is_refused_and_nothing_is_written() {
    // The refusal the driver reads as D5's `Unknown`: nothing observed, the step submits nothing,
    // and the run stops at the guard rather than moving on a record about the wrong document.
    let directory = store("unrelated-task");
    let unrelated = task(&directory, "BILLING-1", "billing");
    let out = directory.join("specification.yaml");

    let output = protocol(&[
        "specification",
        "evidence",
        "--store",
        printable(&directory),
        "--task",
        printable(&unrelated),
        "--out",
        printable(&out),
    ]);
    let said = String::from_utf8_lossy(&output.stderr).into_owned();

    assert_eq!(output.status.code(), Some(1), "{said}");
    for named in [
        "specification:passkeys (approved)",
        "specification:sessions (approved)",
        "this task's work is story:billing, task:BILLING-1",
        "which specifies this task",
    ] {
        assert!(
            said.contains(named),
            "the refusal names both ends — what is declared, and what the task said it was about; \
             `{named}` is missing:\n{said}"
        );
    }
    assert!(
        !out.exists(),
        "a refusal writes no record: two approved specifications are present and neither is this \
         task's, which is exactly the state that used to produce one"
    );
}

#[test]
fn an_artifact_named_on_the_command_line_does_not_lift_the_binding() {
    // The half that decides whether this is a rule or a suggestion. If naming a document were
    // enough, a step map, a script or an agent could mint the record `spec-driven` exists to
    // withhold by adding one flag.
    let directory = store("named-artifact");
    let passkeys = task(&directory, "PASSKEYS-1", "passkeys");
    let out = directory.join("specification.yaml");

    let output = protocol(&[
        "specification",
        "evidence",
        "--store",
        printable(&directory),
        "--task",
        printable(&passkeys),
        "--artifact",
        "specification:sessions",
        "--out",
        printable(&out),
    ]);
    let said = String::from_utf8_lossy(&output.stderr).into_owned();

    assert_eq!(output.status.code(), Some(1), "{said}");
    for named in [
        "specification:sessions",
        "does not lift the binding",
        "this task's work is story:passkeys, task:PASSKEYS-1",
    ] {
        assert!(
            said.contains(named),
            "the refusal says which document was named, that naming it is not enough, and what \
             this task is about; `{named}` is missing:\n{said}"
        );
    }
    assert!(
        !out.exists(),
        "and it writes nothing, so there is no record of a specification the guard would refuse"
    );
}
