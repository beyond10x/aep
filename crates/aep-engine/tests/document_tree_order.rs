//! A tree with no `drivers/` reads every other document *exactly* as this repository's tree does.
//!
//! That is the second half of the acceptance sentence on `story:default-step-map`. The first half —
//! *`drivers/` is the last entry of the loader's tree table* — is deliberately **not** here any
//! more. It used to be, as a walk over a fixture this file built, and it was kept beside
//! `document_tree_order_adversarial.rs`'s source scan on the ground that neither could see the
//! other's mutant. That argument died when the sibling built its fixture out of the parsed rows of
//! `const TREE` instead of out of a list:
//!
//! | mutant | the walk this file had | the sibling's source scan | `the_walk_reads_every_row_…` |
//! |---|---|---|---|
//! | a seventh row appended after `drivers` | green | red | **red** |
//! | `TREE` sorted before the walk, table untouched | red | green | **red** |
//!
//! One test that kills both replaces two that each killed one, so the walk is gone from here rather
//! than kept for company. A row *deleted* from `TREE` is killed by neither of those three — measured,
//! not assumed: `the_table_holds_one_row_for_every_kind_the_loader_accepts` is what reddens, because
//! a walk built from the table cannot notice a row the table no longer has.
//!
//! What is left here is the half no source scan reaches: whether the documents outside `drivers/`
//! are still *read*, and read into the same registry, when the directory is not there at all. Built
//! from the table's own rows, that test now reddens under the seventh-row mutant too.
//!
//! **Why the row is last** is the story's reason, not a cross-validation one: `drivers/` was
//! appended **so that no existing tree's load order moves** (`story:default-step-map`, § *Context*).
//! `Registry::validate` runs once the whole tree has been read, so a step map read *before* the
//! workflow it pins is still checked against it — the reading order is not what makes the
//! cross-validation work, and nothing here rests on the claim that it is.
//!
//! The loader's table is private and stays private. Making it `pub` so a test can look at it is
//! production code written for a test's convenience; reading the source is the third option, and
//! `crates/aep-engine/tests/evidence_scan.rs:29-35` is this crate's precedent for taking it.

use std::fs;
use std::path::{Path, PathBuf};

use aep_project::load_tree_report;

/// The workspace root, from this crate's manifest directory.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The directories named by the rows of `const TREE`, in the order they are written.
///
/// Read out of `src/load.rs` rather than derived from [`DocumentKind::directory`], which is a
/// different statement of the same fact and not the one the loader walks: a row may name a
/// directory no kind maps to, and a kind's directory may be renamed without the row moving. A
/// fixture built from the wrong one of the two goes quiet instead of red — renaming `Workflow`'s
/// directory in `aep-schema` used to make this file stop copying `workflows/` altogether, with
/// nothing here complaining. `document_tree_order_adversarial.rs::tree_rows` reads the same table
/// for the same reason; the two cannot share it, because integration tests are separate binaries
/// and the only place to put a shared helper is a file this unit may not edit.
fn tree_directories() -> Vec<String> {
    let source = fs::read_to_string(repository_root().join("crates/aep-project/src/load.rs"))
        .expect("the loader's module is readable");
    let mut inside = false;
    let mut directories = Vec::new();
    for line in source.lines() {
        if line.starts_with("const TREE: &[(&str, DocumentKind)] = &[") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with("];") {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let directory = trimmed
            .trim_start_matches('(')
            .split_once(", DocumentKind::")
            .unwrap_or_else(|| {
                panic!("a row of TREE reads `(\"dir\", DocumentKind::Kind),`: {trimmed}")
            })
            .0;
        directories.push(directory.trim().trim_matches('"').to_string());
    }
    assert!(
        !directories.is_empty(),
        "the extractor found no rows in `const TREE`; it has stopped seeing the table it reads"
    );
    directories
}

/// A throwaway tree root under this crate's target directory.
///
/// The process id is in the path because `CARGO_TARGET_TMPDIR` is shared between concurrent runs,
/// and a fixture path a second process can compute is a fixture a second process will delete. The
/// tree is removed once the test has passed; a failing one leaves it to be looked at.
fn fixture_root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("document-tree-order-{name}-{}", std::process::id()));
    fs::remove_dir_all(&root).ok();
    root
}

