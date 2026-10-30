//! The suite majors ESS 0.69.2 writes (story:evidence-admits-ess-0692-suites): the ordinary
//! majors /46–/56 and the coverage majors /47–/57, each read for what AEP reads of it and gated
//! by the major that added it. A major newer than the build lists is refused by name in
//! `ess_0770_suites.rs`, over the suites of the release the lists were last transcribed from.
//!
//! The pairs under `fixtures/ess-0692/` were written by `ess 0.69.2`; its README says how. An
//! ordinary suite goes through `adapt_json_v2`, a coverage suite through `wrap_coverage_suite` and
//! `adapt_json_coverage`: the readers `aep plan artifact evidence --from <report> --suite <suite>`
//! records through.
use aep_domain::ess_conformance_coverage::EssConformanceCoverageReading;
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError, EssConformanceV2Reading};
use aep_domain::Evidence;
use aep_ess_evidence::{adapt_json_coverage, adapt_json_v2, suite_route, wrap_coverage_suite};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_54: &str = include_str!("fixtures/ess-0692/suite-54.json");
const REPORT_54: &str = include_str!("fixtures/ess-0692/report-54.json");
const SUITE_55: &str = include_str!("fixtures/ess-0692/suite-55.json");
const REPORT_55: &str = include_str!("fixtures/ess-0692/report-55.json");
const INPUT_55_SELECTED: &str = include_str!("fixtures/ess-0692/input-55-selected.json");
const REPORT_55_SELECTED: &str = include_str!("fixtures/ess-0692/report-55-selected.json");
const SUITE_17_REFUSALS: &str =
    include_str!("fixtures/current-suites/suite-17-aggregate-refusals.json");
const REPORT_17_REFUSALS: &str =
    include_str!("fixtures/current-suites/report-17-aggregate-refusals-external.json");

const REFERENCE: &str = "ledger.incident.Windows/reference/incident_id";
const HELD: &str = "ledger.incident.OpenIncident/outcome/opened/held/title/30";
const WRAPPED: &str = "ledger.incident.IncidentId/invariant-wrapped/at/ledger.incident.Windows/incident_id/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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

fn coverage_input(
    report: &str,
    input: &str,
) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
    let adapted = adapt_json_coverage(report, input)?;
    let Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("the coverage reader returns coverage evidence");
    };
    Ok(sources.reading().expect("admitted").clone())
}

fn coverage(report: &str, suite: &str) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
    coverage_input(report, &wrap_coverage_suite(suite)?)
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

fn relabelled(suite: &str, report: &str, to: &str) -> (String, String) {
    rewritten(suite, report, |suite, _| {
        suite["provenance"]["suite_version"] = to.into();
    })
}

/// Adds `id` as one more passed scenario, its body a copy of the reference scenario's: AEP reads
/// a later major's scenario bodies not at all, so only the identity is under test.
fn with_passed_scenario(suite: &mut Value, report: &mut Value, id: &str) {
    let body = suite["scenarios"][REFERENCE].clone();
    suite["scenarios"][id] = body;
    let passed = report["outcomes"]["passed"].as_array_mut().unwrap();
    passed.push(id.into());
    passed.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    for count in ["total", "passed"] {
        report["counts"][count] = (report["counts"][count].as_u64().unwrap() + 1).into();
    }
}

#[test]
fn an_ordinary_suite_54_ess_0_69_2_wrote_is_admitted_with_its_report() {
    let reading = ordinary(REPORT_54, SUITE_54).expect("an ordinary suite/54 ESS wrote must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/54");
    assert_eq!(data.suite.digest(), digest(SUITE_54));
    assert_eq!((data.counts.total, data.counts.passed), (8, 8));
    assert_eq!(data.execution_status, CountStatus::Passed);
    assert!(reading
        .selected_ids()
        .iter()
        .any(|id| id.as_str() == REFERENCE));
}

#[test]
fn a_coverage_suite_55_ess_0_69_2_wrote_is_admitted_with_its_report() {
    let reading = coverage(REPORT_55, SUITE_55).expect("a coverage suite/55 ESS wrote must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/55");
    assert_eq!(data.suite.digest(), digest(SUITE_55));
    assert_eq!((data.counts.total, data.counts.passed), (8, 8));
    assert_eq!(data.conformance_status, CountStatus::Passed);
    assert!(reading
        .selected_ids()
        .iter()
        .any(|id| id.as_str() == REFERENCE));
}

#[test]
fn every_major_ess_0_69_2_writes_goes_to_its_reader_and_the_newest_pair_admits() {
    for (suite, report, to) in [
        (SUITE_54, REPORT_54, "ess-conformance/56"),
        (SUITE_55, REPORT_55, "ess-conformance/57"),
    ] {
        let (suite, report) = relabelled(suite, report, to);
        suite_route(&suite).unwrap_or_else(|error| panic!("{to}: {error}"));
        if to.ends_with('7') {
            coverage(&report, &suite).unwrap_or_else(|error| panic!("{to}: {error}"));
        } else {
            ordinary(&report, &suite).unwrap_or_else(|error| panic!("{to}: {error}"));
        }
    }
}

