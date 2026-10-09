//! What a project says about itself.
//!
//! A project adopting AEP keeps one small file — `.engineering/project.yaml` — that names the
//! protocol it runs under, the profile it uses, and which protocol source supplies its documents.
//! Everything else is discovered from it.
//!
//! ```yaml
//! version: aep.project/5
//! planning_scope: payments
//! protocol: adp/1
//! profile: development.standard
//! protocols: git+ssh://git@github.com/beyond10x/aep.git#0123456789abcdef0123456789abcdef01234567
//! artifacts: artifacts.yaml
//! task: task.yaml
//! schemas: schemas
//! ```
//!
//! # Why this file is deliberately thin
//!
//! It points; it does not duplicate. A project that restated its principles here would have two
//! copies of its rules and no way to tell which one was in force. It may add governing documents of
//! its own under `.engineering/principles/` and `.engineering/profiles/`, and product or research
//! contracts under the JSON Schema registry named by `schemas`. Each remains in its own validated
//! format; the project file only locates them.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::{ValidationCode, ValidationError, ValidationErrors};
use crate::ids::ProviderId;
use crate::time::{CivilDate, Timestamp};
use crate::version::{ProfileVersionedRef, ProtocolRef};

/// The default name of the directory a project keeps its machine-readable metadata in.
///
/// A default, not a fixed name: `AEP_PROJECT_DIR` renames it, and a repository whose team already
/// calls this directory something else is discovered under the name they use. The variable is read
/// once per process at the discovery edge — `aep_project::project::project_directory` — because this
/// crate reads no ambient state, so the constant here is what that edge falls back to.
pub const PROJECT_DIRECTORY: &str = ".engineering";
/// The file naming the protocol, the profile and where the documents are.
pub const PROJECT_FILE: &str = "project.yaml";
/// The Markdown journal layout (one store-wide `journal.jsonl`), which this build refuses to open.
///
/// Only `aep plan store migrate git` reads such a store, to rewrite it as [`PROJECT_VERSION_V5`].
pub const PROJECT_VERSION_V1: &str = "aep.project/1";
/// The command that moves an `aep.project/1` store to `aep.project/5`, with this build.
pub const MIGRATE_GIT_COMMAND: &str = "aep plan store migrate git --verify";
/// The event-log store versions this build recognises only to refuse.
pub const EVENT_LOG_PROJECT_VERSIONS: [&str; 3] =
    ["aep.project/2", "aep.project/3", "aep.project/4"];
/// Planning authority as plain Markdown files version control merges.
///
/// See `docs/design/git-native-planning-store-v0.1.md`.
pub const PROJECT_VERSION_V5: &str = "aep.project/5";
/// The directory, relative to `.engineering`, an `aep.project/5` store keeps its artifacts in.
pub const GIT_PLANNING_DIRECTORY: &str = "planning";
/// The directory, relative to `.engineering`, an `aep.project/5` store keeps its evidence in.
pub const GIT_EVIDENCE_DIRECTORY: &str = "evidence";

/// The rule an `aep.project/5` `planning_scope` is held to, as refusals state it.
///
/// "Whitespace" is Unicode whitespace (`char::is_whitespace`), so a value of only U+3000 is
/// refused although its bytes are not ASCII whitespace; the design names a non-whitespace
/// *character* (`docs/design/planning-store-selection-and-commands-v0.1.md`, `AuthorityValueV1`).
pub const PLANNING_SCOPE_RULE: &str = "1..=255 UTF-8 bytes including a non-whitespace character";

/// Whether `value` fits [`PLANNING_SCOPE_RULE`]. The value is never trimmed: it is written and
/// compared exactly as given.
#[must_use]
pub fn planning_scope_fits(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 255
}

/// The refusal for an event-log store (`aep.project/2`–`/4`), naming how to migrate it.
#[must_use]
pub fn event_log_store_refusal(version: &str) -> String {
    format!(
        "this store is an event-log store ({version}); AEP no longer reads it — migrate it with \
         `cargo install --git https://github.com/beyond10x/aep --rev 9c0f1da44429ff935fa0b2d743457945d51e1c51 aep-cli` \
         then `aep plan store migrate git --verify`"
    )
}

/// The refusal for a Markdown journal store (`aep.project/1`), naming how to migrate it.
///
/// Unlike an event-log store, this build still migrates one: the command runs here.
#[must_use]
pub fn journal_store_refusal() -> String {
    format!(
        "this store is `{PROJECT_VERSION_V1}`, the Markdown journal layout, which AEP no longer \
         opens — migrate it to `{PROJECT_VERSION_V5}` with `{MIGRATE_GIT_COMMAND}`"
    )
}

/// Which closed project document reader accepted the selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectVersion {
    /// The plan is kept in the store `store:` selects: the Git-native Markdown files by default,
    /// or a `SQLite` or `PostgreSQL` database.
    #[serde(rename = "aep.project/5")]
    V5,
}

impl ProjectVersion {
    /// The canonical document spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V5 => PROJECT_VERSION_V5,
        }
    }
}

/// The protocol documents a project adopts, before the engine resolves them to a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolSource {
    /// A filesystem tree. Relative paths are resolved from the project directory.
    Path(PathBuf),
    /// An immutable revision of a Git repository.
    Git(GitProtocolSource),
}

impl ProtocolSource {
    /// Parses a project file's scalar source locator.
    pub fn parse(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !value.starts_with("git+") {
            if value.contains("://") {
                return Err(ValidationError::new(
                    ValidationCode::TypeMismatch,
                    "project.protocols",
                    format!("`{value}` uses an unsupported source scheme"),
                )
                .with_hint(
                    "use a relative filesystem path or a pinned git+ssh://, git+https://, or git+file:// locator",
                ));
            }
            if let Some(reason) = absolute_path_reason(&value) {
                return Err(ValidationError::new(
                    ValidationCode::TypeMismatch,
                    "project.protocols",
                    format!("`{value}` is an absolute path ({reason})"),
                )
                .with_hint(
                    "use a path relative to the .engineering directory, or a pinned git+ssh://, \
                     git+https://, or git+file:// locator — an absolute path names a place on one \
                     machine, so the project file says something different on every other one and \
                     nothing at all in CI",
                ));
            }
            return Ok(Self::Path(PathBuf::from(value)));
        }

        let (repository, revision) = value.rsplit_once('#').ok_or_else(|| {
            ValidationError::new(
                ValidationCode::TypeMismatch,
                "project.protocols",
                "a Git protocol source has no revision after `#`",
            )
            .with_hint(
                "pin the repository to its full 40-character commit id so one project file always means one document tree",
            )
        })?;
        if !(repository.starts_with("git+ssh://")
            || repository.starts_with("git+https://")
            || repository.starts_with("git+file://"))
        {
            return Err(ValidationError::new(
                ValidationCode::TypeMismatch,
                "project.protocols",
                format!("`{repository}` is not a supported Git repository locator"),
            )
            .with_hint("use git+ssh://, git+https://, or git+file://"));
        }
        if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ValidationError::new(
                ValidationCode::TypeMismatch,
                "project.protocols",
                format!("`{revision}` is not a full Git commit id"),
            )
            .with_hint(
                "use the full 40 hexadecimal characters, not a branch, tag, or abbreviated id",
            ));
        }

        Ok(Self::Git(GitProtocolSource {
            repository: repository.to_owned(),
            revision: revision.to_ascii_lowercase(),
        }))
    }
}

