//! Complete-selection admission against independently authored sources.
#[path = "support/coverage.rs"]
mod fixtures;

use aep_domain::ess_conformance_coverage::{
    EssConformanceCoverageReader, EssConformanceCoverageSources,
};
use aep_domain::ess_conformance_v2::CountStatus;
use aep_ess_evidence::{adapt_json_coverage, CoverageReader};
use serde_json::{json, Value};

#[test]
fn coverage_complete_original_selection_admits_as_passed_for_both_profiles() {
    let (report, input) = fixtures::pair(1);
    for profile in ["rust-scenario-status/1", "go-scenario-status/1"] {
        let report = report.replace("rust-scenario-status/1", profile);
        let sources = EssConformanceCoverageSources::new(report.clone(), input.clone());
        let reading = CoverageReader
            .read(&sources)
            .expect("complete original selection must admit");
        assert_eq!(reading.data().conformance_status, CountStatus::Passed);
        assert_eq!(reading.data().counts.passed, 1);
        assert_eq!(reading.selected_ids()[0].as_str(), fixtures::SELECTED);
        assert_eq!(reading.data().spec_digest.as_str(), fixtures::MODEL);
        assert!(reading.data().coverage.complete());
        let adapted = adapt_json_coverage(&report, &input).unwrap();
        assert_eq!(adapted.observed_at().timestamp().epoch_millis(), 1);
        let wire = serde_json::to_value(adapted).unwrap();
        assert_eq!(wire["report_json"], report);
        assert_eq!(wire["suite_input_json"], input);
        assert!(wire.get("reading").is_none());
    }
}

#[test]
fn coverage_admission_retains_every_exact_unsigned_completion_boundary() {
    for time in [
        0,
        9_007_199_254_740_993,
        i64::MAX as u64,
        i64::MAX as u64 + 1,
        u64::MAX,
    ] {
        let (report, input) = fixtures::pair(time);
        let adapted = adapt_json_coverage(&report, &input).expect("wire range is full u64");
        assert_eq!(adapted.observed_at().timestamp().epoch_millis(), time);
    }
}

fn refuses(report: &str, input: &str, expected: &str) {
    let error = adapt_json_coverage(report, input).expect_err(expected);
    assert!(
        error.issues.iter().any(|issue| issue.reason == expected),
        "expected {expected}: {error}"
    );
    assert!(error.issues.iter().all(|issue| issue.path.starts_with('$')));
}

fn altered(text: &str, change: impl FnOnce(&mut Value)) -> String {
    let mut value = serde_json::from_str(text).unwrap();
    change(&mut value);
    value.to_string()
}

#[test]
fn coverage_report_scalars_profiles_counts_and_terminal_ids_refuse_contradictions() {
    let (report, input) = fixtures::pair(1);
    for (field, value, reason) in [
        (
            "format",
            "ess-conformance-run/2",
            "UnsupportedReportVersion",
        ),
        (
            "producer_profile",
            "untrusted/1",
            "UnsupportedProducerProfile",
        ),
        ("policy", "optimistic/1", "UnsupportedPolicy"),
        (
            "execution_status",
            "inconclusive",
            "ExecutionStatusMismatch",
        ),
        (
            "conformance_status",
            "inconclusive",
            "ConformanceStatusMismatch",
        ),
        ("execution_status", "unknown", "UnsupportedStatus"),
    ] {
        refuses(
            &altered(&report, |r| r[field] = value.into()),
            &input,
            reason,
        );
    }
}

