//! Finding and loading a project.
//!
//! An adopting team should be able to type `aep govern evaluate` in their repository and get an
//! answer. Everything needed to do that is in `.engineering/`, and this module is what finds it.
//!
//! ```text
//! payments/
//! ├── docs/                      the artifacts themselves
//! └── .engineering/
//!     ├── project.yaml           protocol, profile, where the tree is
//!     ├── artifacts.yaml         what exists and how it relates
//!     ├── task.yaml              what is being worked on
//!     ├── schemas/               project-owned JSON Schema contracts
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
    GitProtocolSource, ProjectConfig, ProjectLocalPaths, ProjectPaths, ProtocolSource,
    PROJECT_DIRECTORY, PROJECT_FILE,
};
use aep_domain::task::Task;
use aep_domain::workspace::{Member, Workspace, WORKSPACE_FILE};
use sha2::{Digest, Sha256};

use crate::load::{load_tree_report, DriverRegistry, LoadErrors, LoadFailure, LoadOutcome};
use aep_engine::registry::Registry;

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
    /// Driver projections acquired beside the semantic documents.
    pub drivers: DriverRegistry,
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

/// Loads only the project-owned relative paths from configuration.
///
/// Unlike [`load_paths`], this does not resolve or materialize the protocol source. Use it for a
/// command that needs only a local input such as the project's schema registry: discovering a
/// directory inside the current repository must not fetch an unrelated governing repository.
pub fn load_local_paths(root: &Path) -> Result<ProjectLocalPaths, LoadErrors> {
    read_config(root)
        .map(|(_, _, config)| config.paths)
        .map_err(|failure| LoadErrors::from_failures(vec![failure]))
}

/// Loads only a project's configuration, resolving nothing it names.
///
/// The lightest of the three loaders. Use it for a question the configuration itself answers — how
/// this project builds a link for `jira:DEV-630` — where loading the protocol tree would make a
/// listing depend on a network fetch it has no other reason to make.
pub fn load_config(root: &Path) -> Result<ProjectConfig, LoadErrors> {
    read_config(root)
        .map(|(_, _, config)| config)
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
        drivers,
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
        drivers,
        artifacts,
        task,
    })
}

/// Reads the one document that says where every other project input lives.
fn config_and_paths(root: &Path) -> Result<(ProjectConfig, ProjectPaths), LoadFailure> {
    let (engineering, config_path, config) = read_config(root)?;
    let protocols =
        resolve_protocol_source(&config.protocols, &engineering).map_err(|detail| LoadFailure {
            path: Some(config_path),
            detail,
        })?;
    let paths = config.paths.resolved(&engineering, protocols);
    Ok((config, paths))
}

/// Reads and validates project configuration without resolving any source it names.
fn read_config(root: &Path) -> Result<(PathBuf, PathBuf, ProjectConfig), LoadFailure> {
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
    Ok((engineering, config_path, config))
}

/// Reads `.engineering/workspace.yaml`, if this repository has one.
///
/// `Ok(None)` when the file is absent, which is the ordinary case: a repository that answers only
/// for itself is not a broken workspace, it is a repository without one. A file that exists and
/// does not validate is an error, because somebody wrote it and meant it.
pub fn load_workspace(root: &Path) -> Result<Option<Workspace>, LoadErrors> {
    let path = root.join(project_directory()).join(WORKSPACE_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(LoadErrors::from_failures(vec![LoadFailure {
                path: Some(path),
                detail: format!("cannot be read: {error}"),
            }]))
        }
    };

    aep_schema::parse::workspace(&text, Some(&path.display().to_string()))
        .map(Some)
        .map_err(|error| {
            LoadErrors::from_failures(vec![LoadFailure {
                path: Some(path),
                detail: error.to_string(),
            }])
        })
}

/// Where a member's tree is on this machine, materializing a pinned Git member if it is not yet.
///
/// The same resolution a project's `protocols:` gets, for the same reason: one spelling for *where
/// a tree comes from* means one set of refusals, already tested.
pub fn resolve_member(member: &Member, engineering: &Path) -> Result<PathBuf, String> {
    resolve_protocol_source(&member.source, engineering)
}

/// Resolves a local tree immediately or materializes an immutable repository source in the cache.
fn resolve_protocol_source(source: &ProtocolSource, engineering: &Path) -> Result<PathBuf, String> {
    match source {
        ProtocolSource::Path(path) if path.is_absolute() => Ok(path.clone()),
        ProtocolSource::Path(path) => Ok(engineering.join(path)),
        ProtocolSource::Git(source) => materialize_git_source(source),
    }
}

/// One file sealed into an immutable source snapshot.
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SnapshotEntry {
    path: String,
    mode: u32,
    length: u64,
    sha256: String,
}

/// The complete membership and bytes of one source snapshot.
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SnapshotManifest {
    format: String,
    revision: String,
    files: Vec<SnapshotEntry>,
}

const SNAPSHOT_MANIFEST: &str = ".aep-source-manifest.json";