/// Copies every file under `from` into `to`, keeping the layout.
fn copy_documents(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the fixture tree is writable");
    for entry in fs::read_dir(from).expect("the source directory is readable") {
        let entry = entry.expect("the directory entry is readable");
        let path = entry.path();
        if path.is_dir() {
            copy_documents(&path, &to.join(entry.file_name()));
        } else {
            fs::copy(&path, to.join(entry.file_name())).expect("the document is copyable");
        }
    }
}

/// How many files under `directory` the loader would read as documents.
///
/// Mirrors `collect_documents` (`src/load.rs:246-268`) in both respects that matter here. The
/// leading-dot test comes **before** the `is_dir` branch, so a `drivers/.archive/` is skipped whole
/// and nothing inside it counts; that premise is not this file's to hold, and
/// `document_tree_order_adversarial.rs::a_dot_prefixed_directory_under_drivers_is_not_descended_into`
/// is what goes red if the loader stops being in that order. The extension list is a copy of
/// `EXTENSIONS` (`src/load.rs:37`), which is private; it can only drift once a new extension is
/// added *and* a file of it lands under `drivers/`.
fn document_count(directory: &Path) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(directory).expect("the directory is readable") {
        let entry = entry.expect("the directory entry is readable");
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            count += document_count(&path);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "yaml" | "yml" | "json"))
        {
            count += 1;
        }
    }
    count
}

/// Every failure of a load, one per line, for an assertion message.
fn reported(outcome: &aep_project::LoadOutcome) -> String {
    outcome
        .failures
        .iter()
        .map(|failure| format!("  - {failure}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A tree with no `drivers/` reads every other document exactly as this repository's own tree does.
///
/// *Clean* is not the claim — *unchanged* is, which is why the baseline is a load of the real tree
/// with real step maps in it rather than a synthetic one. Every row of `TREE` except `drivers` is
/// copied, and a row whose directory this repository does not have is an assertion failure rather
/// than a silent skip: a fixture that quietly builds less than it meant to is how the load-order
/// half of this file went green while testing five directories instead of six.
#[test]
fn a_repository_with_no_drivers_directory_loads_exactly_as_before() {
    let root = repository_root();
    assert!(
        root.join("drivers").is_dir(),
        "the fixture reaches the state only if this repository does have a drivers/ directory"
    );

    let before = load_tree_report(&root);
    assert!(
        before.failures.is_empty(),
        "this repository's own tree loads:\n{}",
        reported(&before)
    );

    let rows = tree_directories();
    assert_eq!(
        rows.last().map(String::as_str),
        Some("drivers"),
        "this test takes `drivers` off the end of the table; the table reads {rows:?}"
    );

    let copy = fixture_root("no-drivers");
    let mut copied = 0_usize;
    for directory in rows.iter().filter(|directory| *directory != "drivers") {
        let source = root.join(directory);
        assert!(
            source.is_dir(),
            "`{directory}` is a row of the loader's table and this repository has no such \
             directory, so the fixture would be built out of less than the tree it is compared to"
        );
        copy_documents(&source, &copy.join(directory));
        copied += 1;
    }
    assert_eq!(
        copied,
        rows.len() - 1,
        "every row but `drivers` is copied; the table reads {rows:?}"
    );
    assert!(
        !copy.join("drivers").exists(),
        "the copied tree is the one without a drivers/ directory"
    );

    let after = load_tree_report(&copy);

    assert!(
        after.failures.is_empty(),
        "a missing drivers/ directory is skipped, not reported:\n{}",
        reported(&after)
    );
    assert_eq!(
        after.files_read,
        before.files_read - document_count(&root.join("drivers")),
        "only the step maps go missing; every other document is still read"
    );
    assert_eq!(
        after.registry.protocols().count(),
        before.registry.protocols().count()
    );
    assert_eq!(
        after.registry.principles().count(),
        before.registry.principles().count()
    );
    assert_eq!(
        after.registry.workflows().count(),
        before.registry.workflows().count()
    );
    assert_eq!(
        after.registry.profiles().count(),
        before.registry.profiles().count()
    );
    assert_eq!(
        after.registry.lifecycles().len(),
        before.registry.lifecycles().len()
    );

    fs::remove_dir_all(&copy).ok();
}
