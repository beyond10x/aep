//! Parent comparison follows inherited field owners without rewriting original sources.
#[path = "support/coverage.rs"]
#[allow(dead_code)]
mod fixtures;

use aep_ess_evidence::adapt_json_coverage;
use serde_json::{json, Value};

fn suite(step: Value) -> Value {
    let mut suite = fixtures::suite();
    suite["scenarios"][fixtures::SELECTED]["steps"] = Value::Array(vec![step]);
    suite
}

fn compare(parent: &Value, child: &Value) -> Result<(), String> {
    let mut selected = fixtures::child(parent, &[fixtures::SELECTED]);
    selected["scenarios"] = child["scenarios"].clone();
    selected["provenance"] = child["provenance"].clone();
    let original = parent.to_string();
    let (report, input) =
        fixtures::pair_for(&selected, &[fixtures::SELECTED], "passed", &[original]);
    adapt_json_coverage(&report, &input)
        .map(|admitted| {
            let wire = serde_json::to_value(admitted).unwrap();
            assert_eq!(wire["report_json"], report);
            assert_eq!(wire["suite_input_json"], input);
        })
        .map_err(|error| {
            error
                .issues
                .iter()
                .map(|issue| issue.reason)
                .collect::<Vec<_>>()
                .join(",")
        })
}

fn default_cases() -> Vec<(Value, &'static str, Value, Value)> {
    let literal = json!({"x":{"kind":"literal","value":null}});
    let mut cases = vec![
        (
            json!({"step":"execute_command","command":"demo.core.Read"}),
            "/actor",
            Value::Null,
            json!("demo.core.Reader"),
        ),
        (
            json!({"step":"execute_command","command":"demo.core.Read"}),
            "/input",
            json!({}),
            literal.clone(),
        ),
        (
            json!({"step":"expect_error","error":"demo.core.Refused"}),
            "/fields",
            json!({}),
            json!({"x":null}),
        ),
        (
            json!({"step":"expect_invocation","binding":"read","command":"demo.core.Read"}),
            "/input",
            json!({}),
            literal.clone(),
        ),
        (
            json!({"step":"query_view","view":"demo.core.Reads"}),
            "/params",
            json!({}),
            literal.clone(),
        ),
        (
            json!({"step":"eventually_view","view":"demo.core.Reads","expectation":{"expect":"counts"}}),
            "/params",
            json!({}),
            literal.clone(),
        ),
    ];
    for step in ["expect_event", "eventually_event"] {
        let event = json!({"step":step,"event":"demo.core.Ready"});
        cases.push((event.clone(), "/payload", json!({}), json!({"x":null})));
        cases.push((event, "/shape", json!({}), json!({"x":{"holds":"map"}})));
        cases.push((
            json!({"step":step,"event":"demo.core.Ready","shape":{"x":{"holds":"map"}}}),
            "/shape/x/optional",
            json!(false),
            json!(true),
        ));
    }
    for step in ["expect_halt", "eventually_halt"] {
        cases.push((
            json!({"step":step,"view":"demo.core.Reads","after":1}),
            "/params",
            json!({}),
            literal.clone(),
        ));
    }
    for step in ["expect_view", "eventually_view"] {
        for field in ["at_least", "at_most"] {
            cases.push((
                json!({"step":step,"view":"demo.core.Reads","expectation":{"expect":"counts"}}),
                if field == "at_least" {
                    "/expectation/at_least"
                } else {
                    "/expectation/at_most"
                },
                Value::Null,
                json!(0),
            ));
        }
        cases.push((json!({"step":step,"view":"demo.core.Reads","expectation":{"expect":"at","order_by":["id"],"position":{"row":"first"}}}), "/expectation/fields", json!({}), literal.clone()));
    }
    cases
}

fn insert(value: &mut Value, path: &str, field: Value) {
    let (parent, key) = path.rsplit_once('/').unwrap();
    value
        .pointer_mut(parent)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(key.into(), field);
}

