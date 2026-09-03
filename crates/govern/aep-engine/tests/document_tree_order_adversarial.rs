//! Adversarial cases for `story:default-step-map`: the two states its own tests cannot reach.
//!
//! `tests/document_tree_order.rs` reads the loader's table indirectly, through the order failures
//! come back in from a fixture it built. That is sound as far as it goes, and it goes exactly as
//! far as the fixture: a row of `TREE` naming a directory the fixture never creates is invisible to
//! it, and a directory that exists but yields nothing is a state neither of its two trees is in.
//!
//! Both gaps are demonstrated rather than argued — each case below is green on the tree as it
//! stands and red under a one-line change to `crates/edge/aep-project/src/load.rs` that the rest of the
//! `aep-engine` suite does not notice.

use std::fs;
use std::path::{Path, PathBuf};

use aep_project::load_tree_report;
use aep_schema::parse::DocumentKind;

/// The loader's module, read as text.
///
/// The table is private and must stay private — making it `pub` to let a test look at it is
/// production code written for a test's convenience, which is the reason the sibling file gives for
/// not doing it. Reading the source is the third option, and this crate already takes it for a rule
/// whose failure mode is not observable from inside a running test
/// (`crates/govern/aep-engine/tests/evidence_scan.rs:29-35`). "`drivers/` is the *last entry of the tree
/// table*" is such a rule: a seventh row is not observable from any walk of a six-directory tree.
fn loader_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../edge/aep-project/src/load.rs");
    fs::read_to_string(&path).expect("the loader's module is readable")
}

/// The rows of `const TREE`, in the order they are written, as `(directory, kind)` pairs.
fn tree_rows() -> Vec<(String, String)> {
    let source = loader_source();
    let mut inside = false;
    let mut rows = Vec::new();
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
        let row = trimmed
            .trim_start_matches('(')
            .trim_end_matches(',')
            .trim_end_matches(')');
        let (directory, kind) = row.split_once(", DocumentKind::").unwrap_or_else(|| {
            panic!("a row of TREE reads `(\"dir\", DocumentKind::Kind),`: {row}")
        });
        rows.push((
            directory.trim().trim_matches('"').to_string(),
            kind.trim().to_string(),
        ));
    }
    assert!(
        !rows.is_empty(),
        "the extractor found no rows in `const TREE`; it has stopped seeing the table it reads"
    );
    rows
}

/// A throwaway tree root, named for the case and the process that built it.
fn fixture_root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("adversarial-tree-{name}-{}", std::process::id()));
    fs::remove_dir_all(&root).ok();
    root
}

/// A tree with one of every non-`drivers` document directory, populated from this repository.
///
/// Built from the real documents so that a load of it is a real load: an empty tree loads clean for
/// the uninteresting reason that there is nothing in it to be wrong.
fn tree_without_drivers(name: &str) -> PathBuf {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists");
    let copy = fixture_root(name);
    for directory in [
        "protocols",
        "principles",
        "workflows",
        "profiles",
        "artifacts/lifecycles",
    ] {
        copy_tree(&repository.join(directory), &copy.join(directory));
    }
    assert!(
        !copy.join("drivers").exists(),
        "the base fixture has no drivers/ of any kind yet"
    );
    copy
}

/// Copies `from` into `to`, keeping the layout.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the fixture tree is writable");
    for entry in fs::read_dir(from).expect("the source directory is readable") {
        let entry = entry.expect("the directory entry is readable");
        let path = entry.path();
        if path.is_dir() {
            copy_tree(&path, &to.join(entry.file_name()));
        } else {
            fs::copy(&path, to.join(entry.file_name())).expect("the document is copyable");
        }
    }
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