/// Returns a verified read-only snapshot of exactly the source's declared commit.
fn materialize_git_source(source: &GitProtocolSource) -> Result<PathBuf, String> {
    refuse_credentials(source.git_url())?;
    let (bare, destination) = git_cache_paths(source)?;
    if destination.exists() {
        verify_snapshot_once(&destination, source.revision())?;
        return Ok(destination);
    }

    let repository_root = bare
        .parent()
        .ok_or_else(|| format!("the bare cache path {} has no parent", bare.display()))?;
    std::fs::create_dir_all(repository_root)
        .map_err(|error| format!("creating {}: {error}", repository_root.display()))?;
    if !bare.exists() {
        git_bare(
            &bare,
            None,
            &["init", "--quiet", "--bare"],
            "initializing the bare source cache",
        )?;
    }
    git_bare(
        &bare,
        None,
        &[
            "fetch",
            "--quiet",
            "--depth",
            "1",
            source.git_url(),
            source.revision(),
        ],
        "fetching the pinned protocol revision",
    )?;
    let actual = git_bare(
        &bare,
        None,
        &["rev-parse", "FETCH_HEAD^{commit}"],
        "verifying the fetched commit object",
    )?;
    if actual != source.revision() {
        return Err(format!(
            "the fetched commit is `{actual}`, not declared revision `{}`",
            source.revision()
        ));
    }

    let parent = destination
        .parent()
        .ok_or_else(|| format!("the snapshot path {} has no parent", destination.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    let temporary = create_temporary_checkout(parent, source.revision())?;
    let materialized = (|| {
        git_bare(
            &bare,
            Some(&temporary),
            &[
                "checkout",
                "--quiet",
                "--force",
                source.revision(),
                "--",
                ".",
            ],
            "archiving the pinned commit into its snapshot",
        )?;
        let manifest = snapshot_manifest(&temporary, source.revision(), true)?;
        let path = temporary.join(SNAPSHOT_MANIFEST);
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("serializing {}: {error}", path.display()))?;
        std::fs::write(&path, bytes)
            .map_err(|error| format!("writing {}: {error}", path.display()))?;
        make_read_only(&path, false)?;
        make_directories_read_only(&temporary)?;
        Ok::<(), String>(())
    })();
    if let Err(error) = materialized {
        std::fs::remove_dir_all(&temporary).ok();
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        if destination.exists() {
            std::fs::remove_dir_all(&temporary).ok();
            verify_snapshot_once(&destination, source.revision())?;
            return Ok(destination);
        }
        std::fs::remove_dir_all(&temporary).ok();
        return Err(format!(
            "publishing snapshot {}: {error}",
            destination.display()
        ));
    }
    verify_snapshot_once(&destination, source.revision())?;
    Ok(destination)
}

/// Bare object cache and source-and-revision snapshot paths containing no URL or credentials.
fn git_cache_paths(source: &GitProtocolSource) -> Result<(PathBuf, PathBuf), String> {
    let root = cache_root()?;
    let repository: String = Sha256::digest(source.repository().as_bytes()).iter().fold(
        String::new(),
        |mut output, byte| {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        },
    );
    let base = root.join("protocol-sources").join(repository);
    Ok((
        base.join("objects.git"),
        base.join("snapshots").join(source.revision()),
    ))
}

/// Refuses URL userinfo that could write a secret into process listings or Git configuration.
fn refuse_credentials(url: &str) -> Result<(), String> {
    let Some((_, rest)) = url.split_once("://") else {
        return Ok(());
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    let Some((userinfo, _)) = authority.rsplit_once('@') else {
        return Ok(());
    };
    if userinfo == "git" && !userinfo.contains(':') {
        return Ok(());
    }
    Err("Git protocol source URLs must not contain credentials or user information".to_owned())
}

/// The operator-selected cache root, then the platform cache, then the conventional user cache.
fn cache_root() -> Result<PathBuf, String> {
    if let Some(path) = nonempty_environment_path(CACHE_DIRECTORY_ENV) {
        return Ok(path);
    }
    if let Some(path) = nonempty_environment_path("XDG_CACHE_HOME") {
        return Ok(path.join("aep"));
    }
    if let Some(path) = nonempty_environment_path("HOME") {
        return Ok(path.join(".cache/aep"));
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

/// Runs Git against the bare object database and, optionally, a detached work tree.
fn git_bare(
    bare: &Path,
    work_tree: Option<&Path>,
    arguments: &[&str],
    operation: &str,
) -> Result<String, String> {
    let mut command = Command::new("git");
    command.arg(format!("--git-dir={}", bare.display()));
    if let Some(work_tree) = work_tree {
        command.arg(format!("--work-tree={}", work_tree.display()));
    }
    let output = Command::new("git")
        .args(command.get_args())
        .args(arguments)
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

/// Snapshots this process has already verified, by directory and revision.
fn verified_snapshots() -> &'static std::sync::Mutex<std::collections::BTreeSet<(PathBuf, String)>>
{
    static VERIFIED: OnceLock<std::sync::Mutex<std::collections::BTreeSet<(PathBuf, String)>>> =
        OnceLock::new();
    VERIFIED.get_or_init(Default::default)
}

/// [`verify_snapshot`], at most once per snapshot per process, and in full at most once per pin.
///
/// One command resolves the protocol source several times (configuration, paths, the tree, the
/// planning store), and each full verification re-reads and re-hashes every file of the pinned
/// revision — 14,669 files and 162 MB for a revision that carries its own planning store. The
/// snapshot is read-only and sealed when it is materialized, so re-verifying it within the same
/// short-lived process only repeats the same reads. A failed verification is not remembered.
///
/// Across processes, a full verification leaves a stamp beside the snapshot (see
/// [`verify_snapshot_stamped`]); a later process whose stamp still matches every file's and
/// directory's metadata trusts the snapshot without reading a file or opening a directory.
fn verify_snapshot_once(directory: &Path, revision: &str) -> Result<(), String> {
    let key = (directory.to_path_buf(), revision.to_owned());
    let verified = verified_snapshots();
    if verified
        .lock()
        .is_ok_and(|verified| verified.contains(&key))
    {
        return Ok(());
    }
    let listing = verify_snapshot_stamped(directory, revision, RACY_WINDOW)?;
    crate::load::remember_sealed_listing(directory, listing);
    if let Ok(mut verified) = verified.lock() {
        verified.insert(key);
    }
    Ok(())
}

/// Format of the stamp a full verification leaves beside a snapshot.
///
/// Format 2 adds every directory's metadata, so the trusted path stats entries instead of
/// listing directories. A format-1 stamp is ignored and replaced after one full verification.
const SNAPSHOT_STAMP_FORMAT: &str = "aep.snapshot-verified/2";

/// Stamp key recording the snapshot directory itself, among the directories.
const SNAPSHOT_DIRECTORY_KEY: &str = ".";

/// A file changed this close to the start of a verification is not stamped (Part B § B3 rule 4).
///
/// Linux file times come from a coarse clock, so a write in the same tick as the stat that
/// follows it can leave `ctime` unchanged. Refusing to stamp while any file is that young means a
/// write racing the verification is always re-hashed by the next process.
const RACY_WINDOW: std::time::Duration = std::time::Duration::from_secs(2);

/// The metadata of one snapshot entry a stamp holds it to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FileStamp {
    len: u64,
    mtime_ns: i64,
    ctime_ns: i64,
    ino: u64,
    dev: u64,
    mode: u32,
}

/// Proof that a full verification of one snapshot succeeded, keyed by what it saw on disk.
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotStamp {
    format: String,
    revision: String,
    manifest_sha256: String,
    verifier_version: String,
    /// Every regular file, the manifest included, by path relative to the snapshot.
    files: std::collections::BTreeMap<String, FileStamp>,
    /// Every directory, the snapshot itself as `.`, by path relative to the snapshot.
    directories: std::collections::BTreeMap<String, FileStamp>,
}

/// The metadata a stamp records: every file and every directory of a snapshot.
struct SnapshotEntryStamps {
    files: std::collections::BTreeMap<String, FileStamp>,
    directories: std::collections::BTreeMap<String, FileStamp>,
}

/// Where a snapshot's stamp lives: beside it, outside the read-only tree.
fn snapshot_stamp_path(directory: &Path) -> Option<PathBuf> {
    let name = directory.file_name()?.to_str()?;
    Some(directory.with_file_name(format!("{name}.verified.json")))
}

/// Trusts a matching stamp, or verifies in full and stamps the result.
///
/// A stamp is trusted only when it parses, names this revision and this verifier version, the
/// manifest's current bytes hash to the digest it recorded, and a no-follow stat of every recorded
/// file and directory finds its length, times, inode, device and mode unchanged. No directory is
/// opened: adding, removing or renaming an entry moves its directory's `mtime` and `ctime`, so
/// unchanged directory metadata proves an unchanged listing, and the racy window keeps a change in
/// the clock tick of the stamp from hiding. Anything else falls back to [`verify_snapshot`],
/// unchanged as the full check. A
/// failed verification writes no stamp; a failure to write one is not an error, it only means the
/// next process verifies in full again.
///
/// Returns the verified snapshot's files, relative and sorted, the manifest excluded — the
/// listing a tree load uses instead of opening the snapshot's directories.
fn verify_snapshot_stamped(
    directory: &Path,
    revision: &str,
    racy_window: std::time::Duration,
) -> Result<Vec<String>, String> {
    if let Some(listing) = snapshot_stamp_listing(directory, revision) {
        return Ok(listing);
    }
    let started = std::time::SystemTime::now();
    let listing = verify_snapshot(directory, revision)?;
    write_snapshot_stamp(directory, revision, started, racy_window).ok();
    Ok(listing)
}

/// The snapshot's files, when the stamp beside `directory` still describes it, reading only the
/// manifest.
///
/// The stamp's file map is the listing: a stamp is written only after the tree matched the sealed
/// manifest, and it is trusted only while every recorded file and directory is unchanged.
fn snapshot_stamp_listing(directory: &Path, revision: &str) -> Option<Vec<String>> {
    let path = snapshot_stamp_path(directory)?;
    let bytes = std::fs::read(&path).ok()?;
    let stamp = serde_json::from_slice::<SnapshotStamp>(&bytes).ok()?;
    if stamp.format != SNAPSHOT_STAMP_FORMAT
        || stamp.revision != revision
        || stamp.verifier_version != env!("CARGO_PKG_VERSION")
    {
        return None;
    }
    let manifest = std::fs::read(directory.join(SNAPSHOT_MANIFEST)).ok()?;
    if hex_sha256(&manifest) != stamp.manifest_sha256 {
        return None;
    }
    if !recorded_entries_unchanged(directory, &stamp) {
        return None;
    }
    Some(
        stamp
            .files
            .into_keys()
            .filter(|relative| relative != SNAPSHOT_MANIFEST)
            .collect(),
    )
}

/// Whether every entry the stamp records still has the metadata it recorded, by stat alone.
fn recorded_entries_unchanged(directory: &Path, stamp: &SnapshotStamp) -> bool {
    if !stamp.directories.contains_key(SNAPSHOT_DIRECTORY_KEY)
        || !stamp.files.contains_key(SNAPSHOT_MANIFEST)
    {
        return false;
    }
    let entries = stamp
        .directories
        .iter()
        .map(|entry| (entry, true))
        .chain(stamp.files.iter().map(|entry| (entry, false)));
    for ((relative, recorded), is_directory) in entries {
        let path = if relative == SNAPSHOT_DIRECTORY_KEY {
            directory.to_path_buf()
        } else {
            let relative = Path::new(relative);
            if !relative
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
            {
                return false;
            }
            directory.join(relative)
        };
        match file_stamp(&path) {
            Ok(current) if current == *recorded && is_kind(&current, is_directory) => {}
            _ => return false,
        }
    }
    true
}

/// Whether a stamped entry is a directory (`directory`) or a regular file.
#[cfg(unix)]
fn is_kind(stamp: &FileStamp, directory: bool) -> bool {
    const TYPE_MASK: u32 = 0o170_000;
    let expected = if directory { 0o040_000 } else { 0o100_000 };
    stamp.mode & TYPE_MASK == expected
}

#[cfg(not(unix))]
fn is_kind(_stamp: &FileStamp, _directory: bool) -> bool {
    false
}

/// Records what a successful full verification saw, unless a file is too young to be trusted.
fn write_snapshot_stamp(
    directory: &Path,
    revision: &str,
    started: std::time::SystemTime,
    racy_window: std::time::Duration,
) -> Result<(), String> {
    let path = snapshot_stamp_path(directory)
        .ok_or_else(|| format!("snapshot {} has no stamp path", directory.display()))?;
    let manifest = std::fs::read(directory.join(SNAPSHOT_MANIFEST))
        .map_err(|error| format!("reading snapshot manifest: {error}"))?;
    let SnapshotEntryStamps { files, directories } = snapshot_entry_stamps(directory)?;
    let cutoff = started
        .checked_sub(racy_window)
        .and_then(|cutoff| cutoff.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|cutoff| i64::try_from(cutoff.as_nanos()).ok())
        .ok_or("the racy cutoff is not representable")?;
    if files
        .values()
        .chain(directories.values())
        .any(|entry| entry.ctime_ns >= cutoff)
    {
        return Err("a snapshot entry changed too recently to be stamped".to_owned());
    }
    let stamp = SnapshotStamp {
        format: SNAPSHOT_STAMP_FORMAT.to_owned(),
        revision: revision.to_owned(),
        manifest_sha256: hex_sha256(&manifest),
        verifier_version: env!("CARGO_PKG_VERSION").to_owned(),
        files,
        directories,
    };
    let bytes = serde_json::to_vec_pretty(&stamp)
        .map_err(|error| format!("serializing {}: {error}", path.display()))?;
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("stamp"),
        std::process::id()
    ));
    let written = (|| {
        std::fs::write(&temporary, &bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o644))?;
        }
        std::fs::rename(&temporary, &path)
    })();
    if let Err(error) = written {
        std::fs::remove_file(&temporary).ok();
        return Err(format!("writing {}: {error}", path.display()));
    }
    Ok(())
}

