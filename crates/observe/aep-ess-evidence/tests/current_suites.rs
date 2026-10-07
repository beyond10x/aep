//! Coverage suites current ESS writes: bound by original-byte digest, scenario bodies left to ESS.
//!
//! Every fixture was written by `ess 0.48.0` or `ess 0.55.0`; `fixtures/current-suites/README.md`
//! says how. Each admission goes through `wrap_coverage_suite` and `adapt_json_coverage`, the path
//! `aep plan artifact evidence --from <report> --suite <suite>` records a report through.
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
        "ess-conformance/47",
    );
    let error = wrap_coverage_suite(&suite).expect_err("unknown suite version");
    assert!(refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/47"));
    let input = json!({"format":"ess-conformance-input/1", "suite_json":suite, "parent_suites":[]});
    let error = admit_input(&report, &input.to_string()).expect_err("unknown suite version");
    assert!(refusal(&error, "UnsupportedSuiteVersion").contains("ess-conformance/47"));
    // The report alone names the version as well, beside a suite aep does know.
    let mut report: Value = serde_json::from_str(REPORT_31).unwrap();
    report["suite"]["version"] = "ess-conformance/47".into();
    let error = admit_raw(&report.to_string(), SUITE_31).expect_err("unknown report version");
    let located = refusal(&error, "UnsupportedSuiteVersion");
    assert!(located.starts_with("$.suite.version"), "{located}");
    assert!(located.contains("ess-conformance/47"), "{located}");
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
        ("ESS-AUTHOR-037", "ESS-AUTHOR-042"),
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

#[test]
fn authored_refusal_codes_ess_added_after_0_48_are_admitted_in_any_coverage_major() {
    // ESS 0.55.0 `authored::names_file_refusal` names ESS-AUTHOR-038 through -041 (an act's grant
    // claims and its guards), and `Inventory::validate_refusal` ties no authored code to a major.
    for to in [
        "ESS-AUTHOR-038",
        "ESS-AUTHOR-039",
        "ESS-AUTHOR-040",
        "ESS-AUTHOR-041",
    ] {
        let marker = |code: &str| format!("\"code\": \"{code}\"");
        let changed = SUITE_5_AUTHORED_REFUSAL.replace(&marker("ESS-AUTHOR-037"), &marker(to));
        assert_ne!(changed, SUITE_5_AUTHORED_REFUSAL);
        // The report's coverage summary repeats each refusal, code included.
        let report = REPORT_5_AUTHORED_REFUSAL.replace("\"ESS-AUTHOR-037\"", &format!("\"{to}\""));
        assert_ne!(report, REPORT_5_AUTHORED_REFUSAL);
        let mut report: Value = serde_json::from_str(&report).unwrap();
        report["suite"]["digest"] = digest(&changed).into();
        let reading = admit_raw(&report.to_string(), &changed)
            .unwrap_or_else(|error| panic!("{to}: {error}"));
        assert!(
            reading.data().coverage.refused.iter().any(|r| r.code == to),
            "{to}"
        );
        assert_eq!(reading.data().conformance_status, CountStatus::Inconclusive);
    }
}

/// `SUITE_27_RETRY` and its report with `ids` added as generated scenarios, every one passed, in
/// `version`, with the `scenario_initial_state: empty` ESS requires from suite/34. Each body is a
/// copy of an existing scenario's: above suite/5 AEP reads only the keys and leaves the bodies to
/// ESS, which admitted the suite before reporting on it.
fn with_scenarios(ids: &[&str], version: &str) -> (String, String) {
    let sorted = |list: &mut Value, ids: &[&str]| {
        let mut all: Vec<String> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .chain(ids.iter().map(|id| (*id).to_owned()))
            .collect();
        all.sort();
        *list = json!(all);
        all.len()
    };
    let mut suite: Value = serde_json::from_str(SUITE_27_RETRY).unwrap();
    let body = suite["scenarios"]["demo.ledger.Place/outcome/placed"].clone();
    for id in ids {
        suite["scenarios"][*id] = body.clone();
    }
    let generated = sorted(&mut suite["coverage"]["generated"], ids);
    suite["coverage"]["counts"]["generated"] = generated.into();
    suite["provenance"]["suite_version"] = version.into();
    let major: u32 = version
        .strip_prefix("ess-conformance/")
        .unwrap()
        .parse()
        .unwrap();
    if major >= 34 {
        suite["provenance"]["scenario_initial_state"] = "empty".into();
    }
    let suite = serde_json::to_string_pretty(&suite).unwrap();
    let mut report: Value = serde_json::from_str(REPORT_27_RETRY).unwrap();
    let passed = sorted(&mut report["outcomes"]["passed"], ids);
    report["counts"]["passed"] = passed.into();
    report["counts"]["total"] = passed.into();
    report["coverage"]["counts"]["generated"] = generated.into();
    report["suite"] = json!({"version": version, "digest_profile": "sha256-json-bytes/1", "digest": digest(&suite)});
    (suite, report.to_string())
}

const GRANT_DENIED: &str = "demo.ledger.Place/grant/denied";
const GRANT_ADMITTED: &str = "demo.ledger.Place/grant/admitted/demo.ledger.Clerk";

#[test]
fn command_grant_scenarios_are_admitted_and_recorded_at_every_major_that_carries_them() {
    // ESS grant::ORDINARY 26 and grant::COVERAGE 27 (beyond10x/ess#265), through /45, the newest
    // coverage major ESS 0.55.0 writes. /43 is left out: it requires a seed record, which the
    // seeded fixture below carries.
    for version in [
        "ess-conformance/27",
        "ess-conformance/29",
        "ess-conformance/31",
        "ess-conformance/33",
        "ess-conformance/35",
        "ess-conformance/37",
        "ess-conformance/39",
        "ess-conformance/41",
        "ess-conformance/45",
    ] {
        let (suite, report) = with_scenarios(&[GRANT_DENIED, GRANT_ADMITTED], version);
        let reading =
            admit_raw(&report, &suite).unwrap_or_else(|error| panic!("{version}: {error}"));
        let data = reading.data();
        assert_eq!(data.suite.version(), version);
        assert_eq!(data.suite.digest(), digest(&suite));
        assert_eq!(data.counts.passed, 11);
        for id in [GRANT_DENIED, GRANT_ADMITTED] {
            assert!(
                reading
                    .selected_ids()
                    .iter()
                    .any(|selected| selected.as_str() == id),
                "{version}: {id}"
            );
        }
        assert_eq!(data.conformance_status, CountStatus::Passed, "{version}");
    }
}

#[test]
fn a_scenario_form_below_the_major_that_introduced_it_is_refused_naming_the_form_and_major() {
    // The first ordinary major of each form, from ESS 0.55.0's own constants: grant::ORDINARY
    // (26), view_grant::ORDINARY and one_time_response::ORDINARY (34), refusal_policy::ORDINARY
    // and no_invocation::ORDINARY (36). Each is refused in the coverage major just below its own,
    // and the grant and binding forms in a lower one as well.
    for (id, form, ordinary, below) in [
        (GRANT_DENIED, "grant/denied", 26, &["ess-conformance/25", "ess-conformance/7"][..]),
        (GRANT_ADMITTED, "grant/admitted/<actor>", 26, &["ess-conformance/25", "ess-conformance/7"][..]),
        (
            "demo.ledger.Board/grant/read/denied",
            "grant/read/denied",
            34,
            &["ess-conformance/33"][..],
        ),
        (
            "demo.ledger.Board/grant/read/admitted/demo.ledger.Clerk",
            "grant/read/admitted/<actor>",
            34,
            &["ess-conformance/33"][..],
        ),
        (
            "demo.ledger.Place/disclosure/placed/receipt/origin/as/anonymous",
            "disclosure/...",
            34,
            &["ess-conformance/33"][..],
        ),
        (
            "demo.ledger.Place/disclosure/placed/receipt/denied/demo.ledger.Record/as/actor/demo.ledger.Clerk",
            "disclosure/...",
            34,
            &["ess-conformance/33"][..],
        ),
        (
            "notify-ledger/binding/refusal/rejected",
            "binding/refusal/<outcome>",
            36,
            &["ess-conformance/35", "ess-conformance/33"][..],
        ),
        (
            "notify-ledger/binding/condition-false",
            "binding/condition-false",
            36,
            &["ess-conformance/35", "ess-conformance/33"][..],
        ),
        (
            "notify-ledger/binding/condition-absent",
            "binding/condition-absent",
            36,
            &["ess-conformance/35", "ess-conformance/33"][..],
        ),
    ] {
        for version in below {
            let (suite, report) = with_scenarios(&[id], version);
            let error = admit_raw(&report, &suite).expect_err(id);
            let located = refusal(&error, "UnsupportedVocabulary");
            let expected = format!(
                "{id} uses the `{form}` form, which requires suite/{ordinary} or /{}",
                ordinary + 1
            );
            assert!(located.contains(&expected), "{version}: {located}");
            assert!(located.starts_with("$suite.scenarios"), "{version}: {located}");
        }
    }
}

const SUITE_35_VIEW_GRANTS: &str =
    include_str!("fixtures/current-suites/suite-35-view-grants.json");
const REPORT_35_VIEW_GRANTS: &str =
    include_str!("fixtures/current-suites/report-35-view-grants-external.json");
const SUITE_35_GATEPASS: &str = include_str!("fixtures/current-suites/suite-35-gatepass.json");
const REPORT_35_GATEPASS: &str =
    include_str!("fixtures/current-suites/report-35-gatepass-external.json");
const SUITE_35_DISCLOSURE: &str = include_str!("fixtures/current-suites/suite-35-disclosure.json");
const REPORT_35_DISCLOSURE: &str =
    include_str!("fixtures/current-suites/report-35-disclosure-external.json");
const SUITE_37_REFUSAL_POLICY: &str =
    include_str!("fixtures/current-suites/suite-37-refusal-policy.json");
const REPORT_37_REFUSAL_POLICY: &str =
    include_str!("fixtures/current-suites/report-37-refusal-policy-external.json");
const SUITE_37_BINDING_CONDITION: &str =
    include_str!("fixtures/current-suites/suite-37-binding-condition.json");
const REPORT_37_BINDING_CONDITION: &str =
    include_str!("fixtures/current-suites/report-37-binding-condition-external.json");
const SUITE_39_CONDITIONAL_MEASURES: &str =
    include_str!("fixtures/current-suites/suite-39-conditional-measures.json");
const REPORT_39_CONDITIONAL_MEASURES: &str =
    include_str!("fixtures/current-suites/report-39-conditional-measures-external.json");
const SUITE_41_EXPRESSION: &str = include_str!("fixtures/current-suites/suite-41-expression.json");
const REPORT_41_EXPRESSION: &str =
    include_str!("fixtures/current-suites/report-41-expression-external.json");
const SUITE_43_SEEDS: &str = include_str!("fixtures/current-suites/suite-43-seeds.json");
const REPORT_43_SEEDS: &str = include_str!("fixtures/current-suites/report-43-seeds-external.json");
const INPUT_43_SELECTED: &str = include_str!("fixtures/current-suites/input-43-selected.json");
const REPORT_43_SELECTED: &str =
    include_str!("fixtures/current-suites/report-43-selected-external.json");
const SUITE_45_EVENT_MULTIPLICITY: &str =
    include_str!("fixtures/current-suites/suite-45-event-multiplicity.json");
const REPORT_45_EVENT_MULTIPLICITY: &str =
    include_str!("fixtures/current-suites/report-45-event-multiplicity-external.json");

/// `report` naming the original-byte digest of `suite`, for a suite a test changed.
fn rebound(report: &str, suite: &str) -> String {
    let mut report: Value = serde_json::from_str(report).unwrap();
    report["suite"]["digest"] = digest(suite).into();
    report.to_string()
}

/// `suite` with its provenance changed by `change`, and `report` rebound to the result.
fn with_provenance(suite: &str, report: &str, change: impl Fn(&mut Value)) -> (String, String) {
    let mut value: Value = serde_json::from_str(suite).unwrap();
    change(&mut value["provenance"]);
    let suite = serde_json::to_string_pretty(&value).unwrap();
    let report = rebound(report, &suite);
    (suite, report)
}

/// One suite ESS 0.55.0 wrote, its external report, and what admitting the pair must read.
struct Written {
    suite: &'static str,
    report: &'static str,
    version: &'static str,
    scenarios: u64,
    refusals: u64,
    conformance: CountStatus,
    /// Scenario-key infixes of the later forms the suite carries, with how many keys use each.
    forms: &'static [(&'static str, usize)],
}

