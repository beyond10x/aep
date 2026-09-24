//! Evidence on a tree store is an Entity Runtime observation, not an edit of the artifact.
//!
//! Design § 4.1 (the evidence row) and review N7: an observation does not advance the artifact's
//! revision, so an evidence record on one branch and a move on another merge into one artifact
//! rather than a fork, and the gated move that follows still counts the evidence the other branch
//! recorded.

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

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Runs `aep`, and fails the case with its output when it refuses.
fn accepted(project: &Path, args: &[&str]) -> String {
    let output = aep(project, args);
    assert!(output.status.success(), "aep {args:?}: {}", text(&output));
    text(&output)
}

fn git(project: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(project)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn commit(project: &Path, message: &str) {
    git(project, &["add", "-A"]);
    git(project, &["commit", "-q", "-m", message]);
}

/// A repository on `main` holding a tree store with one story at `proposed`.
fn repository_with_a_proposed_story(name: &str) -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tree-evidence-{name}"));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).expect("the repository directory");
    git(&project, &["init", "-q", "-b", "main", "."]);
    git(
        &project,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&project, &["config", "user.name", "fixture"]);
    let engineering = project.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("the .engineering directory");
    let protocols = relative(&engineering, &workspace());
    accepted(
        &project,
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
    accepted(
        &project,
        &[
            "plan", "artifact", "new", "story", "observed", "--title", "Observed",
        ],
    );
    accepted(
        &project,
        &[
            "plan",
            "artifact",
            "move",
            "story:observed",
            "--to",
            "proposed",
        ],
    );
    commit(&project, "base");
    project
}

fn record_a_test_result(project: &Path) {
    accepted(
        project,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:observed",
            "--kind",
            "test_result",
            "--source",
            "task check",
        ],
    );
}

#[test]
fn evidence_on_one_branch_and_a_move_on_another_merge_without_a_fork_and_the_gate_counts_it() {
    let project = repository_with_a_proposed_story("merge");

    git(&project, &["checkout", "-q", "-b", "evidence"]);
    record_a_test_result(&project);
    commit(&project, "evidence");

    git(&project, &["checkout", "-q", "main"]);
    accepted(
        &project,
        &[
            "plan",
            "artifact",
            "move",
            "story:observed",
            "--to",
            "active",
        ],
    );
    commit(&project, "move");

    git(&project, &["merge", "-q", "--no-edit", "evidence"]);

    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(
        validated.status.success() && !text(&validated).contains("S3"),
        "evidence on one branch and a move on the other left the story forked: {}",
        text(&validated)
    );
    let explained = accepted(&project, &["plan", "artifact", "explain", "story:observed"]);
    assert!(
        explained.contains("next: implemented needs 1 test_result record(s); held: 1"),
        "the merged store does not count the evidence the other branch recorded: {explained}"
    );
    accepted(
        &project,
        &[
            "plan",
            "artifact",
            "move",
            "story:observed",
            "--to",
            "implemented",
        ],
    );
    let history = accepted(&project, &["plan", "artifact", "history", "story:observed"]);
    assert!(
        history.contains("test_result"),
        "the history no longer shows the evidence: {history}"
    );
    let shown = accepted(
        &project,
        &[
            "plan",
            "artifact",
            "show",
            "story:observed",
            "--format",
            "json",
        ],
    );
    assert!(
        shown.contains("implemented"),
        "the gated move did not land after the merge: {shown}"
    );
    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(
        validated.status.success(),
        "the store does not validate after the gated move: {}",
        text(&validated)
    );
}

#[test]
fn evidence_on_a_tree_store_leaves_the_rendered_revision_and_the_projection_unchanged() {
    let project = repository_with_a_proposed_story("render");
    let document = project.join(".engineering/planning/story/observed.md");
    let before = std::fs::read(&document).expect("the story's projection");

    record_a_test_result(&project);

    assert_eq!(
        String::from_utf8(std::fs::read(&document).expect("the story's projection")).unwrap(),
        String::from_utf8(before).unwrap(),
        "recording evidence re-rendered the story, as a change to it would"
    );
    let status = git(
        &project,
        &["status", "--porcelain", "--", ".engineering/planning"],
    );
    assert!(
        status.stdout.is_empty(),
        "recording evidence changed the projection: {}",
        String::from_utf8_lossy(&status.stdout)
    );
    let listed = accepted(&project, &["plan", "artifact", "list"]);
    assert!(listed.contains("story:observed"), "{listed}");
    let explained = accepted(&project, &["plan", "artifact", "explain", "story:observed"]);
    assert!(
        explained.contains("test_result from task check"),
        "explain does not show the evidence: {explained}"
    );
}
