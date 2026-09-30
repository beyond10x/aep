//! Coverage suites current ESS writes: bound by original-byte digest, scenario bodies left to ESS.
//!
//! Every fixture was written by `ess 0.48.0`; `fixtures/current-suites/README.md` says how.
use aep_domain::ess_conformance_coverage::EssConformanceCoverageReading;
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError};
use aep_domain::Evidence;
use aep_ess_evidence::{adapt_json_coverage, wrap_coverage_suite};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_27: &str = include_str!("fixtures/current-suites/suite-27.json");
const REPORT_27: &str = include_str!("fixtures/current-suites/report-27-external.json");
const RUN_27: &str = include_str!("fixtures/current-suites/report-27-run.json");
const INPUT_27_SELECTED: &str = include_str!("fixtures/current-suites/input-27-selected.json");
const REPORT_27_SELECTED: &str =
    include_str!("fixtures/current-suites/report-27-selected-external.json");
const SUITE_31: &str = include_str!("fixtures/current-suites/suite-31.json");
const REPORT_31: &str = include_str!("fixtures/current-suites/report-31-external.json");

fn digest(original: &str) -> String {
    Sha256::digest(original.as_bytes())
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

fn admit_raw(
    report: &str,
    suite: &str,
) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
    let input = wrap_coverage_suite(suite)?;
    admit_input(report, &input)
}

fn admit_input(
    report: &str,
    input: &str,
) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
    let adapted = adapt_json_coverage(report, input)?;
    let Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("coverage adapter returns coverage evidence");
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

/// A current suite relabelled to another version, with the report's reference recomputed for it.
fn relabelled(suite: &str, report: &str, from: &str, to: &str) -> (String, String) {
    let marker = |version: &str| format!("\"suite_version\": \"{version}\"");
    assert!(suite.contains(&marker(from)), "fixture spells {from}");
    let suite = suite.replace(&marker(from), &marker(to));
    let mut report: Value = serde_json::from_str(report).unwrap();
    report["suite"] =
        json!({"version": to, "digest_profile": "sha256-json-bytes/1", "digest": digest(&suite)});
    (suite, report.to_string())
}

#[test]
fn a_suite_27_ess_wrote_is_admitted_with_its_report_from_an_external_runner() {
    let reading = admit_raw(REPORT_27, SUITE_27).expect("current ESS suite/27 must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/27");
    assert_eq!(data.suite.digest(), digest(SUITE_27));
    assert_eq!(data.counts.total, 23);
    assert_eq!(data.counts.passed, 23);
    assert_eq!(reading.selected_ids().len(), 23);
    // ESS refused one generated scenario in scope, so every scenario passing is not conformance.
    assert_eq!(data.coverage.counts.refused, 1);
    assert_eq!(data.execution_status, CountStatus::Passed);
    assert_eq!(data.conformance_status, CountStatus::Inconclusive);
}

#[test]
fn a_suite_27_ess_executed_itself_is_admitted_with_its_own_statuses() {
    let reading = admit_raw(RUN_27, SUITE_27).expect("ESS's own suite/27 run must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/27");
    assert_eq!(data.counts.passed, 7);
    assert_eq!(data.counts.unsupported, 16);
    assert_eq!(data.conformance_status, CountStatus::Failed);
}

#[test]
fn an_explicit_suite_27_selection_is_admitted_with_its_original_parent() {
    let reading = admit_input(REPORT_27_SELECTED, INPUT_27_SELECTED)
        .expect("an ESS-selected suite/27 child and its parent must admit");
    assert_eq!(reading.data().suite.version(), "ess-conformance/27");
    assert_eq!(reading.selected_ids().len(), 3);
}

#[test]
fn a_suite_31_ess_wrote_is_admitted_as_passed() {
    let reading = admit_raw(REPORT_31, SUITE_31).expect("current ESS suite/31 must admit");
    assert_eq!(reading.data().suite.version(), "ess-conformance/31");
    assert_eq!(reading.data().conformance_status, CountStatus::Passed);
}

#[test]
fn a_suite_whose_bytes_differ_from_the_report_digest_is_refused() {
    for changed in [
        format!("{SUITE_27}\n"),
        SUITE_27.replacen("\"caller\"", "\"caller\" ", 1),
    ] {
        assert_ne!(changed, SUITE_27);
        let error = admit_raw(REPORT_27, &changed).expect_err("changed suite bytes");
        refusal(&error, "SuiteDigestMismatch");
    }
}

