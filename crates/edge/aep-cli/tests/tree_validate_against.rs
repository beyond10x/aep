//! `aep plan artifact validate --against <revision>` on an `aep.project/3` tree store.
//!
//! V2 holds the tree's committed files to the ones `<revision>` holds. A base with no tree store —
//! the `aep.project/2` commit a cutover pull request is measured against — has no committed tree
//! file to hold, so every file it does hold would otherwise read as deleted. V2 is skipped there,
//! and says so; a tree base is still held to it.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// `to` written relative to `from`, which is how a project names its protocol tree.
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

fn git(project: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(project)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn repository(name: &str) -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tree-validate-{name}"));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).expect("the repository directory");
    git(&project, &["init", "-q", "-b", "main", "."]);
    git(
        &project,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&project, &["config", "user.name", "fixture"]);
    project
}

fn commit(project: &Path, message: &str) {
    git(project, &["add", "-A"]);
    git(project, &["commit", "-q", "-m", message]);
}

/// A tree store with one story, created by this binary.
fn init_tree_with_one_story(project: &Path) {
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
    let created = aep(
        project,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "fixture-one",
            "--title",
            "Fixture one",
        ],
    );
    assert!(created.status.success(), "{}", text(&created));
}

#[test]
fn a_base_with_no_tree_store_skips_v2_and_says_so() {
    let project = repository("no-tree-base");
    // The base is an `aep.project/2` store: a file authority, whose files a tree never holds.
    let state = project.join(".engineering/state");
    std::fs::create_dir_all(&state).expect("the /2 authority directory");
    std::fs::write(
        project.join(".engineering/project.yaml"),
        "version: aep.project/2\n",
    )
    .expect("the /2 selector");
    std::fs::write(state.join("manifest.json"), "{}").expect("a /2 manifest");
    std::fs::write(state.join("events.jsonl"), "{}\n").expect("a /2 journal");
    commit(&project, "base: an aep.project/2 store");

    // The cutover: the /2 store is replaced by a tree store.
    std::fs::remove_dir_all(project.join(".engineering")).expect("the /2 store removed");
    init_tree_with_one_story(&project);
    commit(&project, "head: the tree store");

    let validated = aep(
        &project,
        &["plan", "artifact", "validate", "--against", "HEAD~1"],
    );
    let out = text(&validated);
    assert!(
        validated.status.success(),
        "a /2 base has no tree file V2 could hold, so nothing it holds is a deleted tree file:\n{out}"
    );
    assert!(
        !out.contains("V2:"),
        "V2 must not run against a base with no tree store:\n{out}"
    );
    assert!(
        out.contains("V2 skipped: HEAD~1 holds no tree store (.engineering/state/store.json)"),
        "the skip must be said, naming the revision and the file it looked for:\n{out}"
    );
    assert!(out.contains("valid"), "{out}");
}

#[test]
fn a_tree_base_still_reports_a_deleted_committed_file_under_v2() {
    let project = repository("tree-base");
    init_tree_with_one_story(&project);
    commit(&project, "a tree store with one story");
    let created = aep(
        &project,
        &[
            "plan",
            "artifact",
            "new",
            "story",
            "fixture-two",
            "--title",
            "Fixture two",
        ],
    );
    assert!(created.status.success(), "{}", text(&created));
    commit(&project, "base: a second story");

    // The head drops the second story's command: its group and event files, committed on the
    // base, are gone. The store left behind still opens, so V2 is the rule that sees it.
    git(&project, &["rm", "-r", "-q", ".engineering"]);
    git(&project, &["checkout", "HEAD~1", "--", ".engineering"]);
    commit(&project, "head: the second story's committed files deleted");

    let validated = aep(
        &project,
        &["plan", "artifact", "validate", "--against", "HEAD~1"],
    );
    let out = text(&validated);
    assert!(
        !validated.status.success(),
        "a deleted committed tree file is a problem:\n{out}"
    );
    assert!(
        out.contains("V2: a committed file was deleted"),
        "a tree base still holds the head to V2:\n{out}"
    );
    assert!(!out.contains("V2 skipped"), "{out}");
}

#[test]
fn a_revision_git_cannot_resolve_is_refused_rather_than_read_as_holding_no_tree() {
    let project = repository("unresolved-base");
    init_tree_with_one_story(&project);
    commit(&project, "a tree store");

    let validated = aep(
        &project,
        &[
            "plan",
            "artifact",
            "validate",
            "--against",
            "refs/heads/never-fetched",
        ],
    );
    let out = text(&validated);
    assert!(
        !validated.status.success(),
        "an unread base must not pass:\n{out}"
    );
    assert!(
        out.contains("could not be read for `--against`; fetch it first"),
        "an unfetched base is not a base with no tree store:\n{out}"
    );
    assert!(!out.contains("V2 skipped"), "{out}");
}
