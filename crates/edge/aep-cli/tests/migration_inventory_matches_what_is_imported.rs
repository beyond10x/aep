//! What the dry-run receipt promises against what the migration actually imports.
//!
//! `story:migration-mapper-reads-the-declared-workspace` `## Acceptance` requires that the
//! cross-member relation "survives migration as a relation record". `inventory.relations` in the
//! dry-run receipt is the number a cutover operator reads and records as the thing that must come
//! out the other side; it is counted from the source's frontmatter (`store_command.rs:1941`),
//! while the records that are imported come from `markdown_boundaries_raw`. For a store that
//! declares members and carries a crossing, the two disagree, and nothing in the command compares
//! them.

use std::path::{Path, PathBuf};
use std::process::Command;

use aep_contract::migration::{
    HexBytesV1, HostPathV1, MarkdownNodeKindV1, MarkdownNodeV1, MarkdownRawV1,
    RegularMarkdownNodeV1,
};
use aep_domain::workspace::MemberName;

const DECLARATION: &str =
    "version: aep.workspace/1\nmembers:\n  - name: other\n    source: ../other\n";

const CROSSING: &str = "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\n\
                        status: draft\ntitle: A story that names another repository\nrelations:\n\
                        - informed_by: other/story:theirs\nrevision: 1\n---\n";

fn project_root() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aep-inventory-import-{}", std::process::id()));
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
    std::fs::write(planning.join("story/crossing.md"), CROSSING).expect("crossing source");
}

/// The exact bytes the acquisition retains for that one document, as a raw capture.
fn capture_of(project: &Path) -> MarkdownRawV1 {
    let bytes = std::fs::read(project.join(".engineering/planning/story/crossing.md"))
        .expect("the crossing document is on disk");
    MarkdownRawV1 {
        nodes: vec![MarkdownNodeV1 {
            relative: HostPathV1::Unix(HexBytesV1::new(b"story/crossing.md".to_vec())),
            node: MarkdownNodeKindV1::Regular(RegularMarkdownNodeV1 {
                bytes: HexBytesV1::new(bytes),
            }),
        }],
    }
}

#[test]
fn the_dry_run_inventory_is_the_number_of_relations_the_migration_imports() {
    let project = project_root();
    write_store(&project);

    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(&project)
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
    assert_eq!(
        json["outcome"]["kind"], "admitted",
        "the declared crossing is admitted: {json}"
    );
    let promised = json["outcome"]["value"]["inventory"]["relations"]
        .as_u64()
        .expect("the receipt carries a relation inventory");

    let membership = aep_domain::workspace::Membership::declaring([
        MemberName::parse("other").expect("a member name")
    ]);
    let histories =
        aep_planning_migration::markdown_boundaries_raw(&capture_of(&project), &membership)
            .expect("the same capture the command mapped");
    let imported = histories
        .iter()
        .filter(|history| history.subject.entity == "aep.relation")
        .count() as u64;

    let _ = std::fs::remove_dir_all(&project);

    assert_eq!(
        imported, promised,
        "the dry-run receipt promises {promised} relation(s) and the migration imports {imported}"
    );
}
