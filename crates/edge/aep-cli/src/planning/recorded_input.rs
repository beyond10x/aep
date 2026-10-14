//! How a record `aep plan artifact evidence --from` writes names a file it read.
//!
//! On 0.71.2 a record named its report and suite by the path as typed, so a record made with an
//! absolute path carried the operator's local directories: they mean nothing on another machine,
//! and Gates' personal-paths scan refuses them in a commit. A record now names an input by its path
//! from the repository root, or, for an input outside the root, by its file name alone, and always
//! beside the `sha256:` digest of the bytes it read. That digest, not the name, is what identifies
//! the input. The rule is `aep.evidence.InputForm` and `aep.evidence.InputPathNotPortable` in
//! `ess/domains/evidence.yaml`.
//!
//! Both paths are made absolute against the working directory and normalized by their components
//! alone. Nothing here resolves a link: `canonicalize` would turn a root reached through a
//! symlink into its target and could put that target's directories into the name.

use anyhow::{bail, Context, Result};
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// How a record names an input: the specification's `aep.evidence.InputForm`, less `as_typed`,
/// which is the refusal rather than a form a record is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputForm {
    /// Inside the root: the path from the root, `/`-separated, with no `.` or `..` component.
    RepositoryRelative,
    /// Outside the root: the file's name alone.
    FileName,
}

impl InputForm {
    /// Both forms, in the specification's order: what the test against the generated
    /// `aep.evidence.InputForm` compares.
    #[cfg(test)]
    const ALL: [Self; 2] = [Self::RepositoryRelative, Self::FileName];

    /// The specification's name for the form.
    fn wire(self) -> &'static str {
        match self {
            Self::RepositoryRelative => "repository_relative",
            Self::FileName => "file_name",
        }
    }

    /// Whether `name` is a name of this form: ordinary components only, and one of them for a
    /// file name.
    fn admits(self, name: &str) -> bool {
        portable(name) && (self == Self::RepositoryRelative || !name.contains('/'))
    }
}

/// A file an evidence record names, as the record names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedInput {
    /// The name the record carries.
    pub(crate) name: String,
    /// `sha256:` and the lowercase hex of the SHA-256 of the bytes read.
    pub(crate) digest: String,
}

impl RecordedInput {
    /// The record's `reference` when the caller gives no `--ref`: `<name>@<digest>`.
    pub(crate) fn reference(&self) -> String {
        format!("{}@{}", self.name, self.digest)
    }
}

/// Names `path`, whose bytes are `bytes`, for a record of the repository at `root`.
///
/// # Errors
///
/// `InputPathNotPortable` when the name would be empty, absolute or carry a `.` or `..`
/// component; the working directory cannot be read to make a relative path absolute.
pub(crate) fn recorded_input(root: &Path, path: &Path, bytes: &[u8]) -> Result<RecordedInput> {
    let (name, form) = name_and_form(root, path)?;
    if !form.admits(&name) {
        bail!(
            "InputPathNotPortable: {} would be recorded as `{name}`, which is not a {} name: \
             neither a path relative to the repository root {} nor a file name; nothing was \
             recorded",
            path.display(),
            form.wire(),
            lexically_absolute(root)?.display()
        );
    }
    Ok(RecordedInput {
        name,
        digest: sha256(bytes),
    })
}

/// `path`'s name from the repository at `root`, or [`None`] when it is not one: the path lies
/// outside the root, or is the root itself.
///
/// The `repository_relative` form alone, for a record that names nothing outside the repository
/// (`aep.evidence.SpendSourceOutsideRepository`).
///
/// # Errors
///
/// The working directory cannot be read to make a relative path absolute.
pub(crate) fn repository_relative(root: &Path, path: &Path) -> Result<Option<String>> {
    let (name, form) = name_and_form(root, path)?;
    Ok((form == InputForm::RepositoryRelative && form.admits(&name)).then_some(name))
}

