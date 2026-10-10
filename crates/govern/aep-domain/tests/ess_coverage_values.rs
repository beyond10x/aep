//! Exact coverage diagnostic projection and immutable admission transitions.
use aep_domain::ess_conformance_coverage::{
    CoverageCounts, CoverageSummary, EssConformanceCoverageReader, EssConformanceCoverageReading,
    EssConformanceCoverageSources, Filter, Knowledge, Origins, ReadingInput, Refusal, Scope,
    Selection, SuiteReference,
};
use aep_domain::ess_conformance_v2::{
    CountStatus, EssAdmissionError, ProducerProfile, ScenarioCounts, ScenarioId,
};
use aep_domain::time::Timestamp;
use aep_domain::{Evidence, FactValue, SpecDigest};
use std::collections::BTreeMap;

const SELECTED: &str = "demo.core.Read/outcome/ready";

struct Case {
    total: u64,
    knowledge: Knowledge,
    refused: Vec<Refusal>,
    component: bool,
    explicit: bool,
    conformance: CountStatus,
    complete: bool,
}
fn cases() -> Vec<Case> {
    let refusal: Refusal = serde_json::from_value(serde_json::json!({"origin":"generated","scenario":SELECTED,"subject":{"kind":"command","name":"demo.core.Read"},"source":null,"code":"ESS-SYNTH-005","message":"original omitted check","effect":"check_not_emitted","retained":{"origin":"generated","source":null},"scope":"in_scope","needs":[]})).unwrap();
    vec![
        Case {
            total: 1,
            knowledge: Knowledge::CompleteInventory,
            refused: vec![],
            component: false,
            explicit: false,
            conformance: CountStatus::Passed,
            complete: true,
        },
        Case {
            total: 0,
            knowledge: Knowledge::CompleteInventory,
            refused: vec![],
            component: false,
            explicit: false,
            conformance: CountStatus::Inconclusive,
            complete: true,
        },
        Case {
            total: 1,
            knowledge: Knowledge::Unknown,
            refused: vec![],
            component: false,
            explicit: false,
            conformance: CountStatus::Inconclusive,
            complete: false,
        },
        Case {
            total: 1,
            knowledge: Knowledge::CompleteInventory,
            refused: vec![refusal.clone(), refusal],
            component: false,
            explicit: false,
            conformance: CountStatus::Inconclusive,
            complete: false,
        },
        Case {
            total: 1,
            knowledge: Knowledge::CompleteInventory,
            refused: vec![],
            component: true,
            explicit: true,
            conformance: CountStatus::Passed,
            complete: true,
        },
    ]
}
fn reference(hex: char) -> SuiteReference {
    SuiteReference::new(
        "ess-conformance/5".into(),
        "sha256-json-bytes/1".into(),
        format!("sha256:{}", hex.to_string().repeat(64)),
    )
    .unwrap()
}
fn input(case: &Case) -> ReadingInput {
    let ids = if case.total == 0 {
        vec![]
    } else {
        vec![ScenarioId::new(SELECTED).unwrap()]
    };
    ReadingInput {
        specification: "independent model label".into(),
        implementation: "independent implementation".into(),
        spec_digest: SpecDigest::new("a".repeat(64)).unwrap(),
        suite: reference('b'),
        coverage: CoverageSummary {
            selection: Selection {
                scope: if case.component {
                    Scope::Component {
                        component: "worker".into(),
                    }
                } else {
                    Scope::System
                },
                origins: Origins::GeneratedAndAuthored,
                filter: if case.explicit {
                    Filter::Explicit {
                        ids: ids.clone(),
                        parent: reference('c'),
                    }
                } else {
                    Filter::All
                },
            },
            knowledge: case.knowledge,
            counts: CoverageCounts {
                generated: case.total,
                authored: 0,
                outside: 0,
                refused: case.refused.len() as u64,
            },
            refused: case.refused.clone(),
        },
        counts: ScenarioCounts {
            total: case.total,
            passed: case.total,
            failed: 0,
            error: 0,
            unsupported: 0,
            skipped: 0,
        },
        outcomes: [ids, vec![], vec![], vec![], vec![]],
        producer_profile: ProducerProfile::Rust,
        execution_status: CountStatus::Passed,
        conformance_status: case.conformance,
        completed_at: Timestamp::from_epoch_millis(u64::MAX),
    }
}
fn expected(case: &Case) -> BTreeMap<String, FactValue> {
    let total = case.total.to_string();
    let mut expected = BTreeMap::new();
    let mut text = |path: &str, value: String| {
        expected.insert(
            format!("ess_conformance_coverage_v1.{path}"),
            FactValue::text(value),
        );
    };
    for (path, value) in [
        ("counts.total", total.clone()),
        ("counts.passed", total.clone()),
        ("counts.failed", "0".into()),
        ("counts.error", "0".into()),
        ("counts.unsupported", "0".into()),
        ("counts.skipped", "0".into()),
        ("coverage.counts.generated", total),
        ("coverage.counts.authored", "0".into()),
        ("coverage.counts.outside", "0".into()),
        ("coverage.counts.refused", case.refused.len().to_string()),
        ("completed_at", "18446744073709551615".into()),
        ("execution_status", "passed".into()),
        ("conformance_status", case.conformance.as_str().into()),
        ("producer_profile", "rust-scenario-status/1".into()),
        ("policy", "complete-selection/1".into()),
        ("spec_digest", "a".repeat(64)),
        ("suite.version", "ess-conformance/5".into()),
        ("suite.digest_profile", "sha256-json-bytes/1".into()),
        ("suite.digest", format!("sha256:{}", "b".repeat(64))),
        ("coverage.knowledge", case.knowledge.as_str().into()),
        (
            "selection.scope.kind",
            if case.component {
                "component"
            } else {
                "system"
            }
            .into(),
        ),
        ("selection.origins", "generated_and_authored".into()),
        (
            "selection.filter.kind",
            if case.explicit { "explicit" } else { "all" }.into(),
        ),
    ] {
        text(path, value);
    }
    if case.component {
        text("selection.scope.component", "worker".into());
    }
    if case.explicit {
        text(
            "selection.filter.parent.version",
            "ess-conformance/5".into(),
        );
        text(
            "selection.filter.parent.digest_profile",
            "sha256-json-bytes/1".into(),
        );
        text(
            "selection.filter.parent.digest",
            format!("sha256:{}", "c".repeat(64)),
        );
    }
    for (path, value) in [
        ("nonempty", case.total != 0),
        ("failed_zero", true),
        (
            "coverage_known",
            case.knowledge == Knowledge::CompleteInventory,
        ),
        ("coverage_complete", case.complete),
    ] {
        expected.insert(
            format!("ess_conformance_coverage_v1.{path}"),
            FactValue::bool(value),
        );
    }
    expected
}

