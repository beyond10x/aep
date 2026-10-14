//! `aep plan artifact evidence <id> --kind task_spend --from <file>` records what one source says
//! one task spent: the model calls, the five token figures and their total, the cost, the wall
//! time and, for a driven run, the llm steps declared and run. The source is a transcript, an
//! `eval.run-manifest/1` or a driven run's cursor; the record keeps its digest and its path from
//! the repository root. A figure the source does not state is absent from the record and printed
//! `unknown`, never `0`.
//!
//! The record is `aep.evidence.SpendRecord` and the command `aep.evidence.RecordSpend` in
//! `ess/domains/evidence.yaml` (story:evidence-records-task-spend).

use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// A recorded Claude Code `stream-json` transcript: one session, one terminal record carrying all
/// five token figures, a cost and a wall time.
const CLAUDE_TRANSCRIPT: &str = "crates/observe/trace-spec/tests/fixtures/plugin-eval-1huAQG.jsonl";
/// A recorded `metaharness.event/1` driven stream: one session, one terminal record.
const DRIVEN_STREAM: &str = "crates/edge/aep-cli/fixtures/eval-run/claude-driven-attested.jsonl";
/// A committed `eval.run-manifest/1`.
const MANIFEST: &str =
    "crates/edge/aep-cli/fixtures/eval-matrix/runs/claude-raw-create-a-story.manifest.yaml";
/// The cursor a driven run persists, with six llm steps declared and two attempts begun.
const CURSOR: &str = "crates/edge/aep-cli/tests/fixtures/task-spend/cursor.json";
/// A transcript whose terminal record states no usage, no cost and no duration, and which made no
/// model call.
const NO_USAGE: &str = "crates/edge/aep-cli/tests/fixtures/task-spend/no-usage.jsonl";

/// The figures every spend record may carry, in the specification's order.
const FIGURES: [&str; 11] = [
    "model_calls",
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "thinking_tokens",
    "total_tokens",
    "cost_micro_usd",
    "wall_time_ms",
    "llm_steps_declared",
    "llm_steps_run",
];

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

/// A scratch repository with a Git-native store at `.engineering/planning`, a story in it, and a
/// directory beside it that is outside it. Returns `(scratch, repository, store, id)`.
fn repository(name: &str) -> (PathBuf, PathBuf, PathBuf, String) {
    let scratch = root()
        .join("target/evidence-task-spend/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch).unwrap();
    }
    let repository = scratch.join("repository");
    let store = repository.join(".engineering/planning");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(scratch.join("outside")).unwrap();
    success(&cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "spend",
        "--title",
        "Task spend",
        "--store",
        store.to_str().unwrap(),
    ]));
    (scratch, repository, store, "story:spend".to_owned())
}

/// Copies a committed fixture to `to`, creating its directory.
fn place(fixture: &str, to: &Path) -> PathBuf {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(root().join(fixture), to).unwrap();
    to.to_path_buf()
}

fn spend(store: &Path, id: &str, from: &Path) -> std::process::Output {
    cli(&[
        "plan",
        "artifact",
        "evidence",
        id,
        "--kind",
        "task_spend",
        "--from",
        from.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
    ])
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
            } else {
                found.insert(path);
            }
        }
    }
    let mut found = BTreeSet::new();
    walk(&repository.join(".engineering/evidence"), &mut found);
    found
}

/// Records the spend `from` states and returns the record's change and the command's stdout.
fn recorded(repository: &Path, store: &Path, id: &str, from: &Path) -> (serde_json::Value, String) {
    let before = evidence_files(repository);
    let output = spend(store, id, from);
    success(&output);
    let written: Vec<PathBuf> = evidence_files(repository)
        .difference(&before)
        .cloned()
        .collect();
    assert_eq!(written.len(), 1, "one record written: {written:?}");
    let entry: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&written[0]).unwrap()).unwrap();
    (
        entry["change"].clone(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// The record's source, which is the spend record itself.
fn source(change: &serde_json::Value) -> serde_json::Value {
    serde_json::from_str(change["source"].as_str().unwrap()).unwrap()
}

/// The line the command prints for one figure, its value with its run of spaces collapsed.
fn printed(stdout: &str, figure: &str) -> String {
    stdout
        .lines()
        .find_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some(figure)).then(|| words.collect::<Vec<_>>().join(" "))
        })
        .unwrap_or_else(|| panic!("no `{figure}` line in {stdout}"))
}

/// The one `"type":"result"` record of a `stream-json` transcript.
fn terminal_record(path: &str) -> serde_json::Value {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|value| value["type"] == "result")
        .expect("the transcript has a terminal record")
}

/// How many API requests a `stream-json` transcript made: distinct `request_id` over its
/// assistant records, and every assistant record without one on its own.
fn api_requests(path: &str) -> u64 {
    let mut identified = BTreeSet::new();
    let mut anonymous = 0;
    for value in std::fs::read_to_string(root().join(path))
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| value["type"] == "assistant")
    {
        match value["request_id"].as_str() {
            Some(id) => {
                identified.insert(id.to_owned());
            }
            None => anonymous += 1,
        }
    }
    identified.len() as u64 + anonymous
}

