//! The planning store backends AEP supports, and the ones it removed, as data.
//!
//! The website's planning-store reference page is rendered from [`CATALOG`] by
//! `cargo xtask status`, and `status-check` refuses a page that lags it. [`catalog_kind`] is the one
//! place a [`StoreConfig`] variant is tied to an entry: its `match` is exhaustive, so a store variant
//! added or removed without touching this file does not compile.
//!
//! Releases are recorded as versions only. Whether a version has shipped is derived from the
//! repository's tags when the page is rendered, so an entry written ahead of a release reads as
//! upcoming until that release is tagged, and needs no edit afterwards.

use crate::project::StoreConfig;

/// One planning store backend, supported or removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StoreKind {
    /// Markdown artifacts and evidence files merged by version control (`aep.project/5`).
    GitNative,
    /// One `SQLite` database file.
    Sqlite,
    /// A `PostgreSQL` database.
    Postgres,
    /// The `aep.project/1` Markdown layout with its `journal.jsonl`.
    MarkdownJournal,
    /// A local store with a replica under a divergence policy.
    Hybrid,
    /// The `aep.project/2`–`/4` event-log stores.
    EventLog,
}

/// Whether a backend can be selected, and since or until which release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreStatus {
    /// Selectable.
    Supported {
        /// The first release that read it.
        since: &'static str,
    },
    /// No longer read.
    Removed {
        /// The first release that refuses it.
        removed_in: &'static str,
        /// How a store of this kind reaches a supported one, as Markdown.
        migration: &'static str,
    },
}

/// One catalog row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreEntry {
    /// The backend.
    pub kind: StoreKind,
    /// Its name on the page.
    pub name: &'static str,
    /// The `version:` a `project.yaml` selecting it carries.
    pub project_version: &'static str,
    /// The `store:` selector, as YAML flow syntax with `<placeholders>`.
    pub selector: &'static str,
    /// Other `project.yaml` lines the selector requires, as YAML with `<placeholders>`.
    pub requires: Option<&'static str>,
    /// Supported or removed.
    pub status: StoreStatus,
    /// One sentence for a reader, as Markdown.
    pub note: &'static str,
}

/// Every planning store backend AEP has shipped, supported ones first.
pub const CATALOG: &[StoreEntry] = &[
    StoreEntry {
        kind: StoreKind::GitNative,
        name: "Git-native",
        project_version: "aep.project/5",
        selector: "store: { git: {} }",
        requires: Some("planning_scope: <scope>"),
        status: StoreStatus::Supported { since: "0.62.0" },
        note: "The default. One Markdown file per artifact under `.engineering/planning/` and one \
               evidence file per record under `.engineering/evidence/`; Git is the history. \
               `store` may be omitted.",
    },
    StoreEntry {
        kind: StoreKind::Sqlite,
        name: "SQLite",
        project_version: "aep.project/1",
        selector: "store: { sqlite: <path> }",
        requires: None,
        status: StoreStatus::Supported { since: "0.30.0" },
        note: "One database file; the path is relative to `.engineering/`.",
    },
    StoreEntry {
        kind: StoreKind::Postgres,
        name: "PostgreSQL",
        project_version: "aep.project/1",
        selector: "store: { postgres: <url> }",
        requires: None,
        status: StoreStatus::Supported { since: "0.30.0" },
        note: "A database reached by a libpq connection string or URL.",
    },
    StoreEntry {
        kind: StoreKind::MarkdownJournal,
        name: "Markdown journal layout",
        project_version: "aep.project/1",
        selector: "store: markdown",
        requires: None,
        status: StoreStatus::Removed {
            removed_in: "0.65.0",
            migration: "`aep plan store migrate git --verify`, which the release that removes \
                        the layout still carries.",
        },
        note: "Markdown artifacts under `.engineering/planning/` plus a `journal.jsonl`; also \
               what an `aep.project/1` file without `store` selected.",
    },
    StoreEntry {
        kind: StoreKind::Hybrid,
        name: "Hybrid",
        project_version: "aep.project/1",
        selector: "store: { hybrid: {…} }",
        requires: None,
        status: StoreStatus::Removed {
            removed_in: "0.65.0",
            migration: "None. Re-create the plan in a Git-native, SQLite or PostgreSQL store.",
        },
        note: "A local store and a replica under a declared divergence policy.",
    },
    StoreEntry {
        kind: StoreKind::EventLog,
        name: "Event-log stores",
        project_version: "aep.project/2 – aep.project/4",
        selector: "store: { eventlog: {…} }",
        requires: None,
        status: StoreStatus::Removed {
            removed_in: "0.62.0",
            migration: "Install the build at commit `9c0f1da44429ff935fa0b2d743457945d51e1c51` \
                        (`cargo install --git https://github.com/beyond10x/aep --rev \
                        9c0f1da44429ff935fa0b2d743457945d51e1c51 aep-cli`), then run \
                        `aep plan store migrate git --verify`.",
        },
        note: "Planning state kept in an Eventlog stream.",
    },
];

