use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const MODEL: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
pub const SELECTED: &str = "review.data/authored/one";

pub fn suite() -> String {
    serde_json::json!({
        "provenance": {"suite_version":"ess-conformance/4", "system":"review",
            "specification_version":"v1", "spec_digest":MODEL, "contract_digest":MODEL},
        "scenarios": {SELECTED: {"purpose":"Read café observations", "steps":[], "source":[]}}
    })
    .to_string()
}

pub fn report(suite: &str, time: u64) -> String {
    let digest =
        Sha256::digest(suite.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    serde_json::json!({
        "format":"ess-conformance-report/2", "specification":"review/v1", "spec_digest":MODEL,
        "implementation":"first reader review", "producer_profile":"rust-scenario-status/1",
        "suite":{"version":"ess-conformance/4", "digest_profile":"sha256-json-bytes/1", "digest":digest},
        "execution_status":"passed", "counts":{"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0},
        "outcomes":{"passed":[SELECTED],"failed":[],"error":[],"unsupported":[],"skipped":[]},
        "coverage":{"knowledge":"unknown"}, "conformance_status":"inconclusive",
        "policy":"complete-selection/1", "completed_at":time
    }).to_string()
}
