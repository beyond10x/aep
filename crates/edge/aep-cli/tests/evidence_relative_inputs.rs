//! `aep plan artifact evidence --from <report>` names the files it read by their path relative to
//! the repository root, beside each one's digest, and never by the path as typed: an absolute
//! local path means nothing on another machine and Gates' personal-paths scan refuses it. An input
//! outside the repository is named by its file name alone. After recording, the command says
//! whether the record it wrote counts toward a move and, when it does not, why.
//!
//! The rule is `aep.evidence.InputForm` and `aep.evidence.InputPathNotPortable` in
//! `ess/domains/evidence.yaml` (story:evidence-records-relative-inputs).

use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const CURRENT_SUITES: &str = "crates/observe/aep-ess-evidence/tests/fixtures/current-suites";
const REPORT_1: &str = "crates/edge/aep-cli/tests/fixtures/conformance-reports/passed.json";
/// `spec_digest` of `report-31-external.json`, a passed report/2 over `suite-31.json`.
const DIGEST_31: &str = "afb43b637e6c8e3eb7743c0a7549e622c7bb317ce856891f30aa5e88c69881d7";
/// `spec_digest` of `report-27-run.json`, a failed report/2 over `suite-27.json`.
const DIGEST_27: &str = "1953c645f2e626e71630aedb67172a979809947f628e53a64a4642da8b8a4520";
/// `spec_digest` of `conformance-reports/passed.json`, a passed report/1.
const DIGEST_V1: &str = "13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861";
/// The digest the synthetic count-stage pair below is run against.
const DIGEST_V2: &str = "8aee51b644a97580e2603ea3c9f57d22ca24d765643f2e0a4e0e6410dbfd1fef";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
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

fn digest_of(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().fold(
        "sha256:".to_owned(),
        |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        },
    )
}

/// A scratch repository with a Git-native store at `.engineering/planning`, and a directory
/// beside it that is outside it. Returns `(scratch, repository, store)`.
fn repository(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let scratch = root()
        .join("target/evidence-relative-inputs/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch).unwrap();
    }
    let repository = scratch.join("repository");
    let store = repository.join(".engineering/planning");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(scratch.join("outside")).unwrap();
    (scratch, repository, store)
}

fn new_artifact(store: &Path, kind: &str, name: &str) -> String {
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        kind,
        name,
        "--title",
        "Relative inputs",
        "--store",
        store.to_str().unwrap(),
    ]));
    format!("{kind}:{name}")
}

/// A specification recording `digest`, moved to `validated`.
fn validated_specification(store: &Path, digest: &str) -> String {
    let id = new_artifact(store, "executable-system-specification", "demo");
    let at = store.to_str().unwrap();
    success(&cli(&[
        "plan",
        "artifact",
        "set",
        &id,
        "--model-digest",
        digest,
        "--store",
        at,
    ]));
    success(&cli(&[
        "plan",
        "artifact",
        "move",
        &id,
        "--to",
        "validated",
        "--store",
        at,
    ]));
    id
}

/// Copies a fixture to `to`, creating its directory.
fn place(fixture: &Path, to: &Path) -> PathBuf {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(fixture, to).unwrap();
    to.to_path_buf()
}

fn current(name: &str) -> PathBuf {
    root().join(CURRENT_SUITES).join(name)
}

/// A passed report/2 over a count-stage (`ess-conformance/4`) suite run against [`DIGEST_V2`],
/// written into `directory`: what `evidence --from --suite` records as `ess_conformance_v2`.
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
    std::fs::create_dir_all(directory).unwrap();
    let report_path = directory.join("report-v2.json");
    let suite_path = directory.join("suite-v2.json");
    std::fs::write(&report_path, report).unwrap();
    std::fs::write(&suite_path, suite).unwrap();
    (report_path, suite_path)
}

fn evidence(store: &Path, id: &str, report: &str, suite: Option<&str>) -> std::process::Output {
    let mut args = vec!["plan", "artifact", "evidence", id, "--from", report];
    if let Some(suite) = suite {
        args.extend(["--suite", suite]);
    }
    args.extend(["--store", store.to_str().unwrap()]);
    cli(&args)
}

/// Every evidence file under the repository's `.engineering/evidence/`.
fn evidence_files(repository: &Path) -> BTreeSet<PathBuf> {
    fn walk(path: &Path, found: &mut BTreeSet<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, found);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                found.insert(path);
            }
        }
    }
    let mut found = BTreeSet::new();
    walk(&repository.join(".engineering/evidence"), &mut found);
    found
}

/// Records through `record` and returns the one evidence file it wrote, as text.
fn recorded(repository: &Path, record: impl FnOnce() -> std::process::Output) -> String {
    let before = evidence_files(repository);
    let output = record();
    success(&output);
    let written: Vec<PathBuf> = evidence_files(repository)
        .difference(&before)
        .cloned()
        .collect();
    assert_eq!(written.len(), 1, "one record written: {written:?}");
    std::fs::read_to_string(&written[0]).unwrap()
}

