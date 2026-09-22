//! The receipt's new `workspace_crossings` count, on a store that crosses nowhere.
//!
//! `InventoryCountsV1::workspace_crossings` (`aep-contract/src/migration/command.rs:463`) is
//! documented as the number of relations "whose target is another workspace member", and
//! `relations` beside it as "Relation records the migration imports. Counted from what the mapper
//! will produce, not from the source frontmatter." Both are published in
//! `schemas/generated/planning-migration-dry-run-v2.schema.json`.
//!
//! This store declares one member and that member is **itself** — the shape this repository's own
//! `.engineering/workspace.yaml` has, which names `engineering-protocols` with `source: ..`. Its
//! one edge is written `mine/story:local`, which `aep_domain::workspace::WorkspaceRef` defines as
//! "that member's story, wherever it is read from" — read here, in `mine`, that is `story:local`,
//! an artifact of this store. Nothing crosses out of this store, and the mapper does have a
//! destination entity for the edge.
//!
//! So the receipt must say one relation and no crossings. It says the opposite of both.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One member, and it is this store. Mirrors `.engineering/workspace.yaml`'s own first entry.
const DECLARATION: &str = "version: aep.workspace/1\nmembers:\n  - name: mine\n    source: ..\n";

const LOCAL: &str = "---\nformat: aep.planning-md/1\nid: story:local\nkind: story\n\
                     status: draft\ntitle: Local\nrelations: []\nrevision: 1\n---\n";

const POINTER: &str = "---\nformat: aep.planning-md/1\nid: story:pointer\nkind: story\n\
                       status: draft\ntitle: Pointer\nrelations:\n\
                       - informed_by: mine/story:local\nrevision: 1\n---\n";

fn project_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aep-self-member-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write_store(project: &Path) {
    let engineering = project.join(".engineering");
    let planning = engineering.join("planning");
    std::fs::create_dir_all(planning.join("story")).expect("planning source");
    std::fs::create_dir_all(project.join("protocols")).expect("protocol source");
    std::fs::write(
        engineering.join("project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
    )
    .expect("legacy selector");
    std::fs::write(engineering.join("workspace.yaml"), DECLARATION).expect("declaration");
    std::fs::write(planning.join("story/local.md"), LOCAL).expect("local source");
    std::fs::write(planning.join("story/pointer.md"), POINTER).expect("pointer source");
}

fn dry_run(project: &Path) -> serde_json::Value {
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
            "planning-review-2",
            "--authority-tenant",
            "tenant-review-2",
            "--authority-new",
            "--format",
            "json",
        ])
        .output()
        .expect("the built `aep` binary runs");
    serde_json::from_slice(&output.stdout).expect("--format json is JSON")
}

#[test]
fn an_edge_into_this_store_s_own_declared_member_is_not_a_workspace_crossing() {
    let project = project_root();
    write_store(&project);
    let json = dry_run(&project);
    let _ = std::fs::remove_dir_all(&project);

    assert_eq!(
        json["outcome"]["kind"], "admitted",
        "the declaration admits the store: {json}"
    );
    let inventory = &json["outcome"]["value"]["inventory"];
    let relations = inventory["relations"]
        .as_u64()
        .expect("the receipt carries a relation count");
    let crossings = inventory["workspace_crossings"]
        .as_u64()
        .expect("the receipt carries a crossing count");

    assert_eq!(
        (relations, crossings),
        (1, 0),
        "`mine/story:local` is this store's own artifact under its own declared member name, so \
         the one edge is a relation record the migration imports and nothing crosses out of this \
         store; the receipt reports relations={relations}, workspace_crossings={crossings}: \
         {inventory}"
    );
}
