//! `--task` chooses the task only: it does not turn off project discovery.
//!
//! Run inside a project, `aep govern resolve --task <task>` (and every command sharing its
//! execution flags) reads the documents the project's `protocols` source names. `--root` still
//! names the tree outright, `--artifacts` still replaces the project's manifest, and outside any
//! project the working directory is still the tree.
//!
//! Each test builds its fixture under Cargo's per-target scratch directory: a project whose
//! `.engineering/project.yaml` points at a protocol tree beside it, never inside it, so a run that
//! reads the working directory instead of the project's source finds no protocol at all.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root, whose protocol documents the fixture tree is copied from.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// The directories a document tree is read from, as `aep-project`'s loader walks them.
const TREE: &[&str] = &[
    "protocols",
    "principles",
    "workflows",
    "profiles",
    "artifacts/lifecycles",
    "drivers",
];

/// An empty scratch directory for one test, reclaimed from any earlier run.
///
/// The process id keeps two concurrent runs of this suite out of each other's fixtures.
fn fixture(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("govern-resolve-task-in-project")
        .join(format!("{name}-{}", std::process::id()));
    remove(&directory);
    std::fs::create_dir_all(&directory).expect("the scratch directory is writable");
    directory
}

/// Removes a fixture, including a sealed (read-only) protocol snapshot inside it.
fn remove(directory: &Path) {
    if directory.exists() {
        make_tree_writable(directory);
        std::fs::remove_dir_all(directory).expect("the scratch directory is removable");
    }
}

/// Restores write permission recursively, so a sealed cache snapshot can be reclaimed.
fn make_tree_writable(path: &Path) {
    let mut permissions = std::fs::symlink_metadata(path)
        .expect("scratch path metadata")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        permissions.set_mode(if path.is_dir() { 0o755 } else { 0o644 });
    }
    #[cfg(not(unix))]
    permissions.set_readonly(false);
    std::fs::set_permissions(path, permissions).expect("scratch permissions can be restored");
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("the scratch tree is readable") {
            make_tree_writable(&entry.expect("a scratch entry").path());
        }
    }
}

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the scratch tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// Copies a directory recursively.
fn copy_directory(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the scratch tree is writable");
    for entry in std::fs::read_dir(from).expect("the source tree is readable") {
        let entry = entry.expect("a source entry");
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if source.is_dir() {
            copy_directory(&source, &destination);
        } else {
            std::fs::copy(&source, &destination).expect("the source document is copied");
        }
    }
}

/// A complete protocol tree at `at`: the repository's own documents, copied.
fn protocol_tree(at: &Path) {
    let repository = repository();
    for directory in TREE {
        copy_directory(&repository.join(directory), &at.join(directory));
    }
}

/// A project at `at` whose `protocols` source is `protocols`, with a task of its own.
///
/// The project's task has a different id from [`flag_task`]'s, so an assertion on the resolved
/// task id says which of the two was read.
fn project(at: &Path, protocols: &str) {
    write(
        &at.join(".engineering/project.yaml"),
        &format!(
            "version: aep.project/5\nplanning_scope: fixture\nprotocol: adp/1\n\
             profile: development.standard\nprotocols: '{protocols}'\n"
        ),
    );
    write(
        &at.join(".engineering/task.yaml"),
        "id: PROJECT-OWN-1\nkind: feature\nobjective: the task the project names\n\
         protocol: adp/1\nprofile: development.standard\n",
    );
}

/// A task document outside every project, as `--task` names one.
fn flag_task(at: &Path) -> PathBuf {
    let path = at.join("tasks/flag-task.yaml");
    write(
        &path,
        "id: FLAG-1\nkind: feature\nobjective: the task the flag names\nprotocol: adp/1\n\
         profile: development.standard\n",
    );
    path
}

/// Runs `aep` in `directory`, with the protocol cache inside the fixture and the project
/// directory at its default name.
fn aep_in(directory: &Path, cache: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .args(args)
        .current_dir(directory)
        .env("AEP_CACHE_DIR", cache)
        .env_remove("AEP_PROJECT_DIR")
        .output()
        .expect("the protocol binary runs")
}

/// Runs Git for a source fixture and returns its standard output.
fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {}", stderr(&output));
    stdout(&output).trim().to_owned()
}

/// Standard output as a string.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Standard error as a string.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The exit code, which is part of the contract with a calling harness.
fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// `govern resolve --format json` output, parsed, after asserting the command succeeded.
fn resolved(output: &Output) -> serde_json::Value {
    assert_eq!(
        code(output),
        0,
        "stdout: {}\nstderr: {}",
        stdout(output),
        stderr(output)
    );
    serde_json::from_str(&stdout(output)).expect("resolve --format json prints one JSON document")
}

/// Asserts a resolved plan is the flag's task under `adp/1`.
fn assert_flag_task_resolved(plan: &serde_json::Value) {
    assert_eq!(
        plan["task"]["id"], "FLAG-1",
        "--task chooses the task, not the project's own: {plan}"
    );
    assert_eq!(plan["protocol"]["id"], "adp", "{plan}");
    assert_eq!(plan["protocol"]["version"], 1, "{plan}");
}

#[test]
fn a_task_flag_inside_a_project_resolves_against_the_projects_sibling_protocol_tree() {
    let fixture = fixture("sibling-tree");
    protocol_tree(&fixture.join("protocols"));
    let project_dir = fixture.join("project");
    // Relative to `.engineering`, as a committed project file must be.
    project(&project_dir, "../../protocols");
    let task = flag_task(&fixture);

    let output = aep_in(
        &project_dir,
        &fixture.join("cache"),
        &[
            "govern",
            "resolve",
            "--task",
            printable(&task),
            "--format",
            "json",
        ],
    );

    assert_flag_task_resolved(&resolved(&output));
    remove(&fixture);
}