/// The record carries no absolute path and no parent component: nothing of where this checkout
/// is on this machine.
fn assert_no_local_path(record: &str, repository: &Path) {
    for local in [repository.to_path_buf(), root()] {
        let local = local.to_str().unwrap();
        assert!(
            !record.contains(local),
            "the record names the local path {local}: {record}"
        );
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        assert!(!record.contains(&home), "the record names $HOME: {record}");
    }
    assert!(!record.contains(".."), "a parent component: {record}");
}

fn change(record: &str) -> serde_json::Value {
    let entry: serde_json::Value = serde_json::from_str(record).unwrap();
    entry["change"].clone()
}

fn source(record: &str) -> serde_json::Value {
    serde_json::from_str(change(record)["source"].as_str().unwrap()).unwrap()
}

#[test]
fn a_count_stage_report_2_inside_the_repository_is_recorded_relative_to_its_root_with_digests() {
    let (_, repository, store) = repository("v2-inside");
    let id = new_artifact(&store, "story", "inputs");
    let (report, suite) = count_stage_pair(&repository.join("reports/run"));

    let record = recorded(&repository, || {
        evidence(
            &store,
            &id,
            report.to_str().unwrap(),
            Some(suite.to_str().unwrap()),
        )
    });

    assert_no_local_path(&record, &repository);
    assert_eq!(change(&record)["kind"], "ess_conformance_v2", "{record}");
    let source = source(&record);
    assert_eq!(source["report_input"], "reports/run/report-v2.json");
    assert_eq!(source["report_input_digest"], digest_of(&report));
    assert_eq!(source["suite_input"], "reports/run/suite-v2.json");
    assert_eq!(source["suite_input_digest"], digest_of(&suite));
    assert_eq!(
        change(&record)["reference"],
        format!("reports/run/report-v2.json@{}", digest_of(&report))
    );
}

#[test]
fn a_coverage_report_2_inside_the_repository_is_recorded_relative_to_its_root_with_digests() {
    let (_, repository, store) = repository("coverage-inside");
    let id = new_artifact(&store, "story", "inputs");
    let report = place(
        &current("report-31-external.json"),
        &repository.join("conformance/report-31.json"),
    );
    let suite = place(
        &current("suite-31.json"),
        &repository.join("conformance/suite-31.json"),
    );
    // Spelled with a `.` and a `..` that stay inside the repository: the name is still the path
    // from the root, with neither component.
    let report_typed = repository.join("conformance/./../conformance/report-31.json");

    let record = recorded(&repository, || {
        evidence(
            &store,
            &id,
            report_typed.to_str().unwrap(),
            Some(suite.to_str().unwrap()),
        )
    });

    assert_no_local_path(&record, &repository);
    assert_eq!(
        change(&record)["kind"],
        "ess_conformance_coverage_v1",
        "{record}"
    );
    let source = source(&record);
    assert_eq!(source["report_input"], "conformance/report-31.json");
    assert_eq!(source["report_input_digest"], digest_of(&report));
    assert_eq!(source["suite_input"], "conformance/suite-31.json");
    assert_eq!(source["suite_input_digest"], digest_of(&suite));
    assert_eq!(
        change(&record)["reference"],
        format!("conformance/report-31.json@{}", digest_of(&report))
    );
}

#[test]
fn a_report_1_inside_the_repository_is_recorded_relative_to_its_root_with_its_digest() {
    let (_, repository, store) = repository("v1-inside");
    let id = new_artifact(&store, "story", "inputs");
    let report = place(
        &root().join(REPORT_1),
        &repository.join("reports/passed.json"),
    );

    let record = recorded(&repository, || {
        evidence(&store, &id, report.to_str().unwrap(), None)
    });

    assert_no_local_path(&record, &repository);
    assert_eq!(change(&record)["kind"], "ess_conformance", "{record}");
    assert_eq!(
        change(&record)["reference"],
        format!("reports/passed.json@{}", digest_of(&report))
    );
}