#[derive(Debug)]
struct Reader(ReadingInput);
impl EssConformanceCoverageReader for Reader {
    fn read(
        &self,
        _: &EssConformanceCoverageSources,
    ) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
        EssConformanceCoverageReading::new(self.0.clone())
    }
}
#[derive(Debug)]
struct Refusing;
impl EssConformanceCoverageReader for Refusing {
    fn read(
        &self,
        _: &EssConformanceCoverageSources,
    ) -> Result<EssConformanceCoverageReading, EssAdmissionError> {
        Err(EssAdmissionError::new(
            "ReaderRefusal",
            "$",
            "independent deliberate reader failure",
        ))
    }
}

#[test]
fn coverage_exact_fact_table_has_only_the_declared_paths_types_values_and_presence() {
    for case in cases() {
        let reading = EssConformanceCoverageReading::new(input(&case)).unwrap();
        let actual: BTreeMap<_, _> = reading
            .facts()
            .into_iter()
            .map(|(path, value)| (path.to_string(), value))
            .collect();
        assert_eq!(actual, expected(&case));
        assert_eq!(
            actual.len(),
            if case.component && case.explicit {
                31
            } else {
                27
            }
        );
    }
}

#[test]
fn coverage_raw_admitted_cloned_replaced_failed_and_replayed_fact_transitions_are_exact() {
    for case in cases() {
        let reader = Reader(input(&case));
        let mut sources =
            EssConformanceCoverageSources::new("report bytes".into(), "input bytes".into());
        assert_eq!(
            Evidence::EssConformanceCoverageV1(sources.clone()).facts(),
            [] as [(aep_domain::FactPath, aep_domain::FactValue); 0]
        );
        sources.admit(&reader).unwrap();
        let admitted = Evidence::EssConformanceCoverageV1(sources.clone());
        let expected = admitted.facts();
        assert_eq!(admitted.clone().facts(), expected);
        let wire = serde_json::to_string(&admitted).unwrap();
        assert!(!wire.contains("reading"));
        let raw: Evidence = serde_json::from_str(&wire).unwrap();
        assert_eq!(
            raw.facts(),
            [] as [(aep_domain::FactPath, aep_domain::FactValue); 0]
        );
        let Evidence::EssConformanceCoverageV1(mut replayed) = raw else {
            panic!("coverage");
        };
        replayed.admit(&reader).unwrap();
        assert_eq!(
            Evidence::EssConformanceCoverageV1(replayed).facts(),
            expected
        );
        let changed = EssConformanceCoverageSources::new(
            sources.report_json().into(),
            "replacement bytes".into(),
        );
        assert_eq!(
            Evidence::EssConformanceCoverageV1(changed).facts(),
            [] as [(aep_domain::FactPath, aep_domain::FactValue); 0]
        );
        assert_eq!(
            sources.admit(&Refusing).unwrap_err().issues[0].reason,
            "ReaderRefusal"
        );
        assert!(sources.reading().is_none());
        assert_eq!(
            Evidence::EssConformanceCoverageV1(sources.clone()).facts(),
            [] as [(aep_domain::FactPath, aep_domain::FactValue); 0]
        );
        sources.admit(&reader).unwrap();
        assert_eq!(
            Evidence::EssConformanceCoverageV1(sources).facts(),
            expected
        );
    }
}

