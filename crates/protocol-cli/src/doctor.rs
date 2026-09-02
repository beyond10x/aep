//! `aep doctor` — whether this checkout is in a state the other verbs will accept.
//!
//! # The gap this closes
//!
//! The second adopter's report ended with *"I am still unsure if I even used them properly"*, and
//! nothing in this CLI answered that. The facts were all obtainable and each from somewhere else:
//! `--version` for the binary, `reverse init` for the protocol source, `artifact validate` for the
//! store, `drive run --plugin-dir` for the plugins, and nothing at all for whether the binary in
//! the path is the one this checkout's tags describe. An adopter who has to know which five verbs
//! to run, and how to read five different reports, is being asked to already understand the tool
//! they are evaluating.
//!
//! So: one verb, one line per check, `ok` / `warn` / `fail`, and exit `1` when anything failed.
//!
//! # What `warn` means here, exactly
//!
//! **`warn` is for a state that does not stop the other verbs.** No Git checkout, no plugin
//! directory named, a pinned protocol source whose snapshot is not in the cache yet — none of these
//! stop `artifact`, `evaluate` or `drive` from working, so none of them may decide the exit code. A
//! preflight that exits `1` on a normal state is a preflight people learn to ignore.
//!
//! `warn` also carries the one thing invariant 5 requires and a boolean cannot: **not checked**. A
//! project file that did not parse leaves the source and store questions *unanswered*, and an
//! unanswered question is reported as one rather than rewritten into a second failure — the defect
//! is one defect, and the line above already named it.
//!
//! # What this reads, and what it refuses to
//!
//! No clock and no network, both load-bearing. The report is a function of the tree, so two runs
//! over one checkout print identical bytes and a diff of two reports is a diff of two checkouts.
//! Concretely:
//!
//! * a pinned `git+…#<40-hex>` protocol source is checked for **shape** and for a **cached
//!   snapshot** — never fetched, because a preflight that silently pulls a governing document tree
//!   from the network has changed the thing it was asked to describe;
//! * the release-tag check reads `git tag --list --merged HEAD`, which is local;
//! * a plan kept in PostgreSQL is reported as *not checked here*, because reading it is a
//!   connection. `aep artifact validate` is the verb that may open one.
//!
//! # Why this fixes nothing
//!
//! `story:aep-doctor-preflight` puts installing and repairing out of scope, and the reason is worth
//! keeping next to the code: a checker that also repairs cannot be run to find out what is wrong.
//! Every line here names the state and, where there is one, the command that changes it.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_domain::project::{GitProtocolSource, ProjectConfig, ProtocolSource, PROJECT_FILE};
use aep_project::project::{project_directory, CACHE_DIRECTORY_ENV};
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use sha2::{Digest as _, Sha256};

use crate::planning::{Plan, Replica};

/// What `aep --version` prints, which is the workspace version this binary was built from.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The manifest a Claude-family plugin directory carries.
const CLAUDE_MANIFEST: &str = ".claude-plugin/plugin.json";

/// The manifest a Codex-family plugin directory carries.
const CODEX_MANIFEST: &str = ".codex-plugin/plugin.json";

/// How `doctor` renders its report.
///
/// Two renderings and not three, for the reason `trace`'s own format enum gives: a report with one
/// line per check has a shape for a person and a shape for a program, and `yaml` would be a third
/// spelling of the second that nothing asked for. A value a verb cannot honour is worse than one it
/// does not offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum DoctorFormat {
    /// One line per check.
    Text,
    /// The same checks as JSON, for a script that gates on one of them.
    Json,
}

/// The arguments of `aep doctor`.
#[derive(Debug, Args)]
pub(crate) struct DoctorArgs {
    /// The checkout to report on.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// A plugin directory to check for a manifest. Repeatable.
    ///
    /// Given directories are the whole list; `AEP_DRIVE_PLUGIN_DIR` supplies one only when none is
    /// given, which is the rule `aep drive` follows and invariant 12 requires. Nothing here guesses
    /// a path under the checkout.
    #[arg(long)]
    plugin_dir: Vec<PathBuf>,
    /// How to render the report.
    #[arg(long, value_enum, default_value_t = DoctorFormat::Text)]
    format: DoctorFormat,
}

/// What one check found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    /// The other verbs will accept this.
    Ok,
    /// Not a defect: a state the other verbs work in, or a question this run could not answer.
    Warn,
    /// Something the other verbs will refuse. Decides the exit code.
    Fail,
}

