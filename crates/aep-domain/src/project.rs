//! What a project says about itself.
//!
//! A project adopting AEP keeps one small file — `.engineering/project.yaml` — that names the
//! protocol it runs under, the profile it uses, and which protocol source supplies its documents.
//! Everything else is discovered from it.
//!
//! ```yaml
//! version: aep.project/1
//! protocol: adp/1
//! profile: development.standard
//! protocols: git+ssh://git@github.com/beyond10x/engineering-protocols.git#0123456789abcdef0123456789abcdef01234567
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

use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::{ValidationCode, ValidationError, ValidationErrors};
use crate::version::{ProfileVersionedRef, ProtocolRef};

/// The directory a project keeps its machine-readable metadata in.
pub const PROJECT_DIRECTORY: &str = ".engineering";
/// The file naming the protocol, the profile and where the documents are.
pub const PROJECT_FILE: &str = "project.yaml";
/// The format version this build reads.
pub const PROJECT_VERSION: &str = "aep.project/1";

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
/// `protocol reverse init` has to fail the same way, and it is `resolve`, `evaluate` and `artifact`
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
    #[serde(default)]
    pub store: StoreConfig,
}

impl fmt::Display for ProjectConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} under {}", self.profile, self.protocol)
    }
}

/// Where a project keeps its plan, as written in `project.yaml` (wave H, story 1).
///
/// ```yaml
/// store: markdown                    # the default: `.engineering/planning/`
/// store: { sqlite: plan.sqlite3 }    # one file, relative to `.engineering/`
/// store: { postgres: "postgres://…" }
/// store:
///   hybrid:
///     authority: local               # local | replica
///     read: local-first              # local-first | replica-first | replica-only
///     on_unreachable: refuse         # refuse | serve-stale
///     on_divergence: record          # refuse | record
///     local: markdown
///     replica: { sqlite: plan.sqlite3 }
/// ```
///
/// A hybrid's four policy words are **required** — the runtime's R-106 says a default here is a
/// policy nobody chose being applied to somebody's data — and a document missing one is refused
/// naming the word, by the parser.
#[derive(Debug, Clone, PartialEq, Eq, schemars::JsonSchema)]
#[serde(untagged)]
pub enum RawStore {
    /// `markdown`, the only bare word.
    Named(String),
    /// `sqlite: <path>`.
    Sqlite {
        /// The database file, relative to `.engineering/`.
        sqlite: PathBuf,
    },
    /// `postgres: <url>`.
    Postgres {
        /// A libpq connection string or URL.
        postgres: String,
    },
    /// `hybrid: {…}`.
    Hybrid {
        /// The composite's policy and its two halves.
        hybrid: Box<RawHybrid>,
    },
}

/// Hand-written rather than `#[serde(untagged)]`, for the refusal's sake: an untagged enum that
/// fails to match reports *"did not match any variant"* and loses the reason, and the reason is the
/// whole point — a hybrid missing `on_divergence` must be refused **naming `on_divergence`**.
impl<'de> serde::Deserialize<'de> for RawStore {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(name) => Ok(Self::Named(name)),
            serde_json::Value::Object(map) if map.len() == 1 => {
                let (key, inner) = map.into_iter().next().expect("one entry");
                match key.as_str() {
                    "sqlite" => serde_json::from_value(inner)
                        .map(|sqlite| Self::Sqlite { sqlite })
                        .map_err(|error| D::Error::custom(format!("store.sqlite: {error}"))),
                    "postgres" => serde_json::from_value(inner)
                        .map(|postgres| Self::Postgres { postgres })
                        .map_err(|error| D::Error::custom(format!("store.postgres: {error}"))),
                    "hybrid" => serde_json::from_value::<RawHybrid>(inner)
                        .map(|hybrid| Self::Hybrid {
                            hybrid: Box::new(hybrid),
                        })
                        .map_err(|error| D::Error::custom(format!("store.hybrid: {error}"))),
                    other => Err(D::Error::custom(format!(
                        "`{other}` is not a store form; write `markdown`, `sqlite: <path>`, \
                         `postgres: <url>` or `hybrid: {{…}}`"
                    ))),
                }
            }
            other => Err(D::Error::custom(format!(
                "a store is `markdown`, `sqlite: <path>`, `postgres: <url>` or `hybrid: {{…}}`, \
                 not {other}"
            ))),
        }
    }
}

