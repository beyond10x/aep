//! `executable-system-specification`'s `conforming` rung, end to end through the CLI.
//!
//! A passed conformance report of any shape `aep plan artifact evidence --from` admits — report/1,
//! report/2 paired with a count-stage suite, report/2 paired with a coverage suite — pays for the
//! rung when it was run against the specification's own `model_digest`. A failed or inconclusive
//! report, or one run against another digest, does not, and the refusal says which and names the
//! kinds that would.

use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const CURRENT_SUITES: &str = "crates/observe/aep-ess-evidence/tests/fixtures/current-suites";
/// `spec_digest` of `report-31-external.json`, a passed report/2 over `suite-31.json`.
const DIGEST_31: &str = "afb43b637e6c8e3eb7743c0a7549e622c7bb317ce856891f30aa5e88c69881d7";
/// `spec_digest` of the two reports over `suite-27.json`: one failed, one inconclusive.
const DIGEST_27: &str = "1953c645f2e626e71630aedb67172a979809947f628e53a64a4642da8b8a4520";
/// `spec_digest` of `conformance-reports/passed.json`, a passed report/1.
const DIGEST_V1: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
/// The digest the synthetic count-stage pair below is run against.
const DIGEST_V2: &str = "8aee51b644a97580e2603ea3c9f57d22ca24d765643f2e0a4e0e6410dbfd1fef";

const KINDS: [&str; 3] = [
    "ess_conformance",
    "ess_conformance_v2",
    "ess_conformance_coverage_v1",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn fixture(name: &str) -> PathBuf {
    root().join(CURRENT_SUITES).join(name)
}

fn cli(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_aep"))
        .current_dir(root())
        .args(args)
        .output()
        .unwrap()
}

fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn success(output: &std::process::Output) {
    assert!(output.status.success(), "{}", text(output));
}

fn scratch(name: &str) -> PathBuf {
    let path = root()
        .join("target/conforming-rung/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path).unwrap();
    }
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// A store holding one validated specification recording `digest`.
fn validated_specification(store: &Path, digest: &str) -> &'static str {
    const ID: &str = "executable-system-specification:demo";
    let at = store.to_str().unwrap();
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "executable-system-specification",
        "demo",
        "--title",
        "Demo",
        "--store",
        at,
    ]));
    success(&cli(&[
        "plan",
        "artifact",
        "set",
        ID,
        "--model-digest",
        digest,
        "--store",
        at,
    ]));
    success(&cli(&[
        "plan",
        "artifact",
        "move",
        ID,
        "--to",
        "validated",
        "--store",
        at,
    ]));
    ID
}

fn record(store: &Path, id: &str, report: &Path, suite: Option<&Path>) {
    let mut args = vec![
        "plan",
        "artifact",
        "evidence",
        id,
        "--from",
        report.to_str().unwrap(),
    ];
    if let Some(suite) = suite {
        args.extend(["--suite", suite.to_str().unwrap()]);
    }
    args.extend(["--store", store.to_str().unwrap()]);
    success(&cli(&args));
}

fn move_to_conforming(store: &Path, id: &str) -> std::process::Output {
    cli(&[
        "plan",
        "artifact",
        "move",
        id,
        "--to",
        "conforming",
        "--store",
        store.to_str().unwrap(),
    ])
}

fn status_of(store: &Path) -> String {
    let document =
        std::fs::read_to_string(store.join("executable-system-specification/demo.md")).unwrap();
    document
        .lines()
        .find_map(|line| line.strip_prefix("status: "))
        .unwrap()
        .to_owned()
}

/// A passed report/2 over a count-stage (`ess-conformance/4`) suite, run against [`DIGEST_V2`]:
/// the pair `evidence --from --suite` records as `ess_conformance_v2`.
fn count_stage_pair(directory: &Path) -> (PathBuf, PathBuf) {
    const SELECTED: &str = "demo.core/authored/one";
    let suite = format!(
        r#"{{"provenance":{{"suite_version":"ess-conformance/4","system":"demo","specification_version":"v1","spec_digest":"{DIGEST_V2}","contract_digest":"{DIGEST_V2}"}},"scenarios":{{"{SELECTED}":{{"purpose":"An independently named scenario","steps":[],"source":[]}}}}}}"#
    );
    let digest =
        Sha256::digest(suite.as_bytes())
            .iter()
            .fold("sha256:".to_owned(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            });
    let report = serde_json::json!({"format":"ess-conformance-report/2","specification":"demo/v1","spec_digest":DIGEST_V2,"implementation":"fixture","producer_profile":"rust-scenario-status/1","suite":{"version":"ess-conformance/4","digest_profile":"sha256-json-bytes/1","digest":digest},"counts":{"total":1,"passed":1,"failed":0,"error":0,"unsupported":0,"skipped":0},"outcomes":{"passed":[SELECTED],"failed":[],"error":[],"unsupported":[],"skipped":[]},"execution_status":"passed","conformance_status":"inconclusive","coverage":{"knowledge":"unknown"},"policy":"complete-selection/1","completed_at":1_700_000_000_001_u64}).to_string();
    let report_path = directory.join("report-v2.json");
    let suite_path = directory.join("suite-v2.json");
    std::fs::write(&report_path, report).unwrap();
    std::fs::write(&suite_path, suite).unwrap();
    (report_path, suite_path)
}

