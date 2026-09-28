//! `aep.project/5` as the default: `aep plan store migrate git` over an `aep.project/1` Markdown
//! store, `reverse init` writing `/5`, the one-line upgrade notice and `aep doctor`'s warning.
//!
//! Each test builds a disposable project under Cargo's per-target scratch directory and drives the
//! real binaries against it.

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

/// An empty scratch directory named `name`, as a Git repository when `git` is true.
fn scratch(name: &str, git: bool) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("migrate-git-{name}"));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the previous scratch project is removable");
    }
    std::fs::create_dir_all(directory.join(".engineering")).expect("the scratch is writable");
    if git {
        run_git(&directory, &["init", "--quiet"]);
    }
    directory
}

/// A fresh `aep.project/1` Markdown project named `name`, in a Git repository.
fn v1_project(name: &str) -> PathBuf {
    let directory = scratch(name, true);
    let engineering = directory.join(".engineering");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\n\
             protocols: {}\nsummary: kept through the migration\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    directory
}

fn run_git(directory: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .current_dir(directory)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {}", args.join(" "));
}

fn commit_all(directory: &Path) {
    run_git(directory, &["add", "-A"]);
    run_git(directory, &["commit", "--quiet", "-m", "fixture"]);
}

/// Runs `binary` with `args` from `directory`, as a fixed actor, with `env` set.
fn run_with(binary: &str, directory: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(directory)
        .env("AEP_ACTOR", "human:tester")
        .env_remove("AEP_NO_UPGRADE_NOTICE");
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the binary runs")
}

fn aep(directory: &Path, args: &[&str]) -> Output {
    run_with(env!("CARGO_BIN_EXE_aep"), directory, args, &[])
}

/// Runs `aep plan artifact <args>` and requires it to succeed.
fn ok(directory: &Path, args: &[&str]) -> String {
    let mut full = vec!["plan", "artifact"];
    full.extend_from_slice(args);
    let output = aep(directory, &full);
    assert!(
        output.status.success(),
        "`{}` failed: {}{}",
        args.join(" "),
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Every file below `directory`, outside `.git`, with its bytes, sorted by path.
fn contents(directory: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, directory: &Path, found: &mut Vec<(String, Vec<u8>)>) {
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
                let relative = path
                    .strip_prefix(base)
                    .expect("below the base")
                    .display()
                    .to_string();
                found.push((relative, std::fs::read(&path).expect("a listed file reads")));
            }
        }
    }
    let mut found = Vec::new();
    walk(directory, directory, &mut found);
    found.sort();
    found
}

/// One journalled evidence record about `artifact`, as a `/1` store's recorder writes it.
fn evidence_line(artifact: &str, revision: u64) -> String {
    serde_json::json!({
        "at": "2026-09-28T10:00:00Z",
        "actor": "human:tester",
        "artifact": artifact,
        "kind": "story",
        "revision": revision,
        "change": {"change": "evidence", "kind": "test_result", "source": "task check"}
    })
    .to_string()
}

/// A committed `/1` store: two stories, three moves, and two identical evidence records.
fn populated_v1(name: &str) -> PathBuf {
    let project = v1_project(name);
    ok(&project, &["new", "story", "walked", "--title", "Walked"]);
    ok(&project, &["move", "story:walked", "--to", "proposed"]);
    ok(&project, &["move", "story:walked", "--to", "active"]);
    ok(&project, &["new", "story", "still", "--title", "Still"]);
    ok(&project, &["move", "story:still", "--to", "proposed"]);
    let journal = project.join(".engineering/planning/journal.jsonl");
    let mut text = std::fs::read_to_string(&journal).expect("the /1 store journals");
    let line = evidence_line("story:walked", 3);
    for _ in 0..2 {
        text.push_str(&line);
        text.push('\n');
    }
    std::fs::write(&journal, text).expect("the journal is writable");
    commit_all(&project);
    project
}

