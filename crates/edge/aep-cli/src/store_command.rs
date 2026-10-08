//! `aep plan store` — the one store operation this build keeps: moving an `aep.project/1` Markdown
//! plan to the Git-native `aep.project/5` layout.
//!
//! The event-log verbs this group carried until 0.62.0 went with the event-log stores. What is
//! left is the migration a `/1` plan still needs, because `/5` is the default for a new store and
//! every planning command suggests it for an old one.

use std::process::ExitCode;

use anyhow::Result;
use clap::Subcommand;

mod migrate_git;

/// What to do with the selected planning store.
#[derive(Debug, Subcommand)]
pub(crate) enum StoreCommand {
    /// Move the store to another layout.
    Migrate {
        /// Which layout to move it to.
        #[command(subcommand)]
        command: MigrateCommand,
    },
}

/// The layouts a store migrates to.
#[derive(Debug, Subcommand)]
pub(crate) enum MigrateCommand {
    /// Rewrite an `aep.project/1` Markdown store as an `aep.project/5` Git-native one.
    ///
    /// Every document becomes `aep.planning-md/3`, its journalled moves become its `transitions`
    /// and each journalled evidence record becomes one file under `.engineering/evidence/`. The
    /// journal is removed; Git history keeps it. Refuses a dirty `.engineering` and refuses the
    /// whole migration, writing nothing, when any document disagrees with its journal.
    Git(migrate_git::GitArgs),
}

/// Where a written `planning_scope` came from, in the order the commands try them. The wire names
/// are the variants of `aep.plan.ScopeSource` (`ess/domains/plan.yaml`); a test holds them to the
/// generated schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeSource {
    /// `--planning-scope <name>`.
    Flag,
    /// The last path segment of the `origin` remote's URL, without `.git`.
    OriginRemote,
    /// The directory holding the Git common directory, when that directory is named `.git`: the
    /// primary checkout, whichever linked worktree the command runs in.
    GitCommonDirectory,
    /// The directory holding `.engineering/`, outside any Git repository.
    EngineeringDirectory,
}

impl ScopeSource {
    /// Every source, in the order the commands try them.
    #[cfg(test)]
    const ALL: [Self; 4] = [
        Self::Flag,
        Self::OriginRemote,
        Self::GitCommonDirectory,
        Self::EngineeringDirectory,
    ];

    /// The source's name in the specification.
    #[cfg(test)]
    const fn wire(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::OriginRemote => "origin_remote",
            Self::GitCommonDirectory => "git_common_directory",
            Self::EngineeringDirectory => "engineering_directory",
        }
    }

    /// How the text output names the source.
    const fn described(self) -> &'static str {
        match self {
            Self::Flag => "from --planning-scope",
            Self::OriginRemote => "from the origin remote",
            Self::GitCommonDirectory => "from the primary checkout's directory",
            Self::EngineeringDirectory => "from the project directory's name",
        }
    }
}

/// A `planning_scope` that fits the rule, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanningScope {
    /// The value written to the project file, exactly as derived.
    pub(crate) value: String,
    /// Where it came from.
    pub(crate) source: ScopeSource,
}

impl std::fmt::Display for PlanningScope {
    /// `planning_scope: <value> (<source>)`, the form both commands print.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "planning_scope: {} ({})",
            self.value,
            self.source.described()
        )
    }
}

/// The `planning_scope` an `aep.project/5` store gets, from the first source that has one:
/// `--planning-scope`; the `origin` remote's repository name; the primary checkout's directory
/// name when the Git common directory is named `.git`; the directory holding `.engineering/` when
/// `project_root` is in no Git repository. A source that has a value decides: a value that does not
/// fit is refused, never passed over for the next. A Git repository with none of the Git sources
/// (no `origin`, a common directory not named `.git`, as in a bare repository's worktree) is
/// refused naming `--planning-scope`, rather than falling back to the checkout's own directory
/// name, which in a linked worktree is whatever the worktree tool chose.
///
/// `/1` refuses the key, so a migrated `/1` project never has one to keep.
pub(crate) fn planning_scope(
    project_root: &std::path::Path,
    flag: Option<&str>,
) -> Result<PlanningScope> {
    let fitted = |value: String, source: ScopeSource| {
        if aep_domain::project::planning_scope_fits(&value) {
            return Ok(PlanningScope { value, source });
        }
        let named = match source {
            ScopeSource::Flag => "`--planning-scope`".to_owned(),
            other => format!("the scope derived {}", other.described()),
        };
        anyhow::bail!(
            "{named} is {value:?}, which does not fit `planning_scope` ({}); pass \
             `--planning-scope <name>`",
            aep_domain::project::PLANNING_SCOPE_RULE
        )
    };
    if let Some(value) = flag {
        return fitted(value.to_owned(), ScopeSource::Flag);
    }
    let absolute = std::path::absolute(project_root)?;
    let root = absolute.canonicalize().unwrap_or(absolute);
    let Some(common) = crate::planning_writer_fence::git_common_directory(&root)? else {
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        return fitted(name, ScopeSource::EngineeringDirectory);
    };
    if let Some(url) = origin_url(&common) {
        return fitted(repository_name_of_url(&url), ScopeSource::OriginRemote);
    }
    if common.file_name().is_some_and(|name| name == ".git") {
        let name = common
            .parent()
            .and_then(std::path::Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        return fitted(name, ScopeSource::GitCommonDirectory);
    }
    anyhow::bail!(
        "`planning_scope` cannot be derived for {}: its Git repository has no `origin` remote and \
         its common directory {} is not named `.git` (a bare repository's worktree, for one); \
         pass `--planning-scope <name>`",
        root.display(),
        common.display()
    )
}

/// The `origin` remote's URL in the repository whose common directory is `common`, or `None` when
/// it has none or `git` cannot answer. Read through `--git-dir`, so the answer is the repository
/// the common directory was found for and not one an inherited `GIT_DIR` names.
fn origin_url(common: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("--git-dir")
        .arg(common)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\n', '\r'])
            .to_owned(),
    )
}