#[test]
fn lineage_applies_every_inherited_optional_default_in_both_directions() {
    let mut failures = Vec::new();
    for (step, path, default, _) in default_cases() {
        let parent = suite(step.clone());
        let mut explicit = step;
        insert(&mut explicit, path, default);
        let child = suite(explicit);
        for (before, after) in [(&parent, &child), (&child, &parent)] {
            if let Err(error) = compare(before, after) {
                failures.push(format!(
                    "{} {path}: {error}",
                    before["scenarios"][fixtures::SELECTED]["steps"][0]["step"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn lineage_preserves_meaningful_changes_at_every_default_boundary() {
    for (step, path, _, changed) in default_cases() {
        let parent = suite(step.clone());
        let mut altered = step;
        insert(&mut altered, path, changed);
        let error = compare(&parent, &suite(altered)).expect_err(path);
        assert!(error.contains("ParentScenarioMismatch"), "{path}: {error}");
    }
}

fn payload_cases() -> Vec<(Value, &'static str)> {
    let mut cases = vec![
        (
            json!({"step":"execute_command","command":"demo.core.Read","input":{"x":{"kind":"literal","value":null}}}),
            "/input/x/value",
        ),
        (
            json!({"step":"expect_invocation","binding":"read","command":"demo.core.Read","input":{"x":{"kind":"literal","value":null}}}),
            "/input/x/value",
        ),
        (
            json!({"step":"expect_error","error":"demo.core.Refused","fields":{"x":null}}),
            "/fields/x",
        ),
    ];
    for step in ["expect_event", "eventually_event"] {
        cases.push((
            json!({"step":step,"event":"demo.core.Ready","payload":{"x":null}}),
            "/payload/x",
        ));
    }
    for step in [
        "query_view",
        "eventually_view",
        "expect_halt",
        "eventually_halt",
    ] {
        let mut value = json!({"step":step,"view":"demo.core.Reads","params":{"x":{"kind":"literal","value":null}}});
        if step == "eventually_view" {
            value["expectation"] = json!({"expect":"counts"});
        } else if step != "query_view" {
            value["after"] = json!(1);
        }
        cases.push((value, "/params/x/value"));
    }
    for step in ["expect_view", "eventually_view"] {
        for expectation in ["contains", "excludes", "at"] {
            let mut value = json!({"step":step,"view":"demo.core.Reads","expectation":{"expect":expectation,"fields":{"x":{"kind":"literal","value":null}}}});
            if expectation == "at" {
                value["expectation"]["order_by"] = json!(["id"]);
                value["expectation"]["position"] = json!({"row":"first"});
            }
            cases.push((value, "/expectation/fields/x/value"));
        }
    }
    cases
}

#[test]
fn lineage_uses_node_numbers_at_every_payload_owner_and_nested_container() {
    let pairs = [
        ("9007199254740993", "9007199254740992"),
        ("-9007199254740993", "-9007199254740992"),
        ("1", "1.0"),
        ("-0.0", "0"),
        ("1e-400", "0"),
    ];
    let mut failures = Vec::new();
    for (step, path) in payload_cases() {
        for (original, equivalent) in pairs {
            let a: aep_domain::Node = serde_json::from_str(original).unwrap();
            let b: aep_domain::Node = serde_json::from_str(equivalent).unwrap();
            assert_eq!(a, b, "inherited Number control");
            let mut before = step.clone();
            let mut after = step.clone();
            for (target, number) in [(&mut before, original), (&mut after, equivalent)] {
                let number: Value = serde_json::from_str(number).unwrap();
                *target.pointer_mut(path).unwrap() =
                    json!({"nested":[number,{"kept":null,"text":"9007199254740993"}]});
            }
            if let Err(error) = compare(&suite(before), &suite(after)) {
                failures.push(format!(
                    "{} {path} {original}/{equivalent}: {error}",
                    step["step"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn lineage_keeps_payload_map_membership_values_types_and_sequence_order() {
    for (step, path) in payload_cases() {
        for (before, after) in [
            (json!({}), json!({"x":null})),
            (json!({"x":null}), json!({"x":{}})),
            (json!([1, 2]), json!([2, 1])),
            (json!(1), json!(2)),
            (json!(1), json!("1")),
            (json!("9007199254740993"), json!("9007199254740992")),
        ] {
            let mut parent = step.clone();
            let mut child = step.clone();
            *parent.pointer_mut(path).unwrap() = before;
            *child.pointer_mut(path).unwrap() = after;
            let error = compare(&suite(parent), &suite(child)).expect_err(path);
            assert!(error.contains("ParentScenarioMismatch"), "{path}: {error}");
        }
    }
}

#[test]
fn lineage_predicate_operands_follow_their_parser_inside_quantifiers() {
    for step in ["expect_view", "eventually_view"] {
        for (before, after) in [
            (
                json!({"value":{"eq":9_007_199_254_740_993_u64}}),
                json!({"value":{"eq":9_007_199_254_740_992_u64}}),
            ),
            (
                json!("value == 9007199254740993"),
                json!("value == 9007199254740992"),
            ),
            (
                json!({"forall":{"in":"rows","as":"row","that":{"row.value":{"eq":9_007_199_254_740_993_u64}}}}),
                json!({"forall":{"in":"rows","as":"row","that":{"row.value":{"eq":9_007_199_254_740_992_u64}}}}),
            ),
        ] {
            let definition = |predicate| {
                suite(
                    json!({"step":step,"view":"demo.core.Reads","expectation":{"expect":"satisfies","predicate":predicate}}),
                )
            };
            compare(&definition(before), &definition(after))
                .expect("equal typed predicate operands");
        }
    }
}

#[test]
fn lineage_keeps_integer_metadata_exact_at_every_numeric_owner() {
    for (step, path) in [
        (
            json!({"step":"expect_halt","view":"demo.core.Reads","after":0}),
            "/after",
        ),
        (
            json!({"step":"eventually_halt","view":"demo.core.Reads","after":0}),
            "/after",
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"counts","at_least":0}}),
            "/expectation/at_least",
        ),
        (
            json!({"step":"eventually_view","view":"demo.core.Reads","expectation":{"expect":"counts","at_most":0}}),
            "/expectation/at_most",
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"at","order_by":["id"],"position":{"row":"nth","index":0}}}),
            "/expectation/position/index",
        ),
    ] {
        for (a, b) in [
            (9_007_199_254_740_993_u64, 9_007_199_254_740_992_u64),
            (u64::MAX, u64::MAX - 1),
        ] {
            let mut parent = step.clone();
            let mut child = step.clone();
            *parent.pointer_mut(path).unwrap() = a.into();
            *child.pointer_mut(path).unwrap() = b.into();
            let error = compare(&suite(parent), &suite(child)).expect_err(path);
            assert!(error.contains("ParentScenarioMismatch"), "{path}: {error}");
        }
    }
}

#[test]
fn lineage_preserves_elapsed_metadata_and_nonliteral_scenario_values() {
    for step in ["expect_not_before", "expect_within", "expect_quiet"] {
        let mut original = json!({"step":step,"instant":"start","elapsed":1});
        if step == "expect_quiet" {
            original["event"] = json!("demo.core.Ready");
        }
        let parent = suite(original.clone());
        original["elapsed"] = json!(2);
        assert_eq!(
            compare(&parent, &suite(original.clone())).unwrap_err(),
            "ParentScenarioMismatch"
        );
        original["elapsed"] = json!(1.0);
        assert_eq!(
            compare(&parent, &suite(original)).unwrap_err(),
            "UnsignedIntegerRequired"
        );
    }
    for (before, after) in [
        (
            json!({"kind":"instance","instance":"first"}),
            json!({"kind":"instance","instance":"second"}),
        ),
        (
            json!({"kind":"observed","event":"demo.core.Ready","field":"9007199254740993"}),
            json!({"kind":"observed","event":"demo.core.Ready","field":"9007199254740992"}),
        ),
    ] {
        let definition = |value| {
            suite(json!({"step":"execute_command","command":"demo.core.Read","input":{"x":value}}))
        };
        assert_eq!(
            compare(&definition(before), &definition(after)).unwrap_err(),
            "ParentScenarioMismatch"
        );
    }
}

#[test]
fn lineage_preserves_predicate_types_order_operators_and_quantifier_identity() {
    for (before, after) in [
        (json!({"value":{"eq":1}}), json!({"value":{"eq":2}})),
        (json!({"value":{"eq":1}}), json!({"value":{"eq":"'1'"}})),
        (
            json!({"value":{"eq":"'9007199254740993'"}}),
            json!({"value":{"eq":"'9007199254740992'"}}),
        ),
        (json!({"value":{"eq":1}}), json!({"value":{"ne":1}})),
        (json!({"all":["a","b"]}), json!({"all":["b","a"]})),
        (
            json!({"forall":{"in":"rows","as":"row","that":true}}),
            json!({"exists":{"in":"rows","as":"row","that":true}}),
        ),
        (
            json!({"forall":{"in":"rows","as":"row","that":true}}),
            json!({"forall":{"in":"others","as":"row","that":true}}),
        ),
        (
            json!({"forall":{"in":"rows","as":"row","that":true}}),
            json!({"forall":{"in":"rows","as":"item","that":true}}),
        ),
    ] {
        let definition = |predicate| {
            suite(
                json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"satisfies","predicate":predicate}}),
            )
        };
        assert_eq!(
            compare(&definition(before), &definition(after)).unwrap_err(),
            "ParentScenarioMismatch"
        );
    }
}

#[test]
fn lineage_checks_unknown_fields_and_both_original_byte_references_before_comparison() {
    let parent = suite(json!({"step":"execute_command","command":"demo.core.Read"}));
    let mut child = parent.clone();
    child["scenarios"][fixtures::SELECTED]["steps"][0]["unknown"] = Value::Null;
    assert!(compare(&parent, &child)
        .unwrap_err()
        .contains("UnknownField"));
    let selected = fixtures::child(&parent, &[fixtures::SELECTED]);
    let (report, input) = fixtures::pair_for(
        &selected,
        &[fixtures::SELECTED],
        "passed",
        &[parent.to_string()],
    );
    for field in ["suite_json", "parent_suites"] {
        let mut changed: Value = serde_json::from_str(&input).unwrap();
        let original = if field == "suite_json" {
            &mut changed[field]
        } else {
            &mut changed[field][0]
        };
        *original = format!("{}\n", original.as_str().unwrap()).into();
        let error = adapt_json_coverage(&report, &changed.to_string()).expect_err(field);
        let expected = if field == "suite_json" {
            "SuiteDigestMismatch"
        } else {
            "ParentReferenceMismatch"
        };
        assert!(
            error.issues.iter().any(|issue| issue.reason == expected),
            "{field}: {error}"
        );
    }
}