/// `drivers` is the last **row of the loader's table**, which is not what a six-directory walk says.
///
/// `drivers_is_the_last_directory_the_loader_walks` asserts that `drivers` came last among the six
/// directories its fixture created. A seventh row appended to `TREE` — `("releases", ..)`, say —
/// names a directory that fixture does not create, so the loader skips it (`src/load.rs:133-135`),
/// the walk still ends at `drivers`, and that test stays green while the acceptance sentence
/// *"`drivers/` is the last entry of the loader's tree table"* has become false. Verified: with such
/// a row added, `cargo test -p aep-engine` exits 0.
#[test]
fn drivers_is_the_last_row_of_the_loaders_table() {
    let rows = tree_rows();
    let written: Vec<String> = rows
        .iter()
        .map(|(directory, kind)| format!("{directory} => {kind}"))
        .collect();

    assert_eq!(
        rows.last()
            .map(|(directory, kind)| (directory.as_str(), kind.as_str())),
        Some(("drivers", "StepMap")),
        "`drivers` must be the last row of `const TREE`, not merely the last directory a fixture \
         with no seventh directory in it happens to reach; the table reads {written:?}"
    );

    for expected in [
        "protocols",
        "principles",
        "workflows",
        "profiles",
        "artifacts/lifecycles",
        "drivers",
    ] {
        assert!(
            rows.iter().any(|(directory, _)| directory == expected),
            "`{expected}` is no longer a row of the loader's table; it reads {written:?}"
        );
    }

    let workflows = rows
        .iter()
        .position(|(directory, _)| directory == "workflows")
        .expect("workflows is a row");
    assert!(
        workflows < rows.len() - 1,
        "`workflows` is read before `drivers` in the table itself, not only in a fixture's walk; \
         the table reads {written:?}"
    );
}

/// A `drivers/` that exists and yields nothing loads exactly like a `drivers/` that is absent.
///
/// `a_repository_with_no_drivers_directory_loads_exactly_as_before` compares two trees: one with a
/// populated `drivers/` and one with no `drivers/` entry at all. *Present but yielding nothing* is
/// a third state and is the one this repository's own history is in — `story:default-step-map`
/// records `drivers/` as "a reserved directory name with nothing writing to it since wave 2", which
/// is an adopter with an empty `drivers/`, not an adopter without one. Verified: making an empty
/// tree directory push a failure leaves `cargo test -p aep-engine` at exit 0.
#[test]
fn a_drivers_directory_that_yields_no_documents_loads_like_one_that_is_absent() {
    let absent = tree_without_drivers("absent");
    let baseline = load_tree_report(&absent);
    assert!(
        baseline.failures.is_empty(),
        "the tree with no drivers/ is the baseline and must load clean:\n{}",
        reported(&baseline)
    );
    assert_eq!(
        baseline.drivers.len(),
        0,
        "a tree with no drivers/ carries no step maps"
    );

    // Empty.
    let empty = tree_without_drivers("empty-drivers");
    fs::create_dir_all(empty.join("drivers")).expect("the fixture tree is writable");
    let outcome = load_tree_report(&empty);
    assert!(
        outcome.failures.is_empty(),
        "an empty drivers/ is not a failure, it is a tree nobody has written a map for yet:\n{}",
        reported(&outcome)
    );
    assert_eq!(
        outcome.files_read, baseline.files_read,
        "an empty drivers/ adds no documents to read"
    );
    assert_eq!(outcome.drivers.len(), 0);

    // Present, holding only a file the loader does not treat as a document.
    let prose = tree_without_drivers("prose-drivers");
    fs::create_dir_all(prose.join("drivers")).expect("the fixture tree is writable");
    fs::write(prose.join("drivers/README.md"), "step maps go here\n")
        .expect("the note is writable");
    let outcome = load_tree_report(&prose);
    assert!(
        outcome.failures.is_empty(),
        "a drivers/ holding only prose is not a failure:\n{}",
        reported(&outcome)
    );
    assert_eq!(
        outcome.files_read, baseline.files_read,
        "a non-document in drivers/ is not read as one"
    );

    // Present, and not a directory at all.
    let file = tree_without_drivers("file-drivers");
    fs::write(file.join("drivers"), "not a directory\n").expect("the file is writable");
    let outcome = load_tree_report(&file);
    assert!(
        outcome.failures.is_empty(),
        "a regular file named drivers is skipped like a missing directory, not read:\n{}",
        reported(&outcome)
    );
    assert_eq!(outcome.files_read, baseline.files_read);

    // Removed on the way out, and only here: a failing run keeps its tree to be looked at, and a
    // passing one leaves nothing behind. Repeating the fixture root in every run's target tmpdir
    // without ever removing it is how a target directory quietly grows a copy of the document set
    // per run.
    for tree in [&absent, &empty, &prose, &file] {
        fs::remove_dir_all(tree).ok();
    }
}

