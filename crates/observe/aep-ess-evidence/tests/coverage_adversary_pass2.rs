//! Second bounded review: complete admitted definitions survive their private comparison view.
#[path = "support/coverage.rs"]
#[allow(dead_code)]
mod fixtures;

use aep_ess_evidence::adapt_json_coverage;
use serde_json::{json, Value};

fn suite(predicate: Value) -> Value {
    let mut suite = fixtures::suite();
    suite["scenarios"][fixtures::SELECTED]["steps"] = json!([{
        "step":"expect_view", "view":"demo.core.Reads",
        "expectation":{"expect":"satisfies", "predicate":null}
    }]);
    suite["scenarios"][fixtures::SELECTED]["steps"][0]["expectation"]["predicate"] = predicate;
    suite
}

fn paired(parent: &Value, changed: &Value) -> (String, String) {
    let mut child = fixtures::child(parent, &[fixtures::SELECTED]);
    child["scenarios"] = changed["scenarios"].clone();
    fixtures::pair_for(
        &child,
        &[fixtures::SELECTED],
        "passed",
        &[parent.to_string()],
    )
}

#[test]
fn comparison_cannot_drop_predicate_children_or_quantified_conditions() {
    for (before, after) in [
        (
            json!({"ready":true,"value":{"eq":3}}),
            json!({"ready":true}),
        ),
        (
            json!({"all":["ready","value == 3"]}),
            json!({"all":["ready"]}),
        ),
        (
            json!({"any":["ready","value == 3"]}),
            json!({"any":["ready"]}),
        ),
        (
            json!({"none":["ready","value == 3"]}),
            json!({"none":["ready"]}),
        ),
        (
            json!({"forall":{"in":"rows","as":"row","that":{"all":[{"row.ready":true},{"row.value":{"gt":0}}]}}}),
            json!({"forall":{"in":"rows","as":"row","that":{"all":[{"row.ready":true}]}}}),
        ),
        (
            json!({"not":{"exists":{"in":"rows","as":"row","that":{"row.ready":true}}}}),
            json!({"exists":{"in":"rows","as":"row","that":{"row.ready":true}}}),
        ),
    ] {
        let parent = suite(before.clone());
        let (report, input) = fixtures::pair_for(&parent, &[fixtures::SELECTED], "passed", &[]);
        adapt_json_coverage(&report, &input).expect("complete original predicate admits");
        let (report, input) = paired(&parent, &suite(after.clone()));
        let error = adapt_json_coverage(&report, &input)
            .expect_err("a changed admitted predicate must not disappear during comparison");
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.reason == "ParentScenarioMismatch"),
            "{before} => {after}: {error}"
        );
    }
}

#[test]
fn comparison_follows_inherited_scalar_parser_through_boolean_and_quantified_forms() {
    for predicate in [
        json!("not not ready"),
        json!({"none_of_these":["ready",{"value":{"ge":3}}]}),
        json!({"value":{"one_of":[1,"'literal'",true]}}),
        json!({"value":{"not_in":[1,2]}}),
        json!({"value":{"defined":false}}),
        json!({"and":["ready",{"value":{"gte":3,"lt":5}}]}),
        json!({"or":[false,{"not":"ready"}]}),
        json!({"value":{"eq":"other.value"}}),
    ] {
        let node: aep_domain::Node = serde_json::from_value(predicate.clone()).unwrap();
        let parsed = aep_domain::Predicate::from_node(&node).expect("inherited scalar grammar");
        let canonical = parsed.to_node();
        assert_eq!(
            aep_domain::Predicate::from_node(&canonical).unwrap(),
            parsed
        );
        let canonical = serde_json::to_value(canonical).unwrap();
        for quantify in [false, true] {
            let wrap = |value: Value| {
                if quantify {
                    json!({"forall":{"in":"rows","as":"row","that":value}})
                } else {
                    value
                }
            };
            let parent = suite(wrap(predicate.clone()));
            let changed = suite(wrap(canonical.clone()));
            let (report, input) = paired(&parent, &changed);
            adapt_json_coverage(&report, &input).unwrap_or_else(|error| {
                panic!("{predicate} => {canonical}, quantified={quantify}: {error}")
            });
        }
    }
}

fn step_suite(step: Value) -> Value {
    let mut result = fixtures::suite();
    result["scenarios"][fixtures::SELECTED]["steps"] = Value::Array(vec![step]);
    result
}

