//! `aep plan artifact evidence --from <report/2> --suite <suite>` with an ordinary suite: the
//! even major from /6 on that ESS 0.56.0 and later write by default, with no `coverage` block.
//!
//! The suite is routed by what it is: a coverage major goes to the coverage reader, a count-stage
//! or ordinary major the build knows to the count reader, and any other version is refused by
//! name. The outcomes are `aep.evidence.RecordFromReport` in `ess/domains/evidence.yaml`.
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const ORDINARY_SUITES: &str = "crates/observe/aep-ess-evidence/tests/fixtures/ordinary-suites";
const CURRENT_SUITES: &str = "crates/observe/aep-ess-evidence/tests/fixtures/current-suites";
const ESS_0692: &str = "crates/observe/aep-ess-evidence/tests/fixtures/ess-0692";
const ESS_0770: &str = "crates/observe/aep-ess-evidence/tests/fixtures/ess-0770";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn ordinary(name: &str) -> PathBuf {
    root().join(ORDINARY_SUITES).join(name)
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

fn digest(original: &str) -> String {
    Sha256::digest(original.as_bytes())
        .iter()
        .fold("sha256:".to_owned(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

/// A scratch directory holding a store with one story, `story:ordinary`.
fn store(name: &str) -> (PathBuf, PathBuf) {
    let directory = root()
        .join("target/ess-ordinary-suites/fixtures")
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
        "ordinary",
        "--title",
        "Ordinary ESS suites",
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
        "story:ordinary",
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
        "story:ordinary",
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

fn refused_naming(output: &std::process::Output, reason: &str, named: &str) {
    assert!(!output.status.success(), "{}", text(output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(reason) && stderr.contains(named),
        "expected {reason} naming {named}: {stderr}"
    );
    assert!(
        !stderr.contains("MissingField"),
        "refused as a missing field: {stderr}"
    );
}

#[test]
fn an_ordinary_suite_ess_wrote_is_recorded_with_its_report() {
    let (_, store) = store("recorded");
    let report = ordinary("report-34-aep.json");
    let suite = ordinary("suite-34-aep.json");
    success(&record(&store, &report, &suite));

    let recorded = evidence(&store);
    assert_eq!(recorded.len(), 1, "one record: {recorded:?}");
    let change = &recorded[0]["change"];
    assert_eq!(change["kind"], "ess_conformance_v2", "{change}");
    // Both fixtures lie outside the repository the scratch store sits in, so each is named by its
    // file name beside its digest, never by the path it was given as
    // (story:evidence-records-relative-inputs).
    let report_digest = digest(&std::fs::read_to_string(&report).unwrap());
    assert_eq!(
        change["reference"],
        format!("report-34-aep.json@{report_digest}")
    );
    let source: serde_json::Value =
        serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
    assert_eq!(source["format"], "ess-conformance-report/2");
    assert_eq!(source["report_input"], "report-34-aep.json");
    assert_eq!(source["report_input_digest"], report_digest);
    assert_eq!(source["suite_input"], "suite-34-aep.json");
    assert_eq!(
        source["suite_input_digest"],
        digest(&std::fs::read_to_string(&suite).unwrap())
    );
    assert_eq!(source["specification"], "aep/v1");
    assert_eq!(source["suite"]["version"], "ess-conformance/34");
    assert_eq!(
        source["suite"]["digest"],
        digest(&std::fs::read_to_string(&suite).unwrap())
    );
    assert_eq!(
        source["counts"],
        serde_json::json!({"total":11,"passed":5,"failed":0,"error":0,"unsupported":6,"skipped":0})
    );
    assert_eq!(source["execution_status"], "failed");
    assert_eq!(source["conformance_status"], "failed");
    assert_eq!(
        source["coverage"],
        serde_json::json!({"knowledge":"unknown"})
    );
    // The report's own instant, never this process's clock.
    assert_eq!(source["completed_at"], "1700000003700");
}

#[test]
fn an_ordinary_suite_whose_bytes_differ_from_the_report_digest_is_refused_and_records_nothing() {
    let (directory, store) = store("digest");
    let changed = directory.join("suite-34-changed.json");
    let original = std::fs::read_to_string(ordinary("suite-34-aep.json")).unwrap();
    std::fs::write(&changed, format!("{original}\n")).unwrap();
    let output = record(&store, &ordinary("report-34-aep.json"), &changed);
    refused_naming(&output, "SuiteDigestMismatch", "sha256:");
    assert!(evidence(&store).is_empty(), "a refusal writes nothing");
}

#[test]
fn a_report_naming_another_suite_version_than_its_ordinary_suite_is_refused_and_records_nothing() {
    let (directory, store) = store("version");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(ordinary("report-34-aep.json")).unwrap())
            .unwrap();
    report["suite"]["version"] = "ess-conformance/36".into();
    let path = directory.join("report-36.json");
    std::fs::write(&path, report.to_string()).unwrap();
    let output = record(&store, &path, &ordinary("suite-34-aep.json"));
    refused_naming(&output, "SuiteVersionMismatch", "ess-conformance/36");
    assert!(evidence(&store).is_empty(), "a refusal writes nothing");
}

#[test]
fn an_ordinary_major_this_build_does_not_know_is_refused_by_name_not_as_a_missing_field() {
    let (directory, store) = store("unknown");
    let original = std::fs::read_to_string(ordinary("suite-34-aep.json")).unwrap();
    let relabelled = original.replace(
        "\"suite_version\": \"ess-conformance/34\"",
        "\"suite_version\": \"ess-conformance/60\"",
    );
    assert_ne!(relabelled, original);
    let suite = directory.join("suite-60.json");
    std::fs::write(&suite, &relabelled).unwrap();
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(ordinary("report-34-aep.json")).unwrap())
            .unwrap();
    report["suite"] = serde_json::json!({"version":"ess-conformance/60","digest_profile":"sha256-json-bytes/1","digest":digest(&relabelled)});
    let path = directory.join("report-60.json");
    std::fs::write(&path, report.to_string()).unwrap();

    let output = record(&store, &path, &suite);
    refused_naming(&output, "UnsupportedSuiteVersion", "ess-conformance/60");
    // The suite's own version is named even beside a report that names one this build knows.
    let output = record(&store, &ordinary("report-34-aep.json"), &suite);
    refused_naming(&output, "UnsupportedSuiteVersion", "ess-conformance/60");
    assert!(evidence(&store).is_empty(), "a refusal writes nothing");
}

#[test]
fn a_coverage_suite_still_goes_to_the_coverage_reader() {
    let (_, store) = store("coverage");
    let fixture = |name: &str| root().join(CURRENT_SUITES).join(name);
    success(&record(
        &store,
        &fixture("report-31-external.json"),
        &fixture("suite-31.json"),
    ));
    let recorded = evidence(&store);
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0]["change"]["kind"], "ess_conformance_coverage_v1");
}

/// `aep.evidence.RecordFromReport/outcome/recorded` for the suite majors ESS 0.69.2 writes
/// (story:evidence-admits-ess-0692-suites): an ordinary /54 and a coverage /55, each written
/// with its report by `ess 0.69.2`, are recorded, the first as count-stage evidence and the second
/// through the coverage reader. On 0.77.1 both were `UnsupportedSuiteVersion`.
#[test]
fn the_suites_ess_0_69_2_writes_are_recorded_with_their_reports() {
    let fixture = |name: &str| root().join(ESS_0692).join(name);
    for (suite, report, kind) in [
        ("suite-54.json", "report-54.json", "ess_conformance_v2"),
        (
            "suite-55.json",
            "report-55.json",
            "ess_conformance_coverage_v1",
        ),
    ] {
        let (_, store) = store(&format!("ess-0692-{kind}"));
        success(&record(&store, &fixture(report), &fixture(suite)));
        let recorded = evidence(&store);
        assert_eq!(recorded.len(), 1, "{suite}: {recorded:?}");
        let change = &recorded[0]["change"];
        assert_eq!(change["kind"], kind, "{suite}: {change}");
        let source: serde_json::Value =
            serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
        assert_eq!(
            source["suite"]["digest"],
            digest(&std::fs::read_to_string(fixture(suite)).unwrap()),
            "{suite}"
        );
        assert_eq!(source["counts"]["passed"], 8, "{suite}: {source}");
    }
}

/// `aep.evidence.RecordFromReport/outcome/recorded` for the newest suite majors ESS 0.77.0 writes
/// (story:evidence-admits-ess-0770-suites): an ordinary /58 and a coverage /59, each written with
/// its report by `ess 0.77.0` over a complete subject carrying a `Decimal`, are recorded, the first
/// as count-stage evidence and the second through the coverage reader. On 0.78.0 both were
/// `UnsupportedSuiteVersion`.
#[test]
fn the_suites_ess_0_77_0_writes_are_recorded_with_their_reports() {
    let fixture = |name: &str| root().join(ESS_0770).join(name);
    for (suite, report, kind) in [
        ("suite-58.json", "report-58.json", "ess_conformance_v2"),
        (
            "suite-59.json",
            "report-59.json",
            "ess_conformance_coverage_v1",
        ),
    ] {
        let (_, store) = store(&format!("ess-0770-{kind}"));
        success(&record(&store, &fixture(report), &fixture(suite)));
        let recorded = evidence(&store);
        assert_eq!(recorded.len(), 1, "{suite}: {recorded:?}");
        let change = &recorded[0]["change"];
        assert_eq!(change["kind"], kind, "{suite}: {change}");
        let source: serde_json::Value =
            serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
        assert_eq!(
            source["suite"]["digest"],
            digest(&std::fs::read_to_string(fixture(suite)).unwrap()),
            "{suite}"
        );
        assert_eq!(source["suite"]["version"], suite_version(suite), "{suite}");
        assert_eq!(
            (&source["counts"]["total"], &source["counts"]["passed"]),
            (&serde_json::json!(5), &serde_json::json!(5)),
            "{suite}: {source}"
        );
    }
}

/// `ess-conformance/<N>` for a fixture named `suite-<N>.json`.
fn suite_version(file: &str) -> String {
    let major = file
        .strip_prefix("suite-")
        .and_then(|rest| rest.strip_suffix(".json"))
        .expect("a fixture named suite-<N>.json");
    format!("ess-conformance/{major}")
}

/// `aep.evidence.RecordFromReport/outcome/unsupported-suite-version` past ESS 0.77.0: the /58
/// fixture relabelled /60, the next ordinary major after the newest this build lists, is refused
/// by name and records nothing.
#[test]
fn a_major_newer_than_ess_0_77_0_is_refused_by_name_and_records_nothing() {
    let (directory, store) = store("ess-0770-newer");
    let fixture = |name: &str| root().join(ESS_0770).join(name);
    let original = std::fs::read_to_string(fixture("suite-58.json")).unwrap();
    let relabelled = original.replace(
        "\"suite_version\": \"ess-conformance/58\"",
        "\"suite_version\": \"ess-conformance/60\"",
    );
    assert_ne!(relabelled, original);
    let suite = directory.join("suite-60.json");
    std::fs::write(&suite, &relabelled).unwrap();
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture("report-58.json")).unwrap()).unwrap();
    report["suite"] = serde_json::json!({"version":"ess-conformance/60","digest_profile":"sha256-json-bytes/1","digest":digest(&relabelled)});
    let path = directory.join("report-60.json");
    std::fs::write(&path, report.to_string()).unwrap();
    refused_naming(
        &record(&store, &path, &suite),
        "UnsupportedSuiteVersion",
        "ess-conformance/60",
    );
    assert!(evidence(&store).is_empty(), "a refusal writes nothing");
}