#[test]
fn coverage_checked_constructor_refuses_profile_counts_order_selection_and_status_lies() {
    let case = cases().remove(0);
    let base = input(&case);
    let mut bad = base.clone();
    bad.counts.passed = u64::MAX;
    bad.counts.failed = 1;
    assert_eq!(
        EssConformanceCoverageReading::new(bad).unwrap_err().issues[0].reason,
        "CountOverflow"
    );
    let mut bad = base.clone();
    bad.conformance_status = CountStatus::Inconclusive;
    assert_eq!(
        EssConformanceCoverageReading::new(bad).unwrap_err().issues[0].reason,
        "ConformanceStatusMismatch"
    );
    let mut bad = base.clone();
    bad.execution_status = CountStatus::Failed;
    assert_eq!(
        EssConformanceCoverageReading::new(bad).unwrap_err().issues[0].reason,
        "ExecutionStatusMismatch"
    );
    let mut bad = base.clone();
    bad.coverage.selection.filter = Filter::Explicit {
        ids: vec![],
        parent: reference('c'),
    };
    assert_eq!(
        EssConformanceCoverageReading::new(bad).unwrap_err().issues[0].reason,
        "SelectedIdsMismatch"
    );
    let mut bad = base.clone();
    bad.counts.passed = 0;
    bad.counts.skipped = 1;
    bad.outcomes.swap(0, 4);
    assert_eq!(
        EssConformanceCoverageReading::new(bad).unwrap_err().issues[0].reason,
        "ProfileOutcomeMismatch"
    );
    let mut bad = base;
    bad.counts.passed = 0;
    bad.counts.error = 1;
    bad.outcomes.swap(0, 2);
    bad.producer_profile = ProducerProfile::Go;
    assert_eq!(
        EssConformanceCoverageReading::new(bad).unwrap_err().issues[0].reason,
        "ProfileOutcomeMismatch"
    );
}

#[test]
fn suite_reference_admits_every_coverage_version_ess_writes_and_names_any_other() {
    let digest = format!("sha256:{}", "a".repeat(64));
    let reference = |version: &str| {
        SuiteReference::new(version.into(), "sha256-json-bytes/1".into(), digest.clone())
    };
    // ESS writes declared coverage in the odd suite majors from /5 through /45 (ESS 0.55.0).
    for major in (5..=45).step_by(2) {
        let version = format!("ess-conformance/{major}");
        let admitted = reference(&version).unwrap_or_else(|error| panic!("{version}: {error}"));
        assert_eq!(admitted.version(), version);
        let wire = serde_json::json!({"version":version, "digest_profile":"sha256-json-bytes/1", "digest":digest});
        assert_eq!(
            serde_json::from_value::<SuiteReference>(wire).unwrap(),
            admitted
        );
    }
    for version in [
        "ess-conformance/4",
        "ess-conformance/6",
        "ess-conformance/32",
        "ess-conformance/34",
        "ess-conformance/44",
        "ess-conformance/46",
        "ess-conformance/47",
        "ess-conformance/05",
        "ess-conformance/+7",
        "ess-conformance/",
        "ess-conformance-input/1",
    ] {
        let error = reference(version).expect_err(version);
        let issue = &error.issues[0];
        assert_eq!(issue.reason, "UnsupportedSuiteVersion", "{version}");
        assert_eq!(issue.path, "$.suite.version");
        assert!(
            issue.detail.contains(version),
            "{version}: {}",
            issue.detail
        );
    }
}

