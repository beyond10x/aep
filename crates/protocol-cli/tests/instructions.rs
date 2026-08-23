//! The committed instruction documents are what the verb writes, byte for byte.
//!
//! `generated/instructions/` holds one document per workflow this tree declares, rendered by
//! `protocol workflow instruct`. They are committed because they are a **treatment**: an evaluation
//! of how well a harness follows this methodology hands an agent the rules, and rules typed into a
//! prompt are a claim about the specification rather than a projection of it. A committed artifact
//! can be diffed, reviewed and pointed at; a prompt someone wrote once cannot.
//!
//! That only holds while the artifact and the specification cannot drift apart, which is what this
//! file is. It is the same shape as the drift checks over `generated/`, `suites/generated/` and the
//! synthesised trees, and it is checked in **both directions** for the reason `xtask` gives about
//! its own: a check that only compares what is generated never notices a committed document that
//! nothing generates any more — an instruction for a workflow this repository has deleted, still
//! being read as though it were in force.
//!
//! # Who owns this tree
//!
//! This test, and not a task in `xtask`. The projection task's orphan scan is carved out of it
//! (`PROJECTION_EXCLUSIONS`), so nothing else writes or deletes here, and the writer is the verb a
//! person runs rather than a second implementation of it — which is the property `xtask` reaches
//! for by shelling out to `protocol` instead of linking the generators. Regenerating is one command
//! and the failure below names it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the committed documents live, relative to the repository root.
const COMMITTED: &str = "generated/instructions";

/// The command that rewrites them, named in every failure here.
const FIX: &str = "protocol workflow instruct --out generated/instructions";

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// An empty scratch directory to render into.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Runs `protocol workflow instruct` over the document tree at `tree`, writing into `out`.
fn instruct(tree: &Path, out: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(["workflow", "instruct", "--root"])
        .arg(tree)
        .arg("--out")
        .arg(out)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "`{FIX}` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every file under `directory`, keyed by its path relative to it.
///
/// Recursive, sorted by the map, and it reads the bytes as text: these are markdown documents, and
/// a difference in them is something a reviewer reads rather than a digest they compare.
fn tree_of(directory: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    collect(directory, directory, &mut files);
    files
}

/// Fills `files` with everything under `at`, keyed relative to `base`.
fn collect(base: &Path, at: &Path, files: &mut BTreeMap<String, String>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(at)
        .unwrap_or_else(|error| panic!("reading {}: {error}", at.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(base, &path, files);
            continue;
        }
        let relative = path
            .strip_prefix(base)
            .expect("every entry is under the directory being walked")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        files.insert(relative, text);
    }
}

/// The first line at which two documents differ, for a failure a person can act on.
///
/// `assert_eq!` on a five-hundred-line markdown document prints two walls of prose and tells the
/// reader nothing, which is the same argument `aep-render`'s snapshot helper makes.
fn first_difference(committed: &str, produced: &str) -> String {
    committed
        .lines()
        .zip(produced.lines())
        .enumerate()
        .find(|(_, (one, other))| one != other)
        .map_or_else(
            || {
                format!(
                    "one document is a prefix of the other: {} committed lines, {} produced",
                    committed.lines().count(),
                    produced.lines().count()
                )
            },
            |(index, (one, other))| {
                format!(
                    "line {}\n  committed: {one}\n  produced:  {other}",
                    index + 1
                )
            },
        )
}

#[test]
fn the_committed_instruction_documents_are_what_the_verb_writes() {
    let out = scratch("protocol-instructions-committed");
    instruct(&root(), &out);

    let produced = tree_of(&out);
    let committed = tree_of(&root().join(COMMITTED));

    assert!(
        !produced.is_empty(),
        "the verb wrote nothing, so this test would pass on an empty tree"
    );

    for (path, text) in &produced {
        let Some(held) = committed.get(path) else {
            panic!("{COMMITTED}/{path} is not committed; run `{FIX}` and commit the result");
        };
        // `assert!` and not `assert_eq!`: these are five-hundred-line documents, and the built-in
        // comparison would print both of them in full where one line is what moved.
        assert!(
            held == text,
            "{COMMITTED}/{path} is not what the workflow and its principles now say — {}\nrun \
             `{FIX}` and commit the result",
            first_difference(held, text)
        );
    }

    // The other direction. A document for a workflow this tree no longer declares is an instruction
    // nobody stands behind, still being read as though somebody did.
    let orphaned: Vec<&String> = committed
        .keys()
        .filter(|path| !produced.contains_key(*path))
        .collect();
    assert!(
        orphaned.is_empty(),
        "{} committed document(s) are rendered by nothing any more: {orphaned:?} — run `{FIX}`, \
         then delete what it did not write",
        orphaned.len()
    );

    std::fs::remove_dir_all(&out).ok();
}

#[test]
fn two_runs_of_the_verb_write_the_same_bytes() {
    // Byte-identity across two processes, not two calls in one: the committed documents survive
    // being regenerated on another machine, on another day, in another order of the filesystem's
    // choosing, or they are not a reviewable artifact.
    let first = scratch("protocol-instructions-once");
    let second = scratch("protocol-instructions-twice");
    instruct(&root(), &first);
    instruct(&root(), &second);
    assert_eq!(tree_of(&first), tree_of(&second));

    std::fs::remove_dir_all(&first).ok();
    std::fs::remove_dir_all(&second).ok();
}

#[test]
fn a_workflow_that_moved_renders_different_bytes_and_leaves_its_neighbours_alone() {
    // Determinism is over *content*, and a byte-identity check that could not tell two different
    // workflows apart would be satisfied by a renderer that emitted a constant. So the guard is
    // verified by breaking it: one guard predicate is changed in a copy of the tree, and the
    // document for that workflow must move while every other document must not.
    let tree = scratch("protocol-instructions-mutated-tree");
    for directory in ["protocols", "principles", "workflows"] {
        copy_tree(&root().join(directory), &tree.join(directory));
    }

    let workflow = tree.join("workflows/development/default.yaml");
    let original = std::fs::read_to_string(&workflow).expect("the copied workflow is readable");
    let mutated = original.replace("when: diff.exists", "when: diff.exists_somewhere_else");
    assert_ne!(
        mutated, original,
        "the mutation must actually change the document"
    );
    std::fs::write(&workflow, mutated).expect("the copy is writable");

    let out = scratch("protocol-instructions-mutated");
    instruct(&tree, &out);
    let produced = tree_of(&out);
    let committed = tree_of(&root().join(COMMITTED));

    assert_ne!(
        produced.get("adp/default.md"),
        committed.get("adp/default.md"),
        "a workflow whose guard moved must render different instructions, or the rendering is not \
         reading the guard at all"
    );
    assert_eq!(
        produced.get("release/progressive.md"),
        committed.get("release/progressive.md"),
        "and a workflow nobody touched must render exactly as it is committed"
    );

    std::fs::remove_dir_all(&tree).ok();
    std::fs::remove_dir_all(&out).ok();
}

/// Copies a directory tree, so a mutation never reaches the repository's own documents.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the scratch tree is writable");
    for entry in std::fs::read_dir(from).expect("the source directory is readable") {
        let path = entry.expect("a readable directory entry").path();
        let target = to.join(path.file_name().expect("every entry is named"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("the file copies");
        }
    }
}
