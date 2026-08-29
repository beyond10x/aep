//! `protocol drive` integration tests.
//!
//! These drive the real binary against a real directory, because that is what the verb family is: a
//! run is a lock file, a run directory, a program that was spawned and a snapshot on disk. A test
//! that called the library would not catch a lock taken after the run id was allocated, a flag that
//! never reaches the driver, or a report that summarised the engine instead of quoting it.
//!
//! The document tree is **this repository's own** — `protocols/`, `workflows/`, `profiles/`,
//! `principles/`, `artifacts/lifecycles/` — and the step map is a fixture, never
//! `drivers/development/default.yaml`. That map's command steps run `cargo test --workspace`, and a
//! test that ran it would be a test that ran itself.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// Runs `protocol` with `args` from the repository root.
fn protocol(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
}

/// Runs one verb with a `PATH` that holds nothing, so no machine can have `metaharness` on it.
///
/// An empty directory rather than `/usr/bin:/bin`: the point is a *guaranteed* absence, and a
/// machine that happened to install the binary into a system directory would otherwise turn this
/// test green for the wrong reason.
fn protocol_without_metaharness(args: &[&str], empty_dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_protocol"))
        .args(args)
        .current_dir(root())
        .env("PATH", empty_dir)
        .output()
        .expect("the protocol binary runs")
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

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// This machine's name, read the way the driver reads it.
fn host() -> String {
    for path in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(name) = std::fs::read_to_string(path) {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_owned();
            }
        }
    }
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_owned())
}

/// A project with a planning store, a task and a step map, built from scratch.
struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    /// Builds the fixture. `operator` puts an `operator` step at the head of `verify`.
    fn new(name: &str, operator: bool) -> Self {
        let directory = std::env::temp_dir().join(format!("protocol-drive-{name}"));
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("the temporary tree is writable");

        // The story, the specification of it, and a task that says which story it is: the shape a
        // driven run leaves behind, and the shape `spec-driven` now reads. Without the `specifies`
        // edge and the task's `derived_from` this store is the one run `NATIVE-1/1` walked — an
        // approved specification of nobody's work in particular — and the run stops at
        // `establish_verifiers -> implement`, which is the guard working rather than a defect here.
        write(
            &directory.join(".engineering/planning/story/passkeys.md"),
            "---\nformat: aep.planning-md/1\nid: story:passkeys\nkind: story\nstatus: active\n\
             title: Sign in with a passkey\nsummary: What the work is.\n---\n# Story\n\n\
             Signing in with a passkey replaces the password prompt.\n",
        );
        write(
            &directory.join(".engineering/planning/specification/passkeys.md"),
            "---\nformat: aep.planning-md/1\nid: specification:passkeys\nkind: specification\n\
             status: approved\ntitle: Passkey sign-in\nsummary: What signing in with a passkey \
             must do.\nrelations:\n- specifies: story:passkeys\n---\n# Specification\n\nThe \
             assertion is verified against the stored public key.\n",
        );
        write(
            &directory.join("task.yaml"),
            "id: DRIVE-1\nkind: feature\nobjective: drive-a-workflow\nprotocol: adp/1\n\
             profile: development.standard\nderived_from:\n  - story:passkeys\n",
        );
        write(&directory.join("steps.yaml"), &step_map(operator));

        Self { directory }
    }

    /// The arguments every verb needs.
    fn location(&self) -> Vec<String> {
        vec![
            "--project".to_owned(),
            printable(&self.directory).to_owned(),
            "--root".to_owned(),
            printable(&root()).to_owned(),
            "--task".to_owned(),
            printable(&self.directory.join("task.yaml")).to_owned(),
            "--map".to_owned(),
            printable(&self.directory.join("steps.yaml")).to_owned(),
        ]
    }

    /// Runs one `protocol drive` verb against this fixture.
    ///
    /// A `run` carries `--allow-evidence-gap`, and that is a statement about the fixture rather
    /// than about the flag. This map declares `test_result`, `diff` and `static_analysis` and
    /// nothing else, so F-W4.2-4's launch check refuses it: `spec-driven` wants a `specification`
    /// record and `provenance-tracking` an independent `verification` one, and no step here mints
    /// either. Every test below is about the routing loop, the lock or the report, none of which
    /// that gap changes — so the tests say *I know* rather than growing two steps that write
    /// evidence documents nobody reads. The refusal itself is tested on its own, without the flag,
    /// in `a_map_that_cannot_produce_demanded_evidence_is_refused_before_the_first_step`.
    fn drive(&self, verb: &[&str], extra: &[&str]) -> Output {
        let mut args: Vec<String> = vec!["drive".to_owned()];
        args.extend(verb.iter().map(ToString::to_string));
        args.extend(self.location());
        if verb == ["run"] {
            args.push("--allow-evidence-gap".to_owned());
        }
        args.extend(extra.iter().map(ToString::to_string));
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        protocol(&borrowed)
    }

    /// The `.engineering/runs` directory.
    fn runs(&self) -> PathBuf {
        self.directory.join(".engineering/runs")
    }

    /// The cursor of one run.
    fn cursor(&self, run: &str) -> serde_json::Value {
        let (task, ordinal) = run.rsplit_once('/').expect("a run id");
        let path = self.runs().join(task).join(ordinal).join("cursor.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        serde_json::from_str(&text).expect("the cursor is JSON")
    }
}

