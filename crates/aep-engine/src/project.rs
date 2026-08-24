//! Finding and loading a project.
//!
//! An adopting team should be able to type `protocol evaluate` in their repository and get an
//! answer. Everything needed to do that is in `.engineering/`, and this module is what finds it.
//!
//! ```text
//! payments/
//! ├── docs/                      the artifacts themselves
//! └── .engineering/
//!     ├── project.yaml           protocol, profile, where the tree is
//!     ├── artifacts.yaml         what exists and how it relates
//!     ├── task.yaml              what is being worked on
//!     ├── principles/            the team's own rules, if any
//!     └── profiles/
//! ```
//!
//! # Project-local documents win
//!
//! A project may ship principles and profiles of its own, and they are merged **over** the protocol
//! tree's. They are documents in the same format, validated by the same rules — not a second
//! mechanism, and not an escape hatch: a project-local profile still cannot grant a capability the
//! protocol's approval floor keeps behind approval.
//!
//! # The directory's name is not a constant here
//!
//! `.engineering` is the default and nothing more. A repository that already spends its dot
//! directory on something else, or whose team calls this `.workflow`, sets `AEP_PROJECT_DIR` and
//! everything below finds it — see [`project_directory`]. The name is read here, at the edge that
//! touches the filesystem, and not in `aep-domain`: the domain crate reads no environment, no clock
//! and no filesystem, so that what a document means never depends on where it is being read.
//!
//! # Repository sources become ordinary trees
//!
//! A project's `protocols` value may be a filesystem tree or a pinned `git+ssh://`, `git+https://`,
//! or `git+file://` repository. Repository sources are materialized under the operator's cache and
//! verified against their full commit id before the document loader sees them. The registry still
//! consumes one local immutable tree; network and cache behavior remain at this filesystem edge.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use aep_domain::artifact::ArtifactGraph;
use aep_domain::project::{
    GitProtocolSource, ProjectConfig, ProjectPaths, ProtocolSource, PROJECT_DIRECTORY, PROJECT_FILE,
};
use aep_domain::task::Task;
use sha2::{Digest, Sha256};

use crate::load::{load_tree_report, LoadErrors, LoadFailure, LoadOutcome};
use crate::registry::Registry;

/// How far up the tree to look for a project before giving up.
///
/// Deep enough for a monorepo, shallow enough that a stray `.engineering` in a home directory does
/// not silently govern unrelated work.
const MAX_ASCENT: usize = 12;

/// The environment variable that renames the project directory.
pub const PROJECT_DIRECTORY_ENV: &str = "AEP_PROJECT_DIR";

/// Overrides the directory where immutable protocol repositories are materialized.
pub const CACHE_DIRECTORY_ENV: &str = "AEP_CACHE_DIR";

/// The directory a project keeps its metadata in: `.engineering`, or whatever `AEP_PROJECT_DIR`
/// says.
///
/// **Read once per process.** Discovery walks up to twelve directories and the loader consults this
/// several times more; a value that could change between two of those reads would give one run two
/// different projects, and the failure would look like a filesystem race rather than an edited
/// environment. So the first call decides and every later one agrees.
///
/// An empty or blank value is treated as absent, because `AEP_PROJECT_DIR=` in a shell profile is
/// how a variable gets unset by hand and reading it as "the project lives in the current directory"
/// would be a surprising way to find that out.
pub fn project_directory() -> &'static str {
    static DIRECTORY: OnceLock<String> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            std::env::var(PROJECT_DIRECTORY_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| PROJECT_DIRECTORY.to_owned())
        })
        .as_str()
}

/// A loaded project: its configuration, its documents, and what it is working on.
#[derive(Debug)]
pub struct Project {
    /// The repository root — the directory holding `.engineering`.
    pub root: PathBuf,
    /// Where each thing is, resolved to absolute paths.
    pub paths: ProjectPaths,
    /// What the project says about itself.
    pub config: ProjectConfig,
    /// The documents in force: the protocol tree's, with the project's own merged over them.
    pub registry: Registry,
    /// The artifact graph, when the project has a manifest.
    pub artifacts: ArtifactGraph,
    /// The task being worked on, when the project names one.
    pub task: Option<Task>,
}