/// Why a protocol source path is absolute, if it is.
///
/// A project file is read on every machine that checks the repository out, and a path rooted at `/`
/// or at a drive letter is true on exactly one of them. The refusal is here, in the one reader every
/// command goes through, rather than in the verb that writes the file — a file hand-edited past
/// `aep plan reverse init` has to fail the same way, and it is `resolve`, `evaluate` and `artifact`
/// that would otherwise carry a machine-local path into a CI run.
///
/// `~` is refused with the others and for a sharper reason: nothing here expands it, so `~/tree` is
/// a directory literally named `~`, and the failure it produces otherwise names a path nobody wrote.
///
/// Checked by spelling rather than by [`std::path::Path::is_absolute`], which answers for the
/// platform the check happens to run on: a Unix build would accept `C:\\tree` and a Windows build
/// would accept `/tree`, so the same project file would validate on one machine and not the other —
/// which is the failure this refusal exists to prevent, one level up.
fn absolute_path_reason(value: &str) -> Option<&'static str> {
    if value.starts_with('/') || value.starts_with('\\') {
        return Some("it is rooted at the filesystem root");
    }
    if value.starts_with('~') {
        return Some("nothing here expands `~`, so it names a directory called `~`");
    }
    let mut characters = value.chars();
    if let (Some(drive), Some(':'), Some(separator)) =
        (characters.next(), characters.next(), characters.next())
    {
        if drive.is_ascii_alphabetic() && (separator == '/' || separator == '\\') {
            return Some("it is rooted at a drive letter");
        }
    }
    None
}

impl Default for ProtocolSource {
    fn default() -> Self {
        Self::Path(PathBuf::from(".."))
    }
}

impl fmt::Display for ProtocolSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "{}", path.display()),
            Self::Git(source) => write!(f, "{}#{}", source.repository, source.revision),
        }
    }
}

impl serde::Serialize for ProtocolSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// A Git repository and the exact commit whose tree is adopted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitProtocolSource {
    repository: String,
    revision: String,
}

impl GitProtocolSource {
    /// The configured repository locator, including the `git+` source marker.
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// The repository URL understood by Git itself.
    pub fn git_url(&self) -> &str {
        self.repository
            .strip_prefix("git+")
            .expect("Git protocol sources are constructed only from git+ locators")
    }

    /// The full immutable commit id.
    pub fn revision(&self) -> &str {
        &self.revision
    }
}

/// Resolved filesystem locations for everything a loaded project uses.
///
/// The protocol source is materialized before this value is built, so a consumer never has to treat
/// a repository locator as if it were a path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProjectPaths {
    /// The protocol document tree.
    pub protocols: PathBuf,
    /// The artifact manifest.
    pub artifacts: PathBuf,
    /// The task being worked on.
    pub task: PathBuf,
    /// Where an execution's state is kept between runs.
    pub state: PathBuf,
    /// Project-local principles, merged over the protocol tree's.
    pub principles: PathBuf,
    /// Project-local profiles.
    pub profiles: PathBuf,
    /// Project-owned JSON Schema contracts.
    pub schemas: PathBuf,
}

/// Project-owned paths before they are resolved from `.engineering/`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProjectLocalPaths {
    /// The artifact manifest.
    pub artifacts: PathBuf,
    /// The task being worked on.
    pub task: PathBuf,
    /// Where an execution's state is kept between runs.
    pub state: PathBuf,
    /// Project-local principles, merged over the protocol tree's.
    pub principles: PathBuf,
    /// Project-local profiles.
    pub profiles: PathBuf,
    /// Project-owned JSON Schema contracts.
    pub schemas: PathBuf,
}

impl Default for ProjectLocalPaths {
    fn default() -> Self {
        Self {
            artifacts: PathBuf::from("artifacts.yaml"),
            task: PathBuf::from("task.yaml"),
            state: PathBuf::from("state.yaml"),
            principles: PathBuf::from("principles"),
            profiles: PathBuf::from("profiles"),
            schemas: PathBuf::from("schemas"),
        }
    }
}

impl ProjectLocalPaths {
    /// Resolves the project-owned paths and combines them with a materialized protocol tree.
    #[must_use]
    pub fn resolved(&self, engineering: &Path, protocols: PathBuf) -> ProjectPaths {
        ProjectPaths {
            protocols,
            artifacts: engineering.join(&self.artifacts),
            task: engineering.join(&self.task),
            state: engineering.join(&self.state),
            principles: engineering.join(&self.principles),
            profiles: engineering.join(&self.profiles),
            schemas: engineering.join(&self.schemas),
        }
    }
}

/// What a project says about itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProjectConfig {
    /// The closed project document reader that accepted this configuration.
    pub version: ProjectVersion,
    /// The protocol version it runs under.
    pub protocol: ProtocolRef,
    /// The profile it uses.
    pub profile: ProfileVersionedRef,
    /// A one-line description, for a report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The source of the governing protocol documents.
    pub protocols: ProtocolSource,
    /// Where project-owned inputs live.
    pub paths: ProjectLocalPaths,
    /// Where the plan is kept.
    pub store: StoreConfig,
    /// How to build a link for each external system an artifact may reference.
    ///
    /// ```yaml
    /// providers:
    ///   jira: https://acme.atlassian.net/browse/{key}
    ///   zendesk: https://acme.zendesk.com/agent/tickets/{key}
    /// ```
    ///
    /// **Configured once, not written per artifact.** A `refs:` entry carries a provider and a key
    /// and no URL, because a URL repeated into every file is the same fact copied N times and it is
    /// the copies that are wrong after a tracker migration. A provider nobody declares here is not
    /// an error: the reference still names the record, it just renders as text.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub providers: BTreeMap<ProviderId, String>,
    /// The first day, in UTC, from which a `review-result` must carry a `findings` block.
    ///
    /// Absent, a review may state its findings as prose only, as it always could. Set, `aep plan
    /// artifact new` refuses a review with no block unless `--prose-only <reason>` says why, and
    /// `aep plan artifact validate` counts one as a problem unless it predates this day, a later
    /// review carrying a block supersedes it, or it was recorded prose-only on purpose.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "date_as_written"
    )]
    pub findings_required_since: Option<CivilDate>,
}

