//! `protocol eval run` — the gates before a cent is spent, and what the manifest is read out of.
//!
//! # The binary is a tool, and a machine without it is not a red gate
//!
//! The evaluation programme's fourth design constant says it plainly: `metaharness` on `PATH` is a
//! *tool* dependency of the eval runner, like `git`, and an absent binary is a **skip by name**.
//! Every test here that would need it either skips and says so, or proves the refusal instead — and
//! the refusal is what a machine without the binary sees, so the two directions cover each other.
//! Nothing in this file calls a model, and nothing in it can spend money: the one test that walks
//! the whole spawn path points `METAHARNESS_BIN` at a shell script this file writes.
//!
//! # What the streams are
//!
//! Two of the fixtures are the committed eval corpus's own transcripts, ingested unchanged. Three
//! are under `crates/protocol-cli/fixtures/eval-run/`, derived from them — the derivation and what
//! is and is not observed about them is in that directory's `README.md`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args`, always from the repository root.
fn protocol(args: &[&str]) -> Output {
    protocol_with(args, &[])
}

/// Runs `protocol` with `args` and the environment given.
fn protocol_with(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_protocol"));
    command.args(args).current_dir(root());
    // Removed rather than left alone: a developer who exported it for a paid sweep must not turn
    // this suite into one, and a test that asserts the *absence* of the flag has to control it.
    command.env_remove("METAHARNESS_LIVE");
    command.env_remove("METAHARNESS_BIN");
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the protocol binary runs")
}

/// Standard output as a string.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Standard error as a string.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The exit code, which is part of the contract with a calling harness.
fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to leave a run's products in.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// The honest development case: a run that did the two things in the order the workflow requires.
const HONEST_CASE: &str = "conformance/eval/development-honest";

/// The violation case beside it, judged by the same document.
const VIOLATION_CASE: &str = "conformance/eval/development-tests-after-the-code";

/// The violation case's own transcript, whose instrument row is empty: nothing was injected.
const NO_PLUGIN_STREAM: &str = "conformance/eval/development-tests-after-the-code/transcript.jsonl";

/// The honest case's own transcript, whose instrument row names a plugin and no digest for it.
///
/// The corpus predates the digest attestation, which is what makes it the honest fixture for
/// `EVAL-STREAM-008` rather than a mutation somebody wrote to produce one.
const UNDIGESTED_STREAM: &str = "conformance/eval/development-honest/transcript.jsonl";

/// A stream whose `session.started` attests the installed plugin with a source **and** a digest.
const ATTESTED_STREAM: &str = "crates/protocol-cli/fixtures/eval-run/claude-plugin-attested.jsonl";

/// The same, for Codex — and its terminal event prices nothing.
const CODEX_STREAM: &str = "crates/protocol-cli/fixtures/eval-run/codex-plugin-attested.jsonl";

/// Ingests one recorded stream as one arm of one case, spending nothing.
fn ingest(out: &Path, case: &str, arm: &str, harness: &str, stream: &str) -> Output {
    protocol(&[
        "eval",
        "run",
        "--case",
        case,
        "--arm",
        arm,
        "--harness",
        harness,
        "--stream",
        stream,
        "--observed-at",
        "2026-08-23",
        "--out",
        printable(out),
        "--redact",
    ])
}

/// The manifest one run left behind.
fn manifest(out: &Path, name: &str) -> String {
    std::fs::read_to_string(out.join(format!("{name}.manifest.yaml")))
        .unwrap_or_else(|error| panic!("{name} left a manifest: {error}"))
}

// --- crossing #4, on this side ---------------------------------------------------------------

