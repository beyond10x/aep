//! Every ladder this repository ships becomes an entity type the kernel holds, with typed content
//! and an edit, and every move the ladder declares is still an operation of it.

use std::fs;
use std::path::{Path, PathBuf};

use aep_backend_entity::definition::{planning_definition, EDIT};
use aep_domain::artifact::ArtifactLifecycle;
use entity_core::Registry;

fn lifecycles_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../artifacts/lifecycles")
}

#[test]
fn every_shipped_ladder_is_an_entity_type_with_its_moves_and_an_edit() {
    let mut read = 0;
    for entry in fs::read_dir(lifecycles_dir()).expect("the lifecycle directory") {
        let path = entry.expect("an entry").path();
        if path.extension().is_none_or(|extension| extension != "yaml") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a readable lifecycle");
        let lifecycle: ArtifactLifecycle = serde_yaml::from_str(&text)
            .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()));
        let definition = planning_definition(lifecycle.kind.as_ref(), &lifecycle)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(
            definition.operations.contains_key(EDIT),
            "{} has no edit",
            path.display()
        );
        for targets in lifecycle.transitions.values() {
            for to in targets {
                assert!(
                    definition.operations.contains_key(to.as_str()),
                    "{}: the move to {} is not an operation",
                    path.display(),
                    to.as_str()
                );
            }
        }
        for field in ["title", "body", "relations", "tags", "extra"] {
            assert!(
                definition.schema.fields.contains_key(field),
                "{}: no `{field}` field",
                path.display()
            );
        }
        Registry::new()
            .register(definition)
            .unwrap_or_else(|error| panic!("{}: the kernel refuses it: {error}", path.display()));
        read += 1;
    }
    assert!(read >= 8, "{read} ladders read");
}
