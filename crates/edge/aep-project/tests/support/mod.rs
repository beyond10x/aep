//! Shared by the protocol source integration tests: a pinned Git fixture, and a child process
//! that resolves a source under an environment the test chooses.
//!
//! Each resolution runs in a process of its own. The variables it reads are process-global and
//! the test harness runs tests on parallel threads, so they are set on the child only, never in
//! the test process; and a fresh process has no memory of snapshots it already verified, which is
//! what a later `aep` invocation is.

// Each test binary compiles this module and uses part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Carries the locator a child resolves; set only on a child.
const SOURCE_VARIABLE: &str = "AEP_TEST_PROTOCOL_SOURCE";

/// Marks the one line of a child's standard output that carries its outcome.
const OUTCOME_PREFIX: &str = "aep-test-outcome: ";

/// Every variable the source resolution reads, removed from a child unless a test sets it.
const RESOLUTION_VARIABLES: [&str; 3] = [
    "AEP_CACHE_DIR",
    "AEP_CACHE_GROUP_WRITABLE",
    "AEP_FETCH_TIMEOUT_SECS",
];

/// What one child's resolution produced.
#[derive(Debug)]
pub enum Outcome {
    /// The snapshot directory it resolved to.
    Resolved(PathBuf),
    /// The refusal, verbatim.
    Refused(String),
}

impl Outcome {
    /// The resolved snapshot, or a panic naming `context` and the refusal.
    pub fn resolved(self, context: &str) -> PathBuf {
        match self {
            Self::Resolved(path) => path,
            Self::Refused(refusal) => panic!("{context}: refused: {refusal}"),
        }
    }

    /// The refusal, or a panic naming `context` and what resolved.
    pub fn refused(self, context: &str) -> String {
        match self {
            Self::Refused(refusal) => refusal,
            Self::Resolved(path) => panic!("{context}: resolved to {}", path.display()),
        }
    }
}

/// The body of each test binary's child entry: resolve the source the parent named, and report.
pub fn child_entry() {
    let locator = std::env::var(SOURCE_VARIABLE)
        .expect("this entry runs only as a child of `resolve_in_child`");
    let source = aep_domain::project::ProtocolSource::parse(locator).expect("a valid locator");
    let outcome = match aep_project::project::resolve_protocols(&source, Path::new(".")) {
        Ok(path) => serde_json::json!({ "resolved": path }),
        Err(refusal) => serde_json::json!({ "refused": refusal }),
    };
    println!("{OUTCOME_PREFIX}{outcome}");
}

/// Runs this test binary's `entry` as a child that resolves `locator` with exactly the resolution
/// variables in `environment` set.
pub fn resolve_in_child(entry: &str, locator: &str, environment: &[(&str, &str)]) -> Outcome {
    let mut command =
        Command::new(std::env::current_exe().expect("the test binary can name itself"));
    command.args([
        entry,
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads",
        "1",
    ]);
    for variable in RESOLUTION_VARIABLES {
        command.env_remove(variable);
    }
    let output = command
        .env(SOURCE_VARIABLE, locator)
        .envs(environment.iter().copied())
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find_map(|line| line.split_once(OUTCOME_PREFIX).map(|(_, outcome)| outcome))
        .unwrap_or_else(|| {
            panic!(
                "the child reported no outcome ({}):\n{stdout}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )
        });
    let outcome: serde_json::Value = serde_json::from_str(line).expect("one JSON outcome");
    if let Some(path) = outcome["resolved"].as_str() {
        Outcome::Resolved(PathBuf::from(path))
    } else {
        Outcome::Refused(
            outcome["refused"]
                .as_str()
                .expect("an outcome is resolved or refused")
                .to_owned(),
        )
    }
}

/// A fresh scratch directory for one test, emptied of an earlier run's sealed leftovers.
pub fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("aep-project-it-{name}"));
    remove(&root);
    std::fs::create_dir_all(&root).expect("the scratch directory is writable");
    root
}

/// Removes `path`, giving its directories back owner write first so a sealed snapshot goes too.
pub fn remove(path: &Path) {
    unseal(path);
    std::fs::remove_dir_all(path).ok();
}

#[cfg(unix)]
fn unseal(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if !metadata.is_dir() {
        return;
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).ok();
    for entry in std::fs::read_dir(path).into_iter().flatten().flatten() {
        unseal(&entry.path());
    }
}

#[cfg(not(unix))]
fn unseal(_path: &Path) {}

/// The permission bits of `path`, setgid included.
#[cfg(unix)]
pub fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::symlink_metadata(path)
        .expect("the entry exists")
        .permissions()
        .mode()
        & 0o7777
}

/// Adds `files` to every file's mode and `directories` to every directory's, `path` included.
#[cfg(unix)]
pub fn add_mode_bits(path: &Path, files: u32, directories: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    let is_directory = std::fs::symlink_metadata(path)
        .expect("the entry exists")
        .is_dir();
    let added = if is_directory { directories } else { files };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode_of(path) | added))
        .expect("the test owns the entry");
    if is_directory {
        for entry in std::fs::read_dir(path).expect("readable") {
            add_mode_bits(&entry.expect("an entry").path(), files, directories);
        }
    }
}

/// A local repository holding a small protocol tree, pinned at its one commit.
pub struct PinnedRemote {
    /// The repository's work tree.
    pub directory: PathBuf,
    /// Its one commit.
    pub revision: String,
}

impl PinnedRemote {
    /// Creates the repository under `root`.
    pub fn create(root: &Path) -> Self {
        let directory = root.join("remote");
        write(&directory.join("protocol.yaml"), "sealed\n");
        write(&directory.join("profiles/standard.yaml"), "standard\n");
        write(&directory.join("bin/check"), "#!/bin/sh\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(
                directory.join("bin/check"),
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("the fixture is writable");
        }
        git(&directory, &["init", "--quiet"]);
        git(&directory, &["add", "."]);
        git(
            &directory,
            &[
                "-c",
                "user.name=Protocol Test",
                "-c",
                "user.email=protocol-test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
                "commit",
                "--quiet",
                "--no-verify",
                "-m",
                "protocol tree",
            ],
        );
        let revision = git(&directory, &["rev-parse", "HEAD"]);
        Self {
            directory,
            revision,
        }
    }

    /// The `git+file://` locator of this pin.
    pub fn locator(&self) -> String {
        format!("git+file://{}#{}", self.directory.display(), self.revision)
    }
}

pub fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the directory is writable");
    }
    std::fs::write(path, contents).expect("the file is writable");
}

/// Runs Git in `directory` and returns its trimmed standard output.
pub fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}