#[test]
fn a_passed_report_2_with_its_coverage_suite_moves_a_specification_to_conforming() {
    let directory = scratch("coverage-passed");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_31);
    record(
        &store,
        id,
        &fixture("report-31-external.json"),
        Some(&fixture("suite-31.json")),
    );

    let moved = move_to_conforming(&store, id);
    success(&moved);
    assert_eq!(status_of(&store), "conforming", "{}", text(&moved));
}

#[test]
fn a_passed_count_stage_report_2_moves_a_specification_to_conforming() {
    let directory = scratch("v2-passed");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_V2);
    let (report, suite) = count_stage_pair(&directory);
    record(&store, id, &report, Some(&suite));

    let moved = move_to_conforming(&store, id);
    success(&moved);
    assert_eq!(status_of(&store), "conforming", "{}", text(&moved));
}

#[test]
fn a_passed_report_1_still_moves_a_specification_to_conforming() {
    let directory = scratch("v1-passed");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_V1);
    record(
        &store,
        id,
        &root().join("crates/edge/aep-cli/tests/fixtures/conformance-reports/passed.json"),
        None,
    );

    let moved = move_to_conforming(&store, id);
    success(&moved);
    assert_eq!(status_of(&store), "conforming", "{}", text(&moved));
}

/// The refusal names what the record was, why it does not count, and every kind that would.
fn assert_refused(store: &Path, id: &str, why: &[&str]) {
    let refused = move_to_conforming(store, id);
    let said = text(&refused);
    assert!(!refused.status.success(), "{said}");
    assert_eq!(status_of(store), "validated", "{said}");
    for kind in KINDS {
        assert!(said.contains(kind), "the refusal names `{kind}`: {said}");
    }
    for reason in why {
        assert!(said.contains(reason), "the refusal says `{reason}`: {said}");
    }
}

#[test]
fn a_failed_report_2_does_not_move_a_specification_to_conforming_and_says_why() {
    let directory = scratch("coverage-failed");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_27);
    record(
        &store,
        id,
        &fixture("report-27-run.json"),
        Some(&fixture("suite-27.json")),
    );
    assert_refused(&store, id, &["ess_conformance_coverage_v1", "failed"]);
}

#[test]
fn an_inconclusive_report_2_does_not_move_a_specification_to_conforming_and_says_why() {
    let directory = scratch("coverage-inconclusive");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_27);
    record(
        &store,
        id,
        &fixture("report-27-external.json"),
        Some(&fixture("suite-27.json")),
    );
    assert_refused(&store, id, &["inconclusive"]);
}

#[test]
fn a_passed_report_2_for_another_digest_does_not_move_a_specification_to_conforming() {
    let directory = scratch("coverage-other-digest");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_27);
    record(
        &store,
        id,
        &fixture("report-31-external.json"),
        Some(&fixture("suite-31.json")),
    );
    assert_refused(&store, id, &[DIGEST_31, DIGEST_27]);
}

#[test]
fn a_failed_report_1_does_not_move_a_specification_to_conforming() {
    let directory = scratch("v1-failed");
    let store = directory.join("store");
    let id = validated_specification(&store, DIGEST_V1);
    let report = directory.join("failed.json");
    std::fs::write(
        &report,
        serde_json::json!({
            "format": "ess-conformance-report/1",
            "specification": "billing/v3",
            "spec_digest": DIGEST_V1,
            "implementation": "billing-reference 0.3.0",
            "status": "failed",
            "scenarios_total": 29,
            "scenarios_failed": 2,
            "suite_version": "ess-conformance/1",
            "failed_scenarios": ["a", "b"],
            "completed_at": 1_700_000_009_900_u64
        })
        .to_string(),
    )
    .unwrap();
    record(&store, id, &report, None);
    assert_refused(&store, id, &["2 of 29"]);
}
