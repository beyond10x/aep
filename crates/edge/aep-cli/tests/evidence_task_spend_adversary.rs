//! Adversarial cases against `aep plan artifact evidence <id> --kind task_spend --from <file>`
//! (story:evidence-records-task-spend). Each case states what the specification
//! (`ess/domains/evidence.yaml`, `aep.evidence.RecordSpend` / `SpendRecord`) or the implementation's
//! own documentation promises, and drives the CLI against it.
//! Each was red against the first cut of the verb; the doc comment on a case says what it did.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const DRIVEN_STREAM: &str = "crates/edge/aep-cli/fixtures/eval-run/claude-driven-attested.jsonl";

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
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `(scratch, repository, store, id)`, with `scratch/outside` beside the repository.
fn repository(name: &str) -> (PathBuf, PathBuf, PathBuf, String) {
    let scratch = root()
        .join("target/evidence-task-spend-adversary/fixtures")
        .join(format!("{name}-{}", std::process::id()));
    if scratch.exists() {
        std::fs::remove_dir_all(&scratch).unwrap();
    }
    let repository = scratch.join("repository");
    let store = repository.join(".engineering/planning");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(scratch.join("outside")).unwrap();
    let made = cli(&[
        "plan",
        "artifact",
        "new",
        "story",
        "spend",
        "--title",
        "Task spend",
        "--store",
        store.to_str().unwrap(),
    ]);
    assert!(made.status.success(), "{}", text(&made));
    (scratch, repository, store, "story:spend".to_owned())
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

/// Asserts the refusal on `output` names the specification's error `code`.
fn refused_as(output: &std::process::Output, code: &str) {
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(code),
        "the refusal is named `{code}`: {}",
        text(output)
    );
}

/// The spend record written by `output`, or `None` when nothing was written.
fn written_record(repository: &Path, before: &BTreeSet<PathBuf>) -> Option<serde_json::Value> {
    let written: Vec<PathBuf> = evidence_files(repository)
        .difference(before)
        .cloned()
        .collect();
    let path = written.first()?;
    let entry: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    Some(entry["change"].clone())
}

