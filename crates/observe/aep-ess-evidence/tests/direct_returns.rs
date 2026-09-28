//! Independently authored new-version fixtures; these are not ER execution evidence.
use aep_ess_evidence::{adapt_json_coverage, adapt_json_v2, wrap_coverage_suite};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const MODEL: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
const ID: &str = "demo.api.Read/outcome/returned";

fn suite(version: u8) -> Value {
    let mut suite = json!({
        "provenance":{"suite_version":format!("ess-conformance/{version}"),"system":"demo","specification_version":"v1","spec_digest":MODEL,"contract_digest":MODEL},
        "scenarios":{ID:{"purpose":"Observe the actual typed library return","source":[{"kind":"command","name":"demo.api.Read"}],"steps":[
            {"step":"execute_command","command":"demo.api.Read","input":{"n":{"kind":"literal","value":9_007_199_254_740_993_u64}}},
            {"step":"expect_outcome","outcome":{"command":"demo.api.Read","outcome":"returned"}},
            {"step":"expect_direct_response","response":{"command":"demo.api.Read","outcome":{"command":"demo.api.Read","outcome":"returned"},"fields":[{"name":"value","type":"demo.api.Document"},{"name":"ok","type":"Boolean"},{"name":"revision","type":"Integer"}],"declarations":{"demo.api.Document":{"kind":"newtype","of":"String"}},"expected":{"value":"{\"number\":9007199254740993}","ok":true,"revision":18_446_744_073_709_551_615_u64}}}
        ]}},
        "coverage":{"selection":{"scope":{"kind":"system"},"origins":"generated_and_authored","filter":{"kind":"all"}},"knowledge":"complete_inventory","generated":[ID],"authored":[],"outside":[],"refused":[],"authored_sources":{},"counts":{"generated":1,"authored":0,"outside":0,"refused":0}}
    });
    if version == 28 {
        suite.as_object_mut().unwrap().remove("coverage");
    }
    suite
}

fn reference(version: &str, original: &str) -> Value {
    let digest =
        Sha256::digest(original.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut out, byte| {
                write!(out, "{byte:02x}").unwrap();
                out
            });
    json!({"version":version,"digest_profile":"sha256-json-bytes/1","digest":digest})
}