/// The name `path` would be recorded under from `root`, and the form that name takes.
fn name_and_form(root: &Path, path: &Path) -> Result<(String, InputForm)> {
    let root = lexically_absolute(root)?;
    let absolute = lexically_absolute(path)?;
    Ok(match absolute.strip_prefix(&root) {
        Ok(relative) if relative.components().next().is_some() => (
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/"),
            InputForm::RepositoryRelative,
        ),
        _ => (
            absolute
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            InputForm::FileName,
        ),
    })
}

/// `true` for a name made only of ordinary `/`-separated components: not empty, not absolute,
/// and no `.` or `..`.
fn portable(name: &str) -> bool {
    !name.is_empty()
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(name)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// `path` made absolute against the working directory, with `.` dropped and each `..` taking
/// the component before it away. No link is followed.
fn lexically_absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    let absolute = std::path::absolute(path)
        .with_context(|| format!("making {} absolute", path.display()))?;
    let mut normal = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other.as_os_str()),
        }
    }
    Ok(normal)
}

/// `sha256:` and the lowercase hex of the SHA-256 of `bytes`.
pub(crate) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn an_input_inside_the_root_is_named_from_the_root() {
        let named = recorded_input(
            Path::new("/work/repository"),
            Path::new("/work/repository/reports/./run/../run/report.json"),
            b"",
        )
        .unwrap();
        assert_eq!(named.name, "reports/run/report.json");
        assert_eq!(named.digest, EMPTY);
        assert_eq!(named.reference(), format!("reports/run/report.json@{EMPTY}"));
    }

    #[test]
    fn an_input_outside_the_root_is_named_by_its_file_name_alone() {
        for path in [
            "/elsewhere/reports/report.json",
            "/work/repository/../reports/report.json",
            "/work/repository-sibling/report.json",
        ] {
            let named =
                recorded_input(Path::new("/work/repository"), Path::new(path), b"").unwrap();
            assert_eq!(named.name, "report.json", "{path}");
        }
    }

    #[test]
    fn a_relative_input_is_named_against_the_working_directory() {
        let here = std::env::current_dir().unwrap();
        let named = recorded_input(&here, Path::new("reports/report.json"), b"").unwrap();
        assert_eq!(named.name, "reports/report.json");
        let named = recorded_input(Path::new(""), Path::new("report.json"), b"").unwrap();
        assert_eq!(named.name, "report.json");
    }

    #[test]
    fn a_name_that_is_neither_form_is_refused_as_not_portable() {
        let refused = recorded_input(Path::new("/work"), Path::new("/"), b"").unwrap_err();
        assert!(
            refused.to_string().starts_with("InputPathNotPortable"),
            "{refused}"
        );
        for name in ["", "/abs/report.json", "../report.json", "a/./b", "a//b"] {
            assert!(!portable(name), "`{name}` is not portable");
        }
        for name in ["report.json", "reports/run/report.json", "report;1.json"] {
            assert!(portable(name), "`{name}` is portable");
        }
        // A file name keeps no directory component.
        assert!(!InputForm::FileName.admits("reports/report.json"));
        assert!(InputForm::RepositoryRelative.admits("reports/report.json"));
    }

    /// The forms are the specification's `aep.evidence.InputForm`, by its names, and the one it
    /// adds is the refusal.
    #[test]
    fn input_forms_are_the_generated_input_form_variants_less_the_refused_one() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../generated/ess/schema/schema/types/aep.evidence.InputForm.schema.json");
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("the generated schema is committed"),
        )
        .expect("the generated schema is JSON");
        let generated: Vec<&str> = schema["$defs"]["aep.evidence.InputForm"]["enum"]
            .as_array()
            .expect("an enum")
            .iter()
            .map(|value| value.as_str().expect("a name"))
            .collect();
        let ours: Vec<&str> = InputForm::ALL.iter().map(|form| form.wire()).collect();
        assert_eq!(ours, generated[..2]);
        assert_eq!(generated[2..], ["as_typed"], "the one form a record is refused in");
    }
}