/// The recorded driven stream, its `stream.closed` dropped, with `extra` lines appended (each a
/// format string whose `{seq}` is replaced with the next sequence number) and a closing marker.
fn driven_stream_with(extra: &[&str]) -> String {
    let one = std::fs::read_to_string(root().join(DRIVEN_STREAM)).unwrap();
    let mut lines: Vec<String> = one
        .lines()
        .filter(|line| !line.contains(r#""event":"stream.closed""#))
        .map(ToOwned::to_owned)
        .collect();
    let started = one
        .lines()
        .find(|line| line.contains(r#""event":"session.started""#))
        .unwrap()
        .to_owned();
    for line in extra {
        let seq = lines.len() + 1;
        if *line == "SESSION_STARTED" {
            let mut value: serde_json::Value = serde_json::from_str(&started).unwrap();
            value["seq"] = seq.into();
            lines.push(value.to_string());
        } else {
            lines.push(line.replace("{seq}", &seq.to_string()));
        }
    }
    let closing = lines.len() + 1;
    lines.push(format!(
        r#"{{"format":"metaharness.event/1","seq":{closing},"run":"R3-4/claude-driven","event":"stream.closed","events":{closing},"reason":"completed"}}"#
    ));
    lines.join("\n") + "\n"
}

/// ESS `aep.evidence.RecordSpend`, outcome `source-digest-missing`: "A spend record names no
/// source digest. Nothing is written." `--kind task_spend` with `--source` and no `--from` takes
/// the ordinary path and writes a `task_spend` record whose source is free text: no digest, no
/// `source_input`, no figures.
#[test]
fn task_spend_without_a_source_file_is_refused_and_writes_nothing() {
    let (_, repository, store, id) = repository("no-from");
    let before = evidence_files(&repository);

    let output = cli(&[
        "plan",
        "artifact",
        "evidence",
        &id,
        "--kind",
        "task_spend",
        "--source",
        "I spent about a dollar",
        "--store",
        store.to_str().unwrap(),
    ]);

    let written = written_record(&repository, &before);
    assert!(
        !output.status.success() && written.is_none(),
        "a task_spend record with no source digest must be refused (SpendSourceDigestMissing) and \
         write nothing; got {} and wrote {written:?}",
        text(&output)
    );
    refused_as(&output, "SpendSourceDigestMissing");
}

/// `read_spend`'s own contract: "a cost no reader can convert, which is refused rather than
/// recorded as unknown", and `micro_usd_stated`: "Unreadable is not unstated". A terminal record
/// whose `total_cost_usd` is a string is read by the stream-json adapter as no cost at all, so
/// the record says `cost_micro_usd` unknown where `aep eval run` (`cost_of`) refuses the stream.
#[test]
fn a_cost_written_as_text_is_refused_not_recorded_unknown() {
    let (_, repository, store, id) = repository("string-cost");
    let transcript = repository.join("runs/string-cost.jsonl");
    std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    std::fs::write(
        &transcript,
        concat!(
            r#"{"type":"system","subtype":"init","session_id":"s-1","model":"claude-sonnet-5","tools":[]}"#,
            "\n",
            r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":1000,"total_cost_usd":"0.80","usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":4,"output_tokens_details":{"thinking_tokens":5}},"session_id":"s-1"}"#,
            "\n"
        ),
    )
    .unwrap();
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &transcript);

    let written = written_record(&repository, &before);
    assert!(
        !output.status.success() && written.is_none(),
        "an unreadable cost is refused, not recorded as unknown; got {} and wrote {written:?}",
        text(&output)
    );
    refused_as(&output, "SpendSourceUnreadable");
}

/// A file that is no transcript, manifest or cursor at all: one JSON object per line that no
/// reader recognises. It is recorded as a `transcript` spend with every figure unknown, so a
/// mistyped `--from` becomes a durable spend record of nothing.
#[test]
fn a_json_file_that_is_no_spend_source_is_refused() {
    let (_, repository, store, id) = repository("not-a-source");
    let file = repository.join("config/settings.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "{\"theme\":\"dark\",\"font_size\":12}\n").unwrap();
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &file);

    let written = written_record(&repository, &before);
    assert!(
        !output.status.success() && written.is_none(),
        "a file that is not a transcript, a run manifest or a cursor is refused; got {} and \
         wrote {written:?}",
        text(&output)
    );
    refused_as(&output, "SpendSourceUnreadable");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(file.to_str().unwrap()),
        "the refusal names the file: {}",
        text(&output)
    );
}

/// Unknown never becomes 0, and a total over part of the run is not a total
/// (`SpendFigures::with_split_total`: "Four of five is not a total: it would understate the
/// run"). Two sessions; the second states no cost (`null`, as a Codex session writes). The record
/// states the first session's cost alone as the task's known cost.
#[test]
fn a_session_that_states_no_cost_leaves_the_task_cost_unknown() {
    let (_, repository, store, id) = repository("partial-cost");
    let stream = repository.join("runs/partial-cost.jsonl");
    std::fs::create_dir_all(stream.parent().unwrap()).unwrap();
    std::fs::write(
        &stream,
        driven_stream_with(&[
            "SESSION_STARTED",
            r#"{"format":"metaharness.event/1","seq":{seq},"run":"R3-4/claude-driven","event":"session.ended","is_error":false,"subtype":"success","terminal_reason":"completed","num_turns":1,"duration_ms":1000,"total_cost_usd":null,"usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":30,"cache_creation_input_tokens":4,"thinking_tokens":5}}"#,
        ]),
    )
    .unwrap();
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &stream);

    assert!(output.status.success(), "{}", text(&output));
    let change = written_record(&repository, &before).expect("one record");
    let record: serde_json::Value =
        serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
    assert!(
        record.get("cost_micro_usd").is_none(),
        "one of two sessions stated no cost, so the task's cost is unknown, not the other \
         session's 521600: {record}"
    );
}

/// The same, for a session cut off before its terminal record: the stream's second session
/// starts, makes a request and never ends. Its spend is unknown; the record states the first
/// session's figures as the task's.
#[test]
fn a_session_with_no_terminal_record_leaves_the_task_figures_unknown() {
    let (_, repository, store, id) = repository("truncated-session");
    let stream = repository.join("runs/truncated.jsonl");
    std::fs::create_dir_all(stream.parent().unwrap()).unwrap();
    std::fs::write(&stream, driven_stream_with(&["SESSION_STARTED"])).unwrap();
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &stream);

    assert!(output.status.success(), "{}", text(&output));
    let change = written_record(&repository, &before).expect("one record");
    let record: serde_json::Value =
        serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
    for figure in [
        "cost_micro_usd",
        "input_tokens",
        "total_tokens",
        "wall_time_ms",
    ] {
        assert!(
            record.get(figure).is_none(),
            "a session that never ended spent an unknown amount, so `{figure}` is unknown: \
             {record}"
        );
    }
}

/// `with_split_total`: "A sum that overflows is not a figure either." Across sessions the fold
/// (`accumulate`) saturates instead, and the record states `u64::MAX` as a known figure.
#[test]
fn a_token_figure_that_overflows_across_sessions_is_not_recorded_as_saturated() {
    let (_, repository, store, id) = repository("overflow");
    let stream = repository.join("runs/overflow.jsonl");
    std::fs::create_dir_all(stream.parent().unwrap()).unwrap();
    std::fs::write(
        &stream,
        driven_stream_with(&[
            "SESSION_STARTED",
            r#"{"format":"metaharness.event/1","seq":{seq},"run":"R3-4/claude-driven","event":"session.ended","is_error":false,"subtype":"success","terminal_reason":"completed","num_turns":1,"duration_ms":1000,"total_cost_usd":0.25,"usage":{"input_tokens":18446744073709551615,"output_tokens":2,"cache_read_input_tokens":30,"cache_creation_input_tokens":4,"thinking_tokens":5}}"#,
        ]),
    )
    .unwrap();
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &stream);

    let record = written_record(&repository, &before).map(|change| {
        serde_json::from_str::<serde_json::Value>(change["source"].as_str().unwrap()).unwrap()
    });
    if let Some(record) = record {
        assert_ne!(
            record
                .get("input_tokens")
                .and_then(serde_json::Value::as_u64),
            Some(u64::MAX),
            "34 + u64::MAX does not fit: refuse or leave it unknown, never record the saturated \
             value as a count: {record} ({})",
            text(&output)
        );
    }
}