/// The catalog entry a validated store configuration belongs to.
///
/// Exhaustive on purpose: adding or removing a [`StoreConfig`] variant fails to compile here until
/// the catalog says what became of it.
#[must_use]
pub const fn catalog_kind(store: &StoreConfig) -> StoreKind {
    match store {
        StoreConfig::Git { .. } => StoreKind::GitNative,
        StoreConfig::Sqlite { .. } => StoreKind::Sqlite,
        StoreConfig::Postgres { .. } => StoreKind::Postgres,
        StoreConfig::Markdown => StoreKind::MarkdownJournal,
        StoreConfig::Hybrid { .. } => StoreKind::Hybrid,
    }
}

/// The catalog row for `kind`.
#[must_use]
pub fn entry(kind: StoreKind) -> Option<&'static StoreEntry> {
    CATALOG.iter().find(|entry| entry.kind == kind)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::project::{HybridPolicy, ProjectConfig, RawProjectConfig};

    /// One value of every `StoreConfig` variant. A new variant fails `catalog_kind` first; this list
    /// is what makes the test below see it.
    fn every_variant() -> Vec<StoreConfig> {
        vec![
            StoreConfig::Git {
                planning: PathBuf::from("planning"),
                evidence: PathBuf::from("evidence"),
            },
            StoreConfig::Sqlite {
                path: PathBuf::from("plan.sqlite3"),
            },
            StoreConfig::Postgres {
                url: "postgres://localhost/plan".to_owned(),
            },
            StoreConfig::Markdown,
            StoreConfig::Hybrid {
                policy: HybridPolicy {
                    authority: "local".to_owned(),
                    read: "local-first".to_owned(),
                    on_unreachable: "refuse".to_owned(),
                    on_divergence: "record".to_owned(),
                },
                local: Box::new(StoreConfig::Markdown),
                replica: Box::new(StoreConfig::Postgres {
                    url: "postgres://localhost/plan".to_owned(),
                }),
            },
        ]
    }

    #[test]
    fn every_store_variant_has_a_catalog_entry() {
        for store in every_variant() {
            let kind = catalog_kind(&store);
            assert!(
                entry(kind).is_some(),
                "{store:?} maps to {kind:?}, which has no catalog entry"
            );
        }
    }

    #[test]
    fn every_supported_entry_is_a_store_the_code_can_select() {
        let reachable: Vec<StoreKind> = every_variant().iter().map(catalog_kind).collect();
        for entry in CATALOG {
            if matches!(entry.status, StoreStatus::Supported { .. }) {
                assert!(
                    reachable.contains(&entry.kind),
                    "{} is listed as supported but no store variant maps to it",
                    entry.name
                );
            }
        }
    }

    #[test]
    fn each_kind_appears_once() {
        let mut kinds: Vec<StoreKind> = CATALOG.iter().map(|entry| entry.kind).collect();
        kinds.sort();
        kinds.dedup();
        assert_eq!(kinds.len(), CATALOG.len(), "a kind is catalogued twice");
    }

    /// The page tells a reader what to write; this writes exactly that and asks the parser.
    #[test]
    fn a_supported_entrys_documented_selector_selects_that_store() {
        for entry in CATALOG {
            if !matches!(entry.status, StoreStatus::Supported { .. }) {
                continue;
            }
            let mut document = format!(
                "version: {}\nprotocol: adp/1\nprofile: development.standard\n",
                entry.project_version
            );
            if let Some(requires) = entry.requires {
                document.push_str(&requires.replace("<scope>", "catalog-test"));
                document.push('\n');
            }
            document.push_str(
                &entry
                    .selector
                    .replace("<path>", "plan.sqlite3")
                    .replace("<url>", "postgres://localhost/plan"),
            );
            document.push('\n');
            let raw: RawProjectConfig = serde_yaml::from_str(&document)
                .unwrap_or_else(|error| panic!("{}: {error}\n{document}", entry.name));
            let config = ProjectConfig::try_from(raw).unwrap_or_else(|errors| {
                panic!(
                    "{}'s documented selector is refused: {errors:?}\n{document}",
                    entry.name
                )
            });
            assert_eq!(
                catalog_kind(&config.store),
                entry.kind,
                "{}'s documented selector selects another store",
                entry.name
            );
        }
    }

    #[test]
    fn a_removed_entry_names_its_migration_path() {
        for entry in CATALOG {
            if let StoreStatus::Removed { migration, .. } = entry.status {
                assert!(
                    !migration.trim().is_empty(),
                    "{} names no migration path",
                    entry.name
                );
            }
        }
    }
}