impl Status {
    /// The word the text rendering prints, padded so the checks line up in a column.
    const fn word(self) -> &'static str {
        match self {
            Self::Ok => "ok  ",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }
}

/// One line of the report.
#[derive(Debug, serde::Serialize)]
struct Check {
    /// The stable code for this line: what a test and a `jq` filter both match on, so that
    /// rewording a `detail` for a reader cannot break either. Named `code` here and `check` in the
    /// rendering, because a field called `check` on a struct called `Check` is a lint.
    #[serde(rename = "check")]
    code: &'static str,
    /// What was found.
    status: Status,
    /// The reason, in the words a person needs to act on it.
    detail: String,
}

impl Check {
    /// A line, spelled the way every construction site here spells one.
    fn new(code: &'static str, status: Status, detail: impl Into<String>) -> Self {
        Self {
            code,
            status,
            detail: detail.into(),
        }
    }
}

/// The whole report.
#[derive(Debug, serde::Serialize)]
struct Report {
    /// The checks, in the fixed order they are made.
    checks: Vec<Check>,
    /// How many failed — the number that decides the exit code, stated rather than recounted.
    failed: usize,
}

// The stable code of each check. A test and a `jq` filter both match on these rather than on a
// line's position or its prose, so a reworded `detail` breaks neither.

/// Which build this is.
const BINARY_VERSION: &str = "binary-version";
/// Whether `.engineering/project.yaml` is there and parses.
const PROJECT_FILE_CHECK: &str = "project-file";
/// Whether the tree that project file governs by can be reached without a network.
const PROTOCOL_SOURCE: &str = "protocol-source";
/// Whether the plan is there and `aep artifact validate` would pass over it.
const PLANNING_STORE: &str = "planning-store";
/// Whether one named plugin directory carries a manifest. One line per directory.
const PLUGIN_DIRECTORY: &str = "plugin-directory";
/// Whether the checkout's newest release tag agrees with this binary's version.
const RELEASE_TAG: &str = "release-tag";

/// `aep doctor`
pub(crate) fn run(args: &DoctorArgs) -> Result<ExitCode> {
    let report = examine(args);
    match args.format {
        DoctorFormat::Text => {
            for check in &report.checks {
                outln!("{}  {}: {}", check.status.word(), check.code, check.detail);
            }
        }
        DoctorFormat::Json => outln!(
            "{}",
            serde_json::to_string_pretty(&report).context("rendering the report as JSON")?
        ),
    }
    Ok(crate::exit_code(report.failed == 0))
}

/// Every check, in a fixed order, over one tree.
///
/// The order is the order an adopter's questions arrive in — *is my binary right, is my project
/// file right, is the tree it names there, is my plan readable, are my plugins where I said, is
/// this the build my tags describe* — and it is fixed rather than derived so two runs over one
/// checkout print identical bytes.
///
/// Two checks are threaded rather than independent, and both for the same reason: the answer they
/// need was already obtained without touching the network. The project file's configuration decides
/// what the source check examines, and the source check's resolved tree is where the store check
/// reads its lifecycles from.
fn examine(args: &DoctorArgs) -> Report {
    let root = args.root.as_path();
    let engineering = root.join(project_directory());

    let (project_check, config) = project_file(&engineering);
    let (source_check, tree) = protocol_source(&engineering, config.as_ref());
    let store_check = planning_store(root, &engineering, config.as_ref(), tree.as_deref());

    let mut checks = vec![
        Check::new(BINARY_VERSION, Status::Ok, VERSION),
        project_check,
        source_check,
        store_check,
    ];
    checks.extend(plugin_directories(&args.plugin_dir));
    checks.push(release_tag(root));

    let failed = checks
        .iter()
        .filter(|check| check.status == Status::Fail)
        .count();
    Report { checks, failed }
}

