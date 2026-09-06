//! Independently authored ESS coverage inputs; expectations never come from emitted reports.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const MODEL: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
pub const SELECTED: &str = "demo.core.Read/outcome/ready";

pub fn suite() -> Value {
    json!({
        "provenance": {"suite_version":"ess-conformance/5", "system":"demo", "specification_version":"v1", "spec_digest":MODEL, "contract_digest":MODEL},
        "scenarios": {SELECTED: {"purpose":"Observe the declared ready outcome", "steps":[], "source":[]}},
        "coverage": {
            "selection":{"scope":{"kind":"system"},"origins":"generated_and_authored","filter":{"kind":"all"}},
            "knowledge":"complete_inventory", "generated":[SELECTED], "authored":[], "outside":[], "refused":[], "authored_sources":{},
            "counts":{"generated":1,"authored":0,"outside":0,"refused":0}
        }
    })
}

pub fn reference(original: &str) -> Value {
    let digest =
        Sha256::digest(original.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    json!({"version":"ess-conformance/5", "digest_profile":"sha256-json-bytes/1", "digest":digest})
}

pub fn pair(time: u64) -> (String, String) {
    let suite = suite();
    let original = suite.to_string();
    let report = json!({
        "format":"ess-conformance-report/2", "specification":"demo/v1", "spec_digest":MODEL, "implementation":"independent fixture",
        "producer_profile":"rust-scenario-status/1", "suite":reference(&original),
        "counts":{"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0},
        "outcomes":{"passed":[SELECTED],"failed":[],"error":[],"unsupported":[],"skipped":[]},
        "execution_status":"passed", "conformance_status":"passed", "policy":"complete-selection/1", "completed_at":time,
        "coverage":{"knowledge":"complete_inventory","selection":suite["coverage"]["selection"],"counts":{"generated":1,"authored":0,"outside":0,"refused":0},"refused":[]}
    });
    let input =
        json!({"format":"ess-conformance-input/1", "suite_json":original, "parent_suites":[]});
    (report.to_string(), input.to_string())
}

pub fn pair_for(
    suite: &Value,
    selected: &[&str],
    conformance: &str,
    parents: &[String],
) -> (String, String) {
    let original = suite.to_string();
    let (baseline, _) = pair(1);
    let mut report: Value = serde_json::from_str(&baseline).unwrap();
    report["suite"] = reference(&original);
    report["counts"]["total"] = selected.len().into();
    report["counts"]["passed"] = selected.len().into();
    report["outcomes"]["passed"] = json!(selected);
    report["conformance_status"] = conformance.into();
    for key in ["selection", "knowledge", "counts", "refused"] {
        report["coverage"][key] = suite["coverage"][key].clone();
    }
    let input =
        json!({"format":"ess-conformance-input/1", "suite_json":original, "parent_suites":parents});
    (report.to_string(), input.to_string())
}

pub fn refusal() -> Value {
    json!({"origin":"generated", "scenario":SELECTED, "subject":{"kind":"command","name":"demo.core.Read"}, "source":null,
        "code":"ESS-SYNTH-005", "message":"original undecidable view cause", "effect":"check_not_emitted",
        "retained":{"origin":"generated","source":null}, "scope":"in_scope", "needs":[]})
}

pub fn outside_suite() -> Value {
    let mut suite = suite();
    suite["provenance"]["component"] = "api".into();
    suite["coverage"]["selection"]["scope"] =
        serde_json::json!({"kind":"component","component":"api"});
    let other = "demo.core.Other/outcome/ready";
    let needs = serde_json::json!([{"kind":"command","name":"demo.core.Other"}]);
    suite["coverage"]["outside"] = serde_json::json!([{"scenario":other,"origin":"generated","reason":"other_component","needs":needs}]);
    suite["coverage"]["counts"]["outside"] = 1.into();
    suite["coverage"]["counts"]["refused"] = 1.into();
    let mut refusal = refusal();
    refusal["scenario"] = other.into();
    refusal["scope"] = "outside_component".into();
    refusal["needs"] = needs;
    suite["coverage"]["refused"] = serde_json::json!([refusal]);
    suite
}

pub fn child(parent: &Value, selected: &[&str]) -> Value {
    let mut child = parent.clone();
    child["coverage"]["selection"]["filter"] =
        json!({"kind":"explicit", "ids":selected, "parent":reference(&parent.to_string())});
    child["scenarios"]
        .as_object_mut()
        .unwrap()
        .retain(|id, _| selected.contains(&id.as_str()));
    for (key, origin) in [("generated", "generated"), ("authored", "authored")] {
        let original = parent["coverage"][key].as_array().unwrap();
        child["coverage"][key] = json!(original
            .iter()
            .filter(|id| selected.contains(&id.as_str().unwrap()))
            .collect::<Vec<_>>());
        child["coverage"]["counts"][key] = child["coverage"][key].as_array().unwrap().len().into();
        for omitted in original
            .iter()
            .filter(|id| !selected.contains(&id.as_str().unwrap()))
        {
            child["coverage"]["outside"].as_array_mut().unwrap().push(json!({"scenario":omitted, "origin":origin, "reason":"selection_filter", "needs":[]}));
        }
    }
    child["coverage"]["outside"]
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["scenario"].as_str().cmp(&b["scenario"].as_str()));
    child["coverage"]["counts"]["outside"] = child["coverage"]["outside"]
        .as_array()
        .unwrap()
        .len()
        .into();
    child
}