/// Metadata of every snapshot file, the manifest and every directory, from a name-only walk.
fn snapshot_entry_stamps(directory: &Path) -> Result<SnapshotEntryStamps, String> {
    let mut paths = Vec::new();
    let mut directory_paths = Vec::new();
    walk_snapshot(directory, directory, &mut paths, &mut directory_paths)?;
    let relative = |path: &Path| {
        path.strip_prefix(directory)
            .expect("collected paths start below the snapshot")
            .to_str()
            .ok_or_else(|| format!("snapshot path {} is not UTF-8", path.display()))
            .map(|relative| relative.replace('\\', "/"))
    };
    let mut files = std::collections::BTreeMap::new();
    for path in paths {
        files.insert(relative(&path)?, file_stamp(&path)?);
    }
    files.insert(
        SNAPSHOT_MANIFEST.to_owned(),
        file_stamp(&directory.join(SNAPSHOT_MANIFEST))?,
    );
    let mut directories = std::collections::BTreeMap::new();
    for path in directory_paths {
        directories.insert(relative(&path)?, file_stamp(&path)?);
    }
    directories.insert(SNAPSHOT_DIRECTORY_KEY.to_owned(), file_stamp(directory)?);
    Ok(SnapshotEntryStamps { files, directories })
}

/// One entry's stamp, without following a symlink.
#[cfg(unix)]
fn file_stamp(path: &Path) -> Result<FileStamp, String> {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("reading metadata for {}: {error}", path.display()))?;
    Ok(FileStamp {
        len: metadata.len(),
        mtime_ns: metadata
            .mtime()
            .saturating_mul(1_000_000_000)
            .saturating_add(metadata.mtime_nsec()),
        ctime_ns: metadata
            .ctime()
            .saturating_mul(1_000_000_000)
            .saturating_add(metadata.ctime_nsec()),
        ino: metadata.ino(),
        dev: metadata.dev(),
        mode: metadata.mode(),
    })
}