#[test]
fn coverage_exact_scalars_counts_and_terminal_ids_refuse_contradictions() {
    let (report, input) = fixtures::pair(1);
    for (token, reason) in [
        ("-0", "UnsignedIntegerRequired"),
        ("-1", "UnsignedIntegerRequired"),
        ("1.0", "UnsignedIntegerRequired"),
        ("1e0", "UnsignedIntegerRequired"),
        ("\"1\"", "UnsignedIntegerRequired"),
        ("18446744073709551616", "UnsignedIntegerOverflow"),
        ("+1", "InvalidDocument"),
        ("01", "InvalidDocument"),
    ] {
        refuses(
            &report.replace("\"completed_at\":1", &format!("\"completed_at\":{token}")),
            &input,
            reason,
        );
        refuses(
            &report.replace("\"total\":1", &format!("\"total\":{token}")),
            &input,
            reason,
        );
    }
    refuses(
        &altered(&report, |r| {
            r["counts"]["passed"] = u64::MAX.into();
            r["counts"]["failed"] = 1.into();
        }),
        &input,
        "CountOverflow",
    );
    refuses(
        &altered(&report, |r| r["counts"]["total"] = 2.into()),
        &input,
        "TotalMismatch",
    );
    refuses(
        &altered(&report, |r| r["outcomes"]["passed"] = json!([])),
        &input,
        "OutcomeLengthMismatch",
    );
    refuses(
        &altered(&report, |r| {
            r["outcomes"]["failed"] = json!([fixtures::SELECTED]);
            r["counts"]["failed"] = 1.into();
            r["counts"]["total"] = 2.into();
        }),
        &input,
        "DuplicateOutcome",
    );
    refuses(
        &altered(&report, |r| {
            r["outcomes"]["passed"] = json!(["demo.core.Other/outcome/ready"]);
        }),
        &input,
        "SelectedIdsMismatch",
    );
    refuses(
        &altered(&report, |r| r["spec_digest"] = "a".repeat(64).into()),
        &input,
        "ModelDigestMismatch",
    );
    refuses(
        &altered(&report, |r| r["spec_digest"] = "1234567890abcdef".into()),
        &input,
        "MalformedModelDigest",
    );
    refuses(
        &altered(&report, |r| {
            r["suite"]["digest"] = format!("sha256:{}", "b".repeat(64)).into();
        }),
        &input,
        "SuiteDigestMismatch",
    );
}

#[test]
fn coverage_closed_original_transport_detects_duplicates_unknown_fields_and_versions() {
    let (report, input) = fixtures::pair(1);
    refuses(
        &report.replacen('{', "{\"format\":\"ess-conformance-report/2\",", 1),
        &input,
        "DuplicateKey",
    );
    refuses(
        &report,
        &input.replacen('{', "{\"format\":\"ess-conformance-input/1\",", 1),
        "DuplicateKey",
    );
    refuses(
        &report,
        &altered(&input, |i| i["trusted"] = true.into()),
        "UnknownField",
    );
    refuses(
        &altered(&report, |r| r["trusted"] = true.into()),
        &input,
        "UnknownField",
    );
    refuses(
        &report,
        &altered(&input, |i| i["format"] = "ess-conformance-input/2".into()),
        "UnsupportedInputVersion",
    );
    for version in [
        "ess-conformance/1",
        "ess-conformance/4",
        "ess-conformance/6",
        "ess-conformance/99",
    ] {
        let mut suite = fixtures::suite();
        suite["provenance"]["suite_version"] = version.into();
        let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
        refuses(&report, &input, "UnsupportedSuiteVersion");
    }
    let suite = fixtures::suite();
    for bad in [
        suite.to_string().replacen('{', "{\"coverage\":{},", 1),
        suite
            .to_string()
            .replace("\"steps\":[]", "\"steps\":[],\"\\u0073teps\":[]"),
    ] {
        refuses(
            &report,
            &altered(&input, |i| i["suite_json"] = bad.into()),
            "DuplicateKey",
        );
    }
    for path in [
        vec!["provenance"],
        vec!["coverage"],
        vec!["scenarios", fixtures::SELECTED],
    ] {
        let mut suite = fixtures::suite();
        let target = path.iter().fold(&mut suite, |value, key| &mut value[*key]);
        target["future"] = true.into();
        let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
        refuses(&report, &input, "UnknownField");
    }
}

#[test]
fn coverage_hashes_original_inner_bytes_and_preserves_open_payload_data() {
    let (report, input) = fixtures::pair(1);
    let original = fixtures::suite().to_string();
    for changed in [
        format!("{original}\n"),
        original.replacen(':', ": ", 1),
        original.replacen("demo", "\\u0064emo", 1),
    ] {
        refuses(
            &report,
            &altered(&input, |i| i["suite_json"] = changed.into()),
            "SuiteDigestMismatch",
        );
    }
    let mut suite = fixtures::suite();
    suite["scenarios"][fixtures::SELECTED]["steps"] = json!([{"step":"expect_event", "event":"demo.core.Ready", "payload":{"trusted":true,"coverage":{"future":-0.0},"list":[1.0,"e\u{301}","\n"]}}]);
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    adapt_json_coverage(&report, &input).unwrap();
    suite["scenarios"][fixtures::SELECTED]["steps"][0]["future"] = true.into();
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    refuses(&report, &input, "UnknownField");
}

