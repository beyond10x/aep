//! Ordinary suites current ESS writes by default: an even major from /6 on, with no `coverage`
//! block, read as a count-stage pair and bound to its report by version and original-byte digest.
//!
//! The pair under `fixtures/ordinary-suites/` was written by `ess 0.57.0`; its README says how.
//! Each admission goes through `adapt_json_v2`, the reader `aep plan artifact evidence --from
//! <report> --suite <suite>` records an ordinary suite through.
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError, EssConformanceV2Reading};
use aep_domain::Evidence;
use aep_ess_evidence::adapt_json_v2;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_34: &str = include_str!("fixtures/ordinary-suites/suite-34-aep.json");
const REPORT_34: &str = include_str!("fixtures/ordinary-suites/report-34-aep.json");

fn digest(original: &str) -> String {
    Sha256::digest(original.as_bytes())
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

fn admit(report: &str, suite: &str) -> Result<EssConformanceV2Reading, EssAdmissionError> {
    let adapted = adapt_json_v2(report, suite)?;
    let Evidence::EssConformanceV2(sources) = adapted.evidence() else {
        panic!("the count reader returns count-stage evidence");
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

/// The fixture suite relabelled to another version, with the report's reference recomputed so
/// only the version differs.
fn relabelled(to: &str) -> (String, String) {
    let marker = |version: &str| format!("\"suite_version\": \"{version}\"");
    assert!(
        SUITE_34.contains(&marker("ess-conformance/34")),
        "fixture spells ess-conformance/34"
    );
    let suite = SUITE_34.replace(&marker("ess-conformance/34"), &marker(to));
    let mut report: Value = serde_json::from_str(REPORT_34).unwrap();
    report["suite"] =
        json!({"version": to, "digest_profile": "sha256-json-bytes/1", "digest": digest(&suite)});
    (suite, report.to_string())
}

#[test]
fn an_ordinary_suite_ess_wrote_is_admitted_with_its_report() {
    let reading = admit(REPORT_34, SUITE_34).expect("an ordinary suite/34 ESS wrote must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/34");
    assert_eq!(data.suite.digest(), digest(SUITE_34));
    assert_eq!(data.specification, "aep/v1");
    assert_eq!(
        (
            data.counts.total,
            data.counts.passed,
            data.counts.unsupported
        ),
        (11, 5, 6)
    );
    assert_eq!(reading.selected_ids().len(), 11);
    // Six unsupported scenarios fail the run, and with no coverage nothing can lift that.
    assert_eq!(data.execution_status, CountStatus::Failed);
    assert_eq!(data.conformance_status, CountStatus::Failed);
    assert_eq!(data.completed_at.epoch_millis(), 1_700_000_003_700);
}

#[test]
fn an_ordinary_suite_whose_bytes_differ_from_the_report_digest_is_refused() {
    let changed = format!("{SUITE_34}\n");
    let error = admit(REPORT_34, &changed).expect_err("changed suite bytes");
    assert!(
        refusal(&error, "SuiteDigestMismatch").contains(&digest(&changed)),
        "{error}"
    );
}

#[test]
fn a_report_naming_another_ordinary_version_than_its_suite_is_refused() {
    let mut report: Value = serde_json::from_str(REPORT_34).unwrap();
    report["suite"]["version"] = "ess-conformance/36".into();
    let error = admit(&report.to_string(), SUITE_34).expect_err("versions differ");
    let located = refusal(&error, "SuiteVersionMismatch");
    assert!(
        located.starts_with("$suite.provenance.suite_version"),
        "{located}"
    );
    assert!(located.contains("ess-conformance/36"), "{located}");
}

#[test]
fn an_ordinary_major_this_build_does_not_know_is_refused_by_name_never_as_a_missing_field() {
    let (suite, report) = relabelled("ess-conformance/46");
    let error = admit(&report, &suite).expect_err("unknown ordinary major");
    assert!(
        refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/46"),
        "{error}"
    );
    assert!(
        error
            .issues
            .iter()
            .all(|issue| issue.reason != "MissingField"),
        "{error}"
    );
}

#[test]
fn an_ordinary_suite_carrying_a_coverage_block_is_refused() {
    let mut suite: Value = serde_json::from_str(SUITE_34).unwrap();
    suite["coverage"] = json!({"knowledge": "unknown"});
    let suite = suite.to_string();
    let mut report: Value = serde_json::from_str(REPORT_34).unwrap();
    report["suite"]["digest"] = digest(&suite).into();
    let error = admit(&report.to_string(), &suite).expect_err("coverage on an ordinary major");
    assert!(
        refusal(&error, "UnknownField").contains("coverage"),
        "{error}"
    );
}

#[test]
fn provenance_an_ordinary_major_does_not_carry_is_refused_as_its_coverage_counterpart_refuses_it() {
    // `scenario_initial_state` arrives at suite/34; an ordinary /32 carrying it is a later
    // major's vocabulary, gated exactly as the coverage reader gates /33.
    let (suite, report) = relabelled("ess-conformance/32");
    let error = admit(&report, &suite).expect_err("initial state below suite/34");
    assert!(
        refusal(&error, "UnsupportedVocabulary").contains("scenario_initial_state"),
        "{error}"
    );
    // And suite/34 and later require it.
    let mut suite: Value = serde_json::from_str(SUITE_34).unwrap();
    suite["provenance"]
        .as_object_mut()
        .unwrap()
        .remove("scenario_initial_state");
    let suite = suite.to_string();
    let mut report: Value = serde_json::from_str(REPORT_34).unwrap();
    report["suite"]["digest"] = digest(&suite).into();
    let error = admit(&report.to_string(), &suite).expect_err("initial state missing");
    assert!(
        refusal(&error, "MissingField").contains("scenario_initial_state"),
        "{error}"
    );
}

#[test]
fn every_known_ordinary_major_admits_and_its_coverage_neighbour_does_not() {
    // `scenario_initial_state` is present from /34 on, so relabel only within that range.
    for major in [34, 36, 38, 40, 44] {
        let version = format!("ess-conformance/{major}");
        let (suite, report) = relabelled(&version);
        let reading = admit(&report, &suite).unwrap_or_else(|error| panic!("{version}: {error}"));
        assert_eq!(reading.data().suite.version(), version);
    }
    // suite/42 requires a seed record (`synthesis_seeds`), which this fixture has none of.
    let (suite, report) = relabelled("ess-conformance/42");
    let error = admit(&report, &suite).expect_err("suite/42 without seeds");
    assert!(
        refusal(&error, "MissingField").contains("synthesis_seeds"),
        "{error}"
    );
    // An odd major is a coverage suite, never read by the count reader.
    let (suite, report) = relabelled("ess-conformance/35");
    let error = admit(&report, &suite).expect_err("coverage major on the count reader");
    assert!(
        refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/35"),
        "{error}"
    );
}
