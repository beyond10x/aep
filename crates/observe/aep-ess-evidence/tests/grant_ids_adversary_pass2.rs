//! Adversary pass 2 for `story:ess-grant-scenario-ids`: the seed-record checks of round 3.
//!
//! A seeded suite ESS 0.55.0 wrote, and its own reader and interpreter admitted, must be admitted;
//! `fixtures/adversary-88/README.md` says how each file was written. A seed record ESS 0.55.0's
//! reader refuses must be refused; the change below was given to `ess 0.55.0 verify conform
//! report --suite`, which refused it with the message quoted beside it.
use aep_domain::ess_conformance_coverage::EssConformanceCoverageReading;
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError};
use aep_domain::Evidence;
use aep_ess_evidence::{adapt_json_coverage, wrap_coverage_suite};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_INTEGER_IDENTITIES: &str =
    include_str!("fixtures/adversary-88/integer-identities/suite-43-integer-identities.json");
const REPORT_INTEGER_IDENTITIES: &str = include_str!(
    "fixtures/adversary-88/integer-identities/report-43-integer-identities-external.json"
);
const RUN_INTEGER_IDENTITIES: &str =
    include_str!("fixtures/adversary-88/integer-identities/report-43-integer-identities-run.json");
const SUITE_43_SEEDS: &str = include_str!("fixtures/current-suites/suite-43-seeds.json");
const REPORT_43_SEEDS: &str = include_str!("fixtures/current-suites/report-43-seeds-external.json");

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
    let adapted = adapt_json_coverage(report, &input)?;
    let Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("coverage adapter returns coverage evidence");
    };
    Ok(sources.reading().expect("admitted").clone())
}

#[test]
fn a_seeded_suite_ess_0_55_wrote_with_distinct_integer_identities_above_2_53_is_admitted() {
    // ESS wrote the two rows' identities as `9007199254740992.0` and `9007199254740993`, which
    // name two rows: ESS's own reader and interpreter admitted the suite and passed all eight.
    assert!(SUITE_INTEGER_IDENTITIES.contains("\"identity\": 9007199254740992.0"));
    assert!(SUITE_INTEGER_IDENTITIES.contains("\"identity\": 9007199254740993"));
    for (producer, report) in [
        ("external", REPORT_INTEGER_IDENTITIES),
        ("ess run", RUN_INTEGER_IDENTITIES),
    ] {
        let reading = admit_raw(report, SUITE_INTEGER_IDENTITIES)
            .unwrap_or_else(|error| panic!("{producer}: {error}"));
        let data = reading.data();
        assert_eq!(data.suite.version(), "ess-conformance/43", "{producer}");
        assert_eq!(data.counts.passed, 8, "{producer}");
        assert_eq!(data.conformance_status, CountStatus::Passed, "{producer}");
    }
}

#[test]
fn seed_applications_out_of_scenario_order_are_refused_as_ess_0_55_refuses_them() {
    // ESS orders a `SeedApplication` by `(ScenarioId, establish_step)`, and `ScenarioId`'s `Ord`
    // is the rendered name (`scenario.rs`, "By the rendered name, not by the variant"). Reversed,
    // the fixture's two applications name `.../revision-exhausted` before `.../authorized`; ESS
    // refused that with "InvalidSynthesisSeeds: applications must be sorted and distinct".
    let mut suite: Value = serde_json::from_str(SUITE_43_SEEDS).unwrap();
    suite["provenance"]["synthesis_seeds"]["applications"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let suite = serde_json::to_string_pretty(&suite).unwrap();
    let mut report: Value = serde_json::from_str(REPORT_43_SEEDS).unwrap();
    report["suite"]["digest"] = digest(&suite).into();
    let Err(error) = admit_raw(&report.to_string(), &suite) else {
        panic!("applications out of ESS's order must be refused");
    };
    let issue = &error.issues[0];
    assert!(
        issue.path.starts_with("$suite.provenance.synthesis_seeds")
            && issue
                .detail
                .contains("applications must be sorted and distinct"),
        "{error}"
    );
}