#[test]
fn coverage_inventory_membership_counts_origins_and_component_agree() {
    let mut variants = Vec::new();
    let mut suite = fixtures::suite();
    suite["coverage"]["counts"]["generated"] = 2.into();
    variants.push((suite, "CoverageCountMismatch"));
    let mut suite = fixtures::suite();
    suite["coverage"]["generated"] = json!([]);
    variants.push((suite, "CoverageCountMismatch"));
    let mut suite = fixtures::suite();
    suite["coverage"]["generated"] = json!([fixtures::SELECTED, fixtures::SELECTED]);
    suite["coverage"]["counts"]["generated"] = 2.into();
    variants.push((suite, "CoverageCountMismatch"));
    let mut suite = fixtures::suite();
    suite["coverage"]["selection"]["origins"] = "authored".into();
    variants.push((suite, "OriginSelectionMismatch"));
    let mut suite = fixtures::suite();
    suite["coverage"]["selection"]["scope"] = json!({"kind":"component","component":"worker"});
    variants.push((suite, "ComponentSelectionMismatch"));
    let mut suite = fixtures::suite();
    suite["coverage"]["outside"] = json!([{"scenario":fixtures::SELECTED,"origin":"generated","reason":"origin_selection","needs":[]}]);
    suite["coverage"]["counts"]["outside"] = 1.into();
    variants.push((suite, "OutsideSelectedOverlap"));
    for (suite, reason) in variants {
        let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
        refuses(&report, &input, reason);
    }
}

#[test]
fn coverage_refusal_occurrences_are_retained_and_never_counted_as_executions() {
    let mut suite = fixtures::suite();
    suite["coverage"]["refused"] = json!([fixtures::refusal(), fixtures::refusal()]);
    suite["coverage"]["counts"]["refused"] = 2.into();
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "inconclusive", &[]);
    let adapted = adapt_json_coverage(&report, &input).unwrap();
    let aep_domain::Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("coverage");
    };
    let data = sources.reading().unwrap().data();
    assert_eq!(data.counts.total, 1);
    assert_eq!(data.coverage.refused.len(), 2);
    assert_eq!(data.coverage.counts.refused, 2);
    assert!(!data.coverage.complete());
    refuses(
        &altered(&report, |r| {
            r["coverage"]["refused"].as_array_mut().unwrap().pop();
            r["coverage"]["counts"]["refused"] = 1.into();
        }),
        &input,
        "CoverageMismatch",
    );
    let child = fixtures::child(&suite, &[]);
    let (report, input) = fixtures::pair_for(&child, &[], "inconclusive", &[suite.to_string()]);
    let admitted = adapt_json_coverage(&report, &input).unwrap();
    let aep_domain::Evidence::EssConformanceCoverageV1(sources) = admitted.evidence() else {
        panic!("coverage");
    };
    assert_eq!(sources.reading().unwrap().data().coverage.refused.len(), 2);
    assert_eq!(sources.reading().unwrap().data().counts.total, 0);
}

#[test]
fn coverage_refusal_mapping_identity_scope_and_occurrence_order_are_checked() {
    let mut baseline = fixtures::suite();
    baseline["coverage"]["refused"] = json!([fixtures::refusal()]);
    baseline["coverage"]["counts"]["refused"] = 1.into();
    for (field, value, reason) in [
        (
            "effect",
            json!("candidate_not_emitted"),
            "RefusalEffectMismatch",
        ),
        ("code", json!("ESS-SYNTH-999"), "UnsupportedRefusalCode"),
        ("code", json!("ESS-AUTHOR-001"), "UnsupportedRefusalCode"),
        ("message", json!(" "), "EmptyRefusalMessage"),
        ("subject", Value::Null, "RefusalIdentityMismatch"),
        ("retained", Value::Null, "RetainedIdentityMismatch"),
        ("scope", json!("outside_origin"), "RefusalScopeMismatch"),
        (
            "needs",
            json!([{"kind":"component","name":"other"}]),
            "NeedsMismatch",
        ),
    ] {
        let mut suite = baseline.clone();
        suite["coverage"]["refused"][0][field] = value;
        let (report, input) =
            fixtures::pair_for(&suite, &[fixtures::SELECTED], "inconclusive", &[]);
        refuses(&report, &input, reason);
    }
    let mut suite = baseline.clone();
    suite["coverage"]["refused"][0]["retained"]["future"] = true.into();
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "inconclusive", &[]);
    refuses(&report, &input, "UnknownField");
    for number in 1..=14 {
        let mut suite = baseline.clone();
        suite["coverage"]["refused"][0]["code"] = format!("ESS-SYNTH-{number:03}").into();
        suite["coverage"]["refused"][0]["effect"] = if [5, 11, 12, 14].contains(&number) {
            "check_not_emitted"
        } else {
            "candidate_not_emitted"
        }
        .into();
        let (report, input) =
            fixtures::pair_for(&suite, &[fixtures::SELECTED], "inconclusive", &[]);
        adapt_json_coverage(&report, &input).unwrap();
    }
}

