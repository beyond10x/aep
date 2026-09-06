//! Independent tests of the original coverage reader contract.
use aep_ess_evidence::adapt_json_coverage;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const ID: &str = "demo.core.Read/outcome/ready";
const MODEL: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";

fn reference(original: &str) -> Value {
    let digest =
        Sha256::digest(original.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    json!({"version":"ess-conformance/5", "digest_profile":"sha256-json-bytes/1",
        "digest":digest})
}

fn suite() -> Value {
    json!({
        "provenance":{"suite_version":"ess-conformance/5", "system":"demo", "specification_version":"v1", "spec_digest":MODEL, "contract_digest":MODEL},
        "scenarios":{ID:{"purpose":"Observe ready", "steps":[{"step":"execute_command", "command":"demo.core.Read"}], "source":[]}},
        "coverage":{
            "selection":{"scope":{"kind":"system"}, "origins":"generated_and_authored", "filter":{"kind":"all"}},
            "knowledge":"complete_inventory", "generated":[ID], "authored":[], "outside":[], "refused":[], "authored_sources":{},
            "counts":{"generated":1, "authored":0, "outside":0, "refused":0}
        }
    })
}

fn pair(suite: &Value, parents: &[String]) -> (String, String) {
    let original = suite.to_string();
    let report = json!({
        "format":"ess-conformance-report/2", "specification":"demo/v1", "spec_digest":MODEL,
        "implementation":"independent adversary fixture", "producer_profile":"rust-scenario-status/1", "suite":reference(&original),
        "counts":{"total":1, "passed":1, "failed":0, "error":0, "unsupported":0, "skipped":0},
        "outcomes":{"passed":[ID], "failed":[], "error":[], "unsupported":[], "skipped":[]},
        "execution_status":"passed", "conformance_status":"passed", "policy":"complete-selection/1", "completed_at":1,
        "coverage":{"selection":suite["coverage"]["selection"], "knowledge":suite["coverage"]["knowledge"], "counts":suite["coverage"]["counts"], "refused":suite["coverage"]["refused"]}
    });
    let input =
        json!({"format":"ess-conformance-input/1", "suite_json":original, "parent_suites":parents});
    (report.to_string(), input.to_string())
}

#[test]
fn parent_lineage_accepts_omitted_nested_defaults() {
    let parent = suite();
    let (report, input) = pair(&parent, &[]);
    adapt_json_coverage(&report, &input).expect("unfiltered parent is admitted");
    let mut child = parent.clone();
    child["coverage"]["selection"]["filter"] =
        json!({"kind":"explicit", "ids":[ID], "parent":reference(&parent.to_string())});
    // ExecuteCommand.actor is Option with default None in the inherited suite vocabulary.
    // Its explicit null and omitted spellings denote the same complete surviving step.
    child["scenarios"][ID]["steps"][0]["actor"] = Value::Null;
    let (report, input) = pair(&child, &[parent.to_string()]);
    adapt_json_coverage(&report, &input)
        .expect("an explicit null default must not change the surviving command definition");
}

#[test]
fn parent_lineage_accepts_omitted_provenance_default() {
    let parent = suite();
    let mut child = parent.clone();
    child["provenance"]["component"] = Value::Null;
    child["coverage"]["selection"]["filter"] =
        json!({"kind":"explicit", "ids":[ID], "parent":reference(&parent.to_string())});
    let (report, input) = pair(&child, &[parent.to_string()]);
    adapt_json_coverage(&report, &input)
        .expect("absent and null provenance.component both declare the same system scope");
}

#[test]
fn parent_lineage_compares_payloads_in_inherited_number_domain() {
    let original: aep_domain::Node = serde_json::from_str("9007199254740993").unwrap();
    let equivalent: aep_domain::Node = serde_json::from_str("9007199254740992").unwrap();
    assert_eq!(
        original, equivalent,
        "the inherited Node domain is finite binary64"
    );
    let mut parent = suite();
    parent["scenarios"][ID]["steps"] = json!([{"step":"expect_event", "event":"demo.core.Ready", "payload":{"number":9_007_199_254_740_993_u64}}]);
    let (report, input) = pair(&parent, &[]);
    adapt_json_coverage(&report, &input).expect("original payload is admitted");
    let mut child = parent.clone();
    child["scenarios"][ID]["steps"][0]["payload"]["number"] = 9_007_199_254_740_992_u64.into();
    child["coverage"]["selection"]["filter"] =
        json!({"kind":"explicit", "ids":[ID], "parent":reference(&parent.to_string())});
    let (report, input) = pair(&child, &[parent.to_string()]);
    adapt_json_coverage(&report, &input)
        .expect("equal finite Node numbers must preserve the surviving payload definition");
}

#[test]
fn parent_lineage_preserves_real_changes_and_original_byte_controls() {
    let mut parent = suite();
    parent["scenarios"][ID]["steps"] = json!([{"step":"expect_halt", "view":"demo.core.Reads", "after":9_007_199_254_740_993_u64}]);
    let original = format!("\r\n{}\r\n", serde_json::to_string_pretty(&parent).unwrap());
    let mut child = parent.clone();
    child["coverage"]["selection"]["filter"] =
        json!({"kind":"explicit", "ids":[ID], "parent":reference(&original)});
    let (report, input) = pair(&child, std::slice::from_ref(&original));
    adapt_json_coverage(&report, &input).expect("exact noncanonical original parent admits");
    for (change, reason) in [
        ("integer", "ParentScenarioMismatch"),
        ("purpose", "ParentScenarioMismatch"),
        ("dependency", "ParentScenarioMismatch"),
        ("original_bytes", "ParentReferenceMismatch"),
    ] {
        let mut altered = child.clone();
        let mut parent_bytes = original.clone();
        match change {
            "integer" => {
                altered["scenarios"][ID]["steps"][0]["after"] = 9_007_199_254_740_992_u64.into();
            }
            "purpose" => altered["scenarios"][ID]["purpose"] = "A different obligation".into(),
            "dependency" => {
                altered["scenarios"][ID]["source"] =
                    json!([{"kind":"command", "name":"demo.core.Read"}]);
            }
            _ => parent_bytes.push('\n'),
        }
        let (report, input) = pair(&altered, &[parent_bytes]);
        let error = adapt_json_coverage(&report, &input).expect_err(change);
        assert!(
            error.issues.iter().any(|issue| issue.reason == reason),
            "{change}: {error}"
        );
    }
}