#[test]
fn the_plugin_digest_in_the_manifest_is_the_one_the_session_attested_byte_for_byte() {
    // **The crossing-#4 golden.** metaharness's `--plugin-dir` copies a plugin into the scratch
    // home and attests what it installed in `session.started`; the manifest's `plugin_digest` is
    // that string and nothing else — not a hash of the directory on disk, which would attest bytes
    // the session never saw.
    //
    // The expected value is read out of the fixture rather than typed here, so the two cannot
    // drift: an edited plugin produces a different attested digest, and this test follows it.
    let attested = {
        let stream = std::fs::read_to_string(root().join(ATTESTED_STREAM))
            .expect("the attested fixture is readable");
        let first = stream.lines().next().expect("the stream opens a session");
        let value: serde_json::Value = serde_json::from_str(first).expect("it is JSON");
        // `hermetic.installed_plugins`, the **instrument's** row — not the top-level `plugins`
        // echo beside it, which is whatever the vendor happened to say and is `null` on Codex.
        // Reading the wrong one of the two cost the first live pilot run two refusals.
        value["hermetic"]["installed_plugins"][0]["digest"]
            .as_str()
            .expect("the attestation carries a digest")
            .to_owned()
    };

    let out = scratch("aep-eval-run-attested-digest");
    let ingested = ingest(&out, HONEST_CASE, "plugin", "claude", ATTESTED_STREAM);
    assert_eq!(code(&ingested), 0, "{}", stderr(&ingested));

    let written = manifest(&out, "claude-plugin-development-honest");
    assert!(
        written.contains(&format!("plugin_digest: {attested}")),
        "the manifest carries the attested digest verbatim:\n{written}"
    );
    assert!(
        written.contains("arm: plugin") && written.contains("harness: claude"),
        "beside the two fields only the runner knows:\n{written}"
    );
}

#[test]
fn arm_plugin_over_a_stream_that_attests_no_plugin_is_refused_by_name() {
    // The treated arm without its treatment. Without this refusal a run that lost its
    // `--plugin-dir` — which is exactly how run `W4-2` lost eight sessions — would enter the matrix
    // as a measurement of the plugin and be a measurement of nothing.
    let out = scratch("aep-eval-run-plugin-without-a-plugin");
    let refused = ingest(&out, VIOLATION_CASE, "plugin", "claude", NO_PLUGIN_STREAM);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-006") && reason.contains("without its treatment"),
        "{reason}"
    );
    assert!(
        std::fs::read_dir(&out)
            .expect("the scratch tree is readable")
            .next()
            .is_none(),
        "and no manifest exists for anyone to read as a measurement"
    );
}

#[test]
fn arm_raw_over_a_stream_that_attests_a_plugin_is_refused_by_name() {
    // The other direction, and the one the manifest reader already refuses one layer out
    // (`EVAL-MANIFEST-005`): the control arm with the treatment applied. Refused here as well
    // because here is where it is still a *run* rather than a document nobody can re-derive.
    let out = scratch("aep-eval-run-raw-with-a-plugin");
    let refused = ingest(&out, HONEST_CASE, "raw", "claude", ATTESTED_STREAM);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-007") && reason.contains("control arm"),
        "{reason}"
    );
}

#[test]
fn an_attested_plugin_with_no_digest_is_refused_because_the_manifest_cannot_say_which_bytes() {
    // The corpus's own honest transcript predates the digest attestation: it names a plugin and
    // says nothing about its bytes. That is not a manifest with a hole in it — it is refused, and
    // the sentence says why an edited plugin would otherwise be indistinguishable from the shipped
    // one.
    let out = scratch("aep-eval-run-undigested-plugin");
    let refused = ingest(&out, HONEST_CASE, "plugin", "claude", UNDIGESTED_STREAM);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-STREAM-008"),
        "{}",
        stderr(&refused)
    );
}

// --- the rest of what the stream has to state --------------------------------------------------

#[test]
fn a_stream_of_another_harness_than_the_run_claims_is_refused() {
    let out = scratch("aep-eval-run-wrong-harness");
    let refused = ingest(&out, HONEST_CASE, "plugin", "claude", CODEX_STREAM);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-005") && reason.contains("`codex`"),
        "the refusal quotes what the stream says: {reason}"
    );
}

#[test]
fn a_session_that_states_no_harness_version_is_refused_and_no_manifest_is_written() {
    // The fail-closed rule decision R3.2 turns on: the manifest's version is *read out of* the
    // stream, so a stream that does not state one leaves no manifest at all. The one-line mutation
    // this guards is a reader that filled the hole with an empty string — which would produce a
    // manifest the matrix reads happily and a row nobody can join a harness release to.
    let out = scratch("aep-eval-run-versionless");
    let stream = out.join("versionless.jsonl");
    let honest = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let mutated = honest.replacen("\"harness_version\":\"2.1.239\",", "", 1);
    assert_ne!(mutated, honest, "the mutation reached the session record");
    std::fs::write(&stream, mutated).expect("the scratch tree is writable");

    let refused = ingest(&out, HONEST_CASE, "plugin", "claude", printable(&stream));
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-004") && reason.contains("`harness_version`"),
        "{reason}"
    );
    assert!(
        !out.join("claude-plugin-development-honest.manifest.yaml")
            .exists(),
        "and nothing was written"
    );
}