fn lineage() -> (Value, Value, Value) {
    let mut ancestor = fixtures::suite();
    for id in [
        "demo.core.Write/outcome/ready",
        "demo.core.Zap/outcome/ready",
    ] {
        ancestor["scenarios"][id] = ancestor["scenarios"][fixtures::SELECTED].clone();
        ancestor["coverage"]["generated"]
            .as_array_mut()
            .unwrap()
            .push(id.into());
    }
    ancestor["coverage"]["counts"]["generated"] = 3.into();
    let parent = fixtures::child(
        &ancestor,
        &[fixtures::SELECTED, "demo.core.Write/outcome/ready"],
    );
    let child = fixtures::child(&parent, &[fixtures::SELECTED]);
    (ancestor, parent, child)
}

#[test]
fn coverage_complete_multi_generation_lineage_is_admitted_before_the_child() {
    let (ancestor, parent, child) = lineage();
    let (report, input) = fixtures::pair_for(
        &child,
        &[fixtures::SELECTED],
        "passed",
        &[parent.to_string(), ancestor.to_string()],
    );
    adapt_json_coverage(&report, &input).unwrap();
    for (parents, reason) in [
        (vec![], "MissingParent"),
        (vec![parent.to_string()], "MissingParent"),
        (
            vec![ancestor.to_string(), parent.to_string()],
            "MissingParent",
        ),
        (
            vec![
                parent.to_string(),
                ancestor.to_string(),
                ancestor.to_string(),
            ],
            "RepeatedParent",
        ),
        (
            vec![format!("{parent}\n"), ancestor.to_string()],
            "ParentReferenceMismatch",
        ),
        (
            vec![
                parent.to_string(),
                ancestor
                    .to_string()
                    .replace("ess-conformance/5", "ess-conformance/4"),
            ],
            "UnsupportedSuiteVersion",
        ),
    ] {
        let (report, input) = fixtures::pair_for(&child, &[fixtures::SELECTED], "passed", &parents);
        refuses(&report, &input, reason);
    }
    let (report, input) = fixtures::pair_for(
        &ancestor,
        &[
            fixtures::SELECTED,
            "demo.core.Write/outcome/ready",
            "demo.core.Zap/outcome/ready",
        ],
        "passed",
        &[fixtures::suite().to_string()],
    );
    refuses(&report, &input, "SurplusParent");
}

#[test]
fn coverage_parent_comparison_covers_surviving_steps_dependencies_provenance_and_inventory() {
    let (ancestor, parent, child) = lineage();
    let parents = [parent.to_string(), ancestor.to_string()];
    let mut changed = child.clone();
    changed["scenarios"][fixtures::SELECTED]["purpose"] = "A changed surviving purpose".into();
    let (r, i) = fixtures::pair_for(&changed, &[fixtures::SELECTED], "passed", &parents);
    refuses(&r, &i, "ParentScenarioMismatch");
    let mut changed = child.clone();
    changed["scenarios"][fixtures::SELECTED]["steps"] =
        json!([{"step":"expect_no_event", "event":"demo.core.Ready"}]);
    let (r, i) = fixtures::pair_for(&changed, &[fixtures::SELECTED], "passed", &parents);
    refuses(&r, &i, "ParentScenarioMismatch");
    let mut changed = child.clone();
    changed["scenarios"][fixtures::SELECTED]["source"] =
        json!([{"kind":"command", "name":"demo.core.Read"}]);
    let (r, i) = fixtures::pair_for(&changed, &[fixtures::SELECTED], "passed", &parents);
    refuses(&r, &i, "ParentScenarioMismatch");
    let mut changed = child.clone();
    changed["provenance"]["system"] = "other".into();
    let (r, i) = fixtures::pair_for(&changed, &[fixtures::SELECTED], "passed", &parents);
    refuses(&r, &i, "ParentProvenanceMismatch");
    let mut changed = child.clone();
    changed["coverage"]["knowledge"] = "unknown".into();
    let (r, i) = fixtures::pair_for(&changed, &[fixtures::SELECTED], "inconclusive", &parents);
    refuses(&r, &i, "ParentSelectionMismatch");
    let mut changed = child;
    changed["coverage"]["outside"].as_array_mut().unwrap().pop();
    changed["coverage"]["counts"]["outside"] = 1.into();
    let (r, i) = fixtures::pair_for(&changed, &[fixtures::SELECTED], "passed", &parents);
    refuses(&r, &i, "ParentOutsideMismatch");
}

