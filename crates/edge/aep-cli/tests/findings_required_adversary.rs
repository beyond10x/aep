//! Adversarial cases against `story:review-result-requires-findings`: how `validate` dates a
//! review in a Git-native store, and what `show` returns for a review recorded `--prose-only`.
//!
//! Each case drives the built `aep` binary against a scratch project, with Git repositories built
//! by `git` itself under a fixed clock and no configuration but their own.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The day the opted-in projects below require a block from.
const OPT_IN: &str = "2026-06-01";

/// Months before `OPT_IN`: when the old review was recorded and committed.
const BEFORE: &str = "2026-01-05T00:00:00Z";

/// After `OPT_IN`: a later, unrelated commit, which is what a clone of depth 1 starts at.
const AFTER: &str = "2026-10-05T00:00:00Z";

/// The repository root, whose document tree supplies the lifecycles.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// A fresh scratch directory for one case.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir()
        .canonicalize()
        .expect("the temporary directory exists")
        .join(format!(
            "aep-findings-adversary-{name}-{}",
            std::process::id()
        ));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch tree is removable");
    }
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the parent directory is writable");
    }
    std::fs::write(path, text).expect("the file is writable");
}

fn printable(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// Runs `aep` from the repository root, so the lifecycles are the repository's own.
fn aep(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("an exit code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Runs Git in `directory` with the clock fixed at `at` and no global or system configuration.
fn git(directory: &Path, at: &str, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .env("GIT_AUTHOR_DATE", at)
        .env("GIT_COMMITTER_DATE", at)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {}", stderr(&output));
}

/// A Git repository at `project` holding an opted-in Git-native store with one prose-only review,
/// `review-result:old`, committed at `BEFORE`, and one later unrelated commit at `AFTER`.
fn opted_in_repository_with_an_old_prose_review(project: &Path) {
    write(
        &project.join(".engineering/project.yaml"),
        &format!(
            "version: aep.project/5\nplanning_scope: findings-adversary\nprotocol: adp/1\n\
             profile: development.standard\nfindings_required_since: {OPT_IN}\n"
        ),
    );
    write(
        &project.join(".engineering/planning/epic/objectives.md"),
        "---\nformat: aep.planning-md/3\nid: epic:objectives\nkind: epic\nstatus: draft\n\
         title: Objectives\nrevision: 1\n---\n# Objectives\n",
    );
    write(
        &project.join(".engineering/planning/review-result/old.md"),
        "---\nformat: aep.planning-md/3\nid: review-result:old\nkind: review-result\n\
         status: active\ntitle: old\nrelations:\n- reviews: epic:objectives\nrevision: 1\n---\n\
         # old\n\nEvery epic names an objective.\n",
    );
    git(project, BEFORE, &["init", "--quiet"]);
    git(
        project,
        BEFORE,
        &["config", "user.name", "Findings Adversary"],
    );
    git(
        project,
        BEFORE,
        &["config", "user.email", "findings-adversary@example.invalid"],
    );
    git(project, BEFORE, &["add", "--all"]);
    git(
        project,
        BEFORE,
        &["commit", "--quiet", "--no-verify", "-m", "old review"],
    );
    write(&project.join("README"), "later\n");
    git(project, AFTER, &["add", "README"]);
    git(
        project,
        AFTER,
        &["commit", "--quiet", "--no-verify", "-m", "later"],
    );
}

/// `validate --format json` over the store in `project`: its exit code and the standing of
/// `review-result:old`.
fn old_review_standing(project: &Path) -> (i32, Option<String>, String) {
    let store = project.join(".engineering/planning");
    let validated = aep(&[
        "plan",
        "artifact",
        "validate",
        "--format",
        "json",
        "--store",
        printable(&store),
    ]);
    let summary: serde_json::Value = serde_json::from_str(&stdout(&validated))
        .unwrap_or_else(|error| panic!("validate prints JSON ({error}): {}", stderr(&validated)));
    let standing = summary["findings_standing"].as_array().and_then(|entries| {
        entries
            .iter()
            .find(|entry| entry["review"] == "review-result:old")
            .and_then(|entry| entry["standing"].as_str())
            .map(str::to_owned)
    });
    (code(&validated), standing, summary.to_string())
}

/// A clone of depth 1 is what `actions/checkout` makes by default. The review was committed months
/// before the opt-in; the clone holds the same files and the same commit at its tip, but not the
/// commit that added the review: `git log --diff-filter=A` reports every file as added by the
/// boundary commit. Dating the review by that commit would date it by the clone, so the review is
/// undated, and undated is never "before" the opt-in: it is counted `missing`, and the problem says
/// why it could not be dated and that the full history is the remedy.
#[test]
fn a_shallow_clone_leaves_a_review_in_its_boundary_commit_undated_and_names_the_full_history() {
    let directory = scratch("shallow");
    let origin = directory.join("origin");
    opted_in_repository_with_an_old_prose_review(&origin);

    let (full_code, full_standing, full_summary) = old_review_standing(&origin);
    assert_eq!(
        (full_code, full_standing.as_deref()),
        (0, Some("exempt_before_opt_in")),
        "control: in the full history the old review predates the opt-in: {full_summary}"
    );

    let shallow = directory.join("shallow");
    git(
        &directory,
        AFTER,
        &[
            "clone",
            "--quiet",
            "--depth",
            "1",
            &format!("file://{}", origin.display()),
            printable(&shallow),
        ],
    );
    let (shallow_code, shallow_standing, shallow_summary) = old_review_standing(&shallow);
    assert_eq!(
        (shallow_code, shallow_standing.as_deref()),
        (1, Some("missing")),
        "a review the shallow clone cannot date is undated, and undated is not before the \
         opt-in: {shallow_summary}"
    );
    let summary: serde_json::Value =
        serde_json::from_str(&shallow_summary).expect("the summary is JSON");
    let problems: Vec<&str> = summary["problems"]
        .as_array()
        .expect("problems is a list")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let problem = problems
        .iter()
        .find(|problem| problem.contains("review-result:old"))
        .unwrap_or_else(|| panic!("the old review is not counted: {shallow_summary}"));
    assert!(
        problem.contains("shallow")
            && problem.contains("full history")
            && problem.contains("fetch-depth: 0"),
        "the problem does not say the clone is shallow and name the full history as the \
         remedy: {problem}"
    );
    let entry = summary["findings_standing"]
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry["review"] == "review-result:old")
        })
        .expect("the old review is listed");
    assert!(
        entry["created_at"].is_null(),
        "a review the clone cannot date carries no date: {entry}"
    );
}

