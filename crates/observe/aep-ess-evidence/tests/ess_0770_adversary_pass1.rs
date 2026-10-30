//! Adversary pass 1 on story:evidence-admits-ess-0770-suites.
//!
//! The unit admits a seed record at /58 and /59 only over the /42 and /43 seed suites relabelled.
//! `fixtures/ess-0770-adversary/` holds the seed suites `ess 0.77.0` writes itself at /58 and /59
//! (its README names the commands), read here through the same readers
//! `aep plan artifact evidence --from <report> --suite <suite>` records through. The last case
//! asserts the story's acceptance 6 against the repository's `CHANGELOG.md`.
use aep_domain::ess_conformance_coverage::EssConformanceCoverageReading;
use aep_domain::ess_conformance_v2::{
    CountStatus, EssAdmissionError, EssConformanceV2Reading, ScenarioId,
};
use aep_domain::Evidence;
use aep_ess_evidence::{
    adapt_json_coverage, adapt_json_v2, suite_route, wrap_coverage_suite, SuiteRoute,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::Path;

const SUITE_58_SEEDS: &str = include_str!("fixtures/ess-0770-adversary/suite-58-seeds.json");
const REPORT_58_SEEDS: &str = include_str!("fixtures/ess-0770-adversary/report-58-seeds.json");
const SUITE_59_SEEDS: &str = include_str!("fixtures/ess-0770-adversary/suite-59-seeds.json");
const REPORT_59_SEEDS: &str = include_str!("fixtures/ess-0770-adversary/report-59-seeds.json");

/// The scenario whose complete subject carries the `Decimal` that raises the seed suite to /58.
const DECIMAL_SUBJECT: &str = "counter.model.Counter/state/Closed/refuses/counter.model.Close";

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

/// The scenarios the suite's seed record says its rows were applied in, as ESS wrote them.
fn seeded_scenarios(suite: &str) -> Vec<String> {
    let suite: Value = serde_json::from_str(suite).unwrap();
    let applications = suite["provenance"]["synthesis_seeds"]["applications"]
        .as_array()
        .expect("ess 0.77.0 wrote a seed record");
    applications
        .iter()
        .map(|application| application["scenario"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_seed_suite_58_ess_0_77_0_wrote_itself_is_admitted_with_its_report() {
    assert_eq!(suite_route(SUITE_58_SEEDS).unwrap(), SuiteRoute::Count);
    let reading = ordinary(REPORT_58_SEEDS, SUITE_58_SEEDS)
        .unwrap_or_else(|error| panic!("a seed suite/58 ess 0.77.0 wrote must admit: {error}"));
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/58");
    assert_eq!(data.suite.digest(), digest(SUITE_58_SEEDS));
    assert_eq!((data.counts.total, data.counts.passed), (13, 13));
    assert_eq!(data.execution_status, CountStatus::Passed);
    let selected: Vec<&str> = reading
        .selected_ids()
        .iter()
        .map(ScenarioId::as_str)
        .collect();
    assert!(selected.contains(&DECIMAL_SUBJECT), "{selected:?}");
    let seeded = seeded_scenarios(SUITE_58_SEEDS);
    assert_eq!(seeded.len(), 2);
    for scenario in &seeded {
        assert!(selected.contains(&scenario.as_str()), "{scenario}");
    }
}

#[test]
fn a_seed_suite_59_ess_0_77_0_wrote_itself_is_admitted_with_its_report() {
    assert_eq!(suite_route(SUITE_59_SEEDS).unwrap(), SuiteRoute::Coverage);
    let reading = coverage(REPORT_59_SEEDS, SUITE_59_SEEDS)
        .unwrap_or_else(|error| panic!("a seed suite/59 ess 0.77.0 wrote must admit: {error}"));
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/59");
    assert_eq!(data.suite.digest(), digest(SUITE_59_SEEDS));
    assert_eq!((data.counts.total, data.counts.passed), (13, 13));
    let selected: Vec<&str> = reading
        .selected_ids()
        .iter()
        .map(ScenarioId::as_str)
        .collect();
    assert!(selected.contains(&DECIMAL_SUBJECT), "{selected:?}");
    for scenario in seeded_scenarios(SUITE_59_SEEDS) {
        assert!(selected.contains(&scenario.as_str()), "{scenario}");
    }
}

/// Acceptance 6 of story:evidence-admits-ess-0770-suites: "`CHANGELOG.md` gains an Unreleased
/// entry." The entry is the one place a consumer reads that /58 and /59 are now recorded and that
/// the pin moved; it names both majors once released as well, so this holds past the release.
#[test]
fn the_changelog_records_the_suite_majors_ess_0_77_0_adds_and_the_pin() {
    let changelog = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../CHANGELOG.md");
    let text = std::fs::read_to_string(&changelog).unwrap();
    // AEP's own release 0.77.0 has a heading; only a line naming ESS speaks of the ESS release.
    let names_ess_0_77_0 = text
        .lines()
        .any(|line| line.contains("ESS") && line.contains("0.77.0"));
    let names_majors = text
        .lines()
        .any(|line| line.contains("ess-conformance/58") && line.contains("/59"));
    assert!(
        names_ess_0_77_0 && names_majors,
        "CHANGELOG.md names neither ESS 0.77.0 ({names_ess_0_77_0}) nor the majors \
         `ess-conformance/58` and `/59` ({names_majors}): acceptance 6 asks for an Unreleased \
         entry for the suites ESS 0.77.0 writes and the pin"
    );
}
