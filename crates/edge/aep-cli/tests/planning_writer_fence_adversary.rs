//! Adversarial cases against the relocated planning writer fence (GitHub issue #37).
//!
//! Each case drives the built `aep` binary inside a real Git repository built by `git` itself.

#![cfg(unix)]

use std::fmt::Write as _;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};

const TASK: &str = "---\nformat: aep.planning-md/1\nid: task:seeded\nkind: task\nstatus: draft\n\
                    title: Seeded\nrelations: []\nrevision: 1\n---\n# Seeded\n";

const STORY: &str = "---\nformat: aep.planning-md/1\nid: story:seeded\nkind: story\n\
                     status: draft\ntitle: Seeded story\nrelations: []\nrevision: 1\n---\n\
                     # Seeded story\n";

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir()
        .canonicalize()
        .expect("the temporary directory exists")
        .join(format!("aep-fence-adversary-{name}-{}", std::process::id()));
    if directory.exists() {
        let _ = Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(&directory)
            .status();
        let _ = std::fs::remove_dir_all(&directory);
    }
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Fence Adversary",
            "-c",
            "user.email=fence-adversary@example.invalid",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "protocol.file.allow=always",
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
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A repository with a committed project (selector + seeded store).
fn project_repository(root: &Path) -> PathBuf {
    let main = root.join("main");
    std::fs::create_dir_all(main.join(".engineering/planning/task")).expect("store directory");
    std::fs::create_dir_all(main.join(".engineering/planning/story")).expect("store directory");
    std::fs::create_dir_all(main.join("protocols")).expect("protocol root");
    std::fs::write(main.join("protocols/.keep"), "").expect("protocol keep");
    std::fs::write(main.join(".engineering/planning/task/seeded.md"), TASK).expect("seed task");
    std::fs::write(main.join(".engineering/planning/story/seeded.md"), STORY).expect("seed story");
    std::fs::write(
        main.join(".engineering/project.yaml"),
        "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\"}\n",
    )
    .expect("project selector");
    git(&main, &["init", "--quiet"]);
    git(&main, &["add", "."]);
    git(&main, &["commit", "--quiet", "-m", "seed"]);
    main
}

fn aep(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(directory)
        .output()
        .expect("aep runs")
}

fn said(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn digest(path: &Path) -> String {
    Sha256::digest(path.as_os_str().as_encoded_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a string cannot fail");
            output
        })
}

fn hold(path: &Path) -> std::fs::File {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .expect("the fence opens");
    file.try_lock_exclusive().expect("the fence is free");
    file
}

/// An `aep` built before this change (0.59.3 is installed on the workstation) locks
/// `<project>/.aep-planning-writer.lock` and `<project>/.aep-planning-writer-<digest>.lock`. The
/// story's acceptance says relocation "must not open a window in which two writers both believe
/// they hold the fence". Holding the legacy fence in a repository stands in for that binary.
#[test]
fn a_peer_holding_the_legacy_in_tree_project_fence_still_excludes_a_writer_in_a_repository() {
    let root = scratch("legacy-project");
    let main = project_repository(&root);
    let engineering = main.join(".engineering");
    let legacy = hold(&engineering.join(".aep-planning-writer.lock"));

    let racing = aep(
        &main,
        &[
            "plan", "artifact", "new", "task", "racing", "--title", "Racing",
        ],
    );
    let entered = main.join(".engineering/planning/task/racing.md").exists();
    drop(legacy);
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        !racing.status.success() && !entered,
        "a writer entered the store while a legacy-protocol peer held its fence: {}",
        said(&racing)
    );
    assert!(
        String::from_utf8_lossy(&racing.stderr).contains("another admitted planning writer"),
        "{}",
        said(&racing)
    );
}