/// The fixture step map: the same states as `adp/default`, with commands that cost nothing.
///
/// `sh -c "exit 0"` and `sh -c "exit 1"` are the two verdicts a command step can carry, which is
/// all an exit status has to say. The red run in `establish_verifiers` is deliberate: `test.exists`
/// is what the guard reads, and a test that passed before there was an implementation would be a
/// test of nothing.
///
/// `operator` puts an `operator` step at the head of `verify` — a state this run genuinely reaches,
/// so the pause is exercised rather than only the pre-flight refusal that names it.
fn step_map(operator: bool) -> String {
    let pause = if operator {
        "      - kind: operator\n        prompt: judge this change before the suites run\n"
    } else {
        ""
    };
    format!(
        "format: aep.driver-steps/1\n\
         id: fixture/drive\n\
         workflow: adp/default/2\n\
         states:\n\
        \x20 establish_verifiers:\n\
        \x20   steps:\n\
        \x20     - kind: command\n\
        \x20       description: the red suite\n\
        \x20       run: [sh, -c, \"exit 1\"]\n\
        \x20       evidence:\n\
        \x20         kind: test_result\n\
        \x20         suite: unit\n\
        \x20         verifier: test-runner\n\
        \x20 implement:\n\
        \x20   steps:\n\
        \x20     - kind: command\n\
        \x20       description: the working tree changed\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: diff\n\
        \x20         verifier: git\n\
        \x20 verify:\n\
        \x20   steps:\n\
         {pause}\
        \x20     - kind: command\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: test_result\n\
        \x20         suite: unit\n\
        \x20         verifier: test-runner\n\
        \x20     - kind: command\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: test_result\n\
        \x20         suite: contract\n\
        \x20         verifier: test-runner\n\
        \x20     - kind: command\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: static_analysis\n\
        \x20         verifier: static-analyzer\n"
    )
}

#[test]
fn every_verb_can_be_asked_for_help() {
    for verb in ["run", "status", "resume"] {
        let output = protocol(&["drive", verb, "--help"]);
        assert_eq!(code(&output), 0, "{}", stderr(&output));
        assert!(
            stdout(&output).contains("--project") || stdout(&output).contains("Usage"),
            "`drive {verb} --help` says nothing useful"
        );
    }
}

#[test]
fn a_run_advances_on_command_step_evidence_and_ends_with_the_engine_speaking() {
    let fixture = Fixture::new("advance", false);
    let output = fixture.drive(&["run"], &[]);

    let text = stdout(&output);
    // Two moves need no evidence — the workflow's first transitions are unguarded or read the
    // store — and the two after them are bought by command steps. The run must reach at least the
    // fourth.
    for movement in [
        "receive -> specify",
        "specify -> decompose",
        "decompose -> establish_verifiers",
        "establish_verifiers -> implement",
    ] {
        assert!(text.contains(movement), "no `{movement}` in:\n{text}");
    }
    assert!(
        text.contains("status     blocked") || text.contains("status     completed"),
        "a run ends by saying which of the two it is:\n{text}"
    );

    // The lock is released on every exit path the driver controls.
    assert!(
        !fixture.runs().join("lock.json").exists(),
        "the lock outlived the run that took it"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.runs().join("current"))
            .expect("a current pointer")
            .trim(),
        "DRIVE-1/1"
    );
}

#[test]
fn a_blocked_run_prints_the_engine_reasons_without_rewording_them() {
    let fixture = Fixture::new("blocked", false);
    let output = fixture.drive(&["run"], &[]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "a blocked run is the execution saying no");
    assert!(text.contains("blocked because:"), "{text}");

    // Verbatim across two surfaces: what the cursor recorded is what was printed, character for
    // character. A report that paraphrased a refusal would be a second, worse protocol.
    let cursor = fixture.cursor("DRIVE-1/1");
    let reasons = cursor["reasons"].as_array().expect("recorded reasons");
    assert!(!reasons.is_empty(), "a blocked run records why: {cursor}");
    for reason in reasons {
        let reason = reason.as_str().expect("a reason is a line");
        assert!(
            text.contains(reason),
            "the report reworded `{reason}`:\n{text}"
        );
    }
    assert_eq!(cursor["status"], "blocked");
}

#[test]
fn a_second_driver_is_refused_by_name_and_writes_nothing() {
    let fixture = Fixture::new("locked", false);
    std::fs::create_dir_all(fixture.runs()).expect("the runs directory is writable");
    // A live pid: this test process. The rule is liveness, never age — any age threshold has to
    // exceed the longest legitimate step, and the longest legitimate step is a person.
    write(
        &fixture.runs().join("lock.json"),
        &format!(
            "{{\"run\":\"DRIVE-1/7\",\"pid\":{},\"host\":\"{}\",\"driver\":\"the test\"}}\n",
            std::process::id(),
            host()
        ),
    );

    let output = fixture.drive(&["run"], &[]);
    let said = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(code(&output), 1);
    assert!(
        said.contains("DRIVE-1/7"),
        "the holder's run is not named:\n{said}"
    );
    assert!(
        said.contains(&std::process::id().to_string()),
        "the holder's pid is not named:\n{said}"
    );
    assert!(
        said.contains("--take-lock"),
        "a refusal names the routes out:\n{said}"
    );
    assert!(
        !fixture.runs().join("DRIVE-1").exists(),
        "a refused run allocated a directory anyway"
    );
}

