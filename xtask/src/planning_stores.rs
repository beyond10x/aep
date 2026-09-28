//! The website's planning-store reference page, rendered from `aep_domain::store_catalog`.
//!
//! Part of `cargo xtask status` because that is the generator the release procedure runs after the
//! tag exists: a removal the catalog records for the next release reads as upcoming until that
//! release is tagged, and the release's own `cargo xtask status` run rewrites it as shipped.

use std::fmt::Write as _;
use std::path::Path;

use aep_domain::store_catalog::{StoreEntry, StoreStatus, CATALOG};
use anyhow::Result;

/// The page the catalog is rendered into.
pub const PAGE: &str = "website/docs/reference/planning-stores.md";

/// The generated region's first line. An MDX comment: Docusaurus compiles the page as MDX 3.
pub const BEGIN: &str =
    "{/* generated:planning-stores:begin — do not edit; run `cargo xtask status` */}";

/// The generated region's last line.
pub const END: &str = "{/* generated:planning-stores:end */}";

/// Writes the page's generated region, or with `check` refuses a page that lags the catalog.
pub fn hold(root: &Path, newest_tag: &str, check: bool) -> Result<bool> {
    super::hold_region(root, PAGE, BEGIN, END, &render(CATALOG, newest_tag), check)
}

/// A release version as a comparable triple; anything unparsable sorts as not yet released.
fn triple(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|part| part.parse::<u64>().ok());
    let triple = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(triple)
}

/// `version`, marked as upcoming while no tag at or after it is reachable.
fn release(version: &str, newest_tag: &str) -> String {
    match (triple(version), triple(newest_tag)) {
        (Some(version_triple), Some(newest)) if version_triple <= newest => version.to_owned(),
        _ => format!("{version} (next release)"),
    }
}

/// A table cell: a `|` would end the cell and a newline the row.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// Renders the generated region's body from `catalog`, judging releases against `newest_tag`.
pub fn render(catalog: &[StoreEntry], newest_tag: &str) -> String {
    let mut out = String::from(
        "\n## Supported\n\n\
         | Store | `version` | `store` selector | Also requires | Since | Notes |\n\
         |---|---|---|---|---|---|\n",
    );
    for entry in catalog {
        if let StoreStatus::Supported { since } = entry.status {
            let _ = writeln!(
                out,
                "| {} | `{}` | `{}` | {} | {} | {} |",
                cell(entry.name),
                cell(entry.project_version),
                cell(entry.selector),
                entry
                    .requires
                    .map_or_else(|| "—".to_owned(), |line| format!("`{}`", cell(line))),
                release(since, newest_tag),
                cell(entry.note),
            );
        }
    }
    out.push_str(
        "\n## Removed\n\n\
         | Store | `version` | `store` selector | Removed in | Migration | Notes |\n\
         |---|---|---|---|---|---|\n",
    );
    for entry in catalog {
        if let StoreStatus::Removed {
            removed_in,
            migration,
        } = entry.status
        {
            let _ = writeln!(
                out,
                "| {} | `{}` | `{}` | {} | {} | {} |",
                cell(entry.name),
                cell(entry.project_version),
                cell(entry.selector),
                release(removed_in, newest_tag),
                cell(migration),
                cell(entry.note),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use aep_domain::store_catalog::StoreKind;

    use super::*;

    const REMOVED_NEXT: StoreEntry = StoreEntry {
        kind: StoreKind::Hybrid,
        name: "Hybrid",
        project_version: "aep.project/1",
        selector: "store: { hybrid: {…} }",
        requires: None,
        status: StoreStatus::Removed {
            removed_in: "0.65.0",
            migration: "None | re-create",
        },
        note: "two halves",
    };

    const SUPPORTED: StoreEntry = StoreEntry {
        kind: StoreKind::GitNative,
        name: "Git-native",
        project_version: "aep.project/5",
        selector: "store: { git: {} }",
        requires: Some("planning_scope: <scope>"),
        status: StoreStatus::Supported { since: "0.62.0" },
        note: "the default",
    };

    #[test]
    fn the_page_lists_supported_and_removed_stores_with_selector_and_migration() {
        let page = render(&[SUPPORTED, REMOVED_NEXT], "0.64.0");
        let supported = page.find("## Supported").expect("a supported section");
        let removed = page.find("## Removed").expect("a removed section");
        let git = page
            .find("| Git-native | `aep.project/5` | `store: { git: {} }` | `planning_scope: <scope>` | 0.62.0 |")
            .expect("the supported row names version, selector, requirement and release");
        let hybrid = page
            .find("| Hybrid | `aep.project/1` | `store: { hybrid: {…} }` |")
            .expect("the removed row names version and selector");
        assert!(supported < git && git < removed && removed < hybrid);
        assert!(
            page.contains("None \\| re-create"),
            "a `|` in a migration path is escaped, not a column break:\n{page}"
        );
    }

    #[test]
    fn a_removal_in_an_untagged_release_reads_as_next_until_it_is_tagged() {
        assert!(render(&[REMOVED_NEXT], "0.64.0").contains("| 0.65.0 (next release) |"));
        let shipped = render(&[REMOVED_NEXT], "0.65.0");
        assert!(shipped.contains("| 0.65.0 |"));
        assert!(!shipped.contains("next release"));
        assert!(!render(&[REMOVED_NEXT], "0.100.0").contains("next release"));
    }

    #[test]
    fn every_catalog_entry_reaches_the_page() {
        let page = render(CATALOG, "0.64.0");
        for entry in CATALOG {
            assert!(
                page.contains(&format!("| {} |", entry.name)),
                "{} is missing from the page",
                entry.name
            );
        }
    }

    #[test]
    fn a_page_that_lags_the_catalog_fails_the_check_and_is_rewritten_without_it() {
        let root = std::env::temp_dir().join(format!(
            "aep-xtask-planning-stores-{}-{}",
            std::process::id(),
            line!()
        ));
        let page = root.join(PAGE);
        std::fs::create_dir_all(page.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&page, format!("# Stores\n\n{BEGIN}\n| stale |\n{END}\n")).expect("write");

        let error = hold(&root, "0.64.0", true).expect_err("a stale page is refused");
        assert!(
            error.to_string().contains("run `cargo xtask status`"),
            "the refusal names the command that fixes it: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(&page).expect("read"),
            format!("# Stores\n\n{BEGIN}\n| stale |\n{END}\n"),
            "a check writes nothing"
        );

        assert!(hold(&root, "0.64.0", false).expect("written"));
        assert!(!hold(&root, "0.64.0", true).expect("fresh page passes the check"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