/// Does `.engineering/project.yaml` exist, and does it parse?
///
/// Parsed through the same loader every verb goes through, so a file this accepts is a file
/// `evaluate` accepts. [`aep_project::project::load_config`] is deliberately the lightest of the
/// three: it resolves nothing the file names, which is what keeps this check offline and what makes
/// the source check below a separate line rather than a hidden precondition of this one.
fn project_file(engineering: &Path) -> (Check, Option<ProjectConfig>) {
    let path = engineering.join(PROJECT_FILE);
    let Some(root) = engineering.parent() else {
        return (
            Check::new(
                PROJECT_FILE_CHECK,
                Status::Fail,
                format!("{} has no parent directory", engineering.display()),
            ),
            None,
        );
    };
    match aep_project::project::load_config(root) {
        Ok(config) => (
            Check::new(
                PROJECT_FILE_CHECK,
                Status::Ok,
                format!(
                    "{} parses: protocol {}, profile {}",
                    path.display(),
                    config.protocol,
                    config.profile
                ),
            ),
            Some(config),
        ),
        Err(errors) => (
            Check::new(
                PROJECT_FILE_CHECK,
                Status::Fail,
                format!(
                    "{} — {}",
                    path.display(),
                    errors
                        .as_slice()
                        .iter()
                        .map(|failure| failure.detail.replace('\n', " "))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            ),
            None,
        ),
    }
}

/// Does the project's `protocols:` source resolve, without fetching anything?
///
/// Two shapes, and the line says which one it read:
///
/// * a **path**, resolved against `.engineering` exactly as
///   [`aep_domain::project::ProjectLocalPaths::resolved`] resolves it, and either there or not;
/// * a **pinned locator**, whose shape [`ProtocolSource::parse`] has already accepted — a
///   `git+ssh://`, `git+https://` or `git+file://` repository and a full 40-hex commit — and whose
///   snapshot either is in the cache or is not.
///
/// An uncached locator is a `warn` and not a `fail`, because the next command that needs the tree
/// will fetch it and succeed. What it must not be is invisible: on a machine with no network, that
/// `warn` is the whole explanation for the refusal that follows.
///
/// Returns the tree when there is one to read, so the store check below can load its lifecycles
/// from it without asking the loader to resolve the same source again — which, for a locator, would
/// fetch.
fn protocol_source(engineering: &Path, config: Option<&ProjectConfig>) -> (Check, Option<PathBuf>) {
    let Some(config) = config else {
        return (
            Check::new(
                PROTOCOL_SOURCE,
                Status::Warn,
                "not checked: the project file was not read, so nothing here names a source",
            ),
            None,
        );
    };
    match &config.protocols {
        ProtocolSource::Path(path) => {
            let resolved = engineering.join(path);
            if resolved.is_dir() {
                (
                    Check::new(
                        PROTOCOL_SOURCE,
                        Status::Ok,
                        format!(
                            "the path `{}` is the directory {}",
                            path.display(),
                            resolved.display()
                        ),
                    ),
                    Some(resolved),
                )
            } else {
                (
                    Check::new(
                        PROTOCOL_SOURCE,
                        Status::Fail,
                        format!(
                            "the path `{}` resolves to {}, which is not a directory",
                            path.display(),
                            resolved.display()
                        ),
                    ),
                    None,
                )
            }
        }
        ProtocolSource::Git(source) => git_source(source),
    }
}

/// A pinned locator: well-formed by construction, cached or not by observation.
fn git_source(source: &GitProtocolSource) -> (Check, Option<PathBuf>) {
    let locator = format!("{}#{}", source.repository(), source.revision());
    match cached_snapshot(source) {
        Err(reason) => (
            Check::new(
                PROTOCOL_SOURCE,
                Status::Warn,
                format!(
                    "the locator `{locator}` is well-formed; whether its snapshot is cached is \
                     unknown — {reason}"
                ),
            ),
            None,
        ),
        Ok(snapshot) if snapshot.exists() => (
            Check::new(
                PROTOCOL_SOURCE,
                Status::Ok,
                format!(
                    "the locator `{locator}` is well-formed and its snapshot is cached at {}",
                    snapshot.display()
                ),
            ),
            Some(snapshot),
        ),
        Ok(snapshot) => (
            Check::new(
                PROTOCOL_SOURCE,
                Status::Warn,
                format!(
                    "the locator `{locator}` is well-formed and no snapshot is cached at {}; the \
                     next command that needs the tree will fetch it, and `doctor` does not",
                    snapshot.display()
                ),
            ),
            None,
        ),
    }
}

/// Where `aep-project` would materialize this source's snapshot.
///
/// **A second spelling of a layout `aep-project` owns** — `git_cache_paths` and `cache_root` in
/// `crates/aep-project/src/project.rs` — and it is spelled again here because both are private and
/// this crate does not widen another's API for its own convenience. The duplication is the price of
/// answering *is it already there* without calling the resolver, which for an uncached source
/// answers by fetching it.
///
/// The predicate is the loader's own: it skips the fetch when the destination **exists**, so that
/// is what is checked, rather than a second opinion about what a valid snapshot looks like.
fn cached_snapshot(source: &GitProtocolSource) -> Result<PathBuf, String> {
    Ok(snapshot_path(&cache_root()?, source))
}

/// The snapshot layout itself, with the cache root supplied.
///
/// Separated from [`cached_snapshot`] so the layout can be asserted without a test setting a
/// process-wide environment variable that every other test in this binary would then race with.
fn snapshot_path(cache: &Path, source: &GitProtocolSource) -> PathBuf {
    let repository = Sha256::digest(source.repository().as_bytes())
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        });
    cache
        .join("protocol-sources")
        .join(repository)
        .join("snapshots")
        .join(source.revision())
}