/// The project key that opts a store in to requiring a `findings` block on every review.
pub const FINDINGS_REQUIRED_SINCE: &str = "findings_required_since";

/// A date as the project file spells it, `YYYY-MM-DD`, rather than as its three numbers.
#[allow(clippy::ref_option)] // `serialize_with` hands the field over by reference, as it is.
fn date_as_written<S: serde::Serializer>(
    date: &Option<CivilDate>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match date {
        Some(date) => serializer.serialize_some(&date.to_string()),
        None => serializer.serialize_none(),
    }
}

impl ProjectConfig {
    /// The instant `findings_required_since` starts at: midnight UTC of its date.
    ///
    /// The comparison a reader makes is against an instant, and the date is a day; this is the one
    /// place the two meet, so "before the opt-in" means the same thing to every reader. Pure
    /// calendar arithmetic, no clock (invariant *Decisions are deterministic*).
    #[must_use]
    pub fn findings_required_from(&self) -> Option<Timestamp> {
        self.findings_required_since.map(CivilDate::to_timestamp)
    }

    /// The URL for `reference`, when this project declares how to build one.
    ///
    /// `None` for an undeclared provider, deliberately: an approximate link is worse than none,
    /// because a reader cannot tell by looking that it lands in the wrong place.
    pub fn link(&self, reference: &crate::artifact::ExternalRef) -> Option<String> {
        self.providers
            .get(&reference.provider)
            .map(|pattern| pattern.replace(PROVIDER_KEY, &reference.reference))
    }
}

impl fmt::Display for ProjectConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} under {}", self.profile, self.protocol)
    }
}

/// Where a project keeps its plan, as written in an `aep.project/5` `project.yaml`.
///
/// ```yaml
/// store: { git: {} }                          # the default: `.engineering/planning/`
/// store: { sqlite: { path: plan.sqlite3 } }   # one file, relative to `.engineering/`
/// store: { postgres: { url: "postgres://…" } }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, schemars::JsonSchema)]
#[serde(untagged)]
pub enum RawStore {
    /// A bare word. None is a store in `aep.project/5`; read so the refusal can name the forms.
    Named(String),
    /// `git: {}`: the Markdown files under `planning` are the authority.
    Git {
        /// Carries no fields in `aep.project/5`.
        git: RawGit,
    },
    /// `sqlite: { path: <path> }`.
    Sqlite {
        /// Where the database file is.
        sqlite: RawSqlite,
    },
    /// `postgres: { url: <url> }`.
    Postgres {
        /// Which database to connect to.
        postgres: RawPostgres,
    },
    /// `eventlog: {…}`, an event-log store selector. Read only so the document reaches its
    /// refusal, which names how to migrate it; its contents are not interpreted.
    Eventlog {
        /// The selector as written.
        eventlog: serde_json::Value,
    },
}

/// The `git` store selector: empty in `aep.project/5`, and any key is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawGit {}

/// The `sqlite` store selector.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawSqlite {
    /// The database file, relative to `.engineering/`.
    pub path: PathBuf,
}

/// The `postgres` store selector.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawPostgres {
    /// A libpq connection string or URL.
    pub url: String,
}

/// The forms a store selector takes, for a refusal.
const STORE_FORMS: &str = "`git: {}`, `sqlite: { path: <path> }` or `postgres: { url: <url> }`";

/// Hand-written rather than `#[serde(untagged)]`, for the refusal's sake: an untagged enum that
/// fails to match reports *"did not match any variant"* and loses the reason, and the reason is the
/// whole point — a `sqlite` selector with a misspelt key must be refused **naming the key**.
impl<'de> serde::Deserialize<'de> for RawStore {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(name) => Ok(Self::Named(name)),
            serde_json::Value::Object(map) if map.len() == 1 => {
                let (key, inner) = map.into_iter().next().expect("one entry");
                match key.as_str() {
                    "git" => serde_json::from_value::<RawGit>(inner)
                        .map(|git| Self::Git { git })
                        .map_err(|error| D::Error::custom(format!("store.git: {error}"))),
                    "sqlite" => serde_json::from_value::<RawSqlite>(inner)
                        .map(|sqlite| Self::Sqlite { sqlite })
                        .map_err(|error| D::Error::custom(format!("store.sqlite: {error}"))),
                    "postgres" => serde_json::from_value::<RawPostgres>(inner)
                        .map(|postgres| Self::Postgres { postgres })
                        .map_err(|error| D::Error::custom(format!("store.postgres: {error}"))),
                    "eventlog" => Ok(Self::Eventlog { eventlog: inner }),
                    other => Err(D::Error::custom(format!(
                        "`{other}` is not a store form; write {STORE_FORMS}"
                    ))),
                }
            }
            other => Err(D::Error::custom(format!(
                "a store is {STORE_FORMS}, not {other}"
            ))),
        }
    }
}

/// Where a project keeps its plan, validated.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoreConfig {
    /// The Markdown files under `planning` are the authority and evidence records live under
    /// `evidence`; version control merges both. The default.
    Git {
        /// Artifact directory, relative to `.engineering` until resolved.
        planning: PathBuf,
        /// Evidence directory, relative to `.engineering` until resolved.
        evidence: PathBuf,
    },
    /// One `SQLite` file, at a path relative to `.engineering/` until resolved.
    Sqlite {
        /// The database file.
        path: PathBuf,
    },
    /// A `PostgreSQL` database the caller connects to.
    Postgres {
        /// A libpq connection string or URL.
        url: String,
    },
}

impl StoreConfig {
    /// The Git-native store at its fixed paths, relative to `.engineering`.
    #[must_use]
    pub fn git() -> Self {
        Self::Git {
            planning: PathBuf::from(GIT_PLANNING_DIRECTORY),
            evidence: PathBuf::from(GIT_EVIDENCE_DIRECTORY),
        }
    }

    /// The same configuration with every relative file path resolved against `engineering`.
    #[must_use]
    pub fn resolved(&self, engineering: &Path) -> Self {
        match self {
            Self::Sqlite { path } => Self::Sqlite {
                path: engineering.join(path),
            },
            Self::Postgres { url } => Self::Postgres { url: url.clone() },
            Self::Git { planning, evidence } => Self::Git {
                planning: engineering.join(planning),
                evidence: engineering.join(evidence),
            },
        }
    }
}