#[test]
fn a_v1_store_migrates_with_verify_keeping_its_moves_and_evidence_and_dropping_its_journal() {
    let project = populated_v1("verify");
    let before = ok(&project, &["list", "--format", "json"]);

    let migrated = aep(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    assert!(
        stdout(&migrated).contains("verified 2 artifact(s)"),
        "{}",
        stdout(&migrated)
    );

    let engineering = project.join(".engineering");
    assert!(
        !engineering.join("planning/journal.jsonl").exists(),
        "the journal is removed"
    );
    let selector =
        std::fs::read_to_string(engineering.join("project.yaml")).expect("the selector reads");
    assert!(selector.contains("aep.project/5"), "{selector}");
    assert!(
        selector.contains("planning_scope: migrate-git-verify"),
        "the scope is the repository directory's name: {selector}"
    );
    assert!(
        selector.contains("kept through the migration"),
        "other keys are kept: {selector}"
    );

    let walked = std::fs::read_to_string(engineering.join("planning/story/walked.md"))
        .expect("the document reads");
    assert!(walked.contains("format: aep.planning-md/3"), "{walked}");
    assert_eq!(
        walked.matches("imported: true").count(),
        2,
        "both journalled moves are carried as imported transitions: {walked}"
    );
    let first = walked.find("to: \"proposed\"").expect("the first move");
    let second = walked.find("to: \"active\"").expect("the second move");
    assert!(first < second, "transitions keep the journal's order");

    let evidence: Vec<_> = contents(&engineering.join("evidence"))
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        evidence.len(),
        2,
        "identical records stay two pieces of evidence: {evidence:?}"
    );
    assert!(evidence
        .iter()
        .all(|path| path.starts_with("story/walked/")));

    let after = ok(&project, &["list", "--format", "json"]);
    assert_eq!(before, after, "`list` answers the same before and after");
    ok(&project, &["validate"]);
}