#[test]
fn a_selected_child_whose_parent_bytes_changed_is_refused() {
    let mut input: Value = serde_json::from_str(INPUT_27_SELECTED).unwrap();
    let parent = input["parent_suites"][0].as_str().unwrap().to_owned();
    input["parent_suites"][0] = format!("{parent}\n").into();
    let error = admit_input(REPORT_27_SELECTED, &input.to_string()).expect_err("changed parent");
    refusal(&error, "ParentReferenceMismatch");
}

#[test]
fn a_selected_child_whose_surviving_scenario_body_differs_from_its_parent_is_refused() {
    let mut input: Value = serde_json::from_str(INPUT_27_SELECTED).unwrap();
    let mut child: Value = serde_json::from_str(input["suite_json"].as_str().unwrap()).unwrap();
    let id = child["coverage"]["generated"][0]
        .as_str()
        .unwrap()
        .to_owned();
    child["scenarios"][&id]["purpose"] = "A different obligation".into();
    let child = serde_json::to_string_pretty(&child).unwrap();
    input["suite_json"] = child.clone().into();
    let mut report: Value = serde_json::from_str(REPORT_27_SELECTED).unwrap();
    report["suite"]["digest"] = digest(&child).into();
    let error = admit_input(&report.to_string(), &input.to_string()).expect_err("changed survivor");
    refusal(&error, "ParentScenarioMismatch");
}

