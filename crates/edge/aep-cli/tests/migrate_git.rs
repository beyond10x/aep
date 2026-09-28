//! `aep plan store migrate git`: an `aep.project/3` tree store rewritten as an `aep.project/5`
//! Git-native one (git-native design § 8).

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

/// A committed `/3` tree store: `story:moved` moved twice with one test result recorded,
/// `story:drafted` never moved, and a relation between them.
fn tree_store(name: &str) -> PathBuf {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("migrate-git-{name}"));
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
    for (name, title) in [("moved", "Moved"), ("drafted", "Drafted")] {
        accepted(
            &project,
            &["plan", "artifact", "new", "story", name, "--title", title],
        );
    }
    accepted(
        &project,
        &[
            "plan",
            "artifact",
            "relate",
            "story:moved",
            "depends_on",
            "story:drafted",
        ],
    );
    for to in ["proposed", "active"] {
        accepted(
            &project,
            &["plan", "artifact", "move", "story:moved", "--to", to],
        );
    }
    accepted(
        &project,
        &[
            "plan",
            "artifact",
            "evidence",
            "story:moved",
            "--kind",
            "test_result",
            "--source",
            "task check",
        ],
    );
    commit(&project, "tree store");
    project
}

/// `(id, status)` for every artifact `plan artifact list --format json` answers.
fn listed(project: &Path) -> Vec<(String, String)> {
    let output = aep(project, &["plan", "artifact", "list", "--format", "json"]);
    assert!(output.status.success(), "list: {}", text(&output));
    let output = String::from_utf8_lossy(&output.stdout).into_owned();
    let value: serde_json::Value = serde_json::from_str(&output)
        .unwrap_or_else(|error| panic!("list JSON: {error}: {output}"));
    let items = value
        .as_array()
        .cloned()
        .or_else(|| value.get("artifacts").and_then(|v| v.as_array()).cloned())
        .unwrap_or_else(|| panic!("no artifact list in {output}"));
    let mut pairs: Vec<(String, String)> = items
        .iter()
        .map(|item| {
            (
                item["id"].as_str().expect("an id").to_owned(),
                item["status"].as_str().expect("a status").to_owned(),
            )
        })
        .collect();
    pairs.sort();
    pairs
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn a_tree_store_migrates_to_a_git_native_one_and_verifies() {
    let project = tree_store("apply");
    let before = listed(&project);
    assert_eq!(
        before,
        vec![
            ("story:drafted".to_owned(), "draft".to_owned()),
            ("story:moved".to_owned(), "active".to_owned()),
        ]
    );

    let output = accepted(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(output.contains("verified 2 artifact(s)"), "{output}");

    let engineering = project.join(".engineering");
    assert!(
        !engineering.join("state").exists(),
        "the event log is still there"
    );
    assert!(!engineering
        .join("planning/.aep-projection-ownership.json")
        .exists());
    let selector: serde_json::Value =
        serde_yaml::from_str(&read(&engineering.join("project.yaml"))).expect("the selector");
    assert_eq!(selector["version"], "aep.project/5");
    assert_eq!(selector["store"], serde_json::json!({ "git": {} }));
    assert_eq!(selector["planning_scope"], "fixture");
    assert!(selector.get("planning_identity").is_none());
    assert!(selector.get("planning_tenant").is_none());
    assert_eq!(selector["protocol"], "adp/1");

    let moved = read(&engineering.join("planning/story/moved.md"));
    assert!(moved.contains("format: aep.planning-md/3"), "{moved}");
    let transition_lines: Vec<&str> = moved
        .lines()
        .filter(|line| line.starts_with("- {from:"))
        .collect();
    assert_eq!(transition_lines.len(), 2, "{moved}");
    assert!(
        transition_lines[0].contains("from: \"draft\", to: \"proposed\""),
        "{moved}"
    );
    assert!(
        transition_lines[1].contains("from: \"proposed\", to: \"active\""),
        "{moved}"
    );
    assert!(moved.contains("status: active"), "{moved}");
    assert!(moved.contains("depends_on"), "{moved}");

    let drafted = read(&engineering.join("planning/story/drafted.md"));
    assert!(drafted.contains("format: aep.planning-md/3"), "{drafted}");
    assert!(!drafted.contains("transitions:"), "{drafted}");

    let evidence = engineering.join("evidence/story/moved");
    let files: Vec<_> = std::fs::read_dir(&evidence)
        .expect("the evidence directory")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(files.len(), 1, "one test_result record");
    assert!(read(&files[0].path()).contains("test_result"));

    // The Git-native store answers the same artifacts at the same statuses.
    assert_eq!(listed(&project), before);
}

#[test]
fn a_dry_run_writes_nothing() {
    let project = tree_store("dry-run");
    let output = accepted(&project, &["plan", "store", "migrate", "git", "--dry-run"]);
    assert!(output.contains("would write 2 document(s)"), "{output}");
    assert!(output.contains("2 transition(s)"), "{output}");
    assert!(output.contains("1 evidence file(s)"), "{output}");
    assert!(output.contains("nothing was written"), "{output}");
    let status = git(&project, &["status", "--porcelain", "--ignored"]);
    assert!(
        status.stdout.is_empty(),
        "a dry run changed the tree: {}",
        String::from_utf8_lossy(&status.stdout)
    );
}

#[test]
fn an_uncommitted_engineering_directory_is_refused() {
    let project = tree_store("dirty");
    let document = project.join(".engineering/planning/story/drafted.md");
    let edited = format!("{}\nA hand edit.\n", read(&document));
    std::fs::write(&document, &edited).expect("an edit");
    let output = aep(&project, &["plan", "store", "migrate", "git"]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("uncommitted changes"),
        "{}",
        text(&output)
    );
    assert!(project.join(".engineering/state").exists());
    assert_eq!(read(&document), edited);
}
