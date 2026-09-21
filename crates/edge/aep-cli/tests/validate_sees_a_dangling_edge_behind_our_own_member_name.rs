//! `plan artifact validate` says *valid* for a store whose only edge points at nothing.
//!
//! The read-path half of the same hole the mapper has. `validate_edges`
//! (`aep-domain/src/artifact.rs:2526-2533`) exempts a target naming a declared member from the
//! "the manifest does not declare it" check, and the predicate it asks
//! (`crosses_to_a_declared_member`, `aep-domain/src/artifact.rs:1324`) has no notion of which
//! member this store is. A workspace that declares itself — which this repository's own
//! `.engineering/workspace.yaml` does, naming `engineering-protocols` with `source: ..` — puts
//! every dangling edge one prefix away from passing.
//!
//! Origin: the exemption is byte-identical at `763d195be`
//! (`git show 763d195be:crates/govern/aep-domain/src/artifact.rs`, lines 2512-2516), and
//! `plan artifact validate` passed `declared_members` there too (`planning.rs:5412-5417` at base),
//! so this half is **pre-existing** and is not something the unit under review introduced. It is
//! recorded because the unit lifted that predicate into a shared function and made the migration a
//! second caller of it, which is the change that turns a read-path blind spot into a migration
//! that imports a store with an edge that dangles for real.

use std::path::{Path, PathBuf};
use std::process::Command;

const DECLARATION: &str = "version: aep.workspace/1\nmembers:\n  - name: mine\n    source: ..\n";

const MISSING: &str = "story:typo-that-does-not-exist";

fn project_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aep-validate-self-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write_store(project: &Path, target: &str) {
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
    std::fs::write(
        planning.join("story/pointer.md"),
        format!(
            "---\nformat: aep.planning-md/1\nid: story:pointer\nkind: story\nstatus: draft\n\
             title: Pointer\nrelations:\n- informed_by: {target}\nrevision: 1\n---\n"
        ),
    )
    .expect("pointer source");
}

fn validate(project: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(project)
        .args(["plan", "artifact", "validate", "--format", "text"])
        .output()
        .expect("the built `aep` binary runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The control: the identical store with the identical declaration and an unprefixed target.
#[test]
fn an_unprefixed_dangling_edge_is_reported() {
    let project = project_root("plain");
    write_store(&project, MISSING);
    let text = validate(&project);
    let _ = std::fs::remove_dir_all(&project);
    assert!(
        text.contains("undeclared_reference"),
        "a dangling edge is a problem the whole-plan check reports: {text}"
    );
}

/// The same dangling edge behind this store's own declared member name is reported as valid.
#[test]
fn a_dangling_edge_behind_this_store_s_own_member_name_is_reported_too() {
    let project = project_root("prefixed");
    write_store(&project, &format!("mine/{MISSING}"));
    let text = validate(&project);
    let _ = std::fs::remove_dir_all(&project);
    assert!(
        text.contains("undeclared_reference"),
        "`mine/{MISSING}` names this store's own member, so it is this store's own `{MISSING}`, \
         which no document declares; the whole-plan check must report it exactly as it reports \
         the unprefixed spelling. It reported: {text}"
    );
}