/// A hybrid store as written: four policy words and two stores, none defaulted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawHybrid {
    /// Whose copy is the record of truth: `local` or `replica`.
    pub authority: String,
    /// Where a read goes first: `local-first`, `replica-first` or `replica-only`.
    pub read: String,
    /// What a read does when the replica does not answer: `refuse` or `serve-stale`.
    pub on_unreachable: String,
    /// What happens to a write that lost: `refuse` or `record`.
    pub on_divergence: String,
    /// The local half.
    pub local: RawStore,
    /// The replica.
    pub replica: RawStore,
}

/// Where a project keeps its plan, validated.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoreConfig {
    /// Markdown documents under `.engineering/planning/`. The default.
    #[default]
    Markdown,
    /// One SQLite file, at a path relative to `.engineering/`.
    Sqlite {
        /// The database file.
        path: PathBuf,
    },
    /// A PostgreSQL database the caller connects to.
    Postgres {
        /// A libpq connection string or URL.
        url: String,
    },
    /// Two stores and a declared rule for when they disagree.
    Hybrid {
        /// The four words, every one typed.
        policy: HybridPolicy,
        /// The local half.
        local: Box<StoreConfig>,
        /// The replica.
        replica: Box<StoreConfig>,
    },
}

impl StoreConfig {
    /// The same configuration with every relative file path resolved against `engineering`.
    #[must_use]
    pub fn resolved(&self, engineering: &Path) -> Self {
        match self {
            Self::Markdown => Self::Markdown,
            Self::Sqlite { path } => Self::Sqlite {
                path: engineering.join(path),
            },
            Self::Postgres { url } => Self::Postgres { url: url.clone() },
            Self::Hybrid {
                policy,
                local,
                replica,
            } => Self::Hybrid {
                policy: policy.clone(),
                local: Box::new(local.resolved(engineering)),
                replica: Box::new(replica.resolved(engineering)),
            },
        }
    }
}

/// A hybrid's policy: the runtime's four required words, as this configuration spells them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HybridPolicy {
    /// `local` or `replica`.
    pub authority: String,
    /// `local-first`, `replica-first` or `replica-only`.
    pub read: String,
    /// `refuse` or `serve-stale`.
    pub on_unreachable: String,
    /// `refuse` or `record`.
    pub on_divergence: String,
}

/// The words each policy field accepts.
const HYBRID_WORDS: &[(&str, &[&str])] = &[
    ("authority", &["local", "replica"]),
    ("read", &["local-first", "replica-first", "replica-only"]),
    ("on_unreachable", &["refuse", "serve-stale"]),
    ("on_divergence", &["refuse", "record"]),
];