#[test]
fn a_run_stopped_by_its_iteration_bound_resumes_where_it_stopped() {
    let fixture = Fixture::new("resume", false);
    let first = fixture.drive(&["run"], &["--max-iterations", "2"]);
    let opening = stdout(&first);
    assert!(opening.contains("run        DRIVE-1/1"), "{opening}");
    let before = fixture.cursor("DRIVE-1/1");
    assert!(
        !fixture.runs().join("lock.json").exists(),
        "a stopped run keeps the lock"
    );

    let second = fixture.drive(&["resume", "DRIVE-1/1"], &[]);
    let text = stdout(&second);
    assert!(
        text.contains("run        DRIVE-1/1"),
        "a resume continues the same run rather than allocating a new one:\n{text}"
    );
    let after = fixture.cursor("DRIVE-1/1");
    assert!(
        after["iterations"].as_u64().unwrap_or(0) > before["iterations"].as_u64().unwrap_or(0),
        "the resumed run did nothing: {before} then {after}"
    );
    assert_eq!(
        after["map_digest"], before["map_digest"],
        "a resume is pinned to the map it started under"
    );
    assert!(
        !fixture.runs().join("DRIVE-1/2").exists(),
        "a resume allocated a second run directory"
    );
}

/// The line the driver prints is a line that works, with nothing else on it.
///
/// **F-W4.2-4, answered.** A stopped run printed `resume with: protocol drive resume <run>` and
/// that command re-read none of `--map`, `--task`, `--pause-on-approval` or `--plugin-dir`: an
/// operator who typed exactly what they were told got a different run, or an error. It was found by
/// running W4-2 on 2026-08-24 and recorded as *the line as printed does not work*.
///
/// The run directory now remembers how it was launched. This drives the fixture with an explicit
/// map and task, stops it on the bound, and then resumes with **only** the run id — the literal
/// printed line — and asserts it continued the same run under the same map.
#[test]
fn the_resume_line_the_driver_prints_works_with_nothing_else_on_it() {
    let fixture = Fixture::new("bare-resume", false);
    let first = fixture.drive(&["run"], &["--max-iterations", "2"]);
    let opening = stdout(&first);
    assert!(
        opening.contains("resume with: protocol drive resume DRIVE-1/1"),
        "the driver printed the line this test is about:\n{opening}"
    );
    let before = fixture.cursor("DRIVE-1/1");

    // Exactly the printed line: the verb, the run id, and the project, which is the only thing a
    // person standing in the repository would not have to type.
    let bare = protocol(&[
        "drive",
        "resume",
        "DRIVE-1/1",
        "--project",
        printable(&fixture.directory),
    ]);
    let text = format!("{}{}", stdout(&bare), stderr(&bare));
    assert!(
        text.contains("run        DRIVE-1/1"),
        "the printed line continued the run it names:\n{text}"
    );
    let after = fixture.cursor("DRIVE-1/1");
    assert_eq!(
        after["map_digest"], before["map_digest"],
        "and under the same map, which the run remembered rather than being told again"
    );
    assert!(
        after["iterations"].as_u64().unwrap_or(0) > before["iterations"].as_u64().unwrap_or(0),
        "and it did something: {before} then {after}"
    );
    assert!(
        fixture.runs().join("DRIVE-1/1/launch.json").is_file(),
        "the run recorded how it was launched, which is what makes the short line true"
    );
}

/// A resumed run gets the budget the operator typed, not what is left of the run's lifetime.
///
/// **Also F-W4.2-4.** `--max-iterations` was compared against the cursor's lifetime count, so a run
/// that had already spent 25 iterations was `budget-exhausted` before evaluating anything: W4-2's
/// first resume returned `steps 0 run`, having done nothing, and no flag the operator could pass
/// would have changed it. The lifetime count stays in the cursor, because *how far did this run
/// get* is a real question; the bound is on the call.
#[test]
fn a_resume_gets_the_iterations_it_was_given_rather_than_what_the_run_has_left() {
    let fixture = Fixture::new("resume-budget", false);
    fixture.drive(&["run"], &["--max-iterations", "2"]);
    let before = fixture.cursor("DRIVE-1/1");
    let spent = before["iterations"].as_u64().unwrap_or(0);
    assert!(spent >= 2, "the first call spent its budget: {before}");

    // A budget *smaller* than what the run has already spent. Under the old rule this did nothing
    // at all; under the new one it is two more iterations.
    let second = fixture.drive(&["resume", "DRIVE-1/1"], &["--max-iterations", "2"]);
    let text = stdout(&second);
    let after = fixture.cursor("DRIVE-1/1");
    assert!(
        after["iterations"].as_u64().unwrap_or(0) > spent,
        "the resume ran: {before} then {after}\n{text}"
    );
    assert!(
        !text.contains("steps      0 run"),
        "a resume that evaluates nothing is the defect this test exists for:\n{text}"
    );
}

