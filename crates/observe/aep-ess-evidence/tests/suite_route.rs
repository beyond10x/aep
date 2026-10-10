//! A suite goes to the reader for what it is, never by its number alone.
use aep_ess_evidence::{suite_route, SuiteRoute};

const ORDINARY_34: &str = include_str!("fixtures/ordinary-suites/suite-34-aep.json");
const COVERAGE_31: &str = include_str!("fixtures/current-suites/suite-31.json");

fn suite(version: &str) -> String {
    serde_json::json!({"provenance": {"suite_version": version}, "scenarios": {}}).to_string()
}

#[test]
fn a_coverage_major_goes_to_the_coverage_reader_and_an_ordinary_one_to_the_count_reader() {
    assert_eq!(suite_route(COVERAGE_31).unwrap(), SuiteRoute::Coverage);
    assert_eq!(suite_route(ORDINARY_34).unwrap(), SuiteRoute::Count);
    for version in [
        "ess-conformance/1",
        "ess-conformance/4",
        "ess-conformance/6",
        "ess-conformance/44",
    ] {
        let route = suite_route(&suite(version)).unwrap();
        assert_eq!(route, SuiteRoute::Count, "{version}");
    }
    for version in [
        "ess-conformance/5",
        "ess-conformance/35",
        "ess-conformance/45",
    ] {
        let route = suite_route(&suite(version)).unwrap();
        assert_eq!(route, SuiteRoute::Coverage, "{version}");
    }
}

#[test]
fn a_version_neither_reader_knows_is_refused_by_name() {
    for version in [
        "ess-conformance/46",
        "ess-conformance/47",
        "ess-conformance/0",
        "ess-conformance/06",
        "ess-conformance/x",
    ] {
        let error = suite_route(&suite(version)).expect_err(version);
        let issue = &error.issues[0];
        assert_eq!(issue.reason, "UnsupportedSuiteVersion", "{version}");
        assert_eq!(issue.path, "$suite.provenance.suite_version", "{version}");
        assert!(
            issue.detail.contains(version),
            "{version}: {}",
            issue.detail
        );
    }
}

#[test]
fn a_suite_stating_no_version_is_left_to_the_count_reader_to_name() {
    for unstated in [
        "not json",
        "{}",
        r#"{"provenance":{}}"#,
        r#"{"provenance":{"suite_version":4}}"#,
    ] {
        let route = suite_route(unstated).unwrap();
        assert_eq!(route, SuiteRoute::Count, "{unstated}");
    }
}