/// A dot-prefixed file under `drivers/` is not a document, and `files_read` does not count it.
///
/// `src/load.rs:253-255` skips any entry whose name begins with a dot, in `drivers/` as everywhere
/// else, and no test said so — verified: deleting that skip leaves `cargo test -p aep-engine` at
/// exit 0. The rule matters here beyond coverage, because
/// `a_repository_with_no_drivers_directory_loads_exactly_as_before` computes its expected
/// `files_read` from a helper that does **not** mirror it (`tests/document_tree_order.rs:80-96`
/// counts every `.yaml`/`.yml`/`.json` including dot-prefixed ones), so the first dot-file
/// committed under `drivers/` turns that assertion red for a reason that has nothing to do with
/// what it is testing.
#[test]
fn a_dot_prefixed_file_under_drivers_is_not_a_step_map() {
    let root = tree_without_drivers("dotfile-drivers");
    fs::create_dir_all(root.join("drivers")).expect("the fixture tree is writable");
    let baseline = load_tree_report(&root);

    fs::write(
        root.join("drivers/.editor-scratch.yaml"),
        "this file is a string, and no document kind is a string\n",
    )
    .expect("the scratch file is writable");

    let outcome = load_tree_report(&root);
    assert!(
        outcome.failures.is_empty(),
        "a dot-prefixed file under drivers/ is skipped, not parsed as a step map:\n{}",
        reported(&outcome)
    );
    assert_eq!(
        outcome.files_read, baseline.files_read,
        "a dot-prefixed file is not counted among the files read"
    );

    fs::remove_dir_all(&root).ok();
}

/// The kind whose `Debug` spelling is `name`.
fn kind_named(name: &str) -> DocumentKind {
    *DocumentKind::ALL
        .iter()
        .find(|kind| format!("{kind:?}") == name)
        .unwrap_or_else(|| panic!("`DocumentKind::{name}` is a kind `DocumentKind::ALL` lists"))
}

/// The kinds `load_file` refuses outright, read off the arm that refuses them.
///
/// `src/load.rs` states the tree/not-tree partition twice — once as the rows of `TREE`, once as the
/// match arm that answers *"… documents do not belong in a protocol tree"* — and nothing checks the
/// two against each other.
fn kinds_refused_by_the_loader() -> Vec<DocumentKind> {
    let source = loader_source();
    let lines: Vec<&str> = source.lines().collect();
    let arm = lines
        .iter()
        .position(|line| line.contains("do not belong in a protocol tree"))
        .expect("the loader still refuses some kinds outright");
    let mut refused = Vec::new();
    for line in lines[..arm].iter().rev() {
        let trimmed = line.trim().trim_start_matches("| ");
        let Some(name) = trimmed.strip_prefix("DocumentKind::") else {
            break;
        };
        refused.push(kind_named(name.trim_end_matches(" => {").trim()));
    }
    assert!(
        !refused.is_empty(),
        "the extractor found no refused kinds; it has stopped seeing the arm it reads"
    );
    refused
}