#[test]
fn comparison_retains_fields_of_every_remaining_step_and_shape_variant() {
    for (step, path, changed) in [
        (
            json!({"step":"configure_external_outcome","force":{"command":"demo.core.Read","outcome":"ready"}}),
            "/force/outcome",
            json!("refused"),
        ),
        (
            json!({"step":"expect_outcome","outcome":{"command":"demo.core.Read","outcome":"ready"}}),
            "/outcome/command",
            json!("demo.core.Other"),
        ),
        (
            json!({"step":"expect_no_event","event":"demo.core.Ready"}),
            "/event",
            json!("demo.core.Other"),
        ),
        (
            json!({"step":"redeliver_event","event":"demo.core.Ready"}),
            "/event",
            json!("demo.core.Other"),
        ),
        (
            json!({"step":"capture_instance","instance":"record","entity":"demo.core.Record","event":"demo.core.Ready","field":"identity"}),
            "/field",
            json!("other"),
        ),
        (
            json!({"step":"capture_instance","instance":"record","entity":"demo.core.Record","event":"demo.core.Ready","field":"identity"}),
            "/instance",
            json!("other"),
        ),
        (
            json!({"step":"capture_instance","instance":"record","entity":"demo.core.Record","event":"demo.core.Ready","field":"identity"}),
            "/entity",
            json!("demo.core.Other"),
        ),
        (
            json!({"step":"expect_invocation","binding":"read","command":"demo.core.Read"}),
            "/binding",
            json!("other"),
        ),
        (
            json!({"step":"mark_instant","instant":"start"}),
            "/instant",
            json!("other"),
        ),
        (
            json!({"step":"expect_event","event":"demo.core.Ready","shape":{"field":{"holds":"primitive","kind":"integer"}}}),
            "/shape/field/kind",
            json!("decimal"),
        ),
        (
            json!({"step":"eventually_event","event":"demo.core.Ready","shape":{"field":{"holds":"enum","variants":["one","two"]}}}),
            "/shape/field/variants",
            json!(["two", "one"]),
        ),
        (
            json!({"step":"expect_event","event":"demo.core.Ready","shape":{"field":{"holds":"map"}}}),
            "/shape/field/holds",
            json!("list"),
        ),
        (
            json!({"step":"expect_event","event":"demo.core.Ready","shape":{"field":{"holds":"list"}}}),
            "/shape/field/holds",
            json!("union"),
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"ranked","order_by":["one","two"]}}),
            "/expectation/order_by",
            json!(["two", "one"]),
        ),
        (
            json!({"step":"eventually_view","view":"demo.core.Reads","expectation":{"expect":"at","order_by":["id"],"position":{"row":"first"}}}),
            "/expectation/position/row",
            json!("last"),
        ),
    ] {
        let parent = step_suite(step.clone());
        let (report, input) = fixtures::pair_for(&parent, &[fixtures::SELECTED], "passed", &[]);
        adapt_json_coverage(&report, &input).expect("original inherited step admits");
        let mut child_step = step.clone();
        *child_step.pointer_mut(path).unwrap() = changed;
        let (report, input) = paired(&parent, &step_suite(child_step));
        let error = adapt_json_coverage(&report, &input).expect_err("complete field must survive");
        assert!(
            error
                .issues
                .iter()
                .any(|issue| issue.reason == "ParentScenarioMismatch"),
            "{step} {path}: {error}"
        );
    }
}

#[test]
fn comparison_view_cannot_admit_unknown_or_new_modeled_vocabulary() {
    for (step, expected) in [
        (
            json!({"step":"execute_command","command":"demo.core.Read","input":{"x":{"kind":"literal","value":1,"optional":false}}}),
            "UnknownField",
        ),
        (
            json!({"step":"expect_outcome","outcome":{"command":"demo.core.Read","outcome":"ready","optional":null}}),
            "UnknownField",
        ),
        (
            json!({"step":"expect_event","event":"demo.core.Ready","shape":{"x":{"holds":"list","kind":"integer"}}}),
            "UnknownField",
        ),
        (
            json!({"step":"expect_event","event":"demo.core.Ready","shape":{"x":{"holds":"primitive","kind":"binary64"}}}),
            "UnsupportedPrimitive",
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"at","order_by":["id"],"position":{"row":"first","index":0}}}),
            "UnknownField",
        ),
        (
            json!({"step":"expect_view","view":"demo.core.Reads","expectation":{"expect":"satisfies","predicate":{"forall":{"in":"rows","as":"row","that":true,"optional":null}}}}),
            "InvalidPredicate",
        ),
    ] {
        let suite = step_suite(step.clone());
        let (report, input) = fixtures::pair_for(&suite, &[fixtures::SELECTED], "passed", &[]);
        let error = adapt_json_coverage(&report, &input).expect_err("closed inherited admission");
        assert!(
            error.issues.iter().any(|issue| issue.reason == expected),
            "{step}: {error}"
        );
    }
}
