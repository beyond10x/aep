//! Where the planning writer fence lives inside a Git repository (GitHub issue #37).
//!
//! A fence file in the working tree outlives every writer, shows up in `git status`, is swept into
//! commits and blocks worktree retirement. Inside a repository the fence lives under the Git
//! common directory instead, which every linked worktree of that repository shares, so the fence
//! still serialises two writers to one store wherever they were started from.

#![cfg(unix)]

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};

const TASK: &str = "---\nformat: aep.planning-md/1\nid: task:seeded\nkind: task\nstatus: draft\n\
                    title: Seeded\nrelations: []\nrevision: 1\n---\n# Seeded\n";

/// A fresh, canonical directory under the system temporary directory. The pid keeps concurrent
/// runs from different worktrees apart, because every one of them shares `TMPDIR`.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir()
        .canonicalize()
        .expect("the temporary directory exists")
        .join(format!("aep-fence-location-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// Runs `git` isolated from the caller's configuration: no hooks, no signing, no global identity.
fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Fence Test",
            "-c",
            "user.email=fence-test@example.invalid",
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
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A repository holding one committed planning store, so a linked worktree checks the store out.
fn repository(name: &str) -> PathBuf {
    let root = scratch(name);
    let main = root.join("main");
    std::fs::create_dir_all(main.join(".engineering/planning/task")).expect("store directory");
    std::fs::write(main.join(".engineering/planning/task/seeded.md"), TASK).expect("seed task");
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

/// Every path under `root`, `.git` excluded, whose file name marks it as a planning writer fence.
fn fence_files_in_working_tree(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the working tree is readable") {
            let entry = entry.expect("a directory entry");
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            let path = entry.path();
            if entry.file_type().expect("an entry type").is_dir() {
                pending.push(path);
            } else if name.to_string_lossy().contains("aep-planning-writer") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The fence for an explicit store, computed from Git's own answer for the common directory and
/// the canonical store path, independently of the code under test.
fn expected_store_fence(worktree: &Path, store: &Path) -> PathBuf {
    let common = PathBuf::from(git(
        worktree,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ))
    .canonicalize()
    .expect("the common directory exists");
    let canonical = store.canonicalize().expect("the store exists");
    let digest = Sha256::digest(canonical.as_os_str().as_encoded_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a string cannot fail");
            output
        });
    common
        .join("aep")
        .join(format!("planning-writer-store-{digest}.lock"))
}

#[test]
fn a_completed_explicit_store_write_leaves_no_fence_file_in_the_working_tree() {
    let main = repository("explicit");

    let created = aep(
        &main,
        &[
            "plan",
            "artifact",
            "new",
            "task",
            "a",
            "--title",
            "A",
            "--store",
            ".engineering/planning",
        ],
    );
    assert!(created.status.success(), "{}", said(&created));
    assert!(main.join(".engineering/planning/task/a.md").exists());

    assert_eq!(
        fence_files_in_working_tree(&main),
        Vec::<PathBuf>::new(),
        "a released writer left its fence in the working tree"
    );
    let status = git(&main, &["status", "--porcelain", "--untracked-files=all"]);
    assert_eq!(
        status, "?? .engineering/planning/journal.jsonl\n?? .engineering/planning/task/a.md",
        "the write must add its artifact and its journal record and nothing else"
    );
    let _ = std::fs::remove_dir_all(main.parent().expect("scratch root"));
}

#[test]
fn a_completed_project_write_leaves_no_fence_file_in_the_working_tree() {
    let main = repository("project");
    std::fs::create_dir_all(main.join("protocols")).expect("protocol root");
    std::fs::write(
        main.join(".engineering/project.yaml"),
        "{\"protocol\":\"adp/1\",\"profile\":\"development.standard\",\"protocols\":\"../protocols\"}\n",
    )
    .expect("project selector");

    let created = aep(
        &main,
        &["plan", "artifact", "new", "task", "b", "--title", "B"],
    );
    assert!(created.status.success(), "{}", said(&created));
    assert!(main.join(".engineering/planning/task/b.md").exists());

    assert_eq!(
        fence_files_in_working_tree(&main),
        Vec::<PathBuf>::new(),
        "a released project writer left its fence in the working tree"
    );
    let _ = std::fs::remove_dir_all(main.parent().expect("scratch root"));
}

/// Adds a linked worktree of `main` beside it and returns its path.
fn linked_worktree(main: &Path) -> PathBuf {
    let linked = main.parent().expect("scratch root").join("linked");
    git(
        main,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "linked",
            linked.to_str().expect("a UTF-8 scratch path"),
        ],
    );
    linked
}

/// Starts `aep plan artifact body --from -` on `store` from `directory`. The command pauses on
/// stdin, which it reads only after it has acquired the fence; the function returns once a probe
/// sees `fence` held.
fn paused_writer(directory: &Path, store: &str, fence: &Path) -> Child {
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
            file.unlock().expect("the probe releases its lock");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = paused.kill();
    let exited = paused.wait_with_output().expect("the paused writer exits");
    panic!(
        "the paused writer never held {}; fence files in the working tree: {:?}\n{}",
        fence.display(),
        fence_files_in_working_tree(directory),
        said(&exited)
    );
}

fn set_title(directory: &Path, store: &str, title: &str) -> Output {
    aep(
        directory,
        &[
            "plan",
            "artifact",
            "set",
            "task:seeded",
            "--title",
            title,
            "--store",
            store,
        ],
    )
}

#[test]
fn writers_to_one_store_from_two_linked_worktrees_contend_on_one_fence_under_the_common_directory()
{
    let main = repository("linked");
    let linked = linked_worktree(&main);
    let store = linked.join(".engineering/planning");
    let store_argument = store.to_str().expect("a UTF-8 scratch path");
    let fence = expected_store_fence(&main, &store);
    assert_eq!(
        fence,
        expected_store_fence(&linked, &store),
        "both worktrees must derive one fence for one store"
    );

    // The first writer starts in the linked worktree, on that worktree's checkout of the store, so
    // its fence is found through the `.git` file and `commondir`.
    let mut paused = paused_writer(&linked, store_argument, &fence);

    // The second writer starts in the main worktree and names the same store.
    let racing = set_title(&main, store_argument, "Racing title");
    assert!(
        !racing.status.success(),
        "a second writer entered the store"
    );
    assert!(
        String::from_utf8_lossy(&racing.stderr)
            .contains("another admitted planning writer or migration holds"),
        "{}",
        said(&racing)
    );

    // The main worktree's own copy of the store is a different store and is not fenced by it.
    let independent = aep(
        &main,
        &[
            "plan",
            "artifact",
            "new",
            "task",
            "own",
            "--title",
            "Own",
            "--store",
            ".engineering/planning",
        ],
    );
    assert!(independent.status.success(), "{}", said(&independent));

    paused
        .stdin
        .take()
        .expect("paused standard input")
        .write_all(b"# Replacement body\n")
        .expect("the first writer resumes");
    let completed = paused.wait_with_output().expect("the first writer exits");
    assert!(completed.status.success(), "{}", said(&completed));

    let entered = set_title(&main, store_argument, "Second title");
    assert!(
        entered.status.success(),
        "the released fence still refuses: {}",
        said(&entered)
    );
    assert_eq!(fence_files_in_working_tree(&main), Vec::<PathBuf>::new());
    assert_eq!(fence_files_in_working_tree(&linked), Vec::<PathBuf>::new());
    let _ = std::fs::remove_dir_all(main.parent().expect("scratch root"));
}

/// A `.git` directory Git would not accept — this machine has had one in a home directory, left
/// holding only `info/exclude` — is not a repository. The fence must not land in it: it would
/// capture every store beneath it, and a writer that does not share that mistake would not
/// contend with one that does.
#[test]
fn a_stray_git_directory_that_names_no_repository_does_not_capture_the_fence() {
    let root = scratch("stray-git");
    std::fs::create_dir_all(root.join(".git/info")).expect("stray Git directory");
    std::fs::write(root.join(".git/info/exclude"), "").expect("stray exclude file");
    let store = root.join("project/planning");
    std::fs::create_dir_all(store.join("task")).expect("store directory");
    std::fs::write(store.join("task/seeded.md"), TASK).expect("seed task");

    let created = aep(
        &root,
        &[
            "plan",
            "artifact",
            "new",
            "task",
            "c",
            "--title",
            "C",
            "--store",
            store.to_str().expect("a UTF-8 scratch path"),
        ],
    );
    assert!(created.status.success(), "{}", said(&created));
    assert!(
        !root.join(".git/aep").exists(),
        "the fence was placed in a directory that names no repository"
    );
    let digest = Sha256::digest(store.as_os_str().as_encoded_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a string cannot fail");
            output
        });
    assert!(
        root.join(format!("project/.aep-planning-writer-{digest}.lock"))
            .is_file(),
        "outside a repository the fence stays beside the store"
    );
    let _ = std::fs::remove_dir_all(root);
}