impl RawStore {
    /// Validates a store configuration, accumulating every refusal under `at`.
    fn validate(self, at: &str, errors: &mut ValidationErrors) -> StoreConfig {
        match self {
            Self::Named(name) if name == "markdown" => StoreConfig::Markdown,
            Self::Named(name) => {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        at,
                        format!(
                            "`{name}` is not a store; write `markdown`, `sqlite: <path>`, \
                             `postgres: <url>` or `hybrid: {{…}}`"
                        ),
                    )
                    .with_hint("the bare word form has one value, `markdown`"),
                );
                StoreConfig::Markdown
            }
            Self::Sqlite { sqlite } => {
                if let Some(reason) = absolute_path_reason(&sqlite.to_string_lossy()) {
                    errors.push(
                        ValidationError::new(
                            ValidationCode::TypeMismatch,
                            format!("{at}.sqlite"),
                            format!("`{}` is an absolute path ({reason})", sqlite.display()),
                        )
                        .with_hint("a store path is relative to `.engineering`, like every other"),
                    );
                }
                StoreConfig::Sqlite { path: sqlite }
            }
            Self::Postgres { postgres } => {
                if postgres.trim().is_empty() {
                    errors.push(ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("{at}.postgres"),
                        "a Postgres store needs a connection URL",
                    ));
                }
                StoreConfig::Postgres { url: postgres }
            }
            Self::Hybrid { hybrid } => {
                let RawHybrid {
                    authority,
                    read,
                    on_unreachable,
                    on_divergence,
                    local,
                    replica,
                } = *hybrid;
                for (field, value) in [
                    ("authority", &authority),
                    ("read", &read),
                    ("on_unreachable", &on_unreachable),
                    ("on_divergence", &on_divergence),
                ] {
                    let accepted = HYBRID_WORDS
                        .iter()
                        .find(|(name, _)| *name == field)
                        .map_or(&[][..], |(_, words)| *words);
                    if !accepted.contains(&value.as_str()) {
                        errors.push(
                            ValidationError::new(
                                ValidationCode::TypeMismatch,
                                format!("{at}.hybrid.{field}"),
                                format!("`{value}` is not a `{field}`; one of {}", accepted.join(", ")),
                            )
                            .with_hint(
                                "a hybrid's four words are required and never defaulted: a default \
                                 here is a policy nobody chose applied to somebody's data",
                            ),
                        );
                    }
                }
                StoreConfig::Hybrid {
                    policy: HybridPolicy {
                        authority,
                        read,
                        on_unreachable,
                        on_divergence,
                    },
                    local: Box::new(local.validate(&format!("{at}.hybrid.local"), errors)),
                    replica: Box::new(replica.validate(&format!("{at}.hybrid.replica"), errors)),
                }
            }
        }
    }
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
    /// Where the plan is kept. Absent means `markdown`.
    #[serde(default)]
    pub store: Option<RawStore>,
}

/// Serde default for the format version.
fn default_version() -> String {
    PROJECT_VERSION.to_owned()
}

impl TryFrom<RawProjectConfig> for ProjectConfig {
    type Error = ValidationErrors;