#[test]
fn a_suite_version_newer_than_aep_knows_is_refused_naming_it() {
    let (suite, report) = relabelled(
        SUITE_31,
        REPORT_31,
        "ess-conformance/31",
        "ess-conformance/35",
    );
    let error = wrap_coverage_suite(&suite).expect_err("unknown suite version");
    assert!(refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/35"));
    let input = json!({"format":"ess-conformance-input/1", "suite_json":suite, "parent_suites":[]});
    let error = admit_input(&report, &input.to_string()).expect_err("unknown suite version");
    assert!(refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/35"));
    // The report alone names the version as well, beside a suite aep does know.
    let mut report: Value = serde_json::from_str(REPORT_31).unwrap();
    report["suite"]["version"] = "ess-conformance/35".into();
    let error = admit_raw(&report.to_string(), SUITE_31).expect_err("unknown report version");
    let located = refusal(&error, "UnsupportedSuiteVersion");
    assert!(located.starts_with("$.suite.version"), "{located}");
    assert!(located.contains("ess-conformance/35"), "{located}");
}

#[test]
fn an_ordinary_suite_version_without_declared_coverage_is_refused_naming_it() {
    let (suite, _) = relabelled(
        SUITE_31,
        REPORT_31,
        "ess-conformance/31",
        "ess-conformance/30",
    );
    let error = wrap_coverage_suite(&suite).expect_err("ordinary suite version");
    assert!(refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/30"));
}

const SUITE_27_AGGREGATE: &str = include_str!("fixtures/current-suites/suite-27-aggregate.json");
const REPORT_27_AGGREGATE: &str =
    include_str!("fixtures/current-suites/report-27-aggregate-external.json");
const SUITE_17_AGGREGATE: &str = include_str!("fixtures/current-suites/suite-17-aggregate.json");
const REPORT_17_AGGREGATE: &str =
    include_str!("fixtures/current-suites/report-17-aggregate-external.json");
const SUITE_27_RETRY: &str = include_str!("fixtures/current-suites/suite-27-retry.json");
const REPORT_27_RETRY: &str = include_str!("fixtures/current-suites/report-27-retry-external.json");

#[test]
fn aggregate_view_and_final_failure_scenarios_ess_synthesizes_are_admitted() {
    for (report, suite, version, form, count) in [
        (
            REPORT_27_AGGREGATE,
            SUITE_27_AGGREGATE,
            "ess-conformance/27",
            "/aggregate",
            5,
        ),
        (
            REPORT_17_AGGREGATE,
            SUITE_17_AGGREGATE,
            "ess-conformance/17",
            "/aggregate",
            3,
        ),
        (
            REPORT_27_RETRY,
            SUITE_27_RETRY,
            "ess-conformance/27",
            "/binding/final-failure",
            1,
        ),
    ] {
        let reading = admit_raw(report, suite).unwrap_or_else(|error| panic!("{form}: {error}"));
        assert_eq!(reading.data().suite.version(), version);
        let forms = reading
            .selected_ids()
            .iter()
            .filter(|id| id.as_str().ends_with(form))
            .count();
        assert_eq!(forms, count, "{version} {form}");
    }
}

#[test]
fn a_scenario_form_older_than_its_suite_major_is_refused_as_ess_refuses_it() {
    // ESS aggregate::admit_suite and bounded_retry::admit_format: the aggregate id needs /16 or
    // /17, the final-failure binding id /26 or /27.
    for (suite, report, from, to) in [
        (
            SUITE_17_AGGREGATE,
            REPORT_17_AGGREGATE,
            "ess-conformance/17",
            "ess-conformance/15",
        ),
        (
            SUITE_27_RETRY,
            REPORT_27_RETRY,
            "ess-conformance/27",
            "ess-conformance/25",
        ),
    ] {
        let (suite, report) = relabelled(suite, report, from, to);
        let error = admit_raw(&report, &suite).expect_err(to);
        refusal(&error, "UnsupportedVocabulary");
    }
}

const SUITE_7_ACCESSOR_REFUSAL: &str =
    include_str!("fixtures/current-suites/suite-7-accessor-refusal.json");
const REPORT_7_ACCESSOR_REFUSAL: &str =
    include_str!("fixtures/current-suites/report-7-accessor-refusal-external.json");
const SUITE_17_AGGREGATE_REFUSALS: &str =
    include_str!("fixtures/current-suites/suite-17-aggregate-refusals.json");
const REPORT_17_AGGREGATE_REFUSALS: &str =
    include_str!("fixtures/current-suites/report-17-aggregate-refusals-external.json");
const SUITE_5_AUTHORED_REFUSAL: &str =
    include_str!("fixtures/current-suites/suite-5-authored-refusal.json");
const REPORT_5_AUTHORED_REFUSAL: &str =
    include_str!("fixtures/current-suites/report-5-authored-refusal-external.json");

#[test]
fn refusal_codes_ess_added_after_suite_5_are_admitted_with_their_effects() {
    for (report, suite, version, codes) in [
        (
            REPORT_7_ACCESSOR_REFUSAL,
            SUITE_7_ACCESSOR_REFUSAL,
            "ess-conformance/7",
            &["ESS-SYNTH-015"][..],
        ),
        (
            REPORT_17_AGGREGATE_REFUSALS,
            SUITE_17_AGGREGATE_REFUSALS,
            "ess-conformance/17",
            &["ESS-SYNTH-016", "ESS-SYNTH-017"][..],
        ),
        (
            REPORT_5_AUTHORED_REFUSAL,
            SUITE_5_AUTHORED_REFUSAL,
            "ess-conformance/5",
            &["ESS-AUTHOR-037"][..],
        ),
    ] {
        let reading = admit_raw(report, suite).unwrap_or_else(|error| panic!("{version}: {error}"));
        let data = reading.data();
        assert_eq!(data.suite.version(), version);
        for code in codes {
            assert!(
                data.coverage.refused.iter().any(|r| r.code == *code),
                "{version}: {code}"
            );
        }
        // Every in-scope refusal leaves the selection incomplete, whatever the runner reported.
        assert_eq!(data.conformance_status, CountStatus::Inconclusive);
    }
}

#[test]
fn a_refusal_code_older_than_its_suite_major_is_refused_as_ess_refuses_it() {
    // ESS coverage.rs Inventory::validate: ESS-SYNTH-015 needs suite/7, the aggregate refusals
    // ESS-SYNTH-016 and -017 suite/17.
    for (suite, report, from, to) in [
        (
            SUITE_7_ACCESSOR_REFUSAL,
            REPORT_7_ACCESSOR_REFUSAL,
            "ess-conformance/7",
            "ess-conformance/5",
        ),
        (
            SUITE_17_AGGREGATE_REFUSALS,
            REPORT_17_AGGREGATE_REFUSALS,
            "ess-conformance/17",
            "ess-conformance/15",
        ),
    ] {
        let (suite, report) = relabelled(suite, report, from, to);
        let error = admit_raw(&report, &suite).expect_err(to);
        refusal(&error, "UnsupportedVocabulary");
    }
}

#[test]
fn a_refusal_code_ess_does_not_define_is_still_refused() {
    for (from, to) in [
        ("ESS-SYNTH-017", "ESS-SYNTH-018"),
        ("ESS-AUTHOR-037", "ESS-AUTHOR-036"),
        ("ESS-AUTHOR-037", "ESS-AUTHOR-038"),
    ] {
        let (suite, report) = if from.starts_with("ESS-SYNTH") {
            (SUITE_17_AGGREGATE_REFUSALS, REPORT_17_AGGREGATE_REFUSALS)
        } else {
            (SUITE_5_AUTHORED_REFUSAL, REPORT_5_AUTHORED_REFUSAL)
        };
        let marker = |code: &str| format!("\"code\": \"{code}\"");
        let changed = suite.replace(&marker(from), &marker(to));
        assert_ne!(changed, suite);
        let mut report: Value = serde_json::from_str(report).unwrap();
        report["suite"]["digest"] = digest(&changed).into();
        let error = admit_raw(&report.to_string(), &changed).expect_err(to);
        refusal(&error, "UnsupportedRefusalCode");
    }
}
