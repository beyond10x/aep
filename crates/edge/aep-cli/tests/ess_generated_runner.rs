//! `aep plan artifact evidence --from <report/2> --suite <suite>` with a report ESS's generated Go
//! runner wrote: `producer_profile: go-scenario-status/2`, the profile of the generated Go and
//! TypeScript runners from ESS 0.56.0 on. On 0.71.1 it was refused `UnsupportedProducerProfile`.
//!
//! The fixtures under `fixtures/generated-runner/` in aep-ess-evidence's tests were written by
//! `ess 0.57.0` and its generated Go package; their README says how. The outcomes are
//! `aep.evidence.RecordFromReport` in `ess/domains/evidence.yaml`.
use std::path::{Path, PathBuf};

const GENERATED_RUNNER: &str = "crates/observe/aep-ess-evidence/tests/fixtures/generated-runner";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn fixture(name: &str) -> PathBuf {
    root().join(GENERATED_RUNNER).join(name)
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
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn success(output: &std::process::Output) {
    assert!(output.status.success(), "{}", text(output));
}

/// A scratch directory holding a store with one story, `story:generated`.
fn store(name: &str) -> (PathBuf, PathBuf) {
    let directory = root()
        .join("target/ess-generated-runner/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    std::fs::create_dir_all(&directory).unwrap();
    let store = directory.join("store");
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "generated",
        "--title",
        "Generated runner reports",
        "--store",
        store.to_str().unwrap(),
    ]));
    (directory, store)
}

fn record(store: &Path, report: &Path, suite: &Path) -> std::process::Output {
    cli(&[
        "plan",
        "artifact",
        "evidence",
        "story:generated",
        "--from",
        report.to_str().unwrap(),
        "--suite",
        suite.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
    ])
}

/// Every evidence entry `history` shows for the story.
fn evidence(store: &Path) -> Vec<serde_json::Value> {
    let output = cli(&[
        "plan",
        "artifact",
        "history",
        "story:generated",
        "--store",
        store.to_str().unwrap(),
        "--format",
        "json",
    ]);
    success(&output);
    let history: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    history
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["change"]["change"] == "evidence")
        .cloned()
        .collect()
}

/// Records the report against its suite and returns the one record's kind and parsed source.
fn recorded_once(name: &str, report: &str, suite: &str) -> (String, serde_json::Value) {
    let (_, store) = store(name);
    let report = fixture(report);
    let suite = fixture(suite);
    success(&record(&store, &report, &suite));
    let recorded = evidence(&store);
    assert_eq!(recorded.len(), 1, "one record: {recorded:?}");
    let change = &recorded[0]["change"];
    assert_eq!(change["reference"], report.to_str().unwrap());
    let source: serde_json::Value =
        serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
    assert_eq!(source["format"], "ess-conformance-report/2");
    assert_eq!(source["suite_input"], suite.to_str().unwrap());
    assert_eq!(source["specification"], "aep/v1");
    assert_eq!(source["producer_profile"], "go-scenario-status/2");
    assert_eq!(source["policy"], "complete-selection/1");
    assert_eq!(
        source["counts"],
        serde_json::json!({"total":19,"passed":0,"failed":0,"error":8,"unsupported":11,"skipped":0})
    );
    // Unsupported scenarios fail execution under go-scenario-status/2.
    assert_eq!(source["execution_status"], "failed");
    assert_eq!(source["conformance_status"], "failed");
    // The report's own instant, never this process's clock.
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert_eq!(
        source["completed_at"],
        written["completed_at"].as_u64().unwrap().to_string()
    );
    (change["kind"].as_str().unwrap().to_owned(), source)
}

#[test]
fn a_generated_runner_report_over_an_ordinary_suite_is_recorded_as_one_count_record() {
    let (kind, source) = recorded_once("ordinary", "report-34-aep-go.json", "suite-34-aep.json");
    assert_eq!(kind, "ess_conformance_v2");
    assert_eq!(source["suite"]["version"], "ess-conformance/34");
    assert_eq!(
        source["coverage"],
        serde_json::json!({"knowledge":"unknown"})
    );
}

#[test]
fn a_generated_runner_report_over_a_coverage_suite_is_recorded_as_one_coverage_record() {
    let (kind, source) = recorded_once("coverage", "report-35-aep-go.json", "suite-35-aep.json");
    // A coverage suite goes to the coverage reader, which records its own kind.
    assert_eq!(kind, "ess_conformance_coverage_v1");
    assert_eq!(source["suite"]["version"], "ess-conformance/35");
    assert_eq!(source["selected_ids"].as_array().unwrap().len(), 19);
}

#[test]
fn an_unknown_go_profile_is_refused_by_name_and_records_nothing() {
    let (directory, store) = store("unknown");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture("report-34-aep-go.json")).unwrap())
            .unwrap();
    report["producer_profile"] = "go-scenario-status/3".into();
    let path = directory.join("report-34-go-3.json");
    std::fs::write(&path, report.to_string()).unwrap();
    let output = record(&store, &path, &fixture("suite-34-aep.json"));
    assert!(!output.status.success(), "{}", text(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("UnsupportedProducerProfile") && stderr.contains("go-scenario-status/3"),
        "expected UnsupportedProducerProfile naming go-scenario-status/3: {stderr}"
    );
    assert!(evidence(&store).is_empty(), "a refusal writes nothing");
}
