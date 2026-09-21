//! A `workspace.yaml` that cannot be read is not a `workspace.yaml` that declares nothing.
//!
//! `AGENTS.md` invariant 5: *"Unknown differs from false. A missing observation cannot satisfy a
//! predicate and is not rewritten as a contradiction."*
//!
//! `planning.rs:2214` reads the declaration with
//! `load_workspace(root).map_or_else(|_| Vec::new(), …)` — every read failure, including a
//! `workspace.yaml` that exists and does not parse, becomes *this store declares no members*.
//! `story:migration-mapper-reads-the-declared-workspace` puts that function on the migration path
//! for the first time (`store_command.rs:1710`), where the consequence is a refusal whose
//! coordinate names an artifact document that is not what is wrong.

use std::path::{Path, PathBuf};
use std::process::Command;

const CROSSING: &str = "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\n\
                        status: draft\ntitle: A story that names another repository\nrelations:\n\
                        - informed_by: other/story:theirs\nrevision: 1\n---\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "aep-unreadable-declaration-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn store(project: &Path, declaration: Option<&str>) {
    let engineering = project.join(".engineering");
    let planning = engineering.join("planning");
    std::fs::create_dir_all(planning.join("story")).expect("planning source");
    std::fs::create_dir_all(project.join("protocols")).expect("protocol source");
    std::fs::write(
        engineering.join("project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
    )
    .expect("legacy selector");
    if let Some(declaration) = declaration {
        std::fs::write(engineering.join("workspace.yaml"), declaration).expect("declaration");
    }
    std::fs::write(planning.join("story/crossing.md"), CROSSING).expect("crossing source");
}

fn refusal(project: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(project)
        .args([
            "plan",
            "store",
            "migrate",
            "dry-run",
            "--project",
            ".engineering/project.yaml",
            "--authority-scope",
            "planning-review-1",
            "--authority-tenant",
            "tenant-review-1",
            "--authority-new",
            "--format",
            "json",
        ])
        .output()
        .expect("the built `aep` binary runs");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--format json is JSON");
    assert_eq!(json["outcome"]["kind"], "refused", "{json}");
    json["outcome"]["value"]["refusals"].clone()
}

/// A declaration the command could not read must not be reported as a store that declares nothing.
///
/// Both stores below are refused, and the operator has to be able to tell which one they have: one
/// has no workspace and a genuinely dangling edge, the other has a workspace file with a typo in
/// it and an edge that would be a crossing if the file parsed. Today the two receipts are
/// byte-identical and both name `story/crossing.md`, which is the document that is not wrong.
#[test]
fn an_unparseable_declaration_is_distinguishable_from_no_declaration() {
    let absent = scratch("absent");
    store(&absent, None);
    let absent_refusals = refusal(&absent);

    let unreadable = scratch("unreadable");
    // A closed format with one key misspelled: the exact mistake `deny_unknown_fields` exists to
    // catch, and the one an operator makes editing a workspace by hand.
    store(
        &unreadable,
        Some("version: aep.workspace/1\nmembers:\n  - name: other\n    sourcz: ../other\n"),
    );
    let unreadable_refusals = refusal(&unreadable);

    let _ = std::fs::remove_dir_all(&absent);
    let _ = std::fs::remove_dir_all(&unreadable);

    assert_ne!(
        unreadable_refusals, absent_refusals,
        "a `workspace.yaml` that does not parse is unknown, not a declaration of nothing; \
         both stores refused with {unreadable_refusals}"
    );
}