const WRITTEN: [Written; 9] = [
    Written {
        suite: SUITE_35_VIEW_GRANTS,
        report: REPORT_35_VIEW_GRANTS,
        version: "ess-conformance/35",
        scenarios: 4,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[
            ("/grant/admitted/", 1),
            ("/grant/read/admitted/", 1),
            ("/grant/read/denied", 1),
        ],
    },
    Written {
        suite: SUITE_35_GATEPASS,
        report: REPORT_35_GATEPASS,
        version: "ess-conformance/35",
        scenarios: 17,
        refusals: 5,
        conformance: CountStatus::Inconclusive,
        forms: &[("/grant/denied", 3)],
    },
    Written {
        suite: SUITE_35_DISCLOSURE,
        report: REPORT_35_DISCLOSURE,
        version: "ess-conformance/35",
        scenarios: 5,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[("/disclosure/", 4)],
    },
    Written {
        suite: SUITE_37_REFUSAL_POLICY,
        report: REPORT_37_REFUSAL_POLICY,
        version: "ess-conformance/37",
        scenarios: 15,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[("/binding/refusal/", 5)],
    },
    Written {
        suite: SUITE_37_BINDING_CONDITION,
        report: REPORT_37_BINDING_CONDITION,
        version: "ess-conformance/37",
        scenarios: 13,
        refusals: 1,
        conformance: CountStatus::Inconclusive,
        forms: &[
            ("/binding/condition-false", 1),
            ("/binding/condition-absent", 1),
        ],
    },
    Written {
        suite: SUITE_39_CONDITIONAL_MEASURES,
        report: REPORT_39_CONDITIONAL_MEASURES,
        version: "ess-conformance/39",
        scenarios: 9,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[("/aggregate", 4)],
    },
    Written {
        suite: SUITE_41_EXPRESSION,
        report: REPORT_41_EXPRESSION,
        version: "ess-conformance/41",
        scenarios: 7,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[],
    },
    Written {
        suite: SUITE_43_SEEDS,
        report: REPORT_43_SEEDS,
        version: "ess-conformance/43",
        scenarios: 8,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[],
    },
    Written {
        suite: SUITE_45_EVENT_MULTIPLICITY,
        report: REPORT_45_EVENT_MULTIPLICITY,
        version: "ess-conformance/45",
        scenarios: 1,
        refusals: 0,
        conformance: CountStatus::Passed,
        forms: &[],
    },
];

