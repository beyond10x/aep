//! The runbook descriptor's closed set follows the rule every ladder follows: declared closed
//! states plus every state with no successor. The runbook ladder declares none, so its closed set
//! is exactly its terminal states.

use std::collections::BTreeSet;

use aep_domain::artifact::ArtifactStatus;

#[test]
fn the_runbook_descriptor_closes_exactly_its_terminal_states() {
    let descriptor = aep_profile_operations::type_descriptors()
        .into_iter()
        .find(|descriptor| descriptor.entity_type.to_string() == "aop.runbook/v1")
        .expect("the runbook is described");
    let lifecycle = descriptor.lifecycle.expect("with a lifecycle");

    let terminal: BTreeSet<ArtifactStatus> = lifecycle
        .transitions
        .iter()
        .filter(|(_, to)| to.is_empty())
        .map(|(from, _)| from.clone())
        .collect();
    let published: BTreeSet<ArtifactStatus> = lifecycle.closed.iter().cloned().collect();

    assert_eq!(published, BTreeSet::from([ArtifactStatus::Archived]));
    assert_eq!(published, terminal);
}