#[test]
fn a_suite_level_catalog_is_refused_below_suite_52_as_ess_refuses_it() {
    for (suite, report, to) in [
        (SUITE_54, REPORT_54, "ess-conformance/50"),
        (SUITE_55, REPORT_55, "ess-conformance/51"),
    ] {
        let (suite, report) = relabelled(suite, report, to);
        let error = if to.ends_with('1') {
            coverage(&report, &suite).expect_err(to)
        } else {
            ordinary(&report, &suite).expect_err(to)
        };
        let located = refusal(&error, "UnsupportedVocabulary");
        assert!(
            located.contains("events") && located.contains("suite/52"),
            "{to}: {located}"
        );
    }
}

#[test]
fn the_view_reference_form_is_refused_below_suite_54_as_ess_refuses_it() {
    for (suite, report, to) in [
        (SUITE_54, REPORT_54, "ess-conformance/52"),
        (SUITE_55, REPORT_55, "ess-conformance/53"),
    ] {
        let (suite, report) = relabelled(suite, report, to);
        let error = if to.ends_with('3') {
            coverage(&report, &suite).expect_err(to)
        } else {
            ordinary(&report, &suite).expect_err(to)
        };
        let located = refusal(&error, "UnsupportedVocabulary");
        assert!(
            located.contains(REFERENCE) && located.contains("suite/54"),
            "{to}: {located}"
        );
    }
}

#[test]
fn the_held_outcome_and_wrapped_invariant_forms_are_gated_at_their_own_majors() {
    // ESS 0.69.2 `hold_now::ORDINARY` (50) and `value_invariant::ORDINARY` (52). Without the
    // suite-level catalog, which itself needs /52, so only the form decides.
    for (id, first) in [(HELD, 50), (WRAPPED, 52)] {
        let at = |major: u32| {
            rewritten(SUITE_54, REPORT_54, |suite, report| {
                suite["provenance"]["suite_version"] = format!("ess-conformance/{major}").into();
                suite.as_object_mut().unwrap().remove("events");
                with_passed_scenario(suite, report, id);
            })
        };
        let (suite, report) = at(first - 2);
        let located = refusal(
            &ordinary(&report, &suite).expect_err(id),
            "UnsupportedVocabulary",
        );
        assert!(
            located.contains(id) && located.contains(&format!("suite/{first}")),
            "{located}"
        );
        // From its own major on, the form is admitted, here beside the view reference at /54.
        let (suite, report) = at(54);
        let reading = ordinary(&report, &suite).unwrap_or_else(|error| panic!("{id}: {error}"));
        assert!(reading
            .selected_ids()
            .iter()
            .any(|admitted| admitted.as_str() == id));
    }
}

#[test]
fn the_system_issued_refusal_codes_are_gated_at_suite_53_as_ess_gates_them() {
    // ESS 0.69.2 `system_issued::REFUSALS` (`ESS-SYNTH-023`, `-024`) need `system_issued::COVERAGE`
    // (53); both omit a candidate. An ESS-written refusal is respelled as each, in the suite and
    // in the report that carries the same inventory.
    for code in ["ESS-SYNTH-023", "ESS-SYNTH-024"] {
        let at = |major: u32| {
            rewritten(SUITE_17_REFUSALS, REPORT_17_REFUSALS, |suite, report| {
                suite["provenance"]["suite_version"] = format!("ess-conformance/{major}").into();
                suite["provenance"]["scenario_initial_state"] = "empty".into();
                for refused in [
                    &mut suite["coverage"]["refused"],
                    &mut report["coverage"]["refused"],
                ] {
                    for refusal in refused.as_array_mut().unwrap() {
                        if refusal["code"] == "ESS-SYNTH-017" {
                            refusal["code"] = code.into();
                            refusal["effect"] = "candidate_not_emitted".into();
                        }
                    }
                }
            })
        };
        let (suite, report) = at(51);
        let located = refusal(
            &coverage(&report, &suite).expect_err(code),
            "UnsupportedVocabulary",
        );
        assert!(
            located.contains(code) && located.contains("suite/53"),
            "{located}"
        );
        let (suite, report) = at(53);
        let reading = coverage(&report, &suite).unwrap_or_else(|error| panic!("{code}: {error}"));
        assert!(reading
            .data()
            .coverage
            .refused
            .iter()
            .any(|r| r.code == code));
    }
}

#[test]
fn a_selected_child_of_a_suite_with_a_suite_level_catalog_is_refused_by_name() {
    // ESS's selection drops the catalog entries the survivors no longer need and re-spells their
    // `absent_events`, so the survivors' bytes differ from the parent's while their expansion does
    // not. This build compares bytes and does not expand, so it names what it does not compare
    // rather than calling an unchanged survivor changed.
    let error = coverage_input(REPORT_55_SELECTED, INPUT_55_SELECTED).expect_err("catalog lineage");
    let located = refusal(&error, "UnsupportedVocabulary");
    assert!(located.contains("`events` catalog"), "{located}");
    assert!(
        error
            .issues
            .iter()
            .all(|issue| issue.reason != "ParentScenarioMismatch"),
        "{error}"
    );
}