#[test]
fn a_stream_that_stops_before_the_session_ends_is_refused_rather_than_reported_as_a_whole_run() {
    let out = scratch("aep-eval-run-truncated");
    let stream = out.join("truncated.jsonl");
    let whole = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let lines: Vec<&str> = whole.lines().collect();
    std::fs::write(
        &stream,
        format!("{}\n", lines[..lines.len() - 1].join("\n")),
    )
    .expect("the scratch tree is writable");

    let refused = ingest(&out, HONEST_CASE, "plugin", "claude", printable(&stream));
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-STREAM-010"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn a_wire_that_names_no_model_assembles_a_manifest_that_says_so() {
    // **The first live pilot run's second refusal, fixed.** Codex states no model at
    // `session.started` — the whole of a 62-event run never states one — so the honest manifest
    // writes `model: null`. Inventing `gpt-5-codex` there because it is the likely answer would be
    // writing the one document the matrix trusts, and the matrix would carry a model nobody
    // observed.
    let out = scratch("aep-eval-run-unstated-model");
    let ingested = ingest(&out, HONEST_CASE, "plugin", "codex", CODEX_STREAM);
    assert_eq!(code(&ingested), 0, "{}", stderr(&ingested));
    assert!(
        stdout(&ingested).contains("model:    (unstated)"),
        "and a person reading the runner is told, in words no model is named: {}",
        stdout(&ingested)
    );

    let written = manifest(&out, "codex-plugin-development-honest");
    assert!(
        written.contains("model: null"),
        "the key is written and says null, never omitted:\n{written}"
    );
}

#[test]
fn a_session_that_omits_the_model_key_altogether_is_still_refused() {
    // The boundary the fix above must not have erased. *The harness did not say* and *nobody wrote
    // the key down* are different findings, and only the first is a run this verb can describe —
    // the one-line mutation this guards is a reader that answered `None` for both.
    let out = scratch("aep-eval-run-modelless");
    let stream = out.join("modelless.jsonl");
    let codex = std::fs::read_to_string(root().join(CODEX_STREAM)).expect("readable");
    let mutated = codex.replacen("\"model\":null,", "", 1);
    assert_ne!(mutated, codex, "the mutation reached the session record");
    std::fs::write(&stream, mutated).expect("the scratch tree is writable");

    let refused = ingest(&out, HONEST_CASE, "plugin", "codex", printable(&stream));
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-004") && reason.contains("`model`"),
        "{reason}"
    );
    assert!(
        !out.join("codex-plugin-development-honest.manifest.yaml")
            .exists(),
        "and nothing was written"
    );
}

#[test]
fn the_digest_is_read_from_the_instruments_row_and_not_from_the_vendors_echo() {
    // **The first live pilot run's other refusal, fixed.** `session.started` carries two lists and
    // they answer different questions: top-level `plugins` is the vendor's own init list, echoed —
    // Codex writes `null` there because its vendor states nothing and metaharness will not mint a
    // field it did not receive — and `hermetic.installed_plugins` is what the **instrument**
    // injected, written on every adapter.
    //
    // Asserted by taking the vendor echo away entirely: the manifest must be unchanged, because
    // nothing here reads it. A reader that fell back to the echo would pass every other test in
    // this file and fail this one.
    let out = scratch("aep-eval-run-vendor-echo");
    let honest = ingest(&out, HONEST_CASE, "plugin", "claude", ATTESTED_STREAM);
    assert_eq!(code(&honest), 0, "{}", stderr(&honest));
    let expected = manifest(&out, "claude-plugin-development-honest");

    let stream = out.join("no-vendor-echo.jsonl");
    let attested = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let mutated = attested.replacen(
        "\"plugins\":[{\"name\":\"engineering-protocols\"",
        "\"plugins\":null,\"ignored\":[{\"name\":\"engineering-protocols\"",
        1,
    );
    assert_ne!(mutated, attested, "the mutation reached the vendor echo");
    std::fs::write(&stream, mutated).expect("the scratch tree is writable");

    let without = ingest(&out, HONEST_CASE, "plugin", "claude", printable(&stream));
    assert_eq!(
        code(&without),
        0,
        "a stream whose vendor says nothing is a perfectly good run: {}",
        stderr(&without)
    );
    let after = manifest(&out, "claude-plugin-development-honest");
    assert_eq!(
        after
            .lines()
            .filter(|line| !line.starts_with("transcript_digest"))
            .collect::<Vec<_>>(),
        expected
            .lines()
            .filter(|line| !line.starts_with("transcript_digest"))
            .collect::<Vec<_>>(),
        "removing the vendor echo changes nothing but the bytes' own digest"
    );
}

