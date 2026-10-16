//! story:bugfix-workflow: `bugfix/standard` and its step map, `drivers/bugfix/standard.yaml`,
//! driven end to end with a stub in place of the model.
//!
//! The workflow and the map are **this repository's own files**, read from the tree and never
//! copied into the test: a test of a fixture written beside it would pass while the shipped map
//! drifted. Only the protocol, the profile and the task around them are fixtures, because no shipped
//! profile names `bugfix/standard` and the test must not depend on one appearing.
//!
//! `aep drive run` refuses a map with `llm` steps, so the run goes through
//! `aep_cli::drive::start_with_host` with a host whose `llm` executor is a stub, while every
//! `command` step runs for real. That call prints its report to the process's standard output,
//! which a test cannot read from inside the process, so each case runs **itself** as a child
//! process: the child drives and exits with the call's exit code, and the parent reads the report
//! the child printed, the run directory it left and the record of every call the stub saw.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

/// Set in the child process to the fixture directory it drives.
const CHILD: &str = "AEP_BUGFIX_WORKFLOW_FIXTURE";

/// Where the stub `llm` executor records each call, one JSON document per line.
const CALLS: &str = "llm-calls.jsonl";

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the workspace root exists")
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

/// The protocol around the shipped workflow: the phases its states declare, the facts its guards
/// read, and the capabilities its steps exercise. A read that declares no evidence is
/// `command.execute`; a fix that writes is `repository.write`.
const PROTOCOL: &str = "id: fixture\nversion: 1\ntitle: Fixture\n\
    capabilities: [repository.read, repository.write, tests.execute, command.execute]\n\
    evidence_kinds: [test_result]\nverifiers: [test-runner]\nartifact_kinds: []\n\
    phases: [verification-setup, planning, implementation, verification, completion]\n\
    observables:\n  - 'tests.**'\n  - 'evidence.**'\n  - 'state.**'\n";

const PROFILE: &str = "id: fixture.bugfix\ntitle: Fixture bug fix\nprotocol: fixture/1\n\
    workflow: bugfix/standard\ncapabilities:\n  allow: [repository.read, repository.write, \
    tests.execute, command.execute]\ncompletion:\n  all:\n    - tests.regression.failed == 0\n    \
    - tests.gate.failed == 0\n";

/// The project file that makes the fixture a planning store `aep plan artifact show` can read.
const PROJECT: &str =
    "version: aep.project/5\nplanning_scope: bugfix-fixture\nprotocol: fixture/1\n\
    profile: fixture.bugfix\n";

/// The story the `artifact` input names, as the planning store holds one.
const STORY: &str = "---\nformat: aep.planning-md/3\nid: story:login-crash\nkind: story\n\
    status: active\ntitle: Login crashes on an empty password\nrevision: 1\n---\n# Story\n\n\
    Submitting the login form with an empty password crashes the server.\n";

