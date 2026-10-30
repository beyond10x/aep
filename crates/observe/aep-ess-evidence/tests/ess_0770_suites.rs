//! The suite majors ESS 0.77.0 writes (story:evidence-admits-ess-0770-suites): the pair /58 and
//! /59 that ESS 0.70.0 added (`exact_decimal`), each read for what AEP reads of it, and a major
//! newer than the build lists refused by name.
//!
//! The pair's new vocabulary, a `Decimal` in an exact observer (a complete subject or a retained
//! result), sits in scenario steps, which AEP does not read past suite/5. What /58 and /59 change
//! that AEP does read is the seed record: ESS 0.77.0 `synthesis_seeds::admitted_in` admits it in
//! `exact_decimal::ADMITTED` too. ESS 0.77.0 added no provenance member, root member, scenario-id
//! form or refusal code.
//!
//! The files under `fixtures/ess-0770/` were written by `ess 0.77.0`; its README says how. An
//! ordinary suite goes through `adapt_json_v2`, a coverage suite through `wrap_coverage_suite` and
//! `adapt_json_coverage`: the readers `aep plan artifact evidence --from <report> --suite <suite>`
//! records through.
use aep_domain::ess_conformance_coverage::EssConformanceCoverageReading;
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError, EssConformanceV2Reading};
use aep_domain::Evidence;
use aep_ess_evidence::{
    adapt_json_coverage, adapt_json_v2, suite_route, wrap_coverage_suite, SuiteRoute,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_58: &str = include_str!("fixtures/ess-0770/suite-58.json");
const REPORT_58: &str = include_str!("fixtures/ess-0770/report-58.json");
const SUITE_59: &str = include_str!("fixtures/ess-0770/suite-59.json");
const REPORT_59: &str = include_str!("fixtures/ess-0770/report-59.json");
const SUITE_42_SEEDS: &str = include_str!("fixtures/ess-0770/suite-42-seeds.json");
const REPORT_42_SEEDS: &str = include_str!("fixtures/ess-0770/report-42-seeds.json");
const SUITE_43_SEEDS: &str = include_str!("fixtures/current-suites/suite-43-seeds.json");
const REPORT_43_SEEDS: &str = include_str!("fixtures/current-suites/report-43-seeds-external.json");

/// The scenario whose complete subject carries the `Decimal` that puts the fixture at /58.
const DECIMAL_SUBJECT: &str = "demo.p.Item/state/Closed/refuses/demo.p.Close";

fn digest(original: &str) -> String {
    Sha256::digest(original.as_bytes())
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

fn ordinary(report: &str, suite: &str) -> Result<EssConformanceV2Reading, EssAdmissionError> {
    let adapted = adapt_json_v2(report, suite)?;
    let Evidence::EssConformanceV2(sources) = adapted.evidence() else {
        panic!("the count reader returns count-stage evidence");
    };
    Ok(sources.reading().expect("admitted").clone())
}

fn coverage(report: &str, suite: &str) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
    let adapted = adapt_json_coverage(report, &wrap_coverage_suite(suite)?)?;
    let Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("the coverage reader returns coverage evidence");
    };
    Ok(sources.reading().expect("admitted").clone())
}

fn refusal(error: &EssAdmissionError, reason: &str) -> String {
    let issue = error
        .issues
        .iter()
        .find(|issue| issue.reason == reason)
        .unwrap_or_else(|| panic!("expected {reason}: {error}"));
    format!("{} {}", issue.path, issue.detail)
}

/// `suite` and `report` changed by `change`, with the report's suite reference rebound to the
/// changed suite's version and bytes.
fn rewritten(
    suite: &str,
    report: &str,
    change: impl Fn(&mut Value, &mut Value),
) -> (String, String) {
    let mut suite: Value = serde_json::from_str(suite).unwrap();
    let mut report: Value = serde_json::from_str(report).unwrap();
    change(&mut suite, &mut report);
    let version = suite["provenance"]["suite_version"].clone();
    let suite = serde_json::to_string_pretty(&suite).unwrap();
    report["suite"] = json!({"version": version, "digest_profile": "sha256-json-bytes/1", "digest": digest(&suite)});
    (suite, report.to_string())
}

fn relabelled(suite: &str, report: &str, major: u32) -> (String, String) {
    rewritten(suite, report, |suite, _| {
        suite["provenance"]["suite_version"] = format!("ess-conformance/{major}").into();
    })
}

