//! Reports ESS's generated Go and TypeScript runners write: `producer_profile:
//! go-scenario-status/2`, all five categories, with execution failed when any scenario failed or
//! was unsupported, otherwise inconclusive when any errored or was skipped, otherwise passed.
//!
//! The pairs under `fixtures/generated-runner/` were written by `ess 0.57.0` and its generated Go
//! package; their README says how. Both readers `aep plan artifact evidence --from <report>
//! --suite <suite>` routes to are held to the same rule: the count-stage reader (`adapt_json_v2`,
//! an ordinary suite/34) and the coverage reader (`adapt_json_coverage`, a coverage suite/35).
//! The outcomes are `aep.evidence.RecordFromReport` in `ess/domains/evidence.yaml`.
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError};
use aep_domain::{Evidence, FactValue};
use aep_ess_evidence::{adapt_json_coverage, adapt_json_v2, wrap_coverage_suite};
use serde_json::{json, Value};

const SUITE_34: &str = include_str!("fixtures/generated-runner/suite-34-aep.json");
const REPORT_34: &str = include_str!("fixtures/generated-runner/report-34-aep-go.json");
const SUITE_35: &str = include_str!("fixtures/generated-runner/suite-35-aep.json");
const REPORT_35: &str = include_str!("fixtures/generated-runner/report-35-aep-go.json");

const PROFILE: &str = "go-scenario-status/2";
const CATEGORIES: [&str; 5] = ["passed", "failed", "error", "unsupported", "skipped"];

/// What one reader admitted: its execution status, its counts by category, and the producer
/// profile its facts record.
#[derive(Debug)]
struct Admitted {
    execution: CountStatus,
    counts: Vec<(&'static str, u64)>,
    profile: String,
}

fn recorded_profile(facts: Vec<(aep_domain::FactPath, FactValue)>) -> String {
    let (_, value) = facts
        .into_iter()
        .find(|(path, _)| path.to_string().ends_with(".producer_profile"))
        .expect("an admitted reading states its producer profile");
    value.as_text().unwrap().to_owned()
}

fn count_stage(report: &str) -> Result<Admitted, EssAdmissionError> {
    let adapted = adapt_json_v2(report, SUITE_34)?;
    let Evidence::EssConformanceV2(sources) = adapted.evidence() else {
        panic!("the count reader returns count-stage evidence");
    };
    let reading = sources.reading().expect("admitted");
    Ok(Admitted {
        execution: reading.data().execution_status,
        counts: reading.data().counts.categories().into_iter().collect(),
        profile: recorded_profile(reading.facts()),
    })
}

fn coverage(report: &str) -> Result<Admitted, EssAdmissionError> {
    let input = wrap_coverage_suite(SUITE_35).expect("the coverage suite ESS wrote wraps");
    let adapted = adapt_json_coverage(report, &input)?;
    let Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("the coverage reader returns coverage evidence");
    };
    let reading = sources.reading().expect("admitted");
    Ok(Admitted {
        execution: reading.data().execution_status,
        counts: reading.data().counts.categories().into_iter().collect(),
        profile: recorded_profile(reading.facts()),
    })
}

/// One reader, from original report bytes to what it admitted.
type Read = fn(&str) -> Result<Admitted, EssAdmissionError>;

/// Every reader, with the report ESS wrote for its suite.
fn readers() -> [(&'static str, &'static str, Read); 2] {
    [
        ("count-stage", REPORT_34, count_stage),
        ("coverage", REPORT_35, coverage),
    ]
}

fn edited(report: &str, edit: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(report).unwrap();
    edit(&mut value);
    value.to_string()
}

/// The report with its scenarios redistributed: the sorted ids are dealt out in category order,
/// `sizes[i]` to `CATEGORIES[i]`, the passed bucket taking whatever is left. The suite is
/// untouched, so its digest still matches.
fn distributed(report: &str, sizes: [usize; 4], execution: &str) -> String {
    edited(report, |r| {
        let mut ids: Vec<String> = CATEGORIES
            .iter()
            .flat_map(|category| r["outcomes"][*category].as_array().unwrap().clone())
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        ids.sort();
        let total = ids.len();
        let passed = total - sizes.iter().sum::<usize>();
        let mut rest = ids.as_slice();
        for (category, size) in CATEGORIES
            .iter()
            .zip([passed, sizes[0], sizes[1], sizes[2], sizes[3]])
        {
            let (taken, left) = rest.split_at(size);
            r["outcomes"][*category] = json!(taken);
            r["counts"][*category] = size.into();
            rest = left;
        }
        r["counts"]["total"] = total.into();
        r["execution_status"] = execution.into();
        // Coverage is unknown under /34 and carries in-scope refusals under /35, so nothing but
        // a failed run is anything other than inconclusive.
        r["conformance_status"] = if execution == "failed" {
            "failed"
        } else {
            "inconclusive"
        }
        .into();
    })
}

fn refusal_reasons(error: &EssAdmissionError) -> Vec<&'static str> {
    error.issues.iter().map(|issue| issue.reason).collect()
}

