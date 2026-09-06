//! Coverage transport is a separate raw value, never serialized admission authority.
use aep_domain::requirement::EvidenceRequirement;
use aep_domain::Evidence;

#[test]
fn coverage_raw_transport_preserves_sources_without_projecting_admission() {
    let wire = r#"{"kind":"ess_conformance_coverage_v1","report_json":"original report bytes","suite_input_json":"original input bytes"}"#;
    let evidence: Evidence = serde_json::from_str(wire).expect("the separate raw carrier exists");
    assert!(evidence.facts().is_empty());
    assert!(evidence.spec_digest().is_none());
    assert!(evidence.summary().contains("unadmitted"));
    assert_eq!(serde_json::to_string(&evidence).unwrap(), wire);
}

#[test]
fn coverage_requirement_cannot_use_zero_as_an_admission_bypass() {
    let error = serde_json::from_str::<EvidenceRequirement>(
        r#"{"kind":"ess_conformance_coverage_v1","at_least":0}"#,
    )
    .expect_err("coverage requires a positive number of qualified records");
    assert!(error.to_string().contains("at_least"), "{error}");
}

#[test]
fn coverage_context_free_match_cannot_qualify_raw_source_claims() {
    let evidence: Evidence = serde_json::from_str(r#"{"kind":"ess_conformance_coverage_v1","report_json":"untrusted","suite_input_json":"untrusted"}"#).unwrap();
    let record = aep_domain::evidence::EvidenceRecord {
        id: "raw-coverage".parse().unwrap(),
        value: evidence,
        observed_at: aep_domain::time::ObservedAt::new(aep_domain::time::Timestamp::EPOCH),
        produced_at: aep_domain::time::Timestamp::EPOCH,
        producer: aep_domain::evidence::Producer::Verifier {
            verifier: aep_domain::verification::Verifier::ConformanceRunner,
        },
        subject: None,
        provenance: aep_domain::evidence::Provenance::default(),
    };
    let required: EvidenceRequirement =
        serde_json::from_str("\"ess_conformance_coverage_v1\"").unwrap();
    assert!(
        !required.matches(&record),
        "coverage requires independent context and admission"
    );
}
