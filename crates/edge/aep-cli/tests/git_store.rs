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
        let alias = run_as(env!("CARGO_BIN_EXE_aep"), &project, &args);
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

/// Commits everything in `project`, as a fixed identity and without hooks.
fn commit(project: &Path, message: &str) {
    for args in [
        vec!["add", "--all"],
        vec![
            "-c",
            "user.name=tester",
            "-c",
            "user.email=tester@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--no-verify",
            "-m",
            message,
        ],
    ] {
        let status = Command::new("git")
            .args(&args)
            .current_dir(project)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {}", args.join(" "));
    }
}

/// Runs `validate` and requires it to refuse, returning everything it said.
fn refused_validate(project: &Path) -> String {
    let refused = aep(project, &["validate"]);
    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(!refused.status.success(), "validate passed: {said}");
    said
}

#[test]
fn a_hand_edited_status_on_a_never_moved_artifact_fails_validate_naming_it() {
    let project = project("unmoved-edit");
    ok(
        &project,
        &["new", "story", "untouched", "--title", "Untouched"],
    );
    let document = project.join(".engineering/planning/story/untouched.md");
    let text = std::fs::read_to_string(&document).expect("the document reads");
    let edited = text.replace("status: draft", "status: implemented");
    assert_ne!(text, edited, "the fixture has a status line to edit");

    // Before its first commit, and after it: neither is a history that predates recorded moves.
    std::fs::write(&document, &edited).expect("the edit is written");
    let said = refused_validate(&project);
    assert!(
        said.contains("story:untouched"),
        "names the artifact: {said}"
    );
    assert!(
        said.contains("initial state `draft`"),
        "says where a never-moved artifact stands: {said}"
    );

    std::fs::write(&document, &text).expect("the edit is undone");
    commit(&project, "create");
    std::fs::write(&document, &edited).expect("the edit is written");
    let said = refused_validate(&project);
    assert!(
        said.contains("story:untouched") && said.contains("initial state `draft`"),
        "a committed Git-native artifact is held to its initial state too: {said}"
    );
}

#[test]
fn a_new_artifact_validates_before_and_after_its_first_commit() {
    let project = project("unmoved-new");
    ok(&project, &["new", "story", "fresh", "--title", "Fresh"]);
    ok(&project, &["validate"]);
    commit(&project, "create");
    ok(&project, &["validate"]);
}

#[test]
fn an_artifact_migrated_at_a_later_status_validates_and_keeps_that_status() {
    let project = project("unmoved-migrated");
    ok(&project, &["new", "story", "legacy", "--title", "Legacy"]);
    let document = project.join(".engineering/planning/story/legacy.md");
    let migrated = std::fs::read_to_string(&document)
        .expect("the document reads")
        .replace("status: draft", "status: implemented");
    // Its history begins in the format of a store that kept moves outside the file.
    std::fs::write(
        &document,
        migrated.replace("format: aep.planning-md/3", "format: aep.planning-md/1"),
    )
    .expect("the old version is written");
    commit(&project, "an older store");
    std::fs::write(&document, &migrated).expect("the migrated version is written");
    ok(&project, &["validate"]);
    commit(&project, "migrate");
    ok(&project, &["validate"]);

    std::fs::write(
        &document,
        migrated.replace("status: implemented", "status: active"),
    )
    .expect("the edit is written");
    let said = refused_validate(&project);
    assert!(
        said.contains("story:legacy") && said.contains("its history carries `implemented`"),
        "a migrated status is pinned to what the migration wrote: {said}"
    );
}