/// The same window for an explicit `--store` writer: the legacy store fence is
/// `<store parent>/.aep-planning-writer-<sha256 of canonical store>.lock`.
#[test]
fn a_peer_holding_the_legacy_in_tree_store_fence_still_excludes_an_explicit_writer_in_a_repository()
{
    let root = scratch("legacy-store");
    let main = project_repository(&root);
    let store = main.join(".engineering/planning");
    let legacy = hold(&main.join(format!(
        ".engineering/.aep-planning-writer-{}.lock",
        digest(&store)
    )));

    let racing = aep(
        &main,
        &[
            "plan",
            "artifact",
            "set",
            "task:seeded",
            "--title",
            "Racing",
            "--store",
            store.to_str().expect("UTF-8 scratch path"),
        ],
    );
    drop(legacy);
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        !racing.status.success(),
        "an explicit writer entered while a legacy-protocol peer held the store fence: {}",
        said(&racing)
    );
}

/// Every write verb the story names, run one after another in a repository, leaves `git status`
/// (ignored files included) showing nothing but changes inside the store.
#[test]
fn every_write_verb_in_a_repository_changes_nothing_outside_its_store() {
    let root = scratch("verbs");
    let main = project_repository(&root);
    let runs: &[&[&str]] = &[
        &["plan", "artifact", "new", "task", "a", "--title", "A"],
        &[
            "plan",
            "artifact",
            "relate",
            "task:a",
            "depends_on",
            "task:seeded",
        ],
        &["plan", "artifact", "set", "task:a", "--title", "A2"],
        &[
            "plan",
            "artifact",
            "scope",
            "story:seeded",
            "--add",
            "src/lib.rs",
        ],
        &[
            "plan",
            "artifact",
            "evidence",
            "task:a",
            "--kind",
            "test_result",
            "--source",
            "adversary",
        ],
        &["plan", "artifact", "move", "task:a", "--to", "active"],
    ];
    let mut leaked = Vec::new();
    let mut outcomes = Vec::new();
    for args in runs {
        let output = aep(&main, args);
        outcomes.push(format!("{args:?} -> {:?}", output.status.code()));
        let status = git(
            &main,
            &[
                "status",
                "--porcelain",
                "--ignored",
                "--untracked-files=all",
            ],
        );
        for line in status.lines() {
            let path = line.split_whitespace().last().unwrap_or_default();
            if !path.starts_with(".engineering/planning/")
                || Path::new(path)
                    .extension()
                    .is_some_and(|extension| extension == "lock")
            {
                leaked.push(format!("after {args:?}: {line}"));
            }
        }
    }
    // body reads stdin; run it last.
    let mut body = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(["plan", "artifact", "body", "task:a", "--from", "-"])
        .current_dir(&main)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("body starts");
    body.stdin
        .take()
        .expect("stdin")
        .write_all(b"# Replaced\n")
        .expect("body input");
    let body = body.wait_with_output().expect("body exits");
    outcomes.push(format!("body -> {:?}", body.status.code()));
    let status = git(
        &main,
        &[
            "status",
            "--porcelain",
            "--ignored",
            "--untracked-files=all",
        ],
    );
    for line in status.lines() {
        let path = line.split_whitespace().last().unwrap_or_default();
        if !path.starts_with(".engineering/planning/")
            || Path::new(path)
                .extension()
                .is_some_and(|extension| extension == "lock")
        {
            leaked.push(format!("after body: {line}"));
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(leaked, Vec::<String>::new(), "outcomes: {outcomes:#?}");
}

fn paused_body(directory: &Path, store: &str, fence: &Path) -> Child {
    let mut paused = Command::new(env!("CARGO_BIN_EXE_aep"))
        .args([
            "plan",
            "artifact",
            "body",
            "task:seeded",
            "--from",
            "-",
            "--store",
            store,
        ])
        .current_dir(directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the paused writer starts");
    for _ in 0..500 {
        if let Ok(file) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(fence)
        {
            if file.try_lock_exclusive().is_err() {
                return paused;
            }
            file.unlock().expect("probe releases");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = paused.kill();
    let exited = paused.wait_with_output().expect("exits");
    panic!(
        "the paused writer never held {}\n{}",
        fence.display(),
        said(&exited)
    );
}

fn git_common(directory: &Path) -> PathBuf {
    PathBuf::from(git(
        directory,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ))
    .canonicalize()
    .expect("common dir exists")
}

/// A linked worktree made with `--relative-paths` (relative `gitdir:` and `commondir`), its store
/// named through a symlink from outside the repository by one writer and canonically by another:
/// both must derive Git's own common directory and contend.
#[test]
fn a_relative_path_worktree_store_named_through_a_symlink_contends_with_its_canonical_spelling() {
    let root = scratch("relative-symlink");
    let main = project_repository(&root);
    let linked = root.join("linked");
    git(
        &main,
        &[
            "worktree",
            "add",
            "--quiet",
            "--relative-paths",
            "-b",
            "linked",
            linked.to_str().expect("UTF-8"),
        ],
    );
    let gitfile = std::fs::read_to_string(linked.join(".git")).expect("gitfile");
    assert!(gitfile.contains("gitdir: ../"), "not relative: {gitfile}");
    let alias = root.join("alias");
    std::os::unix::fs::symlink(&linked, &alias).expect("symlink");

    let store = linked.join(".engineering/planning");
    let fence = git_common(&linked)
        .join("aep")
        .join(format!("planning-writer-store-{}.lock", digest(&store)));
    let aliased = alias.join(".engineering/planning");
    let mut paused = paused_body(&root, aliased.to_str().expect("UTF-8"), &fence);
    let racing = aep(
        &main,
        &[
            "plan",
            "artifact",
            "set",
            "task:seeded",
            "--title",
            "Racing",
            "--store",
            store.to_str().expect("UTF-8"),
        ],
    );
    paused
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"# Body\n")
        .expect("resume");
    let done = paused.wait_with_output().expect("exits");
    let _ = std::fs::remove_dir_all(&root);
    assert!(done.status.success(), "{}", said(&done));
    assert!(!racing.status.success(), "{}", said(&racing));
}

/// A store inside a submodule: the fence must land in the submodule's own Git directory
/// (`<super>/.git/modules/<name>`), which `git rev-parse --git-common-dir` reports.
#[test]
fn a_store_inside_a_submodule_is_fenced_in_the_submodules_git_directory() {
    let root = scratch("submodule");
    let inner = project_repository(&root);
    let sup = root.join("super");
    std::fs::create_dir_all(&sup).expect("super");
    git(&sup, &["init", "--quiet"]);
    git(
        &sup,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--quiet",
            inner.to_str().expect("UTF-8"),
            "sub",
        ],
    );
    let sub = sup.join("sub");
    let store = sub.join(".engineering/planning");
    let fence = git_common(&sub)
        .join("aep")
        .join(format!("planning-writer-store-{}.lock", digest(&store)));
    let mut paused = paused_body(&sup, store.to_str().expect("UTF-8"), &fence);
    paused
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"# Body\n")
        .expect("resume");
    let done = paused.wait_with_output().expect("exits");
    let _ = std::fs::remove_dir_all(&root);
    assert!(done.status.success(), "{}", said(&done));
}

/// A repository whose Git directory is not writable (a sandbox that protects `.git`, a checkout
/// owned by someone else): the write is refused, the refusal names the fence, and nothing in the
/// store changed (invariant 7).
#[test]
fn a_read_only_git_directory_refuses_the_write_and_changes_nothing() {
    let root = scratch("readonly-git");
    let main = project_repository(&root);
    let dot_git = main.join(".git");
    std::fs::set_permissions(&dot_git, std::fs::Permissions::from_mode(0o555)).expect("chmod");
    let output = aep(
        &main,
        &[
            "plan", "artifact", "new", "task", "blocked", "--title", "Blocked",
        ],
    );
    std::fs::set_permissions(&dot_git, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let status = git(
        &main,
        &[
            "status",
            "--porcelain",
            "--ignored",
            "--untracked-files=all",
        ],
    );
    let _ = std::fs::remove_dir_all(&root);
    assert!(!output.status.success(), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("planning writer fence"),
        "{}",
        said(&output)
    );
    assert_eq!(status, "", "a refused write changed the tree");
}