impl Project {
    /// The project directory: `.engineering`, or what [`project_directory`] resolved to.
    pub fn engineering(&self) -> PathBuf {
        self.root.join(project_directory())
    }

    /// The task, or an explanation of why there is none.
    pub fn require_task(&self) -> Result<&Task, String> {
        self.task.as_ref().ok_or_else(|| {
            format!(
                "this project names no task; write one at {}",
                self.paths.task.display()
            )
        })
    }
}

/// Finds the directory holding `.engineering`, starting at `from` and walking up.
///
/// Returns `None` rather than an error: not being in a project is an ordinary state, and the caller
/// usually has something else to try.
pub fn discover(from: &Path) -> Option<PathBuf> {
    let mut current = if from.is_dir() {
        Some(from.to_path_buf())
    } else {
        from.parent().map(Path::to_path_buf)
    };

    for _ in 0..MAX_ASCENT {
        let candidate = current.as_ref()?;
        if candidate
            .join(project_directory())
            .join(PROJECT_FILE)
            .is_file()
        {
            return Some(candidate.clone());
        }
        current = candidate.parent().map(Path::to_path_buf);
    }
    None
}

/// Loads the project rooted at `root`, reporting everything that is wrong with it.
pub fn load(root: &Path) -> Result<Project, LoadErrors> {
    let outcome = load_report(root);
    match outcome {
        Ok(project) => Ok(project),
        Err(failures) => Err(LoadErrors::from_failures(failures)),
    }
}

/// Loads only the paths declared by a project's configuration.
///
/// Use this when a caller needs to find one project-owned input before it can do its own work. It
/// deliberately does not load the protocol tree, local documents, artifact manifest, or task: a
/// command asking where those things are must not first require all of them to be valid. A declared
/// Git protocol source may be materialized into the cache as part of resolving its path.
pub fn load_paths(root: &Path) -> Result<ProjectPaths, LoadErrors> {
    config_and_paths(root)
        .map(|(_, paths)| paths)
        .map_err(|failure| LoadErrors::from_failures(vec![failure]))
}

/// Loads a project, or returns every failure found.
// One pass over one directory. Splitting it would thread the failure list through five helpers to
// hide the fact that loading a project is, in order: read the config, load the tree, merge the
// project's own documents, check the pairing, read the manifest, read the task.
#[allow(clippy::too_many_lines)]
fn load_report(root: &Path) -> Result<Project, Vec<LoadFailure>> {
    let (config, paths) = config_and_paths(root).map_err(|failure| vec![failure])?;
    let mut failures: Vec<LoadFailure> = Vec::new();

    // The protocol tree first, then the project's own documents over it.
    let LoadOutcome {
        mut registry,
        failures: tree_failures,
        ..
    } = load_tree_report(&paths.protocols);
    failures.extend(tree_failures);

    for (directory, kind) in [
        (
            &paths.principles,
            aep_schema::parse::DocumentKind::Principle,
        ),
        (&paths.profiles, aep_schema::parse::DocumentKind::Profile),
    ] {
        if !directory.is_dir() {
            continue;
        }
        if let Err(local) = merge_local(&mut registry, directory, kind) {
            failures.extend(local);
        }
    }

    // Re-check the whole set: a project-local profile is checked against the protocol tree it
    // extends, which is the only place that pairing exists.
    for error in registry.validate() {
        failures.push(LoadFailure {
            path: None,
            detail: error.to_string(),
        });
    }

    let artifacts = if paths.artifacts.is_file() {
        match std::fs::read_to_string(&paths.artifacts)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                aep_schema::parse::artifact_manifest(
                    &text,
                    Some(&paths.artifacts.display().to_string()),
                )
                .map_err(|error| error.to_string())
            }) {
            Ok(graph) => graph,
            Err(detail) => {
                failures.push(LoadFailure {
                    path: Some(paths.artifacts.clone()),
                    detail,
                });
                ArtifactGraph::new()
            }
        }
    } else {
        ArtifactGraph::new()
    };

    for error in artifacts.validate_lifecycles(registry.lifecycles()) {
        failures.push(LoadFailure {
            path: Some(paths.artifacts.clone()),
            detail: error.to_string(),
        });
    }

    let task = if paths.task.is_file() {
        match std::fs::read_to_string(&paths.task)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                aep_schema::parse::task(&text, Some(&paths.task.display().to_string()))
                    .map_err(|error| error.to_string())
            }) {
            Ok(task) => Some(task),
            Err(detail) => {
                failures.push(LoadFailure {
                    path: Some(paths.task.clone()),
                    detail,
                });
                None
            }
        }
    } else {
        None
    };

    if !failures.is_empty() {
        return Err(failures);
    }

    Ok(Project {
        root: root.to_path_buf(),
        paths,
        config,
        registry,
        artifacts,
        task,
    })
}