/// The operator-selected cache root, then the platform cache, then the conventional user cache.
///
/// The same order `aep-project` reads them in, for the same reason it is duplicated: see
/// [`cached_snapshot`].
fn cache_root() -> Result<PathBuf, String> {
    for name in [CACHE_DIRECTORY_ENV, "XDG_CACHE_HOME", "HOME"] {
        let Some(value) = std::env::var_os(name).filter(|value| !value.is_empty()) else {
            continue;
        };
        let path = PathBuf::from(value);
        return Ok(match name {
            CACHE_DIRECTORY_ENV => path,
            "XDG_CACHE_HOME" => path.join("aep"),
            _ => path.join(".cache/aep"),
        });
    }
    Err(format!(
        "this machine names no cache directory (set `{CACHE_DIRECTORY_ENV}` or `XDG_CACHE_HOME`)"
    ))
}

/// Does the planning store exist, and would `aep artifact validate` pass over it?
///
/// The second half is answered by [`crate::planning::store_findings`], which is the verb's own
/// accumulation and not a re-implementation of it. That is the point of the check: a preflight that
/// reads the store more weakly than the verb would say *ready* about a store the verb refuses,
/// which is the failure this story exists to prevent.
///
/// The store is checked even when there is no project file, because `<root>/.engineering/planning`
/// is what a store defaults to and *there is no store here* is the most useful thing this verb can
/// say to somebody who has not adopted anything yet.
fn planning_store(
    root: &Path,
    engineering: &Path,
    config: Option<&ProjectConfig>,
    tree: Option<&Path>,
) -> Check {
    if config.is_none() && engineering.join(PROJECT_FILE).exists() {
        return Check::new(
            PLANNING_STORE,
            Status::Warn,
            "not checked: the project file did not parse, so nothing here says where the plan is",
        );
    }
    let plan = match Plan::for_project(engineering) {
        Ok(plan) => plan,
        Err(error) => {
            return Check::new(
                PLANNING_STORE,
                Status::Fail,
                format!("the project file does not name a plan this build can open: {error:#}"),
            )
        }
    };
    if let Some(missing) = absent(&plan) {
        return Check::new(
            PLANNING_STORE,
            Status::Fail,
            format!("there is no planning store at {missing}"),
        );
    }
    if reaches_a_network(&plan) {
        return Check::new(
            PLANNING_STORE,
            Status::Warn,
            format!(
                "not checked: this project keeps its plan in {}, and reading it is a connection \
                 `doctor` does not open — `aep artifact validate` answers this one",
                plan.describe()
            ),
        );
    }

    // The lifecycles come from the tree the source check resolved, or from the checkout itself when
    // it resolved none. The fallback is not a guess: a tree with no `artifacts/lifecycles/` yields
    // an empty registry and every kind gets the permissive ladder, which is exactly what
    // `artifact validate` does in a repository that has adopted no document tree.
    let document_root = tree.unwrap_or(root);
    let describe = plan.describe();
    match crate::planning::store_findings(plan, root, document_root) {
        Err(error) => Check::new(
            PLANNING_STORE,
            Status::Fail,
            format!("{describe} could not be read: {error:#}"),
        ),
        Ok(summary) if summary.problems.is_empty() => Check::new(
            PLANNING_STORE,
            Status::Ok,
            format!(
                "{}: {} artifact(s), no problems",
                summary.store, summary.artifacts
            ),
        ),
        Ok(summary) => Check::new(
            PLANNING_STORE,
            Status::Fail,
            format!(
                "{}: {} problem(s), the first of them `{}` — `aep artifact validate` lists them all",
                summary.store,
                summary.problems.len(),
                summary.problems[0].replace('\n', " ")
            ),
        ),
    }
}