fn pair(suite: &Value, parents: &[String]) -> (String, String) {
    let original = suite.to_string();
    let coverage = if suite.get("coverage").is_some() {
        let mut result = suite["coverage"].clone();
        for key in ["generated", "authored", "outside", "authored_sources"] {
            result.as_object_mut().unwrap().remove(key);
        }
        result
    } else {
        json!({"knowledge":"unknown"})
    };
    let report = json!({"format":"ess-conformance-report/2","specification":"demo/v1","spec_digest":MODEL,"implementation":"independently authored reader fixture","producer_profile":"rust-scenario-status/1","suite":reference(suite["provenance"]["suite_version"].as_str().unwrap(),&original),"execution_status":"passed","counts":{"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0},"outcomes":{"passed":[ID],"failed":[],"error":[],"unsupported":[],"skipped":[]},"coverage":coverage,"conformance_status":if suite.get("coverage").is_some(){"passed"}else{"inconclusive"},"policy":"complete-selection/1","completed_at":9_007_199_254_740_993_u64});
    let input =
        json!({"format":"ess-conformance-input/1","suite_json":original,"parent_suites":parents});
    (report.to_string(), input.to_string())
}

fn refuses(suite: &Value, reason: &str) {
    let (report, input) = pair(suite, &[]);
    let error = adapt_json_coverage(&report, &input).expect_err(reason);
    assert!(
        error.issues.iter().any(|issue| issue.reason == reason),
        "expected {reason}, got {error}"
    );
}

#[test]
fn direct_return_inventory_preserves_original_bytes_counts_and_exact_integers() {
    let suite = suite(29);
    let (report, input) = pair(&suite, &[]);
    let adapted = adapt_json_coverage(&report, &input).unwrap();
    let aep_domain::Evidence::EssConformanceCoverageV1(sources) = adapted.evidence() else {
        panic!("coverage evidence")
    };
    assert_eq!(sources.report_json(), report);
    assert_eq!(sources.suite_input_json(), input);
    assert_eq!(sources.reading().unwrap().data().counts.passed, 1);
    assert_eq!(
        sources.reading().unwrap().data().suite.version(),
        "ess-conformance/29"
    );
    assert_eq!(
        adapted.observed_at().timestamp().epoch_millis(),
        9_007_199_254_740_993
    );
    assert_eq!(wrap_coverage_suite(&suite.to_string()).unwrap(), input);
}

#[test]
fn direct_return_ordinary_suite_remains_nonqualifying_count_evidence() {
    let suite = suite(28);
    let (report, _) = pair(&suite, &[]);
    let adapted = adapt_json_v2(&report, &suite.to_string()).unwrap();
    let aep_domain::Evidence::EssConformanceV2(sources) = adapted.evidence() else {
        panic!("count evidence")
    };
    assert_eq!(
        sources.reading().unwrap().data().suite.version(),
        "ess-conformance/28"
    );
    assert_eq!(
        sources
            .reading()
            .unwrap()
            .data()
            .conformance_status
            .as_str(),
        "inconclusive"
    );
}

#[test]
fn direct_return_reader_keeps_unknown_fields_and_variants_closed() {
    for pointer in ["/scenarios/demo.api.Read~1outcome~1returned/steps/2/response","/scenarios/demo.api.Read~1outcome~1returned/steps/2/response/fields/0","/scenarios/demo.api.Read~1outcome~1returned/steps/2/response/declarations/demo.api.Document"] {
        let mut candidate=suite(29); candidate.pointer_mut(pointer).unwrap()["future"]=true.into(); refuses(&candidate,"UnknownField");
    }
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Document"]
        ["kind"] = "future".into();
    refuses(&candidate, "UnsupportedResponseDeclaration");
}

#[test]
fn direct_return_type_profile_is_explicitly_narrow() {
    for ty in [
        "Binary64",
        "Decimal",
        "Json",
        "List<String>",
        "Optional<Integer>",
    ] {
        let mut candidate = suite(29);
        candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Document"]
            ["of"] = ty.into();
        refuses(&candidate, "UnsupportedResponseType");
    }
}

#[test]
fn er_optional_text_and_text_lists_keep_null_and_ordered_duplicate_literals() {
    for (ty, value) in [
        ("Optional<String>", json!(null)),
        ("Optional<String>", json!("9007199254740993")),
        ("Optional<List<String>>", json!(null)),
        ("Optional<List<String>>", json!([])),
        ("Optional<List<String>>", json!(["later", "first", "first"])),
    ] {
        let mut candidate = suite(29);
        candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Document"]
            ["of"] = ty.into();
        candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["value"] = value;
        let (report, input) = pair(&candidate, &[]);
        adapt_json_coverage(&report, &input).unwrap();
    }
    for (ty, value) in [
        ("Optional<String>", json!(3)),
        ("Optional<List<String>>", json!("text")),
        ("Optional<List<String>>", json!(["text", 3])),
        ("Optional<List<String>>", json!([null])),
    ] {
        let mut candidate = suite(29);
        candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Document"]
            ["of"] = ty.into();
        candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["value"] = value;
        refuses(&candidate, "InvalidResponseValue");
    }
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Document"]["of"] =
        "Optional<List<String>>".into();
    candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["value"] =
        json!(vec![""; 65_537]);
    refuses(&candidate, "ResponseResourceLimit");
}

#[test]
fn direct_return_expected_values_must_match_the_complete_authority() {
    for (key, value) in [
        ("value", json!(3)),
        ("ok", json!("true")),
        ("revision", json!("18446744073709551615")),
    ] {
        let mut candidate = suite(29);
        candidate["scenarios"][ID]["steps"][2]["response"]["expected"][key] = value;
        refuses(&candidate, "InvalidResponseValue");
    }
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["unknown"] = true.into();
    refuses(&candidate, "UnknownResponseField");
}

#[test]
fn direct_return_requires_the_preceding_invocation_and_matching_outcome() {
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["command"] = "demo.api.Other".into();
    refuses(&candidate, "ResponseCommandMismatch");
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["outcome"]["command"] =
        "demo.api.Other".into();
    refuses(&candidate, "ResponseCommandMismatch");
}

#[test]
fn direct_return_pairing_and_report_integrity_remain_required() {
    let original = suite(29);
    let (report, input) = pair(&original, &[]);
    for (key, field, value, reason) in [
        (
            "suite",
            "digest",
            json!(format!("sha256:{}", "0".repeat(64))),
            "SuiteDigestMismatch",
        ),
        ("counts", "total", json!(2), "TotalMismatch"),
    ] {
        let mut changed: Value = serde_json::from_str(&report).unwrap();
        changed[key][field] = value;
        let error = adapt_json_coverage(&changed.to_string(), &input).unwrap_err();
        assert!(
            error.issues.iter().any(|issue| issue.reason == reason),
            "{error}"
        );
    }
    let mut changed: Value = serde_json::from_str(&report).unwrap();
    changed["spec_digest"] = "a".repeat(64).into();
    assert_eq!(
        adapt_json_coverage(&changed.to_string(), &input)
            .unwrap_err()
            .issues[0]
            .reason,
        "ModelDigestMismatch"
    );
}

#[test]
fn direct_return_inventory_is_not_inferred_from_passing_counts() {
    let mut candidate = suite(29);
    candidate["coverage"]["counts"]["generated"] = 2.into();
    refuses(&candidate, "CoverageCountMismatch");
    let mut candidate = suite(29);
    candidate["scenarios"].as_object_mut().unwrap().clear();
    refuses(&candidate, "CoverageCountMismatch");
    let mut candidate = suite(29);
    candidate["scenarios"]["demo.api.Other/outcome/returned"] = candidate["scenarios"][ID].clone();
    refuses(&candidate, "CoverageCountMismatch");
    candidate["scenarios"].as_object_mut().unwrap().remove(ID);
    refuses(&candidate, "SelectedIdsMismatch");
}

#[test]
fn direct_return_declarations_are_exact_reachable_and_finite() {
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Document"]["of"] =
        "demo.api.Document".into();
    refuses(&candidate, "ResponseTypeCycle");
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]
        .as_object_mut()
        .unwrap()
        .clear();
    refuses(&candidate, "MissingResponseDeclaration");
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["declarations"]["demo.api.Unused"] =
        json!({"kind":"newtype","of":"String"});
    refuses(&candidate, "UnusedResponseDeclaration");
    let mut candidate = suite(29);
    let field = candidate["scenarios"][ID]["steps"][2]["response"]["fields"][0].clone();
    candidate["scenarios"][ID]["steps"][2]["response"]["fields"]
        .as_array_mut()
        .unwrap()
        .push(field);
    refuses(&candidate, "DuplicateResponseField");
}

#[test]
fn direct_return_literals_are_bounded_without_reusing_legacy_float_conversion() {
    let mut candidate = suite(29);
    candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["value"] =
        "x".repeat(4097).into();
    let (report, input) = pair(&candidate, &[]);
    adapt_json_coverage(&report, &input).unwrap();
    candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["revision"] = i64::MIN.into();
    let (report, input) = pair(&candidate, &[]);
    adapt_json_coverage(&report, &input).unwrap();
    candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["revision"] = json!(1.5);
    refuses(&candidate, "InvalidResponseValue");
    candidate["scenarios"][ID]["steps"][2]["response"]["expected"]["value"] =
        "x".repeat(1_048_576).into();
    refuses(&candidate, "ResponseResourceLimit");
}

#[test]
fn direct_return_parent_comparison_does_not_round_integer_literals() {
    let parent = suite(29);
    let original = parent.to_string();
    let mut child = parent.clone();
    child["coverage"]["selection"]["filter"] =
        json!({"kind":"explicit","ids":[ID],"parent":reference("ess-conformance/29",&original)});
    let (report, input) = pair(&child, std::slice::from_ref(&original));
    adapt_json_coverage(&report, &input).unwrap();
    child["scenarios"][ID]["steps"][0]["input"]["n"]["value"] = 9_007_199_254_740_992_u64.into();
    let (report, input) = pair(&child, &[original]);
    assert_eq!(
        adapt_json_coverage(&report, &input).unwrap_err().issues[0].reason,
        "ParentScenarioMismatch"
    );
}

#[test]
fn direct_return_extension_does_not_reinterpret_allocated_intermediate_versions() {
    for version in [6, 25, 26, 27, 30] {
        refuses(&suite(version), "UnsupportedSuiteVersion");
    }
}

#[test]
fn direct_return_step_is_not_admitted_under_legacy_suite_versions() {
    refuses(&suite(5), "UnsupportedStep");
    let mut ordinary = suite(28);
    ordinary["provenance"]["suite_version"] = "ess-conformance/4".into();
    let (report, _) = pair(&ordinary, &[]);
    assert_eq!(
        adapt_json_v2(&report, &ordinary.to_string())
            .unwrap_err()
            .issues[0]
            .reason,
        "UnsupportedStep"
    );
}

#[test]
fn genuine_scalar_producer_suites_are_read_without_rewriting_their_bytes() {
    for (original, version) in [
        (
            include_str!("fixtures/direct_returns/direct-suite28.json"),
            "ess-conformance/28",
        ),
        (
            include_str!("fixtures/direct_returns/direct-suite29.json"),
            "ess-conformance/29",
        ),
    ] {
        let suite: Value = serde_json::from_str(original).unwrap();
        let (report, _) = pair(&suite, &[]);
        let mut report: Value = serde_json::from_str(&report).unwrap();
        report["suite"] = reference(version, original);
        report["spec_digest"] = suite["provenance"]["spec_digest"].clone();
        report["specification"] = "library/v1".into();
        report["counts"]["total"] = 2.into();
        report["counts"]["passed"] = 2.into();
        report["outcomes"]["passed"] = json!([
            "library.api.Read/outcome/returned",
            "library.api/authored/pure-return"
        ]);
        if version == "ess-conformance/29" {
            let input = json!({"format":"ess-conformance-input/1", "suite_json":original, "parent_suites":[]}).to_string();
            adapt_json_coverage(&report.to_string(), &input).unwrap();
        } else {
            adapt_json_v2(&report.to_string(), original).unwrap();
        }
    }
}