impl RawStore {
    /// Validates a store selector, accumulating every refusal under `at`.
    fn validate(self, at: &str, errors: &mut ValidationErrors) -> StoreConfig {
        match self {
            Self::Git { git: RawGit {} } => StoreConfig::git(),
            Self::Sqlite { sqlite } => {
                if let Some(reason) = absolute_path_reason(&sqlite.path.to_string_lossy()) {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::TypeMismatch,
                            format!("{at}.sqlite.path"),
                            format!("`{}` is an absolute path ({reason})", sqlite.path.display()),
                        )
                        .with_hint("a store path is relative to `.engineering`, like every other"),
                    );
                }
                StoreConfig::Sqlite { path: sqlite.path }
            }
            Self::Postgres { postgres } => {
                if postgres.url.trim().is_empty() {
                    errors.push(ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("{at}.postgres.url"),
                        "a Postgres store needs a connection URL",
                    ));
                }
                StoreConfig::Postgres { url: postgres.url }
            }
            Self::Named(name) => {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        at,
                        format!("`{name}` is not a store; write {STORE_FORMS}"),
                    )
                    .with_hint(format!(
                        "`{PROJECT_VERSION_V5}` has no bare-word store; the Markdown journal \
                         layout of `{PROJECT_VERSION_V1}` is migrated with `{MIGRATE_GIT_COMMAND}`"
                    )),
                );
                StoreConfig::git()
            }
            Self::Eventlog { .. } => {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("{at}.eventlog"),
                        format!(
                            "`eventlog` selects an event-log store, which `{PROJECT_VERSION_V5}` \
                             does not have"
                        ),
                    )
                    .with_hint(
                        "migrate the store to `aep.project/5` with the release that still reads it",
                    ),
                );
                StoreConfig::git()
            }
        }
    }
}

/// `aep.project/5`: `planning_scope` stays required; the two event-log identities are refused,
/// because they name an event-log tenant and stream this store does not have.
fn validate_store(
    raw_store: Option<RawStore>,
    planning_scope: Option<&str>,
    planning_tenant: Option<&str>,
    planning_identity: Option<&str>,
    errors: &mut ValidationErrors,
) -> StoreConfig {
    match planning_scope {
        Some(value) if planning_scope_fits(value) => {}
        Some(_) => errors.push(ValidationError::new(
            ValidationCode::TypeMismatch,
            "project.planning_scope",
            format!("`planning_scope` must contain {PLANNING_SCOPE_RULE}"),
        )),
        None => errors.push(ValidationError::new(
            ValidationCode::TypeMismatch,
            "project.planning_scope",
            format!("`{PROJECT_VERSION_V5}` requires `planning_scope`"),
        )),
    }
    for (field, value) in [
        ("planning_tenant", planning_tenant),
        ("planning_identity", planning_identity),
    ] {
        if value.is_some() {
            errors.push(
                ValidationError::new(
                    ValidationCode::TypeMismatch,
                    format!("project.{field}"),
                    format!(
                        "`{field}` names an event-log authority, which `{PROJECT_VERSION_V5}` does not have"
                    ),
                )
                .with_hint("remove the field; a planning store is identified by `planning_scope` alone"),
            );
        }
    }
    raw_store.map_or_else(StoreConfig::git, |store| {
        store.validate("project.store", errors)
    })
}

/// A project configuration document, as parsed.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawProjectConfig {
    /// The format version.
    #[serde(default = "default_version")]
    pub version: String,
    /// The protocol version this project runs under.
    pub protocol: ProtocolRef,
    /// The profile it uses.
    pub profile: ProfileVersionedRef,
    /// A one-line description.
    #[serde(default)]
    pub summary: Option<String>,
    /// A local tree path or a pinned `git+ssh://`, `git+https://`, or `git+file://` repository.
    #[serde(default)]
    pub protocols: Option<String>,
    /// Where the artifact manifest is.
    #[serde(default)]
    pub artifacts: Option<PathBuf>,
    /// Where the task document is.
    #[serde(default)]
    pub task: Option<PathBuf>,
    /// Where execution state is kept.
    #[serde(default)]
    pub state: Option<PathBuf>,
    /// Where project-local principles are.
    #[serde(default)]
    pub principles: Option<PathBuf>,
    /// Where project-local profiles are.
    #[serde(default)]
    pub profiles: Option<PathBuf>,
    /// Where project-owned JSON Schema contracts are.
    #[serde(default)]
    pub schemas: Option<PathBuf>,
    /// Where the plan is kept. Absent means `git: {}`.
    ///
    /// Held as written and read as a [`RawStore`] only after the version is accepted, so a `/1`
    /// document whose selector this build no longer spells still reaches the refusal naming its
    /// migration.
    #[serde(default)]
    #[schemars(with = "Option<RawStore>")]
    pub store: Option<serde_json::Value>,
    /// Provider-complete logical scope, required by `aep.project/5`.
    #[serde(default)]
    pub planning_scope: Option<String>,
    /// An event-log tenant identity. Refused: no store this build reads has one.
    #[serde(default)]
    pub planning_tenant: Option<String>,
    /// An event-log stream identity. Refused: no store this build reads has one.
    #[serde(default)]
    pub planning_identity: Option<String>,
    /// A URL pattern per external system, each carrying `{key}`.
    #[serde(default)]
    pub providers: BTreeMap<String, String>,
    /// The first day, `YYYY-MM-DD` in UTC, from which a `review-result` must carry a `findings`
    /// block. Text until validated, so a value that is not a calendar date is refused by the key's
    /// name beside every other defect. Written with no value (empty, `~` or `null`), it is refused
    /// rather than read as absent.
    #[serde(default, deserialize_with = "written_even_when_null")]
    #[schemars(with = "String")]
    pub findings_required_since: Option<Written>,
}

/// A present key's value as written, keeping a key written with a null value apart from an absent
/// one (which is `None` beside it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// The key with a value.
    Text(String),
    /// The key written with no value: empty, `~` or `null`, all YAML's null.
    Null,
}

/// Reads a key that is present, whatever its value.
///
/// Serde reads a null as an absent `Option`; read here, a present key is always `Some`, and an
/// absent one stays `None` by `#[serde(default)]`.
fn written_even_when_null<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Written>, D::Error> {
    <Option<String> as serde::Deserialize>::deserialize(deserializer)
        .map(|value| Some(value.map_or(Written::Null, Written::Text)))
}

/// The placeholder a provider's URL pattern must carry.
pub const PROVIDER_KEY: &str = "{key}";

/// Serde default for the format version.
fn default_version() -> String {
    PROJECT_VERSION_V1.to_owned()
}

impl TryFrom<RawProjectConfig> for ProjectConfig {
    type Error = ValidationErrors;