#[test]
fn count_suite_reference_admits_the_frozen_and_ordinary_majors_and_names_any_other() {
    use aep_domain::ess_conformance_coverage::COVERAGE_SUITE_MAJORS;
    use aep_domain::ess_conformance_v2::{
        SuiteReference as CountReference, ORDINARY_SUITE_MAJORS,
    };
    let digest = format!("sha256:{}", "a".repeat(64));
    let reference = |version: &str| {
        CountReference::new(version.into(), "sha256-json-bytes/1".into(), digest.clone())
    };
    // Each ordinary major is the even neighbour below a coverage major the build knows, and the
    // two lists never share a major: what a suite is decides its reader, not its number.
    for major in ORDINARY_SUITE_MAJORS {
        assert_eq!(major % 2, 0, "{major}");
        assert!(COVERAGE_SUITE_MAJORS.contains(&(major + 1)), "{major}");
        assert!(!COVERAGE_SUITE_MAJORS.contains(major), "{major}");
    }
    // ESS writes an ordinary suite in the even majors from /6 through /44 (ESS 0.55.0).
    for major in (1..=4).chain((6..=44).step_by(2)) {
        let version = format!("ess-conformance/{major}");
        let admitted = reference(&version).unwrap_or_else(|error| panic!("{version}: {error}"));
        assert_eq!(admitted.version(), version);
        let wire = serde_json::json!({"version":version, "digest_profile":"sha256-json-bytes/1", "digest":digest});
        assert_eq!(
            serde_json::from_value::<CountReference>(wire).unwrap(),
            admitted
        );
    }
    for version in [
        "ess-conformance/5",
        "ess-conformance/35",
        "ess-conformance/45",
        "ess-conformance/46",
        "ess-conformance/47",
        "ess-conformance/06",
        "ess-conformance/+6",
        "ess-conformance/",
        "ess-conformance-input/1",
    ] {
        let error = reference(version).expect_err(version);
        let issue = &error.issues[0];
        assert_eq!(issue.reason, "UnsupportedSuiteVersion", "{version}");
        assert_eq!(issue.path, "$.suite.version");
        assert!(
            issue.detail.contains(version),
            "{version}: {}",
            issue.detail
        );
    }
}

/// Every form ESS's `ScenarioId::parse` reads (ESS 0.55.0,
/// `crates/verify/ess-conformance/src/scenario.rs`), one example each; every disclosure aspect
/// and both callers (`one_time_response/cells.rs` `Cell::parse`). The first eight are the frozen
/// suite/1–4 forms.
const CURRENT_IDS: [&str; 24] = [
    "billing.invoice.CreateInvoice/outcome/rejected",
    "billing.invoice.Invoice/transition/settle/by/billing.invoice.PayInvoice/settled",
    "billing.invoice.Invoice/state/Paid/refuses/billing.invoice.CancelInvoice",
    "billing.invoice.Invoice/state/Ended/accepts/billing.invoice.EndCall",
    "billing.invoice.Invoice/invariant/after/billing.invoice.CreateInvoice/accepted",
    "billing.invoice.Money/invariant/at/billing.invoice.InvoiceById/line.unit_price",
    "notify-on-invoice-created/binding/on-failure",
    "billing.invoice/authored/two-issued-invoices-rank-latest-first",
    "metrics.session.ByAgent/aggregate",
    "notify-ledger/binding/final-failure",
    "billing.invoice.IssueInvoice/grant/denied",
    "desk.ops.Tally/grant/admitted/desk.ops.Watcher",
    "desk.tickets.Board/grant/read/denied",
    "desk.tickets.Board/grant/read/admitted/desk.tickets.Clerk",
    "notify-ledger/binding/refusal/at-limit",
    "notify-ledger/binding/condition-false",
    "notify-ledger/binding/condition-absent",
    "auth.keys.Issue/disclosure/issued/secret/origin/as/anonymous",
    "auth.keys.Issue/disclosure/issued/secret/retry/as/actor/auth.keys.Owner",
    "auth.keys.Issue/disclosure/issued/_url/rotation/as/anonymous",
    "auth.keys.Issue/disclosure/issued/secret/read/auth.keys.KeyById/as/actor/auth.keys.Owner",
    "auth.keys.Issue/disclosure/issued/secret/command/auth.keys.Revoke/revoked/as/anonymous",
    "auth.keys.Issue/disclosure/issued/secret/denied/auth.keys.Revoke/as/actor/auth.keys.Other",
    "auth.keys.Issue/disclosure/issued/secret/origin/as/actor/anonymous",
];