/// `--prose-only <reason>` writes the reason as front matter, and in an SQLite or Postgres store
/// there is no file to read it from: `show` is the read surface. `show --format json` returns
/// `withholds`, and `findings: []` for a review with no block at all, so without `prose_only` a
/// reader cannot tell a review that found nothing from one recorded prose-only on purpose.
#[test]
fn show_returns_the_prose_only_reason_of_a_review_recorded_prose_only() {
    let directory = scratch("show");
    let store = directory.join("project/.engineering/planning");
    std::fs::create_dir_all(&store).expect("the planning directory is writable");
    write(
        &directory.join("project/.engineering/project.yaml"),
        "version: aep.project/5\nplanning_scope: findings-adversary\nprotocol: adp/1\n\
         profile: development.standard\n",
    );
    let epic = aep(&[
        "plan",
        "artifact",
        "new",
        "epic",
        "objectives",
        "--title",
        "Objectives",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&epic), 0, "{}", stderr(&epic));
    let body = directory.join("prose.md");
    write(&body, "# Attack\n\nThe loop never advances.\n");
    let reason = "the reviewing tool writes no block yet";
    let created = aep(&[
        "plan",
        "artifact",
        "new",
        "review-result",
        "prose",
        "--title",
        "prose",
        "--relate",
        "reviews:epic:objectives",
        "--from",
        printable(&body),
        "--prose-only",
        reason,
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&created), 0, "{}", stderr(&created));

    let shown = aep(&[
        "plan",
        "artifact",
        "show",
        "review-result:prose",
        "--format",
        "json",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&shown), 0, "{}", stderr(&shown));
    let document: serde_json::Value =
        serde_json::from_str(&stdout(&shown)).expect("show --format json prints JSON");
    assert_eq!(
        document["prose_only"].as_str(),
        Some(reason),
        "show does not return the recorded prose-only reason: {document}"
    );

    let text = aep(&[
        "plan",
        "artifact",
        "show",
        "review-result:prose",
        "--store",
        printable(&store),
    ]);
    assert_eq!(code(&text), 0, "{}", stderr(&text));
    assert!(
        stdout(&text)
            .lines()
            .any(|line| line.starts_with("prose_only") && line.contains(reason)),
        "show does not print the recorded prose-only reason: {}",
        stdout(&text)
    );
}