#[test]
fn a_headless_start_refuses_what_only_a_person_can_answer_and_the_flag_is_the_route_through() {
    let fixture = Fixture::new("operator", true);

    let refused = fixture.drive(&["run"], &[]);
    let said = stdout(&refused);
    assert_eq!(code(&refused), 1);
    assert!(
        said.contains("operator step"),
        "the refusal names what is owed:\n{said}"
    );
    assert!(
        said.contains("--pause-on-approval"),
        "the refusal names the route through:\n{said}"
    );
    assert!(
        !fixture.runs().join("lock.json").exists(),
        "a refused start left a lock behind"
    );

    let paused = fixture.drive(&["run"], &["--pause-on-approval"]);
    let text = stdout(&paused);
    assert_eq!(
        code(&paused),
        0,
        "with the flag, a green exit means finished or waiting:\n{text}"
    );
    assert!(
        text.contains("resume with: protocol drive resume DRIVE-1/1"),
        "a pause ends with the one word that continues it:\n{text}"
    );
}

#[test]
fn status_reports_the_run_and_whether_the_lock_is_free() {
    let fixture = Fixture::new("status", false);
    fixture.drive(&["run"], &[]);
    let output = fixture.drive(&["status"], &[]);
    let text = stdout(&output);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(text.contains("lock       free"), "{text}");
    assert!(text.contains("run        DRIVE-1/1"), "{text}");
    assert!(text.contains("map        fixture/drive"), "{text}");
}