#[test]
fn a_task_flag_inside_a_project_resolves_against_its_pinned_git_protocol_source() {
    let fixture = fixture("pinned-git");
    let remote = fixture.join("remote");
    protocol_tree(&remote);
    git(&remote, &["init", "--quiet"]);
    git(&remote, &["config", "user.name", "Protocol Test"]);
    git(
        &remote,
        &["config", "user.email", "protocol-test@example.invalid"],
    );
    git(&remote, &["config", "commit.gpgsign", "false"]);
    git(&remote, &["add", "."]);
    git(
        &remote,
        &["commit", "--quiet", "--no-verify", "-m", "protocol tree"],
    );
    let revision = git(&remote, &["rev-parse", "HEAD"]);
    let project_dir = fixture.join("project");
    project(
        &project_dir,
        &format!("git+file://{}#{revision}", remote.display()),
    );
    let task = flag_task(&fixture);

    let output = aep_in(
        &project_dir,
        &fixture.join("cache"),
        &[
            "govern",
            "resolve",
            "--task",
            printable(&task),
            "--format",
            "json",
        ],
    );

    assert_flag_task_resolved(&resolved(&output));
    remove(&fixture);
}

#[test]
fn an_explicit_root_still_wins_over_the_projects_protocol_source() {
    let fixture = fixture("explicit-root");
    protocol_tree(&fixture.join("protocols"));
    let project_dir = fixture.join("project");
    project(&project_dir, "../../protocols");
    let task = flag_task(&fixture);
    let cache = fixture.join("cache");

    // The project's source would resolve this task, so a refusal here can only mean `--root` was
    // the tree read.
    let empty = fixture.join("empty-tree");
    std::fs::create_dir_all(&empty).expect("the scratch tree is writable");
    let refused = aep_in(
        &project_dir,
        &cache,
        &[
            "govern",
            "resolve",
            "--task",
            printable(&task),
            "--root",
            printable(&empty),
            "--format",
            "json",
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("[unknown_protocol]"),
        "an empty --root must be what was read, not the project's tree: {}",
        stderr(&refused)
    );

    let other = fixture.join("other-tree");
    protocol_tree(&other);
    let output = aep_in(
        &project_dir,
        &cache,
        &[
            "govern",
            "resolve",
            "--task",
            printable(&task),
            "--root",
            printable(&other),
            "--format",
            "json",
        ],
    );
    assert_flag_task_resolved(&resolved(&output));
    remove(&fixture);
}

#[test]
fn outside_any_project_a_task_flag_still_resolves_against_the_working_directory() {
    let fixture = fixture("no-project");
    // Discovery climbs at most twelve directories. Twelve levels below the fixture, the climb never
    // reaches the repository this suite runs in, whose own `.engineering` would otherwise be found.
    let mut working = fixture.clone();
    for level in 1..=12 {
        working = working.join(level.to_string());
    }
    protocol_tree(&working);
    let task = flag_task(&fixture);

    let output = aep_in(
        &working,
        &fixture.join("cache"),
        &[
            "govern",
            "resolve",
            "--task",
            printable(&task),
            "--format",
            "json",
        ],
    );

    assert_flag_task_resolved(&resolved(&output));
    remove(&fixture);
}

#[test]
fn a_task_flag_inside_a_project_reads_the_projects_artifacts_unless_artifacts_replaces_them() {
    let fixture = fixture("artifacts");
    protocol_tree(&fixture.join("protocols"));
    let project_dir = fixture.join("project");
    project(&project_dir, "../../protocols");
    let example = repository().join("examples/development-passkeys");
    // The example's manifest holds the approved specification that, with a failing test, carries
    // the task to `implement`; without it the same evidence stops at `specify`.
    std::fs::copy(
        example.join("artifacts.yaml"),
        project_dir.join(".engineering/artifacts.yaml"),
    )
    .expect("the manifest is copied");
    let task = fixture.join("tasks/passkeys.yaml");
    write(
        &task,
        &std::fs::read_to_string(example.join("task.yaml")).expect("the example task is readable"),
    );
    let evidence = example.join("evidence/01-red-test.yaml");
    let empty = fixture.join("empty-artifacts.yaml");
    write(&empty, "version: aep.artifacts/1\nartifacts: []\n");
    let cache = fixture.join("cache");

    let evaluate = |extra: &[&str]| {
        let mut args = vec![
            "govern",
            "evaluate",
            "--task",
            printable(&task),
            "--evidence",
            printable(&evidence),
            "--advance",
            "--format",
            "json",
        ];
        args.extend_from_slice(extra);
        let output = aep_in(&project_dir, &cache, &args);
        assert_eq!(
            code(&output),
            0,
            "stdout: {}\nstderr: {}",
            stdout(&output),
            stderr(&output)
        );
        let evaluation: serde_json::Value =
            serde_json::from_str(&stdout(&output)).expect("evaluate --format json prints JSON");
        evaluation["state"].clone()
    };

    assert_eq!(
        evaluate(&[]),
        "implement",
        "the project's manifest is read when --task alone is given"
    );
    assert_eq!(
        evaluate(&["--artifacts", printable(&empty)]),
        "specify",
        "--artifacts replaces the project's manifest"
    );
    remove(&fixture);
}
