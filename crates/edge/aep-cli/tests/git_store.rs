//! `aep.project/5`: the CLI over a Git-native planning store (git-native design §§ 3–7).
//!
//! Each test builds a disposable project under Cargo's per-target scratch directory, as a Git
//! repository with `store: {git: {}}`, and drives the real binaries against it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root, whose lifecycles the fixture project names.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// `from` to `to`, as a relative path: a committed project file refuses an absolute one.
fn relative(from: &Path, to: &Path) -> String {
    let base: Vec<_> = from.components().collect();
    let target: Vec<_> = to.components().collect();
    let shared = base
        .iter()
        .zip(&target)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; base.len() - shared];
    parts.extend(
        target[shared..]
            .iter()
            .map(|component| component.as_os_str().to_str().expect("a printable path")),
    );
    parts.join("/")
}

/// A fresh `aep.project/5` project named `name`, as a Git repository with nothing in it yet.
fn project(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("git-store-{name}"));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch project is removable");
    }
    let engineering = directory.join(".engineering");
    std::fs::create_dir_all(&engineering).expect("the scratch project is writable");
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&directory)
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "version: aep.project/5\nplanning_scope: git-store-test\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: {}\nstore:\n  git: {{}}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    directory
}

/// Runs `binary` with `args` from `directory`, as a fixed actor.
fn run_as(binary: &str, directory: &Path, args: &[&str]) -> Output {
    Command::new(binary)
        .args(args)
        .current_dir(directory)
        .env("AEP_ACTOR", "human:tester")
        .output()
        .expect("the binary runs")
}

/// Runs `aep plan artifact <args>` from `directory`.
fn aep(directory: &Path, args: &[&str]) -> Output {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    run_as(env!("CARGO_BIN_EXE_aep"), directory, &full)
}

/// Runs `aep plan artifact <args>` and requires it to succeed.
fn ok(directory: &Path, args: &[&str]) -> String {
    let output = aep(directory, args);
    assert!(
        output.status.success(),
        "`{}` failed: {}{}",
        args.join(" "),
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Every file below `directory`, relative to it, outside `.git`, sorted.
fn files(directory: &Path) -> Vec<String> {
    fn walk(base: &Path, directory: &Path, found: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.map(|entry| entry.expect("a directory entry")) {
            let path = entry.path();
            if entry.file_name() == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                found.push(
                    path.strip_prefix(base)
                        .expect("below the base")
                        .display()
                        .to_string(),
                );
            }
        }
    }
    let mut found = Vec::new();
    walk(directory, directory, &mut found);
    found.sort();
    found
}

/// Every file below `directory` with its bytes, so a test can tell which ones a verb changed.
fn contents(directory: &Path) -> Vec<(String, Vec<u8>)> {
    files(directory)
        .into_iter()
        .map(|file| {
            let bytes = std::fs::read(directory.join(&file)).expect("a listed file reads");
            (file, bytes)
        })
        .collect()
}

/// The files that differ between two snapshots, or exist in only one of them.
fn changed(before: &[(String, Vec<u8>)], after: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut names: Vec<String> = before
        .iter()
        .chain(after)
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|name| {
            let find = |set: &[(String, Vec<u8>)]| {
                set.iter()
                    .find(|(candidate, _)| candidate == name)
                    .map(|(_, bytes)| bytes.clone())
            };
            find(before) != find(after)
        })
        .collect()
}

/// Asserts no journal and no Eventlog state was ever created in the project.
fn assert_no_journal_or_state(project: &Path) {
    let all = files(project);
    assert!(
        !all.iter().any(|file| file.ends_with("journal.jsonl")),
        "a Git-native store keeps no journal: {all:?}"
    );
    assert!(
        !project.join(".engineering/state").exists(),
        "a Git-native store keeps no Eventlog state: {all:?}"
    );
}

#[test]
fn a_move_writes_exactly_the_artifacts_file_and_nothing_else() {
    let project = project("one-file-per-move");
    ok(&project, &["new", "story", "one", "--title", "One"]);
    assert_no_journal_or_state(&project);
    let document = project.join(".engineering/planning/story/one.md");
    let written = std::fs::read_to_string(&document).expect("new wrote the document");
    assert!(
        written.contains("format: aep.planning-md/3"),
        "a Git-native store writes its own format: {written}"
    );

    let before = contents(&project);
    ok(&project, &["move", "story:one", "--to", "proposed"]);
    let after = contents(&project);

    assert_eq!(
        changed(&before, &after),
        vec![".engineering/planning/story/one.md".to_owned()],
        "a move writes the moved artifact's file and no other"
    );
    let moved = std::fs::read_to_string(&document).expect("the document reads");
    assert!(moved.contains("status: proposed"), "{moved}");
    assert!(
        moved.contains("transitions:\n- {from: \"draft\", to: \"proposed\""),
        "the move is one appended transition line: {moved}"
    );
    assert_no_journal_or_state(&project);
}

