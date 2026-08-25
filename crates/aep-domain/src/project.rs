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
                    "use a filesystem path or a pinned git+ssh://, git+https://, or git+file:// locator",
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
}

impl fmt::Display for ProjectConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} under {}", self.profile, self.protocol)
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
            if path.is_absolute() {
                errors.push(
                    ValidationError::new(
                        ValidationCode::TypeMismatch,
                        format!("project.{label}"),
                        format!("`{}` is absolute", path.display()),
                    )
                    .with_hint(
                        "paths inside the project are relative to `.engineering`, so the repository \
                         can be cloned anywhere without editing them; `protocols` is a separate \
                         source and may identify an external tree",
                    ),
                );
            }
        }

        let config = Self {
            protocol: raw.protocol,
            profile: raw.profile,
            summary: raw.summary,
            protocols,
            paths,
        };
        errors.into_result(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn the_protocol_tree_may_live_outside_the_project() {
        let parsed = config(
            r"
protocol: adp/1
profile: development.standard
protocols: /opt/engineering-protocols
",
        )
        .expect("an absolute protocol tree is allowed");
        assert_eq!(
            parsed.protocols,
            ProtocolSource::Path(PathBuf::from("/opt/engineering-protocols"))
        );
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
