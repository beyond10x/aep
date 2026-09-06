//! Original-byte count-stage admission, independent of an ESS writer release.
use aep_domain::ess_conformance_v2::{
    CountStatus, EssConformanceV2Reader, EssConformanceV2Sources,
};
use aep_ess_evidence::{adapt_json_v2, CountStageReader};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const MODEL: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";

fn pair(version: u8) -> (String, String) {
    let suite = format!(
        r#"{{"provenance":{{"suite_version":"ess-conformance/{version}","system":"demo","specification_version":"v1","spec_digest":"{MODEL}","contract_digest":"{MODEL}"}},"scenarios":{{"demo.core/authored/one":{{"purpose":"A concrete observation","steps":[{{"step":"expect_event","event":"demo.core.Observed","payload":{{"kind":"payload keys are arbitrary","nested":{{"unfamiliar":true}}}}}}],"source":[{{"kind":"event","name":"demo.core.Observed"}}]}}}}}}"#
    );
    let digest =
        Sha256::digest(suite.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    let report = serde_json::json!({
        "format":"ess-conformance-report/2", "specification":"demo/v1", "spec_digest":MODEL,
        "implementation":"independent fixture", "producer_profile":"rust-scenario-status/1",
        "suite":{"version":format!("ess-conformance/{version}"),"digest_profile":"sha256-json-bytes/1","digest":digest},
        "execution_status":"passed", "counts":{"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0},
        "outcomes":{"passed":["demo.core/authored/one"],"failed":[],"error":[],"unsupported":[],"skipped":[]},
        "coverage":{"knowledge":"unknown"},"conformance_status":"inconclusive","policy":"complete-selection/1","completed_at":9_007_199_254_740_993_u64
    }).to_string();
    (report, suite)
}

#[test]
fn original_suite_pairs_admit_with_exact_time_and_unknown_coverage() {
    for version in 1..=4 {
        let (report, suite) = pair(version);
        let sources = EssConformanceV2Sources::new(report.clone(), suite.clone());
        let reading = CountStageReader.read(&sources).unwrap();
        assert_eq!(reading.data().counts.passed, 1);
        assert_eq!(
            reading.data().completed_at.epoch_millis(),
            9_007_199_254_740_993
        );
        assert_eq!(reading.data().conformance_status, CountStatus::Inconclusive);
        let adapted = adapt_json_v2(&report, &suite).unwrap();
        assert_eq!(
            adapted.observed_at().timestamp().epoch_millis(),
            9_007_199_254_740_993
        );
        let serialized = serde_json::to_value(adapted).unwrap();
        assert_eq!(serialized["report_json"], report);
        assert_eq!(serialized["suite_json"], suite);
        assert!(serialized.get("reading").is_none());
    }
}

fn changed(report: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut report: serde_json::Value = serde_json::from_str(report).unwrap();
    edit(&mut report);
    report.to_string()
}
fn refuses(report: &str, suite: &str, expected: &str) {
    let error = adapt_json_v2(report, suite).expect_err(expected);
    assert!(
        error.issues.iter().any(|issue| issue.reason == expected),
        "expected {expected}: {error}"
    );
    assert!(error.issues.iter().all(|issue| issue.path.starts_with('$')));
}

#[test]
fn unsupported_format_profile_policy_and_coverage_reasons_are_distinct() {
    let (report, suite) = pair(4);
    for (field, value, reason) in [
        (
            "format",
            "ess-conformance-run/2",
            "UnsupportedReportVersion",
        ),
        (
            "producer_profile",
            "unknown/1",
            "UnsupportedProducerProfile",
        ),
        ("policy", "optimistic/1", "UnsupportedPolicy"),
        ("conformance_status", "passed", "ConformanceStatusMismatch"),
    ] {
        refuses(
            &changed(&report, |r| r[field] = value.into()),
            &suite,
            reason,
        );
    }
    refuses(
        &changed(&report, |r| {
            r["suite"]["version"] = "ess-conformance/5".into();
        }),
        &suite,
        "UnsupportedSuiteVersion",
    );
    refuses(
        &changed(&report, |r| {
            r["suite"]["digest_profile"] = "canonical/1".into();
        }),
        &suite,
        "UnsupportedDigestProfile",
    );
    refuses(
        &changed(&report, |r| {
            r["coverage"]["knowledge"] = "complete_inventory".into();
        }),
        &suite,
        "UnsupportedCoverage",
    );
    refuses(
        &changed(&report, |r| {
            r["coverage"]["inventory"] = serde_json::json!([]);
        }),
        &suite,
        "UnknownField",
    );
    refuses(
        &changed(&report, |r| r["spec_digest"] = "1234567890abcdef".into()),
        &suite,
        "MalformedModelDigest",
    );
}

#[test]
fn detailed_run_and_future_suite_refuse_before_interpreting_their_body() {
    let (_, suite) = pair(4);
    refuses(
        r#"{"format":"ess-conformance-run/2","summary":{}}"#,
        &suite,
        "UnsupportedReportVersion",
    );
    let (report, _) = pair(4);
    refuses(
        &report,
        r#"{"provenance":{"suite_version":"ess-conformance/5"},"coverage":{"knowledge":"complete_inventory"}}"#,
        "UnsupportedSuiteVersion",
    );
}

#[test]
fn exact_original_suite_bytes_and_selected_membership_are_required() {
    let (report, suite) = pair(4);
    for changed in [
        format!("{suite}\n"),
        suite.replacen("demo", "\\u0064emo", 1),
        suite.replacen(':', ": ", 1),
    ] {
        refuses(&report, &changed, "SuiteDigestMismatch");
    }
    refuses(&report, "", "MissingSuite");
    refuses(
        &report,
        &suite.replacen("ess-conformance/4", "ess-conformance/3", 1),
        "SuiteVersionMismatch",
    );
    refuses(
        &report,
        &suite.replacen(MODEL, &"a".repeat(64), 1),
        "ModelDigestMismatch",
    );
    let extra = changed(&report, |r| {
        r["outcomes"]["passed"] = serde_json::json!(["demo.core/authored/other"]);
    });
    refuses(&extra, &suite, "SelectedIdsMismatch");
}

#[test]
fn duplicate_and_unknown_closed_fields_refuse_even_inside_unused_metadata() {
    let (report, suite) = pair(4);
    for bad in [
        report.replacen('{', "{\"format\":\"ess-conformance-report/2\",", 1),
        report.replace("\"total\":1", "\"total\":1,\"total\":1"),
        report.replace("\"passed\":[", "\"failed\":[],\"passed\":["),
    ] {
        refuses(&bad, &suite, "DuplicateKey");
    }
    for bad in [
        suite.replacen('{', "{\"provenance\":{},", 1),
        suite.replace("\"nested\":{", "\"nested\":{},\"nested\":{"),
    ] {
        refuses(&report, &bad, "DuplicateKey");
    }
    for bad in [
        suite.replace("\"purpose\":", "\"verified\":true,\"purpose\":"),
        suite.replace(
            "\"step\":\"expect_event\"",
            "\"step\":\"expect_event\",\"future\":1",
        ),
        suite.replace("\"kind\":\"event\"", "\"kind\":\"event\",\"future\":1"),
    ] {
        refuses(&report, &bad, "UnknownField");
    }
    refuses(
        &report,
        &suite.replace("\"step\":\"expect_event\"", "\"step\":\"future_step\""),
        "UnsupportedStep",
    );
    refuses(
        &report,
        &suite.replace("\"kind\":\"event\"", "\"kind\":\"future_ref\""),
        "UnsupportedSemanticReference",
    );
    assert!(
        adapt_json_v2(&report, &suite).is_ok(),
        "arbitrary declared payload keys are valid"
    );
}

#[test]
fn report_completion_tokens_are_exact_and_lexically_unsigned() {
    let (report, suite) = pair(4);
    for value in [
        0,
        9_007_199_254_740_993,
        i64::MAX as u64,
        i64::MAX as u64 + 1,
        u64::MAX,
    ] {
        let report = report.replace("9007199254740993", &value.to_string());
        assert_eq!(
            adapt_json_v2(&report, &suite)
                .unwrap()
                .observed_at()
                .timestamp()
                .epoch_millis(),
            value
        );
    }
    for value in ["-1", "-0", "1.0", "1e0", "\"1\""] {
        refuses(
            &report.replace("9007199254740993", value),
            &suite,
            "UnsignedIntegerRequired",
        );
    }
    for value in ["+1", "01"] {
        refuses(
            &report.replace("9007199254740993", value),
            &suite,
            "InvalidDocument",
        );
    }
    refuses(
        &report.replace("9007199254740993", "18446744073709551616"),
        &suite,
        "UnsignedIntegerOverflow",
    );
}

#[test]
fn category_arithmetic_lists_and_producer_aggregates_are_checked() {
    let (report, suite) = pair(4);
    refuses(
        &changed(&report, |r| {
            r["counts"]["passed"] = u64::MAX.into();
            r["counts"]["failed"] = 1.into();
        }),
        &suite,
        "CountOverflow",
    );
    refuses(
        &changed(&report, |r| r["counts"]["total"] = 2.into()),
        &suite,
        "TotalMismatch",
    );
    refuses(
        &changed(&report, |r| r["outcomes"]["passed"] = serde_json::json!([])),
        &suite,
        "OutcomeLengthMismatch",
    );
    refuses(
        &changed(&report, |r| {
            r["outcomes"]["passed"] =
                serde_json::json!(["demo.core/authored/two", "demo.core/authored/one"]);
            r["counts"]["passed"] = 2.into();
            r["counts"]["total"] = 2.into();
        }),
        &suite,
        "OutcomeOrder",
    );
    refuses(
        &changed(&report, |r| {
            r["outcomes"]["failed"] = r["outcomes"]["passed"].clone();
            r["counts"]["failed"] = 1.into();
            r["counts"]["total"] = 2.into();
        }),
        &suite,
        "DuplicateOutcome",
    );
    for (profile, category, execution, conformance) in [
        (
            "rust-scenario-status/1",
            "error",
            "inconclusive",
            "inconclusive",
        ),
        ("rust-scenario-status/1", "unsupported", "failed", "failed"),
        (
            "go-scenario-status/1",
            "skipped",
            "inconclusive",
            "inconclusive",
        ),
        ("go-scenario-status/1", "failed", "failed", "failed"),
    ] {
        let variant = changed(&report, |r| {
            r["producer_profile"] = profile.into();
            r["counts"]["passed"] = 0.into();
            r["counts"][category] = 1.into();
            r["outcomes"][category] = r["outcomes"]["passed"].clone();
            r["outcomes"]["passed"] = serde_json::json!([]);
            r["execution_status"] = execution.into();
            r["conformance_status"] = conformance.into();
        });
        let adapted = adapt_json_v2(&variant, &suite).unwrap();
        let aep_domain::Evidence::EssConformanceV2(sources) = adapted.evidence() else {
            panic!("v2");
        };
        assert_eq!(
            sources
                .reading()
                .unwrap()
                .data()
                .counts
                .categories()
                .into_iter()
                .find(|(name, _)| *name == category)
                .unwrap()
                .1,
            1
        );
        let wrong_profile = if profile.starts_with("rust") {
            "go-scenario-status/1"
        } else {
            "rust-scenario-status/1"
        };
        if category != "failed" {
            refuses(
                &changed(&variant, |r| r["producer_profile"] = wrong_profile.into()),
                &suite,
                "ProfileOutcomeMismatch",
            );
        }
    }
}

#[test]
fn source_readback_cannot_restore_or_retain_a_failed_admission() {
    #[derive(Debug)]
    struct Refusing;
    impl EssConformanceV2Reader for Refusing {
        fn read(
            &self,
            _: &EssConformanceV2Sources,
        ) -> Result<
            aep_domain::ess_conformance_v2::EssConformanceV2Reading,
            aep_domain::ess_conformance_v2::EssAdmissionError,
        > {
            Err(aep_domain::ess_conformance_v2::EssAdmissionError::new(
                "ReaderRefusal",
                "$",
                "deliberate reader failure",
            ))
        }
    }
    let (report, suite) = pair(4);
    let mut sources = EssConformanceV2Sources::new(report, suite);
    sources.admit(&CountStageReader).unwrap();
    let wire = serde_json::to_string(&sources).unwrap();
    let raw: EssConformanceV2Sources = serde_json::from_str(&wire).unwrap();
    assert!(raw.reading().is_none());
    for key in ["verified", "admitted", "reading", "counts"] {
        let bad = wire.replacen('{', &format!("{{\"{key}\":true,"), 1);
        let error = serde_json::from_str::<EssConformanceV2Sources>(&bad).expect_err(key);
        assert!(error.to_string().contains("unknown field"), "{error}");
    }
    assert_eq!(
        sources.admit(&Refusing).unwrap_err().issues[0].reason,
        "ReaderRefusal"
    );
    assert!(sources.reading().is_none());
    sources.admit(&CountStageReader).unwrap();
    assert!(sources.reading().is_some());
    let replacement =
        EssConformanceV2Sources::new(sources.report_json().into(), "different suite".into());
    assert!(replacement.reading().is_none());
}

fn with_suite(report: &str, suite: &serde_json::Value) -> (String, String) {
    let suite = suite.to_string();
    let digest =
        Sha256::digest(suite.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    (
        changed(report, |r| r["suite"]["digest"] = digest.into()),
        suite,
    )
}

#[test]
fn frozen_predicate_depth_and_quantified_metadata_are_admitted_without_rewriting_bytes() {
    let (report, suite) = pair(4);
    let mut suite: serde_json::Value = serde_json::from_str(&suite).unwrap();
    let mut predicate = serde_json::json!("row.total > 0");
    for _ in 0..32 {
        predicate = serde_json::json!({"all":[predicate]});
    }
    suite["scenarios"]["demo.core/authored/one"]["steps"] = serde_json::json!([{"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"satisfies","predicate":predicate}}]);
    let (report, original) = with_suite(&report, &suite);
    assert!(
        adapt_json_v2(&report, &original).is_ok(),
        "ESS admits its exact32-level predicate boundary"
    );
    suite["scenarios"]["demo.core/authored/one"]["steps"][0]["expectation"]["predicate"] = serde_json::json!({"forall":{"in":"rows","as":"row","that":{"any":["row.total > 0",{"exists":{"in":"row.items","as":"item","that":"item.ready"}}]}}});
    let (report, original) = with_suite(&report, &suite);
    assert!(adapt_json_v2(&report, &original).is_ok());
    suite["scenarios"]["demo.core/authored/one"]["steps"][0]["expectation"]["predicate"]
        ["forall"]["unchecked"] = true.into();
    let (report, original) = with_suite(&report, &suite);
    refuses(&report, &original, "InvalidPredicate");
}

#[test]
fn complete_frozen_step_shape_view_and_dependency_vocabulary_is_read_for_every_legacy_major() {
    let steps = serde_json::json!([
        {"step":"configure_external_outcome","force":{"command":"demo.core.Read","outcome":"ready"}},
        {"step":"execute_command","command":"demo.core.Read","actor":null,"input":{"literal":{"kind":"literal","value":{"unexpected":1.5}},"bound":{"kind":"instance","instance":"one"},"observed":{"kind":"observed","event":"demo.core.Observed","field":"a.b"}}},
        {"step":"expect_outcome","outcome":{"command":"demo.core.Read","outcome":"ready"}},
        {"step":"expect_error","error":"demo.core.Missing","fields":{"arbitrary":[null,false]}},
        {"step":"expect_event","event":"demo.core.Observed","shape":{"a":{"holds":"primitive","kind":"decimal"},"b":{"holds":"enum","variants":["one","two"],"optional":true},"c":{"holds":"list"},"d":{"holds":"map"},"e":{"holds":"union"}}},
        {"step":"expect_no_event","event":"demo.core.Observed"},
        {"step":"capture_instance","instance":"one","entity":"demo.core.Item","event":"demo.core.Observed","field":"id"},
        {"step":"redeliver_event","event":"demo.core.Observed"},
        {"step":"expect_invocation","binding":"on-read","command":"demo.core.Read","input":{}},
        {"step":"query_view","view":"demo.core.Rows","params":{}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"contains","fields":{"id":{"kind":"instance","instance":"one"}}}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"excludes","fields":{}}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"satisfies","predicate":{"row.total":{"gte":0}}}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"counts","at_least":null,"at_most":3}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"ranked","order_by":["time","name asc","total descending"]}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"at","order_by":[],"position":{"row":"first"},"fields":{}}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"at","order_by":[],"position":{"row":"last"}}},
        {"step":"expect_view","view":"demo.core.Rows","expectation":{"expect":"at","order_by":[],"position":{"row":"nth","index":0}}},
        {"step":"eventually_event","event":"demo.core.Observed","payload":{},"shape":{}},
        {"step":"eventually_view","view":"demo.core.Rows","expectation":{"expect":"counts"}},
        {"step":"mark_instant","instant":"started"},
        {"step":"expect_not_before","instant":"started","elapsed":0},
        {"step":"expect_within","instant":"started","elapsed":4_294_967_295_u64},
        {"step":"expect_quiet","event":"demo.core.Observed","instant":"started","elapsed":1},
        {"step":"expect_halt","view":"demo.core.Rows","params":{},"after":0},
        {"step":"eventually_halt","view":"demo.core.Rows","after":1}
    ]);
    let refs = serde_json::json!([
        {"kind":"domain","name":"demo.core"},{"kind":"type","name":"demo.core.Value"},
        {"kind":"entity","name":"demo.core.Item"},{"kind":"command","name":"demo.core.Read"},
        {"kind":"outcome","name":{"command":"demo.core.Read","outcome":"ready"}},
        {"kind":"event","name":"demo.core.Observed"},{"kind":"error","name":"demo.core.Missing"},
        {"kind":"view","name":"demo.core.Rows"},{"kind":"actor","name":"demo.core.Reader"},
        {"kind":"transition","name":{"entity":"demo.core.Item","transition":"move"}},
        {"kind":"binding","name":"on-read"},{"kind":"component","name":"reader"}
    ]);
    for version in 1..=4 {
        let (report, suite) = pair(version);
        let mut suite: serde_json::Value = serde_json::from_str(&suite).unwrap();
        suite["scenarios"]["demo.core/authored/one"]["steps"] = steps.clone();
        suite["scenarios"]["demo.core/authored/one"]["source"] = refs.clone();
        // Frozen legacy SpecDigest permits shorter contract digests; report/2 model stays full64.
        suite["provenance"]["contract_digest"] = "1234567890abcdef".into();
        let (report, original) = with_suite(&report, &suite);
        adapt_json_v2(&report, &original).unwrap();
        suite["scenarios"]["demo.core/authored/one"]["steps"][4]["shape"]["a"]["future"] =
            true.into();
        let (bad_report, bad_suite) = with_suite(&report, &suite);
        refuses(&bad_report, &bad_suite, "UnknownField");
        suite["scenarios"]["demo.core/authored/one"]["steps"][4]["shape"]["a"]
            .as_object_mut()
            .unwrap()
            .remove("future");
        suite["scenarios"]["demo.core/authored/one"]["steps"][25]["after"] = "tomorrow".into();
        let (bad_report, bad_suite) = with_suite(&report, &suite);
        refuses(&bad_report, &bad_suite, "UnsignedIntegerRequired");
    }
}