#[test]
fn an_ordinary_suite_58_ess_0_77_0_wrote_is_admitted_with_its_report() {
    assert_eq!(suite_route(SUITE_58).unwrap(), SuiteRoute::Count);
    let reading = ordinary(REPORT_58, SUITE_58).expect("an ordinary suite/58 ESS wrote must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/58");
    assert_eq!(data.suite.digest(), digest(SUITE_58));
    assert_eq!((data.counts.total, data.counts.passed), (5, 5));
    assert_eq!(data.execution_status, CountStatus::Passed);
    assert!(reading
        .selected_ids()
        .iter()
        .any(|id| id.as_str() == DECIMAL_SUBJECT));
}

#[test]
fn a_coverage_suite_59_ess_0_77_0_wrote_is_admitted_with_its_report() {
    assert_eq!(suite_route(SUITE_59).unwrap(), SuiteRoute::Coverage);
    let reading = coverage(REPORT_59, SUITE_59).expect("a coverage suite/59 ESS wrote must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/59");
    assert_eq!(data.suite.digest(), digest(SUITE_59));
    assert_eq!((data.counts.total, data.counts.passed), (5, 5));
    assert_eq!(data.conformance_status, CountStatus::Passed);
    assert!(reading
        .selected_ids()
        .iter()
        .any(|id| id.as_str() == DECIMAL_SUBJECT));
}

#[test]
fn a_major_newer_than_ess_0_77_0_is_refused_by_name_never_as_an_unknown_member() {
    // ESS 0.77.0 `admission.rs` `validate_suite`: "execution readers admit suite majors 1–59".
    for (suite, report, to) in [
        (SUITE_58, REPORT_58, "ess-conformance/60"),
        (SUITE_59, REPORT_59, "ess-conformance/61"),
    ] {
        // A later major may add a root or provenance member; the version is named before any
        // member is read.
        let (suite, report) = rewritten(suite, report, |suite, _| {
            suite["provenance"]["suite_version"] = to.into();
            suite["provenance"]["later_member"] = json!("x");
            suite["later"] = json!([]);
        });
        let error = suite_route(&suite).expect_err(to);
        assert!(refusal(&error, "UnsupportedSuiteVersion").contains(to));
        let error = if to.ends_with('1') {
            wrap_coverage_suite(&suite).expect_err(to)
        } else {
            ordinary(&report, &suite).expect_err(to)
        };
        assert!(
            refusal(&error, "UnsupportedSuiteVersion").contains(to),
            "{error}"
        );
        assert!(
            error
                .issues
                .iter()
                .all(|issue| issue.reason != "UnknownField"),
            "{to}: {error}"
        );
    }
}

#[test]
fn a_seed_record_is_admitted_in_suite_58_and_59_as_ess_0_77_0_admits_it() {
    // ESS 0.77.0 `synthesis_seeds::admitted_in` adds `exact_decimal::ADMITTED` (58, 59); its
    // `OUTSIDE` names "/58 or /59". The ordinary record is the one ess 0.77.0 wrote at /42, and
    // ess 0.77.0 runs it relabelled /58 (the fixture README).
    let reading = ordinary(REPORT_42_SEEDS, SUITE_42_SEEDS)
        .expect("the seed record ess 0.77.0 wrote at suite/42 must admit");
    assert_eq!(
        (reading.data().counts.total, reading.data().counts.passed),
        (8, 8)
    );
    let (suite, report) = relabelled(SUITE_42_SEEDS, REPORT_42_SEEDS, 58);
    let reading = ordinary(&report, &suite)
        .unwrap_or_else(|error| panic!("a seed record in suite/58 must admit: {error}"));
    assert_eq!(reading.data().suite.version(), "ess-conformance/58");
    let (suite, report) = relabelled(SUITE_43_SEEDS, REPORT_43_SEEDS, 59);
    let reading = coverage(&report, &suite)
        .unwrap_or_else(|error| panic!("a seed record in suite/59 must admit: {error}"));
    assert_eq!(reading.data().suite.version(), "ess-conformance/59");
    // Below the seed pair it is still a later major's vocabulary, as ess 0.77.0 refuses it.
    let (suite, report) = relabelled(SUITE_42_SEEDS, REPORT_42_SEEDS, 40);
    let located = refusal(
        &ordinary(&report, &suite).expect_err("a seed record in suite/40"),
        "UnsupportedVocabulary",
    );
    assert!(
        located.starts_with("$suite.provenance.synthesis_seeds") && located.contains("suite/42"),
        "{located}"
    );
}