#[test]
fn every_coverage_suite_ess_0_55_wrote_is_admitted_with_its_report_from_an_external_runner() {
    for written in &WRITTEN {
        let version = written.version;
        let reading = admit_raw(written.report, written.suite)
            .unwrap_or_else(|error| panic!("{version}: {error}"));
        let data = reading.data();
        assert_eq!(data.suite.version(), version);
        assert_eq!(data.suite.digest(), digest(written.suite), "{version}");
        assert_eq!(data.counts.total, written.scenarios, "{version}");
        assert_eq!(data.counts.passed, written.scenarios, "{version}");
        assert_eq!(
            u64::try_from(reading.selected_ids().len()).unwrap(),
            written.scenarios,
            "{version}"
        );
        assert_eq!(data.coverage.counts.refused, written.refusals, "{version}");
        assert_eq!(data.execution_status, CountStatus::Passed, "{version}");
        assert_eq!(data.conformance_status, written.conformance, "{version}");
        for (form, count) in written.forms {
            let carried = reading
                .selected_ids()
                .iter()
                .filter(|id| id.as_str().contains(form))
                .count();
            assert_eq!(carried, *count, "{version} {form}");
        }
    }
}

#[test]
fn an_explicit_seeded_selection_is_admitted_with_its_original_parent() {
    let reading = admit_input(REPORT_43_SELECTED, INPUT_43_SELECTED)
        .expect("an ESS-selected suite/43 child and its seeded parent must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/43");
    assert_eq!(reading.selected_ids().len(), 2);
    // One seeded scenario survives; the other is outside by the filter, named by its application.
    assert_eq!(data.coverage.counts.outside, 6);
}