#[test]
fn the_committed_step_map_loads_and_is_refused_when_a_state_is_renamed() {
    // The real map, cross-validated against the real workflow by the document loader.
    let output = protocol(&["validate", "--root", "."]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(
        stdout(&output).contains("2 step map(s)"),
        "both shipped maps load and cross-validate: `development/default` is the cargo one and \
         `development/checks` runs the checks a story carries\n{}",
        stdout(&output)
    );

    // And the negative: a map naming a state the workflow does not have is refused at load, with
    // the workflow's own states listed. A tree is built from scratch so the repository's own
    // `drivers/` is not what is being read.
    let tree = std::env::temp_dir().join("protocol-drive-tree");
    std::fs::remove_dir_all(&tree).ok();
    for directory in ["protocols", "workflows"] {
        std::fs::create_dir_all(tree.join(directory)).expect("the temporary tree is writable");
    }
    std::fs::copy(
        root().join("protocols/adp/1.yaml"),
        tree.join("protocols/adp.yaml"),
    )
    .expect("the protocol is readable");
    std::fs::copy(
        root().join("protocols/aep/1.yaml"),
        tree.join("protocols/aep.yaml"),
    )
    .expect("the protocol is readable");
    std::fs::copy(
        root().join("workflows/development/default.yaml"),
        tree.join("workflows/default.yaml"),
    )
    .expect("the workflow is readable");
    write(
        &tree.join("drivers/broken.yaml"),
        "format: aep.driver-steps/1\nid: broken/map\nworkflow: adp/default/2\n\
         states:\n  polishing:\n    steps: []\n",
    );

    let output = protocol(&["validate", "--root", printable(&tree)]);
    let text = stdout(&output);
    assert_eq!(code(&output), 1, "{text}");
    assert!(text.contains("unknown_state"), "{text}");
    assert!(text.contains("polishing"), "{text}");
    assert!(
        text.contains("implement"),
        "the refusal lists the states the workflow does declare:\n{text}"
    );
}

/// Two maps fit `adp/default/2`, and the driver refuses to pick one on the caller's behalf.
///
/// This changed when `development/checks` shipped: before it, a run with no `--map` was given the
/// only map that fitted, and the wave-4 run `W4-1/1` was started that way. The refusal is the
/// wanted outcome rather than a regression — which map a run is under decides how its evidence is
/// obtained, and guessing that is the one thing the driver does not do — but it is a change to what
/// a bare `protocol drive run` does, so it is asserted rather than left to be discovered.
#[test]
fn two_maps_fit_the_workflow_so_the_driver_refuses_to_choose_and_names_both() {
    let fixture = Fixture::new("two-maps", false);
    let output = protocol(&[
        "drive",
        "run",
        "--project",
        printable(&fixture.directory),
        "--root",
        printable(&root()),
        "--task",
        printable(&fixture.directory.join("task.yaml")),
    ]);
    let said = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(code(&output), 1, "{said}");
    for named in ["development/default", "development/checks", "--map"] {
        assert!(
            said.contains(named),
            "the refusal names both maps and the flag that chooses one; `{named}` is missing:\n{said}"
        );
    }
    assert!(
        !fixture.runs().join("lock.json").exists(),
        "a refused start left a lock behind"
    );
}

/// **F-W4.2-4** at the surface an operator meets: the map is checked against the plan it will
/// drive, before anything runs.
///
/// `W4-2/1` learned this at a guard six states in, having spent ten model sessions, 76 minutes and
/// $31.46. Everything the refusal below prints was in two documents on disk at launch.
#[test]
fn a_map_that_cannot_produce_demanded_evidence_is_refused_before_the_first_step() {
    let fixture = Fixture::new("evidence-gap", false);
    let mut args: Vec<String> = vec!["drive".to_owned(), "run".to_owned()];
    args.extend(fixture.location());
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = protocol(&borrowed);
    let said = format!("{}{}", stdout(&output), stderr(&output));

    assert_eq!(code(&output), 1, "{said}");
    for named in [
        "specification",
        "spec-driven",
        "verification",
        "provenance-tracking",
        "adversarial_verify -> review",
        "--allow-evidence-gap",
    ] {
        assert!(
            said.contains(named),
            "the refusal names the kind, who asked, what it blocks and the way through; `{named}` \
             is missing:\n{said}"
        );
    }
    assert!(
        !fixture.runs().join("lock.json").exists(),
        "a refused start left a lock behind"
    );
    assert!(
        !fixture.runs().join("DRIVE-1").exists(),
        "a refused start allocated a run directory, so the refusal was not before everything"
    );

    // And the way through, which is what makes this a refusal rather than a wall: the same command
    // with the flag starts, prints the same gap, and gets as far as the run would have got anyway.
    let allowed = fixture.drive(&["run"], &[]);
    let told = stdout(&allowed);
    assert!(
        told.contains("--allow-evidence-gap` was given") && told.contains("specification"),
        "the flag acknowledges the gap rather than hiding it:\n{told}"
    );
    assert!(
        fixture.runs().join("DRIVE-1").exists(),
        "the flagged run allocated a run directory:\n{told}"
    );
}

/// The cargo map starts a `kind: feature` run with no `--allow-evidence-gap`, which is the whole of
/// `story:evidence-producers-for-the-driven-map`.
///
/// This is the one test here that uses `drivers/development/default.yaml` rather than a fixture,
/// and it is safe for the reason the module header gives for avoiding it: `--max-iterations 0`
/// means the loop body never runs, so no step of that map is executed and nothing runs
/// `cargo test --workspace` inside a test. What is exercised is everything *before* the loop — the
/// map loading, `check_run`, the approval pre-flight and F-W4.2-4's coverage pre-flight — which is
/// exactly where arm c of pilot 1 was refused.
#[test]
fn the_cargo_map_starts_a_feature_run_without_the_evidence_gap_flag() {
    let directory = std::env::temp_dir().join("protocol-drive-cargo-map-preflight");
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(directory.join(".engineering/planning"))
        .expect("the temporary tree is writable");
    let task = directory.join("task.yaml");
    write(
        &task,
        "id: PRE-1\nkind: feature\nobjective: exercise-the-pre-flight\nprotocol: adp/1\n\
         profile: development.driven\n",
    );

    let map = root().join("drivers/development/default.yaml");
    let output = protocol(&[
        "drive",
        "run",
        "--project",
        printable(&directory),
        "--root",
        printable(&root()),
        "--task",
        printable(&task),
        "--map",
        printable(&map),
        // Nothing runs. The pre-flights are what this test is about.
        "--max-iterations",
        "0",
        // The map has two `operator` steps, and a headless start refuses over them before it
        // refuses over anything else. That refusal has its own test; this flag gets past it so the
        // coverage pre-flight is the thing being observed.
        "--pause-on-approval",
    ]);
    let said = format!("{}{}", stdout(&output), stderr(&output));

    assert!(
        !said.contains("--allow-evidence-gap"),
        "the coverage pre-flight named no gap, so nothing should offer the flag that waives one. \
         Before `story:evidence-producers-for-the-driven-map` this run was refused for \
         `contract_result`, `property_test_result`, `verification` and `specification`:\n{said}"
    );
    // The coverage pre-flight runs **before** the metaharness one, deliberately, so this assertion
    // means the same thing on a machine that has the binary and one that does not. With the order
    // the other way round this test passed vacuously in CI, where `metaharness` is not installed:
    // the run stopped at the machine check and the gap was never looked for.
    let allocated = said.contains("run        PRE-1/1");
    let stopped_for_the_machine = said.contains("is not on PATH");
    assert!(
        allocated || stopped_for_the_machine,
        "the run got past coverage — either to a run id, or to the machine check that follows \
         it. Neither means the pre-flight is refusing something it should not:\n{said}"
    );
    if allocated {
        assert!(
            directory.join(".engineering/runs/PRE-1").exists(),
            "the run directory exists:\n{said}"
        );
        assert_eq!(
            stdout(&output).matches("steps      0 run").count(),
            1,
            "no step of the cargo map was executed:\n{said}"
        );
    }
}

/// A map with **both** defects reports the one that will not go away by installing something.
///
/// The two static pre-flights answer different questions: coverage says *this map can never finish
/// this plan*, on every machine; the metaharness check says *this machine cannot run it today*.
/// Until 2026-08-28 the machine check ran first, and the consequence was not merely a worse
/// message — on any machine without that binary the coverage gap was **never looked for**, so the
/// test asserting the committed map's gap was closed passed in CI while guarding nothing.
///
/// Nothing pinned the order for a map with both problems, so a future edit could swap them back and
/// only the single-defect tests would notice. This is that pin, and it asserts the order in both
/// directions: coverage first while it has something to say, and the machine check still reached
/// once coverage is waived.
#[test]
fn a_map_that_is_both_uncoverable_and_unspawnable_reports_the_defect_that_travels() {
    let fixture = Fixture::new("both-defects", false);
    // The fixture map declares `test_result`, `diff` and `static_analysis` and the plan demands more
    // — that is why `Fixture::drive` passes `--allow-evidence-gap` for every other test in this
    // file. Here the gap is the subject. One `llm` step is added so the map also needs a harness to
    // spawn, which is the second defect.
    let map = fixture.directory.join("steps.yaml");
    let with_an_llm_step = format!(
        "{}\x20 review:\n\x20   steps:\n\x20     - kind: llm\n\
         \x20       description: a step that needs a harness this machine has not got\n\
         \x20       prompt: judge the change\n",
        std::fs::read_to_string(&map).expect("the fixture map")
    );
    write(&map, &with_an_llm_step);

    let nowhere = fixture.directory.join("an-empty-path");
    std::fs::create_dir_all(&nowhere).expect("the empty PATH directory is writable");

    let mut args: Vec<String> = vec!["drive".to_owned(), "run".to_owned()];
    args.extend(fixture.location());
    args.extend(["--max-iterations".to_owned(), "0".to_owned()]);
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();

    let refused = protocol_without_metaharness(&borrowed, &nowhere);
    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert_ne!(
        code(&refused),
        0,
        "a map with a coverage gap does not start:\n{said}"
    );
    assert!(
        said.contains("cannot produce evidence"),
        "the coverage gap is what the operator is told about:\n{said}"
    );
    assert!(
        !said.contains("is not on PATH"),
        "and not the machine check, which runs second on purpose — this map's coverage gap is true \
         on every machine, and the missing binary is true only on this one:\n{said}"
    );

    // The other direction: waive the gap, and the machine check is reached. Without this the test
    // would also pass if the metaharness pre-flight had simply been deleted.
    let waived: Vec<&str> = borrowed
        .iter()
        .copied()
        .chain(["--allow-evidence-gap"])
        .collect();
    let machine = protocol_without_metaharness(&waived, &nowhere);
    let complaint = format!("{}{}", stdout(&machine), stderr(&machine));
    assert!(
        complaint.contains("is not on PATH"),
        "the machine check still runs, second:\n{complaint}"
    );
}

/// A crossing relation the workspace manifest declares is not a reason to refuse the run.
///
/// The two readers of one store used to disagree: `protocol artifact validate` resolved a relation
/// into another repository against `.engineering/workspace.yaml` and called it declared, while
/// `protocol drive` read the graph with no manifest at all and refused to start — *the planning
/// store cannot be trusted* — on a store the other verb had just called valid. Both were answering
/// honestly; only one of them was answering the question a person asked.
///
/// Driven over the fixture map, whose steps are all `command`, so this runs on a machine with no
/// `metaharness` — which is every CI runner. The first version of this test used the committed
/// cargo map, and its six `llm` steps meant it could only ever pass here.
#[test]
fn a_relation_the_workspace_declares_does_not_stop_a_run_before_it_starts() {
    let declared = Fixture::new("declared-crossing", false);
    write(
        &declared.directory.join(".engineering/workspace.yaml"),
        "version: aep.workspace/1\nmembers:\n  - name: elsewhere\n    source: ../elsewhere\n",
    );
    let crossing = "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\n\
         status: draft\ntitle: A story that points at another repository\nrelations:\n\
         - informed_by: elsewhere/story:theirs\nrevision: 1\n---\n# Story\n\nBody.\n";
    write(
        &declared
            .directory
            .join(".engineering/planning/story/crossing.md"),
        crossing,
    );

    let output = declared.drive(&["run"], &["--max-iterations", "0"]);
    let said = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        !said.contains("cannot be trusted"),
        "the manifest declares `elsewhere`, so the edge is declared and not dangling:\n{said}"
    );
    assert!(
        said.contains("run        DRIVE-1/1"),
        "the run was allocated, which only happens after the store was read:\n{said}"
    );

    // The other half, so the fix is not *trust everything*: the same story with no manifest beside
    // it is a dangling edge, and the run still refuses and still says which edge.
    let undeclared = Fixture::new("undeclared-crossing", false);
    write(
        &undeclared
            .directory
            .join(".engineering/planning/story/crossing.md"),
        crossing,
    );
    let refused = undeclared.drive(&["run"], &["--max-iterations", "0"]);
    let complaint = format!("{}{}", stdout(&refused), stderr(&refused));
    assert_ne!(
        code(&refused),
        0,
        "an undeclared crossing still stops the run:\n{complaint}"
    );
    assert!(
        complaint.contains("does not declare") || complaint.contains("undeclared"),
        "and it says which edge and why:\n{complaint}"
    );
}