fn authored_refusal(code: &str) -> Refusal {
    serde_json::from_value(serde_json::json!({"origin":"authored","scenario":null,"subject":null,"source":"refused.yaml","code":code,"message":"original authored cause","effect":"candidate_not_emitted","retained":null,"scope":"in_scope","needs":[]})).unwrap()
}

#[test]
fn every_authored_refusal_code_ess_0_55_names_a_file_with_is_admitted_and_no_other() {
    // ESS 0.55.0 `authored::names_file_refusal` (`authored.rs`): every key of the authored
    // catalogue, 001 through 041, but 036, which refuses the model before any file is read.
    for code in (1..=35).chain(37..=41) {
        let code = format!("ESS-AUTHOR-{code:03}");
        authored_refusal(&code)
            .validate()
            .unwrap_or_else(|error| panic!("{code}: {error}"));
    }
    for code in [
        "ESS-AUTHOR-036",
        "ESS-AUTHOR-042",
        "ESS-AUTHOR-000",
        "ESS-AUTHOR-38",
    ] {
        let error = authored_refusal(code).validate().expect_err(code);
        assert_eq!(error.issues[0].reason, "UnsupportedRefusalCode", "{code}");
        assert_eq!(error.issues[0].detail, code);
    }
}

/// Refuses each id as `MalformedScenarioId`, naming the id.
fn assert_malformed(ids: &[&str]) {
    for id in ids {
        let error = ScenarioId::new(*id).expect_err(id);
        assert_eq!(error.issues[0].reason, "MalformedScenarioId", "{id}");
        assert_eq!(error.issues[0].detail, *id);
        let wire = serde_json::from_value::<ScenarioId>(serde_json::json!(id)).expect_err(id);
        assert!(
            wire.to_string().contains("MalformedScenarioId"),
            "{id}: {wire}"
        );
    }
}

#[test]
fn a_later_scenario_form_with_an_empty_segment_is_malformed() {
    assert_malformed(&[
        "/grant/denied",
        "billing.invoice.IssueInvoice/grant/admitted/",
        "/grant/read/denied",
        "desk.tickets.Board/grant/read/admitted/",
        "/binding/refusal/at-limit",
        "notify-ledger/binding/refusal/",
        "/binding/condition-false",
        "auth.keys.Issue/disclosure//secret/origin/as/anonymous",
        "auth.keys.Issue/disclosure/issued//origin/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret//as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/read//as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/actor/",
        "/disclosure/issued/secret/origin/as/anonymous",
    ]);
}

#[test]
fn a_later_scenario_form_with_a_malformed_name_is_malformed() {
    assert_malformed(&[
        "billing..IssueInvoice/grant/denied",
        "1IssueInvoice/grant/denied",
        "desk.ops.Tally/grant/admitted/desk.ops.Watcher.",
        "desk.ops.Tally/grant/admitted/desk ops.Watcher",
        "desk.tickets.Board./grant/read/denied",
        "desk.tickets.Board/grant/read/admitted/desk.1Clerk",
        "Notify-ledger/binding/refusal/at-limit",
        "notify-ledger/binding/refusal/At-limit",
        "notify-ledger/binding/refusal/at--limit",
        "notify-ledger/binding/refusal/at-limit-",
        "notify_ledger/binding/condition-false",
        "Notify-ledger/binding/condition-absent",
        "auth..keys.Issue/disclosure/issued/secret/origin/as/anonymous",
        "auth.keys.Issue/disclosure/Issued/secret/origin/as/anonymous",
        "auth.keys.Issue/disclosure/issued/se-cret/origin/as/anonymous",
        "auth.keys.Issue/disclosure/issued/_/origin/as/anonymous",
        "auth.keys.Issue/disclosure/issued/1secret/origin/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/read/auth.keys.1KeyById/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/command/auth.keys.Revoke/Revoked/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/denied/auth keys/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/actor/auth..Owner",
    ]);
}

