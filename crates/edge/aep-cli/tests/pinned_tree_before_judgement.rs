//! A store that pins a protocol tree older than 0.72.0 validates.
//!
//! 0.72.0 refused every `kind: llm` step without `judgement:`, and `aep plan artifact validate`
//! loads the step maps of the tree a store's `protocols:` names. A pinned tree is immutable, so
//! every store pinning one written before the rule failed validate with twelve
//! `missing_declaration` errors it could never fix. The rule now belongs to `aep.driver-steps/2`,
//! and the maps those trees ship are `aep.driver-steps/1`.
//!
//! `tests/fixtures/protocol-tree-0.71.3/drivers/` is a byte-for-byte copy of the 0.71.3 tag's
//! `drivers/`. The rest of that tag's document tree (`protocols`, `principles`, `workflows`,
//! `profiles`, `artifacts/lifecycles`) is identical to 0.72.0's, so the fixture copies those from
//! this repository rather than committing a second copy of them.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository root.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The directories of a document tree the fixture copies from this repository.
const UNCHANGED_SINCE_0_71_3: &[&str] = &[
    "protocols",
    "principles",
    "workflows",
    "profiles",
    "artifacts/lifecycles",
    "artifacts/templates",
];

/// An empty scratch directory for one test, distinct per process.
fn fixture(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("pinned-tree-0-71-3")
        .join(format!("{name}-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the scratch directory is removable");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory is writable");
    directory
}

/// Copies a directory recursively.
fn copy_directory(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the scratch tree is writable");
    for entry in std::fs::read_dir(from).expect("the source tree is readable") {
        let entry = entry.expect("a source entry");
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if source.is_dir() {
            copy_directory(&source, &destination);
        } else {
            std::fs::copy(&source, &destination).expect("the source document is copied");
        }
    }
}

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the scratch tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// The 0.71.3 document tree at `at`: its own step maps, and the documents it shares with this
/// repository.
fn tree_of_0_71_3(at: &Path) {
    let repository = repository();
    for directory in UNCHANGED_SINCE_0_71_3 {
        copy_directory(&repository.join(directory), &at.join(directory));
    }
    copy_directory(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/protocol-tree-0.71.3/drivers"),
        &at.join("drivers"),
    );
}

#[test]
fn a_store_pinned_to_the_0_71_3_tree_validates_without_a_judgement_error() {
    let root = fixture("validate");
    let tree = root.join("tree");
    let project = root.join("project");
    tree_of_0_71_3(&tree);
    for map in ["default.yaml", "checks.yaml"] {
        let text = std::fs::read_to_string(tree.join("drivers/development").join(map))
            .expect("the fixture map is readable");
        assert!(
            text.contains("format: aep.driver-steps/1") && !text.contains("judgement:"),
            "the fixture is a pre-0.72.0 map, with no judgement on any llm step: {map}"
        );
    }
    write(
        &project.join(".engineering/project.yaml"),
        "version: aep.project/5\nplanning_scope: fixture\nprotocol: adp/1\n\
         profile: development.standard\nprotocols: ../../tree\n",
    );
    write(
        &project.join(".engineering/planning/story/pinned.md"),
        "---\nformat: aep.planning-md/1\nid: story:pinned\nkind: story\nstatus: draft\n\
         title: A story in a store pinned to an older tree\nrevision: 1\n---\n# Story\n\nBody.\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "artifact", "validate"])
        .current_dir(&project)
        .env("AEP_CACHE_DIR", root.join("cache"))
        .output()
        .expect("the protocol binary runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // The refusal's own words, not the bare word: the scratch path is printed too.
    assert!(
        !said.contains(".judgement") && !said.contains("missing_declaration"),
        "a pinned tree can never gain the key, so no judgement error is reported:\n{said}"
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "a store pinned to the 0.71.3 tree validates:\n{said}"
    );
}
