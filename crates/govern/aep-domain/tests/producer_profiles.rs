//! The producer profiles a report/2 may name: each spelling ESS writes is read and written back
//! exactly, and any other spelling is not a profile. `aep.evidence.ProducerProfile` in
//! `ess/domains/evidence.yaml` lists them.
use aep_domain::ess_conformance_v2::ProducerProfile;

#[test]
fn every_profile_ess_writes_reads_and_writes_back_exactly() {
    for wire in [
        "rust-scenario-status/1",
        "go-scenario-status/1",
        // ESS 0.56.0 and later: the generated Go and TypeScript runners.
        "go-scenario-status/2",
        "external-scenario-status/1",
        "external-scenario-status/1;runner=acme-runner@1.4.0",
    ] {
        let profile = ProducerProfile::from_wire(wire)
            .unwrap_or_else(|| panic!("{wire} is a profile ESS writes"));
        assert_eq!(profile.wire(), wire);
    }
}

#[test]
fn the_two_go_profiles_are_distinct_and_neither_is_external() {
    let first = ProducerProfile::from_wire("go-scenario-status/1").unwrap();
    let second = ProducerProfile::from_wire("go-scenario-status/2")
        .expect("go-scenario-status/2 is a profile ESS writes");
    assert_ne!(first, second);
    assert!(!second.is_external());
}

#[test]
fn a_spelling_ess_does_not_write_is_not_a_profile() {
    for wire in [
        "go-scenario-status/3",
        "go-scenario-status/0",
        "go-scenario-status",
        "go-scenario-status/2 ",
        "go-scenario-status/2;runner=go@1.22",
        "rust-scenario-status/2",
        "typescript-scenario-status/1",
    ] {
        assert_eq!(ProducerProfile::from_wire(wire), None, "{wire}");
    }
}