#[test]
fn a_dry_run_writes_nothing() {
    let project = populated_v1("dry-run");
    let before = contents(&project);
    let output = aep(&project, &["plan", "store", "migrate", "git", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("would write 2 document(s)"),
        "{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("dry run: nothing was written"),
        "{}",
        stdout(&output)
    );
    assert_eq!(before, contents(&project), "a dry run changed a file");
}

#[test]
fn a_dirty_store_is_refused_and_left_as_it_was() {
    let project = populated_v1("dirty");
    ok(
        &project,
        &["new", "story", "uncommitted", "--title", "Uncommitted"],
    );
    let before = contents(&project);
    let output = aep(&project, &["plan", "store", "migrate", "git"]);
    assert!(!output.status.success(), "a dirty store must be refused");
    assert!(
        stderr(&output).contains("uncommitted changes"),
        "{}",
        stderr(&output)
    );
    assert_eq!(before, contents(&project));
}

#[test]
fn a_status_its_last_move_did_not_reach_refuses_the_whole_migration() {
    let project = populated_v1("inconsistent");
    let path = project.join(".engineering/planning/story/still.md");
    let text = std::fs::read_to_string(&path).expect("the document reads");
    std::fs::write(&path, text.replace("status: proposed", "status: active"))
        .expect("the document is writable");
    commit_all(&project);
    let before = contents(&project);
    let output = aep(&project, &["plan", "store", "migrate", "git"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("story:still: `status: active`, but the last journalled move"),
        "the refusal names the artifact: {}",
        stdout(&output)
    );
    assert_eq!(before, contents(&project), "a refusal writes nothing");
}

#[test]
fn reverse_init_writes_a_v5_project_whose_new_and_move_each_write_one_file() {
    let project = scratch("reverse-init", true);
    std::fs::remove_dir_all(project.join(".engineering")).expect("the scratch is removable");
    let tree = relative(&project.join(".engineering"), &repository());
    let init = aep(
        &project,
        &[
            "plan",
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
    );
    assert!(init.status.success(), "{}", stderr(&init));
    let selector = std::fs::read_to_string(project.join(".engineering/project.yaml"))
        .expect("the project file was written");
    assert!(selector.contains("version: aep.project/5"), "{selector}");
    assert!(
        selector.contains("planning_scope: \"migrate-git-reverse-init\""),
        "{selector}"
    );
    assert!(selector.contains("git: {}"), "{selector}");

    let before = contents(&project);
    ok(&project, &["new", "story", "first", "--title", "First"]);
    let created = contents(&project);
    let written: Vec<_> = created
        .iter()
        .filter(|entry| !before.contains(entry))
        .map(|(path, _)| path.clone())
        .collect();
    assert_eq!(written, [".engineering/planning/story/first.md"]);

    ok(&project, &["move", "story:first", "--to", "proposed"]);
    let moved = contents(&project);
    let changed: Vec<_> = moved
        .iter()
        .filter(|entry| !created.contains(entry))
        .map(|(path, _)| path.clone())
        .collect();
    assert_eq!(changed, [".engineering/planning/story/first.md"]);
    assert!(
        !project.join(".engineering/planning/journal.jsonl").exists(),
        "a /5 store keeps no journal"
    );
}

#[test]
fn reverse_init_over_an_existing_plan_is_refused_and_migrate_git_adopts_it_as_v5() {
    let project = populated_v1("unconfigured");
    std::fs::remove_file(project.join(".engineering/project.yaml")).expect("removable");
    commit_all(&project);
    let tree = relative(&project.join(".engineering"), &repository());

    let before = contents(&project);
    let init = aep(
        &project,
        &[
            "plan",
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
    );
    assert_eq!(init.status.code(), Some(1), "{}", stdout(&init));
    assert!(
        stderr(&init).contains("already holds a plan")
            && stderr(&init).contains("aep plan store migrate git"),
        "the refusal names the migration: {}",
        stderr(&init)
    );
    assert_eq!(before, contents(&project), "a refused init writes nothing");

    let bare = aep(&project, &["plan", "store", "migrate", "git"]);
    assert_eq!(bare.status.code(), Some(1));
    assert!(
        stderr(&bare).contains("--protocols <source> --profile <profile>"),
        "{}",
        stderr(&bare)
    );

    let migrated = aep(
        &project,
        &[
            "plan",
            "store",
            "migrate",
            "git",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
            "--verify",
        ],
    );
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    let selector = std::fs::read_to_string(project.join(".engineering/project.yaml"))
        .expect("the project file was written");
    assert!(selector.contains("aep.project/5"), "{selector}");
    assert!(
        selector.contains("planning_scope: migrate-git-unconfigured"),
        "{selector}"
    );
    ok(&project, &["validate"]);
}

#[test]
fn the_upgrade_notice_is_one_stderr_line_for_v1_and_absent_for_v5_or_when_suppressed() {
    let project = populated_v1("notice");
    let listed = aep(&project, &["plan", "artifact", "list"]);
    assert!(listed.status.success(), "{}", stderr(&listed));
    let notice = stderr(&listed);
    assert_eq!(
        notice
            .lines()
            .filter(|line| line.starts_with("note: "))
            .count(),
        1,
        "exactly one notice: {notice}"
    );
    assert!(
        notice.contains("aep.project/1") && notice.contains("aep plan store migrate git --verify"),
        "{notice}"
    );

    let quiet = run_with(
        env!("CARGO_BIN_EXE_aep"),
        &project,
        &["plan", "artifact", "list"],
        &[("AEP_NO_UPGRADE_NOTICE", "1")],
    );
    assert_eq!(quiet.status.code(), listed.status.code());
    assert_eq!(
        quiet.stdout, listed.stdout,
        "stdout does not carry the notice"
    );
    assert!(
        !stderr(&quiet).contains("note: "),
        "the variable suppresses it: {}",
        stderr(&quiet)
    );

    let json = aep(&project, &["plan", "artifact", "list", "--format", "json"]);
    serde_json::from_slice::<serde_json::Value>(&json.stdout)
        .expect("`--format json` stdout stays one clean JSON document");

    // `protocol` prints the same bytes as `aep` (invariant 10), notice included.
    let alias = run_with(
        env!("CARGO_BIN_EXE_protocol"),
        &project,
        &["plan", "artifact", "list"],
        &[],
    );
    assert_eq!(alias.stdout, listed.stdout);
    assert_eq!(alias.stderr, listed.stderr);

    let migrated = aep(&project, &["plan", "store", "migrate", "git"]);
    assert!(migrated.status.success(), "{}", stderr(&migrated));
    assert!(
        !stderr(&migrated).contains("note: "),
        "the migration does not suggest itself"
    );
    let after = aep(&project, &["plan", "artifact", "list"]);
    assert!(
        !stderr(&after).contains("note: "),
        "a /5 store gets no notice: {}",
        stderr(&after)
    );
}

#[test]
fn doctor_warns_on_a_v1_store_naming_the_migration() {
    let project = v1_project("doctor");
    ok(&project, &["new", "story", "seen", "--title", "Seen"]);
    ok(&project, &["move", "story:seen", "--to", "proposed"]);
    let output = aep(&project, &["doctor", "--root", "."]);
    let report = stdout(&output);
    let line = report
        .lines()
        .find(|line| line.contains("planning-store:"))
        .unwrap_or_else(|| panic!("no planning-store line:\n{report}"));
    assert!(line.starts_with("warn"), "{line}");
    assert!(
        line.contains("aep plan store migrate git --verify"),
        "{line}"
    );
}