#[test]
fn a_session_whose_hermetic_row_is_missing_is_refused_by_the_field_that_is_missing() {
    // The instrument's row is *always* written, on every adapter — so a stream without it is not a
    // run with no plugin, it is a stream this reader cannot place. Refused by the name of the row
    // it looked for, so the next reader knows which of the two lists is the one that matters.
    let out = scratch("aep-eval-run-no-hermetic-row");
    let stream = out.join("no-instrument-row.jsonl");
    let attested = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let mut lines: Vec<String> = attested.lines().map(ToOwned::to_owned).collect();
    let mut first: serde_json::Value =
        serde_json::from_str(&lines[0]).expect("the stream opens with a session");
    let removed = first["hermetic"]
        .as_object_mut()
        .expect("the session declares a hermetic record")
        .remove("installed_plugins");
    assert!(
        removed.is_some(),
        "the mutation reached the instrument's row"
    );
    lines[0] = serde_json::to_string(&first).expect("it serialises");
    std::fs::write(&stream, format!("{}\n", lines.join("\n")))
        .expect("the scratch tree is writable");

    let refused = ingest(&out, HONEST_CASE, "plugin", "claude", printable(&stream));
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-004") && reason.contains("hermetic.installed_plugins"),
        "{reason}"
    );
}

#[test]
fn a_cost_the_wire_writes_as_null_leaves_the_manifest_silent_and_never_says_zero() {
    // The polarity of the matrix's resource columns, one layer upstream of them. A Codex stream
    // prices nothing; a runner that wrote `cost_micro_usd: 0` would make that run look free, and
    // the cell's total would be a total over runs it does not cover.
    let out = scratch("aep-eval-run-unpriced");
    let ingested = ingest(&out, HONEST_CASE, "plugin", "codex", CODEX_STREAM);
    assert_eq!(code(&ingested), 0, "{}", stderr(&ingested));

    let written = manifest(&out, "codex-plugin-development-honest");
    assert!(
        !written.contains("cost_micro_usd"),
        "an unpriced run states no cost at all:\n{written}"
    );
    assert!(
        written.contains("tokens: ") && written.contains("wall_time_ms: "),
        "and the quantities the same event did state are there:\n{written}"
    );
}

#[test]
fn a_run_of_arm_driven_is_read_even_though_it_is_not_launched_here() {
    // The split this verb makes: `protocol drive run` launches a driven run, and this reads the
    // stream it wrote. Arm `driven` may carry a plugin digest or not — what enforces it is the
    // driver at the seam — so the manifest is written either way.
    let out = scratch("aep-eval-run-driven-ingest");
    let ingested = ingest(
        &out,
        HONEST_CASE,
        "driven",
        "claude",
        "crates/protocol-cli/fixtures/eval-run/claude-driven-attested.jsonl",
    );
    assert_eq!(code(&ingested), 0, "{}", stderr(&ingested));
    assert!(manifest(&out, "claude-driven-development-honest").contains("arm: driven"));
}

#[test]
fn one_recorded_stream_is_one_run_and_naming_two_cases_is_refused() {
    let out = scratch("aep-eval-run-two-cases-one-stream");
    let refused = protocol(&[
        "eval",
        "run",
        "--case",
        HONEST_CASE,
        "--case",
        VIOLATION_CASE,
        "--arm",
        "plugin",
        "--harness",
        "claude",
        "--stream",
        ATTESTED_STREAM,
        "--observed-at",
        "2026-08-23",
        "--out",
        printable(&out),
    ]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-008"),
        "{}",
        stderr(&refused)
    );
}