/// Reads the one document that says where every other project input lives.
fn config_and_paths(root: &Path) -> Result<(ProjectConfig, ProjectPaths), LoadFailure> {
    let engineering = root.join(project_directory());
    let config_path = engineering.join(PROJECT_FILE);
    let text = std::fs::read_to_string(&config_path).map_err(|error| LoadFailure {
        path: Some(config_path.clone()),
        detail: format!("cannot be read: {error}"),
    })?;
    let config = aep_schema::parse::project(&text, Some(&config_path.display().to_string()))
        .map_err(|error| LoadFailure {
            path: Some(config_path.clone()),
            detail: error.to_string(),
        })?;
    let protocols =
        resolve_protocol_source(&config.protocols, &engineering).map_err(|detail| LoadFailure {
            path: Some(config_path),
            detail,
        })?;
    let paths = config.paths.resolved(&engineering, protocols);
    Ok((config, paths))
}

/// Resolves a local tree immediately or materializes an immutable repository source in the cache.
fn resolve_protocol_source(source: &ProtocolSource, engineering: &Path) -> Result<PathBuf, String> {
    match source {
        ProtocolSource::Path(path) if path.is_absolute() => Ok(path.clone()),
        ProtocolSource::Path(path) => Ok(engineering.join(path)),
        ProtocolSource::Git(source) => materialize_git_source(source),
    }
}

/// Returns a cached checkout of exactly the source's declared commit.
fn materialize_git_source(source: &GitProtocolSource) -> Result<PathBuf, String> {
    let destination = git_cache_path(source)?;
    if destination.exists() {
        verify_cached_revision(&destination, source.revision())?;
        return Ok(destination);
    }

    let parent = destination.parent().ok_or_else(|| {
        format!(
            "the protocol cache path {} has no parent",
            destination.display()
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "creating protocol source cache {}: {error}",
            parent.display()
        )
    })?;
    let temporary = create_temporary_checkout(parent, source.revision())?;

    let materialized = (|| {
        git_at(
            &temporary,
            &["init", "--quiet"],
            "initializing the source cache",
        )?;
        git_at(
            &temporary,
            &["remote", "add", "origin", source.git_url()],
            "recording the protocol repository",
        )?;
        git_at(
            &temporary,
            &[
                "fetch",
                "--quiet",
                "--depth",
                "1",
                "origin",
                source.revision(),
            ],
            "fetching the pinned protocol revision",
        )?;
        git_at(
            &temporary,
            &["checkout", "--quiet", "--detach", "FETCH_HEAD"],
            "checking out the pinned protocol revision",
        )?;
        verify_cached_revision(&temporary, source.revision())
    })();

    if let Err(error) = materialized {
        std::fs::remove_dir_all(&temporary).ok();
        return Err(error);
    }

    if let Err(error) = std::fs::rename(&temporary, &destination) {
        // Another process may have completed the same immutable source while this one fetched it.
        if destination.exists() {
            std::fs::remove_dir_all(&temporary).ok();
            verify_cached_revision(&destination, source.revision())?;
            return Ok(destination);
        }
        std::fs::remove_dir_all(&temporary).ok();
        return Err(format!(
            "installing protocol source cache at {}: {error}",
            destination.display()
        ));
    }

    Ok(destination)
}

/// A source-and-revision-specific cache path that contains no repository credentials or URL text.
fn git_cache_path(source: &GitProtocolSource) -> Result<PathBuf, String> {
    let root = cache_root()?;
    let repository = format!("{:x}", Sha256::digest(source.repository().as_bytes()));
    Ok(root
        .join("protocol-sources")
        .join(repository)
        .join(source.revision()))
}

