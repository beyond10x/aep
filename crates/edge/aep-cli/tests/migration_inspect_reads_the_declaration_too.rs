//! `plan store inspect` reads the same declaration `dry-run` does, and refuses the same way.
//!
//! The unit added three readers of `Resolved::declared_members()` at the command edge —
//! `inspect` (`store_command.rs:906`), `dry_run` (`store_command.rs:1202`) and `mapped_histories`
//! (`store_command.rs:2114`) — each mapping the error to `workspace_refusal()`. The suite the unit
//! shipped covers two of them: `an_unparseable_declaration_is_distinguishable_from_no_declaration`
//! and `store_command::tests::an_unparseable_declaration_is_refused_at_the_workspace_file` both go
//! through `dry-run`, and `cli::an_unparseable_workspace_file_is_refused_naming_the_file_and_the_
//! reason` goes through the read path. Nothing in the tree names `inspect` and `workspace` in one
//! case, so the guard at `store_command.rs:906` is a line that can be deleted — replaced by
//! `unwrap_or_default()` — with the whole suite staying green, and the failure it opens is the
//! silent one: `inspect` would report a crossing as an ordinary relation on a store whose
//! declaration is unreadable, which is the reading this unit exists to remove.
//!
//! This case is the mutant's detector. It is expected to pass against the shipped code.

use std::path::{Path, PathBuf};
use std::process::Command;

const BROKEN: &str = "version: aep.workspace/1\nmembers:\n  - name: other\n    source\n\t- ]]\n";

const CROSSING: &str = "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\n\
                        status: draft\ntitle: Crossing\nrelations:\n\
                        - informed_by: other/story:theirs\nrevision: 1\n---\n";

fn project_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aep-inspect-decl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write_store(project: &Path, declaration: Option<&str>) {
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

fn inspect(project: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(project)
        .args([
            "plan",
            "store",
            "inspect",
            "--project",
            ".engineering/project.yaml",
            "--format",
            "json",
        ])
        .output()
        .expect("the built `aep` binary runs");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "--format json is JSON; stdout was {:?}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn inspect_refuses_an_unparseable_declaration_at_the_workspace_file() {
    let project = project_root("broken");
    write_store(&project, Some(BROKEN));
    let json = inspect(&project);
    let _ = std::fs::remove_dir_all(&project);

    assert_eq!(
        json["outcome"]["kind"], "refused",
        "a `workspace.yaml` that does not parse is unknown, not a declaration of nothing, so \
         `inspect` may not answer from an empty member list: {json}"
    );
    let refusal = &json["outcome"]["value"]["refusals"][0];
    assert_eq!(
        refusal["code"], "invalid_project",
        "the refusal names the project configuration, not the source: {refusal}"
    );
    assert_eq!(
        refusal["at"]["kind"], "config",
        "the refusal coordinate is the config file that is wrong: {refusal}"
    );
    assert_eq!(
        refusal["at"]["value"]["field"], "workspace",
        "the config field named is `workspace`, the file that did not parse: {refusal}"
    );
}

/// The control: with no declaration at all, `inspect` still answers, so the case above is about
/// the unreadable declaration and not about `inspect` refusing everything.
#[test]
fn inspect_still_answers_for_a_store_that_declares_no_members() {
    let project = project_root("absent");
    write_store(&project, None);
    let json = inspect(&project);
    let _ = std::fs::remove_dir_all(&project);

    assert_eq!(
        json["outcome"]["kind"], "observed",
        "a store with no `workspace.yaml` is not an error: {json}"
    );
}
