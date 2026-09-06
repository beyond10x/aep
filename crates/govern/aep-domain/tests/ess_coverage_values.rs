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
        assert!(Evidence::EssConformanceCoverageV1(sources.clone())
            .facts()
            .is_empty());
        sources.admit(&reader).unwrap();
        let admitted = Evidence::EssConformanceCoverageV1(sources.clone());
        let expected = admitted.facts();
        assert_eq!(admitted.clone().facts(), expected);
        let wire = serde_json::to_string(&admitted).unwrap();
        assert!(!wire.contains("reading"));
        let raw: Evidence = serde_json::from_str(&wire).unwrap();
        assert!(raw.facts().is_empty());
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
        assert!(Evidence::EssConformanceCoverageV1(changed)
            .facts()
            .is_empty());
        assert_eq!(
            sources.admit(&Refusing).unwrap_err().issues[0].reason,
            "ReaderRefusal"
        );
        assert!(sources.reading().is_none());
        assert!(Evidence::EssConformanceCoverageV1(sources.clone())
            .facts()
            .is_empty());
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