/// Without `ctime` and inode numbers a stamp cannot detect an in-place edit, so none is kept.
#[cfg(not(unix))]
fn file_stamp(path: &Path) -> Result<FileStamp, String> {
    Err(format!(
        "snapshot stamps need Unix file metadata ({})",
        path.display()
    ))
}

/// Lowercase hexadecimal SHA-256 of `bytes`.
fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

#[cfg(test)]
thread_local! {
    /// Snapshot files this thread has read and hashed, so tests can prove a stamp spared them.
    static FILES_HASHED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Snapshot directories this thread has listed, so tests can prove a stamp spared them.
    static DIRECTORIES_READ: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Rebuilds a snapshot's manifest from its current bytes and compares it with the sealed one,
/// returning the sealed manifest's file paths.
fn verify_snapshot(directory: &Path, revision: &str) -> Result<Vec<String>, String> {
    let path = directory.join(SNAPSHOT_MANIFEST);
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("reading snapshot manifest {}: {error}", path.display()))?;
    let expected: SnapshotManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("reading snapshot manifest {}: {error}", path.display()))?;
    if expected.format != "aep.source-snapshot/1" || expected.revision != revision {
        return Err(format!(
            "snapshot manifest {} does not identify revision `{revision}`",
            path.display()
        ));
    }
    let actual = snapshot_manifest(directory, revision, false)?;
    if actual == expected {
        Ok(expected.files.into_iter().map(|entry| entry.path).collect())
    } else {
        Err(format!(
            "protocol source snapshot {} does not match its path, mode and byte manifest",
            directory.display()
        ))
    }
}