#[test]
fn naming_neither_a_case_nor_a_workflow_is_refused_rather_than_running_the_whole_corpus() {
    let out = scratch("aep-eval-run-no-case");
    let refused = protocol(&[
        "eval",
        "run",
        "--arm",
        "raw",
        "--harness",
        "claude",
        "--observed-at",
        "2026-08-23",
        "--out",
        printable(&out),
    ]);
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-007") && stderr(&refused).contains("bill nobody asked"),
        "{}",
        stderr(&refused)
    );
}

// --- the gates before a spawn ------------------------------------------------------------------

/// Whether the tool this verb drives is installed on this machine.
fn tool_installed() -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join("metaharness").is_file())
    })
}

/// The arguments of a spawn — no `--stream`, so every gate below is reached.
fn spawn_args<'a>(out: &'a Path, cwd: &'a Path, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "eval",
        "run",
        "--case",
        HONEST_CASE,
        "--harness",
        "claude",
        "--observed-at",
        "2026-08-23",
        "--cwd",
        printable(cwd),
        "--out",
        printable(out),
        "--redact",
    ];
    args.extend_from_slice(extra);
    args
}

#[test]
fn without_the_binary_the_runner_refuses_by_name_and_exits_two() {
    // Design constant 4: an absent binary is a skip, never a red gate. This asserts the other half
    // of that — what a machine without it is *told* — and skips by name where the binary is
    // installed, because there the refusal cannot be reached at all.
    if tool_installed() {
        eprintln!(
            "skipped by name: `metaharness` is on PATH here, so the missing-binary refusal is \
             unreachable. The gate is green either way — that is what this skip is for."
        );
        return;
    }
    let out = scratch("aep-eval-run-no-binary");
    let cwd = scratch("aep-eval-run-no-binary-tree");
    let refused = protocol(&spawn_args(&out, &cwd, &["--arm", "raw"]));
    assert_eq!(
        code(&refused),
        2,
        "the tool being absent has its own exit code: {}",
        stderr(&refused)
    );
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-001") && reason.contains("drives it as a tool"),
        "{reason}"
    );
    assert!(
        reason.contains("--stream"),
        "and the refusal names what needs no binary: {reason}"
    );
}

/// A stand-in for the tool, which records what it was called with and prints a canned stream.
///
/// This is what lets the whole spawn path — argv, capture, ingest, budget — be proven for nothing.
/// It is a `sh` script rather than a mock inside the binary on purpose: a mock would test a seam
/// this verb does not have, and the thing being asserted is that a **process** is started with
/// those arguments.
#[cfg(unix)]
fn stub(directory: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let path = directory.join("metaharness-stub");
    // Each argument is written whole and followed by a marker line, because one of them is a
    // multi-line prompt: a stub that separated arguments by newlines would make *the last argument*
    // unrecoverable, which is exactly the one this file asserts on.
    std::fs::write(
        &path,
        "#!/bin/sh\n: > \"$STUB_ARGV\"\nfor word in \"$@\"; do\n  printf '%s\\n%s\\n' \"$word\" \"$ARGV_MARKER\" >> \"$STUB_ARGV\"\ndone\ncat \"$STUB_STREAM\"\n",
    )
    .expect("the scratch tree is writable");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the stub can be made executable");
    path
}

#[cfg(unix)]
#[test]
fn a_spawn_without_the_live_flag_is_refused_by_name_and_nothing_is_started() {
    // The tool is present — the stub is right there — and the runner still refuses, which is the
    // point: *installed* is not *permitted to spend*.
    let out = scratch("aep-eval-run-not-live");
    let cwd = scratch("aep-eval-run-not-live-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "raw", "--budget-usd", "1.00"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-002"),
        "{}",
        stderr(&refused)
    );
    assert!(
        !argv.exists(),
        "and the tool was never started: it would have written its arguments here"
    );
}

#[cfg(unix)]
#[test]
fn a_spawn_with_no_cap_on_what_it_may_spend_is_refused_by_name() {
    let out = scratch("aep-eval-run-no-budget");
    let cwd = scratch("aep-eval-run-no-budget-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "raw"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-003"),
        "{}",
        stderr(&refused)
    );
    assert!(!argv.exists(), "and nothing was started");
}