/// A project around the shipped workflow and map.
struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let directory = std::env::temp_dir().join(format!("protocol-bugfix-{name}"));
        std::fs::remove_dir_all(&directory).ok();
        let tree = directory.join("tree");
        write(&tree.join("protocols/fixture/1.yaml"), PROTOCOL);
        // The shipped workflow, as the tree holds it.
        let workflow = std::fs::read_to_string(root().join("workflows/bugfix/standard.yaml"))
            .expect("the repository ships workflows/bugfix/standard.yaml");
        write(&tree.join("workflows/bugfix/standard.yaml"), &workflow);
        write(&tree.join("profiles/fixture.yaml"), PROFILE);
        write(&directory.join(".engineering/project.yaml"), PROJECT);
        write(
            &directory.join(".engineering/planning/story/login-crash.md"),
            STORY,
        );
        write(
            &directory.join("task.yaml"),
            "id: BUG-1\nkind: bugfix\nobjective: login-survives-empty-password\n\
             protocol: fixture/1\nprofile: fixture.bugfix\n",
        );
        write(
            &directory.join("evidence/server.log"),
            "ERROR login: unwrap on None at auth.rs:42\n",
        );
        write(
            &directory.join("evidence/trace.txt"),
            "span login.submit -> auth.verify (panicked)\n",
        );
        Self { directory }
    }

    /// The directory of run `BUG-1/1`.
    fn run_directory(&self) -> PathBuf {
        self.directory.join(".engineering/runs/BUG-1/1")
    }

    fn cursor(&self) -> serde_json::Value {
        let path = self.run_directory().join("cursor.json");
        serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display())),
        )
        .expect("the cursor is JSON")
    }

    /// Every call the stub `llm` executor saw, in order.
    fn calls(&self) -> Vec<serde_json::Value> {
        std::fs::read_to_string(self.directory.join(CALLS))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("a call is JSON"))
            .collect()
    }

    /// `aep drive run` with the shipped map, as its arguments.
    fn arguments(&self, repro: &str) -> Vec<String> {
        let mut argv = vec![
            "run".to_owned(),
            "--project".to_owned(),
            printable(&self.directory).to_owned(),
            "--root".to_owned(),
            printable(&self.directory.join("tree")).to_owned(),
            "--task".to_owned(),
            printable(&self.directory.join("task.yaml")).to_owned(),
            "--map".to_owned(),
            printable(&root().join("drivers/bugfix/standard.yaml")).to_owned(),
            // The fixture profile demands nothing beyond what the map's steps record; the flag
            // keeps the launch check's reading of that out of what these tests are about.
            "--allow-evidence-gap".to_owned(),
        ];
        for (name, value) in [
            ("repro", repro.to_owned()),
            (
                "logs",
                printable(&self.directory.join("evidence/server.log")).to_owned(),
            ),
            (
                "traces",
                printable(&self.directory.join("evidence/trace.txt")).to_owned(),
            ),
            ("artifact", "story:login-crash".to_owned()),
            ("gate", "true".to_owned()),
        ] {
            argv.push("--input".to_owned());
            argv.push(format!("{name}={value}"));
        }
        argv
    }

    /// Runs the test named `case` again as a child process that drives this fixture.
    fn drive_in_child(&self, case: &str) -> Output {
        Command::new(std::env::current_exe().expect("the test binary names itself"))
            .args([case, "--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD, &self.directory)
            .output()
            .expect("the test binary runs")
    }
}

/// The child's half of a case: drive the fixture through the stub host and exit with the call's
/// exit code. Returns only in the parent.
fn drive_if_child(repro: &str) {
    use clap::Parser as _;

    #[derive(clap::Parser)]
    struct Run {
        #[command(flatten)]
        args: aep_cli::drive::RunArgs,
    }

    let Some(directory) = std::env::var_os(CHILD) else {
        return;
    };
    let fixture = Fixture {
        directory: PathBuf::from(directory),
    };
    let run = Run::try_parse_from(fixture.arguments(repro)).expect("the arguments parse");
    let host = stub_host::Stub {
        project: fixture.directory.clone(),
    };
    let code = match aep_cli::drive::start_with_host(&run.args, &host) {
        Ok(code) if code == ExitCode::SUCCESS => 0,
        Ok(code) if code == ExitCode::from(1) => 1,
        Ok(_) => 3,
        Err(error) => {
            println!("start_with_host failed: {error:#}");
            4
        }
    };
    std::process::exit(code);
}

/// A host whose `llm` executor records what it was handed instead of calling a model.
mod stub_host {
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

    use aep_cli::drive::{ExecutionHost, Inputs, Launch, PreparedExecution};
    use aep_driver::executor::{LlmStepExecutor, StepAuthorizer, StepContext, StepOutcome};
    use aep_driver_spec::map::LlmStep;

    pub struct Stub {
        pub project: PathBuf,
    }

    impl ExecutionHost for Stub {
        fn prepare(
            &self,
            _: &Inputs,
            _: Option<&Launch>,
            _: Option<&str>,
            _: Option<&str>,
        ) -> anyhow::Result<PreparedExecution> {
            let project = self.project.clone();
            Ok(PreparedExecution {
                launch_fields: std::collections::BTreeMap::new(),
                // The `aep` the artifact read runs is this tree's build, never the test binary
                // (which is what the driver's own executable is, in here) or whatever is on PATH.
                protocol_binary: Some(PathBuf::from(env!("CARGO_BIN_EXE_aep"))),
                executor: Box::new(move |_, _| {
                    Ok(Box::new(Executor { project, fixes: 0 }) as Box<dyn LlmStepExecutor>)
                }),
            })
        }
    }

    struct Executor {
        project: PathBuf,
        fixes: u32,
    }

    impl LlmStepExecutor for Executor {
        fn run_llm(
            &mut self,
            step: &LlmStep,
            context: &StepContext<'_>,
            _: StepAuthorizer<'_>,
        ) -> StepOutcome {
            let handed: Vec<serde_json::Value> = step
                .context
                .iter()
                .map(|entry| {
                    let path = entry.replace(
                        "{run_directory}",
                        &context.run_directory.display().to_string(),
                    );
                    let name = Path::new(&path)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned());
                    serde_json::json!({
                        "file": name,
                        "content": std::fs::read_to_string(&path).ok(),
                    })
                })
                .collect();
            let state = context.state.to_string();
            // The fix that works is the second one, so verify fails once and routes back.
            if state == "fix" {
                self.fixes += 1;
                if self.fixes == 2 {
                    std::fs::write(self.project.join("fixed"), "")
                        .expect("the project is writable");
                }
            }
            let mut calls = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.project.join(super::CALLS))
                .expect("the call record opens");
            writeln!(
                calls,
                "{}",
                serde_json::json!({ "state": state, "context": handed })
            )
            .expect("the call is recorded");
            StepOutcome::Nothing
        }
    }
}