/// ESS `aep.evidence.SpendSourceOutsideRepository`: "The source lies outside the repository root
/// ... Nothing is written." A directory inside the repository that is a link to a directory
/// outside it: the file read lies outside, and the record names it as if it were inside.
#[cfg(unix)]
#[test]
fn a_source_reached_through_a_link_to_outside_the_repository_is_refused() {
    let (scratch, repository, store, id) = repository("link-dir");
    let outside = scratch.join("outside/transcript.jsonl");
    std::fs::copy(root().join(DRIVEN_STREAM), &outside).unwrap();
    std::os::unix::fs::symlink(scratch.join("outside"), repository.join("runs")).unwrap();
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &repository.join("runs/transcript.jsonl"));

    let written = written_record(&repository, &before);
    assert!(
        !output.status.success() && written.is_none(),
        "the file lies outside the repository and is refused; got {} and wrote {written:?}",
        text(&output)
    );
    refused_as(&output, "SpendSourceOutsideRepository");
}

/// `link/../transcript.jsonl` is normalised lexically to `transcript.jsonl` at the repository
/// root, but the operating system resolves `link/..` to the link target's parent, outside the
/// repository. The record names a file that does not exist in the repository, beside the digest of
/// a different file outside it.
#[cfg(unix)]
#[test]
fn a_dot_dot_after_a_link_names_the_file_the_record_read() {
    let (scratch, repository, store, id) = repository("link-dotdot");
    std::fs::create_dir_all(scratch.join("outside/deeper")).unwrap();
    std::fs::copy(
        root().join(DRIVEN_STREAM),
        scratch.join("outside/transcript.jsonl"),
    )
    .unwrap();
    std::os::unix::fs::symlink(scratch.join("outside/deeper"), repository.join("link")).unwrap();
    let typed = repository.join("link/../transcript.jsonl");
    assert!(typed.exists(), "the OS resolves it outside the repository");
    assert!(
        !repository.join("transcript.jsonl").exists(),
        "and nothing of that name is in the repository"
    );
    let before = evidence_files(&repository);

    let output = spend(&store, &id, &typed);

    let written = written_record(&repository, &before);
    if let Some(change) = &written {
        let record: serde_json::Value =
            serde_json::from_str(change["source"].as_str().unwrap()).unwrap();
        let named = repository.join(record["source_input"].as_str().unwrap());
        assert!(
            named.exists(),
            "the record names `{}`, which is not in the repository; the bytes it digested were \
             read outside it: {record}",
            record["source_input"]
        );
    } else {
        // The file read lies outside the repository, so a refusal is the outside one.
        refused_as(&output, "SpendSourceOutsideRepository");
    }
}

/// `total_tokens` is one field of one record whichever source it was read from. For a run
/// manifest it is the manifest's `tokens`, which `aep eval run` writes as `tokens_of`: the four
/// `TOKEN_KEYS`, thinking excluded. For the transcript of that same run it is the sum of five,
/// thinking included. The recorded driven stream states `thinking_tokens: 104`, so the two records
/// of one run disagree by 104.
#[test]
fn a_transcript_total_equals_the_total_its_run_manifest_states() {
    let (_, repository, store, id) = repository("total-drift");
    let stream = repository.join("runs/driven.jsonl");
    std::fs::create_dir_all(stream.parent().unwrap()).unwrap();
    std::fs::copy(root().join(DRIVEN_STREAM), &stream).unwrap();

    // What `aep eval run` puts in this run's manifest as `tokens`.
    let ended: Vec<serde_json::Value> = std::fs::read_to_string(&stream)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|value| value["event"] == "session.ended")
        .collect();
    let mut refusals = Vec::new();
    let manifest_tokens = aep_cli::eval::totals_of(&[ended], &mut refusals).tokens;
    assert_eq!(refusals, Vec::new(), "the recorded stream reads");
    assert_eq!(manifest_tokens, Some(34 + 1_310 + 97_998 + 17_204));

    let before = evidence_files(&repository);
    let output = spend(&store, &id, &stream);
    assert!(output.status.success(), "{}", text(&output));
    let change = written_record(&repository, &before).expect("one record");
    let record: serde_json::Value =
        serde_json::from_str(change["source"].as_str().unwrap()).unwrap();

    assert_eq!(
        record["total_tokens"].as_u64(),
        manifest_tokens,
        "one run's total_tokens differs by source: {record}"
    );
}
