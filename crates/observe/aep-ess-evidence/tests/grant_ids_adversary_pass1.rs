//! Adversary pass 1 for `story:ess-grant-scenario-ids`.
//!
//! Two attacks. Suites ESS 0.55.0 wrote that the unit's own fixtures do not carry (disclosure
//! cells sent by actors, a component-scoped grant suite, authored grant refusals beside grant
//! scenarios, ESS's own interpreted run) must be admitted; every file under
//! `fixtures/adversary-88/` was written by `ess 0.55.0`, and `fixtures/adversary-88/README.md`
//! says how. Seed records ESS 0.55.0's reader refuses must not be admitted: each change below
//! was given to `ess 0.55.0 verify conform report --suite`, which refused it with the message
//! quoted beside it. And a later form under the transcribed suite/5 must be refused as the
//! vocabulary of a later major, as it is under every other major below its own.
#[path = "support/coverage.rs"]
#[allow(dead_code)]
mod fixtures;

use aep_domain::ess_conformance_coverage::EssConformanceCoverageReading;
use aep_domain::ess_conformance_v2::{CountStatus, EssAdmissionError};
use aep_domain::Evidence;
use aep_ess_evidence::{adapt_json_coverage, wrap_coverage_suite};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_ONE_TIME: &str = include_str!("fixtures/adversary-88/suite-35-one-time-actors.json");
const REPORT_ONE_TIME: &str =
    include_str!("fixtures/adversary-88/report-35-one-time-actors-external.json");
const RUN_ONE_TIME: &str = include_str!("fixtures/adversary-88/report-35-one-time-actors-run.json");
const INPUT_ONE_TIME_SELECTED: &str =
    include_str!("fixtures/adversary-88/input-35-one-time-actors-selected.json");
const REPORT_ONE_TIME_SELECTED: &str =
    include_str!("fixtures/adversary-88/report-35-one-time-actors-selected-external.json");
const SUITE_DESK_COMPONENT: &str =
    include_str!("fixtures/adversary-88/suite-35-desk-component.json");
const REPORT_DESK_COMPONENT: &str =
    include_str!("fixtures/adversary-88/report-35-desk-component-external.json");
const SUITE_DESK_AUTHORED: &str = include_str!("fixtures/adversary-88/suite-35-desk-authored.json");
const REPORT_DESK_AUTHORED: &str =
    include_str!("fixtures/adversary-88/report-35-desk-authored-external.json");
const RUN_DESK_AUTHORED: &str =
    include_str!("fixtures/adversary-88/report-35-desk-authored-run.json");
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

/// How many selected ids contain `infix`.
fn carrying(reading: &EssConformanceCoverageReading, infix: &str) -> usize {
    reading
        .selected_ids()
        .iter()
        .filter(|id| id.as_str().contains(infix))
        .count()
}

#[test]
fn disclosure_cells_sent_by_actors_ess_0_55_wrote_are_admitted_from_both_producers() {
    for (producer, report) in [("external", REPORT_ONE_TIME), ("ess run", RUN_ONE_TIME)] {
        let reading =
            admit_raw(report, SUITE_ONE_TIME).unwrap_or_else(|error| panic!("{producer}: {error}"));
        let data = reading.data();
        assert_eq!(data.suite.version(), "ess-conformance/35", "{producer}");
        assert_eq!(data.counts.total, 14, "{producer}");
        assert_eq!(data.counts.passed, 14, "{producer}");
        assert_eq!(carrying(&reading, "/disclosure/"), 12, "{producer}");
        // Every cell is sent by a declared actor, and every aspect ESS has is among them.
        assert_eq!(
            carrying(&reading, "/as/actor/credentials.api."),
            12,
            "{producer}"
        );
        for aspect in [
            "/secret/origin/",
            "/secret/retry/",
            "/secret/rotation/",
            "/secret/read/credentials.api.Records/",
            "/secret/command/credentials.api.Read/read/",
            "/secret/denied/credentials.api.Read/",
        ] {
            assert!(carrying(&reading, aspect) >= 1, "{producer}: no {aspect}");
        }
        assert_eq!(data.conformance_status, CountStatus::Passed, "{producer}");
    }
}

#[test]
fn an_explicit_selection_of_disclosure_cells_is_admitted_with_its_parent() {
    let reading = admit_input(REPORT_ONE_TIME_SELECTED, INPUT_ONE_TIME_SELECTED)
        .expect("an ESS-selected suite/35 child of disclosure cells and its parent must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/35");
    assert_eq!(reading.selected_ids().len(), 4);
    assert_eq!(carrying(&reading, "/disclosure/"), 3);
    // The other ten of the parent's fourteen are outside by the filter.
    assert_eq!(data.coverage.counts.outside, 10);
    assert_eq!(data.conformance_status, CountStatus::Passed);
}

#[test]
fn a_component_scoped_grant_suite_ess_0_55_wrote_is_admitted() {
    let reading = admit_raw(REPORT_DESK_COMPONENT, SUITE_DESK_COMPONENT)
        .expect("ESS's component-scoped suite/35 with grant scenarios must admit");
    let data = reading.data();
    assert_eq!(data.suite.version(), "ess-conformance/35");
    assert_eq!(reading.selected_ids().len(), 7);
    for (form, count) in [
        ("/grant/denied", 1),
        ("/grant/admitted/", 1),
        ("/grant/read/denied", 2),
        ("/grant/read/admitted/", 2),
    ] {
        assert_eq!(carrying(&reading, form), count, "{form}");
    }
    assert_eq!(data.conformance_status, CountStatus::Passed);
}