/// Acceptance *Nothing reproduced*: a reproduction that exits 0 stops the run `blocked` at
/// `reproduce`, says why, and nothing after it runs.
#[test]
fn a_reproduction_that_passes_blocks_the_run_before_any_model_is_called() {
    drive_if_child("true");
    let fixture = Fixture::new("nothing-reproduced");
    let output = fixture
        .drive_in_child("a_reproduction_that_passes_blocks_the_run_before_any_model_is_called");
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a blocked run exits 1:\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cursor = fixture.cursor();
    assert_eq!(cursor["status"], "blocked", "{cursor}");
    assert_eq!(cursor["state"], "reproduce", "{cursor}");
    assert!(
        text.lines().any(|line| line
            .trim_end()
            .ends_with("exited 0, and the step expects it to fail: nothing reproduced")),
        "the report says nothing was reproduced:\n{text}"
    );
    assert!(
        !fixture.run_directory().join("logs.txt").exists(),
        "diagnose never ran, so nothing was collected"
    );
    assert!(
        fixture.calls().is_empty(),
        "no model is called for a bug nobody reproduced: {:?}",
        fixture.calls()
    );
}

/// Acceptance *Driven to completion*: the run collects at diagnose, hands the model all four
/// files, routes back from a failing verify once, and completes.
#[test]
fn a_reproduced_bug_is_diagnosed_fixed_and_verified_to_completion() {
    drive_if_child("test -f fixed");
    let fixture = Fixture::new("driven-to-completion");
    let output =
        fixture.drive_in_child("a_reproduced_bug_is_diagnosed_fixed_and_verified_to_completion");
    let text = String::from_utf8_lossy(&output.stdout);
    let errors = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a completed run exits 0:\n{text}\n{errors}"
    );

    let run = fixture.run_directory();
    for file in ["repro.txt", "logs.txt", "traces.txt", "artifact.txt"] {
        assert!(
            run.join(file).exists(),
            "the run directory holds {file}:\n{text}"
        );
    }

    let calls = fixture.calls();
    let states: Vec<&str> = calls
        .iter()
        .map(|call| call["state"].as_str().expect("a state"))
        .collect();
    assert_eq!(
        states,
        ["diagnose", "fix", "fix"],
        "diagnose once, fix twice:\n{text}"
    );
    let handed = calls[0]["context"].as_array().expect("a context list");
    let read = |file: &str| -> String {
        std::fs::read_to_string(run.join(file))
            .unwrap_or_else(|error| panic!("reading {file}: {error}"))
    };
    let expected: Vec<serde_json::Value> = ["repro.txt", "logs.txt", "traces.txt", "artifact.txt"]
        .iter()
        .map(|file| serde_json::json!({ "file": file, "content": read(file) }))
        .collect();
    assert_eq!(
        handed, &expected,
        "the diagnose step is handed every collected file, with its content"
    );
    assert!(
        read("logs.txt").contains("unwrap on None at auth.rs:42"),
        "the logs were collected from the input file"
    );
    assert!(
        read("traces.txt").contains("auth.verify (panicked)"),
        "the traces were collected from the input file"
    );
    assert!(
        read("artifact.txt").contains("story:login-crash"),
        "the artifact was read through this tree's aep:\n{}",
        read("artifact.txt")
    );

    let moved: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("moved      "))
        .collect();
    assert_eq!(
        moved,
        [
            "reproduce -> diagnose",
            "diagnose -> fix",
            "fix -> verify",
            "verify -> fix",
            "fix -> verify",
            "verify -> fixed"
        ],
        "verify routes back to fix once, then into `fixed`:\n{text}"
    );
    // The engine moves into the terminal state and then reports the completion from there.
    assert!(
        text.lines()
            .any(|line| line.trim_end() == "status     completed"),
        "{text}"
    );
    assert!(
        text.contains("Task complete in `fixed`"),
        "the run is complete in `fixed`:\n{text}"
    );

    let cursor = fixture.cursor();
    assert_eq!(cursor["status"], "completed", "{cursor}");
    assert_eq!(cursor["llm_steps_declared"], 2, "{cursor}");
    assert_eq!(cursor["llm_steps_run"], 3, "{cursor}");
}
