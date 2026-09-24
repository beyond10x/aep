//! Adversarial cases for `aep plan artifact validate --against <revision>` on a tree store.
//!
//! The skip of V2 is decided by asking Git whether `<revision>` holds
//! `<authority>/store.json`. These cases hold that question to the two things the change says it
//! answers: "this revision holds no tree store" must be true when printed, and an unreadable
//! revision must be refused rather than read as one holding nothing.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

fn relative(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<Component<'_>> = from.components().collect();
    let to: Vec<Component<'_>> = to.components().collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut path = PathBuf::new();
    for _ in shared..from.len() {
        path.push("..");
    }
    for component in &to[shared..] {
        path.push(component);
    }
    path
}

fn aep(project: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(project)
        .output()
        .expect("the aep binary runs")
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn git(dir: &Path, args: &[&str]) {
    git_out(dir, args);
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn repository(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tree-validate-adv-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the repository directory");
    git(&root, &["init", "-q", "-b", "main", "."]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    git(&root, &["config", "user.name", "fixture"]);
    root
}

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

fn init_tree(project: &Path) {
    let engineering = project.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("the .engineering directory");
    let protocols = relative(&engineering, &workspace());
    let init = aep(
        project,
        &[
            "plan",
            "store",
            "init-tree",
            "--engineering",
            ".engineering",
            "--scope",
            "fixture",
            "--protocols",
            protocols.to_str().expect("a UTF-8 path"),
        ],
    );
    assert!(init.status.success(), "{}", text(&init));
}

fn new_story(project: &Path, name: &str) {
    let created = aep(
        project,
        &["plan", "artifact", "new", "story", name, "--title", name],
    );
    assert!(created.status.success(), "{}", text(&created));
}

/// A project whose `.engineering/` sits in a subdirectory of its Git repository — a monorepo.
/// Discovery walks up to the nearest `.engineering/`, so every verb works from `sub/`, and
/// `materialize` (git archive, cwd-relative) reads the base correctly. `holds_file` asks
/// `git cat-file -e <rev>:<path>`, whose path is relative to the repository **top level**, so
/// it answers "no tree store" for a base that holds one and V2 is skipped.
#[test]
fn a_subdirectory_project_with_a_tree_base_is_still_held_to_v2() {
    let root = repository("subdirectory");
    let project = root.join("sub");
    std::fs::create_dir_all(&project).expect("the project subdirectory");
    init_tree(&project);
    new_story(&project, "fixture-one");
    commit(&root, "a tree store with one story");
    new_story(&project, "fixture-two");
    commit(&root, "base: a second story");

    git(&root, &["rm", "-r", "-q", "sub/.engineering"]);
    git(&root, &["checkout", "HEAD~1", "--", "sub/.engineering"]);
    commit(&root, "head: the second story's committed files deleted");

    let validated = aep(
        &project,
        &["plan", "artifact", "validate", "--against", "HEAD~1"],
    );
    let out = text(&validated);
    assert!(
        !out.contains("V2 skipped"),
        "HEAD~1 holds sub/.engineering/state/store.json, so it holds a tree store:\n{out}"
    );
    assert!(
        out.contains("V2: a committed file was deleted"),
        "a deleted committed tree file in a subdirectory project is a V2 problem:\n{out}"
    );
    assert!(!validated.status.success(), "{out}");
}

/// A base whose commit resolves but whose `store.json` object cannot be read — the shape of a
/// partial clone whose promisor cannot be reached. `cat-file -e` fails for it exactly as it fails
/// for a path the revision does not hold, and the code reads both as "holds no tree store".
#[test]
fn a_base_whose_store_file_cannot_be_read_is_refused_not_skipped() {
    let project = repository("missing-object");
    init_tree(&project);
    new_story(&project, "fixture-one");
    commit(&project, "base: a tree store");
    new_story(&project, "fixture-two");
    commit(&project, "head: a second story");

    let blob = git_out(
        &project,
        &["rev-parse", "HEAD~1:.engineering/state/store.json"],
    );
    let object = project
        .join(".git/objects")
        .join(&blob[..2])
        .join(&blob[2..]);
    assert!(
        object.exists(),
        "the fixture's store.json blob is a loose object"
    );
    std::fs::remove_file(&object).expect("the blob removed from the object store");

    let validated = aep(
        &project,
        &["plan", "artifact", "validate", "--against", "HEAD~1"],
    );
    let out = text(&validated);
    assert!(
        !out.contains("V2 skipped"),
        "HEAD~1 does hold store.json; saying it holds no tree store is false:\n{out}"
    );
    assert!(
        !validated.status.success(),
        "a base that cannot be read must be refused:\n{out}"
    );
}