/// Where a plan says it is, when nothing is there.
///
/// A Postgres plan is absent from this answer on purpose: whether a database exists is a question
/// only a connection answers, and [`reaches_a_network`] is what reports that instead.
fn absent(plan: &Plan) -> Option<String> {
    match plan {
        Plan::Markdown { root } | Plan::Hybrid { root, .. } if !root.is_dir() => {
            Some(root.display().to_string())
        }
        Plan::Sqlite { path } if !path.is_file() => Some(path.display().to_string()),
        _ => None,
    }
}

/// Would reading this plan open a connection?
///
/// `doctor` opens none, so a plan that keeps its documents in PostgreSQL — on its own or as the
/// replica half of a hybrid — is reported as not checked rather than checked over the network.
fn reaches_a_network(plan: &Plan) -> bool {
    match plan {
        Plan::Postgres { .. } => true,
        Plan::Hybrid { replica, .. } => matches!(replica, Replica::Postgres(_)),
        Plan::Markdown { .. } | Plan::Sqlite { .. } => false,
    }
}

/// Does each plugin directory the operator named carry a manifest?
///
/// The list is built by exactly the rule `aep drive` uses, and invariant 12 is why: named
/// directories are the whole list, `AEP_DRIVE_PLUGIN_DIR` supplies one only when none is named, and
/// nothing falls back to a path inside the checkout. A preflight that reported on a plugin
/// directory the driver would not load would be describing a different run.
///
/// No directory at all is a `warn`: every verb but `eval run --arm plugin` works without one.
fn plugin_directories(named: &[PathBuf]) -> Vec<Check> {
    let directories: Vec<PathBuf> = if named.is_empty() {
        std::env::var_os(crate::drive::PLUGIN_DIR_ENV)
            .map(|value| vec![PathBuf::from(value)])
            .unwrap_or_default()
    } else {
        named.to_vec()
    };
    if directories.is_empty() {
        return vec![Check::new(
            PLUGIN_DIRECTORY,
            Status::Warn,
            format!(
                "none given: pass `--plugin-dir <path>` or set `{}`. AEP ships no plugin sources \
                 and guesses no path",
                crate::drive::PLUGIN_DIR_ENV
            ),
        )];
    }
    directories
        .iter()
        .map(|directory| {
            let manifest = [CLAUDE_MANIFEST, CODEX_MANIFEST]
                .into_iter()
                .find(|manifest| directory.join(manifest).is_file());
            match manifest {
                Some(manifest) => Check::new(
                    PLUGIN_DIRECTORY,
                    Status::Ok,
                    format!("{} carries {manifest}", directory.display()),
                ),
                None if !directory.is_dir() => Check::new(
                    PLUGIN_DIRECTORY,
                    Status::Fail,
                    format!("{} is not a directory", directory.display()),
                ),
                None => Check::new(
                    PLUGIN_DIRECTORY,
                    Status::Fail,
                    format!(
                        "{} carries neither {CLAUDE_MANIFEST} nor {CODEX_MANIFEST}",
                        directory.display()
                    ),
                ),
            }
        })
        .collect()
}

/// Does the newest release tag this checkout can reach agree with the binary's version?
///
/// A `warn` either way it disagrees, never a `fail`, and the reason is in the report's own words:
/// an adopting repository's tags are **its** tags, so a disagreement there says nothing about the
/// binary. Inside this repository it says something sharp — `aep --version` prints the workspace
/// version, so while these disagree the binary cannot say which build it is, which is how a stale
/// install writes nothing and looks like it worked (`cargo xtask version` is the gate that refuses
/// that, and it is a gate rather than a preflight because it answers for one repository).
fn release_tag(root: &Path) -> Check {
    if git(root, &["rev-parse", "--is-inside-work-tree"])
        .is_none_or(|answer| answer.trim() != "true")
    {
        return Check::new(
            RELEASE_TAG,
            Status::Warn,
            format!(
                "{} is not a Git checkout, so there is no tag to compare version {VERSION} against",
                root.display()
            ),
        );
    }
    match newest_bare_version_tag(root) {
        None => Check::new(
            RELEASE_TAG,
            Status::Warn,
            format!(
                "no bare-version tag is reachable from HEAD, so there is nothing to compare \
                 version {VERSION} against — `git fetch --tags` first"
            ),
        ),
        Some(tag) if tag == VERSION => Check::new(
            RELEASE_TAG,
            Status::Ok,
            format!("the newest bare-version tag reachable from HEAD is {tag}, this binary's"),
        ),
        Some(tag) => Check::new(
            RELEASE_TAG,
            Status::Warn,
            format!(
                "the newest bare-version tag reachable from HEAD is {tag} and this binary is \
                 {VERSION}; expected when the checkout is not AEP's own, and a stale install when \
                 it is"
            ),
        ),
    }
}