    #[allow(clippy::too_many_lines)] // One validation pass accumulates every project-field defect.
    fn try_from(raw: RawProjectConfig) -> Result<Self, Self::Error> {
        let mut errors = ValidationErrors::new();

        // An event-log store is refused alone: its other fields describe a store this build does
        // not read, so a refusal of each of them would only bury the one that says what to do.
        if EVENT_LOG_PROJECT_VERSIONS.contains(&raw.version.as_str()) {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedProtocolVersion,
                    "project.version",
                    event_log_store_refusal(&raw.version),
                )
                .with_hint("the migration writes an `aep.project/5` store this build reads"),
            );
            return Err(errors);
        }
        // The Markdown journal layout is refused alone too, and for the same reason.
        if raw.version == PROJECT_VERSION_V1 {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedProtocolVersion,
                    "project.version",
                    journal_store_refusal(),
                )
                .with_hint(format!(
                    "a `{PROJECT_VERSION_V1}` project whose `store:` is SQLite or Postgres is \
                     rewritten by hand as `{PROJECT_VERSION_V5}` with a `planning_scope` and \
                     `store: {{ sqlite: {{ path: <path> }} }}` or \
                     `store: {{ postgres: {{ url: <url> }} }}`"
                )),
            );
            return Err(errors);
        }

        let version = if raw.version == PROJECT_VERSION_V5 {
            ProjectVersion::V5
        } else {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedProtocolVersion,
                    "project.version",
                    format!(
                        "this build reads `{PROJECT_VERSION_V5}`, not `{}`",
                        raw.version
                    ),
                )
                .with_hint("upgrade the tooling rather than reinterpreting the document"),
            );
            ProjectVersion::V5
        };

        let protocols = match raw.protocols {
            Some(value) => match ProtocolSource::parse(value) {
                Ok(source) => source,
                Err(error) => {
                    errors.push(error);
                    ProtocolSource::default()
                }
            },
            None => ProtocolSource::default(),
        };
        // A pattern with no `{key}` in it is the mistake that produces a link to a tracker's front
        // page for every artifact — it opens, so nobody notices it is the wrong page. Refused here
        // rather than at render time, where nothing is looking.
        let mut providers = BTreeMap::new();
        for (name, pattern) in raw.providers {
            let provider = match ProviderId::new(&name) {
                Ok(provider) => provider,
                Err(error) => {
                    errors.push(ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("project.providers.{name}"),
                        error.to_string(),
                    ));
                    continue;
                }
            };
            if !pattern.contains(PROVIDER_KEY) {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("project.providers.{name}"),
                        format!(
                            "`{pattern}` carries no `{PROVIDER_KEY}`, so every reference would \
                             render the same link"
                        ),
                    )
                    .with_hint("write the pattern as `https://…/browse/{key}`"),
                );
                continue;
            }
            providers.insert(provider, pattern);
        }

        let defaults = ProjectLocalPaths::default();
        let paths = ProjectLocalPaths {
            artifacts: raw.artifacts.unwrap_or(defaults.artifacts),
            task: raw.task.unwrap_or(defaults.task),
            state: raw.state.unwrap_or(defaults.state),
            principles: raw.principles.unwrap_or(defaults.principles),
            profiles: raw.profiles.unwrap_or(defaults.profiles),
            schemas: raw.schemas.unwrap_or(defaults.schemas),
        };

        // Paths *inside* the project must be relative. The separately typed protocol source may be
        // a path outside it or a repository locator.
        for (label, path) in [
            ("artifacts", &paths.artifacts),
            ("task", &paths.task),
            ("state", &paths.state),
            ("principles", &paths.principles),
            ("profiles", &paths.profiles),
            ("schemas", &paths.schemas),
        ] {
            // The same spelling-based rule `protocols` is held to, and deliberately not
            // `Path::is_absolute` — that answers for the platform the check runs on, so a Linux
            // build accepted `schemas: C:\registry` and a Windows one accepted `schemas:
            // /registry`. One project file, two verdicts, which is the failure the rule exists to
            // prevent.
            if let Some(reason) = absolute_path_reason(&path.to_string_lossy()) {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("project.{label}"),
                        format!("`{}` is an absolute path ({reason})", path.display()),
                    )
                    .with_hint(
                        "every path in a project file is relative to `.engineering`, so the \
                         repository can be cloned anywhere without editing them; `protocols` is \
                         held to the same rule and names an external tree with a pinned \
                         git+ssh://, git+https:// or git+file:// locator",
                    ),
                );
            }
        }

        let raw_store = match raw
            .store
            .map(serde_json::from_value::<RawStore>)
            .transpose()
        {
            Ok(store) => store,
            Err(error) => {
                errors.push(ValidationError::new(
                    ValidationCode::TypeMismatch,
                    "project.store",
                    error.to_string(),
                ));
                None
            }
        };
        let store = validate_store(
            raw_store,
            raw.planning_scope.as_deref(),
            raw.planning_tenant.as_deref(),
            raw.planning_identity.as_deref(),
            &mut errors,
        );

        // A day, and only a day: a value read as some other day, or as no opt-in, would move the
        // line every review is measured against without anybody having written that line.
        let findings_required_since = raw.findings_required_since.and_then(|written| {
            let Written::Text(value) = written else {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("project.{FINDINGS_REQUIRED_SINCE}"),
                        format!(
                            "`{FINDINGS_REQUIRED_SINCE}` is written with no value (empty, `~` or \
                             `null`); it is a calendar date written YYYY-MM-DD, and a key left \
                             without one is not read as no opt-in"
                        ),
                    )
                    .with_hint(
                        "write the first day a review-result must carry a `findings` block, such \
                         as `2026-10-01`, or remove the key to require none",
                    ),
                );
                return None;
            };
            match CivilDate::parse(&value) {
                Ok(date) => Some(date),
                Err(error) => {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::TypeMismatch,
                            format!("project.{FINDINGS_REQUIRED_SINCE}"),
                            format!(
                                "`{FINDINGS_REQUIRED_SINCE}: {value}` is not a calendar date \
                                     written YYYY-MM-DD: {error}"
                            ),
                        )
                        .with_hint(
                            "write the first day a review-result must carry a `findings` \
                                 block, such as `2026-10-01`; it is read as starting at midnight \
                                 UTC",
                        ),
                    );
                    None
                }
            }
        });

        let config = Self {
            version,
            protocol: raw.protocol,
            profile: raw.profile,
            summary: raw.summary,
            protocols,
            paths,
            store,
            providers,
            findings_required_since,
        };
        errors.into_result(config)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const BASE: &str = "protocol: adp/1\nprofile: development.standard\n";
    const V5: &str = "version: aep.project/5\nplanning_scope: aep\n";

    #[test]
    fn a_v5_project_that_names_no_store_keeps_its_plan_in_git_native_markdown() {
        let parsed = config(&format!("{V5}{BASE}")).expect("valid");
        assert_eq!(parsed.version, ProjectVersion::V5);
        assert_eq!(parsed.store, StoreConfig::git());
    }

    #[test]
    fn an_event_log_store_is_refused_alone_naming_how_to_migrate_it() {
        for (version, rest) in [
            (
                "aep.project/2",
                "planning_scope: aep.planning\nplanning_tenant: planning-main\n\
                 planning_identity: stream-01\n",
            ),
            (
                "aep.project/3",
                "planning_scope: aep.planning\nplanning_tenant: planning-main\n\
                 planning_identity: stream-01\n\
                 store:\n  eventlog:\n    path: state\n    projection: planning\n",
            ),
            (
                "aep.project/4",
                "planning_scope: ' '\nstore:\n  eventlog:\n    path: state\n    \
                 projection: planning\n    blobs: blobs\n",
            ),
        ] {
            let errors = raw_config(&format!("version: {version}\n{BASE}{rest}"))
                .expect_err("an event-log store is refused");
            let [error] = errors.as_slice() else {
                panic!("one refusal, not {errors:?}");
            };
            assert_eq!(error.code, ValidationCode::UnsupportedProtocolVersion);
            assert_eq!(error.location, "project.version");
            assert!(
                error.message.starts_with(&format!(
                    "this store is an event-log store ({version}); AEP no longer reads it"
                )),
                "{}",
                error.message
            );
            assert!(
                error
                    .message
                    .contains("aep plan store migrate git --verify"),
                "{}",
                error.message
            );
        }
    }

    #[test]
    fn a_v1_journal_store_is_refused_alone_naming_the_migration_this_build_runs() {
        for text in [
            format!("version: aep.project/1\n{BASE}"),
            // No `version` is the `/1` document it always was.
            BASE.to_owned(),
            format!("version: aep.project/1\n{BASE}store:\n  sqlite: plan.sqlite3\n"),
            format!("version: aep.project/1\n{BASE}store:\n  hybrid:\n    authority: local\n"),
        ] {
            let errors = raw_config(&text).expect_err("a /1 store is refused");
            let [error] = errors.as_slice() else {
                panic!("one refusal, not {errors:?}");
            };
            assert_eq!(error.code, ValidationCode::UnsupportedProtocolVersion);
            assert_eq!(error.location, "project.version");
            assert!(
                error.message.contains(PROJECT_VERSION_V1)
                    && error.message.contains(MIGRATE_GIT_COMMAND),
                "{}",
                error.message
            );
            assert!(
                error
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.contains("sqlite: { path: <path> }")),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_v5_project_selects_the_git_native_store_at_its_fixed_paths() {
        let parsed = config(&format!("{V5}{BASE}store:\n  git: {{}}\n")).expect("valid v5");
        assert_eq!(parsed.version, ProjectVersion::V5);
        assert_eq!(parsed.version.as_str(), PROJECT_VERSION_V5);
        assert_eq!(
            parsed.store,
            StoreConfig::Git {
                planning: PathBuf::from("planning"),
                evidence: PathBuf::from("evidence"),
            }
        );
        assert_eq!(
            parsed.store.resolved(Path::new("/repo/.engineering")),
            StoreConfig::Git {
                planning: PathBuf::from("/repo/.engineering/planning"),
                evidence: PathBuf::from("/repo/.engineering/evidence"),
            }
        );
    }

    #[test]
    fn v5_refuses_an_eventlog_store_and_eventlog_identities_naming_v5() {
        let errors = config(&format!(
            "{V5}{BASE}planning_tenant: planning-main\nplanning_identity: stream-01\n\
             store:\n  eventlog:\n    path: state\n    projection: planning\n"
        ))
        .expect_err("v5 has no Eventlog");
        for location in [
            "project.store.eventlog",
            "project.planning_tenant",
            "project.planning_identity",
        ] {
            let error = errors
                .as_slice()
                .iter()
                .find(|error| error.location == location)
                .unwrap_or_else(|| panic!("no refusal at {location}: {errors:?}"));
            assert!(
                error.message.contains(PROJECT_VERSION_V5),
                "{}",
                error.message
            );
        }
    }

    #[test]
    fn v5_requires_a_planning_scope() {
        let errors = config(&format!(
            "version: aep.project/5\n{BASE}store:\n  git: {{}}\n"
        ))
        .expect_err("scope is required");
        assert!(errors
            .as_slice()
            .iter()
            .any(|error| error.location == "project.planning_scope"
                && error.message.contains(PROJECT_VERSION_V5)));
    }

    /// U+3000 (ideographic space) is whitespace by Unicode although none of its bytes are ASCII
    /// whitespace: the rule counts characters, and the refusal says so.
    #[test]
    fn a_planning_scope_of_only_unicode_whitespace_is_refused_naming_a_non_whitespace_character() {
        for scope in ["\"\u{3000}\"", "\" \t\"", "\"\""] {
            let errors = config(&format!(
                "version: aep.project/5\nplanning_scope: {scope}\n{BASE}"
            ))
            .expect_err("a blank scope is refused");
            let error = errors
                .as_slice()
                .iter()
                .find(|error| error.location == "project.planning_scope")
                .unwrap_or_else(|| panic!("no planning_scope refusal for {scope}: {errors:?}"));
            assert_eq!(error.code, ValidationCode::TypeMismatch);
            assert!(
                error.message.contains("a non-whitespace character"),
                "{}",
                error.message
            );
        }
        assert!(planning_scope_fits("\u{3000}a"));
        assert!(planning_scope_fits(&"x".repeat(255)));
        assert!(!planning_scope_fits(&"x".repeat(256)));
    }

    #[test]
    fn an_unknown_key_under_store_git_is_refused() {
        let errors = config(&format!("{V5}{BASE}store:\n  git:\n    path: state\n"))
            .expect_err("store.git carries no fields in v5");
        let error = &errors.as_slice()[0];
        assert_eq!(error.location, "project.store");
        assert!(error.message.contains("store.git"), "{error}");
    }

    #[test]
    fn a_sqlite_store_is_a_relative_path_resolved_against_engineering() {
        let parsed = config(&format!(
            "{V5}{BASE}store:\n  sqlite:\n    path: plan.sqlite3\n"
        ))
        .expect("valid");
        assert_eq!(
            parsed.store.resolved(Path::new("/repo/.engineering")),
            StoreConfig::Sqlite {
                path: PathBuf::from("/repo/.engineering/plan.sqlite3")
            }
        );
        let absolute = config(&format!(
            "{V5}{BASE}store:\n  sqlite:\n    path: /var/plan.sqlite3\n"
        ))
        .expect_err("an absolute path is refused");
        assert_eq!(absolute.as_slice()[0].location, "project.store.sqlite.path");
    }

    #[test]
    fn a_postgres_store_carries_its_url_and_refuses_an_empty_one() {
        let parsed = config(&format!(
            "{V5}{BASE}store:\n  postgres:\n    url: postgres://db/plan\n"
        ))
        .expect("valid");
        assert_eq!(
            parsed.store,
            StoreConfig::Postgres {
                url: "postgres://db/plan".to_owned()
            }
        );
        let empty = config(&format!("{V5}{BASE}store:\n  postgres:\n    url: ' '\n"))
            .expect_err("an empty URL is refused");
        assert_eq!(empty.as_slice()[0].location, "project.store.postgres.url");
    }

    #[test]
    fn a_misspelt_database_selector_is_refused_naming_the_key() {
        for (text, named) in [
            (
                "store:\n  sqlite:\n    file: plan.sqlite3\n",
                "store.sqlite",
            ),
            ("store:\n  sqlite: plan.sqlite3\n", "store.sqlite"),
            (
                "store:\n  postgres:\n    dsn: postgres://db\n",
                "store.postgres",
            ),
            ("store:\n  hybrid: {}\n", "`hybrid` is not a store form"),
        ] {
            let errors = config(&format!("{V5}{BASE}{text}"))
                .expect_err("a selector of the wrong shape is refused");
            let error = &errors.as_slice()[0];
            assert_eq!(error.location, "project.store");
            assert!(error.message.contains(named), "{named}: {error}");
        }
    }

    #[test]
    fn a_bare_word_store_is_refused_in_v5() {
        let errors = config(&format!("{V5}{BASE}store: markdown\n"))
            .expect_err("`markdown` names the /1 layout");
        assert_eq!(errors.as_slice()[0].location, "project.store");
    }

    /// Parses `yaml` as it is.
    fn raw_config(yaml: &str) -> Result<ProjectConfig, ValidationErrors> {
        let raw: RawProjectConfig = serde_yaml::from_str(yaml).expect("document parses");
        ProjectConfig::try_from(raw)
    }

    /// Parses `yaml`, as an `aep.project/5` document when it names no version.
    fn config(yaml: &str) -> Result<ProjectConfig, ValidationErrors> {
        if yaml.contains("version:") {
            raw_config(yaml)
        } else {
            raw_config(&format!("{V5}{yaml}"))
        }
    }

    #[test]
    fn a_minimal_project_file_names_only_what_it_must() {
        let parsed = config(
            r"
protocol: adp/1
profile: development.standard
",
        )
        .expect("validates");

        assert_eq!(parsed.protocol.to_string(), "adp/1");
        assert_eq!(parsed.profile.to_string(), "development.standard");
        assert_eq!(parsed.paths.artifacts, PathBuf::from("artifacts.yaml"));
        assert_eq!(parsed.paths.schemas, PathBuf::from("schemas"));
        assert_eq!(parsed.protocols, ProtocolSource::Path(PathBuf::from("..")));
    }

    #[test]
    fn an_absolute_protocol_source_is_refused_however_it_is_spelt() {
        // A project file is read on every machine that checks the repository out. An absolute path
        // is true on exactly one of them, and in CI it is true on none — so this is refused at the
        // reader rather than left to fail later as a missing directory, where the message names a
        // path nobody on that machine wrote.
        for spelling in [
            "/srv/trees/aep",
            "~/trees/aep",
            "~",
            r"C:\trees\aep",
            r"D:/trees/aep",
            r"\\fileserver\trees",
        ] {
            let error = ProtocolSource::parse(spelling)
                .expect_err(&format!("`{spelling}` must be refused"));
            assert_eq!(error.code, ValidationCode::TypeMismatch, "{spelling}");
            assert!(
                error.message.contains("absolute path"),
                "`{spelling}` must be refused as absolute, not as something else: {}",
                error.message
            );
        }
    }

    #[test]
    fn a_relative_protocol_source_is_still_accepted() {
        // The rule is about a path being rooted somewhere only one machine has, not about paths.
        // `..` is what this repository's own project file uses.
        for spelling in ["..", ".", "../..", "vendor/protocols", "./tree"] {
            assert_eq!(
                ProtocolSource::parse(spelling).expect("a relative path is accepted"),
                ProtocolSource::Path(PathBuf::from(spelling)),
                "{spelling}"
            );
        }
    }

    #[test]
    fn a_pinned_git_source_may_carry_an_absolute_path_inside_its_locator() {
        // `git+file:///srv/mirror.git#<sha>` is absolute *inside a URL* and is not the thing being
        // refused: it names a repository and a commit, so what it resolves to is the same tree
        // everywhere the repository is reachable. Asserted because the check runs before the `git+`
        // branch is taken, and a careless tightening would break every file-backed fixture.
        let source = ProtocolSource::parse(
            "git+file:///srv/mirror/aep.git#0123456789abcdef0123456789abcdef01234567",
        )
        .expect("a pinned file-backed source is accepted");
        assert!(matches!(source, ProtocolSource::Git(_)));
    }

    #[test]
    fn paths_resolve_against_the_engineering_directory() {
        let parsed = config(
            r"
protocol: adp/1
profile: development.standard
protocols: ../../protocols
artifacts: graph.yaml
",
        )
        .expect("validates");

        assert_eq!(
            parsed.protocols,
            ProtocolSource::Path(PathBuf::from("../../protocols"))
        );
        let resolved = parsed.paths.resolved(
            Path::new("/work/payments/.engineering"),
            PathBuf::from("/work/payments/.engineering/../../protocols"),
        );
        assert_eq!(
            resolved.artifacts,
            PathBuf::from("/work/payments/.engineering/graph.yaml")
        );
        assert_eq!(
            resolved.protocols,
            PathBuf::from("/work/payments/.engineering/../../protocols")
        );
        assert_eq!(
            resolved.schemas,
            PathBuf::from("/work/payments/.engineering/schemas")
        );
    }

    /// The tree may still live outside the project — by a relative path, or by a pinned locator.
    ///
    /// This test asserted the opposite until 2026-08-25: `protocols: /opt/aep`
    /// validated, on the reading that where a tree sits is the adopter's business. It is, and that
    /// is not what the value decides. A project file is committed and read on every machine that
    /// checks the repository out, so an absolute path makes the file mean a different thing on each
    /// one and nothing at all in CI — the failure arriving as a missing directory naming a path
    /// nobody on that machine wrote. **This is a breaking change** for a project file that carries
    /// one; the two forms below are what it becomes.
    #[test]
    fn the_protocol_tree_may_live_outside_the_project_without_being_named_absolutely() {
        let by_relative_path = config(
            r"
protocol: adp/1
profile: development.standard
protocols: ../../aep
",
        )
        .expect("a relative path out of the project is allowed");
        assert_eq!(
            by_relative_path.protocols,
            ProtocolSource::Path(PathBuf::from("../../aep"))
        );

        let by_pinned_locator = config(
            r"
protocol: adp/1
profile: development.standard
protocols: git+https://example.com/aep.git#0123456789abcdef0123456789abcdef01234567
",
        )
        .expect("a pinned locator is allowed");
        assert!(matches!(
            by_pinned_locator.protocols,
            ProtocolSource::Git(_)
        ));

        let refused = config(
            r"
protocol: adp/1
profile: development.standard
protocols: /opt/aep
",
        )
        .expect_err("an absolute path is refused");
        assert!(format!("{refused:?}").contains("absolute path"));
    }

    #[test]
    fn a_git_protocol_source_is_a_repository_pinned_to_one_full_commit() {
        let revision = "0123456789abcdef0123456789abcdef01234567";
        let parsed = config(&format!(
            "protocol: adp/1\nprofile: development.standard\n\
             protocols: git+ssh://git@github.com/beyond10x/aep.git#{revision}\n"
        ))
        .expect("the pinned repository source validates");

        let ProtocolSource::Git(source) = parsed.protocols else {
            panic!("the repository locator was not reinterpreted as a path");
        };
        assert_eq!(
            source.repository(),
            "git+ssh://git@github.com/beyond10x/aep.git"
        );
        assert_eq!(source.git_url(), "ssh://git@github.com/beyond10x/aep.git");
        assert_eq!(source.revision(), revision);
    }

    #[test]
    fn a_git_protocol_source_without_an_immutable_revision_is_refused() {
        for protocols in [
            "git+ssh://git@github.com/beyond10x/aep.git",
            "git+ssh://git@github.com/beyond10x/aep.git#main",
        ] {
            let errors = config(&format!(
                "protocol: adp/1\nprofile: development.standard\nprotocols: {protocols}\n"
            ))
            .expect_err("a moving or absent Git revision is not a reproducible source");
            assert!(errors.to_string().contains("commit"), "{errors}");
        }
    }

    #[test]
    fn an_absolute_path_is_refused() {
        let errors = config(
            r"
protocol: adp/1
profile: development.standard
artifacts: /etc/engineering/artifacts.yaml
",
        )
        .expect_err("absolute path");
        assert!(
            errors.to_string().contains("cloned anywhere"),
            "the refusal must say why relative paths matter: {errors}"
        );
    }

    #[test]
    fn an_absolute_schema_registry_is_refused() {
        let errors = config(
            r"
protocol: adp/1
profile: development.standard
schemas: /etc/engineering/schemas
",
        )
        .expect_err("absolute schema registry");
        assert!(errors.to_string().contains("project.schemas"), "{errors}");
    }

    #[test]
    fn an_unknown_format_version_is_refused_rather_than_guessed() {
        let errors = config(
            r"
version: aep.project/9
protocol: adp/1
profile: development.standard
",
        )
        .expect_err("unknown version");
        assert!(errors.contains(ValidationCode::UnsupportedProtocolVersion));
    }

    #[test]
    fn an_unknown_key_is_refused_rather_than_ignored() {
        let raw: Result<RawProjectConfig, _> = serde_yaml::from_str(
            r"
protocol: adp/1
profile: development.standard
artefacts: graph.yaml
",
        );
        assert!(
            raw.is_err(),
            "a misspelled key that is silently ignored is a project pointing at nothing"
        );
    }

    /// `findings_required_since` is a calendar day as a person writes one, and nothing else: a
    /// month that does not exist, an unpadded field, a time of day or a word is refused, by the
    /// key's name, rather than read as some other day or as no opt-in at all.
    #[test]
    fn a_findings_required_since_that_is_not_a_calendar_date_is_refused_naming_the_key() {
        for written in [
            "2026-02-30",
            "2026-13-01",
            "2026-1-05",
            "2026-10-01T00:00:00Z",
            "yesterday",
            "\"\"",
        ] {
            let errors = config(&format!("{BASE}findings_required_since: {written}\n"))
                .expect_err("a value that is not a calendar date is refused");
            let error = errors
                .as_slice()
                .iter()
                .find(|error| error.location == "project.findings_required_since")
                .unwrap_or_else(|| {
                    panic!("no findings_required_since refusal for {written}: {errors:?}")
                });
            assert_eq!(error.code, ValidationCode::TypeMismatch, "{written}");
            assert!(
                error.message.contains("`findings_required_since")
                    && error.message.contains("calendar date"),
                "the refusal names the key and what it must be: {}",
                error.message
            );
        }
    }

    /// A key written with no value — empty, `~` or `null`, all YAML's null — is a key somebody
    /// wrote and left without its date, not an absent key: refused by the key's name, never read
    /// as no opt-in.
    #[test]
    fn a_findings_required_since_written_empty_or_null_is_refused_naming_the_key() {
        for written in ["", " ~", " null", " Null"] {
            let errors = config(&format!("{BASE}findings_required_since:{written}\n"))
                .expect_err("a key with no value is refused");
            let error = errors
                .as_slice()
                .iter()
                .find(|error| error.location == "project.findings_required_since")
                .unwrap_or_else(|| {
                    panic!("no findings_required_since refusal for {written:?}: {errors:?}")
                });
            assert_eq!(error.code, ValidationCode::TypeMismatch, "{written:?}");
            assert!(
                error.message.contains("`findings_required_since`")
                    && error.message.contains("no value"),
                "the refusal names the key and that it has no value: {}",
                error.message
            );
        }
    }

    /// The opt-in is a day, read in UTC: the first instant it covers is that day's midnight UTC,
    /// so a review recorded one millisecond earlier predates it and one recorded at midnight does
    /// not. Absent, nothing is required.
    #[test]
    fn findings_required_since_starts_at_midnight_utc_of_its_date() {
        let parsed = config(&format!("{BASE}findings_required_since: 2026-10-01\n"))
            .expect("a calendar date is accepted");
        assert_eq!(
            parsed
                .findings_required_since
                .as_ref()
                .map(ToString::to_string),
            Some("2026-10-01".to_owned())
        );
        let from = parsed
            .findings_required_from()
            .expect("an opted-in project names the instant findings are required from");
        assert_eq!(from.iso_8601(), "2026-10-01T00:00:00Z");
        assert_eq!(from.epoch_millis(), 1_790_812_800_000);

        let quoted = config(&format!("{BASE}findings_required_since: \"2026-10-01\"\n"))
            .expect("the quoted spelling is the same date");
        assert_eq!(quoted.findings_required_from(), Some(from));

        let absent = config(BASE).expect("the key is optional");
        assert_eq!(absent.findings_required_since, None);
        assert_eq!(absent.findings_required_from(), None);
    }
}