/// Walks every snapshot file in deterministic order, optionally making it read-only first.
fn snapshot_manifest(
    directory: &Path,
    revision: &str,
    seal: bool,
) -> Result<SnapshotManifest, String> {
    let mut paths = Vec::new();
    collect_snapshot_files(directory, directory, &mut paths)?;
    paths.sort();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        if seal {
            make_read_only(&path, false)?;
        }
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("reading metadata for {}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(directory)
            .expect("collected paths start below the snapshot")
            .to_str()
            .ok_or_else(|| format!("snapshot path {} is not UTF-8", path.display()))?
            .replace('\\', "/");
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("reading snapshot file {}: {error}", path.display()))?;
        #[cfg(test)]
        FILES_HASHED.with(|hashed| hashed.set(hashed.get() + 1));
        files.push(SnapshotEntry {
            path: relative,
            mode: file_mode(&metadata),
            length: metadata.len(),
            sha256: hex_sha256(&bytes),
        });
    }
    Ok(SnapshotManifest {
        format: "aep.source-snapshot/1".to_owned(),
        revision: revision.to_owned(),
        files,
    })
}

/// Collects regular files and refuses symlinks or special filesystem entries.
fn collect_snapshot_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    walk_snapshot(root, directory, files, &mut Vec::new())
}

/// Collects regular files and the directories below `directory`, refusing anything else.
fn walk_snapshot(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
    directories: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory).map_err(|error| {
        format!(
            "reading snapshot directory {}: {error}",
            directory.display()
        )
    })?;
    #[cfg(test)]
    DIRECTORIES_READ.with(|read| read.set(read.get() + 1));
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "reading snapshot directory {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path == root.join(SNAPSHOT_MANIFEST) {
            continue;
        }
        let kind = entry
            .file_type()
            .map_err(|error| format!("reading file type for {}: {error}", path.display()))?;
        if kind.is_symlink() {
            return Err(format!(
                "protocol source snapshot contains symlink {}, which could escape the pinned tree",
                path.display()
            ));
        }
        if kind.is_dir() {
            walk_snapshot(root, &path, files, directories)?;
            directories.push(path);
        } else if kind.is_file() {
            files.push(path);
        } else {
            return Err(format!(
                "protocol source snapshot contains non-file {}",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Makes one snapshot file read-only without discarding its executable bit.
fn make_read_only(path: &Path, directory: bool) -> Result<(), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("reading metadata for {}: {error}", path.display()))?;
    let mut permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let executable = permissions.mode() & 0o111;
        permissions.set_mode(if directory { 0o555 } else { 0o444 | executable });
    }
    #[cfg(not(unix))]
    permissions.set_readonly(true);
    std::fs::set_permissions(path, permissions)
        .map_err(|error| format!("making {} read-only: {error}", path.display()))
}