#[test]
fn coverage_authored_source_ownership_keeps_rejected_known_ids_and_all_causes() {
    let authored = "demo.core/authored/one";
    let mut suite = fixtures::suite();
    suite["scenarios"][authored] = suite["scenarios"][fixtures::SELECTED].clone();
    suite["scenarios"]
        .as_object_mut()
        .unwrap()
        .remove(fixtures::SELECTED);
    suite["coverage"]["generated"] = json!([]);
    suite["coverage"]["authored"] = json!([authored]);
    suite["coverage"]["counts"]["generated"] = 0.into();
    suite["coverage"]["counts"]["authored"] = 1.into();
    suite["coverage"]["counts"]["refused"] = 1.into();
    let source_digest = format!("sha256:{}", "a".repeat(64));
    suite["coverage"]["authored_sources"] = json!({"accepted.yaml":{"digest":source_digest,"scenario":authored,"disposition":"accepted"}, "rejected.yaml":{"digest":source_digest,"scenario":authored,"disposition":"refused"}});
    let mut refusal = fixtures::refusal();
    refusal["origin"] = "authored".into();
    refusal["scenario"] = authored.into();
    refusal["subject"] = Value::Null;
    refusal["source"] = "rejected.yaml".into();
    refusal["effect"] = "candidate_not_emitted".into();
    refusal["retained"] = json!({"origin":"authored", "source":"accepted.yaml"});
    for number in 1..=35 {
        refusal["code"] = format!("ESS-AUTHOR-{number:03}").into();
        suite["coverage"]["refused"] = json!([refusal]);
        let (report, input) = fixtures::pair_for(&suite, &[authored], "inconclusive", &[]);
        adapt_json_coverage(&report, &input).unwrap();
    }
    let mut wrong = suite.clone();
    wrong["coverage"]["refused"][0]["retained"]["source"] = "rejected.yaml".into();
    let (r, i) = fixtures::pair_for(&wrong, &[authored], "inconclusive", &[]);
    refuses(&r, &i, "RetainedIdentityMismatch");
    let mut wrong = suite.clone();
    wrong["coverage"]["authored_sources"]["rejected.yaml"]["disposition"] = "accepted".into();
    let (r, i) = fixtures::pair_for(&wrong, &[authored], "inconclusive", &[]);
    refuses(&r, &i, "AuthoredOwnershipMismatch");
    for name in [
        "/absolute.yaml",
        "a//b.yaml",
        "a/./b.yaml",
        "a/../b.yaml",
        "C:x.yaml",
        "a\\b.yaml",
        "a\nb.yaml",
        "",
    ] {
        let mut wrong = suite.clone();
        let entry = wrong["coverage"]["authored_sources"]["accepted.yaml"].clone();
        wrong["coverage"]["authored_sources"]
            .as_object_mut()
            .unwrap()
            .insert(name.into(), entry);
        let (r, i) = fixtures::pair_for(&wrong, &[authored], "inconclusive", &[]);
        refuses(&r, &i, "MalformedSourceIdentity");
    }
}

