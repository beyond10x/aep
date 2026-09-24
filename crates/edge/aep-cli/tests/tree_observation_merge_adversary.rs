//! Adversarial cases for N7: evidence on a tree store is an Entity Runtime observation.
//!
//! The unit's own cases prove that evidence on one branch and a move on another merge without a
//! fork. These drive the merged store further: what `explain` says the moves rested on, a second
//! evidence record written onto a subject whose history now has an observation tip beside its
//! decision head, and what `resolve` reports as not carried over when an observation hangs off the
//! head it keeps.

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

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

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
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("n7-adversary-{name}"));
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
    move_to(&project, "proposed");
    commit(&project, "base");
    project
}

fn record_a_test_result(project: &Path, source: &str) {
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
            source,
        ],
    );
}

fn move_to(project: &Path, to: &str) {
    accepted(
        project,
        &["plan", "artifact", "move", "story:observed", "--to", to],
    );
}

fn explained(project: &Path) -> serde_json::Value {
    let output = accepted(
        project,
        &[
            "plan",
            "artifact",
            "explain",
            "story:observed",
            "--format",
            "json",
        ],
    );
    let start = output.find('{').expect("explain prints JSON");
    serde_json::from_str(&output[start..]).unwrap_or_else(|error| {
        panic!("explain's JSON does not parse ({error}): {output}");
    })
}

/// The sources of the records `explain` says the move to `to` rested on.
fn rested_on(explained: &serde_json::Value, to: &str) -> Vec<String> {
    explained["reached"]
        .as_array()
        .expect("reached is a list")
        .iter()
        .find(|step| step["to"] == to)
        .unwrap_or_else(|| panic!("no move to {to} is explained: {explained:#}"))["rested_on"]
        .as_array()
        .expect("rested_on is a list")
        .iter()
        .map(|record| record["source"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// Evidence on one branch, a move on another, the merge, then the gated move the evidence pays
/// for. `evidence_on_main` puts the evidence on the branch merged into and the move on the branch
/// merged from; otherwise the other way round.
fn merged_then_implemented(name: &str, evidence_on_main: bool) -> PathBuf {
    let project = repository_with_a_proposed_story(name);
    git(&project, &["checkout", "-q", "-b", "other"]);
    if evidence_on_main {
        move_to(&project, "active");
    } else {
        record_a_test_result(&project, "concurrent suite");
    }
    commit(&project, "other");
    git(&project, &["checkout", "-q", "main"]);
    if evidence_on_main {
        record_a_test_result(&project, "concurrent suite");
    } else {
        move_to(&project, "active");
    }
    commit(&project, "main");
    git(&project, &["merge", "-q", "--no-edit", "other"]);
    move_to(&project, "implemented");
    project
}

/// `explain`'s join is log order: "what a move rested on is what the store had already admitted
/// when the move was made". The move to `active` was made on a branch that never held the
/// evidence, and the move to `implemented` was admitted only because of it.
fn assert_the_gated_move_rests_on_the_merged_evidence(project: &Path) {
    let explained = explained(project);
    assert_eq!(
        rested_on(&explained, "implemented"),
        vec!["concurrent suite".to_owned()],
        "the move to implemented, which the merged test_result alone made legal, is not \
         explained as resting on it: {explained:#}"
    );
    assert!(
        rested_on(&explained, "active").is_empty(),
        "the move to active, made on a branch that never held the test_result, is explained as \
         resting on it: {explained:#}"
    );
}

#[test]
fn after_a_merge_the_gated_move_rests_on_evidence_the_merged_branch_recorded() {
    let project = merged_then_implemented("rested-branch", false);
    assert_the_gated_move_rests_on_the_merged_evidence(&project);
}

#[test]
fn after_a_merge_the_gated_move_rests_on_evidence_the_merged_into_branch_recorded() {
    let project = merged_then_implemented("rested-main", true);
    assert_the_gated_move_rests_on_the_merged_evidence(&project);
}

#[test]
fn evidence_recorded_after_a_merge_left_an_observation_tip_is_accepted_and_counted_once_each() {
    let project = repository_with_a_proposed_story("after-merge");
    git(&project, &["checkout", "-q", "-b", "evidence"]);
    record_a_test_result(&project, "branch suite");
    commit(&project, "evidence");
    git(&project, &["checkout", "-q", "main"]);
    move_to(&project, "active");
    commit(&project, "move");
    git(&project, &["merge", "-q", "--no-edit", "evidence"]);

    record_a_test_result(&project, "merged suite");
    commit(&project, "after");

    let explained_text = accepted(&project, &["plan", "artifact", "explain", "story:observed"]);
    assert!(
        explained_text.contains("next: implemented needs 1 test_result record(s); held: 2"),
        "the two test_result records are not each counted once after the merge: {explained_text}"
    );
    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(
        validated.status.success(),
        "the store does not validate after evidence on a merged subject: {}",
        text(&validated)
    );
}

/// Every `not carried over:` line `resolve` printed.
fn not_carried(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("not carried over: "))
        .map(str::to_owned)
        .collect()
}

/// Branch `kept` moves the story to `active` and, when `with_evidence`, then records a test
/// result about it; `main` changes its title. The merge forks the story (two decisions) and
/// `resolve --onto kept` keeps `kept`'s head.
fn resolved_keeping_the_branch(name: &str, with_evidence: bool) -> (PathBuf, String) {
    let project = repository_with_a_proposed_story(name);
    git(&project, &["checkout", "-q", "-b", "kept"]);
    move_to(&project, "active");
    if with_evidence {
        record_a_test_result(&project, "kept suite");
    }
    commit(&project, "kept");
    git(&project, &["checkout", "-q", "main"]);
    accepted(
        &project,
        &[
            "plan",
            "artifact",
            "set",
            "story:observed",
            "--title",
            "Retitled",
        ],
    );
    commit(&project, "retitle");
    // Design § 7: the `.md` conflicts and the event files merge clean; `resolve` runs in the
    // merging worktree with only the `.md` unmerged, and re-renders and stages it.
    let merged = Command::new("git")
        .args(["merge", "-q", "--no-edit", "kept"])
        .current_dir(&project)
        .output()
        .expect("git runs");
    assert!(
        String::from_utf8_lossy(&merged.stdout).contains("observed.md") || merged.status.success(),
        "the merge failed for a reason other than the projection: {}",
        String::from_utf8_lossy(&merged.stdout)
    );
    let resolved = accepted(
        &project,
        &[
            "plan",
            "artifact",
            "resolve",
            "story:observed",
            "--onto",
            "kept",
        ],
    );
    (project, resolved)
}

#[test]
fn resolve_does_not_report_evidence_on_the_kept_head_as_not_carried_over() {
    let (_, without) = resolved_keeping_the_branch("resolve-control", false);
    let (project, with) = resolved_keeping_the_branch("resolve-evidence", true);
    assert_eq!(
        not_carried(&with).len(),
        not_carried(&without).len(),
        "resolve keeping the branch that recorded the evidence reports more records as not \
         carried over than the same resolve without it — the observation that hangs off the kept \
         head is listed for the operator to issue again, although its record is still counted.\n\
         with evidence: {with}\nwithout: {without}"
    );
    let explained_text = accepted(&project, &["plan", "artifact", "explain", "story:observed"]);
    assert!(
        explained_text.contains("next: implemented needs 1 test_result record(s); held: 1"),
        "the kept branch's evidence is not counted after resolve: {explained_text}"
    );
}