#[test]
fn three_moves_are_three_moves_in_history() {
    let project = project("three-moves");
    ok(&project, &["new", "story", "walk", "--title", "Walk"]);
    for rung in ["proposed", "active", "archived"] {
        ok(&project, &["move", "story:walk", "--to", rung]);
    }
    let history = ok(&project, &["history", "story:walk", "--format", "json"]);
    let entries: serde_json::Value = serde_json::from_str(&history).expect("history is JSON");
    let moves: Vec<(String, String)> = entries
        .as_array()
        .expect("history lists entries")
        .iter()
        .map(|entry| &entry["change"])
        .filter(|change| change["change"] == "moved")
        .map(|change| {
            (
                change["from"].as_str().unwrap_or_default().to_owned(),
                change["to"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        moves,
        vec![
            ("draft".to_owned(), "proposed".to_owned()),
            ("proposed".to_owned(), "active".to_owned()),
            ("active".to_owned(), "archived".to_owned()),
        ],
        "history reads the document's transitions in order: {history}"
    );
    assert_no_journal_or_state(&project);
}

#[test]
fn recorded_evidence_is_one_file_and_admits_the_rung_that_requires_it() {
    let project = project("evidence-gate");
    ok(&project, &["new", "story", "gated", "--title", "Gated"]);
    ok(&project, &["move", "story:gated", "--to", "proposed"]);
    ok(&project, &["move", "story:gated", "--to", "active"]);

    let refused = aep(&project, &["move", "story:gated", "--to", "implemented"]);
    assert!(
        !refused.status.success(),
        "`implemented` requires a test result and none is held"
    );
    assert!(
        format!("{}{}", stdout(&refused), stderr(&refused)).contains("test_result"),
        "the refusal names the evidence it lacks: {}{}",
        stdout(&refused),
        stderr(&refused)
    );

    let before = contents(&project);
    ok(
        &project,
        &[
            "evidence",
            "story:gated",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    let after = contents(&project);
    let written = changed(&before, &after);
    assert_eq!(
        written.len(),
        1,
        "one evidence record, one file: {written:?}"
    );
    assert!(
        written[0].starts_with(".engineering/evidence/story/gated/")
            && Path::new(&written[0])
                .extension()
                .is_some_and(|extension| extension == "json"),
        "the record is filed under its artifact: {written:?}"
    );

    ok(&project, &["move", "story:gated", "--to", "implemented"]);
    let history = ok(&project, &["history", "story:gated"]);
    assert!(
        history.contains("test_result recorded from task check"),
        "{history}"
    );
    let explained = ok(&project, &["explain", "story:gated", "--format", "json"]);
    assert!(
        explained.contains("\"implemented\"") && explained.contains("test_result"),
        "explain reads the same record the gate decided on: {explained}"
    );
    assert_no_journal_or_state(&project);
}

#[test]
fn list_and_validate_answer_over_a_git_store() {
    let project = project("list-validate");
    ok(&project, &["new", "story", "alpha", "--title", "Alpha"]);
    ok(&project, &["new", "task", "beta", "--title", "Beta"]);
    ok(&project, &["move", "story:alpha", "--to", "proposed"]);

    let listed = ok(&project, &["list", "--format", "json"]);
    let listed: serde_json::Value = serde_json::from_str(&listed).expect("list is JSON");
    let text = listed.to_string();
    assert!(
        text.contains("story:alpha") && text.contains("task:beta"),
        "{text}"
    );

    let validated = ok(&project, &["validate"]);
    assert!(validated.contains("2 artifact(s)"), "{validated}");
}

#[test]
fn a_hand_edited_status_that_breaks_the_walk_fails_validate_naming_the_artifact() {
    let project = project("broken-walk");
    ok(&project, &["new", "story", "edited", "--title", "Edited"]);
    ok(&project, &["move", "story:edited", "--to", "proposed"]);
    let document = project.join(".engineering/planning/story/edited.md");
    let text = std::fs::read_to_string(&document).expect("the document reads");

    // The status line alone: it disagrees with where the transitions end.
    std::fs::write(
        &document,
        text.replace("status: proposed", "status: implemented"),
    )
    .expect("the edit is written");
    let refused = aep(&project, &["validate"]);
    assert!(!refused.status.success(), "{}", stdout(&refused));
    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(said.contains("edited"), "names the artifact: {said}");
    assert!(said.contains("transition"), "says why: {said}");

    // Both lines, consistently: the last transition now claims a move the lifecycle forbids.
    std::fs::write(
        &document,
        text.replace("status: proposed", "status: implemented")
            .replace("to: \"proposed\"", "to: \"implemented\""),
    )
    .expect("the edit is written");
    let refused = aep(&project, &["validate"]);
    assert!(!refused.status.success(), "{}", stdout(&refused));
    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(said.contains("story:edited"), "names the artifact: {said}");
    assert!(
        said.contains("`draft` -> `implemented`") && said.contains("does not permit"),
        "names the illegal step: {said}"
    );
}

#[test]
fn aep_and_protocol_list_a_git_store_identically() {
    let project = project("aliases");
    ok(&project, &["new", "story", "same", "--title", "Same"]);
    ok(&project, &["move", "story:same", "--to", "proposed"]);
    for format in ["text", "json"] {
        let args = ["plan", "artifact", "list", "--format", format];
        let canonical = run_as(env!("CARGO_BIN_EXE_aep"), &project, &args);
        let alias = run_as(env!("CARGO_BIN_EXE_protocol"), &project, &args);
        assert!(canonical.status.success(), "{}", stderr(&canonical));
        assert_eq!(canonical.status.code(), alias.status.code());
        assert_eq!(canonical.stdout, alias.stdout, "aep and protocol diverged");
        assert_eq!(canonical.stderr, alias.stderr, "aep and protocol diverged");
    }
}

#[test]
fn doctor_reports_a_git_store_as_ok() {
    let project = project("doctor");
    ok(&project, &["new", "story", "seen", "--title", "Seen"]);
    let root = printable(&project);
    let output = run_as(
        env!("CARGO_BIN_EXE_aep"),
        &project,
        &["doctor", "--root", root],
    );
    let report = stdout(&output);
    let line = report
        .lines()
        .find(|line| line.contains("planning-store:"))
        .unwrap_or_else(|| panic!("no planning-store line: {report}{}", stderr(&output)));
    assert!(line.starts_with("ok"), "a /5 store is ok: {line}");
    assert!(
        line.contains("store: git") && line.contains("1 artifact(s), 0 evidence file(s)"),
        "{line}"
    );
}