/// The operator-selected cache root, then the platform cache, then the conventional user cache.
fn cache_root() -> Result<PathBuf, String> {
    if let Some(path) = nonempty_environment_path(CACHE_DIRECTORY_ENV) {
        return Ok(path);
    }
    if let Some(path) = nonempty_environment_path("XDG_CACHE_HOME") {
        return Ok(path.join("engineering-protocols"));
    }
    if let Some(path) = nonempty_environment_path("HOME") {
        return Ok(path.join(".cache/engineering-protocols"));
    }
    Err(format!(
        "a Git protocol source needs a cache; set `{CACHE_DIRECTORY_ENV}` or `XDG_CACHE_HOME`"
    ))
}

fn nonempty_environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Creates one process-owned empty directory without reusing a crashed process's partial checkout.
fn create_temporary_checkout(parent: &Path, revision: &str) -> Result<PathBuf, String> {
    for attempt in 0..100_u8 {
        let candidate = parent.join(format!(".{revision}-{}-{attempt}.tmp", std::process::id()));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "creating temporary protocol checkout {}: {error}",
                    candidate.display()
                ))
            }
        }
    }
    Err(format!(
        "could not reserve a temporary protocol checkout under {}",
        parent.display()
    ))
}

fn verify_cached_revision(directory: &Path, expected: &str) -> Result<(), String> {
    let actual = git_at(
        directory,
        &["rev-parse", "HEAD"],
        "reading the cached protocol revision",
    )?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "protocol source cache {} is at `{actual}`, not declared revision `{expected}`",
            directory.display()
        ))
    }
}