    fn try_from(raw: RawProjectConfig) -> Result<Self, Self::Error> {
        let mut errors = ValidationErrors::new();

        if raw.version != PROJECT_VERSION {
            errors.push(
                ValidationError::new(
                    ValidationCode::UnsupportedProtocolVersion,
                    "project.version",
                    format!(
                        "this build reads `{PROJECT_VERSION}`, not `{}`",
                        raw.version
                    ),
                )
                .with_hint("upgrade the tooling rather than reinterpreting the document"),
            );
        }

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

        let store = raw.store.map_or(StoreConfig::Markdown, |store| {
            store.validate("project.store", &mut errors)
        });

        let config = Self {
            protocol: raw.protocol,
            profile: raw.profile,
            summary: raw.summary,
            protocols,
            paths,
            store,
        };
        errors.into_result(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "protocol: adp/1\nprofile: development.standard\n";

    #[test]
    fn a_project_that_names_no_store_keeps_its_plan_in_markdown() {
        let parsed = config(BASE).expect("valid");
        assert_eq!(parsed.store, StoreConfig::Markdown);
    }

    #[test]
    fn a_sqlite_store_is_a_relative_path_resolved_against_engineering() {
        let parsed = config(&format!("{BASE}store:\n  sqlite: plan.sqlite3\n")).expect("valid");
        assert_eq!(
            parsed.store.resolved(Path::new("/repo/.engineering")),
            StoreConfig::Sqlite {
                path: PathBuf::from("/repo/.engineering/plan.sqlite3")
            }
        );
        let absolute = config(&format!("{BASE}store:\n  sqlite: /var/plan.sqlite3\n"))
            .expect_err("an absolute path is refused");
        assert_eq!(absolute.as_slice()[0].location, "project.store.sqlite");
    }

    #[test]
    fn a_hybrid_missing_a_word_is_refused_naming_the_word() {
        // The parser refuses it: the four words are required fields, not defaulted ones.
        let text = format!(
            "{BASE}store:\n  hybrid:\n    authority: local\n    read: local-first\n    \
             on_unreachable: refuse\n    local: markdown\n    replica:\n      sqlite: plan.sqlite3\n"
        );
        let error = serde_yaml::from_str::<RawProjectConfig>(&text)
            .expect_err("a hybrid without `on_divergence` does not parse");
        assert!(error.to_string().contains("on_divergence"), "{error}");
    }

    #[test]
    fn a_hybrid_word_the_runtime_does_not_know_is_refused_naming_the_field_and_the_words() {
        let text = format!(
            "{BASE}store:\n  hybrid:\n    authority: local\n    read: sometimes\n    \
             on_unreachable: refuse\n    on_divergence: record\n    local: markdown\n    \
             replica:\n      sqlite: plan.sqlite3\n"
        );
        let errors = config(&text).expect_err("`sometimes` is not a read path");
        let error = &errors.as_slice()[0];
        assert_eq!(error.location, "project.store.hybrid.read");
        assert!(error.message.contains("local-first"), "{}", error.message);
    }

    #[test]
    fn a_complete_hybrid_carries_its_words_and_both_halves() {
        let text = format!(
            "{BASE}store:\n  hybrid:\n    authority: local\n    read: local-first\n    \
             on_unreachable: serve-stale\n    on_divergence: record\n    local: markdown\n    \
             replica:\n      sqlite: plan.sqlite3\n"
        );
        let parsed = config(&text).expect("valid");
        let StoreConfig::Hybrid {
            policy,
            local,
            replica,
        } = parsed.store
        else {
            panic!("a hybrid");
        };
        assert_eq!(policy.on_unreachable, "serve-stale");
        assert_eq!(*local, StoreConfig::Markdown);
        assert_eq!(
            *replica,
            StoreConfig::Sqlite {
                path: PathBuf::from("plan.sqlite3")
            }
        );
    }

    fn config(yaml: &str) -> Result<ProjectConfig, ValidationErrors> {
        let raw: RawProjectConfig = serde_yaml::from_str(yaml).expect("document parses");
        ProjectConfig::try_from(raw)
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
            "/srv/trees/engineering-protocols",
            "~/trees/engineering-protocols",
            "~",
            r"C:\trees\engineering-protocols",
            r"D:/trees/engineering-protocols",
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
            "git+file:///srv/mirror/engineering-protocols.git#0123456789abcdef0123456789abcdef01234567",
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
    /// This test asserted the opposite until 2026-08-25: `protocols: /opt/engineering-protocols`
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
protocols: ../../engineering-protocols
",
        )
        .expect("a relative path out of the project is allowed");
        assert_eq!(
            by_relative_path.protocols,
            ProtocolSource::Path(PathBuf::from("../../engineering-protocols"))
        );

        let by_pinned_locator = config(
            r"
protocol: adp/1
profile: development.standard
protocols: git+https://example.com/engineering-protocols.git#0123456789abcdef0123456789abcdef01234567
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
protocols: /opt/engineering-protocols
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
             protocols: git+ssh://git@github.com/beyond10x/engineering-protocols.git#{revision}\n"
        ))
        .expect("the pinned repository source validates");

        let ProtocolSource::Git(source) = parsed.protocols else {
            panic!("the repository locator was not reinterpreted as a path");
        };
        assert_eq!(
            source.repository(),
            "git+ssh://git@github.com/beyond10x/engineering-protocols.git"
        );
        assert_eq!(
            source.git_url(),
            "ssh://git@github.com/beyond10x/engineering-protocols.git"
        );
        assert_eq!(source.revision(), revision);
    }

    #[test]
    fn a_git_protocol_source_without_an_immutable_revision_is_refused() {
        for protocols in [
            "git+ssh://git@github.com/beyond10x/engineering-protocols.git",
            "git+ssh://git@github.com/beyond10x/engineering-protocols.git#main",
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
}
