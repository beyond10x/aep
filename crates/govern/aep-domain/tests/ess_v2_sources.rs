//! Raw source transport never supplies admission authority.
use aep_domain::Evidence;

#[test]
fn raw_source_transport_has_no_admitted_facts_or_model() {
    let wire = r#"{"kind":"ess_conformance_v2","report_json":"untrusted report bytes","suite_json":"untrusted suite bytes"}"#;
    let evidence: Evidence =
        serde_json::from_str(wire).expect("raw transport admits only its shape");
    assert!(evidence.facts().is_empty());
    assert!(evidence.spec_digest().is_none());
    assert!(evidence.summary().contains("unadmitted"));
    assert_eq!(serde_json::to_string(&evidence).unwrap(), wire);
}