fn git_at(directory: &Path, arguments: &[&str], operation: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes")
        .output()
        .map_err(|error| format!("{operation}: could not run Git: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{operation}: {}", detail.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Merges a directory of project-local documents into `registry`.
fn merge_local(
    registry: &mut Registry,
    directory: &Path,
    kind: aep_schema::parse::DocumentKind,
) -> Result<(), Vec<LoadFailure>> {
    let mut failures = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();

    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(vec![LoadFailure {
                path: Some(directory.to_path_buf()),
                detail: format!("cannot be read: {error}"),
            }])
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_document = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "yaml" | "yml" | "json"));
        if is_document {
            files.push(path);
        }
    }
    files.sort();

    for file in files {
        let text = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) => {
                failures.push(LoadFailure {
                    path: Some(file),
                    detail: format!("cannot be read: {error}"),
                });
                continue;
            }
        };
        let origin = file.display().to_string();
        let outcome = match kind {
            aep_schema::parse::DocumentKind::Principle => {
                aep_schema::parse::principle(&text, Some(&origin))
                    .map_err(|error| error.to_string())
                    .and_then(|document| {
                        registry
                            .insert_principle(document)
                            .map_err(|error| error.to_string())
                    })
            }
            aep_schema::parse::DocumentKind::Profile => {
                aep_schema::parse::profile(&text, Some(&origin))
                    .map_err(|error| error.to_string())
                    .and_then(|document| {
                        registry
                            .insert_profile(document)
                            .map_err(|error| error.to_string())
                    })
            }
            other => Err(format!("{other} documents do not belong here")),
        };
        if let Err(detail) = outcome {
            failures.push(LoadFailure {
                path: Some(file),
                detail,
            });
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a throwaway project tree under the scratch directory.
    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("aep-project-{name}"));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(root.join(project_directory())).expect("the tree is writable");
        root
    }

    /// The repository's own protocol tree, which every fixture project points at.
    fn protocol_tree() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("the workspace root exists")
    }

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the directory is writable");
        }
        std::fs::write(path, contents).expect("the file is writable");
    }

    fn minimal_project(name: &str) -> PathBuf {
        let root = scratch(name);
        write(
            &root.join(".engineering/project.yaml"),
            &format!(
                "protocol: adp/1\nprofile: development.standard\nprotocols: {}\n",
                protocol_tree().display()
            ),
        );
        root
    }

    #[test]
    fn a_project_is_found_from_anywhere_inside_it() {
        let root = minimal_project("discover");
        let nested = root.join("crates/deep/src");
        std::fs::create_dir_all(&nested).expect("writable");

        assert_eq!(discover(&nested), Some(root.clone()));
        assert_eq!(discover(&root), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn without_the_environment_variable_the_directory_is_the_one_it_always_was() {
        // No test in this binary sets `AEP_PROJECT_DIR`, so this is the absent-variable case, and
        // every other test here is the same case exercised end to end. The override's own tree
        // lives in `tests/project_directory_env.rs`, which is a separate process for exactly that
        // reason.
        assert!(std::env::var(PROJECT_DIRECTORY_ENV).is_err());
        assert_eq!(project_directory(), PROJECT_DIRECTORY);
        assert_eq!(project_directory(), ".engineering");
    }

    #[test]
    fn not_being_in_a_project_is_not_an_error() {
        let elsewhere = std::env::temp_dir().join("aep-project-none");
        std::fs::create_dir_all(&elsewhere).expect("writable");
        assert_eq!(discover(&elsewhere), None);
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    #[test]
    fn a_project_loads_the_protocol_tree_it_points_at() {
        let root = minimal_project("load");
        let project = load(&root).expect("the project loads");

        assert_eq!(project.config.profile.to_string(), "development.standard");
        assert!(project.registry.principles().count() >= 20);
        assert!(project.task.is_none(), "this project names no task yet");
        assert!(project.require_task().is_err());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_project_local_profile_is_merged_over_the_protocol_trees() {
        let root = minimal_project("local");
        write(
            &root.join(".engineering/profiles/house.yaml"),
            "id: house.standard\nversion: 1\ntitle: House rules\nprotocol: adp/1\n\
             extends: development.standard\nprinciples: [mutation-testing]\n",
        );
        write(
            &root.join(".engineering/project.yaml"),
            &format!(
                "protocol: adp/1\nprofile: house.standard\nprotocols: {}\n",
                protocol_tree().display()
            ),
        );

        let project = load(&root).expect("the project loads");
        let resolved = project
            .registry
            .resolved_profile(&"house.standard".parse().expect("reference"))
            .expect("the project's own profile resolves against the protocol tree");
        assert!(
            resolved
                .principles
                .iter()
                .any(|principle| principle.id().as_str() == "mutation-testing"),
            "a project may add rules of its own"
        );
        assert!(
            resolved
                .principles
                .iter()
                .any(|principle| principle.id().as_str() == "test-driven"),
            "and inherits the ones it extends"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_broken_project_document_is_reported_with_its_path() {
        let root = minimal_project("broken");
        write(
            &root.join(".engineering/profiles/broken.yaml"),
            "id: house.broken\ntitle: Broken\nprotocol: adp/1\nworkflow: adp/default\n\
             principles: [does-not-exist]\ncompletion:\n  - tests.unit.failed == 0\n",
        );

        let errors = load(&root).expect_err("the profile names a principle nobody wrote");
        let rendered = errors.to_string();
        assert!(rendered.contains("does-not-exist"), "{rendered}");
        assert!(rendered.contains("unknown_principle"), "{rendered}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_project_local_profile_cannot_escape_the_approval_floor() {
        // The point of merging rather than replacing: a project may add rules, not shed them.
        let root = minimal_project("floor");
        write(
            &root.join(".engineering/profiles/reckless.yaml"),
            "id: house.reckless\nversion: 1\ntitle: Reckless\nprotocol: adp/1\n\
             workflow: adp/default\nprinciples: []\ncapabilities:\n  allow: [production.write]\n\
             completion:\n  - evidence.missing == 0\n",
        );
        write(
            &root.join(".engineering/project.yaml"),
            &format!(
                "protocol: adp/1\nprofile: house.reckless\nprotocols: {}\n",
                protocol_tree().display()
            ),
        );

        let project = load(&root).expect("the documents themselves are well formed");
        let task = aep_schema::parse::task(
            "id: T-1\nkind: feature\nobjective: something\nprotocol: adp/1\nprofile: house.reckless\n",
            None,
        )
        .expect("the task parses");

        let errors = crate::resolve(&task, &project.registry)
            .expect_err("the approval floor still applies to a project's own profile");
        assert!(errors.contains(aep_domain::error::ValidationCode::ProductionWriteWithoutApproval));
        std::fs::remove_dir_all(&root).ok();
    }
}