#[test]
fn an_input_outside_the_repository_is_recorded_by_file_name_and_digest() {
    let (scratch, repository, store) = repository("outside");
    let id = new_artifact(&store, "story", "inputs");
    let outside = scratch.join("outside");

    // ess_conformance_v2, the report reached through the repository and out of it again.
    let (report, suite) = count_stage_pair(&outside.join("v2"));
    let report_typed = repository.join("../outside/v2/report-v2.json");
    let record = recorded(&repository, || {
        evidence(
            &store,
            &id,
            report_typed.to_str().unwrap(),
            Some(suite.to_str().unwrap()),
        )
    });
    assert_no_local_path(&record, &repository);
    assert!(!record.contains("outside/"), "a directory kept: {record}");
    let read = source(&record);
    assert_eq!(read["report_input"], "report-v2.json");
    assert_eq!(read["report_input_digest"], digest_of(&report));
    assert_eq!(read["suite_input"], "suite-v2.json");
    assert_eq!(read["suite_input_digest"], digest_of(&suite));
    assert_eq!(
        change(&record)["reference"],
        format!("report-v2.json@{}", digest_of(&report))
    );

    // ess_conformance_coverage_v1.
    let report = place(
        &current("report-31-external.json"),
        &outside.join("coverage/report-31.json"),
    );
    let suite = place(
        &current("suite-31.json"),
        &outside.join("coverage/suite-31.json"),
    );
    let record = recorded(&repository, || {
        evidence(
            &store,
            &id,
            report.to_str().unwrap(),
            Some(suite.to_str().unwrap()),
        )
    });
    assert_no_local_path(&record, &repository);
    assert!(!record.contains("coverage/"), "a directory kept: {record}");
    let read = source(&record);
    assert_eq!(read["report_input"], "report-31.json");
    assert_eq!(read["report_input_digest"], digest_of(&report));
    assert_eq!(read["suite_input"], "suite-31.json");
    assert_eq!(read["suite_input_digest"], digest_of(&suite));

    // ess_conformance, from report/1.
    let report = place(&root().join(REPORT_1), &outside.join("v1/passed.json"));
    let record = recorded(&repository, || {
        evidence(&store, &id, report.to_str().unwrap(), None)
    });
    assert_no_local_path(&record, &repository);
    assert_eq!(
        change(&record)["reference"],
        format!("passed.json@{}", digest_of(&report))
    );
}

#[test]
fn a_failed_report_2_against_the_specifications_model_digest_says_why_it_does_not_count() {
    let (_, repository, store) = repository("does-not-count");
    let id = validated_specification(&store, DIGEST_27);
    let report = place(
        &current("report-27-run.json"),
        &repository.join("conformance/report-27.json"),
    );
    let suite = place(
        &current("suite-27.json"),
        &repository.join("conformance/suite-27.json"),
    );

    let output = evidence(
        &store,
        &id,
        report.to_str().unwrap(),
        Some(suite.to_str().unwrap()),
    );
    success(&output);
    let said = String::from_utf8_lossy(&output.stdout);
    let line = said
        .lines()
        .find(|line| line.contains("does not count:"))
        .unwrap_or_else(|| panic!("no `does not count:` line: {}", text(&output)));
    // The reason is the one `conformance_count` gives the move refusal.
    assert!(
        line.contains("conformance_status is failed"),
        "the reason is the judgement's own: {line}"
    );
}

#[test]
fn a_passed_report_2_against_the_specifications_model_digest_says_it_counts() {
    let (_, repository, store) = repository("counts");
    let id = validated_specification(&store, DIGEST_31);
    let report = place(
        &current("report-31-external.json"),
        &repository.join("conformance/report-31.json"),
    );
    let suite = place(
        &current("suite-31.json"),
        &repository.join("conformance/suite-31.json"),
    );

    let output = evidence(
        &store,
        &id,
        report.to_str().unwrap(),
        Some(suite.to_str().unwrap()),
    );
    success(&output);
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        said.lines()
            .any(|line| line.trim() == "this record: counts"),
        "{}",
        text(&output)
    );
    assert!(!said.contains("does not count"), "{}", text(&output));
}

/// Records already in stores keep the path as typed in 0.71.2 and earlier; they are read,
/// counted and validated as before, and nothing rewrites them.
#[test]
fn a_record_written_with_absolute_paths_still_reads_counts_and_validates() {
    let (_, repository, store) = repository("old-shape");
    let id = validated_specification(&store, DIGEST_V1);
    // The shape 0.71.2 wrote for a passed report/1: `reference` is the path as typed.
    let typed = "/srv/checkout/reports/passed.json";
    let old = serde_json::json!({
        "at": "2023-11-14T22:13:29.900Z",
        "actor": "human:operator",
        "artifact": id,
        "kind": "executable-system-specification",
        "revision": 3,
        "change": {
            "change": "evidence",
            "kind": "ess_conformance",
            "source": format!(
                "ess conform run: billing-reference 0.3.0 against billing/v3 at {DIGEST_V1}, 0 of \
                 29 scenario(s) failed"
            ),
            "reference": typed
        }
    });
    let directory = repository.join(".engineering/evidence/executable-system-specification/demo");
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("20231114T221329Z-000-0123456789ab.json");
    let bytes = serde_json::to_string_pretty(&old).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    let at = store.to_str().unwrap();

    let history = cli(&[
        "plan", "artifact", "history", &id, "--store", at, "--format", "json",
    ]);
    success(&history);
    assert!(
        String::from_utf8_lossy(&history.stdout).contains(typed),
        "read as written: {}",
        text(&history)
    );
    success(&cli(&["plan", "artifact", "validate", "--store", at]));
    let moved = cli(&[
        "plan",
        "artifact",
        "move",
        &id,
        "--to",
        "conforming",
        "--store",
        at,
    ]);
    success(&moved);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        bytes,
        "nothing rewrites it"
    );
}