/// One test that kills both mutants the two files were kept apart to kill separately.
///
/// The pair is defended on the ground that a source scan cannot see a reordered *walk* and a walk
/// cannot see a *row* naming a directory no fixture creates. Both are true of the two tests as
/// written, and neither is true of a walk whose fixture is built **from the table it is checking**:
/// a seventh row gets a directory, so it appears in the walk, and the walk is then compared against
/// the table row for row.
///
/// Verified against both mutants the pair was justified by:
///
/// | mutant | `..._last_row_of_the_loaders_table` | `a_load_reads_drivers_last_...` | this |
/// |---|---|---|---|
/// | a row appended after `drivers` | red | green | **red** |
/// | `TREE` sorted before the walk, table untouched | green | red | **red** |
#[test]
fn the_walk_reads_every_row_of_the_table_in_the_order_it_is_written() {
    let rows = tree_rows();
    let root = fixture_root("table-order");
    for (directory, _) in &rows {
        let path = root.join(directory);
        fs::create_dir_all(&path).expect("the fixture tree is writable");
        fs::write(
            path.join("broken.yaml"),
            "this file is a string, and no document kind is a string\n",
        )
        .expect("the document is writable");
    }

    let outcome = load_tree_report(&root);
    let walked: Vec<String> = outcome
        .failures
        .iter()
        .filter_map(|failure| failure.path.as_deref())
        .map(|path| {
            path.strip_prefix(&root)
                .expect("a file failure names a path inside the tree")
                .parent()
                .expect("a document file has a parent directory")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    let written: Vec<&str> = rows
        .iter()
        .map(|(directory, _)| directory.as_str())
        .collect();
    assert_eq!(
        walked, written,
        "the loader must read the rows of its table in the order they are written, once each"
    );

    assert_eq!(
        rows.last()
            .map(|(directory, kind)| (directory.as_str(), kind.as_str())),
        Some(("drivers", "StepMap")),
        "and `drivers` must be the row it ends on; the table reads {written:?}"
    );

    fs::remove_dir_all(&root).ok();
}

/// The table's rows are exactly the kinds the loader does not refuse, each under its own directory.
///
/// This is the assertion the derived fixture in `document_tree_order.rs` gave up. That file used to
/// require the walk to reach every tree directory (`walked.len() == TREE_DIRECTORIES.len()`); once
/// the fixture came from `DocumentKind::ALL` — ten directories, of which the loader walks six — an
/// exact count was no longer available and it relaxed to `walked.len() >= 2`. A row deleted from
/// `TREE` now leaves both files green. `ALL` minus the refused kinds is the six, derived, so the
/// count comes back without a hand-written list.
#[test]
fn the_table_holds_one_row_for_every_kind_the_loader_accepts() {
    let rows = tree_rows();
    let refused = kinds_refused_by_the_loader();

    let expected: Vec<DocumentKind> = DocumentKind::ALL
        .iter()
        .copied()
        .filter(|kind| !refused.contains(kind))
        .collect();
    let mut accepted: Vec<DocumentKind> = rows.iter().map(|(_, kind)| kind_named(kind)).collect();
    accepted.sort_unstable();
    accepted.dedup();

    let mut wanted = expected.clone();
    wanted.sort_unstable();
    assert_eq!(
        accepted, wanted,
        "every kind `load_file` does not refuse needs a row of `TREE` to be read from, and every \
         row needs a kind that is not refused; the table reads {rows:?} and the loader refuses \
         {refused:?}"
    );

    for (directory, kind) in &rows {
        assert_eq!(
            directory.as_str(),
            kind_named(kind).directory(),
            "a row of `TREE` puts `{kind}` under `{directory}`, but `DocumentKind::directory` \
             spells it `{}`",
            kind_named(kind).directory()
        );
    }
}

/// A dot-prefixed *directory* under `drivers/` is not descended into.
///
/// `src/load.rs` tests the name for a leading dot **before** it tests whether the entry is a
/// directory, so `.archive/` is skipped whole and the documents inside it are never seen. That
/// ordering is what `document_count` in `tests/document_tree_order.rs:101-120` mirrors, and it is
/// the premise its `files_read` comparison rests on — but nothing asserted the loader is in that
/// order. Verified: swapping the two branches in `src/load.rs:253-259` leaves
/// `cargo test -p aep-engine` at exit 0 with the mirror now wrong, because the dot-*file* case is
/// unaffected by the swap and no case covered the dot-*directory* one.
#[test]
fn a_dot_prefixed_directory_under_drivers_is_not_descended_into() {
    let root = tree_without_drivers("dotdir-drivers");
    fs::create_dir_all(root.join("drivers")).expect("the fixture tree is writable");
    let baseline = load_tree_report(&root);

    fs::create_dir_all(root.join("drivers/.archive")).expect("the fixture tree is writable");
    fs::write(
        root.join("drivers/.archive/retired.yaml"),
        "this file is a string, and no document kind is a string\n",
    )
    .expect("the retired map is writable");

    let outcome = load_tree_report(&root);
    assert!(
        outcome.failures.is_empty(),
        "a dot-prefixed directory is skipped whole, not descended into:\n{}",
        reported(&outcome)
    );
    assert_eq!(
        outcome.files_read, baseline.files_read,
        "nothing under a dot-prefixed directory is counted among the files read"
    );

    fs::remove_dir_all(&root).ok();
}