#[test]
fn an_edited_committed_evidence_file_fails_validate_naming_it() {
    let project = project("evidence-edit");
    ok(&project, &["new", "story", "proven", "--title", "Proven"]);
    ok(
        &project,
        &[
            "evidence",
            "story:proven",
            "--kind",
            "test_result",
            "--source",
            "task check",
            "--at",
            "2026-09-28T10:00:00Z",
        ],
    );
    // Uncommitted, a new evidence file is simply new.
    ok(&project, &["validate"]);
    commit(&project, "record");
    ok(&project, &["validate"]);

    let directory = project.join(".engineering/evidence/story/proven");
    let file = std::fs::read_dir(&directory)
        .expect("the evidence directory reads")
        .map(|entry| entry.expect("an entry").path())
        .next()
        .expect("one evidence file");
    let text = std::fs::read_to_string(&file).expect("the evidence reads");
    let edited = text.replace("task check", "task check --release");
    assert_ne!(text, edited, "the fixture has a source to edit");
    std::fs::write(&file, edited).expect("the edit is written");
    let said = refused_validate(&project);
    let name = file
        .file_name()
        .expect("a file name")
        .to_string_lossy()
        .into_owned();
    assert!(said.contains(&name), "names the file: {said}");
    assert!(
        said.contains("differs from its committed blob"),
        "says why: {said}"
    );
}

/// A committed review result, and the path of its document.
fn committed_review(name: &str) -> (PathBuf, PathBuf) {
    let project = project(name);
    let body = project.join("review-body.md");
    std::fs::write(&body, "## Verdict\n\nApproved as written.\n").expect("the body is written");
    ok(&project, &["new", "story", "subject", "--title", "Subject"]);
    ok(
        &project,
        &[
            "new",
            "review-result",
            "looked",
            "--title",
            "Looked at it",
            "--relate",
            "reviews:story:subject",
            "--from",
            printable(&body),
        ],
    );
    std::fs::remove_file(&body).expect("the body file is removable");
    commit(&project, "review");
    ok(&project, &["validate"]);
    let document = project.join(".engineering/planning/review-result/looked.md");
    (project, document)
}

#[test]
fn an_edited_committed_review_result_body_fails_validate_naming_it() {
    let (project, document) = committed_review("review-edit");
    let text = std::fs::read_to_string(&document).expect("the document reads");
    let edited = text.replace("Approved as written.", "Approved, with changes.");
    assert_ne!(text, edited, "the fixture has a verdict to edit");
    std::fs::write(&document, edited).expect("the edit is written");
    let said = refused_validate(&project);
    assert!(said.contains("review-result:looked"), "names it: {said}");
    assert!(said.contains("immutable"), "says why: {said}");

    // Committing the edit does not launder it: the baseline is the creating commit.
    commit(&project, "an edit");
    let said = refused_validate(&project);
    assert!(
        said.contains("review-result:looked") && said.contains("body"),
        "{said}"
    );
}

#[test]
fn a_move_on_a_committed_review_result_still_validates() {
    let (project, _) = committed_review("review-move");
    ok(
        &project,
        &["move", "review-result:looked", "--to", "archived"],
    );
    ok(&project, &["validate"]);
    commit(&project, "retire");
    ok(&project, &["validate"]);
}

#[test]
fn a_move_with_an_executor_records_it_and_history_and_explain_print_it() {
    let project = project("executor");
    ok(
        &project,
        &["new", "story", "delegated", "--title", "Delegated"],
    );
    ok(
        &project,
        &[
            "move",
            "story:delegated",
            "--to",
            "proposed",
            "--executor",
            "agent:x",
            "--correlation",
            "wave-7",
        ],
    );
    let document =
        std::fs::read_to_string(project.join(".engineering/planning/story/delegated.md"))
            .expect("the document reads");
    assert!(
        document.contains(
            "actor: \"human:tester\", revision: 2, executor: \"agent:x\", correlation: \"wave-7\"}"
        ),
        "{document}"
    );

    let history = ok(&project, &["history", "story:delegated"]);
    assert!(
        history.contains(
            "human:tester  moved draft -> proposed, executed by agent:x, correlation wave-7"
        ),
        "{history}"
    );
    let history = ok(
        &project,
        &["history", "story:delegated", "--format", "json"],
    );
    let entries: serde_json::Value = serde_json::from_str(&history).expect("history is JSON");
    let change = &entries[0]["change"];
    assert_eq!(change["executor"], "agent:x", "{history}");
    assert_eq!(change["correlation"], "wave-7", "{history}");

    let explained = ok(&project, &["explain", "story:delegated"]);
    assert!(
        explained.contains("(revision 2, executed by agent:x, correlation wave-7)"),
        "{explained}"
    );
    let explained = ok(
        &project,
        &["explain", "story:delegated", "--format", "json"],
    );
    let explained: serde_json::Value = serde_json::from_str(&explained).expect("explain is JSON");
    assert_eq!(explained["reached"][0]["executor"], "agent:x");
    assert_eq!(explained["reached"][0]["correlation"], "wave-7");
}

