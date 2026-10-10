//! `aep plan store migrate git` over an `aep.project/1` Markdown store — the one reader of that
//! layout this build keeps — `reverse init` writing `/5`, the refusal of a `/1` store, and the
//! repair `migrate git` makes of a `/5` store that repeats a transition.
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
        .env("AEP_ACTOR", "human:tester");
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

/// One journalled evidence record about `artifact`, as a `/1` store's recorder wrote it.
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

/// One journal line in the shape a `/1` store wrote before its provider: an entry.
fn entry_line(at: &str, artifact: &str, revision: u64, change: &serde_json::Value) -> String {
    serde_json::json!({
        "at": at,
        "actor": "human:tester",
        "artifact": artifact,
        "kind": "story",
        "revision": revision,
        "change": change
    })
    .to_string()
}

/// One journal line in the shape a `/1` store's provider wrote: a domain event carrying the change.
fn event_line(at: &str, name: &str, revision: u64, change: &serde_json::Value) -> String {
    serde_json::json!({
        "entity": "story",
        "version": 1,
        "id": name,
        "revision": revision,
        "type": "moved",
        "from_state": change["from"],
        "to_state": change["to"],
        "changed": {"status": change["to"]},
        "args": {},
        "payload": {
            "recorded_at": at,
            "actor": "human:tester",
            "change": change
        }
    })
    .to_string()
}

/// An `aep.planning-md/1` story document.
fn story(name: &str, status: &str, revision: u64) -> String {
    format!(
        "---\nformat: aep.planning-md/1\nid: story:{name}\nkind: story\nstatus: {status}\n\
         title: {name}\nrevision: {revision}\n---\n\n# {name}\n"
    )
}

/// A `/1` store written the way a `/1` build left one: two stories, three moves (in both journal
/// shapes), two identical evidence records, and one story the journal never moved that a person
/// set to `active` by hand.
fn write_v1_store(project: &Path) {
    let planning = project.join(".engineering/planning");
    std::fs::create_dir_all(planning.join("story")).expect("the store is writable");
    for (name, status, revision) in [
        ("walked", "active", 3),
        ("still", "proposed", 2),
        ("handmade", "active", 1),
    ] {
        std::fs::write(
            planning.join(format!("story/{name}.md")),
            story(name, status, revision),
        )
        .expect("the document is writable");
    }
    let moved =
        |from: &str, to: &str| serde_json::json!({"change": "moved", "from": from, "to": to});
    let lines = [
        entry_line(
            "2026-09-28T09:00:00Z",
            "story:walked",
            1,
            &serde_json::json!({"change": "created", "status": "draft"}),
        ),
        event_line(
            "2026-09-28T09:10:00Z",
            "walked",
            2,
            &moved("draft", "proposed"),
        ),
        entry_line(
            "2026-09-28T09:20:00Z",
            "story:walked",
            3,
            &moved("proposed", "active"),
        ),
        entry_line(
            "2026-09-28T09:30:00Z",
            "story:still",
            1,
            &serde_json::json!({"change": "created", "status": "draft"}),
        ),
        event_line(
            "2026-09-28T09:40:00Z",
            "still",
            2,
            &moved("draft", "proposed"),
        ),
        evidence_line("story:walked", 3),
        evidence_line("story:walked", 3),
    ];
    std::fs::write(planning.join("journal.jsonl"), lines.join("\n") + "\n")
        .expect("the journal is writable");
}

/// A committed `/1` store.
fn populated_v1(name: &str) -> PathBuf {
    let project = v1_project(name);
    write_v1_store(&project);
    commit_all(&project);
    project
}

