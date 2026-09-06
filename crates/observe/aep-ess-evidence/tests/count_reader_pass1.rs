//! First bounded reader review: lexical JSON and complete legacy metadata.
#[path = "support/count_reader_pass1.rs"]
mod support;

use aep_ess_evidence::adapt_json_v2;
use serde_json::{json, Value};

fn refuses(report: &str, suite: &str, reason: &str, path: &str) {
    let error = adapt_json_v2(report, suite).expect_err(reason);
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.reason == reason && issue.path == path),
        "expected {reason} at {path}; got {error}"
    );
}

fn with_steps(steps: Value) -> String {
    let mut suite: Value = serde_json::from_str(&support::suite()).unwrap();
    suite["scenarios"][support::SELECTED]["steps"] = steps;
    suite.to_string()
}

#[test]
fn decoded_duplicate_keys_refuse_in_report_and_arbitrary_nested_payload() {
    let suite = with_steps(
        json!([{"step":"expect_event", "event":"review.data.Observed",
        "payload":{"outer":{"café":1,"ready":true}}}]),
    );
    let report = support::report(&suite, 1);
    let bad_report = report.replace("\"total\":1", r#""total":1,"\u0074otal":1"#);
    assert_ne!(bad_report, report);
    refuses(&bad_report, &suite, "DuplicateKey", "$.counts");

    for (original, duplicate) in [
        ("\"ready\":true", r#""ready":true,"\u0072eady":false"#),
        ("\"café\":1", r#""café":1,"caf\u00e9":2"#),
    ] {
        let bad_suite = suite.replace(original, duplicate);
        assert_ne!(bad_suite, suite);
        refuses(
            &support::report(&bad_suite, 1),
            &bad_suite,
            "DuplicateKey",
            "$suite.scenarios.review.data/authored/one.steps[0].payload.outer",
        );
    }
    let escaped = suite.replace("\"café\":1", r#""caf\u00e9":1"#);
    assert!(adapt_json_v2(&support::report(&escaped, 1), &escaped).is_ok());
}

#[test]
fn count_tokens_remain_strict_while_legacy_payload_numbers_keep_their_grammar() {
    let suite = with_steps(
        json!([{"step":"expect_event", "event":"review.data.Observed",
        "payload":{"number":0}}]),
    );
    for token in ["-0", "0.0", "0e0", "1.5", "1e0"] {
        let suite = suite.replace("\"number\":0", &format!("\"number\":{token}"));
        assert!(
            adapt_json_v2(&support::report(&suite, 1), &suite).is_ok(),
            "payload {token}"
        );
    }
    let report = support::report(&suite, 1);
    for (field, original) in [
        ("total", 1),
        ("passed", 1),
        ("failed", 0),
        ("error", 0),
        ("unsupported", 0),
        ("skipped", 0),
    ] {
        for token in ["-0", "0.0", "0e0", "\"0\""] {
            let changed = report.replace(
                &format!("\"{field}\":{original}"),
                &format!("\"{field}\":{token}"),
            );
            assert_ne!(changed, report);
            refuses(
                &changed,
                &suite,
                "UnsignedIntegerRequired",
                &format!("$.counts.{field}"),
            );
        }
        let overflow = report.replace(
            &format!("\"{field}\":{original}"),
            &format!("\"{field}\":18446744073709551616"),
        );
        refuses(
            &overflow,
            &suite,
            "UnsignedIntegerOverflow",
            &format!("$.counts.{field}"),
        );
    }
}

#[test]
fn structural_and_textual_predicate_depth_share_the_frozen_limit() {
    for (maps, words, valid) in [
        (0, 32, true),
        (16, 16, true),
        (31, 1, true),
        (16, 17, false),
        (31, 2, false),
    ] {
        let mut predicate = Value::String(format!("{}row.ready", "not ".repeat(words)));
        for depth in 0..maps {
            predicate = if depth % 2 == 0 {
                json!({"not":predicate})
            } else {
                json!({"all":[predicate]})
            };
        }
        let suite = with_steps(json!([{"step":"expect_view", "view":"review.data.Rows",
            "expectation":{"expect":"satisfies", "predicate":predicate}}]));
        let report = support::report(&suite, 1);
        if valid {
            assert!(
                adapt_json_v2(&report, &suite).is_ok(),
                "{maps} maps + {words} prefixes"
            );
        } else {
            refuses(
                &report,
                &suite,
                "InvalidPredicate",
                "$suite.scenarios.review.data/authored/one.steps[0].expectation.predicate",
            );
        }
    }
}

#[test]
fn closed_scenario_values_and_shapes_do_not_close_literal_payload_keys() {
    let steps = json!([
        {"step":"execute_command", "command":"review.data.Do", "input":{
            "body":{"kind":"literal","value":{"optional":"payload", "verified":true}},
            "owner":{"kind":"observed","event":"review.data.Seen","field":""}}},
        {"step":"expect_event", "event":"review.data.Seen", "shape":{
            "arbitrary field key":{"holds":"list","optional":true},
            "text":{"holds":"primitive","kind":"string"}}}
    ]);
    let suite = with_steps(steps.clone());
    assert!(adapt_json_v2(&support::report(&suite, 1), &suite).is_ok());
    for (step, field, path) in [
        (
            0,
            "input",
            "$suite.scenarios.review.data/authored/one.steps[0].input.body.verified",
        ),
        (
            1,
            "shape",
            "$suite.scenarios.review.data/authored/one.steps[1].shape.arbitrary field key.verified",
        ),
    ] {
        let mut changed = steps.clone();
        if field == "input" {
            changed[step][field]["body"]["verified"] = true.into();
        } else {
            changed[step][field]["arbitrary field key"]["verified"] = true.into();
        }
        let suite = with_steps(changed);
        refuses(&support::report(&suite, 1), &suite, "UnknownField", path);
    }
}