/// The newest bare-version tag reachable from `HEAD`.
///
/// The listing `cargo xtask version` makes — `--merged HEAD`, `--sort=-v:refname`, and bare-version
/// tags only, because the pre-0.12.0 slugged form sorts above a plain number that is actually newer.
/// Spelled again here rather than called: `xtask` is a binary in this workspace, not a library, and
/// a shipped verb may not depend on a development task.
fn newest_bare_version_tag(root: &Path) -> Option<String> {
    git(root, &["tag", "--list", "--merged", "HEAD", "--sort=-v:refname"])?
        .lines()
        .map(str::trim)
        .find(|tag| is_bare_version(tag))
        .map(str::to_owned)
}

/// `1.2.3` and not `v1.2.3`, `0.11.0-plans` or an empty line.
fn is_bare_version(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|character| character.is_ascii_digit()))
}

/// Runs `git` in `root` and returns its standard output, or `None` when it fails.
///
/// A failure is not an error: every question asked here has *no answer* as an ordinary outcome — no
/// tags, no commits, not a work tree, no `git` on the machine at all — and the shape of that answer
/// is a `warn` line, not a stopped command. Nothing passed to it reaches a network.
fn git(root: &Path, arguments: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tag list is filtered by shape, and the shape is what decides which tag is *newest*.
    ///
    /// The mutation invariant 15 asks for is in the second half: a filter that accepted everything
    /// would answer `v0.42.0`, which sorts above `0.41.0` under `-v:refname` and is not a release
    /// tag of this convention at all.
    #[test]
    fn a_slugged_or_prefixed_tag_is_not_a_bare_version_and_cannot_win() {
        assert!(is_bare_version("0.41.0"), "the convention's own form");
        assert!(is_bare_version("1"), "a single component is still bare");
        for rejected in ["v0.42.0", "0.11.0-plans", "0..1", "", "0.41.0-rc1"] {
            assert!(
                !is_bare_version(rejected),
                "`{rejected}` is not a bare-version tag, and accepting it would let it sort above \
                 a plain number that is actually newer"
            );
        }
    }

    /// The cache path is derived from the repository locator, not from the revision alone, so two
    /// repositories pinned to the same commit id do not share a snapshot.
    #[test]
    fn a_snapshot_path_is_addressed_by_repository_and_revision_together() {
        let revision = "0".repeat(40);
        let one = ProtocolSource::parse(format!("git+https://example.test/one#{revision}"))
            .expect("a pinned locator");
        let two = ProtocolSource::parse(format!("git+https://example.test/two#{revision}"))
            .expect("a pinned locator");
        let (ProtocolSource::Git(one), ProtocolSource::Git(two)) = (one, two) else {
            panic!("both parse as Git sources");
        };

        let cache = Path::new("/cache");
        let one = snapshot_path(cache, &one);
        let two = snapshot_path(cache, &two);
        assert_ne!(
            one, two,
            "two repositories pinned to one commit must not share a snapshot directory"
        );
        assert!(
            one.ends_with(format!("snapshots/{revision}")),
            "the revision is the leaf, as `aep-project` writes it: {}",
            one.display()
        );
    }

    /// A directory with no manifest is named as the reason, not counted silently.
    #[test]
    fn a_plugin_directory_without_a_manifest_fails_naming_both_manifests_it_looked_for() {
        let checks = plugin_directories(&[PathBuf::from(env!("CARGO_MANIFEST_DIR"))]);
        assert_eq!(checks.len(), 1, "one directory, one line");
        assert_eq!(checks[0].status, Status::Fail);
        assert!(
            checks[0].detail.contains(CLAUDE_MANIFEST) && checks[0].detail.contains(CODEX_MANIFEST),
            "the line says what it looked for: {}",
            checks[0].detail
        );
    }
}
