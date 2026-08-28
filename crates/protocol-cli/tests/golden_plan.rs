//! Nobody can tell: the write verbs leave exactly the documents the recorded binary left, and the
//! read verbs print exactly what it printed.
//!
//! `story:markdown-backend-is-the-adapter`'s golden test. The fixture under
//! `fixtures/golden-plan/` was recorded with the released 0.28.0 binary — the last one whose
//! `MarkdownBackend` had its own hand-written `persist`, latch and journal append: `store/` is a
//! three-artifact plan; `expected/` is that plan after `new`, `relate`, `body`, `move` and
//! `evidence`; `reads/` is what `list`, `board`, `graph`, `validate`, `lifecycle` and `history`
//! printed over it. This binary applies the same five verbs to a copy of `store/` and must produce
//! the same bytes in every document, and the same output for every read.
//!
//! Two amendments since. After wave G's story 4, `validate` says how many documents predate the
//! event log (`4 document(s) predate the event log`, `pre_provider` in JSON), which 0.28.0 could not
//! print. On 2026-08-28 (`story:relation-bumps-a-document-revision-but-not-an-entity`) an edge
//! stopped moving its source document's revision — the contract never counted a relation as one,
//! and every store now agrees — so `expected/` and `reads/` were re-recorded with that build:
//! `story:golden-one` ends at revision 3 rather than 4, and the `depends_on` event sits at the
//! revision the document already had. Everything else is still 0.28.0's bytes.
//!
//! The journal is compared by what it says and not by its bytes: an entry carries the instant it
//! was written and the user who wrote it, and since wave G a new line is the runtime's event rather
//! than the 0.19.0 entry — `journal::read` answers the same entries for both, which is what
//! `history` is compared through.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-plan")
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

/// Every markdown document under `dir`, by relative path.
fn documents(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, dir: &Path, into: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).expect("readable") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(base, &path, into);
            } else if path.extension().is_some_and(|e| e == "md") {
                let relative = path
                    .strip_prefix(base)
                    .expect("under the base")
                    .to_string_lossy()
                    .replace('\\', "/");
                into.insert(relative, std::fs::read_to_string(&path).expect("readable"));
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(dir, dir, &mut found);
    found
}

fn scratch(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("golden-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    copy_tree(&fixture().join("store"), &path);
    path
}

#[test]
fn the_write_verbs_leave_exactly_the_documents_recorded() {
    let store = scratch("writes");
    let body = fixture().join("body.md");
    let body = body.to_str().expect("a printable path");
    let steps: [&[&str]; 5] = [
        &[
            "artifact",
            "new",
            "story",
            "golden-three",
            "--title",
            "Golden three",
            "--relate",
            "decomposes:epic:golden",
        ],
        &[
            "artifact",
            "relate",
            "story:golden-three",
            "depends_on",
            "story:golden-one",
        ],
        &["artifact", "body", "story:golden-two", "--from", body],
        &["artifact", "move", "--to", "proposed", "story:golden-one"],
        &[
            "artifact",
            "evidence",
            "story:golden-one",
            "--kind",
            "test_result",
            "--source",
            "golden",
            "--at",
            "2026-08-28",
        ],
    ];
    for step in steps {
        let output = protocol(&store, step);
        assert!(
            output.status.success(),
            "{step:?} failed:\n{}{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let expected = documents(&fixture().join("expected"));
    let actual = documents(&store);
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "the same documents exist"
    );
    for (path, wanted) in &expected {
        assert_eq!(
            &actual[path], wanted,
            "{path} differs from what was recorded"
        );
    }

    // The journal, by what it says. Five writes on three documents plus one observation: the same
    // entries, in the same order, about the same artifacts, with the same changes and revisions.
    let (recorded, unreadable) = aep_backend_markdown::journal::read(&fixture().join("expected"));
    assert_eq!(unreadable, 0, "the recorded journal reads cleanly");
    let (ours, unreadable) = aep_backend_markdown::journal::read(&store);
    assert_eq!(unreadable, 0, "and so does one with event lines in it");
    let shape = |entries: &[aep_backend_markdown::journal::Entry]| {
        entries
            .iter()
            .map(|entry| {
                format!(
                    "{} {} {} (revision {})",
                    entry.artifact, entry.kind, entry.change, entry.revision
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&ours), shape(&recorded), "the history reads the same");
}

#[test]
fn the_read_verbs_print_exactly_what_was_recorded() {
    let expected = fixture().join("expected");
    let store_path = expected.to_string_lossy().into_owned();
    let reads: [(&[&str], &str); 10] = [
        (&["artifact", "list", "--format", "text"], "list.text"),
        (&["artifact", "list", "--format", "json"], "list.json"),
        (&["artifact", "board", "--format", "text"], "board.text"),
        (&["artifact", "board", "--format", "json"], "board.json"),
        (&["artifact", "graph", "--format", "text"], "graph.text"),
        (&["artifact", "graph", "--format", "json"], "graph.json"),
        (
            &["artifact", "validate", "--format", "text"],
            "validate.text",
        ),
        (
            &["artifact", "validate", "--format", "json"],
            "validate.json",
        ),
        (
            &["artifact", "lifecycle", "story", "--format", "text"],
            "lifecycle.text",
        ),
        (
            &["artifact", "lifecycle", "story", "--format", "json"],
            "lifecycle.json",
        ),
    ];
    for (args, recorded) in reads {
        let output = protocol(&expected, args);
        let printed = format!(
            "{}{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        )
        .replace(&store_path, "<store>");
        let wanted = std::fs::read_to_string(fixture().join("reads").join(recorded))
            .expect("the recording exists");
        assert_eq!(
            printed,
            wanted,
            "`protocol {}` differs from the recording",
            args.join(" ")
        );
    }

    // `history` carries the instant and the user of each write, which no two runs share; what is
    // compared is everything else.
    let output = protocol(
        &expected,
        &[
            "artifact",
            "history",
            "story:golden-one",
            "--format",
            "json",
        ],
    );
    let strip = |text: &str| -> serde_json::Value {
        let mut value: serde_json::Value = serde_json::from_str(text).expect("JSON");
        for entry in value.as_array_mut().expect("a list") {
            let entry = entry.as_object_mut().expect("an entry");
            entry.remove("at");
            entry.remove("actor");
        }
        value
    };
    let wanted =
        std::fs::read_to_string(fixture().join("reads/history.json")).expect("the recording");
    assert_eq!(strip(&stdout(&output)), strip(&wanted));
}