#[cfg(unix)]
#[test]
fn arm_driven_is_not_launched_here_and_the_refusal_names_the_verb_that_does() {
    // A second way to launch a driven session would be a second policy to forget, which is the
    // mistake `epic:metaharness-migration` retired. The refusal is the design, not a gap.
    let out = scratch("aep-eval-run-driven-spawn");
    let cwd = scratch("aep-eval-run-driven-spawn-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "driven", "--budget-usd", "1.00"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-004") && reason.contains("protocol drive run"),
        "the refusal names the verb that does launch one: {reason}"
    );
    assert!(
        reason.contains("--arm driven --stream"),
        "and how a driven run reaches the matrix: {reason}"
    );
    assert!(!argv.exists(), "and nothing was started");
}

/// What the stub was called with, one whole argument per entry.
///
/// Split on the marker the stub writes, because one of the arguments is a multi-line prompt and
/// this file asserts on exactly that one.
#[cfg(unix)]
fn recorded_argv(path: &Path) -> Vec<String> {
    let recorded = std::fs::read_to_string(path).expect("the stub recorded its arguments");
    recorded
        .split(&format!("\n{ARGV_MARKER}\n"))
        .map(ToOwned::to_owned)
        .filter(|word| !word.is_empty())
        .collect()
}

/// The line the stub writes between two arguments.
#[cfg(unix)]
const ARGV_MARKER: &str = "<<<one argument ended>>>";

/// The environment a stubbed spawn runs in.
#[cfg(unix)]
fn stub_env(binary: &Path, argv: &Path, stream: &str) -> Vec<(String, String)> {
    vec![
        ("METAHARNESS_BIN".to_owned(), printable(binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        (
            "STUB_STREAM".to_owned(),
            root().join(stream).display().to_string(),
        ),
    ]
}

/// Borrows an owned environment for [`protocol_with`].
#[cfg(unix)]
fn as_pairs(owned: &[(String, String)]) -> Vec<(&str, &str)> {
    owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
}

#[cfg(unix)]
#[test]
fn a_spawn_gives_arm_raw_the_committed_instructions_and_arm_plugin_the_plugin() {
    // The two treatments, asserted on the argv and the prompt a real process was started with.
    // They are deliberately different: arm a is *text and hope* — the workflow's rendered
    // instructions in front of the task — and arm b's treatment **is** the plugin, so giving it the
    // instructions too would measure both and attribute the result to b.
    let out = scratch("aep-eval-run-treatments");
    let cwd = scratch("aep-eval-run-treatments-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    // --- arm raw ---------------------------------------------------------------------------
    // The violation case, because its own transcript is the one that attests no plugin — and arm
    // `raw` over a stream attesting one is refused, which is the point of the sibling test.
    let owned = stub_env(&binary, &argv, NO_PLUGIN_STREAM);
    let mut raw_args = spawn_args(&out, &cwd, &["--arm", "raw", "--budget-usd", "1.00"]);
    raw_args[3] = VIOLATION_CASE;
    let spawned = protocol_with(&raw_args, &as_pairs(&owned));
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let words = recorded_argv(&argv);
    assert_eq!(
        &words[..4],
        &["run", "claude", "--hermetic", "--cwd"],
        "the instrument is the same for every arm: {words:?}"
    );
    assert!(
        words
            .windows(2)
            .any(|pair| pair == ["--decisions", "observe"]),
        "arms a and b record everything and decide nothing: {words:?}"
    );
    assert!(
        !words.iter().any(|word| word == "--plugin-dir"),
        "and arm a is the arm with no plugin in it: {words:?}"
    );
    let prompt = words.last().expect("the prompt is the last argument");
    assert!(
        // The document, not the version. What this row asserts is *arm a is given the rendered
        // instructions and not the raw workflow*; spelling the version out made it a second,
        // unstated assertion that the workflow never changes, which `adp/default/2` then broke
        // without anything being wrong.
        prompt.starts_with("<!-- Rendered from `adp/default/")
            && prompt.contains("` by `protocol workflow instruct`"),
        "arm a's treatment is the committed instruction document, in front of the task: {prompt}"
    );
    assert!(
        prompt.contains("Work this through the development workflow"),
        "with the case's own task after it: {prompt}"
    );

    // --- arm plugin -------------------------------------------------------------------------
    let owned = stub_env(&binary, &argv, ATTESTED_STREAM);
    let spawned = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "plugin", "--budget-usd", "1.00"]),
        &as_pairs(&owned),
    );
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let words = recorded_argv(&argv);
    assert!(
        words
            .windows(2)
            .any(|pair| pair == ["--plugin-dir", "integrations/claude-code"]),
        "arm b's treatment is the shipped plugin: {words:?}"
    );
    let prompt = words.last().expect("the prompt is the last argument");
    assert!(
        !prompt.contains("Rendered from"),
        "and arm b gets the task alone, or the two arms would measure the same thing: {prompt}"
    );

    // And the run left the pair the matrix reads.
    assert!(out
        .join("claude-plugin-development-honest.manifest.yaml")
        .exists());
    assert!(out
        .join("claude-plugin-development-honest.report.json")
        .exists());
    assert!(
        out.join("claude-plugin-development-honest.events.jsonl")
            .exists(),
        "beside the stream it was judged over, which is what makes the pair re-derivable"
    );
}