#[test]
fn a_selected_child_whose_seed_record_differs_from_its_parent_is_refused() {
    let mut input: Value = serde_json::from_str(INPUT_43_SELECTED).unwrap();
    let (child, report) = with_provenance(
        input["suite_json"].as_str().unwrap(),
        REPORT_43_SELECTED,
        |provenance| {
            let applications = provenance["synthesis_seeds"]["applications"]
                .as_array_mut()
                .unwrap();
            assert_eq!(applications.len(), 2, "the parent's two applications");
            applications.pop();
        },
    );
    input["suite_json"] = child.into();
    let error = admit_input(&report, &input.to_string()).expect_err("changed seed record");
    refusal(&error, "ParentProvenanceMismatch");
}

#[test]
fn scenario_initial_state_is_required_empty_from_suite_34_and_refused_below_it() {
    // ESS 0.55.0 `admission.rs` `validate_suite`: `empty` from suite/34, absent below it.
    let (suite, report) = relabelled(
        SUITE_35_VIEW_GRANTS,
        REPORT_35_VIEW_GRANTS,
        "ess-conformance/35",
        "ess-conformance/33",
    );
    let error = admit_raw(&report, &suite).expect_err("initial state below /34");
    let located = refusal(&error, "UnsupportedVocabulary");
    assert!(
        located.starts_with("$suite.provenance.scenario_initial_state")
            && located.contains("suite/34"),
        "{located}"
    );
    let (suite, report) = with_provenance(SUITE_35_VIEW_GRANTS, REPORT_35_VIEW_GRANTS, |p| {
        p.as_object_mut().unwrap().remove("scenario_initial_state");
    });
    let error = admit_raw(&report, &suite).expect_err("no initial state at /35");
    let located = refusal(&error, "MissingField");
    assert!(
        located.starts_with("$suite.provenance.scenario_initial_state"),
        "{located}"
    );
    let (suite, report) = with_provenance(SUITE_35_VIEW_GRANTS, REPORT_35_VIEW_GRANTS, |p| {
        p["scenario_initial_state"] = "seeded".into();
    });
    let error = admit_raw(&report, &suite).expect_err("unknown initial state");
    let located = refusal(&error, "UnsupportedVocabulary");
    assert!(located.contains("empty"), "{located}");
}