#[test]
fn a_claude_code_transcript_records_its_requests_usage_cost_and_wall_time() {
    let (_, repository, store, id) = repository("transcript");
    let transcript = place(
        CLAUDE_TRANSCRIPT,
        &repository.join("runs/one/transcript.jsonl"),
    );

    let (change, stdout) = recorded(&repository, &store, &id, &transcript);

    assert_eq!(change["kind"], "task_spend", "{change}");
    let record = source(&change);
    let terminal = terminal_record(CLAUDE_TRANSCRIPT);
    let usage = &terminal["usage"];
    assert_eq!(record["source"], "transcript", "{record}");
    assert_eq!(record["source_input"], "runs/one/transcript.jsonl");
    assert_eq!(record["source_digest"], digest_of(&transcript));
    assert_eq!(
        change["reference"],
        format!("runs/one/transcript.jsonl@{}", digest_of(&transcript))
    );
    assert_eq!(
        record["model_calls"],
        api_requests(CLAUDE_TRANSCRIPT),
        "model calls are the transcript's API requests"
    );
    assert_eq!(record["input_tokens"], usage["input_tokens"]);
    assert_eq!(record["output_tokens"], usage["output_tokens"]);
    assert_eq!(
        record["cache_read_tokens"],
        usage["cache_read_input_tokens"]
    );
    assert_eq!(
        record["cache_write_tokens"],
        usage["cache_creation_input_tokens"]
    );
    assert_eq!(
        record["thinking_tokens"],
        usage["output_tokens_details"]["thinking_tokens"]
    );
    let five: u64 = [
        &usage["input_tokens"],
        &usage["output_tokens"],
        &usage["cache_read_input_tokens"],
        &usage["cache_creation_input_tokens"],
        &usage["output_tokens_details"]["thinking_tokens"],
    ]
    .iter()
    .map(|figure| figure.as_u64().unwrap())
    .sum();
    assert_eq!(record["total_tokens"], five, "all five known: their sum");
    // `0.25222239999999996` US dollars, to the nearest millionth.
    assert_eq!(
        terminal["total_cost_usd"].as_f64(),
        Some(0.252_222_399_999_999_96)
    );
    assert_eq!(record["cost_micro_usd"], 252_222);
    assert_eq!(record["wall_time_ms"], terminal["duration_ms"]);
    assert!(record.get("llm_steps_declared").is_none(), "{record}");
    assert!(record.get("llm_steps_run").is_none(), "{record}");

    assert_eq!(printed(&stdout, "cost_micro_usd"), "252222");
    assert_eq!(printed(&stdout, "llm_steps_run"), "unknown");
}

#[test]
fn a_run_manifest_records_its_tokens_cost_and_wall_time_and_no_split() {
    let (_, repository, store, id) = repository("manifest");
    let manifest = place(MANIFEST, &repository.join("runs/raw.manifest.yaml"));

    let (change, stdout) = recorded(&repository, &store, &id, &manifest);

    let record = source(&change);
    assert_eq!(record["source"], "run_manifest", "{record}");
    assert_eq!(record["source_input"], "runs/raw.manifest.yaml");
    assert_eq!(record["source_digest"], digest_of(&manifest));
    assert_eq!(record["total_tokens"], 30_412, "the manifest's `tokens`");
    assert_eq!(record["cost_micro_usd"], 83_700);
    assert_eq!(record["wall_time_ms"], 3_390);
    for split in [
        "model_calls",
        "input_tokens",
        "output_tokens",
        "cache_read_tokens",
        "cache_write_tokens",
        "thinking_tokens",
    ] {
        assert!(
            record.get(split).is_none(),
            "a manifest states no `{split}`: {record}"
        );
        assert_eq!(printed(&stdout, split), "unknown", "{stdout}");
    }
    assert_eq!(printed(&stdout, "total_tokens"), "30412");
}

#[test]
fn a_driven_run_cursor_records_the_llm_steps_declared_and_run() {
    let (_, repository, store, id) = repository("cursor");
    let cursor = place(CURSOR, &repository.join(".aep/runs/AUTH-142/3/cursor.json"));

    let (change, stdout) = recorded(&repository, &store, &id, &cursor);

    let record = source(&change);
    assert_eq!(record["source"], "driven_run", "{record}");
    assert_eq!(record["source_input"], ".aep/runs/AUTH-142/3/cursor.json");
    assert_eq!(record["llm_steps_declared"], 6, "{record}");
    assert_eq!(record["llm_steps_run"], 2, "{record}");
    for unstated in &FIGURES[..9] {
        assert!(record.get(*unstated).is_none(), "{unstated}: {record}");
    }
    assert_eq!(printed(&stdout, "llm_steps_declared"), "6");
    assert_eq!(printed(&stdout, "llm_steps_run"), "2");
}