/// The repository name a remote URL ends in: its last path segment with a trailing `.git`
/// removed. Handles `git@host:org/name.git`, `https://host/org/name(.git)(/)`, `file://` URLs and
/// local paths, including one that names a checkout's `.git` directory.
fn repository_name_of_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches(['/', '\\']);
    let mut segments = trimmed.rsplit(['/', '\\', ':']);
    let mut last = segments.next().unwrap_or_default();
    if last == ".git" {
        last = segments.next().unwrap_or_default();
    }
    last.strip_suffix(".git").unwrap_or(last).to_owned()
}

/// Runs one `aep plan store` verb.
pub(crate) fn run(command: StoreCommand) -> Result<ExitCode> {
    match command {
        StoreCommand::Migrate {
            command: MigrateCommand::Git(args),
        } => migrate_git::run(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remote_url_names_its_repository_by_its_last_segment_without_dot_git() {
        for (url, name) in [
            ("git@github.com:org/name.git", "name"),
            ("git@host:name.git", "name"),
            ("ssh://git@host:2222/org/name.git", "name"),
            ("https://example.invalid/org/name.git", "name"),
            ("https://example.invalid/org/name", "name"),
            ("https://example.invalid/org/name/", "name"),
            ("https://example.invalid/org/name.git/", "name"),
            ("file:///srv/git/name.git", "name"),
            ("/srv/git/name.git", "name"),
            ("../name", "name"),
            ("/srv/checkouts/name/.git", "name"),
            ("C:\\repos\\name.git", "name"),
            ("https://example.invalid/org/name.github.io.git\n", "name.github.io"),
        ] {
            assert_eq!(repository_name_of_url(url), name, "{url:?}");
        }
    }

    #[test]
    fn a_remote_url_with_no_segment_names_no_repository_and_is_refused_as_unfit() {
        for url in ["", "/", ".git", "https://", "git@host:"] {
            let name = repository_name_of_url(url);
            assert!(
                !aep_domain::project::planning_scope_fits(&name),
                "{url:?} names {name:?}"
            );
        }
    }

    #[test]
    fn the_flag_decides_without_reading_the_repository_and_is_held_to_the_rule() {
        let missing = std::path::Path::new("/nonexistent/aep-scope-flag");
        assert_eq!(
            planning_scope(missing, Some("chosen")).expect("a fitting flag"),
            PlanningScope {
                value: "chosen".to_owned(),
                source: ScopeSource::Flag
            }
        );
        for unfit in ["", "\u{3000}", &"x".repeat(256)] {
            let error = planning_scope(missing, Some(unfit))
                .expect_err("an unfit flag is refused")
                .to_string();
            assert!(
                error.contains("`--planning-scope`")
                    && error.contains(aep_domain::project::PLANNING_SCOPE_RULE),
                "{error}"
            );
        }
    }

    /// The sources are the specification's `aep.plan.ScopeSource`, in its order, by its names.
    #[test]
    fn scope_sources_are_the_generated_scope_source_variants_in_order() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../generated/ess/schema/schema/types/aep.plan.ScopeSource.schema.json");
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("the generated schema is committed"),
        )
        .expect("the generated schema is JSON");
        let generated: Vec<&str> = schema["$defs"]["aep.plan.ScopeSource"]["enum"]
            .as_array()
            .expect("an enum")
            .iter()
            .map(|value| value.as_str().expect("a name"))
            .collect();
        let ours: Vec<&str> = ScopeSource::ALL.iter().map(|source| source.wire()).collect();
        assert_eq!(ours, generated);
    }
}