#[test]
fn a_seed_record_is_required_in_suite_43_admitted_in_45_and_refused_elsewhere() {
    // ESS 0.55.0 `synthesis_seeds::seed_major` (/42, /43) and `admitted_in` (and
    // `event_multiplicity::ADMITTED`, /44, /45).
    let (suite, report) = relabelled(
        SUITE_43_SEEDS,
        REPORT_43_SEEDS,
        "ess-conformance/43",
        "ess-conformance/45",
    );
    admit_raw(&report, &suite).expect("a seed record is admitted in suite/45");
    for to in ["ess-conformance/41", "ess-conformance/35"] {
        let (suite, report) = relabelled(SUITE_43_SEEDS, REPORT_43_SEEDS, "ess-conformance/43", to);
        let error = admit_raw(&report, &suite).expect_err(to);
        let located = refusal(&error, "UnsupportedVocabulary");
        assert!(
            located.starts_with("$suite.provenance.synthesis_seeds")
                && located.contains("suite/42 or /43"),
            "{to}: {located}"
        );
    }
    let (suite, report) = with_provenance(SUITE_43_SEEDS, REPORT_43_SEEDS, |p| {
        p.as_object_mut().unwrap().remove("synthesis_seeds");
    });
    let error = admit_raw(&report, &suite).expect_err("no seed record in /43");
    let located = refusal(&error, "MissingField");
    assert!(
        located.starts_with("$suite.provenance.synthesis_seeds"),
        "{located}"
    );
}

/// A change to a suite's provenance object.
type ProvenanceChange = fn(&mut Value);

