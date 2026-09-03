//! The loader is an edge: the semantic engine cannot reacquire filesystem or harness concerns.

use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
}

#[test]
fn the_engine_normal_graph_has_no_schema_driver_or_acquisition_dependency() {
    let root = workspace();
    let manifest = std::fs::read_to_string(root.join("crates/govern/aep-engine/Cargo.toml"))
        .expect("the engine manifest is readable");
    let normal = manifest
        .split("[dev-dependencies]")
        .next()
        .expect("the normal dependency section");
    for refused in ["aep-schema", "aep-driver-spec", "aep-project", "sha2"] {
        assert!(
            !normal.contains(refused),
            "the semantic engine's normal graph still names edge dependency `{refused}`"
        );
    }

    let library = std::fs::read_to_string(root.join("crates/govern/aep-engine/src/lib.rs"))
        .expect("the engine root is readable");
    assert!(!library.contains("pub mod load"));
    assert!(!library.contains("pub mod project"));
    assert!(!root.join("crates/govern/aep-engine/src/load.rs").exists());
    assert!(!root
        .join("crates/govern/aep-engine/src/project.rs")
        .exists());
}