#[test]
fn a_later_scenario_form_with_an_unknown_keyword_is_malformed() {
    assert_malformed(&[
        "billing.invoice.IssueInvoice/grant/refused",
        "billing.invoice.IssueInvoice/grants/denied",
        "billing.invoice.IssueInvoice/grant/allowed/desk.ops.Watcher",
        "desk.tickets.Board/grant/read/refused",
        "desk.tickets.Board/grant/write/denied",
        "desk.tickets.Board/grant/read/allowed/desk.tickets.Clerk",
        "notify-ledger/binding/refusals/at-limit",
        "notify-ledger/binding/condition-true",
        "notify-ledger/binding/condition_false",
        "notify-ledger/binding/conditionabsent",
        "auth.keys.Issue/disclosure/issued/secret/replay/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/someone",
        "auth.keys.Issue/disclosure/issued/secret/origin/by/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/user/auth.keys.Owner",
        "auth.keys.Issue/disclosures/issued/secret/origin/as/anonymous",
    ]);
}

#[test]
fn a_later_scenario_form_missing_or_adding_a_segment_is_malformed() {
    assert_malformed(&[
        "billing.invoice.IssueInvoice/grant",
        "billing.invoice.IssueInvoice/grant/denied/extra",
        "billing.invoice.IssueInvoice/grant/admitted",
        "desk.ops.Tally/grant/admitted/desk.ops.Watcher/extra",
        "desk.tickets.Board/grant/read",
        "desk.tickets.Board/grant/read/denied/extra",
        "desk.tickets.Board/grant/read/admitted",
        "desk.tickets.Board/grant/read/admitted/desk.tickets.Clerk/extra",
        "notify-ledger/binding/refusal",
        "notify-ledger/binding/refusal/at-limit/extra",
        "notify-ledger/binding/condition-false/extra",
        "auth.keys.Issue/disclosure",
        "auth.keys.Issue/disclosure/issued",
        "auth.keys.Issue/disclosure/issued/secret",
        "auth.keys.Issue/disclosure/issued/secret/origin",
        "auth.keys.Issue/disclosure/issued/secret/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/origin/as",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/actor",
        "auth.keys.Issue/disclosure/issued/secret/origin/extra/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/read/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/read/auth.keys.KeyById/extra/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/command/auth.keys.Revoke/as/anonymous",
        "auth.keys.Issue/disclosure/issued/secret/denied/as/actor/auth.keys.Other",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/actor/auth.keys.Owner/extra",
        "auth.keys.Issue/disclosure/issued/secret/origin/as/anonymous/extra",
    ]);
}

#[test]
fn scenario_ids_follow_the_grammar_current_ess_parses_and_no_wider() {
    for id in CURRENT_IDS {
        let parsed = ScenarioId::new(id).unwrap_or_else(|error| panic!("{id}: {error}"));
        assert_eq!(parsed.as_str(), id);
        let wire: ScenarioId = serde_json::from_value(serde_json::json!(id)).unwrap();
        assert_eq!(wire, parsed);
    }
    for id in [
        "metrics.session.ByAgent/aggregate/extra",
        "/aggregate",
        "metrics..ByAgent/aggregate",
        "metrics.session.1ByAgent/aggregate",
        "metrics.session.By Agent/aggregate",
        "metrics.session.ByAgent/aggregates",
        "notify-ledger/binding/final_failure",
        "notify-ledger/binding/finalfailure",
        "Notify-ledger/binding/final-failure",
        "notify-ledger/binding",
    ] {
        let error = ScenarioId::new(id).expect_err(id);
        assert_eq!(error.issues[0].reason, "MalformedScenarioId", "{id}");
        assert_eq!(error.issues[0].detail, id);
    }
}

#[test]
fn the_frozen_count_grammar_still_refuses_forms_later_ess_added() {
    for id in &CURRENT_IDS[..8] {
        assert!(ScenarioId::frozen(*id).is_ok(), "{id}");
    }
    for id in &CURRENT_IDS[8..] {
        let error = ScenarioId::frozen(*id).expect_err(id);
        assert_eq!(error.issues[0].reason, "MalformedScenarioId", "{id}");
    }
}