#[test]
fn authored_grant_refusals_ess_0_55_wrote_are_admitted_beside_grant_scenarios() {
    for (producer, report) in [
        ("external", REPORT_DESK_AUTHORED),
        ("ess run", RUN_DESK_AUTHORED),
    ] {
        let reading = admit_raw(report, SUITE_DESK_AUTHORED)
            .unwrap_or_else(|error| panic!("{producer}: {error}"));
        let data = reading.data();
        assert_eq!(data.suite.version(), "ess-conformance/35", "{producer}");
        assert_eq!(data.counts.passed, 8, "{producer}");
        assert_eq!(data.coverage.counts.authored, 1, "{producer}");
        let codes: Vec<&str> = data
            .coverage
            .refused
            .iter()
            .map(|refusal| refusal.code.as_str())
            .collect();
        assert_eq!(codes, ["ESS-AUTHOR-038", "ESS-AUTHOR-039"], "{producer}");
        assert_eq!(carrying(&reading, "/grant/"), 6, "{producer}");
        // An in-scope authored refusal leaves conformance undecided however many passed.
        assert_eq!(
            data.conformance_status,
            CountStatus::Inconclusive,
            "{producer}"
        );
    }
}

/// A change to a suite/43 seed record, and the refusal ESS 0.55.0 gave the changed suite.
type SeedChange = (&'static str, fn(&mut Value));

#[test]
fn a_seed_record_ess_0_55_refuses_is_not_admitted() {
    // Each refusal is ESS's, measured on the changed `suite-43-seeds.json`. None of them reads a
    // scenario body: they are checks on the record itself, on the scenario ids its applications
    // name, and on the typed names it carries (`synthesis_seeds::admit_selections`,
    // `admit_applications`, and the `SeedRecord` and `SeedApplication` field types).
    let changes: [SeedChange; 6] = [
        ("InvalidSynthesisSeeds: invalid source digest", |seeds| {
            seeds["sources"]["below.yaml"] = "sha256:not-a-digest".into();
        }),
        (
            "InvalidSynthesisSeeds: an application names an authored scenario",
            |seeds| {
                seeds["applications"][1]["scenario"] = "counter.model/authored/seeded".into();
            },
        ),
        (
            "InvalidSynthesisSeeds: an application names an unknown selection",
            |seeds| {
                seeds["applications"][1]["instance"] = "nowhere".into();
            },
        ),
        (
            "InvalidSynthesisSeeds: a selection names an unknown source",
            |seeds| {
                seeds["selections"][1]["source"] = "other.yaml".into();
            },
        ),
        (
            "InvalidSynthesisSeeds: selections must be sorted and distinct",
            |seeds| {
                seeds["selections"].as_array_mut().unwrap().reverse();
            },
        ),
        (
            "InvalidSuite: invalid instance name identifier \"Not Kebab\"",
            |seeds| {
                seeds["selections"][0]["instance"] = "Not Kebab".into();
            },
        ),
    ];
    let mut admitted = Vec::new();
    for (ess_refusal, change) in changes {
        let mut suite: Value = serde_json::from_str(SUITE_43_SEEDS).unwrap();
        change(&mut suite["provenance"]["synthesis_seeds"]);
        let suite = serde_json::to_string_pretty(&suite).unwrap();
        assert_ne!(suite, SUITE_43_SEEDS);
        let mut report: Value = serde_json::from_str(REPORT_43_SEEDS).unwrap();
        report["suite"]["digest"] = digest(&suite).into();
        match admit_raw(&report.to_string(), &suite) {
            Ok(_) => admitted.push(ess_refusal),
            Err(error) => assert!(
                error.issues[0].path.starts_with("$suite.provenance"),
                "{ess_refusal}: refused for another reason: {error}"
            ),
        }
    }
    assert!(
        admitted.is_empty(),
        "admitted seed records ESS 0.55.0 refuses: {admitted:#?}"
    );
}

#[test]
fn a_later_form_in_the_transcribed_suite_5_is_refused_naming_the_form_and_its_major() {
    // The story's acceptance: keys "under a major below the one that introduced them are refused
    // as `UnsupportedVocabulary` naming the form and the major". Suite/5 is below every one of
    // them, and each id here is one ESS 0.55.0 parses (`scenario_id_adversary_pass1.rs`).
    let mut wrong = Vec::new();
    for (id, form, ordinary) in [
        ("demo.core.Reads/aggregate", "aggregate", 16),
        (
            "notify-ledger/binding/final-failure",
            "binding/final-failure",
            26,
        ),
        ("demo.core.Read/grant/denied", "grant/denied", 26),
        (
            "demo.core.Read/grant/admitted/demo.core.Clerk",
            "grant/admitted/<actor>",
            26,
        ),
        ("demo.core.Reads/grant/read/denied", "grant/read/denied", 34),
        (
            "demo.core.Read/disclosure/ready/secret/origin/as/anonymous",
            "disclosure/...",
            34,
        ),
        (
            "notify-ledger/binding/refusal/rejected",
            "binding/refusal/<outcome>",
            36,
        ),
    ] {
        let mut suite = fixtures::suite();
        let body = suite["scenarios"][fixtures::SELECTED].clone();
        suite["scenarios"] = serde_json::json!({ id: body });
        suite["coverage"]["generated"] = serde_json::json!([id]);
        let (report, input) = fixtures::pair_for(&suite, &[id], "passed", &[]);
        let error = admit_input(&report, &input).expect_err(id);
        let issue = &error.issues[0];
        let expected = format!(
            "{id} uses the `{form}` form, which requires suite/{ordinary} or /{}",
            ordinary + 1
        );
        if issue.reason != "UnsupportedVocabulary" || !issue.detail.contains(&expected) {
            wrong.push(format!(
                "{id}: {} at {}: {}",
                issue.reason, issue.path, issue.detail
            ));
        }
    }
    assert!(wrong.is_empty(), "suite/5 refusals: {wrong:#?}");
}