#[test]
fn a_seed_record_outside_its_closed_structure_is_refused() {
    // ESS 0.55.0 `synthesis_seeds::admit_json`: closed members, unsigned step indices; AEP also
    // reads each application's scenario as a scenario id.
    let cases: [(&str, ProvenanceChange); 4] = [
        ("UnknownField", |p| {
            p["synthesis_seeds"]["extra"] = 1.into();
        }),
        ("MissingField", |p| {
            p["synthesis_seeds"]["selections"][0]
                .as_object_mut()
                .unwrap()
                .remove("state");
        }),
        ("UnsignedIntegerRequired", |p| {
            p["synthesis_seeds"]["applications"][0]["establish_step"] = (-1).into();
        }),
        ("MalformedScenarioId", |p| {
            p["synthesis_seeds"]["applications"][0]["scenario"] = "not a scenario".into();
        }),
    ];
    for (reason, change) in cases {
        let (suite, report) = with_provenance(SUITE_43_SEEDS, REPORT_43_SEEDS, change);
        let error = admit_raw(&report, &suite).expect_err(reason);
        let located = refusal(&error, reason);
        assert!(
            located.starts_with("$suite.provenance.synthesis_seeds"),
            "{reason}: {located}"
        );
    }
}

#[test]
fn a_suite_ess_0_55_wrote_relabelled_below_a_form_it_carries_is_refused_naming_the_form() {
    for (suite, report, id, form) in [
        (
            SUITE_37_REFUSAL_POLICY,
            REPORT_37_REFUSAL_POLICY,
            "notify-ledger/binding/refusal/at-limit",
            "binding/refusal/<outcome>",
        ),
        (
            SUITE_37_BINDING_CONDITION,
            REPORT_37_BINDING_CONDITION,
            "received/binding/condition-absent",
            "binding/condition-absent",
        ),
    ] {
        let (suite, report) = relabelled(suite, report, "ess-conformance/37", "ess-conformance/35");
        let error = admit_raw(&report, &suite).expect_err(id);
        let located = refusal(&error, "UnsupportedVocabulary");
        let expected = format!("{id} uses the `{form}` form, which requires suite/36 or /37");
        assert!(located.contains(&expected), "{located}");
    }
}

/// A change to the seed record of `suite-43-seeds.json`, the reason AEP refuses it for, and the
/// refusal `ess 0.55.0 verify conform report --suite` gave the same change.
type SeedRefusal = (fn(&mut Value), &'static str, &'static str);