/// Both readers admit the report with the stated execution status, and refuse it as
/// `ExecutionStatusMismatch` once it states any other.
fn execution_is(sizes: [usize; 4], expected: CountStatus) {
    for (reader, report, read) in readers() {
        let admitted = read(&distributed(report, sizes, expected.as_str()))
            .unwrap_or_else(|error| panic!("{reader} reader refused {sizes:?}: {error}"));
        assert_eq!(admitted.execution, expected, "{reader} {sizes:?}");
        let [failed, error, unsupported, skipped] = sizes.map(|size| size as u64);
        assert_eq!(
            admitted.counts,
            vec![
                ("passed", admitted.counts[0].1),
                ("failed", failed),
                ("error", error),
                ("unsupported", unsupported),
                ("skipped", skipped)
            ],
            "{reader} keeps every category"
        );
        for other in [
            CountStatus::Passed,
            CountStatus::Failed,
            CountStatus::Inconclusive,
        ] {
            if other == expected {
                continue;
            }
            let error = read(&distributed(report, sizes, other.as_str()))
                .expect_err("a contradicted execution status must refuse");
            assert!(
                refusal_reasons(&error).contains(&"ExecutionStatusMismatch"),
                "{reader} {sizes:?} stated {}: {error}",
                other.as_str()
            );
        }
    }
}

#[test]
fn a_generated_runner_report_is_admitted_by_the_count_stage_reader() {
    let admitted = count_stage(REPORT_34)
        .unwrap_or_else(|error| panic!("the report ESS's Go runner wrote must admit: {error}"));
    assert_eq!(admitted.profile, PROFILE);
    assert_eq!(admitted.execution, CountStatus::Failed);
    assert_eq!(
        admitted.counts,
        vec![
            ("passed", 0),
            ("failed", 0),
            ("error", 8),
            ("unsupported", 11),
            ("skipped", 0)
        ]
    );
}

#[test]
fn a_generated_runner_report_is_admitted_by_the_coverage_reader() {
    let admitted = coverage(REPORT_35)
        .unwrap_or_else(|error| panic!("the report ESS's Go runner wrote must admit: {error}"));
    assert_eq!(admitted.profile, PROFILE);
    assert_eq!(admitted.execution, CountStatus::Failed);
    assert_eq!(
        admitted.counts,
        vec![
            ("passed", 0),
            ("failed", 0),
            ("error", 8),
            ("unsupported", 11),
            ("skipped", 0)
        ]
    );
}

// The five branches of the rule. Each case carries every other category it can without changing
// the branch, so a reader that lets a later category decide is caught.

#[test]
fn under_the_generated_runner_profile_a_failed_scenario_fails_execution() {
    execution_is([1, 1, 0, 1], CountStatus::Failed);
}

#[test]
fn under_the_generated_runner_profile_an_unsupported_scenario_fails_execution() {
    execution_is([0, 1, 1, 1], CountStatus::Failed);
}

#[test]
fn under_the_generated_runner_profile_an_errored_scenario_makes_execution_inconclusive() {
    execution_is([0, 1, 0, 0], CountStatus::Inconclusive);
}

#[test]
fn under_the_generated_runner_profile_a_skipped_scenario_makes_execution_inconclusive() {
    execution_is([0, 0, 0, 1], CountStatus::Inconclusive);
}

#[test]
fn under_the_generated_runner_profile_every_scenario_passing_is_passed_execution() {
    execution_is([0, 0, 0, 0], CountStatus::Passed);
}

#[test]
fn the_first_go_profile_still_refuses_error_and_unsupported_counts() {
    for (reader, report, read) in readers() {
        for sizes in [[0, 1, 0, 0], [0, 0, 1, 0]] {
            let first = edited(&distributed(report, sizes, "inconclusive"), |r| {
                r["producer_profile"] = "go-scenario-status/1".into();
            });
            let first = if sizes[2] == 1 {
                edited(&first, |r| {
                    r["execution_status"] = "failed".into();
                    r["conformance_status"] = "failed".into();
                })
            } else {
                first
            };
            let error = read(&first).expect_err("go-scenario-status/1 cannot carry these");
            assert!(
                refusal_reasons(&error).contains(&"ProfileOutcomeMismatch"),
                "{reader} {sizes:?}: {error}"
            );
        }
        // Its own categories are still read: one skipped scenario is inconclusive.
        let skipped = edited(&distributed(report, [0, 0, 0, 1], "inconclusive"), |r| {
            r["producer_profile"] = "go-scenario-status/1".into();
        });
        let admitted = read(&skipped).unwrap_or_else(|error| panic!("{reader}: {error}"));
        assert_eq!(admitted.profile, "go-scenario-status/1");
        assert_eq!(admitted.execution, CountStatus::Inconclusive);
    }
}

#[test]
fn unknown_producer_profiles_stay_refused() {
    for (reader, report, read) in readers() {
        for profile in [
            "go-scenario-status/3",
            "go-scenario-status/0",
            "go-scenario-status",
            "go-scenario-status/2;runner=go@1.22",
            "Go-Scenario-Status/2",
            "typescript-scenario-status/1",
        ] {
            let unknown = edited(report, |r| r["producer_profile"] = profile.into());
            let error = read(&unknown).expect_err("an unlisted profile must refuse");
            let issue = error
                .issues
                .iter()
                .find(|issue| issue.reason == "UnsupportedProducerProfile")
                .unwrap_or_else(|| panic!("{reader} {profile}: {error}"));
            assert_eq!(issue.path, "$.producer_profile", "{reader} {profile}");
        }
    }
}