#[test]
fn a_transcript_with_no_usage_records_every_figure_unknown_and_prints_unknown() {
    let (_, repository, store, id) = repository("no-usage");
    let transcript = place(NO_USAGE, &repository.join("runs/no-usage.jsonl"));

    let (change, stdout) = recorded(&repository, &store, &id, &transcript);

    let record = source(&change);
    assert_eq!(record["source"], "transcript", "{record}");
    for figure in FIGURES {
        assert!(
            record.get(figure).is_none(),
            "`{figure}` is unknown and absent, never 0: {record}"
        );
        assert_eq!(printed(&stdout, figure), "unknown", "{stdout}");
    }
    assert!(
        !stdout.lines().any(|line| line.trim_end().ends_with(" 0")),
        "no figure is printed as 0: {stdout}"
    );
}

#[test]
fn a_session_that_writes_several_terminal_records_is_folded_as_one_spend() {
    let (_, repository, store, id) = repository("fold");
    let one = std::fs::read_to_string(root().join(DRIVEN_STREAM)).unwrap();
    let ended: serde_json::Value = one
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|value| value["event"] == "session.ended")
        .unwrap();
    let started = one
        .lines()
        .find(|line| line.contains(r#""event":"session.started""#))
        .unwrap();

    // The recorded session, stopped at its cap after its `result`: the second terminal record
    // restates the cost, zeroes its usage and states a shorter wall clock. Then a second session
    // of its own. The stream closes after both.
    let mut lines: Vec<String> = one
        .lines()
        .filter(|line| !line.contains(r#""event":"stream.closed""#))
        .map(ToOwned::to_owned)
        .collect();
    let mut seq = lines.len();
    let mut next = || {
        seq += 1;
        seq
    };
    lines.push(format!(
        r#"{{"format":"metaharness.event/1","seq":{},"run":"R3-4/claude-driven","event":"session.ended","is_error":true,"subtype":"error_max_budget_usd","terminal_reason":"budget_exhausted","num_turns":1,"duration_ms":12532,"total_cost_usd":0.5216,"usage":{{"input_tokens":0,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"thinking_tokens":0}}}}"#,
        next()
    ));
    let mut second: serde_json::Value = serde_json::from_str(started).unwrap();
    second["seq"] = next().into();
    lines.push(second.to_string());
    lines.push(format!(
        r#"{{"format":"metaharness.event/1","seq":{},"run":"R3-4/claude-driven","event":"session.ended","is_error":false,"subtype":"success","terminal_reason":"completed","num_turns":1,"duration_ms":1000,"total_cost_usd":0.25,"usage":{{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":30,"cache_creation_input_tokens":4,"thinking_tokens":5}}}}"#,
        next()
    ));
    let closing = next();
    lines.push(format!(
        r#"{{"format":"metaharness.event/1","seq":{closing},"run":"R3-4/claude-driven","event":"stream.closed","events":{closing},"reason":"completed"}}"#
    ));
    let stream = repository.join("runs/driven.jsonl");
    std::fs::create_dir_all(stream.parent().unwrap()).unwrap();
    std::fs::write(&stream, lines.join("\n") + "\n").unwrap();

    let (change, _) = recorded(&repository, &store, &id, &stream);

    let record = source(&change);
    let usage = &ended["usage"];
    let figure = |key: &str| usage[key].as_u64().unwrap();
    assert_eq!(
        record["input_tokens"],
        figure("input_tokens") + 1,
        "the larger of one session's records, plus the next session's: {record}"
    );
    assert_eq!(record["output_tokens"], figure("output_tokens") + 2);
    assert_eq!(
        record["cache_read_tokens"],
        figure("cache_read_input_tokens") + 30
    );
    assert_eq!(
        record["cache_write_tokens"],
        figure("cache_creation_input_tokens") + 4
    );
    assert_eq!(record["thinking_tokens"], figure("thinking_tokens") + 5);
    assert_eq!(
        record["cost_micro_usd"],
        521_600 + 250_000,
        "the capped session is charged once"
    );
    assert_eq!(
        record["wall_time_ms"],
        ended["duration_ms"].as_u64().unwrap() + 1_000,
        "the capped session ran as long as its longest statement"
    );
}

#[test]
fn a_spend_source_outside_the_repository_is_refused_naming_it() {
    let (scratch, repository, store, id) = repository("outside");
    let transcript = place(CLAUDE_TRANSCRIPT, &scratch.join("outside/transcript.jsonl"));
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &transcript);

    assert!(!output.status.success(), "{}", text(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("SpendSourceOutsideRepository"),
        "the refusal is named: {stderr}"
    );
    assert!(
        stderr.contains(transcript.to_str().unwrap()),
        "the refusal names the path: {stderr}"
    );
    assert_eq!(evidence_files(&repository), before, "nothing was written");
}
