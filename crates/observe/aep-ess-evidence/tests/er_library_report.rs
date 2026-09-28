//! An actual ER runner report, kept distinct from authored reader fixtures.
use aep_domain::ess_conformance_v2::CountStatus;
use aep_ess_evidence::{adapt_json_coverage, wrap_coverage_suite};

#[test]
fn the_complete_entity_runtime_library_run_is_admitted_without_rewriting_its_sources() {
    let report = include_str!("fixtures/er_library/report.json");
    let suite = include_str!("fixtures/er_library/suite.json");
    let input = wrap_coverage_suite(suite).unwrap();
    let adapted = adapt_json_coverage(report, &input).unwrap();
    let aep_domain::Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("the actual inventory report must produce typed coverage evidence");
    };
    assert_eq!(sources.report_json(), report);
    let retained: serde_json::Value = serde_json::from_str(sources.suite_input_json()).unwrap();
    assert_eq!(retained["suite_json"].as_str(), Some(suite));
    let reading = sources.reading().unwrap();
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/29");
    assert_eq!(data.counts.total, 414);
    assert_eq!(data.counts.passed, 414);
    assert_eq!(data.counts.failed, 0);
    assert_eq!(data.counts.error, 0);
    assert_eq!(data.counts.unsupported, 0);
    assert_eq!(data.counts.skipped, 0);
    assert_eq!(reading.selected_ids().len(), 414);
    assert_eq!(data.execution_status, CountStatus::Passed);
    assert_eq!(data.conformance_status, CountStatus::Passed);
}