#[test]
fn a_move_by_the_actor_alone_writes_no_executor_key() {
    let project = project("actor-alone");
    ok(&project, &["new", "story", "own", "--title", "Own"]);
    ok(&project, &["move", "story:own", "--to", "proposed"]);
    ok(
        &project,
        &[
            "move",
            "story:own",
            "--to",
            "active",
            "--executor",
            "human:tester",
        ],
    );
    let document = std::fs::read_to_string(project.join(".engineering/planning/story/own.md"))
        .expect("the document reads");
    assert!(
        !document.contains("executor") && !document.contains("correlation"),
        "neither the actor as its own executor nor the placeholder correlation is written: {document}"
    );
    let history = ok(&project, &["history", "story:own", "--format", "json"]);
    assert!(!history.contains("\"executor\""), "{history}");
    let explained = ok(&project, &["explain", "story:own", "--format", "json"]);
    let explained: serde_json::Value = serde_json::from_str(&explained).expect("explain is JSON");
    for step in explained["reached"].as_array().expect("moves") {
        assert!(
            step.get("executor").is_none() && step.get("correlation").is_none(),
            "{step}"
        );
    }
}

#[test]
fn a_move_refuses_an_executor_that_is_not_an_actor() {
    let project = project("bad-executor");
    ok(&project, &["new", "story", "bad", "--title", "Bad"]);
    let refused = aep(
        &project,
        &[
            "move",
            "story:bad",
            "--to",
            "proposed",
            "--executor",
            "not an actor",
        ],
    );
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("--executor"),
        "{}",
        stderr(&refused)
    );
}

/// A store written by the released 0.64.0 binary validates and reads as it did, and a move with an
/// executor appends a line without touching the lines 0.64.0 wrote.
#[test]
fn a_store_written_by_0_64_0_validates_and_keeps_its_lines_when_moved_again() {
    let project = project("written-by-0-64-0");
    let fixture =
        repository().join("crates/plan/aep-backend-markdown/tests/fixtures/written-by-0.64.0");
    for (from, to) in [("planning", "planning"), ("evidence", "evidence")] {
        copy_tree(&fixture.join(from), &project.join(".engineering").join(to));
    }
    let path = project.join(".engineering/planning/story/fixture.md");
    let written = std::fs::read_to_string(&path).expect("the fixture copied");

    let validated = ok(&project, &["validate"]);
    assert!(validated.contains("1 artifact(s)"), "{validated}");
    let history = ok(&project, &["history", "story:fixture", "--format", "json"]);
    assert!(!history.contains("\"executor\""), "{history}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("reads"),
        written,
        "reading changes no byte"
    );

    ok(
        &project,
        &[
            "move",
            "story:fixture",
            "--to",
            "archived",
            "--executor",
            "agent:x",
        ],
    );
    let moved = std::fs::read_to_string(&path).expect("reads");
    let old_lines: Vec<&str> = written
        .lines()
        .filter(|line| line.starts_with("- {"))
        .collect();
    for line in &old_lines {
        assert!(
            moved.contains(line),
            "`{line}` kept byte for byte:\n{moved}"
        );
    }
    assert!(moved.contains("executor: \"agent:x\"}"), "{moved}");
    ok(&project, &["validate"]);
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a target directory");
    for entry in std::fs::read_dir(from).expect("a fixture directory") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("a fixture file copies");
        }
    }
}