#[test]
fn coverage_inherits_checked_suite_four_steps_predicates_and_dependencies() {
    for (step, reason) in [
        (json!({"step":"future_step"}), "UnsupportedStep"),
        (
            json!({"step":"expect_event","event":"demo.core.Ready","shape":{"value":{"holds":"primitive","kind":"binary64"}}}),
            "UnsupportedPrimitive",
        ),
        (
            json!({"step":"expect_within","instant":"start","elapsed":4_294_967_296_u64}),
            "InvalidElapsed",
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"satisfies","predicate":{"forall":{"in":"rows","as":"row.value","that":true}}}}),
            "InvalidPredicate",
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"satisfies","predicate":{"left":{"bogus":1}}}}),
            "InvalidPredicate",
        ),
    ] {
        let mut suite = fixtures::suite();
        suite["scenarios"][fixtures::SELECTED]["steps"] = json!([step]);
        let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
        refuses(&report, &input, reason);
    }
    let mut suite = fixtures::suite();
    suite["scenarios"][fixtures::SELECTED]["source"] =
        json!([{"kind":"future","name":"demo.core.Read"}]);
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    refuses(&report, &input, "UnsupportedSemanticReference");
    suite["scenarios"][fixtures::SELECTED]["source"] = json!([]);
    suite["scenarios"][fixtures::SELECTED]["steps"] = json!([{"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"satisfies","predicate":{"forall":{"in":"rows","as":"row","that":{"row.value":">= 0"}}}}}]);
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    adapt_json_coverage(&report, &input).unwrap();
    let mut invalid = suite;
    invalid["scenarios"][fixtures::SELECTED]["steps"][0]["expectation"]["predicate"] =
        json!({"bad..path":true});
    let (report, input) = fixtures::pair_for(&invalid, &[fixtures::SELECTED], "passed", &[]);
    refuses(&report, &input, "InvalidPredicate");
}

#[test]
fn coverage_filter_cannot_promote_unknown_erase_refusals_or_round_large_survivor_integers() {
    let mut parent = fixtures::suite();
    parent["coverage"]["knowledge"] = "unknown".into();
    parent["coverage"]["refused"] = json!([fixtures::refusal(), fixtures::refusal()]);
    parent["coverage"]["counts"]["refused"] = 2.into();
    parent["scenarios"][fixtures::SELECTED]["steps"] =
        json!([{"step":"expect_halt","view":"demo.core.Reads","after":9_007_199_254_740_993_u64}]);
    let child = fixtures::child(&parent, &[fixtures::SELECTED]);
    for (field, expected) in [
        ("knowledge", "ParentSelectionMismatch"),
        ("refusals", "ParentInventoryMismatch"),
        ("after", "ParentScenarioMismatch"),
    ] {
        let mut changed = child.clone();
        match field {
            "knowledge" => changed["coverage"]["knowledge"] = "complete_inventory".into(),
            "refusals" => {
                changed["coverage"]["refused"] = json!([fixtures::refusal()]);
                changed["coverage"]["counts"]["refused"] = 1.into();
            }
            _ => {
                changed["scenarios"][fixtures::SELECTED]["steps"][0]["after"] =
                    9_007_199_254_740_992_u64.into();
            }
        }
        let (report, input) = fixtures::pair_for(
            &changed,
            &[fixtures::SELECTED],
            "inconclusive",
            &[parent.to_string()],
        );
        refuses(&report, &input, expected);
    }
    let (report, input) = fixtures::pair_for(
        &child,
        &[fixtures::SELECTED],
        "inconclusive",
        &[parent.to_string()],
    );
    adapt_json_coverage(&report, &input).unwrap();
}

#[test]
fn coverage_outside_component_witness_is_complete_and_checked_without_erasing_refusal() {
    let suite = fixtures::outside_suite();
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
    let admitted = adapt_json_coverage(&report, &input).unwrap();
    let aep_domain::Evidence::EssConformanceCoverageV1(source) = admitted.evidence() else {
        panic!("coverage");
    };
    assert!(source.reading().unwrap().data().coverage.complete());
    assert_eq!(source.reading().unwrap().data().coverage.counts.refused, 1);
    let mut wrong = suite;
    wrong["coverage"]["refused"][0]["needs"] =
        json!([{"kind":"command","name":"demo.core.Unrelated"}]);
    let (report, input) = fixtures::pair_for(&wrong, &[fixtures::SELECTED], "passed", &[]);
    refuses(&report, &input, "RefusalScopeMismatch");
}

#[test]
fn coverage_flat_parent_lineage_has_no_artificial_chain_length_limit() {
    let mut suite = fixtures::suite();
    let mut parents = Vec::new();
    for _ in 0..130 {
        let child = fixtures::child(&suite, &[fixtures::SELECTED]);
        parents.insert(0, suite.to_string());
        suite = child;
    }
    let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &parents);
    adapt_json_coverage(&report, &input).unwrap();
}
