//! The dry-run receipt counts every authored relation exactly once, and says which kind it is.
//!
//! `story:migration-mapper-reads-the-declared-workspace`, amended acceptance, item 2.
//!
//! Before this, `inventory.relations` was counted from the source frontmatter while the migration
//! imported only the relations whose target is itself a captured document. On the real AEP store —
//! 314 artifacts, 564 authored relations, exactly one of them a crossing into a declared member —
//! the receipt promised **564** and the authority received **563**, and no count taken afterwards
//! revealed it, because the post-migration read agrees with 564 through the retained body copy.
//!
//! The fixture below is that store's shape: 314 documents, 563 local edges and one declared
//! crossing. 564 must be reported as **563 + 1**, not as 564 against 563.

use std::path::{Path, PathBuf};
use std::process::Command;

const DOCUMENTS: usize = 314;
const RELATIONS: u64 = 564;

const DECLARATION: &str =
    "version: aep.workspace/1\nmembers:\n  - name: other\n    source: ../other\n";

fn project_root() -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("aep-counts-every-relation-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A store shaped like the AEP planning store: many local edges, exactly one crossing.
fn write_store(project: &Path) -> u64 {
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

    let mut authored = 0_u64;
    for index in 0..DOCUMENTS {
        let mut relations = String::new();
        if index == 0 {
            // The one document the whole unit exists for: `story:assemble-across-sources` and its
            // `- informed_by: entity-runtime/story:typed-references`.
            relations.push_str("  - informed_by: other/story:theirs\n");
            authored += 1;
        } else {
            relations.push_str("  - informed_by: story:s000\n");
            authored += 1;
            // 250 second edges, so the authored total is the real store's 564.
            if index <= 250 {
                relations.push_str("  - informed_by: story:s313\n");
                authored += 1;
            }
        }
        std::fs::write(
            planning.join(format!("story/s{index:03}.md")),
            format!(
                "---\nformat: aep.planning-md/1\nid: story:s{index:03}\nkind: story\n\
                 status: draft\ntitle: Story {index}\nrelations:\n{relations}revision: 1\n---\n"
            ),
        )
        .expect("planning source document");
    }
    authored
}

/// `story:migration-mapper-reads-the-declared-workspace`, amended acceptance, item 2.
///
/// The receipt is what a cutover operator records as the thing that must come out the other side.
/// It may not promise a relation record the migration will not import, and the crossing it does
/// not import may not simply vanish from the count — so the two figures are reported separately
/// and must add up to what the source declares.
#[test]
fn the_receipt_splits_relation_records_from_workspace_crossings_and_they_sum_to_the_source() {
    let project = project_root();
    let authored = write_store(&project);
    assert_eq!(authored, RELATIONS, "the fixture is the real store's shape");

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
            "planning-counts",
            "--authority-tenant",
            "tenant-counts",
            "--authority-new",
            "--format",
            "json",
        ])
        .output()
        .expect("the built `aep` binary runs");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--format json is JSON");
    let text = Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(&project)
        .args([
            "plan",
            "store",
            "migrate",
            "dry-run",
            "--project",
            ".engineering/project.yaml",
            "--authority-scope",
            "planning-counts",
            "--authority-tenant",
            "tenant-counts",
            "--authority-new",
            "--format",
            "text",
        ])
        .output()
        .expect("the built `aep` binary runs");
    let rendered = String::from_utf8_lossy(&text.stdout).into_owned();

    let _ = std::fs::remove_dir_all(&project);

    assert_eq!(
        json["outcome"]["kind"], "admitted",
        "the declared crossing is admitted: {json}"
    );
    let inventory = &json["outcome"]["value"]["inventory"];
    let relations = inventory["relations"]
        .as_u64()
        .expect("the receipt counts relation records");
    let crossings = inventory["workspace_crossings"]
        .as_u64()
        .expect("the receipt counts workspace crossings");

    assert_eq!(
        crossings, 1,
        "the one declared crossing is counted as a crossing: {inventory}"
    );
    assert_eq!(
        relations,
        RELATIONS - 1,
        "the relations the migration imports are counted as relations: {inventory}"
    );
    assert_eq!(
        relations + crossings,
        RELATIONS,
        "every authored relation is counted exactly once: {inventory}"
    );

    // A number nobody reads is not a correction: the operator reading the default rendering has
    // to see the crossings named there too.
    assert!(
        rendered
            .lines()
            .any(|line| line.contains("inventory.workspace_crossings")),
        "the text rendering names the crossings: {rendered}"
    );
}