#[test]
fn a_seed_record_ess_0_55_refuses_without_reading_a_scenario_body_is_refused() {
    // ESS `synthesis_seeds::admit_selections` and `admit_applications`, the `SeedRecord` and
    // `SeedApplication` field types, and `admission::setup_row`. The detail AEP gives is ESS's.
    let cases: [SeedRefusal; 13] = [
        (
            |s| s["selections"] = json!([]),
            "InvalidShape",
            "InvalidSynthesisSeeds: sources and selections must be nonempty",
        ),
        (
            |s| s["selections"] = json!(vec![s["selections"][0].clone(); 65]),
            "InvalidShape",
            "InvalidSynthesisSeeds: more than 64 selections",
        ),
        (
            |s| s["sources"]["extra.yaml"] = s["sources"]["max.yaml"].clone(),
            "InvalidShape",
            "InvalidSynthesisSeeds: a source is selected by no selection",
        ),
        (
            |s| s["selections"][1]["identity"] = s["selections"][0]["identity"].clone(),
            "InvalidShape",
            "InvalidSynthesisSeeds: two selections share one qualified identity",
        ),
        (
            |s| s["selections"][0]["identity"] = Value::Null,
            "InvalidShape",
            "InvalidEntitySetup: identity cannot be null",
        ),
        (
            |s| s["selections"][0]["fields"] = json!({"bad-name": 1}),
            "MalformedName",
            "InvalidEntitySetup: field names must be local identifiers",
        ),
        (
            |s| {
                let mut nested = json!(0);
                for _ in 0..121 {
                    nested = json!([nested]);
                }
                s["selections"][0]["fields"]["revision"] = nested;
            },
            "InvalidDocument",
            "InvalidEntitySetup: setup literal nesting exceeds 120",
        ),
        (
            |s| s["selections"][0]["entity"] = "counter..Counter".into(),
            "MalformedName",
            "InvalidSuite: invalid qualified name identifier \"counter..Counter\"",
        ),
        (
            |s| s["selections"][0]["state"] = "active".into(),
            "MalformedName",
            "InvalidSuite: invalid state name identifier \"active\"",
        ),
        (
            |s| {
                let digest = s["sources"]["below.yaml"].clone();
                let sources = s["sources"].as_object_mut().unwrap();
                sources.remove("below.yaml");
                sources.insert("../below.yaml".into(), digest);
            },
            "MalformedSourceIdentity",
            "InvalidCoverage: authored source identity must be a checked root-relative path",
        ),
        (
            |s| {
                let mut extra = s["applications"][1].clone();
                extra["scenario"] = "counter.model.Create/outcome/nowhere".into();
                s["applications"].as_array_mut().unwrap().push(extra);
            },
            "InvalidShape",
            "InvalidSynthesisSeeds: an application names a scenario the suite does not hold",
        ),
        (
            |s| s["applications"][1] = s["applications"][0].clone(),
            "InvalidShape",
            "InvalidSynthesisSeeds: applications must be sorted and distinct",
        ),
        (
            |s| s["applications"][0]["instance"] = "Below Max".into(),
            "MalformedName",
            "InvalidSuite: invalid instance name identifier \"Below Max\"",
        ),
    ];
    for (change, reason, ess_refusal) in cases {
        let (suite, report) = with_provenance(SUITE_43_SEEDS, REPORT_43_SEEDS, |provenance| {
            change(&mut provenance["synthesis_seeds"]);
        });
        let error = admit_raw(&report, &suite).expect_err(ess_refusal);
        let issue = &error.issues[0];
        assert_eq!(issue.reason, reason, "{ess_refusal}: {error}");
        assert!(
            issue.path.starts_with("$suite.provenance.synthesis_seeds"),
            "{ess_refusal}: {error}"
        );
        // ESS's message after its own code prefix.
        let (_, message) = ess_refusal.split_once(": ").unwrap();
        assert!(issue.detail.contains(message), "{ess_refusal}: {error}");
    }
}

#[test]
fn a_seeded_selection_in_suite_45_whose_application_its_filter_moved_outside_is_refused() {
    // ESS admits a seed record without its inventory first (`admission.rs` `construct_formats`),
    // and only suite/43 (`synthesis_seeds::COVERAGE`) excuses an application whose scenario the
    // suite does not hold there; `ess 0.55.0 verify conform select` refuses this /45 child with
    // "an application names a scenario the suite does not hold".
    let mut input: Value = serde_json::from_str(INPUT_43_SELECTED).unwrap();
    let to45 = |text: &str| {
        let marker = |version: &str| format!("\"suite_version\": \"{version}\"");
        assert!(text.contains(&marker("ess-conformance/43")));
        text.replace(&marker("ess-conformance/43"), &marker("ess-conformance/45"))
    };
    let parent = to45(input["parent_suites"][0].as_str().unwrap());
    let mut child: Value = serde_json::from_str(input["suite_json"].as_str().unwrap()).unwrap();
    child["provenance"]["suite_version"] = "ess-conformance/45".into();
    child["coverage"]["selection"]["filter"]["parent"] = json!({"version": "ess-conformance/45", "digest_profile": "sha256-json-bytes/1", "digest": digest(&parent)});
    let selection = child["coverage"]["selection"].clone();
    let child = serde_json::to_string_pretty(&child).unwrap();
    input["parent_suites"][0] = parent.into();
    input["suite_json"] = child.clone().into();
    let mut report: Value = serde_json::from_str(REPORT_43_SELECTED).unwrap();
    // The report copies the suite's coverage summary, its parent reference included.
    report["coverage"]["selection"] = selection;
    report["suite"] = json!({"version": "ess-conformance/45", "digest_profile": "sha256-json-bytes/1", "digest": digest(&child)});
    let error = admit_input(&report.to_string(), &input.to_string()).expect_err("a /45 child");
    let located = refusal(&error, "InvalidShape");
    assert!(
        located.starts_with("$suite.provenance.synthesis_seeds")
            && located.contains("an application names a scenario the suite does not hold"),
        "{located}"
    );
    // The same selection in suite/43 is admitted (`an_explicit_seeded_selection_...`).
}