/// The attested claude stream with its terminal cost replaced, written into a scratch file.
///
/// Whatever a fixture happens to cost is not what the ledger has to be tested against: the two
/// answers the ledger distinguishes are *the wire stated a cost* and *the wire stated none*, and a
/// test has to hand it each.
#[cfg(unix)]
fn stream_costing(directory: &Path, name: &str, cost: &str) -> PathBuf {
    let attested = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let mut lines: Vec<String> = attested.lines().map(ToOwned::to_owned).collect();
    let last = lines.len() - 1;
    let mut ended: serde_json::Value =
        serde_json::from_str(&lines[last]).expect("the stream ends a session");
    assert_eq!(
        ended["event"], "session.ended",
        "the last line is the terminal event"
    );
    ended["total_cost_usd"] =
        serde_json::from_str(cost).expect("the cost is JSON — a number or `null`");
    lines[last] = serde_json::to_string(&ended).expect("it serialises");
    let path = directory.join(name);
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("the scratch tree is writable");
    path
}

#[cfg(unix)]
#[test]
fn a_run_whose_stream_states_a_cost_is_charged_that_cost_and_never_the_assumption() {
    // **The live defect, in the place it was visible.** A Claude run stated
    // `0.7977854999999999` — the shortest text that round-trips the `f64` sum of its per-turn
    // costs — and the ledger printed `$0.250000 spent`, because the cost reader refused seventeen
    // significant figures and `.ok()` turned the refusal into *this run stated no cost*. Eighty
    // cents were charged as twenty-five, and the manifest carried no cost at all, so the matrix
    // would have under-reported the sweep as well.
    //
    // Both halves are asserted here: what the ledger charged, and what the manifest kept.
    let out = scratch("aep-eval-run-ledger-states-a-cost");
    let cwd = scratch("aep-eval-run-ledger-states-a-cost-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = stream_costing(&out, "priced.jsonl", "0.7977854999999999");

    let owned = vec![
        ("METAHARNESS_BIN".to_owned(), printable(&binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(&argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        ("STUB_STREAM".to_owned(), printable(&stream).to_owned()),
    ];
    let spawned = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "plugin", "--budget-usd", "5.00"]),
        &as_pairs(&owned),
    );
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let said = stdout(&spawned);
    assert!(
        said.contains("charged:  $0.797785 (stated)"),
        "the run is charged what its stream stated, and the line says which of the two numbers \
         the ledger used: {said}"
    );
    assert!(
        said.contains("1 run(s), $0.797785 spent against a cap of $5.000000"),
        "and the sweep's total is that cost, not the assumed rate: {said}"
    );
    assert!(
        !said.contains("$0.250000"),
        "the assumption is nowhere near a run that priced itself: {said}"
    );

    let written = manifest(&out, "claude-plugin-development-honest");
    assert!(
        written.contains("cost_micro_usd: 797785"),
        "and the manifest keeps it, so the matrix reports what the sweep actually spent:\n{written}"
    );
}

#[cfg(unix)]
#[test]
fn the_assumed_rate_is_charged_only_where_the_stream_priced_nothing() {
    // The other side of the boundary, without which the test above would pass against a ledger
    // that had simply stopped consulting the assumption at all.
    let out = scratch("aep-eval-run-ledger-states-nothing");
    let cwd = scratch("aep-eval-run-ledger-states-nothing-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = stream_costing(&out, "unpriced.jsonl", "null");

    let owned = vec![
        ("METAHARNESS_BIN".to_owned(), printable(&binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(&argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        ("STUB_STREAM".to_owned(), printable(&stream).to_owned()),
    ];
    let spawned = protocol_with(
        &spawn_args(
            &out,
            &cwd,
            &[
                "--arm",
                "plugin",
                "--budget-usd",
                "5.00",
                "--assume-usd-per-run",
                "0.40",
            ],
        ),
        &as_pairs(&owned),
    );
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let said = stdout(&spawned);
    assert!(
        said.contains("charged:  $0.400000 (assumed)"),
        "a wire that priced nothing is charged the assumed rate, and said to be: {said}"
    );
    let written = manifest(&out, "claude-plugin-development-honest");
    assert!(
        !written.contains("cost_micro_usd"),
        "and the assumption never reaches the manifest, where it would become a measurement:\n\
         {written}"
    );
}

#[cfg(unix)]
#[test]
fn a_stated_cost_this_reader_cannot_convert_stops_the_run_instead_of_becoming_an_estimate() {
    // The failure mode that made the defect silent, refused where it enters. Charging an
    // unreadable cost at the assumed rate is how a sweep under-reports what it spent, and it
    // leaves nothing behind for anyone to notice.
    let out = scratch("aep-eval-run-ledger-unreadable");
    let cwd = scratch("aep-eval-run-ledger-unreadable-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = stream_costing(&out, "unreadable.jsonl", "\"0.80\"");

    let owned = vec![
        ("METAHARNESS_BIN".to_owned(), printable(&binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(&argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        ("STUB_STREAM".to_owned(), printable(&stream).to_owned()),
    ];
    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "plugin", "--budget-usd", "5.00"]),
        &as_pairs(&owned),
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-011") && reason.contains("under-reports"),
        "{reason}"
    );
    assert!(
        !out.join("claude-plugin-development-honest.manifest.yaml")
            .exists(),
        "and no manifest claims a run whose cost nobody could read"
    );
}

#[cfg(unix)]
#[test]
fn the_cap_stops_the_sweep_before_the_run_that_would_pass_it() {
    // Checked before the spawn and against the assumed rate, because the only number available
    // before a run is the assumed one — a cap enforced afterwards is a receipt. The stub's stream
    // costs $0.5216, the cap is $0.60 and an unpriced run is counted at $0.25, so the first run
    // fits and the second cannot.
    let out = scratch("aep-eval-run-budget");
    let cwd = scratch("aep-eval-run-budget-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = root().join(ATTESTED_STREAM).display().to_string();

    let stopped = protocol_with(
        &[
            "eval",
            "run",
            "--case",
            HONEST_CASE,
            "--case",
            VIOLATION_CASE,
            "--arm",
            "plugin",
            "--harness",
            "claude",
            "--observed-at",
            "2026-08-23",
            "--cwd",
            printable(&cwd),
            "--out",
            printable(&out),
            "--budget-usd",
            "0.60",
            "--redact",
        ],
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
            ("ARGV_MARKER", ARGV_MARKER),
            ("STUB_STREAM", &stream),
        ],
    );
    assert_eq!(code(&stopped), 0, "{}", stderr(&stopped));
    let said = stdout(&stopped);
    assert!(
        said.contains("EVAL-RUN-006") && said.contains("1 run(s), with 1 not launched"),
        "the stop is reported by name, with what was and was not run: {said}"
    );
    assert!(
        said.contains("$0.521600") && said.contains("$0.600000"),
        "and with the numbers it decided on: {said}"
    );
    assert!(
        out.join("claude-plugin-development-honest.manifest.yaml")
            .exists(),
        "the run that fit happened"
    );
    assert!(
        !out.join("claude-plugin-development-tests-after-the-code.manifest.yaml")
            .exists(),
        "and the one that did not fit was never started"
    );
}
