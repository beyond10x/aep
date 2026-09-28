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

/// The `planning_scope` an `aep.project/5` store gets when nothing names one: the name of the
/// directory that holds `.engineering/`, which is the repository's own name in every layout this
/// build writes. `/1` refuses the key, so a migrated `/1` project never has one to keep.
pub(crate) fn default_planning_scope(project_root: &std::path::Path) -> Result<String> {
    let absolute = std::path::absolute(project_root)?;
    let name = absolute
        .canonicalize()
        .unwrap_or(absolute)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.trim().is_empty() || name.len() > 255 {
        anyhow::bail!(
            "`planning_scope` is derived from the repository directory's name, and {} has none \
             that fits (1..=255 bytes, not only whitespace)",
            project_root.display()
        );
    }
    Ok(name)
}

/// Runs one `aep plan store` verb.
pub(crate) fn run(command: StoreCommand) -> Result<ExitCode> {
    match command {
        StoreCommand::Migrate {
            command: MigrateCommand::Git(args),
        } => migrate_git::run(&args),
    }
}