/// A one-scenario suite in `version` whose only key is `id`, with the report passing it.
fn single_scenario(id: &str, version: &str) -> (String, String) {
    let model = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
    let mut provenance = json!({"suite_version": version, "system": "demo", "specification_version": "v1", "spec_digest": model, "contract_digest": model});
    let major: u32 = version["ess-conformance/".len()..].parse().unwrap();
    if major >= 34 {
        provenance["scenario_initial_state"] = "empty".into();
    }
    let selection =
        json!({"scope": {"kind": "system"}, "origins": "generated", "filter": {"kind": "all"}});
    let counts = json!({"generated": 1, "authored": 0, "outside": 0, "refused": 0});
    let suite = json!({
        "provenance": provenance,
        "scenarios": {id: {"purpose": "Observe one later scenario form", "steps": [], "source": []}},
        "coverage": {"selection": selection, "knowledge": "complete_inventory", "generated": [id], "authored": [], "outside": [], "refused": [], "authored_sources": {}, "counts": counts},
    })
    .to_string();
    let report = json!({
        "format": "ess-conformance-report/2", "specification": "demo/v1", "spec_digest": model, "implementation": "example-service",
        "producer_profile": "rust-scenario-status/1",
        "suite": {"version": version, "digest_profile": "sha256-json-bytes/1", "digest": digest(&suite)},
        "counts": {"total": 1, "passed": 1, "failed": 0, "error": 0, "unsupported": 0, "skipped": 0},
        "outcomes": {"passed": [id], "failed": [], "error": [], "unsupported": [], "skipped": []},
        "execution_status": "passed", "conformance_status": "passed", "policy": "complete-selection/1", "completed_at": 1_790_000_000_000_u64,
        "coverage": {"knowledge": "complete_inventory", "selection": selection, "counts": counts, "refused": []},
    })
    .to_string();
    (suite, report)
}

#[test]
fn each_later_form_is_admitted_from_its_first_coverage_major_and_refused_below_it_and_in_suite_5() {
    // One id of each form ESS added after suite/5, with the first ordinary major ESS 0.55.0
    // carries it in (`LATER_ID_FORMS`); each goes through the reader the evidence command uses.
    for (id, ordinary) in [
        ("metrics.session.ByAgent/aggregate", 16),
        ("notify-ledger/binding/final-failure", 26),
        ("billing.invoice.IssueInvoice/grant/denied", 26),
        ("desk.ops.Tally/grant/admitted/desk.ops.Watcher", 26),
        ("desk.tickets.Board/grant/read/denied", 34),
        ("desk.tickets.Board/grant/read/admitted/desk.tickets.Clerk", 34),
        ("auth.keys.Issue/disclosure/issued/secret/origin/as/anonymous", 34),
        ("auth.keys.Issue/disclosure/issued/secret/denied/auth.keys.Revoke/as/actor/auth.keys.Other", 34),
        ("notify-ledger/binding/refusal/at-limit", 36),
        ("notify-ledger/binding/condition-false", 36),
        ("notify-ledger/binding/condition-absent", 36),
    ] {
        for major in [ordinary + 1, ordinary + 3] {
            let version = format!("ess-conformance/{major}");
            let (suite, report) = single_scenario(id, &version);
            let reading = admit_raw(&report, &suite)
                .unwrap_or_else(|error| panic!("{id} at {version}: {error}"));
            assert_eq!(reading.selected_ids()[0].as_str(), id);
        }
        for major in [ordinary - 1, 5] {
            let version = format!("ess-conformance/{major}");
            let (suite, report) = single_scenario(id, &version);
            let error = admit_raw(&report, &suite).expect_err(id);
            let issue = &error.issues[0];
            assert_eq!(issue.reason, "UnsupportedVocabulary", "{id} at {version}: {error}");
            assert_eq!(issue.path, "$suite.scenarios");
            let requires = format!("requires suite/{ordinary} or /{}", ordinary + 1);
            assert!(issue.detail.contains(&requires), "{id} at {version}: {error}");
        }
    }
}