/// The checks map plans against this repository's own task, which is the check `drive run` makes
/// before it executes anything.
///
/// Planning-level on purpose: running it would run nine shell checks, a model session and this
/// repository's own document validation. `check_run` is phase two of the map's cross-validation —
/// every evidence kind against the protocol the **task** resolves to, and the workflow pin against
/// the workflow in the tree — and it is what stands between a map that loads and a map that fails
/// halfway through a run that has already spent a budget.
#[test]
fn the_checks_map_plans_against_the_repositorys_own_task() {
    let registry = aep_engine::load::load_tree(&root()).expect("the document tree loads");
    let text = std::fs::read_to_string(root().join(".engineering/task.yaml"))
        .expect("the repository's own task document is readable");
    let task = aep_schema::parse::task(&text, Some(".engineering/task.yaml")).expect("it parses");
    let plan = aep_engine::resolve(&task, &registry).expect("it resolves");
    let id = "development/checks".parse().expect("a step map id");
    let map = registry
        .step_map(&id)
        .expect("the checks map is in the tree");

    let refusals = map.check_run(&plan.protocol, &plan.workflow);
    assert!(
        refusals.is_empty(),
        "the map is not runnable against the task the repository drives: {refusals}"
    );

    // The property the whole map exists for: the first suite a run records is the story's own
    // checks, red, in the state before implementation. `test.first_result` is the first result ever
    // recorded and never changes, so a map that ran anything else first would wedge exactly as
    // `W4-1/1` did.
    let establish = "establish_verifiers".parse().expect("a state id");
    let first_suite = map
        .steps_for(&establish)
        .iter()
        .find_map(|step| match step {
            aep_driver_spec::map::Step::Command(command) => command
                .evidence
                .as_ref()
                .filter(|mapping| mapping.kind == aep_domain::evidence::EvidenceKind::TestResult)
                .map(|_| command.run.clone()),
            _ => None,
        })
        .expect("`establish_verifiers` records a test result");
    assert_eq!(
        first_suite,
        vec!["bash".to_owned(), ".engineering/checks/run.sh".to_owned()],
        "the first suite a run under this map records is the story's own checks"
    );

    // And the trace step, which is the other thing this map has that its sibling does not: the
    // record is read from the document the checker wrote, never minted from an exit status.
    let implement = "implement".parse().expect("a state id");
    let record = map
        .steps_for(&implement)
        .iter()
        .find_map(|step| match step {
            aep_driver_spec::map::Step::Command(command) => command
                .evidence
                .as_ref()
                .and_then(|mapping| mapping.record.clone()),
            _ => None,
        })
        .expect("`implement` reads a record a verifier wrote");
    assert_eq!(record, "{run_directory}/trace-implement.yaml");
}

