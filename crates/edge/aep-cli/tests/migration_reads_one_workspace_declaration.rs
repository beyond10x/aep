//! Independent verification of `story:migration-mapper-reads-the-declared-workspace`, clause 1
//! and the bound in `## Why`, measured through the built binary rather than in process.
//!
//! The defect being fixed is *two views of one rule disagreeing*. The unit's own cases call the
//! mapper and the command directly and build their expected member list with a second, hand-written
//! copy of `declared_members` (`store_command.rs:2902`). These cases know nothing about either:
//! they put a `workspace.yaml` somewhere on disk and ask whether `aep plan store migrate dry-run`
//! read it, which is the only question a cutover operator can ask.
//!
//! Each case names the one-line mutation it stands in for, since a mutation sweep can only be
//! re-run by editing the source under review and this pass does not edit it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("aep-one-declaration-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

const DECLARATION: &str =
    "version: aep.workspace/1\nmembers:\n  - name: other\n    source: ../other\n";

/// A v1 Markdown store with one story crossing into `other`, and `workspace.yaml` written at
/// `project/<at>` — `at` is relative to the project root, so a case can put the declaration in the
/// right place or in the place the `.engineering/.engineering` bug would have looked.
fn store_with_declaration_at(project: &Path, at: Option<&str>) {
    let engineering = project.join(".engineering");
    let planning = engineering.join("planning");
    std::fs::create_dir_all(planning.join("story")).expect("planning source");
    std::fs::create_dir_all(project.join("protocols")).expect("protocol source");
    std::fs::write(
        engineering.join("project.yaml"),
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
    )
    .expect("legacy selector");
    if let Some(at) = at {
        let path = project.join(at);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("declaration directory");
        std::fs::write(path, DECLARATION).expect("workspace declaration");
    }
    std::fs::write(
        planning.join("story/crossing.md"),
        "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\nstatus: draft\n\
         title: A story that names another repository\nrelations:\n\
         - informed_by: other/story:theirs\nrevision: 1\n---\n# Story\n\nBody.\n",
    )
    .expect("crossing planning source");
}

fn dry_run_from(cwd: &Path, selector: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(cwd)
        .args([
            "plan",
            "store",
            "migrate",
            "dry-run",
            "--project",
            selector,
            "--authority-scope",
            "planning-review-1",
            "--authority-tenant",
            "tenant-review-1",
            "--authority-new",
            "--format",
            "json",
        ])
        .output()
        .expect("the built `aep` binary runs")
}

fn outcome(output: &Output) -> String {
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--format json is JSON");
    json["outcome"]["kind"]
        .as_str()
        .unwrap_or("<absent>")
        .to_owned()
}

/// Stands in for M1 — `graph_in_workspace(members)` reverted to `graph_in_workspace(empty())`.
///
/// The declaration beside the store is read, and the crossing it makes checkable is admitted.
#[test]
fn a_declaration_beside_the_store_admits_the_crossing() {
    let project = scratch("declared");
    store_with_declaration_at(&project, Some(".engineering/workspace.yaml"));
    let output = dry_run_from(&project, ".engineering/project.yaml");
    assert_eq!(
        outcome(&output),
        "admitted",
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let _ = std::fs::remove_dir_all(&project);
}

/// Stands in for M4 — `declared_members(engineering.parent())` reverted to
/// `declared_members(&engineering)`, the `.engineering/.engineering` bug the codebase already
/// records once in `planning.rs:2211`.
///
/// A declaration written where that bug would have looked is **not** the store's declaration, so
/// the crossing is still a dangling edge and the migration is still refused.
#[test]
fn a_declaration_one_level_too_deep_is_not_this_store_s_declaration() {
    let project = scratch("too-deep");
    store_with_declaration_at(&project, Some(".engineering/.engineering/workspace.yaml"));
    let output = dry_run_from(&project, ".engineering/project.yaml");
    assert_eq!(
        outcome(&output),
        "refused",
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let _ = std::fs::remove_dir_all(&project);
}

/// The bound the story sets in `## Why`: the fix must not make the mapper ignore crossings.
///
/// No declaration at all, and a declaration naming a *different* member, both leave the edge
/// pointing at nothing, and both are still refused.
#[test]
fn an_undeclared_or_misspelled_member_is_still_a_dangling_edge() {
    for (name, at, declaration) in [
        ("absent", None, None),
        (
            "misspelled",
            Some(".engineering/workspace.yaml"),
            Some("version: aep.workspace/1\nmembers:\n  - name: otherr\n    source: ../otherr\n"),
        ),
    ] {
        let project = scratch(name);
        store_with_declaration_at(&project, at);
        if let Some(declaration) = declaration {
            std::fs::write(project.join(".engineering/workspace.yaml"), declaration)
                .expect("declaration");
        }
        let output = dry_run_from(&project, ".engineering/project.yaml");
        assert_eq!(
            outcome(&output),
            "refused",
            "a member nobody declared leaves the edge dangling ({name}): {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let _ = std::fs::remove_dir_all(&project);
    }
}

/// The declaration is found from an absolute `--project`, run from an unrelated directory.
///
/// `Resolved::declared_members` derives the project root from the selector path
/// (`store_command.rs:1710`), while every ordinary read command derives it from `--store` or by
/// discovery from the working directory (`planning.rs:143`). Two derivations of one root is the
/// shape of the defect this unit fixed, so the one the migration uses is pinned here: a reader
/// that resolved the declaration against the working directory would fail this and pass the case
/// above.
#[test]
fn the_declaration_is_found_from_an_absolute_selector_run_from_elsewhere() {
    let project = scratch("absolute");
    store_with_declaration_at(&project, Some(".engineering/workspace.yaml"));
    let elsewhere = scratch("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("an unrelated working directory");
    let selector = project.join(".engineering/project.yaml");
    let output = dry_run_from(&elsewhere, &selector.display().to_string());
    assert_eq!(
        outcome(&output),
        "admitted",
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let _ = std::fs::remove_dir_all(&project);
    let _ = std::fs::remove_dir_all(&elsewhere);
}
