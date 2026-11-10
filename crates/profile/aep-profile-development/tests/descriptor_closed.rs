//! The specification descriptor's closed set is the one `artifacts/lifecycles/specification.yaml`
//! declares, which is what `aep plan artifact lifecycle specification` prints.
//!
//! The descriptor reproduces that ladder by hand ("reproduced rather than invented"), so nothing
//! but a test ties the two together.

use std::collections::BTreeSet;
use std::path::Path;

use aep_domain::artifact::ArtifactStatus;

/// The `closed: [...]` line of the shipped specification ladder, as statuses.
fn declared_closed() -> BTreeSet<ArtifactStatus> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../artifacts/lifecycles/specification.yaml");
    let text = std::fs::read_to_string(&path).expect("the specification ladder reads");
    let line = text
        .lines()
        .find(|line| line.starts_with("closed:"))
        .expect("the specification ladder declares `closed:`");
    line.trim_start_matches("closed:")
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|name| ArtifactStatus::parse(name.trim()).expect("a status"))
        .collect()
}

#[test]
fn the_specification_descriptor_closes_what_its_ladder_file_closes() {
    let descriptor = aep_profile_development::type_descriptors()
        .into_iter()
        .find(|descriptor| {
            descriptor.entity_type.to_string() == aep_profile_development::body::SPECIFICATION_TYPE
        })
        .expect("the specification is described");
    let lifecycle = descriptor.lifecycle.expect("with a lifecycle");

    let mut expected = declared_closed();
    expected.extend(
        lifecycle
            .transitions
            .iter()
            .filter(|(_, to)| to.is_empty())
            .map(|(from, _)| from.clone()),
    );
    let published: BTreeSet<ArtifactStatus> = lifecycle.closed.iter().cloned().collect();

    assert_eq!(published.len(), lifecycle.closed.len(), "no duplicates");
    assert_eq!(
        published, expected,
        "the descriptor's closed set and the ladder file's differ"
    );
}