/// The fixture map's `establish_verifiers`, rewritten to invoke `protocol` and one other program.
///
/// `--version` is the argument, because the version string is the one thing only *this* build
/// prints: a namesake on the operator's `PATH` answers with its own number, and four releases of
/// drift is exactly what run `W4-3/1` spent a retry budget on. `/bin/sh` is spelled absolutely so
/// the step still runs with the empty `PATH` the test spawns the driver with — which is what makes
/// the proof machine-independent rather than a fact about this laptop.
///
/// **`/bin/sh` is first on purpose.** A step whose program cannot be spawned produces no verdict
/// and spends the state's retry budget, so with `protocol` first a driver that had stopped
/// substituting would stop the run before the other step ever ran — and the test asserting *other
/// programs are untouched* would fail for the substitution's reason rather than its own.
fn map_invoking_protocol() -> String {
    "format: aep.driver-steps/1\n\
     id: fixture/drive\n\
     workflow: adp/default/2\n\
     states:\n\
    \x20 establish_verifiers:\n\
    \x20   steps:\n\
    \x20     - kind: command\n\
    \x20       description: a program that is not this CLI\n\
    \x20       run: [/bin/sh, -c, \"echo resolved as written\"]\n\
    \x20     - kind: command\n\
    \x20       description: the build that is driving says which build it is\n\
    \x20       run: [protocol, --version]\n\
    \x20       evidence:\n\
    \x20         kind: test_result\n\
    \x20         suite: unit\n\
    \x20         verifier: test-runner\n\
    \x20 implement:\n\
    \x20   steps:\n\
    \x20     - kind: command\n\
    \x20       run: [/bin/sh, -c, \"exit 0\"]\n\
    \x20       evidence:\n\
    \x20         kind: diff\n\
    \x20         verifier: git\n"
        .to_owned()
}

/// Every line of a run's `commands.jsonl`, parsed.
fn command_record(run_directory: &Path) -> Vec<serde_json::Value> {
    let path = run_directory.join("commands.jsonl");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each line of the record is JSON"))
        .collect()
}

