//! Adversary pass 1 against story:evidence-admits-ess-0692-suites. Each case names the ESS 0.69.2
//! source it is read from. Each pins behaviour the unit's own suite left unpinned, so a mutant of
//! the named line survived it. The CI checksum case of this pass is held in
//! `xtask/tests/ess_gate.rs`, beside the pin.
use aep_domain::ess_conformance_v2::{EssAdmissionError, ScenarioId};
use aep_ess_evidence::{adapt_json_coverage, wrap_coverage_suite};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const SUITE_55: &str = include_str!("fixtures/ess-0692/suite-55.json");
const REPORT_55: &str = include_str!("fixtures/ess-0692/report-55.json");
const SUITE_17_REFUSALS: &str =
    include_str!("fixtures/current-suites/suite-17-aggregate-refusals.json");
const REPORT_17_REFUSALS: &str =
    include_str!("fixtures/current-suites/report-17-aggregate-refusals-external.json");

fn digest(original: &str) -> String {
    Sha256::digest(original.as_bytes())
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

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

fn coverage(report: &str, suite: &str) -> Result<(), EssAdmissionError> {
    adapt_json_coverage(report, &wrap_coverage_suite(suite)?).map(|_| ())
}

/// `coverage_suite.rs` `admit_known_version` exists so a newer suite is refused by version before
/// its members are read. The unit's /59 case keeps only members this build allows (`events` is in
/// `ROOT_CATALOGS`), so the later version check refuses it anyway and deleting the guard leaves
/// every test green. A member only a later major could add tells them apart.
#[test]
fn a_newer_coverage_suite_with_a_member_this_build_does_not_know_is_refused_by_version() {
    for (place, change) in [
        (
            "root",
            (|suite: &mut Value| suite["later_catalog"] = json!({})) as fn(&mut Value),
        ),
        ("provenance", |suite: &mut Value| {
            suite["provenance"]["later_member"] = json!("x");
        }),
    ] {
        let (suite, _) = rewritten(SUITE_55, REPORT_55, |suite, _| {
            suite["provenance"]["suite_version"] = "ess-conformance/59".into();
            change(suite);
        });
        let error = wrap_coverage_suite(&suite).expect_err(place);
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.reason == "UnsupportedSuiteVersion"
                    && issue.path == "$suite.provenance.suite_version"),
            "{place}: {error}"
        );
        assert!(
            error
                .issues
                .iter()
                .all(|issue| issue.reason != "UnknownField"),
            "{place}: {error}"
        );
    }
}

/// ESS 0.69.2 `coverage.rs` `generated_refusal_effect` over `synthesize::CHECK_NOT_EMITTED`
/// (`synthesize.rs:1138`: 5, 11, 12, 14, 16, 17, 20 `ABSENCE_UNWITNESSED`, 21 `INSTANT_UNTAGGED`,
/// 22 `CHILD_UNWITNESSED`); 18 `COUNT_UNWITNESSED` and 19 `PRECEDENCE_CONTRADICTED` omit a
/// candidate. No test in the unit reads any of 018–022, so dropping `20 | 21 | 22` from
/// `values.rs` `Refusal::validate`, or adding 18 or 19, survives it. ESS gates none of them by
/// major (`Inventory::validate`), so the newest coverage major carries each.
#[test]
fn the_refusal_codes_ess_added_from_018_to_022_carry_the_effects_ess_assigns() {
    for (number, check) in [(18, false), (19, false), (20, true), (21, true), (22, true)] {
        let code = format!("ESS-SYNTH-{number:03}");
        let at = |effect: &str| {
            rewritten(SUITE_17_REFUSALS, REPORT_17_REFUSALS, |suite, report| {
                suite["provenance"]["suite_version"] = "ess-conformance/57".into();
                suite["provenance"]["scenario_initial_state"] = "empty".into();
                for refused in [
                    &mut suite["coverage"]["refused"],
                    &mut report["coverage"]["refused"],
                ] {
                    for refusal in refused.as_array_mut().unwrap() {
                        if refusal["code"] == "ESS-SYNTH-017" {
                            refusal["code"] = code.clone().into();
                            refusal["effect"] = effect.into();
                        }
                    }
                }
            })
        };
        let (right, wrong) = if check {
            ("check_not_emitted", "candidate_not_emitted")
        } else {
            ("candidate_not_emitted", "check_not_emitted")
        };
        let (suite, report) = at(right);
        coverage(&report, &suite).unwrap_or_else(|error| panic!("{code} {right}: {error}"));
        let (suite, report) = at(wrong);
        let error = coverage(&report, &suite).expect_err(&code);
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.reason == "RefusalEffectMismatch"),
            "{code} {wrong}: {error}"
        );
    }
}

/// ESS 0.69.2 `ScenarioId::parse` (`scenario.rs`), the three arms added after 0.55.0, predicate
/// by predicate. Nothing in the unit's suite refuses a malformed id of these forms, so dropping
/// any conjunct of the new arms in `ess_conformance_v2.rs` `ScenarioId::new` survives it.
#[test]
fn the_new_id_arms_admit_and_refuse_exactly_what_ess_parse_does() {
    let hex = "0123456789abcdef".repeat(4);
    let admitted = [
        // `seconds.parse::<i64>()`: a sign, zero and the bounds are what Rust reads.
        "a.B/outcome/opened/held/at/+5".to_owned(),
        "a.B/outcome/opened/held/at/-0".to_owned(),
        format!("a.B/outcome/opened/held/at/{}", i64::MIN),
        // The held field is only non-empty.
        "a.B/outcome/opened/held/At.x-1/30".to_owned(),
        format!("a.T/invariant-wrapped/at/a.V/any root/{hex}"),
        // `is_field_name`: underscores lead, then a letter.
        "a.V/reference/_url".to_owned(),
    ];
    for id in &admitted {
        ScenarioId::new(id.as_str()).unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let refused = [
        "a.B/outcome/opened/held/at/5.0".to_owned(),
        "a.B/outcome/opened/held/at/ 5".to_owned(),
        "a.B/outcome/opened/held/at/9223372036854775808".to_owned(),
        "a.B/outcome/opened/held/at/".to_owned(),
        "a.B/outcome/opened/held//5".to_owned(),
        "a.B/outcome/Opened/held/at/5".to_owned(),
        "a.B/outcome/open--ed/held/at/5".to_owned(),
        "1a/outcome/opened/held/at/5".to_owned(),
        format!("a.T/invariant-wrapped/at/a.V//{hex}"),
        format!("a.T/invariant-wrapped/at/a.V/root/{}", hex.to_uppercase()),
        format!("a.T/invariant-wrapped/at/a.V/root/{}", &hex[1..]),
        format!("a.T/invariant-wrapped/at/a.V/root/{hex}0"),
        format!("a.T/invariant-wrapped/at/a..V/root/{hex}"),
        format!("a.T./invariant-wrapped/at/a.V/root/{hex}"),
        "a.V/reference/_".to_owned(),
        "a.V/reference/1id".to_owned(),
        "a.V/reference/incident-id".to_owned(),
        "a.V/reference/".to_owned(),
        ".a/reference/id".to_owned(),
    ];
    for id in &refused {
        let error = ScenarioId::new(id.as_str()).expect_err(id);
        assert_eq!(error.issues[0].reason, "MalformedScenarioId", "{id}");
    }
}