#[test]
fn zero_and_mixed_terminal_selections_keep_each_profile_aggregate_truthful() {
    for (profile, buckets, execution, conformance) in [
        (
            "rust-scenario-status/1",
            [vec![], vec![], vec![], vec![], vec![]],
            "passed",
            "inconclusive",
        ),
        (
            "rust-scenario-status/1",
            [vec![], vec![], vec!["error"], vec!["unsupported"], vec![]],
            "failed",
            "failed",
        ),
        (
            "go-scenario-status/1",
            [vec!["passed"], vec![], vec![], vec![], vec![]],
            "passed",
            "inconclusive",
        ),
        (
            "go-scenario-status/1",
            [vec![], vec!["failed"], vec![], vec![], vec!["skipped"]],
            "failed",
            "failed",
        ),
    ] {
        let (report, suite) = pair(4);
        let mut report: serde_json::Value = serde_json::from_str(&report).unwrap();
        let mut suite: serde_json::Value = serde_json::from_str(&suite).unwrap();
        let scenario = suite["scenarios"]["demo.core/authored/one"].clone();
        suite["scenarios"] = serde_json::json!({});
        let mut total = 0_u64;
        for (category, names) in ["passed", "failed", "error", "unsupported", "skipped"]
            .into_iter()
            .zip(buckets)
        {
            let ids: Vec<_> = names
                .iter()
                .map(|name| format!("demo.core/authored/{name}"))
                .collect();
            let count = u64::try_from(ids.len()).unwrap();
            total += count;
            report["counts"][category] = count.into();
            report["outcomes"][category] = serde_json::json!(ids);
            for id in ids {
                suite["scenarios"][id] = scenario.clone();
            }
        }
        report["counts"]["total"] = total.into();
        report["producer_profile"] = profile.into();
        report["execution_status"] = execution.into();
        report["conformance_status"] = conformance.into();
        let (report, suite) = with_suite(&report.to_string(), &suite);
        let admitted = adapt_json_v2(&report, &suite).unwrap();
        let aep_domain::Evidence::EssConformanceV2(sources) = admitted.evidence() else {
            panic!("v2");
        };
        assert_eq!(
            sources.reading().unwrap().data().execution_status.as_str(),
            execution
        );
        assert_eq!(
            sources
                .reading()
                .unwrap()
                .data()
                .conformance_status
                .as_str(),
            conformance
        );
        assert_eq!(sources.reading().unwrap().data().counts.total, total);
        assert!(sources
            .reading()
            .unwrap()
            .facts()
            .iter()
            .any(
                |(path, value)| path.to_string() == "ess_conformance_v2.coverage_known"
                    && *value == aep_domain::FactValue::bool(false)
            ));
    }
}