/// A `command` step that says `protocol` runs the binary the driver **is**, not a namesake.
///
/// **Run `W4-3/1`, 2026-08-28.** `protocol property evidence --out …` was step 4 of `verify`; the
/// first `protocol` on the driver's own `PATH` was a 0.28.0 install that predates the `property`
/// verb, so the step wrote nothing, the driver correctly reported *nothing was observed*, and the
/// step burned its whole retry budget three times with the cause invisible in the message.
///
/// The driver is `protocol`, so `current_exe()` is the binary a step asking for `protocol` means.
/// Spawned here with an **empty** `PATH`, which is what makes this a test of the substitution and
/// not of this machine: without it the step cannot be run at all, and with it the step prints a
/// version string only this build prints.
#[test]
fn a_command_step_that_says_protocol_runs_the_build_that_is_driving_it() {
    let fixture = Fixture::new("protocol-command", false);
    write(
        &fixture.directory.join("steps.yaml"),
        &map_invoking_protocol(),
    );
    let nowhere = fixture.directory.join("an-empty-path");
    std::fs::create_dir_all(&nowhere).expect("the empty PATH directory is writable");

    let mut args: Vec<String> = vec!["drive".to_owned(), "run".to_owned()];
    args.extend(fixture.location());
    args.extend([
        "--allow-evidence-gap".to_owned(),
        "--max-iterations".to_owned(),
        "8".to_owned(),
    ]);
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = protocol_without_metaharness(&borrowed, &nowhere);
    let said = format!("{}{}", stdout(&output), stderr(&output));

    let run_directory = fixture.runs().join("DRIVE-1").join("1");

    // The step ran and printed what only this build prints. A namesake would answer with its own
    // number, and an unresolvable `protocol` would not have produced a log at all.
    let banner = format!("protocol {}", env!("CARGO_PKG_VERSION"));
    let log = std::fs::read_to_string(run_directory.join("establish_verifiers-1-1.log"))
        .unwrap_or_else(|error| panic!("the step wrote no log: {error}\n{said}"));
    assert!(
        log.contains(&banner),
        "the step did not run this build, which is the only one that prints `{banner}`:\n{log}\n{said}"
    );

    // Visible, on the step's own note: substituting a binary silently is its own kind of lie.
    let header = log.lines().next().expect("the log opens with its header");
    assert!(
        header.starts_with("# ran:") && header.contains("/protocol"),
        "the step's log does not name the binary that ran:\n{header}"
    );

    // And on the run's record, where a reader can tell one step's binary from another's.
    let record = command_record(&run_directory);
    let substituted = record
        .iter()
        .find(|entry| entry["state"] == "establish_verifiers" && entry["index"] == 1)
        .expect("the record holds the step that said `protocol`");
    assert_eq!(substituted["program"], "protocol");
    assert_eq!(substituted["resolved"], "driver");
    assert!(
        substituted["ran"]
            .as_str()
            .expect("the record names a path")
            .ends_with("/protocol"),
        "the record does not name the binary that ran: {substituted}"
    );
}

/// A `command` step naming any other program resolves exactly as it always did.
///
/// The substitution is keyed on the name `protocol` and nothing else — `cargo`, `bash` and `git`
/// are tools the driver finds the way it always did, and a driver that rewrote one of those would
/// be a second, undeclared thing to reason about.
///
/// This step is the state's **first**, so the assertion does not depend on the substitution
/// working: a build that never substituted and a build that substituted everything both fail here,
/// and only the second is what this test is for.
#[test]
fn a_command_step_naming_another_program_is_resolved_as_written() {
    let fixture = Fixture::new("other-program", false);
    write(
        &fixture.directory.join("steps.yaml"),
        &map_invoking_protocol(),
    );
    let nowhere = fixture.directory.join("an-empty-path");
    std::fs::create_dir_all(&nowhere).expect("the empty PATH directory is writable");

    let mut args: Vec<String> = vec!["drive".to_owned(), "run".to_owned()];
    args.extend(fixture.location());
    args.extend([
        "--allow-evidence-gap".to_owned(),
        "--max-iterations".to_owned(),
        "8".to_owned(),
    ]);
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = protocol_without_metaharness(&borrowed, &nowhere);
    let said = format!("{}{}", stdout(&output), stderr(&output));

    let run_directory = fixture.runs().join("DRIVE-1").join("1");
    let record = command_record(&run_directory);
    let untouched = record
        .iter()
        .find(|entry| entry["state"] == "establish_verifiers" && entry["index"] == 0)
        .unwrap_or_else(|| panic!("the record holds the step that named another program:\n{said}"));
    assert_eq!(untouched["program"], "/bin/sh");
    assert_eq!(untouched["ran"], "/bin/sh");
    assert_eq!(
        untouched["resolved"], "as-written",
        "a program that is not this CLI was rewritten: {untouched}"
    );

    let log = std::fs::read_to_string(run_directory.join("establish_verifiers-0-1.log"))
        .unwrap_or_else(|error| panic!("that step wrote no log: {error}\n{said}"));
    assert!(
        log.contains("resolved as written"),
        "that step did not run:\n{log}"
    );
    assert!(
        !log.contains(&format!("protocol {}", env!("CARGO_PKG_VERSION"))),
        "that step ran this CLI, which is not what it named:\n{log}"
    );
}
