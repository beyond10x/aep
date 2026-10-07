//! Adversary pass 1 for `story:ess-grant-scenario-ids`: the story's outcome through the public CLI.
//!
//! `aep plan artifact evidence --from <report> --suite <suite>` records a report whose suite ESS
//! 0.55.0 wrote with command-grant, view-grant, seed and disclosure scenarios. Every fixture was
//! written by `ess 0.55.0`; the READMEs beside them say how.
use std::path::{Path, PathBuf};

const CURRENT_SUITES: &str = "crates/observe/aep-ess-evidence/tests/fixtures/current-suites";
const ADVERSARY_88: &str = "crates/observe/aep-ess-evidence/tests/fixtures/adversary-88";

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

fn success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `<set>/<file>`, where the set is `current-suites` or `adversary-88`.
fn fixture(name: &str) -> PathBuf {
    let (set, file) = name.split_once('/').unwrap();
    let set = if set == "current-suites" {
        CURRENT_SUITES
    } else {
        ADVERSARY_88
    };
    root().join(set).join(file)
}

/// A report, the flag naming its suite, the suite, the version and one id the record must name.
type Recording = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const RECORDINGS: [Recording; 5] = [
    (
        "current-suites/report-35-gatepass-external.json",
        "--suite",
        "current-suites/suite-35-gatepass.json",
        "ess-conformance/35",
        "gatepass.visit.AdmitVisitor/grant/denied",
    ),
    (
        "current-suites/report-35-view-grants-external.json",
        "--suite",
        "current-suites/suite-35-view-grants.json",
        "ess-conformance/35",
        "desk.tickets.Board/grant/read/admitted/desk.tickets.Clerk",
    ),
    (
        "current-suites/report-43-seeds-external.json",
        "--suite",
        "current-suites/suite-43-seeds.json",
        "ess-conformance/43",
        "counter.model.Authorize/outcome/authorized",
    ),
    (
        "adversary-88/report-35-desk-authored-external.json",
        "--suite",
        "adversary-88/suite-35-desk-authored.json",
        "ess-conformance/35",
        "desk.tickets.Titles/grant/read/admitted/desk.tickets.Visitor",
    ),
    (
        "adversary-88/report-35-one-time-actors-selected-external.json",
        "--suite-input",
        "adversary-88/input-35-one-time-actors-selected.json",
        "ess-conformance/35",
        "credentials.api.Issue/disclosure/issued/secret/denied/credentials.api.Read/as/actor/credentials.api.Guest",
    ),
];

#[test]
fn reports_whose_suites_carry_grant_seed_and_disclosure_scenarios_are_recorded_through_the_cli() {
    let directory = root()
        .join("target/ess-conformance-coverage/fixtures")
        .join(format!("adversary-88-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let store = directory.join("store");
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "granted",
        "--title",
        "Granted ESS suites",
        "--store",
        store.to_str().unwrap(),
    ]));
    for (report, flag, suite, _, _) in RECORDINGS {
        success(&cli(&[
            "plan",
            "artifact",
            "evidence",
            "story:granted",
            "--from",
            fixture(report).to_str().unwrap(),
            flag,
            fixture(suite).to_str().unwrap(),
            "--store",
            store.to_str().unwrap(),
        ]));
    }
    let output = cli(&[
        "plan",
        "artifact",
        "history",
        "story:granted",
        "--store",
        store.to_str().unwrap(),
        "--format",
        "json",
    ]);
    success(&output);
    let history: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let recorded: Vec<serde_json::Value> = history
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["change"]["source"].as_str())
        .map(|source| serde_json::from_str(source).unwrap())
        .collect();
    assert_eq!(recorded.len(), RECORDINGS.len());
    for (source, (report, _, _, version, id)) in recorded.iter().zip(RECORDINGS) {
        assert_eq!(source["suite"]["version"], version, "{report}");
        assert!(
            source["selected_ids"]
                .as_array()
                .unwrap()
                .iter()
                .any(|selected| selected == id),
            "{report}: {id} not recorded"
        );
    }
}