/// Seals directories from the leaves upwards after their files and manifest are complete.
fn make_directories_read_only(directory: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(directory).map_err(|error| {
        format!(
            "reading snapshot directory {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("reading {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            make_directories_read_only(&path)?;
        }
    }
    make_read_only(directory, true)
}

/// Platform mode recorded in the manifest.
#[cfg(unix)]
fn file_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o777
}

/// Portable read-only bit where Unix modes are unavailable.
#[cfg(not(unix))]
fn file_mode(metadata: &std::fs::Metadata) -> u32 {
    u32::from(metadata.permissions().readonly())
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
            .join("../../..")
            .canonicalize()
            .expect("the workspace root exists")
    }

    /// The protocol tree as a path relative to `<root>/.engineering`.
    ///
    /// A project file may not name a path absolutely — `ProtocolSource::parse` refuses one, because
    /// the file is committed and an absolute path is true on one machine. These fixtures used to
    /// write `protocol_tree()` straight in; they now write the climb to it, which is also what a
    /// real adopter with a sibling checkout writes.
    fn relative_tree(root: &Path) -> String {
        let base = root.join(".engineering");
        let base: Vec<_> = base.components().collect();
        let tree = protocol_tree();
        let tree: Vec<_> = tree.components().collect();
        let shared = base
            .iter()
            .zip(&tree)
            .take_while(|(left, right)| left == right)
            .count();
        let mut parts = vec![".."; base.len() - shared];
        parts.extend(
            tree[shared..]
                .iter()
                .map(|component| component.as_os_str().to_str().expect("a printable path")),
        );
        parts.join("/")
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
                relative_tree(&root)
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
                relative_tree(&root)
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
                relative_tree(&root)
            ),
        );

        let project = load(&root).expect("the documents themselves are well formed");
        let task = aep_schema::parse::task(
            "id: T-1\nkind: feature\nobjective: something\nprotocol: adp/1\nprofile: house.reckless\n",
            None,
        )
        .expect("the task parses");

        let errors = aep_engine::resolve(&task, &project.registry)
            .expect_err("the approval floor still applies to a project's own profile");
        assert!(errors.contains(aep_domain::error::ValidationCode::ProductionWriteWithoutApproval));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn credential_bearing_git_urls_are_refused_before_git_sees_them() {
        assert!(refuse_credentials("https://example.invalid/repository").is_ok());
        assert!(refuse_credentials("ssh://git@example.invalid/repository").is_ok());
        let refusal = refuse_credentials("https://person:secret@example.invalid/repository")
            .expect_err("credentials must not enter Git configuration or process arguments");
        assert!(refusal.contains("credentials"), "{refusal}");
    }

    /// Seals `root` as a snapshot of `revision`, as materialization does.
    fn seal(root: &Path, revision: &str) {
        let manifest = snapshot_manifest(root, revision, false).expect("a sealable tree");
        std::fs::write(
            root.join(SNAPSHOT_MANIFEST),
            serde_json::to_vec_pretty(&manifest).expect("a serializable manifest"),
        )
        .expect("the manifest is writable");
    }

    #[test]
    fn a_snapshot_verified_once_is_not_re_read_by_the_same_process() {
        let root = scratch("source-verified-once");
        let revision = "b".repeat(40);
        write(&root.join("protocol.yaml"), "sealed\n");
        seal(&root, &revision);
        verify_snapshot_once(&root, &revision).expect("the sealed bytes verify");

        write(&root.join("protocol.yaml"), "changed after verification\n");
        assert!(
            verify_snapshot(&root, &revision).is_err(),
            "a full verification still sees the changed bytes"
        );
        verify_snapshot_once(&root, &revision)
            .expect("the process already verified this snapshot and does not re-read it");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_failed_snapshot_verification_is_not_remembered() {
        let root = scratch("source-verification-failed");
        let revision = "c".repeat(40);
        write(&root.join("protocol.yaml"), "sealed\n");
        seal(&root, &revision);
        write(&root.join("protocol.yaml"), "changed before verification\n");
        for attempt in 0..2 {
            let refusal = verify_snapshot_once(&root, &revision)
                .expect_err("changed bytes are refused on every attempt");
            assert!(
                refusal.contains("does not match"),
                "attempt {attempt}: {refusal}"
            );
        }
        std::fs::remove_dir_all(&root).ok();
    }

    /// A sealed two-file snapshot with no stamp beside it, and the stamp's path.
    fn sealed_snapshot(name: &str, revision: &str) -> (PathBuf, PathBuf) {
        let root = scratch(name);
        let stamp = snapshot_stamp_path(&root).expect("the scratch directory has a name");
        std::fs::remove_file(&stamp).ok();
        write(&root.join("protocol.yaml"), "sealed\n");
        write(&root.join("profiles/standard.yaml"), "standard\n");
        seal(&root, revision);
        (root, stamp)
    }

    /// Runs the cross-process path with no racy window and returns how many files it hashed.
    fn stamped_verification(root: &Path, revision: &str) -> (Result<(), String>, usize) {
        let (result, hashed, _) = stamped_verification_counting(root, revision);
        (result, hashed)
    }

    /// [`stamped_verification`], also returning how many directories it listed.
    fn stamped_verification_counting(
        root: &Path,
        revision: &str,
    ) -> (Result<(), String>, usize, usize) {
        FILES_HASHED.with(|hashed| hashed.set(0));
        DIRECTORIES_READ.with(|read| read.set(0));
        let result = verify_snapshot_stamped(root, revision, std::time::Duration::ZERO).map(drop);
        (
            result,
            FILES_HASHED.with(std::cell::Cell::get),
            DIRECTORIES_READ.with(std::cell::Cell::get),
        )
    }

    /// Lets the coarse file clock move past the stamp before a test tampers with the snapshot.
    fn after_the_stamp() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    /// Asserts that a tampered snapshot is refused by a full verification that writes no stamp.
    fn assert_refused_without_restamping(root: &Path, stamp: &Path, revision: &str, before: &[u8]) {
        let (refused, hashed, listed) = stamped_verification_counting(root, revision);
        let refusal = refused.expect_err("the tree no longer matches the sealed manifest");
        assert!(refusal.contains("does not match"), "{refusal}");
        assert!(
            hashed > 0 && listed > 0,
            "the stamp was not trusted, so the tree was listed and re-hashed"
        );
        assert_eq!(
            std::fs::read(stamp).expect("still there"),
            before,
            "a failed verification writes no stamp"
        );
    }

    fn cleanup(root: &Path, stamp: &Path) {
        std::fs::remove_dir_all(root).ok();
        std::fs::remove_file(stamp).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_stamped_snapshot_is_trusted_by_a_later_process_without_reading_its_files() {
        let revision = "d".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-trusted", &revision);

        let (first, hashed) = stamped_verification(&root, &revision);
        first.expect("the sealed bytes verify");
        assert_eq!(hashed, 2, "the first verification hashes every file");
        assert!(
            stamp.exists(),
            "a successful full verification leaves a stamp"
        );

        let (second, hashed, listed) = stamped_verification_counting(&root, &revision);
        second.expect("the stamp still matches");
        assert_eq!(hashed, 0, "a matching stamp spares every file read");
        assert_eq!(listed, 0, "a matching stamp spares every directory listing");
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn an_in_place_edit_with_its_length_and_mtime_restored_is_still_refused() {
        let revision = "e".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-edited", &revision);
        stamped_verification(&root, &revision).0.expect("verifies");
        assert!(stamp.exists());

        // Let the coarse file clock move past the stamp before tampering.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let file = root.join("protocol.yaml");
        let mtime = std::fs::metadata(&file)
            .and_then(|m| m.modified())
            .expect("mtime");
        std::fs::write(&file, "forged\n").expect("same length, different bytes");
        std::fs::File::options()
            .write(true)
            .open(&file)
            .and_then(|handle| handle.set_modified(mtime))
            .expect("the mtime is restored");
        assert_eq!(std::fs::metadata(&file).expect("stat").len(), 7);

        let (refused, hashed) = stamped_verification(&root, &revision);
        let refusal =
            refused.expect_err("ctime moved, so the stamp does not hold and bytes differ");
        assert!(refusal.contains("does not match"), "{refusal}");
        assert!(hashed > 0, "the fallback re-hashed the tree");
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_added_after_stamping_is_refused() {
        let revision = "f".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-extra", &revision);
        stamped_verification(&root, &revision).0.expect("verifies");
        let before = std::fs::read(&stamp).expect("stamped");

        after_the_stamp();
        write(&root.join("profiles/extra.yaml"), "smuggled\n");
        assert_refused_without_restamping(&root, &stamp, &revision, &before);
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_removed_after_stamping_is_refused() {
        let revision = "5".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-removed", &revision);
        stamped_verification(&root, &revision).0.expect("verifies");
        let before = std::fs::read(&stamp).expect("stamped");

        after_the_stamp();
        std::fs::remove_file(root.join("profiles/standard.yaml")).expect("removable");
        assert_refused_without_restamping(&root, &stamp, &revision, &before);
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_renamed_within_its_directory_after_stamping_is_refused() {
        let revision = "6".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-renamed", &revision);
        stamped_verification(&root, &revision).0.expect("verifies");
        let before = std::fs::read(&stamp).expect("stamped");

        after_the_stamp();
        std::fs::rename(
            root.join("profiles/standard.yaml"),
            root.join("profiles/renamed.yaml"),
        )
        .expect("renamable");
        assert_refused_without_restamping(&root, &stamp, &revision, &before);
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_format_one_stamp_is_ignored_and_replaced() {
        let revision = "7".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-format-one", &revision);
        stamped_verification(&root, &revision).0.expect("verifies");
        let mut current: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&stamp).expect("stamped")).expect("parses");
        let fields = current.as_object_mut().expect("a stamp is an object");
        fields.insert("format".to_owned(), "aep.snapshot-verified/1".into());
        let directories = fields
            .remove("directories")
            .expect("format 2 lists directories");
        fields["files"].as_object_mut().expect("files").insert(
            SNAPSHOT_DIRECTORY_KEY.to_owned(),
            directories[SNAPSHOT_DIRECTORY_KEY].clone(),
        );
        std::fs::write(&stamp, serde_json::to_vec(&current).expect("serializes"))
            .expect("writable");

        let (result, hashed, listed) = stamped_verification_counting(&root, &revision);
        result.expect("the snapshot itself is intact");
        assert_eq!(hashed, 2, "a format-1 stamp is not trusted");
        assert!(listed > 0, "a format-1 stamp is not trusted");
        let rewritten: SnapshotStamp =
            serde_json::from_slice(&std::fs::read(&stamp).expect("rewritten")).expect("parses");
        assert_eq!(rewritten.format, SNAPSHOT_STAMP_FORMAT);
        assert!(rewritten.directories.contains_key("profiles"));
        assert_eq!(stamped_verification_counting(&root, &revision).2, 0);
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_corrupt_stamp_falls_back_to_full_verification_and_is_rewritten() {
        let revision = "1".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-corrupt", &revision);
        std::fs::write(&stamp, "{ not a stamp").expect("a corrupt stamp");

        let (result, hashed) = stamped_verification(&root, &revision);
        result.expect("the snapshot itself is intact");
        assert_eq!(hashed, 2, "a corrupt stamp is not trusted");
        let rewritten: SnapshotStamp =
            serde_json::from_slice(&std::fs::read(&stamp).expect("rewritten")).expect("parses");
        assert_eq!(rewritten.format, SNAPSHOT_STAMP_FORMAT);
        assert_eq!(rewritten.revision, revision);
        assert_eq!(stamped_verification(&root, &revision).1, 0);
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_stamp_for_another_revision_is_ignored() {
        let revision = "2".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-revision", &revision);
        stamped_verification(&root, &revision).0.expect("verifies");
        let mut other: SnapshotStamp =
            serde_json::from_slice(&std::fs::read(&stamp).expect("stamped")).expect("parses");
        other.revision = "3".repeat(40);
        std::fs::write(&stamp, serde_json::to_vec(&other).expect("serializes")).expect("writable");

        let (result, hashed) = stamped_verification(&root, &revision);
        result.expect("the snapshot itself is intact");
        assert_eq!(
            hashed, 2,
            "a stamp naming another revision proves nothing here"
        );
        cleanup(&root, &stamp);
    }

    #[cfg(unix)]
    #[test]
    fn a_snapshot_changed_within_the_racy_window_is_not_stamped() {
        let revision = "4".repeat(40);
        let (root, stamp) = sealed_snapshot("stamp-racy", &revision);
        verify_snapshot_stamped(&root, &revision, RACY_WINDOW).expect("the sealed bytes verify");
        assert!(
            !stamp.exists(),
            "files written just now could change again within the clock's resolution"
        );
        cleanup(&root, &stamp);
    }

    /// Copies `from` into `to`, recursively.
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("the copy is writable");
        for entry in std::fs::read_dir(from).expect("the source is readable") {
            let entry = entry.expect("an entry");
            let target = to.join(entry.file_name());
            if entry.file_type().expect("a file type").is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).expect("the file copies");
            }
        }
    }

    /// A sealed snapshot of the repository's own protocol tree, plus `extra` files.
    fn sealed_protocol_snapshot(name: &str, revision: &str, extra: &[(&str, &str)]) -> PathBuf {
        let root = scratch(name);
        std::fs::remove_dir_all(root.join(project_directory())).ok();
        if let Some(stamp) = snapshot_stamp_path(&root) {
            std::fs::remove_file(stamp).ok();
        }
        for directory in [
            "protocols",
            "principles",
            "workflows",
            "profiles",
            "artifacts/lifecycles",
            "drivers",
        ] {
            copy_tree(&protocol_tree().join(directory), &root.join(directory));
        }
        for (path, contents) in extra {
            write(&root.join(path), contents);
        }
        seal(&root, revision);
        root
    }

    /// Everything a tree load observably produced, for comparing two loads of one tree.
    fn load_fingerprint(outcome: &crate::load::LoadOutcome) -> String {
        let failures: Vec<String> = outcome.failures.iter().map(ToString::to_string).collect();
        format!(
            "files_read={}\nfailures={failures:#?}\nregistry={:#?}\ndrivers={:#?}",
            outcome.files_read, outcome.registry, outcome.drivers
        )
    }

    fn tree_directories_listed() -> usize {
        crate::load::TREE_DIRECTORIES_LISTED.with(std::cell::Cell::get)
    }

    /// Loads `root` once from its sealed listing and once by walking it, and returns both.
    fn listed_and_walked(root: &Path) -> (crate::load::LoadOutcome, crate::load::LoadOutcome) {
        let listed = load_tree_report(root);
        crate::load::forget_sealed_listing(root);
        let walked = load_tree_report(root);
        (listed, walked)
    }

    #[cfg(unix)]
    #[test]
    fn a_stamped_snapshot_tree_loads_without_opening_a_directory() {
        let revision = "8".repeat(40);
        let root = sealed_protocol_snapshot("tree-load-stamped", &revision, &[]);
        let stamp = snapshot_stamp_path(&root).expect("a stamp path");
        verify_snapshot_stamped(&root, &revision, std::time::Duration::ZERO)
            .expect("the sealed tree verifies and is stamped");
        crate::load::forget_sealed_listing(&root);

        DIRECTORIES_READ.with(|read| read.set(0));
        crate::load::TREE_DIRECTORIES_LISTED.with(|listed| listed.set(0));
        verify_snapshot_once(&root, &revision).expect("the stamp holds");
        let outcome = load_tree_report(&root);
        let bundle = crate::load::load_bundle(&root).expect("the tree is a valid bundle");
        assert_eq!(
            DIRECTORIES_READ.with(std::cell::Cell::get),
            0,
            "a matching stamp spares every directory listing"
        );
        assert_eq!(
            tree_directories_listed(),
            0,
            "a stamped snapshot's tree is listed from its stamp, not from its directories"
        );
        assert!(outcome.is_clean(), "{}", load_fingerprint(&outcome));
        assert!(outcome.files_read > 50, "the whole tree loaded");

        crate::load::forget_sealed_listing(&root);
        let walked = load_tree_report(&root);
        let walked_bundle = crate::load::load_bundle(&root).expect("the walked bundle loads");
        assert!(
            tree_directories_listed() > 0,
            "without a listing the tree is walked"
        );
        assert_eq!(load_fingerprint(&outcome), load_fingerprint(&walked));
        assert_eq!(bundle.digest, walked_bundle.digest);
        assert_eq!(bundle.files_read, walked_bundle.files_read);
        cleanup(&root, &stamp);
    }

    /// The listing applies the walk's rules: dot entries skipped at any depth, documents chosen by
    /// extension, subdirectories descended. Every extra here is garbage, so any divergence shows up
    /// as a different failure or file count.
    #[test]
    fn a_sealed_listing_selects_exactly_the_files_the_walk_selects() {
        let revision = "9".repeat(40);
        let root = sealed_protocol_snapshot(
            "tree-load-rules",
            &revision,
            &[
                ("protocols/.hidden/skipped.yaml", "not: [a protocol"),
                ("principles/.skipped.yaml", "not: [a principle"),
                ("principles/nested/.deeper/skipped.json", "{"),
                ("profiles/notes.md", "not a document"),
                ("profiles/README", "not a document"),
                ("workflows/nested/deeper/broken.yaml", "not: [a workflow"),
                ("workflows/broken.yml", "not: [a workflow"),
                ("drivers/broken.json", "{"),
                ("artifacts/lifecycles/broken.yaml", "not: [a lifecycle"),
                ("artifacts/lifecyclesx/outside.yaml", "not: [a lifecycle"),
                ("artifacts/elsewhere.yaml", "not: [a lifecycle"),
                (".hidden.yaml", "not: [anything"),
                ("top-level.yaml", "not: [anything"),
            ],
        );
        let listing = verify_snapshot(&root, &revision).expect("the sealed tree verifies");
        crate::load::remember_sealed_listing(&root, listing);

        crate::load::TREE_DIRECTORIES_LISTED.with(|listed| listed.set(0));
        let (listed, walked) = listed_and_walked(&root);
        assert!(
            tree_directories_listed() > 0,
            "the second load walked the tree"
        );
        let said: Vec<String> = listed.failures.iter().map(ToString::to_string).collect();
        assert_eq!(
            said.len(),
            4,
            "the four garbage documents the walk reads: {said:#?}"
        );
        assert_eq!(load_fingerprint(&listed), load_fingerprint(&walked));
        std::fs::remove_dir_all(&root).ok();
    }

    /// A local protocol root has no sealed listing and keeps being walked.
    #[test]
    fn a_local_protocol_root_is_still_read_from_its_directories() {
        let root = minimal_project("local-root-walked");
        crate::load::TREE_DIRECTORIES_LISTED.with(|listed| listed.set(0));
        let project = load(&root).expect("the project loads");
        assert!(project.registry.principles().count() >= 20);
        assert!(
            tree_directories_listed() > 0,
            "a local tree is not a sealed snapshot, so its directories are listed"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_refused_before_a_snapshot_manifest_can_seal_it() {
        use std::os::unix::fs::symlink;

        let root = scratch("source-symlink");
        write(&root.join("outside.yaml"), "outside\n");
        symlink("../outside.yaml", root.join("linked.yaml")).expect("a symlink fixture");
        let refusal = snapshot_manifest(&root, &"a".repeat(40), false)
            .expect_err("a symlink could escape the pinned tree");
        assert!(refusal.contains("symlink"), "{refusal}");
        std::fs::remove_dir_all(&root).ok();
    }
}