#[test]
fn a_v1_store_migrates_with_verify_keeping_its_moves_and_evidence_and_dropping_its_journal() {
    let project = populated_v1("verify");

    let migrated = aep(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    assert!(
        stdout(&migrated).contains("verified 3 artifact(s)"),
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
        "both journalled moves, one per journal shape, are carried as imported transitions: \
         {walked}"
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

    let listed = ok(&project, &["list"]);
    for line in ["story:walked", "story:still", "story:handmade"] {
        assert!(listed.contains(line), "{listed}");
    }
    let history = ok(&project, &["history", "story:walked"]);
    assert_eq!(
        history.lines().count(),
        4,
        "two moves and two evidence records: {history}"
    );
    ok(&project, &["validate"]);
}

/// `--verify` compares every evidence record in order, so a faithful migration of two distinct
/// records made in one second, and of one record journalled twice, must still verify clean: the
/// new store answers them in the old order and keeps both copies.
#[test]
fn same_second_records_and_identical_duplicates_verify_clean_in_order_and_multiplicity() {
    let project = v1_project("same-second");
    let planning = project.join(".engineering/planning");
    std::fs::create_dir_all(planning.join("story")).expect("the store is writable");
    std::fs::write(
        planning.join("story/observed.md"),
        story("observed", "draft", 1),
    )
    .expect("the document is writable");
    let observation = |at: &str, source: &str, reference: &str| {
        entry_line(
            at,
            "story:observed",
            1,
            &serde_json::json!({
                "change": "evidence",
                "kind": "test_result",
                "source": source,
                "reference": reference
            }),
        )
    };
    let lines = [
        entry_line(
            "2026-09-28T09:00:00Z",
            "story:observed",
            1,
            &serde_json::json!({"change": "created", "status": "draft"}),
        ),
        observation("2026-09-28T10:00:00Z", "first suite", "run-1"),
        observation("2026-09-28T10:00:00Z", "second suite", "run-2"),
        observation("2026-09-28T10:01:00Z", "repeated suite", "run-3"),
        observation("2026-09-28T10:01:00Z", "repeated suite", "run-3"),
    ];
    std::fs::write(planning.join("journal.jsonl"), lines.join("\n") + "\n")
        .expect("the journal is writable");
    commit_all(&project);

    let migrated = aep(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    assert!(
        stdout(&migrated).contains("verified 1 artifact(s) and 4 evidence record(s)"),
        "{}",
        stdout(&migrated)
    );

    let history = ok(&project, &["history", "story:observed"]);
    let sources: Vec<&str> = history
        .lines()
        .filter_map(|line| {
            ["first suite", "second suite", "repeated suite"]
                .into_iter()
                .find(|source| line.contains(source))
        })
        .collect();
    assert_eq!(
        sources,
        [
            "first suite",
            "second suite",
            "repeated suite",
            "repeated suite"
        ],
        "the same-second pair keeps its order and the duplicate both copies: {history}"
    );
}

/// A `/1` recorder took `--at`, so a journal can hold a record about one artifact observed before
/// one journalled ahead of it. Both records are migrated, each into its own file: the migration
/// is faithful, and an unmodified migration verifies clean.
#[test]
fn a_journal_with_backdated_evidence_migrates_and_verifies_clean() {
    let project = v1_project("backdated");
    let planning = project.join(".engineering/planning");
    std::fs::create_dir_all(planning.join("story")).expect("the store is writable");
    std::fs::write(
        planning.join("story/observed.md"),
        story("observed", "draft", 2),
    )
    .expect("the document is writable");
    let observation = |at: &str, revision: u64, source: &str| {
        entry_line(
            at,
            "story:observed",
            revision,
            &serde_json::json!({"change": "evidence", "kind": "test_result", "source": source}),
        )
    };
    let lines = [
        entry_line(
            "2026-09-28T09:00:00Z",
            "story:observed",
            1,
            &serde_json::json!({"change": "created", "status": "draft"}),
        ),
        observation("2026-09-28T10:00:00Z", 1, "recorded first"),
        observation("2026-09-28T09:30:00Z", 2, "observed earlier"),
    ];
    std::fs::write(planning.join("journal.jsonl"), lines.join("\n") + "\n")
        .expect("the journal is writable");
    commit_all(&project);

    let migrated = aep(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(
        migrated.status.success(),
        "a faithful migration of backdated evidence must verify clean:\n{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
}

/// Records of different kinds observed at one instant share the artifact's evidence directory and
/// its per-second sequence, so they keep journal order whatever their kinds' order.
#[test]
fn records_of_different_kinds_at_one_instant_verify_clean_in_journal_order() {
    let project = v1_project("one-instant-kinds");
    let planning = project.join(".engineering/planning");
    std::fs::create_dir_all(planning.join("story")).expect("the store is writable");
    std::fs::write(
        planning.join("story/observed.md"),
        story("observed", "draft", 1),
    )
    .expect("the document is writable");
    let observation = |kind: &str, source: &str| {
        entry_line(
            "2026-09-28T10:00:00Z",
            "story:observed",
            1,
            &serde_json::json!({"change": "evidence", "kind": kind, "source": source}),
        )
    };
    let lines = [
        observation("review", "kind review"),
        observation("approval", "kind approval"),
        observation("test_result", "kind test result"),
    ];
    std::fs::write(planning.join("journal.jsonl"), lines.join("\n") + "\n")
        .expect("the journal is writable");
    commit_all(&project);

    let migrated = aep(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    let history = ok(&project, &["history", "story:observed"]);
    let sources: Vec<&str> = history
        .lines()
        .filter_map(|line| {
            ["kind review", "kind approval", "kind test result"]
                .into_iter()
                .find(|source| line.contains(source))
        })
        .collect();
    assert_eq!(
        sources,
        ["kind review", "kind approval", "kind test result"],
        "one instant keeps journal order across kinds: {history}"
    );
}

/// A `/1` store first committed in the same commit as its migration — a squash of the two — is
/// valid: an artifact the journal never moved carries its status as one imported transition, so
/// `validate` does not depend on a history that begins in `aep.planning-md/1`.
#[test]
fn a_store_migrated_and_committed_in_one_commit_validates_and_a_hand_edited_status_is_still_refused(
) {
    let project = v1_project("one-commit");
    run_git(
        &project,
        &["commit", "--quiet", "--allow-empty", "-m", "empty"],
    );
    write_v1_store(&project);
    commit_all(&project);

    let migrated = aep(&project, &["plan", "store", "migrate", "git", "--verify"]);
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    assert!(
        stdout(&migrated).contains("1 artifact(s) the journal never moved"),
        "{}",
        stdout(&migrated)
    );
    let handmade = std::fs::read_to_string(project.join(".engineering/planning/story/handmade.md"))
        .expect("the document reads");
    assert!(
        handmade.contains("from: \"draft\", to: \"active\"") && handmade.contains("imported: true"),
        "the status is carried as one imported move from the initial state: {handmade}"
    );

    // The `/1` commit and the migration, squashed into one: every file's history now begins in
    // the Git-native format.
    run_git(&project, &["reset", "--quiet", "--soft", "HEAD~1"]);
    commit_all(&project);
    let validated = aep(&project, &["plan", "artifact", "validate"]);
    assert!(
        validated.status.success(),
        "{}{}",
        stdout(&validated),
        stderr(&validated)
    );

    // A document born in the Git-native store is still held to its transitions.
    ok(&project, &["new", "story", "born", "--title", "Born"]);
    commit_all(&project);
    let path = project.join(".engineering/planning/story/born.md");
    let text = std::fs::read_to_string(&path).expect("the document reads");
    std::fs::write(&path, text.replace("status: draft", "status: active"))
        .expect("the document is writable");
    let refused = aep(&project, &["plan", "artifact", "validate"]);
    assert_eq!(refused.status.code(), Some(1), "{}", stdout(&refused));
    assert!(
        stdout(&refused).contains("story:born: `status: active` with no transitions"),
        "{}",
        stdout(&refused)
    );
}

#[test]
fn a_dry_run_writes_nothing() {
    let project = populated_v1("dry-run");
    let before = contents(&project);
    let output = aep(&project, &["plan", "store", "migrate", "git", "--dry-run"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("would write 3 document(s)"),
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
    std::fs::write(
        project.join(".engineering/planning/story/uncommitted.md"),
        story("uncommitted", "draft", 1),
    )
    .expect("the document is writable");
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

    // `--engineering` names the scratch store: without it discovery climbs out of the scratch
    // directory, and on CI the target directory sits inside this repository's own checkout, whose
    // `.engineering/project.yaml` is already `aep.project/5`.
    let engineering = project.join(".engineering");
    let engineering = engineering.to_str().expect("a printable path");
    let bare = aep(
        &project,
        &[
            "plan",
            "store",
            "migrate",
            "git",
            "--engineering",
            engineering,
        ],
    );
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
            "--engineering",
            engineering,
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

/// A `/1` store is refused by every verb that opens it, naming the migration this build runs —
/// through its project file, and through `--store` at a directory that still holds its journal.
#[test]
fn a_v1_store_is_refused_naming_the_migration_until_it_is_migrated() {
    let project = populated_v1("refused");
    let listed = aep(&project, &["plan", "artifact", "list"]);
    assert_eq!(listed.status.code(), Some(1), "{}", stdout(&listed));
    assert!(
        stderr(&listed).contains("aep.project/1")
            && stderr(&listed).contains("aep plan store migrate git --verify"),
        "{}",
        stderr(&listed)
    );
    assert!(stdout(&listed).is_empty(), "{}", stdout(&listed));

    let unconfigured = scratch("refused-store", false);
    let planning = unconfigured.join("plan");
    write_v1_store(&unconfigured);
    std::fs::rename(unconfigured.join(".engineering/planning"), &planning)
        .expect("the store moves");
    let explicit = aep(
        &unconfigured,
        &[
            "plan",
            "artifact",
            "list",
            "--store",
            planning.to_str().expect("a printable path"),
        ],
    );
    assert_eq!(explicit.status.code(), Some(1), "{}", stdout(&explicit));
    assert!(
        stderr(&explicit).contains("journal.jsonl")
            && stderr(&explicit).contains("aep plan store migrate git --verify"),
        "{}",
        stderr(&explicit)
    );

    let migrated = aep(&project, &["plan", "store", "migrate", "git"]);
    assert!(migrated.status.success(), "{}", stderr(&migrated));
    let after = aep(&project, &["plan", "artifact", "list"]);
    assert!(after.status.success(), "{}", stderr(&after));
}

#[test]
fn doctor_names_the_migration_for_a_v1_store() {
    let project = populated_v1("doctor");
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

// ---------------------------------------------------------------------------------------------
// Where `planning_scope` comes from (story:planning-scope-comes-from-the-repository).
//
// These fixtures live under the system temporary directory, not Cargo's target directory: the
// target directory sits inside this repository's own checkout, whose `origin` and Git common
// directory would otherwise be the ones every derivation finds.
// ---------------------------------------------------------------------------------------------

/// A fresh, canonical directory under the system temporary directory, outside every repository.
/// The pid keeps concurrent runs from different worktrees apart, because they share `TMPDIR`.
fn outside(name: &str) -> PathBuf {
    let directory = std::env::temp_dir()
        .canonicalize()
        .expect("the temporary directory exists")
        .join(format!("aep-planning-scope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Runs `git` in `directory` isolated from the caller's repository and identity.
fn git_in(directory: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A primary checkout `<root>/<name>` holding a committed `aep.project/1` store.
fn primary_with_v1_store(root: &Path, name: &str) -> PathBuf {
    let primary = root.join(name);
    std::fs::create_dir_all(primary.join(".engineering")).expect("the checkout is writable");
    git_in(&primary, &["init", "--quiet"]);
    let engineering = primary.join(".engineering");
    std::fs::write(
        engineering.join("project.yaml"),
        format!(
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\n\
             protocols: {}\n",
            relative(&engineering, &repository())
        ),
    )
    .expect("the project file is writable");
    write_v1_store(&primary);
    git_in(&primary, &["add", "-A"]);
    git_in(&primary, &["commit", "--quiet", "-m", "fixture"]);
    primary
}

/// A primary checkout `<root>/<name>` with one committed file and no `.engineering/`.
fn primary_without_project(root: &Path, name: &str) -> PathBuf {
    let primary = root.join(name);
    std::fs::create_dir_all(&primary).expect("the checkout is writable");
    git_in(&primary, &["init", "--quiet"]);
    std::fs::write(primary.join("README.md"), "fixture\n").expect("the file is writable");
    git_in(&primary, &["add", "-A"]);
    git_in(&primary, &["commit", "--quiet", "-m", "fixture"]);
    primary
}

/// A linked worktree of `primary` at `<primary's parent>/<name>`.
fn linked(primary: &Path, name: &str) -> PathBuf {
    let path = primary.parent().expect("a scratch root").join(name);
    git_in(
        primary,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            name,
            path.to_str().expect("a printable path"),
        ],
    );
    path
}

/// A bare repository `<root>/<name>.git` with no `origin`, cloned from a checkout holding a
/// committed `/1` store, and a linked worktree of it at `<root>/checkout`.
fn bare_worktree_without_origin(root: &Path, name: &str) -> PathBuf {
    let seed = primary_with_v1_store(root, "seed");
    let bare = root.join(format!("{name}.git"));
    git_in(
        root,
        &[
            "clone",
            "--quiet",
            "--bare",
            seed.to_str().expect("a printable path"),
            bare.to_str().expect("a printable path"),
        ],
    );
    git_in(&bare, &["remote", "remove", "origin"]);
    let checkout = root.join("checkout");
    git_in(
        &bare,
        &[
            "worktree",
            "add",
            "--quiet",
            checkout.to_str().expect("a printable path"),
        ],
    );
    checkout
}

fn migrate_in(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["plan", "store", "migrate", "git"];
    args.extend_from_slice(extra);
    aep(directory, &args)
}

fn reverse_init_in(directory: &Path, extra: &[&str]) -> Output {
    let tree = relative(&directory.join(".engineering"), &repository());
    let mut args = vec![
        "plan",
        "reverse",
        "init",
        "--protocols",
        &tree,
        "--profile",
        "development.standard",
    ];
    args.extend_from_slice(extra);
    aep(directory, &args)
}

/// The `planning_scope` line of `<directory>/.engineering/project.yaml`, unquoted.
fn written_scope(directory: &Path) -> String {
    let selector = std::fs::read_to_string(directory.join(".engineering/project.yaml"))
        .expect("the project file was written");
    selector
        .lines()
        .find_map(|line| line.strip_prefix("planning_scope: "))
        .unwrap_or_else(|| panic!("no planning_scope line in:\n{selector}"))
        .trim_matches('"')
        .to_owned()
}

#[test]
fn migrate_git_in_a_linked_worktree_writes_the_primary_checkouts_name_not_the_worktrees() {
    let root = outside("migrate-linked");
    let primary = primary_with_v1_store(&root, "repo-a");
    let worktree = linked(&primary, "repo-a-wt-xyz");

    let migrated = migrate_in(&worktree, &[]);
    assert!(
        migrated.status.success(),
        "{}{}",
        stdout(&migrated),
        stderr(&migrated)
    );
    assert_eq!(
        written_scope(&worktree),
        "repo-a",
        "the scope is the repository's name, not the linked worktree's directory name"
    );
    assert!(
        stdout(&migrated)
            .contains("planning_scope: repo-a (from the primary checkout's directory)"),
        "the output names the scope's source: {}",
        stdout(&migrated)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn reverse_init_in_a_linked_worktree_writes_the_primary_checkouts_name_not_the_worktrees() {
    let root = outside("init-linked");
    let primary = primary_without_project(&root, "repo-a");
    let worktree = linked(&primary, "repo-a-wt-xyz");

    let init = reverse_init_in(&worktree, &[]);
    assert!(init.status.success(), "{}{}", stdout(&init), stderr(&init));
    assert_eq!(
        written_scope(&worktree),
        "repo-a",
        "the scope is the repository's name, not the linked worktree's directory name"
    );
    assert!(
        stdout(&init).contains("planning_scope: repo-a (from the primary checkout's directory)"),
        "the output names the scope's source: {}",
        stdout(&init)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_origin_remote_names_the_scope_ahead_of_the_primary_checkout() {
    let root = outside("origin");
    let primary = primary_with_v1_store(&root, "repo-a");
    git_in(
        &primary,
        &[
            "remote",
            "add",
            "origin",
            "git@example.invalid:org/service-name.git",
        ],
    );
    let worktree = linked(&primary, "repo-a-wt-origin");

    let migrated = migrate_in(&worktree, &[]);
    assert!(migrated.status.success(), "{}", stderr(&migrated));
    assert_eq!(
        written_scope(&worktree),
        "service-name",
        "the origin URL's last segment, without `.git`, wins over the checkout's name"
    );
    assert!(
        stdout(&migrated).contains("planning_scope: service-name (from the origin remote)"),
        "{}",
        stdout(&migrated)
    );

    let other = primary_without_project(&root, "repo-b");
    git_in(
        &other,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/org/other-service/",
        ],
    );
    let init = reverse_init_in(&other, &[]);
    assert!(init.status.success(), "{}", stderr(&init));
    assert_eq!(written_scope(&other), "other-service");
    assert!(
        stdout(&init).contains("planning_scope: other-service (from the origin remote)"),
        "{}",
        stdout(&init)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_planning_scope_flag_names_the_scope_ahead_of_every_derived_source() {
    let root = outside("flag");
    let primary = primary_with_v1_store(&root, "repo-a");
    git_in(
        &primary,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/org/remote-name.git",
        ],
    );
    let migrated = migrate_in(&primary, &["--planning-scope", "chosen"]);
    assert!(migrated.status.success(), "{}", stderr(&migrated));
    assert_eq!(
        written_scope(&primary),
        "chosen",
        "the flag wins over origin"
    );
    assert!(
        stdout(&migrated).contains("planning_scope: chosen (from --planning-scope)"),
        "{}",
        stdout(&migrated)
    );

    let other = primary_without_project(&root, "repo-b");
    let init = reverse_init_in(&other, &["--planning-scope", "chosen too"]);
    assert!(init.status.success(), "{}", stderr(&init));
    assert_eq!(written_scope(&other), "chosen too");
    assert!(
        stdout(&init).contains("planning_scope: chosen too (from --planning-scope)"),
        "{}",
        stdout(&init)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn outside_any_git_repository_the_scope_is_the_project_directorys_name() {
    let root = outside("no-git");
    let project = root.join("plain-project");
    std::fs::create_dir_all(&project).expect("the directory is writable");

    let init = reverse_init_in(&project, &[]);
    assert!(init.status.success(), "{}", stderr(&init));
    assert_eq!(written_scope(&project), "plain-project");
    assert!(
        stdout(&init).contains("planning_scope: plain-project (from the project directory's name)"),
        "{}",
        stdout(&init)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_bare_repositorys_worktree_without_origin_is_refused_naming_the_flag_and_writes_nothing() {
    let root = outside("bare");
    let checkout = bare_worktree_without_origin(&root, "repo-a");

    let before = contents(&checkout);
    let migrated = migrate_in(&checkout, &[]);
    assert_eq!(migrated.status.code(), Some(1), "{}", stdout(&migrated));
    assert!(
        stderr(&migrated).contains("--planning-scope")
            && stderr(&migrated).contains("no `origin` remote"),
        "the refusal says why and names the flag: {}",
        stderr(&migrated)
    );
    assert_eq!(
        before,
        contents(&checkout),
        "a refused migration writes nothing"
    );

    let explicit = migrate_in(&checkout, &["--planning-scope", "repo-a"]);
    assert!(explicit.status.success(), "{}", stderr(&explicit));
    assert_eq!(written_scope(&checkout), "repo-a");

    let fresh = root.join("fresh");
    git_in(
        &root.join("repo-a.git"),
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            fresh.to_str().expect("a printable path"),
        ],
    );
    std::fs::remove_dir_all(fresh.join(".engineering")).expect("the store is removable");
    let init = reverse_init_in(&fresh, &[]);
    assert_eq!(init.status.code(), Some(1), "{}", stdout(&init));
    assert!(
        stderr(&init).contains("--planning-scope") && stderr(&init).contains("no `origin` remote"),
        "{}",
        stderr(&init)
    );
    assert!(
        !fresh.join(".engineering/project.yaml").exists(),
        "a refused init writes no project file"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_planning_scope_flag_of_only_unicode_whitespace_is_refused_and_writes_nothing() {
    let root = outside("flag-unfit");
    let primary = primary_without_project(&root, "repo-a");
    let init = reverse_init_in(&primary, &["--planning-scope", "\u{3000}"]);
    assert_eq!(init.status.code(), Some(1), "{}", stdout(&init));
    assert!(
        stderr(&init).contains("--planning-scope")
            && stderr(&init).contains("a non-whitespace character"),
        "the refusal names the flag and the rule: {}",
        stderr(&init)
    );
    assert!(
        !primary.join(".engineering").exists(),
        "a refused init leaves the repository as it was"
    );
    let _ = std::fs::remove_dir_all(root);
}

// ---------------------------------------------------------------------------------------------
// Adversarial cases against the `planning_scope` derivation (uncommitted, adversary pass).
// ---------------------------------------------------------------------------------------------

/// `git remote set-url --add origin <url>` gives `origin` a second `url`; Git fetches from the
/// first and `git remote get-url origin` answers the first. `git config --get` answers the last.
#[test]
fn adversary_an_origin_with_a_second_url_names_the_scope_by_its_first_url() {
    let root = outside("adv-two-urls");
    let primary = primary_without_project(&root, "repo-a");
    git_in(
        &primary,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/org/first-name.git",
        ],
    );
    git_in(
        &primary,
        &[
            "remote",
            "set-url",
            "--add",
            "origin",
            "https://example.invalid/mirror/second-name.git",
        ],
    );
    let init = reverse_init_in(&primary, &[]);
    assert!(init.status.success(), "{}", stderr(&init));
    let written = written_scope(&primary);
    let _ = std::fs::remove_dir_all(root);
    assert_eq!(
        written, "first-name",
        "the scope is the repository `origin` fetches from (`git remote get-url origin`)"
    );
}

/// `--engineering .engineering`, run from the checkout, is a relative path whose parent is the
/// empty path; the scope must still be derived from the checkout it names.
#[test]
fn adversary_migrate_git_with_a_relative_engineering_flag_derives_the_scope() {
    let root = outside("adv-relative-engineering");
    let primary = primary_with_v1_store(&root, "repo-a");
    let migrated = migrate_in(&primary, &["--engineering", ".engineering"]);
    let out = format!("{}{}", stdout(&migrated), stderr(&migrated));
    let written = migrated.status.success().then(|| written_scope(&primary));
    let _ = std::fs::remove_dir_all(root);
    assert_eq!(
        written.as_deref(),
        Some("repo-a"),
        "`--engineering .engineering` migrates and writes the checkout's name: {out}"
    );
}

/// The story's source is the URL's last *path* segment. A URL with no path has none, and a query
/// string is not part of the path; neither may become the scope.
#[test]
fn adversary_an_origin_url_names_its_last_path_segment_not_a_host_port_or_query() {
    let root = outside("adv-url-forms");
    let mut wrong = Vec::new();
    for (index, (url, expected)) in [
        (
            "https://example.invalid/org/name.git?ref=main",
            Some("name"),
        ),
        ("https://example.invalid", None),
        ("ssh://git@example.invalid:2222", None),
    ]
    .into_iter()
    .enumerate()
    {
        let primary = primary_without_project(&root, &format!("repo-{index}"));
        git_in(&primary, &["remote", "add", "origin", url]);
        let init = reverse_init_in(&primary, &[]);
        let got = init.status.success().then(|| written_scope(&primary));
        if got.as_deref() != expected {
            wrong.push(format!("{url:?}: expected {expected:?}, wrote {got:?}"));
        }
    }
    let _ = std::fs::remove_dir_all(root);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// `url.<base>.insteadOf` rewrites the origin Git actually fetches from; the scope is the
/// repository that URL names, not the alias.
#[test]
fn adversary_an_origin_alias_rewritten_by_insteadof_names_the_effective_repository() {
    let root = outside("adv-insteadof");
    let primary = primary_without_project(&root, "repo-a");
    git_in(
        &primary,
        &[
            "config",
            "url.https://example.invalid/org/real-name.git.insteadOf",
            "upstream-alias",
        ],
    );
    git_in(&primary, &["remote", "add", "origin", "upstream-alias"]);
    let init = reverse_init_in(&primary, &[]);
    assert!(init.status.success(), "{}", stderr(&init));
    let written = written_scope(&primary);
    let _ = std::fs::remove_dir_all(root);
    assert_eq!(
        written, "real-name",
        "the effective origin URL names the scope"
    );
}

/// When `git` cannot be run, no `origin` is observed and the next source decides: the primary
/// checkout's directory name (the specification's `origin_repository_name`, absent when `git`
/// cannot be run).
#[test]
fn without_git_no_origin_is_observed_and_the_primary_checkout_names_the_scope() {
    let root = outside("adv-no-git");
    let primary = primary_without_project(&root, "repo-a");
    git_in(
        &primary,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/org/service-name.git",
        ],
    );
    let worktree = linked(&primary, "repo-a-wt-nogit");
    let empty_path = root.join("empty-path");
    std::fs::create_dir_all(&empty_path).expect("the directory is writable");
    let tree = relative(&worktree.join(".engineering"), &repository());
    let init = run_with(
        env!("CARGO_BIN_EXE_aep"),
        &worktree,
        &[
            "plan",
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
        &[("PATH", empty_path.to_str().expect("a printable path"))],
    );
    let written = init.status.success().then(|| written_scope(&worktree));
    let out = format!("{}{}", stdout(&init), stderr(&init));
    let _ = std::fs::remove_dir_all(root);
    assert_eq!(
        written.as_deref(),
        Some("repo-a"),
        "without `git` the primary checkout's name is written, never the worktree's: {out}"
    );
}

/// `reverse init` in `directory` with extra environment, as a caller (a hook, a wrapper) runs it.
fn adversary_reverse_init_with_env(directory: &Path, env: &[(&str, &str)]) -> Output {
    let tree = relative(&directory.join(".engineering"), &repository());
    run_with(
        env!("CARGO_BIN_EXE_aep"),
        directory,
        &[
            "plan",
            "reverse",
            "init",
            "--protocols",
            &tree,
            "--profile",
            "development.standard",
        ],
        env,
    )
}

/// `origin_url` documents that `--git-dir` makes the answer the repository the common directory
/// was found for. `git config` also honours `GIT_CONFIG` (read that file instead of the
/// repository's) and `GIT_COMMON_DIR` (read another repository's config); an inherited value of
/// either changes which `origin` is read, or hides it, so the scope written is not this
/// repository's.
#[test]
fn adversary_an_inherited_git_config_or_common_dir_does_not_change_the_origin_read() {
    let root = outside("adv-env-leak");
    let other = primary_without_project(&root, "other");
    git_in(
        &other,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/org/wrong.git",
        ],
    );
    let empty_config = root.join("empty.cfg");
    std::fs::write(&empty_config, "").expect("the file is writable");
    let other_common = other.join(".git");
    let mut wrong = Vec::new();
    for (index, (key, value)) in [
        (
            "GIT_CONFIG",
            empty_config.to_str().expect("a printable path"),
        ),
        (
            "GIT_COMMON_DIR",
            other_common.to_str().expect("a printable path"),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let primary = primary_without_project(&root, &format!("repo-{index}"));
        git_in(
            &primary,
            &[
                "remote",
                "add",
                "origin",
                "https://example.invalid/org/right.git",
            ],
        );
        let init = adversary_reverse_init_with_env(&primary, &[(key, value)]);
        let got = init.status.success().then(|| written_scope(&primary));
        if got.as_deref() != Some("right") {
            wrong.push(format!(
                "{key} inherited: wrote {got:?}: {}{}",
                stdout(&init),
                stderr(&init)
            ));
        }
    }
    let _ = std::fs::remove_dir_all(root);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A remote URL that is a relative path of only dot segments (`git remote add origin ..`) names
/// no repository: its last segment is a path step, not a name. Writing `..` as the scope is the
/// checkout-directory defect in another form; the story's rule is "the last path segment ...
/// without `.git`", which the specification calls the repository's name.
#[test]
fn adversary_an_origin_of_only_dot_segments_is_not_written_as_the_scope() {
    let root = outside("adv-dot-origin");
    let mut wrong = Vec::new();
    for (index, url) in ["..", ".", "../.."].into_iter().enumerate() {
        let primary = primary_without_project(&root, &format!("repo-{index}"));
        git_in(&primary, &["remote", "add", "origin", url]);
        let init = reverse_init_in(&primary, &[]);
        let got = init.status.success().then(|| written_scope(&primary));
        if matches!(got.as_deref(), Some("." | "..")) {
            wrong.push(format!("origin {url:?} wrote {got:?}"));
        }
    }
    let _ = std::fs::remove_dir_all(root);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// A move written once (story:migration-writes-each-move-once): a `/1` journal holding a move line
// twice migrates it once, and `migrate git` on a `/5` store drops a transition repeating the one
// immediately before it.
// ---------------------------------------------------------------------------------------------

/// The transition a consumer store's `epic:acyclic-latest-upgrades` carried twice after its
/// migration, as its front matter quoted it.
const QUOTED_MOVE: &str = "- {from: \"draft\", to: \"proposed\", at: \"2026-09-07T19:51:43Z\", \
                           actor: \"human:operator\", revision: 4, imported: true}\n";

/// The epic in that shape: `validate` refuses it, and no verb but the repair removes the line.
fn repeated_epic() -> String {
    format!(
        "---\nformat: aep.planning-md/3\nid: epic:acyclic-latest-upgrades\nkind: epic\n\
         status: proposed\ntitle: Acyclic latest upgrades\nrevision: 4\ntransitions:\n\
         {QUOTED_MOVE}{QUOTED_MOVE}---\n# Acyclic latest upgrades\n"
    )
}

/// A committed `aep.project/5` project named `name`, written by `reverse init`, holding a story
/// moved once and, when `with_repeat`, the epic repeating its transition.
fn v5_project(name: &str, with_repeat: bool) -> PathBuf {
    let project = scratch(name, true);
    std::fs::remove_dir_all(project.join(".engineering")).expect("the scratch is removable");
    let init = reverse_init_in(&project, &[]);
    assert!(init.status.success(), "{}", stderr(&init));
    let planning = project.join(".engineering/planning");
    std::fs::create_dir_all(planning.join("story")).expect("the store is writable");
    std::fs::write(
        planning.join("story/steady.md"),
        "---\nformat: aep.planning-md/3\nid: story:steady\nkind: story\nstatus: proposed\n\
         title: Steady\nrevision: 2\ntransitions:\n\
         - {from: \"draft\", to: \"proposed\", at: \"2026-09-07T19:00:00Z\", actor: \
         \"human:operator\", revision: 2}\n---\n# Steady\n",
    )
    .expect("the document is writable");
    if with_repeat {
        std::fs::create_dir_all(planning.join("epic")).expect("the store is writable");
        std::fs::write(
            planning.join("epic/acyclic-latest-upgrades.md"),
            repeated_epic(),
        )
        .expect("the document is writable");
    }
    commit_all(&project);
    project
}

#[test]
fn a_v5_document_repeating_its_transition_is_repaired_by_migrate_git_dropping_that_line_only() {
    let project = v5_project("repair", true);
    let epic = project.join(".engineering/planning/epic/acyclic-latest-upgrades.md");

    // The defect as the consumer store had it.
    let refused = aep(&project, &["plan", "artifact", "validate"]);
    assert_eq!(refused.status.code(), Some(1), "{}", stdout(&refused));
    for problem in [
        "epic:acyclic-latest-upgrades: transition 2 moves from `draft`, and the walk before it \
         stands at `proposed`",
        "epic:acyclic-latest-upgrades: transition 2 records revision 4, which is not above the \
         one before it (4)",
    ] {
        assert!(
            stdout(&refused).contains(problem),
            "the fixture reproduces the refusal: {}",
            stdout(&refused)
        );
    }

    let before = contents(&project);
    let dry = migrate_in(&project, &["--dry-run"]);
    assert!(dry.status.success(), "{}{}", stdout(&dry), stderr(&dry));
    for line in [
        "would drop 1 repeated transition(s)",
        "epic:acyclic-latest-upgrades: 1 repeated transition(s)",
        "dry run: nothing was written",
    ] {
        assert!(stdout(&dry).contains(line), "{}", stdout(&dry));
    }
    assert_eq!(before, contents(&project), "a dry run changed a file");

    let repaired = migrate_in(&project, &["--verify"]);
    assert!(
        repaired.status.success(),
        "{}{}",
        stdout(&repaired),
        stderr(&repaired)
    );
    assert!(
        stdout(&repaired).contains("verified 1 repaired document(s)"),
        "{}",
        stdout(&repaired)
    );
    assert_eq!(
        std::fs::read_to_string(&epic).expect("the document reads"),
        repeated_epic().replacen(QUOTED_MOVE, "", 1),
        "exactly the repeated line is gone"
    );
    let after = contents(&project);
    let changed: Vec<_> = after
        .iter()
        .filter(|entry| !before.contains(entry))
        .map(|(path, _)| path.clone())
        .collect();
    assert_eq!(
        changed,
        [".engineering/planning/epic/acyclic-latest-upgrades.md"],
        "the repair writes nothing else"
    );
    assert_eq!(
        before.len(),
        after.len(),
        "the repair adds and removes no file"
    );
    ok(&project, &["validate"]);
}

#[test]
fn a_v5_store_without_a_repeated_transition_is_left_as_it_was_and_says_so() {
    let project = v5_project("nothing-to-repair", false);
    let before = contents(&project);
    for flags in [&[][..], &["--dry-run"][..], &["--verify"][..]] {
        let output = migrate_in(&project, flags);
        assert!(
            output.status.success(),
            "{flags:?}: {}{}",
            stdout(&output),
            stderr(&output)
        );
        assert!(
            stdout(&output).contains("no transition repeats the one before it")
                && stdout(&output).contains("nothing was written"),
            "{flags:?}: {}",
            stdout(&output)
        );
        assert_eq!(before, contents(&project), "{flags:?} changed a file");
    }
}

#[test]
fn a_dirty_v5_store_is_refused_and_left_as_it_was() {
    let project = v5_project("dirty-v5", true);
    std::fs::write(
        project.join(".engineering/planning/story/uncommitted.md"),
        "---\nformat: aep.planning-md/3\nid: story:uncommitted\nkind: story\nstatus: draft\n\
         title: Uncommitted\nrevision: 1\n---\n# Uncommitted\n",
    )
    .expect("the document is writable");
    let before = contents(&project);
    let output = migrate_in(&project, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("uncommitted changes"),
        "{}",
        stderr(&output)
    );
    assert_eq!(before, contents(&project), "a refusal writes nothing");
}

#[test]
fn a_journal_holding_one_move_line_twice_migrates_it_once_and_verifies() {
    let project = v1_project("move-twice");
    let planning = project.join(".engineering/planning");
    std::fs::create_dir_all(planning.join("story")).expect("the store is writable");
    for name in ["entry-shape", "event-shape"] {
        std::fs::write(
            planning.join(format!("story/{name}.md")),
            story(name, "proposed", 3),
        )
        .expect("the document is writable");
    }
    let moved = serde_json::json!({"change": "moved", "from": "draft", "to": "proposed"});
    let entry = entry_line(MOVED_AT, "story:entry-shape", 2, &moved);
    let event = event_line(MOVED_AT, "event-shape", 2, &moved);
    // Each line twice, byte for byte, as a merge of two branches that both carried it leaves it.
    let lines = [entry.clone(), entry, event.clone(), event];
    std::fs::write(planning.join("journal.jsonl"), lines.join("\n") + "\n")
        .expect("the journal is writable");
    commit_all(&project);

    let dry = migrate_in(&project, &["--dry-run"]);
    assert!(dry.status.success(), "{}{}", stdout(&dry), stderr(&dry));
    assert!(
        stdout(&dry).contains("2 repeated move entries dropped"),
        "the dry run counts what it drops: {}",
        stdout(&dry)
    );
    assert!(
        stdout(&dry).contains("carrying 2 transition(s)"),
        "{}",
        stdout(&dry)
    );

    let migrated = migrate_in(&project, &["--verify"]);
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
    for name in ["entry-shape", "event-shape"] {
        let written = std::fs::read_to_string(planning.join(format!("story/{name}.md")))
            .expect("the document reads");
        assert_eq!(
            written.matches("imported: true").count(),
            1,
            "the move journalled twice is one transition: {written}"
        );
    }
    ok(&project, &["validate"]);
}

/// The instant of the move journalled twice.
const MOVED_AT: &str = "2026-09-07T19:51:43Z";
