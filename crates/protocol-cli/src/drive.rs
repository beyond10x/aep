//! `protocol drive` — walking a workflow by asking the engine, and doing only what the answers
//! permit.
//!
//! The third module split of `main.rs`, on the criterion the first two set: a verb family with its
//! own store — here, its own *run directory* — its own vocabulary, and no shared state with the
//! rest of the binary.
//!
//! # What is here and what is deliberately not
//!
//! The routing core is [`aep_driver`], and it is pure: it consumes an `Evaluation` and a
//! `TransitionResult` verbatim, never re-derives a verdict and never evaluates a gate. **The three
//! things that touch the world are here**, because they are the three things that cannot be in a
//! crate that claims to be deterministic:
//!
//! | this module | why it cannot be in `aep-driver` |
//! |---|---|
//! | running a program and reading its exit status | a process is the world |
//! | invoking a model | a network call, a credential and a transcript |
//! | pausing for a person | a terminal |
//! | the store lock, the pid-liveness probe and the run directory | a liveness probe reads ambient OS state and uses neither `SystemTime::now` nor `rand`, so a banned-token scan would not catch it. Placement is the only thing keeping the pure crate's claim true — review finding **F19** |
//!
//! # Exit codes
//!
//! | code | meaning |
//! |---|---|
//! | `0` | the run completed — or paused at an `operator` step **with** `--pause-on-approval`, which is what makes the flag opt-in: without it a green exit means finished, with it a green exit means finished **or** waiting, and a caller has to choose to be told that |
//! | `1` | the execution says no: blocked, a budget spent, a store that stopped parsing, a lock another run holds, a headless start that would cross a person |
//! | `2` | `clap`'s, for arguments it refuses |
//!
//! # What this driver does not do, stated rather than left to be discovered
//!
//! * **It never constructs an `Evidence::Approval` and never stamps `Producer::Human`**, under any
//!   flag. `approval_recorded` matches on subject and decision and does **not** check who granted
//!   it, so nothing below the driver would stop a harness minting its own approval: the refusal has
//!   to be the driver's, and it is a source scan in `aep-driver` rather than a promise here.
//! * **A command step's evidence carries a verdict, not counts.** An exit status says *the verifier
//!   ran and said yes or no*; it does not say how many tests passed. So a `test_result` minted here
//!   is the smallest result that carries the verdict — one passing or one failing — and a guard
//!   that reads `tests.unit.passed > 40` needs a step kind that reads a report, which this driver
//!   does not have. Named here rather than discovered later.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode, Stdio};

use aep_domain::action::{
    Action, ActionRequest, CommandExecute, NetworkIntent, NetworkRequest, RepositoryRead,
    RepositoryWrite,
};
use aep_domain::capability::Capability;
use aep_domain::evidence::{
    ChangeSet, ContractResult, Evidence, EvidenceKind, Producer, Provenance, StaticAnalysisResult,
    TestResult, TestSuite,
};
use aep_domain::ids::{StateId, TaskId, ToolRef};
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_driver::coverage::CoverageReport;
use aep_driver::executor::{
    CommandStepExecutor, LlmStepExecutor, OperatorStepExecutor, StepAuthorizer, StepContext,
    StepOutcome,
};
use aep_driver::lock::{Liveness, LockState};
use aep_driver::run::{DriveError, DriverOptions, RunDirectory, RunReport};
use aep_driver::tool::TOOL_CANDIDATES;
use aep_driver_spec::cursor::{DriverCursor, RunId, RunStatus, StolenLock};
use aep_driver_spec::map::{
    placeholders_in, CommandStep, EvidenceMapping, LlmStep, OperatorStep, ScopeRule, Step, StepMap,
    WriteScope,
};
use aep_driver_spec::tool::ToolConfig;
use aep_engine::engine::EvidenceSubmission;
use aep_engine::policy::Decision;
use aep_engine::project::project_directory;
use aep_engine::{Engine, Registry};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

/// The directory inside `.engineering` that holds runs.
const RUNS_DIRECTORY: &str = "runs";

/// The one lock file per store.
///
/// **One fixed path, taken before any run id is allocated.** The reviewed first draft put the lock
/// inside `.engineering/runs/<run-id>/`, which is circular: two invocations counting the existing
/// directories at slightly different moments get `3` and `4`, and **both `create_new` succeed**,
/// because they are different paths — two live runs over one store, which is the option D6
/// explicitly rejected, reached by accident. Review finding **F2**.
const LOCK_FILE: &str = "lock.json";

/// The store-level pointer to the run that last held the lock.
const CURRENT_FILE: &str = "current";

/// The transcript directory inside a run.
const TRANSCRIPTS: &str = "transcripts";

/// Where one attempt at one `llm` step leaves its transcript.
///
/// One function rather than two spellings of the same format string: the step that *writes* the
/// transcript and the step that *checks* it are different steps of a map, and a checker pointed at
/// a path the writer never used would report that a session did nothing.
fn transcript_path(run_directory: &Path, state: &StateId, index: usize, attempt: u32) -> PathBuf {
    run_directory
        .join(TRANSCRIPTS)
        .join(format!("{state}-{index}-{attempt}.jsonl"))
}

/// Expands the placeholders a step map admits, or says which one it could not.
///
/// The vocabulary is `aep_driver_spec::map::CommandStep::PLACEHOLDERS` and an unknown name is
/// refused at load, so the only failure reachable here is a `{transcript}` in a run where the
/// `llm` step before it has not run — which is a fact about the run and cannot be decided from the
/// document.
fn expand(word: &str, context: &StepContext<'_>) -> Result<String, String> {
    let mut expanded = word.to_owned();
    for name in placeholders_in(word) {
        let value = match name {
            "run_directory" => context.run_directory.display().to_string(),
            "transcript" => {
                let Some(step) = context.preceding_llm else {
                    return Err(format!(
                        "`{{transcript}}` is the transcript of the `llm` step this one follows, \
                         and no `llm` step of `{}` has run in this run",
                        context.state
                    ));
                };
                transcript_path(
                    context.run_directory,
                    context.state,
                    step.index,
                    step.attempt,
                )
                .display()
                .to_string()
            }
            other => return Err(format!("nothing expands `{{{other}}}`")),
        };
        expanded = expanded.replace(&format!("{{{name}}}"), &value);
    }
    Ok(expanded)
}

/// Where the plugin lives, when no `--plugin-dir` said.
/// Where a project keeps its own Claude Code plugin, tried last when no flag and no
/// environment variable named one.
const PROJECT_PLUGIN_DIR: &str = "integrations/claude-code";

const PLUGIN_DIR_ENV: &str = "AEP_DRIVE_PLUGIN_DIR";

/// What can be done with a driven run.
#[derive(Debug, Subcommand)]
pub(crate) enum DriveCommand {
    /// Start a new run of a task, allocating a run id.
    Run(RunArgs),
    /// Report what the store's last run is doing, and who holds the lock.
    Status(StatusArgs),
    /// Continue a run that stopped, re-taking the store lock.
    Resume(ResumeArgs),
}

/// Where the run's inputs are.
#[derive(Debug, Clone, Args)]
pub(crate) struct DriveLocation {
    /// The project directory — the one holding `.engineering`. Discovered when omitted.
    #[arg(long)]
    project: Option<PathBuf>,
    /// The document tree. Comes from the project when omitted.
    #[arg(long)]
    root: Option<PathBuf>,
    /// The task document. Comes from the project when omitted.
    #[arg(long)]
    task: Option<PathBuf>,
    /// The planning store, as a markdown directory. Defaults to the store `project.yaml` names —
    /// `.engineering/planning/` unless it says `store: sqlite` or `store: postgres`.
    #[arg(long)]
    store: Option<PathBuf>,
    /// The step map: a file, or the id of one in the document tree.
    #[arg(long)]
    map: Option<String>,
    /// A plugin directory to load into every `llm` step's session. Repeatable.
    ///
    /// **W3.4's integration seam, and the reason it is a flag rather than a constant.** The
    /// plugin's `hooks/hooks.json` is the driver's enforcement arm — the layer that sees a tool's
    /// *arguments*, which `--allowedTools` cannot — and a session that never loaded the plugin
    /// never loaded the hooks. Where the plugin lives is a property of the machine, not of the
    /// protocol, so it is named here rather than guessed at. `AEP_DRIVE_PLUGIN_DIR` supplies it
    /// when the flag is absent, which is what lets an eval script set it once for a whole run.
    #[arg(long)]
    plugin_dir: Vec<PathBuf>,
}

/// What a `harness: b10x` step needs and a step map cannot say.
///
/// **Three facts about the machine, not about the work.** `metaharness run b10x` refuses a launch
/// that names no endpoint and no model rather than defaulting either — *"a default here would aim
/// an evaluation arm at somebody's production API the first time the flag was forgotten"* — and
/// the b10x loop holds no vendor login of its own, so the credential is a file or a variable
/// somebody named. None of that belongs in a step map: a map is pinned, committed and driven on
/// more than one machine, and an endpoint written into one would be the same URL for all of them.
///
/// They are flags rather than environment variables for the reason the launch record exists at
/// all: a run has to be able to say what it was started with, and a variable that was exported in
/// one shell is not a fact anybody can read back afterwards. They persist into `launch.json` with
/// everything else, so a `resume` re-reads them instead of being told again.
#[derive(Debug, Clone, Default, Args, serde::Serialize, serde::Deserialize)]
pub(crate) struct B10xOptions {
    /// The endpoint a `harness: b10x` step's loop is pointed at, as the gateway's root URL.
    #[arg(long = "b10x-endpoint", value_name = "BASE_URL")]
    #[serde(default)]
    endpoint: Option<String>,
    /// The model that endpoint serves. The loop picks none of its own.
    #[arg(long = "b10x-model", value_name = "MODEL")]
    #[serde(default)]
    model: Option<String>,
    /// Point the loop at `OPENAI_API_KEY` instead of launching it with no credential at all.
    ///
    /// Off by default, because a gateway that authenticates nobody is the case a driven b10x run
    /// starts in and a credential nobody asked to send is one that travels by accident. With it,
    /// metaharness refuses the launch by name when the variable is not in this process's
    /// environment rather than starting a run that will fail at its first request.
    #[arg(long = "b10x-api-key")]
    #[serde(default)]
    api_key: bool,
}

/// The arguments of `protocol drive run`.
#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    /// Where the run's inputs are.
    #[command(flatten)]
    location: DriveLocation,
    /// What a `harness: b10x` step needs from this machine.
    #[command(flatten)]
    b10x: B10xOptions,
    /// Run until the first thing a person owes, then persist and exit 0.
    #[arg(long)]
    pause_on_approval: bool,
    /// Stop after this many loop iterations, whatever the state of the run.
    #[arg(long, default_value_t = 25)]
    max_iterations: u32,
    /// Take the store lock from a holder that is provably dead.
    #[arg(long)]
    take_lock: bool,
    /// Start even though the map cannot produce evidence the plan will demand.
    ///
    /// **This weakens no rule the engine enforces.** The pre-flight it turns off is an *economic*
    /// check, not a protocol one: without it a run walks every state and blocks at the guard that
    /// wanted the record, which for `W4-2/1` cost $31.46 and 76 minutes. With this flag the gap is
    /// still printed and the run still blocks at that guard — the caller has simply said they know,
    /// which is the position somebody driving a run to a `--pause-on-approval` stop and supplying
    /// the record by hand is legitimately in.
    #[arg(long)]
    allow_evidence_gap: bool,
}

/// The arguments of `protocol drive status`.
#[derive(Debug, Args)]
pub(crate) struct StatusArgs {
    /// Where the run's inputs are.
    #[command(flatten)]
    location: DriveLocation,
    /// Which run to report on. The store's current one when omitted.
    #[arg(long)]
    run: Option<String>,
}

/// The arguments of `protocol drive resume`.
#[derive(Debug, Args)]
pub(crate) struct ResumeArgs {
    /// The run to continue, such as `AUTH-142/3`.
    run: String,
    /// Where the run's inputs are.
    #[command(flatten)]
    location: DriveLocation,
    /// Run until the first thing a person owes, then persist and exit 0.
    #[arg(long)]
    pause_on_approval: bool,
    /// Stop after this many loop iterations, whatever the state of the run.
    #[arg(long, default_value_t = 25)]
    max_iterations: u32,
    /// Take the store lock from a holder that is provably dead.
    #[arg(long)]
    take_lock: bool,
}

/// Runs one `protocol drive` verb.
pub(crate) fn run(command: DriveCommand) -> Result<ExitCode> {
    match command {
        DriveCommand::Run(args) => start(&args),
        DriveCommand::Status(args) => status(&args),
        DriveCommand::Resume(args) => resume(&args),
    }
}

/// The project this was run in, or a refusal naming what to pass instead.
fn discover_project() -> Result<PathBuf> {
    let here = std::env::current_dir().context("reading the working directory")?;
    let directory = project_directory();
    aep_engine::project::discover(&here).with_context(|| {
        format!(
            "no `--project` was given and no `{directory}/project.yaml` was found in {} or \
             any parent",
            here.display()
        )
    })
}

/// Everything a run needs, resolved from flags or from the project it was run in.
struct Inputs {
    /// The project directory — the one holding `.engineering`.
    project: PathBuf,
    /// The documents in force.
    registry: Registry,
    /// The task being driven.
    task: Task,
    /// The planning store the artifact graph is rebuilt from every iteration.
    store: crate::planning::DrivenPlan,
    /// The step map driving the run.
    map: StepMap,
    /// Where the step map came from, for a report.
    map_origin: String,
    /// The plugin directories every `llm` step's session loads.
    plugin_dirs: Vec<PathBuf>,
}

impl DriveLocation {
    /// Resolves the run's inputs.
    fn inputs(&self) -> Result<Inputs> {
        let project = match &self.project {
            Some(path) => path.clone(),
            None => discover_project()?,
        };

        // The registry is loaded **once per invocation** and the store is rebuilt **per
        // iteration**, and the asymmetry is chosen rather than accidental (review finding F8): a
        // mid-run edit to `workflows/` is not picked up, because the cursor pins the workflow for
        // the life of the run precisely so a governing document cannot move under it; a mid-run
        // edit to the planning store *is*, because that is the work happening.
        let registry = match &self.root {
            Some(root) => crate::load(root)?,
            None => {
                aep_engine::project::load(&project)
                    .map_err(|errors| anyhow::anyhow!("{errors}"))?
                    .registry
            }
        };

        let task = match &self.task {
            Some(path) => crate::read_task(path)?,
            None => aep_engine::project::load(&project)
                .map_err(|errors| anyhow::anyhow!("{errors}"))?
                .task
                .context("the project names no task, and no `--task` was given")?,
        };

        let store = crate::planning::DrivenPlan::for_project(self.store.as_deref(), &project)?;

        let (map, map_origin) = self.step_map(&registry, &task)?;

        let plugin_dirs = self.plugin_dirs(&project);

        Ok(Inputs {
            project,
            registry,
            task,
            store,
            map,
            map_origin,
            plugin_dirs,
        })
    }

    /// This location, with anything the caller left out filled in from what the run remembers.
    ///
    /// A flag always wins: an operator who names a map on a resume means that map, and a run
    /// directory is a record of what happened rather than a policy about what happens next.
    fn remembering(&self, launch: Option<&Launch>, project: &Path) -> Self {
        let mut merged = self.clone();
        merged.project = Some(
            merged
                .project
                .clone()
                .unwrap_or_else(|| project.to_path_buf()),
        );
        if let Some(launch) = launch {
            if merged.task.is_none() {
                merged.task.clone_from(&launch.task);
            }
            if merged.map.is_none() {
                merged.map.clone_from(&launch.map);
            }
            if merged.root.is_none() {
                merged.root.clone_from(&launch.root);
            }
            if merged.plugin_dir.is_empty() {
                merged.plugin_dir.clone_from(&launch.plugin_dirs);
            }
        }
        merged
    }

    /// The plugin directories a session loads: the flags, then the environment, then the project's own.
    ///
    /// The environment is a fallback and never an addition — a caller that named directories meant
    /// those directories, and silently appending one from the ambient environment is how a run
    /// ends up enforcing something its own command line does not mention.
    ///
    /// **The project's own plugin is the last fallback, and run `W4-3/1` is why.** Started without
    /// the flag, its sessions loaded *no* plugin and answered `Unknown skill: planning` to the very
    /// first thing the step map asks for — while offering the operator's personal skills and tools,
    /// because a session with no plugin is not a session with no inventory. A driven run of *this*
    /// repository that has to be told where *this* repository's plugin lives is a flag nobody will
    /// remember, and forgetting it does not fail: it produces a run that walks, spends and records
    /// the wrong thing.
    ///
    /// It is a fallback rather than an addition for the same reason the environment is: a caller
    /// who named directories meant those, and the run report says which were loaded either way.
    fn plugin_dirs(&self, project: &Path) -> Vec<PathBuf> {
        if !self.plugin_dir.is_empty() {
            return self.plugin_dir.clone();
        }
        if let Some(value) = std::env::var_os(PLUGIN_DIR_ENV) {
            return vec![PathBuf::from(value)];
        }
        let own = project.join(PROJECT_PLUGIN_DIR);
        if own.is_dir() {
            return vec![own];
        }
        Vec::new()
    }

    /// The step map: the file named by `--map`, the map with that id, or the only one that fits.
    fn step_map(&self, registry: &Registry, task: &Task) -> Result<(StepMap, String)> {
        if let Some(named) = &self.map {
            let path = Path::new(named);
            if path.is_file() {
                let text = fs::read_to_string(path)
                    .with_context(|| format!("reading {}", path.display()))?;
                let origin = path.display().to_string();
                let map = aep_schema::parse::step_map(&text, Some(&origin))
                    .map_err(|error| anyhow::anyhow!("{error}"))?;
                return Ok((map, origin));
            }
            let id = named.parse().map_err(|error| {
                anyhow::anyhow!("{named} is not a file and not a step map id: {error}")
            })?;
            let map = registry
                .step_map(&id)
                .with_context(|| format!("no step map `{named}` is in the document tree"))?;
            return Ok((map.clone(), format!("step map {named}")));
        }

        // No `--map`: the map is the one written against the workflow this task resolves to. More
        // than one is a choice the driver refuses to make on the caller's behalf — the same
        // position `protocol artifact move` takes for an illegal transition, and for the same
        // reason: the refusal names what to do instead.
        let plan = aep_engine::resolve(task, registry)
            .map_err(|errors| anyhow::anyhow!("{errors}"))
            .context("the task cannot be resolved")?;
        let fitting: Vec<&StepMap> = registry
            .step_maps()
            .filter(|map| {
                *map.workflow.id() == plan.workflow.id
                    && map.workflow.accepts(plan.workflow.version)
            })
            .collect();
        match fitting.as_slice() {
            [only] => Ok(((*only).clone(), format!("step map {}", only.id))),
            [] => bail!(
                "no step map in the document tree is written against `{}/{}`; pass `--map <file>`",
                plan.workflow.id,
                plan.workflow.version
            ),
            several => bail!(
                "{} step maps are written against `{}/{}` ({}); pass `--map` to choose one",
                several.len(),
                plan.workflow.id,
                plan.workflow.version,
                several
                    .iter()
                    .map(|map| map.id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// `protocol drive run`
fn start(args: &RunArgs) -> Result<ExitCode> {
    let inputs = args.location.inputs()?;
    let runs = runs_directory(&inputs.project)?;

    let engine = Engine::new(inputs.registry.clone());
    let plan = aep_engine::resolve(&inputs.task, &inputs.registry)
        .map_err(|errors| anyhow::anyhow!("{errors}"))
        .context("the task cannot be resolved")?;

    // Phase two of the map's cross-validation, run **before the first step executes**. The protocol
    // in force comes from the task, which no document loader has seen, so this cannot have happened
    // at load: without it a map validates and then fails at `ProtocolError::EvidenceRejected`
    // halfway through a run that has already spent a budget.
    let refusals = inputs.map.check_run(&plan.protocol, &plan.workflow);
    if !refusals.is_empty() {
        outln!("{} is not runnable against this task:", inputs.map_origin);
        for refusal in refusals.as_slice() {
            outln!("  - {refusal}");
        }
        return Ok(ExitCode::from(1));
    }

    // The static pre-flights, both checked before the lock is taken for the same reason: a run
    // that cannot spawn its `llm` steps — or that no map step can ever evidence out of — should
    // not own a run id and a lock to find that out.
    //
    // **Coverage first, and the order is load-bearing.** Both are static, but they answer about
    // different things: coverage is decidable from the two documents and says *this map can never
    // finish this plan* on every machine, while the metaharness check says *this machine cannot
    // run it today*. With the machine check first, a map with a real coverage gap read as fine
    // wherever the binary was missing — which is exactly what happened: the test asserting the
    // cargo map's gap is closed passed **vacuously** in CI, where `metaharness` is not installed,
    // and would have gone on passing if the gap came back.

    // F-W4.2-4: the other half of `check_run`, and the half that was missing. `check_run` asks
    // whether every kind the map declares is one the protocol knows; this asks whether every kind
    // the *plan* will demand is one some step can produce. Both questions were answerable from the
    // same two documents before `W4-2/1` spent $31.46 and 76 minutes discovering the second one at
    // a guard, six states in.
    let coverage = aep_driver::evidence_coverage(&plan, &inputs.map);
    if !coverage.is_covered() {
        report_evidence_gap(&coverage, &inputs.map_origin, args.allow_evidence_gap);
        if !args.allow_evidence_gap {
            return Ok(ExitCode::from(1));
        }
    }
    for warning in &coverage.warnings {
        // Printed and never blocking. Each of these is a question nobody can answer from documents
        // — who will have produced a record when the step runs, or whether a person will hand one
        // over between runs — and refusing on an undecided question is what invariant 5 forbids.
        outln!("note: {warning}");
    }

    if let Some(refusal) = machine_preflights(&inputs.map, &inputs.project, &args.b10x) {
        outln!("{refusal}");
        return Ok(ExitCode::from(1));
    }

    // D3(c): the headless pre-flight, static and decidable and run before anything executes.
    let owed = owed_to_a_person(&plan, &inputs.map);
    if !owed.is_empty() && !args.pause_on_approval {
        outln!(
            "this run would reach {} thing(s) only a person can answer, and `--pause-on-approval` \
             was not given:",
            owed.len()
        );
        for line in &owed {
            outln!("  - {line}");
        }
        outln!();
        outln!(
            "`--pause-on-approval` runs until the first of them, persists and exits 0. There is no \
             flag that answers one: nothing below the driver checks who granted an approval, so \
             the refusal has to be the driver's."
        );
        return Ok(ExitCode::from(1));
    }

    let lock = take_lock(&runs, args.take_lock)?;
    let run_id = allocate_run(&runs, &inputs.task.id)?;
    lock.record_run(&run_id)?;
    let directory = RunDirectory::at(run_path(&runs, &run_id));
    fs::create_dir_all(directory.path())
        .with_context(|| format!("creating {}", directory.path().display()))?;
    // How this run was launched, so `resume` needs none of it again and the line this command
    // prints is a line that works.
    Launch {
        task: args.location.task.clone(),
        map: args.location.map.clone(),
        project: Some(inputs.project.clone()),
        root: args.location.root.clone(),
        pause_on_approval: args.pause_on_approval,
        plugin_dirs: inputs.plugin_dirs.clone(),
        b10x: args.b10x.clone(),
    }
    .write(directory.path());
    fs::write(runs.join(CURRENT_FILE), format!("{run_id}\n"))
        .with_context(|| format!("writing {}", runs.join(CURRENT_FILE).display()))?;

    let options = DriverOptions {
        max_iterations: args.max_iterations,
        pause_on_approval: args.pause_on_approval,
        headless: true,
    };
    let mut executors = CliExecutors::new(
        inputs.project.clone(),
        directory.path().to_path_buf(),
        inputs.plugin_dirs.clone(),
        inputs.map.workflow.id().to_string(),
        inputs.map.workflow.major().to_string(),
        args.b10x.clone(),
    );
    let report = aep_driver::run::drive(
        &engine,
        &inputs.task,
        &inputs.store,
        &inputs.map,
        &directory,
        &mut executors,
        &options,
    );

    if let Some(stolen) = lock.stolen() {
        outln!(
            "note: this run took the lock from pid {} of run {}",
            stolen.pid,
            stolen.run
        );
    }
    let outcome = finish(report, &run_id, &inputs.map_origin);
    lock.release();
    outcome
}

/// `protocol drive resume`
fn resume(args: &ResumeArgs) -> Result<ExitCode> {
    // The run directory is found before the inputs are resolved, because the inputs are what the
    // run directory remembers: `protocol drive resume <run>` with no other flag is the line this
    // command prints, and until 2026-08-29 that line did not work.
    let project = match &args.location.project {
        Some(named) => named.clone(),
        None => discover_project()?,
    };
    let runs = runs_directory(&project)?;
    let run_id: RunId = args
        .run
        .parse()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let directory = RunDirectory::at(run_path(&runs, &run_id));
    if !directory.path().is_dir() {
        bail!("no run {run_id} in {}", runs.display());
    }
    let launch = Launch::read(directory.path());
    let location = args.location.remembering(launch.as_ref(), &project);
    let inputs = location.inputs()?;
    let pause_on_approval =
        args.pause_on_approval || launch.as_ref().is_some_and(|l| l.pause_on_approval);

    // What the run was pointed at, remembered rather than retyped — the same argument as `--map`
    // and `--task`, and stronger: a `resume` given a different endpoint or a different model would
    // be a run whose second half was produced by something its own record does not name.
    let b10x = launch.as_ref().map(|l| l.b10x.clone()).unwrap_or_default();

    // The same pre-flights `run` does, and a resume needs them just as much: a resume re-takes the
    // lock, so discovering the missing binary mid-step costs a lock and an attempt in the cursor of
    // a run that was already stopped once.
    if let Some(refusal) = metaharness_preflight(&inputs.map) {
        outln!("{refusal}");
        return Ok(ExitCode::from(1));
    }
    if let Some(refusal) = b10x_preflight(&inputs.map, &b10x) {
        outln!("{refusal}");
        return Ok(ExitCode::from(1));
    }

    // A paused run holds no lock, because the pause has no bound — so a resume must **re-take** it,
    // and must refuse when another run now holds it. The first draft said a pause releases and
    // never said a resume re-acquires, which left the obvious assumption to produce two live runs.
    let lock = take_lock(&runs, args.take_lock)?;
    lock.record_run(&run_id)?;

    let engine = Engine::new(inputs.registry.clone());
    let options = DriverOptions {
        max_iterations: args.max_iterations,
        pause_on_approval,
        headless: true,
    };
    let mut executors = CliExecutors::new(
        inputs.project.clone(),
        directory.path().to_path_buf(),
        inputs.plugin_dirs.clone(),
        inputs.map.workflow.id().to_string(),
        inputs.map.workflow.major().to_string(),
        b10x,
    );
    let report = aep_driver::run::resume(
        &engine,
        &inputs.task,
        &inputs.store,
        &inputs.map,
        &directory,
        &mut executors,
        &options,
    );
    let outcome = finish(report, &run_id, &inputs.map_origin);
    lock.release();
    outcome
}

/// `protocol drive status`
fn status(args: &StatusArgs) -> Result<ExitCode> {
    let project = match &args.location.project {
        Some(path) => path.clone(),
        None => discover_project()?,
    };
    let runs = project.join(project_directory()).join(RUNS_DIRECTORY);
    if !runs.is_dir() {
        outln!("no runs in {}", runs.display());
        return Ok(ExitCode::SUCCESS);
    }

    match read_lock(&runs)? {
        Some(holder) => {
            let state = holder.state();
            outln!(
                "lock       held by run {} (pid {} on {}, {})",
                holder.file.run.as_deref().unwrap_or("<unallocated>"),
                holder.file.pid,
                holder.file.host,
                match state.liveness {
                    Liveness::Alive => "alive",
                    Liveness::Dead => "not alive — stale, and still refused without --take-lock",
                    Liveness::OtherHost => "another host, so never stale here",
                }
            );
        }
        None => outln!("lock       free"),
    }

    let named = match &args.run {
        Some(run) => run.clone(),
        None => fs::read_to_string(runs.join(CURRENT_FILE))
            .unwrap_or_default()
            .trim()
            .to_owned(),
    };
    if named.is_empty() {
        outln!("current    none");
        return Ok(ExitCode::SUCCESS);
    }
    let run_id: RunId = named.parse().map_err(|error| anyhow::anyhow!("{error}"))?;
    let directory = RunDirectory::at(run_path(&runs, &run_id));
    let cursor = directory
        .read_cursor()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    print_cursor(&cursor);
    Ok(ExitCode::SUCCESS)
}

/// Prints a cursor, which is what `status` is for.
fn print_cursor(cursor: &DriverCursor) {
    outln!("run        {}", cursor.run);
    outln!("task       {}", cursor.task);
    outln!("execution  {}", cursor.execution);
    outln!("workflow   {}", cursor.workflow);
    outln!("map        {} ({})", cursor.map, cursor.map_digest);
    outln!("state      {} (step {})", cursor.state, cursor.step);
    outln!("status     {}", cursor.status);
    outln!("iterations {}", cursor.iterations);
    for (state, visits) in &cursor.visits {
        outln!("visits     {state}: {visits}");
    }
    for (step, attempts) in &cursor.attempts {
        outln!("attempts   {step}: {attempts}");
    }
    if let Some(stolen) = &cursor.took_lock_from {
        outln!(
            "took lock  from pid {} of run {} on {}",
            stolen.pid,
            stolen.run,
            stolen.host
        );
    }
    for reason in &cursor.reasons {
        outln!("           {reason}");
    }
}

/// Renders a finished run and chooses the exit code.
fn finish(
    report: Result<RunReport, DriveError>,
    run: &RunId,
    map_origin: &str,
) -> Result<ExitCode> {
    let report = match report {
        Ok(report) => report,
        Err(error) => bail!("{error}"),
    };

    outln!("run        {run}");
    outln!("map        {map_origin}");
    outln!("status     {}", report.cursor.status);
    outln!("state      {}", report.cursor.state);
    outln!(
        "steps      {} run, {} submitted",
        report.steps_run,
        report.evidence_submitted
    );
    for (from, to) in &report.transitions {
        outln!("moved      {from} -> {to}");
    }
    for note in &report.notes {
        outln!("note       {note}");
    }
    // The engine's words, verbatim. The driver adds its own lines beside them and never summarises
    // or re-words them: a report that paraphrased a refusal would be a second, worse protocol.
    if !report.reasons.is_empty() {
        outln!("blocked because:");
        for reason in &report.reasons {
            outln!("  - {reason}");
        }
    }
    if let Some(explanation) = &report.explanation {
        outln!("{explanation}");
    }
    if report.cursor.status.is_resumable() {
        // The line has to work as printed. Until 2026-08-29 it did not: `--map`, `--task`,
        // `--pause-on-approval` and `--plugin-dir` were all re-read from nothing, so an operator
        // who typed exactly this got a different run or an error (F-W4.2-4). The run directory now
        // remembers all four, so the short line is the true one.
        outln!("resume with: protocol drive resume {run}");
    }

    Ok(match report.cursor.status {
        RunStatus::Completed | RunStatus::AwaitingOperator => ExitCode::SUCCESS,
        _ => ExitCode::from(1),
    })
}

/// Prints the evidence the plan will demand and no step of the map can produce.
///
/// One line per **kind**, not per requirement: two principles wanting the same missing kind are one
/// thing to fix. Every line names who asked and what stays shut, so the refusal can be navigated to
/// rather than argued with — and the paragraph after it says what to do, because a refusal that does
/// not answer the question it creates is a wall.
fn report_evidence_gap(report: &CoverageReport, origin: &str, allowed: bool) {
    if allowed {
        outln!("{origin} cannot produce evidence this task's plan will demand, and `--allow-evidence-gap` was given:");
    } else {
        outln!("{origin} cannot produce evidence this task's plan will demand:");
    }
    for entry in &report.missing {
        outln!(
            "  - `{}`: demanded by {}, and no step of the map declares it",
            entry.kind.as_str(),
            entry.demanded_by.join("; ")
        );
        if !entry.blocks.is_empty() {
            outln!("      blocks: {}", entry.blocks.join(", "));
        }
    }
    outln!();
    if allowed {
        outln!(
            "the run will walk every state before the guard that wants these and stop there. That \
             is the cost the flag accepts; nothing about the guard itself has changed."
        );
        return;
    }
    outln!(
        "no run under this map can reach `evidence.missing == 0`, so it would walk every state \
         before that guard and stop at it. Three ways forward: add a `command` step whose \
         `evidence:` declares the kind — one outside the driver's mintable set needs `record: \
         <path>` and a verifier that writes the document, the way this repository's `checks` map \
         mints `trace_conformance`; drive the task under a map that has one; or, if the record \
         will arrive from outside the run, pass `--allow-evidence-gap` and accept that the run \
         stops at the guard."
    );
}

/// Everything only a person can answer that this run would reach.
///
/// Two static, decidable sources, and both are checked before the first step because the
/// alternative is starting a run that will certainly wedge:
///
/// * the plan's own reachable approvals — `human: true` approvals and reviews, human verifiers, and
///   capabilities a `command` step would exercise that need one ([`aep_driver::approval`]);
/// * an `operator` step in a state this workflow can reach from where the run starts. The map is
///   saying a person is owed something there, which is the same fact in a different document.
fn owed_to_a_person(plan: &aep_domain::plan::ExecutionPlan, map: &StepMap) -> Vec<String> {
    let mut owed: Vec<String> = aep_driver::approval::reachable_approvals(plan, map)
        .into_iter()
        .map(|approval| format!("{}: {}", approval.source, approval.detail))
        .collect();

    for state in reachable_states(&plan.workflow) {
        for (index, step) in map.steps_for(&state).iter().enumerate() {
            if let Step::Operator(operator) = step {
                owed.push(format!(
                    "step map, state {state} step {index}: an operator step — {}",
                    operator
                        .description
                        .clone()
                        .unwrap_or_else(|| operator.prompt.clone())
                ));
            }
        }
    }
    owed
}

/// Every state reachable from the workflow's initial state, including it.
fn reachable_states(workflow: &aep_domain::workflow::Workflow) -> BTreeSet<StateId> {
    let mut reached: BTreeSet<StateId> = BTreeSet::new();
    let mut frontier = vec![workflow.initial.clone()];
    while let Some(state) = frontier.pop() {
        if !reached.insert(state.clone()) {
            continue;
        }
        for transition in &workflow.transitions {
            if transition.from == state {
                frontier.push(transition.to.clone());
            }
        }
    }
    reached
}

/// The `.engineering/runs/` directory, created if it is not there.
fn runs_directory(project: &Path) -> Result<PathBuf> {
    let runs = project.join(project_directory()).join(RUNS_DIRECTORY);
    fs::create_dir_all(&runs).with_context(|| format!("creating {}", runs.display()))?;
    Ok(runs)
}

/// The directory of one run.
fn run_path(runs: &Path, run: &RunId) -> PathBuf {
    let [task, ordinal] = run.segments();
    runs.join(task).join(ordinal)
}

/// The next run id for a task: one more than the highest that exists.
///
/// Allocated **after** the lock is taken, which is the whole of review finding F2. A run directory
/// is never deleted and never reused, so the count only goes up.
fn allocate_run(runs: &Path, task: &TaskId) -> Result<RunId> {
    let directory = runs.join(task.as_str());
    let mut highest = 0_u32;
    if directory.is_dir() {
        for entry in
            fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
        {
            let entry = entry.with_context(|| format!("reading {}", directory.display()))?;
            if let Some(ordinal) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
            {
                highest = highest.max(ordinal);
            }
        }
    }
    RunId::new(task, highest + 1).map_err(|error| anyhow::anyhow!("{error}"))
}

/// What `lock.json` holds.
///
/// **No timestamp.** Staleness is decided by liveness rather than by a number somebody wrote into a
/// file: any age threshold has to exceed the longest legitimate step, and the longest legitimate
/// step is an `operator` step waiting for a person, which has no bound. A driver that broke a lock
/// after two hours would break exactly the runs that paused correctly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LockFile {
    /// The run it granted, once one has been allocated.
    run: Option<String>,
    /// The process holding it.
    pid: u32,
    /// The host that process is on.
    host: String,
    /// The driver that took it.
    driver: String,
}

/// A lock this process holds, and the run it took it from if it took one.
struct HeldLock {
    path: PathBuf,
    stolen: Option<StolenLock>,
}

impl HeldLock {
    /// Records the run id inside the lock, so a refusal can name it without a second read.
    fn record_run(&self, run: &RunId) -> Result<()> {
        let mut file: LockFile = serde_json::from_str(
            &fs::read_to_string(&self.path)
                .with_context(|| format!("reading {}", self.path.display()))?,
        )
        .with_context(|| format!("reading {}", self.path.display()))?;
        file.run = Some(run.to_string());
        fs::write(&self.path, serde_json::to_string_pretty(&file)?)
            .with_context(|| format!("writing {}", self.path.display()))
    }

    /// What this lock was taken from, when it was taken from somebody.
    fn stolen(&self) -> Option<&StolenLock> {
        self.stolen.as_ref()
    }

    /// Releases the lock.
    ///
    /// Called on every exit path the driver controls, including the approval pause and budget
    /// exhaustion: a paused run does not hold a lock, because the pause has no bound. What a paused
    /// run keeps is `current`, so resuming is one word.
    fn release(self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// A lock somebody else holds.
struct Holder {
    file: LockFile,
}

impl Holder {
    /// The holder as the router sees it — a value, never a probe.
    fn state(&self) -> LockState {
        LockState {
            run: self
                .file
                .run
                .clone()
                .unwrap_or_else(|| "<unallocated>".to_owned()),
            pid: self.file.pid,
            host: self.file.host.clone(),
            liveness: liveness(&self.file),
        }
    }
}

/// Reads the lock, when there is one.
fn read_lock(runs: &Path) -> Result<Option<Holder>> {
    let path = runs.join(LOCK_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    let file: LockFile =
        serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    Ok(Some(Holder { file }))
}

/// Takes the store lock, or refuses and names the holder.
fn take_lock(runs: &Path, force: bool) -> Result<HeldLock> {
    let path = runs.join(LOCK_FILE);
    let mine = LockFile {
        run: None,
        pid: std::process::id(),
        host: host(),
        driver: format!("protocol-cli {}", env!("CARGO_PKG_VERSION")),
    };
    let body = serde_json::to_string_pretty(&mine)?;

    // One `create_new` syscall: atomic on every filesystem that matters, and it needs no advisory
    // locking. `flock` was rejected because its semantics differ across the filesystems people keep
    // repositories on, NFS in particular.
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut handle) => {
            use std::io::Write as _;
            handle
                .write_all(body.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
            return Ok(HeldLock { path, stolen: None });
        }
        Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
            return Err(error).with_context(|| format!("creating {}", path.display()));
        }
        Err(_) => {}
    }

    let holder = read_lock(runs)?.context("the lock exists and cannot be read")?;
    let state = holder.state();
    if !force || !state.is_stale() {
        bail!("{}", state.refusal(force));
    }

    // `--take-lock` supersedes rather than erases: what was there goes into the new run's cursor,
    // so *"this run took the lock from pid 4711"* is in the record rather than in nobody's memory.
    fs::write(&path, &body).with_context(|| format!("writing {}", path.display()))?;
    Ok(HeldLock {
        path,
        stolen: Some(StolenLock {
            run: state.run.clone(),
            pid: state.pid,
            host: state.host.clone(),
        }),
    })
}

/// Whether the process named in a lock is alive, dead, or somebody else's problem.
///
/// **Liveness, never age.** A pid on another host says nothing to this one's process table, so a
/// lock naming another host is never stale here whatever the local table says.
fn liveness(file: &LockFile) -> Liveness {
    if file.host != host() {
        return Liveness::OtherHost;
    }
    if Path::new("/proc").is_dir() {
        return if Path::new(&format!("/proc/{}", file.pid)).exists() {
            Liveness::Alive
        } else {
            Liveness::Dead
        };
    }
    // No `/proc` to read: the honest answer is that this build cannot tell, and the safe one is to
    // treat the holder as alive. A lock nobody can prove is dead is a lock nobody may take.
    Liveness::Alive
}

/// This machine's name, for the lock.
fn host() -> String {
    for path in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(name) = fs::read_to_string(path) {
            let name = name.trim();
            if !name.is_empty() {
                return name.to_owned();
            }
        }
    }
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_owned())
}

/// The three things that touch the world.
struct CliExecutors {
    /// Where a command step runs.
    working_directory: PathBuf,
    /// Where transcripts and logs go.
    run_directory: PathBuf,
    /// The plugins every `llm` step's session loads — and with them, the hooks.
    plugin_dirs: Vec<PathBuf>,
    /// The workflow the run resolved to, for the frame the `metaharness` executor writes.
    workflow_id: String,
    /// Its pinned major version, as the step map states it.
    workflow_version: String,
    /// What a `harness: b10x` step needs from this machine, and nothing else reads.
    b10x: B10xOptions,
}

impl CliExecutors {
    /// Builds the executors for one run.
    fn new(
        working_directory: PathBuf,
        run_directory: PathBuf,
        plugin_dirs: Vec<PathBuf>,
        workflow_id: String,
        workflow_version: String,
        b10x: B10xOptions,
    ) -> Self {
        Self {
            working_directory,
            run_directory,
            plugin_dirs,
            workflow_id,
            workflow_version,
            b10x,
        }
    }

    /// The step's sealed frame document, written beside the transcript it governs.
    fn write_frame_document(
        &self,
        context: &StepContext<'_>,
        transcripts: &Path,
    ) -> Result<PathBuf, String> {
        let frame = metaharness_frame(context, &self.workflow_id, &self.workflow_version);
        let path = transcripts.join(format!(
            "{}-{}-{}.frame.json",
            context.state, context.index, context.attempt
        ));
        let document = frame_document(&frame)?;
        fs::write(&path, document)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;

        // Beside the frame, when this state refused anything. `protocol trace check` reads it as
        // it reads any specification.
        if let Some(refusals) = refusal_specification(context.state, context.index, context.tools) {
            let refusals_path = transcripts.join(format!(
                "{}-{}-{}.refused.json",
                context.state, context.index, context.attempt
            ));
            let rendered = serde_json::to_string_pretty(&refusals)
                .map_err(|error| format!("cannot render the refusal specification: {error}"))?;
            fs::write(&refusals_path, rendered)
                .map_err(|error| format!("cannot write {}: {error}", refusals_path.display()))?;
        }
        Ok(path)
    }

    /// The `metaharness run` invocation for one step, on whichever arm the step named.
    ///
    /// The frame document is written for **both** harnesses and passed to only one. See
    /// [`b10x_argv`]: metaharness refuses a b10x launch that carries a frame, because a frame is
    /// enforced through a decision channel that loop does not have. What the file is on that arm
    /// is the record of what the step was, in the same neutral vocabulary, beside the same refusal
    /// specification and the same `metaharness.event/1` transcript — which is what makes the two
    /// arms comparable at all.
    fn argv_for(
        &self,
        harness: Harness,
        step: &LlmStep,
        frame_file: &Path,
        prompt: &str,
    ) -> Vec<String> {
        match harness {
            Harness::ClaudeCode => metaharness_argv(
                frame_file,
                &self.working_directory,
                &self.plugin_dirs,
                prompt,
            ),
            Harness::B10x => b10x_argv(
                &self.b10x,
                &self.working_directory,
                &step.scope,
                &step.context,
                prompt,
            ),
        }
    }

    /// The one `llm` executor: the vendor is driven through the metaharness seam, in ask mode.
    ///
    /// The step's surface travels twice, deliberately (F9's "both halves"): the sealed
    /// `metaharness.frame/1` document pins what the step *is*, and this process answers every
    /// `tool.requested` event at decision time through [`decide_tool`] — the two retired shell
    /// hooks, ported, plus the per-state allowlist that used to ride on `--allowedTools` — and then
    /// through the **engine**, which is what `authorize` is. The decisions and denials arrive as
    /// `tool.decided` events in the event stream this executor writes as the transcript, never in a
    /// side-channel log a forgotten flag can silence: run `W4-2` lost all eight of its post-fix
    /// sessions to exactly that, a resume that dropped `--plugin-dir` and ran unenforced while
    /// looking clean.
    fn run_llm_metaharness(
        &mut self,
        harness: Harness,
        step: &LlmStep,
        context: &StepContext<'_>,
        authorize: StepAuthorizer<'_>,
    ) -> StepOutcome {
        let transcripts = self.run_directory.join(TRANSCRIPTS);
        if let Err(error) = fs::create_dir_all(&transcripts) {
            return StepOutcome::NoVerdict {
                reason: format!(
                    "cannot write transcripts to {}: {error}",
                    transcripts.display()
                ),
            };
        }
        let transcript = transcript_path(
            self.run_directory.as_path(),
            context.state,
            context.index,
            context.attempt,
        );

        let frame_file = match self.write_frame_document(context, &transcripts) {
            Ok(path) => path,
            Err(reason) => return StepOutcome::NoVerdict { reason },
        };

        let argv = self.argv_for(harness, step, &frame_file, &prompt_for(step, context));
        // No `current_dir`: the working directory travels as `--cwd` and metaharness spawns the
        // vendor there itself, with a constructed environment nothing here needs to reach into.
        let spawned = Process::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => {
                return StepOutcome::NoVerdict {
                    reason: format!("`{}` could not be run: {error}", argv.join(" ")),
                }
            }
        };
        let mut commands = child.stdin.take().expect("stdin was piped");
        let events = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        // Drained on its own thread: a child blocked writing a full stderr pipe while this loop
        // blocks reading stdout is a deadlock, not a slow run.
        let stderr_thread = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut std::io::BufReader::new(stderr), &mut text);
            text
        });

        let mut transcript_file = match fs::File::create(&transcript) {
            Ok(file) => file,
            Err(error) => {
                let _ = child.kill();
                return StepOutcome::NoVerdict {
                    reason: format!("cannot write {}: {error}", transcript.display()),
                };
            }
        };
        let adjudication = answer_events(
            harness,
            context,
            events,
            &mut commands,
            &mut transcript_file,
            authorize,
        );
        drop(commands);
        outln!("{}", adjudication.line(harness, context.state));
        let status = child.wait();
        let stderr_text = stderr_thread.join().unwrap_or_default();

        match status {
            Ok(status) if status.success() => {
                // An `llm` step never carries evidence, and the type is what makes that true.
                // What the model achieved that is checkable is observed by the command step
                // after it.
                StepOutcome::Nothing
            }
            Ok(status) => {
                let tail: String = stderr_text
                    .lines()
                    .rev()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" | ");
                StepOutcome::NoVerdict {
                    reason: format!(
                        "metaharness exited {}; {}the event stream is at {}",
                        status
                            .code()
                            .map_or_else(|| "on a signal".to_owned(), |code| code.to_string()),
                        if tail.is_empty() {
                            String::new()
                        } else {
                            format!("it said: {tail}; ")
                        },
                        transcript.display()
                    ),
                }
            }
            Err(error) => StepOutcome::NoVerdict {
                reason: format!("waiting on metaharness failed: {error}"),
            },
        }
    }
}

impl CommandStepExecutor for CliExecutors {
    fn run_command(&mut self, step: &CommandStep, context: &StepContext<'_>) -> StepOutcome {
        // Expanded before anything is spawned, and a placeholder that cannot be filled is D5's
        // `Unknown`: a command line carrying the literal characters `{transcript}` would run, fail
        // to open that file and be recorded as a verdict about the subject.
        let words: Vec<String> = match step.run.iter().map(|word| expand(word, context)).collect() {
            Ok(words) => words,
            Err(reason) => return StepOutcome::NoVerdict { reason },
        };
        let rendered = words.join(" ");
        let outcome = Process::new(&words[0])
            .args(&words[1..])
            .current_dir(&self.working_directory)
            .stdin(Stdio::null())
            .output();

        let output = match outcome {
            Ok(output) => output,
            // Nothing was observed: a missing executable is not a failing suite. Submitting a
            // failing `TestResult` for a suite that never ran would fabricate an observation, which
            // is invariant 7's failure one layer above the engine.
            Err(error) => {
                return StepOutcome::NoVerdict {
                    reason: format!("`{rendered}` could not be run: {error}"),
                }
            }
        };

        let log = self.run_directory.join(format!(
            "{}-{}-{}.log",
            context.state, context.index, context.attempt
        ));
        let mut body = String::new();
        body.push_str(&String::from_utf8_lossy(&output.stdout));
        body.push_str(&String::from_utf8_lossy(&output.stderr));
        let _ = fs::write(&log, body);

        let Some(code) = output.status.code() else {
            // Killed by a signal: a partial suite is not a failing suite.
            return StepOutcome::NoVerdict {
                reason: format!("`{rendered}` was killed before it produced a verdict"),
            };
        };

        // A verifier that wrote its own record: read what it wrote. The exit status is not
        // consulted at all — `protocol trace evidence` exits 0 on a run that gapped, because the
        // verdict is in the document and the engine is what decides on it.
        if let Some(mapping) = &step.evidence {
            if let Some(record) = &mapping.record {
                return read_record(record, mapping, &rendered, context);
            }
        }

        let Some(mapping) = &step.evidence else {
            return if code == 0 {
                StepOutcome::Nothing
            } else {
                StepOutcome::NoVerdict {
                    reason: format!(
                        "`{rendered}` exited {code} and declares no evidence, so \
                                     nothing was observed"
                    ),
                }
            };
        };

        match mint(mapping, code == 0, &rendered, observed_now()) {
            Some(submission) => StepOutcome::Observed(Box::new(submission)),
            None => StepOutcome::NoVerdict {
                reason: format!(
                    "`{rendered}` exited {code}, and a `{}` record has no form that says so",
                    mapping.kind.as_str()
                ),
            },
        }
    }
}

impl OperatorStepExecutor for CliExecutors {
    fn run_operator(&mut self, step: &OperatorStep, context: &StepContext<'_>) -> StepOutcome {
        outln!();
        outln!("this run needs a person, in state {}:", context.state);
        outln!("  {}", step.prompt);
        // Verbatim, one line per requirement, because that is what the explanation is *for*: a
        // summary of what is outstanding is a second opinion about it.
        if !context.requirements.is_empty() {
            outln!();
            outln!("what is outstanding here:");
            for line in context.requirements {
                outln!("  {line}");
            }
        }
        StepOutcome::Paused {
            reason: format!("an operator step in {} is owed an answer", context.state),
        }
    }
}

impl LlmStepExecutor for CliExecutors {
    fn run_llm(
        &mut self,
        step: &LlmStep,
        context: &StepContext<'_>,
        authorize: StepAuthorizer<'_>,
    ) -> StepOutcome {
        // The seam § 4.9 point 3 names, and the reason it is a name rather than a trait: a
        // second harness is a second executor selected by this string. Since
        // `epic:metaharness-migration` there is no bare-argv path left to select — every name here
        // reaches a `metaharness run` invocation, because a second way to launch a session is a
        // second policy to forget. `claude-code` names the vendor, `metaharness` is the name the
        // executor first landed under, and `b10x` is the loop this org owns.
        let Some(harness) = Harness::named(&step.harness) else {
            return StepOutcome::NoVerdict {
                reason: format!(
                    "the step names harness `{}`, and this build invokes {}",
                    step.harness,
                    Harness::NAMES.map(|name| format!("`{name}`")).join(", ")
                ),
            };
        };
        self.run_llm_metaharness(harness, step, context, authorize)
    }
}

/// What the session is told about its own surface, in the harness's own words.
///
/// Split out of [`prompt_for`] because it is the one paragraph that is genuinely per-harness, and
/// because the two arms differ in *what kind of thing* bounds them: one is refused by a seam, the
/// other is never offered the tool at all. Rendered from the same [`ToolConfig`] the policy reads,
/// so the prompt and the enforcement cannot disagree — two lists that could drift would be worse
/// than one list nobody has, because the model would trust the wrong one.
fn surface_lines(harness: Harness, tools: &ToolConfig) -> String {
    let mut lines = String::new();
    let offered = harness.tools(tools);
    if !offered.is_empty() {
        lines.push_str("\nThe tools this state admits, and there are no others: ");
        lines.push_str(&offered.join(", "));
        lines.push_str(if harness.adjudicates() {
            ".\nA call outside that set is refused by the driver before it runs. Do not search for \
             a tool that is not on the list — it is not hidden, it does not exist here.\n"
        } else {
            // Not *refused* — **absent**. Telling an observed loop that a call will be refused
            // would describe a seam it does not have, and a session told to expect a refusal that
            // never comes learns nothing from the silence.
            ".\nThat list is what this **state** admits, and there is no seam behind it: a tool \
             outside it is not published to you at all, so there is nothing to search for and \
             nothing that will refuse you. What you were actually **given** is the part of it \
             this machine can confine, which may be less — a write or an exec entry appears only \
             where the workspace is confined. Work from the tools you have, and do not reach for \
             one that is named here and absent from your surface: it is not hidden, this machine \
             could not publish it.\n"
        });
    }
    // **The shell's two rules, stated rather than discovered.** `driven_surface` refuses on both,
    // and a session not told either learns them by being refused: measured over run `W4-3/1`, 21 of
    // 174 calls — 12% of everything the run did — were one of these two, in every state, from the
    // first to the last. They are the cheapest possible thing to say and the most expensive thing
    // to find out.
    //
    // The b10x arm needs one of the two and not the other, and the difference is the point: its
    // `run` entry takes an argv **list**, so there is no string for a `&&` to appear in and a
    // composed command is not a thing that can be written. The program restriction still has to be
    // stated, because a declared program set is only cheap to obey when it is known.
    if tools.shell_offered() && !harness.adjudicates() {
        lines.push_str(
            "\n`run` takes an argv **list** and starts one program. Nothing is composed, \
             redirected or substituted — there is no shell here to do it with — and the only \
             program it will start is `protocol`, and only `protocol artifact …` and `protocol \
             trace …`. Building and testing are `command` steps the driver runs itself, so that \
             their records carry a verifier's provenance instead of yours.\n",
        );
    } else if tools.shell_offered() {
        lines.push_str(
            "\n`Bash` runs **one simple invocation per call**. No `&&`, no `|`, no `;`, no `$(…)`, \
             no redirect — a composed command is refused whole, so two things you want are two \
             calls.\n\
             It runs `protocol artifact …`, `protocol trace …`, and the readers `grep`, `rg`, \
             `ls`, `cat`, `head`, `tail` and `wc` — those only because nothing here can redirect \
             their output into a file. Not `git`, not `cargo`, not `sed`, not `awk`, not `find`, \
             not `xargs`, not `protocol --help`. \
             Building and testing are `command` steps the driver runs itself, so that their records \
             carry a verifier's provenance instead of yours — running them here would produce \
             nothing the engine can admit.\n",
        );
    } else {
        lines.push_str(
            "\nThis state holds **no shell**. Anything a suite must observe is run by the driver as \
             a `command` step, recorded with a verifier's provenance rather than yours.\n",
        );
    }
    lines
}

/// The prompt one `llm` step is given.
///
/// Assembled from the step map's own prompt and the state's requirement lines, each of which names
/// the document that asked for it. Everything an `llm` step knows is either in a file or in this
/// string — which is the property that makes a step's input a function of persisted state, and
/// therefore the property the narrow replay claim rests on.
///
/// **The harness is read off the step rather than passed in**, so the prompt and the tool set the
/// step will actually be given cannot be rendered from two different tables. A step map that names
/// `b10x` and a prompt naming `Bash` would be an instruction to reach for a tool that does not
/// exist in that loop's catalogue — a whole turn spent, per session, learning what the driver
/// already knew. A name this build does not invoke falls back to the default rendering; nothing
/// runs on that path, because [`Harness::named`] has already refused the step.
fn prompt_for(step: &LlmStep, context: &StepContext<'_>) -> String {
    let harness = Harness::named(&step.harness).unwrap_or(Harness::ClaudeCode);
    let mut prompt = String::new();
    // **Which task this run is driving, before anything the map says.** A step map is written once
    // and driven many times, so its prompt can only say *the task under `.engineering/`* — and a
    // repository that has driven more than one run has several sitting there. Run `W4-3/1` read
    // `task.yaml`, which is `W4-1`, and reported that the intake it had been asked for already
    // existed; the cursor said `W4-3` the whole time. The engine knew and the model did not.
    //
    // It leads rather than follows the step's own prompt because it is the subject of every
    // sentence after it, and it names the artifacts rather than a path: a path has to be read
    // correctly, and an id is what the store answers to.
    prompt.push_str("This run drives task `");
    prompt.push_str(context.task.id.as_str());
    prompt.push_str("` — objective `");
    prompt.push_str(context.task.objective.summary.as_str());
    prompt.push('`');
    let derived = &context.task.artifacts.derived_from;
    if !derived.is_empty() {
        prompt.push_str(", derived from ");
        for (position, artifact) in derived.iter().enumerate() {
            if position > 0 {
                prompt.push_str(", ");
            }
            prompt.push('`');
            prompt.push_str(&artifact.to_string());
            prompt.push('`');
        }
    }
    prompt.push_str(
        ". Any other task document in this tree belongs to another run and is not yours to read.\n\n",
    );
    prompt.push_str(&step.prompt);
    // The skills the step names, in the prompt rather than on the command line. `--agents` takes a
    // JSON object of *agent definitions* and is not a skill selector; a step map's `skills:` list
    // reaches the session by being asked for, and the `Skill` tool — a named exemption in the tool
    // table, because loading instructions takes no action — is what answers.
    if !step.skills.is_empty() {
        prompt.push_str("\n\nLoad ");
        for (position, skill) in step.skills.iter().enumerate() {
            if position > 0 {
                prompt.push_str(" and ");
            }
            prompt.push_str("the `");
            prompt.push_str(skill);
            prompt.push('`');
        }
        // Named without a tool on a harness that has no skill mechanism: the b10x catalogue has no
        // entry for `skill.load`, so instructing it to use one would be instructing it to reach
        // for something the loop cannot publish.
        prompt.push_str(match (step.skills.len() == 1, harness.adjudicates()) {
            (true, true) => " skill before you act, with the `Skill` tool.\n",
            (false, true) => " skills before you act, with the `Skill` tool.\n",
            (true, false) => " skill before you act.\n",
            (false, false) => " skills before you act.\n",
        });
    }
    prompt.push_str("\n\nYou are in workflow state `");
    prompt.push_str(context.state.as_str());
    prompt.push_str("`.\n");
    if !context.requirements.is_empty() {
        prompt.push_str("\nWhat must hold here, one line per requirement:\n");
        for line in context.requirements {
            prompt.push_str("  ");
            prompt.push_str(line);
            prompt.push('\n');
        }
    }
    // The other half of the same question, and the half no step was ever told: what the state is
    // trying to *reach*. Under its own heading rather than merged into the list above, because the
    // two are different obligations — one is owed while here, the other is owed before the run may
    // leave — and a step that cannot tell them apart cannot tell which one it is being refused on.
    if !context.reaching.is_empty() {
        prompt.push_str(
            "\nWhat this state is trying to reach, one line per requirement that does not hold yet \
             on the way out:\n",
        );
        for line in context.reaching {
            prompt.push_str("  ");
            prompt.push_str(line);
            prompt.push('\n');
        }
    }
    // **The surface, stated rather than discovered.** `decide_tool` refuses a call outside this set
    // and prints exactly this list in the refusal — so a session that is not told it up front
    // learns its own surface by being refused, one wasted turn at a time. Run `W4-3/1` spent a turn
    // per session doing precisely that, and its first attempt was a `ToolSearch` for `Grep` and
    // `Glob`: it was not guessing, it was trying to *load* what nothing had told it it did not have.
    //
    // Rendered from `context.tools`, the same value `decide_tool` reads, so the prompt and the
    // policy cannot disagree. Two lists that could drift would be worse than one list nobody has:
    // the model would trust the wrong one.
    prompt.push_str(&surface_lines(harness, context.tools));
    prompt.push_str(
        "\nYou cannot submit evidence, and nothing you say is evidence. What you achieve is \
         observed by the verifier the driver runs after this step.\n",
    );
    prompt
}

/// The harness's tool names for an admitted capability set.
///
/// The shell metacharacter this command composes with, if any — **respecting quotes**.
///
/// A scan for the bare characters refused `grep -n "StolenLock\|took_lock_from" crates/`, which is
/// one invocation whose `|` is an *argument* to grep. Run `A3` hit it three times in one state, and
/// it was a defect of the same hour: the readers were admitted, and then the most natural way to use
/// the most useful one was refused. A rule that forbids what it was written to allow is worse than
/// no rule, because the session cannot tell which half to believe.
///
/// Deliberately not a shell parser. It tracks three things — single quotes, double quotes and a
/// backslash escape — and asks whether a metacharacter is *outside* both quotes. `$(` and a backtick
/// substitute inside **double** quotes, so those are refused there too; inside single quotes they
/// are literal and are not. That is the whole of the grammar this rule needs, and every case it
/// decides is one a reader can check by eye.
fn composes(command: &str) -> Option<char> {
    let (mut single, mut double, mut escaped) = (false, false, false);
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' if !single => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '`' if !single => return Some('`'),
            '$' if !single && chars.peek() == Some(&'(') => return Some('$'),
            ';' | '&' | '|' | '>' | '<' | '\n' if !single && !double => return Some(c),
            _ => {}
        }
    }
    None
}

/// Programs a driven step may run because they only ever **read**, and the state admits reading.
///
/// **A driven session had no way to search, and the harness told it to use the one thing the driver
/// denied.** `repository.read` renders `Glob` and `Grep`, which Claude Code 2.1.247 does not offer;
/// its own error then says *search file contents with `grep` via the Bash tool instead*, and
/// `driven_surface` refused `grep`. Run `W4-3/1` spent 13 calls on `sed`, `ls` and `cat`, 6 on the
/// two tools that do not exist, and never once searched anything. A state that admits reading and
/// offers no way to read at scale is a capability gap wearing a policy's clothes.
///
/// The set is small and every member is chosen for one property: **it cannot write.** That holds
/// only because composition and redirection are already refused a few lines above — `>` , `|`, `;`,
/// `&&` and `$(…)` never reach here — so there is no route from a reader to a file. Deliberately
/// absent, each for a reason rather than an oversight:
///
/// * `sed` and `awk` — both write. `sed -i` edits in place; `awk` has `print > "file"`.
/// * `find` — `-delete`, `-exec` and `-fprintf` are writes wearing a search's name.
/// * `xargs`, `env`, `sh`, `bash` — each runs a program this list did not admit.
///
/// It is not a general shell and this does not make it one. The rule is unchanged: a driven step's
/// shell reaches the `protocol` CLI, and now also reads what the state already permits it to read.
const READ_ONLY_PROGRAMS: &[&str] = &["grep", "rg", "ls", "cat", "head", "tail", "wc"];

/// The rendering half of adapter point 2: the *decision* about which capabilities admit which
/// actions is the protocol's and is shared; only this table is Claude Code's. Three entries are not
/// functions of a capability and each is decided rather than left to an implementer — a shell is
/// offered only with `command.execute`, `Skill` is a named exemption, and `Task` is never offered,
/// because a subagent's tool set is derived by nothing in these decisions and would be a route
/// around the per-state allowlist.
fn allowed_tools(config: &ToolConfig) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        tools.extend(["Read", "Glob", "Grep"].map(ToOwned::to_owned));
    }
    if config.admits(&Capability::RepositoryWrite) {
        tools.extend(["Edit", "Write", "NotebookEdit"].map(ToOwned::to_owned));
    }
    if config.admits(&Capability::NetworkRead) {
        tools.extend(["WebFetch", "WebSearch"].map(ToOwned::to_owned));
    }
    if config.shell_offered() {
        tools.push("Bash".to_owned());
    }
    if config.skills_offered() {
        tools.push("Skill".to_owned());
    }
    tools.sort();
    tools.dedup();
    tools
}

/// One vendor tool call as the `ActionRequest` the engine decides on, or nothing when no honest one
/// exists.
///
/// **The reverse direction of [`allowed_tools`], and it lives beside it for that reason** — the
/// answer to `story:metaharness-executor`'s open question. `allowed_tools` renders *capability →
/// tool names*; this renders *one call → the action it is*. Neither decides anything: the protocol
/// owns which capability an action needs (`Action::required_capability`), and a table here that
/// tried to be clever would be a second, weaker policy.
///
/// | tool | action | capability it therefore needs |
/// |---|---|---|
/// | `Read` | `repository.read` of the named file | `repository.read` |
/// | `Glob`, `Grep` | `repository.read` of the searched directory | `repository.read` |
/// | `Edit`, `Write` | `repository.write` of the named file | `repository.write` |
/// | `NotebookEdit` | `repository.write` of the named notebook | `repository.write` |
/// | `Bash` | `command.execute` of the program and its arguments | `command.execute` |
/// | `WebFetch` | a reading network request to the named URL | `network.read` |
///
/// **Two offered tools deliberately return `None`, and the engine is not consulted about them:**
///
/// * `Skill` — it loads instructions and takes no action. It is a named exemption in
///   [`allowed_tools`] for the same reason, and everything it *causes* is a subsequent, governed
///   call that arrives here on its own.
/// * `WebSearch` — a search names no URL, and a `NetworkRequest` carrying a query string in its
///   `url` field would state a destination nobody requested. The capability layer still gates it:
///   the tool is only offered when `network.read` is admitted.
///
/// Everything else — `Task` above all — never reaches this function, because [`decide_tool`] has
/// already refused a tool the state does not offer.
///
/// One disagreement is worth naming rather than discovering: [`allowed_tools`] offers `Read`,
/// `Glob` and `Grep` when **either** `repository.read` **or** `artifact.read` is admitted, and this
/// renders all three as a repository read. A state admitting only `artifact.read` therefore has the
/// engine refuse what the rendering table offered — and the engine wins, which is the right way
/// round: reading a file is a repository read whatever tool asked for it.
fn action_for(tool: &str, input: &serde_json::Value) -> Option<ActionRequest> {
    /// Every path a payload names under `keys`, in the order the keys are given.
    fn paths(input: &serde_json::Value, keys: &[&str]) -> Vec<String> {
        keys.iter()
            .filter_map(|key| input[*key].as_str())
            .map(ToOwned::to_owned)
            .collect()
    }

    let action = match tool {
        "Read" => Action::RepositoryRead(RepositoryRead {
            paths: paths(input, &["file_path"]),
        }),
        // A search with no `path` is a search of the working directory, which is what it is
        // recorded as rather than as a read of nothing.
        "Glob" | "Grep" => Action::RepositoryRead(RepositoryRead {
            paths: match paths(input, &["path"]) {
                empty if empty.is_empty() => vec![".".to_owned()],
                named => named,
            },
        }),
        "Edit" | "Write" => Action::RepositoryWrite(RepositoryWrite {
            paths: paths(input, &["file_path"]),
            intent: None,
        }),
        "NotebookEdit" => Action::RepositoryWrite(RepositoryWrite {
            paths: paths(input, &["notebook_path"]),
            intent: None,
        }),
        // Splitting on whitespace is honest **here and only here**: `driven_surface` has already
        // refused anything that composes, redirects or substitutes, so what is left is one simple
        // invocation and its arguments.
        "Bash" => {
            let mut words = input["command"]
                .as_str()
                .unwrap_or_default()
                .split_whitespace();
            Action::CommandExecute(CommandExecute {
                program: words.next().unwrap_or_default().to_owned(),
                args: words.map(ToOwned::to_owned).collect(),
            })
        }
        "WebFetch" => Action::NetworkRequest(NetworkRequest {
            url: input["url"].as_str().unwrap_or_default().to_owned(),
            intent: NetworkIntent::Read,
        }),
        _ => return None,
    };
    Some(ActionRequest::new(action))
}

/// The engine's refusal as one line the model can act on.
///
/// The engine's own `DecisionExplanation` is four lines and belongs in a terminal; a `tool.decided`
/// event carries one reason string. Nothing is re-worded — the operation, the capability, the
/// decision, the document that decided and what is missing are all the engine's — and the layer is
/// named, so the event stream says who refused.
fn engine_refusal(decision: &Decision) -> String {
    let rule = decision.reason.as_ref().map_or_else(String::new, |reason| {
        format!(" ({} rule {})", reason.source, reason.rule)
    });
    let missing = if decision.missing.is_empty() {
        String::new()
    } else {
        format!(". Missing: {}", decision.missing.join("; "))
    };
    format!(
        "the engine refuses this call: `{}` needs the capability `{}`, which is {} in state \
         `{}`{rule}{missing}",
        decision.operation, decision.capability, decision.decision, decision.current_state
    )
}

/// The session loop: every event line into the transcript, every decision back down stdin.
///
/// A free function of its streams so the executor stays under its own roof: nothing here knows a
/// process, only a reader of event lines, a writer of command lines, and the engine.
///
/// # Two layers, in this order, and the reason it is this one
///
/// 1. **[`decide_tool`]** — the ported hooks and the per-state allowlist. It runs first because it
///    is the only layer that sees a call's *arguments*: `protocol artifact list | tee out` and
///    `protocol artifact list` need the same capability and are not the same act, and no
///    `ActionRequest` can express the difference.
/// 2. **the engine** — [`action_for`] renders the call as an `ActionRequest` and `authorize`
///    decides. Asked only about calls layer 1 admitted, so a refusal is attributed to the layer
///    that took it rather than to both, and **the engine's deny wins**: the two layers read the
///    same effective policy, so a disagreement means the rendering table is looser than the
///    protocol, and the protocol is what governs.
///
/// Every reason names its layer, because the event stream is where a person finds out who refused.
fn answer_events(
    harness: Harness,
    context: &StepContext<'_>,
    events: impl std::io::Read,
    commands: &mut impl std::io::Write,
    transcript: &mut impl std::io::Write,
    authorize: StepAuthorizer<'_>,
) -> Adjudication {
    let mut tally = Adjudication::default();
    for line in std::io::BufRead::lines(std::io::BufReader::new(events)) {
        let Ok(line) = line else { break };
        let _ = transcript
            .write_all(line.as_bytes())
            .and_then(|()| transcript.write_all(b"\n"));
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        // **The driver audits its own claim.** The per-state tool set is the primary enforcement
        // mechanism, and the standard this repository sets for itself is that an enforcement
        // mechanism nobody audits is a claim. Until now nothing compared the set the driver
        // *renders* against the set the session was actually given — so `tool_config` named `Glob`
        // and `Grep` to a Claude Code that offers neither, and every session of run `W4-3/1` spent
        // a turn finding that out for itself.
        //
        // Reported and never fatal: a harness offering *more* than the state admits is normal and
        // is what `decide_tool` is for; a harness offering *less* is a rendering this repository
        // owns and should fix. Only the second is printed.
        if event["event"] == "session.started" {
            if let Some(offered) = event["offered_tools"].as_array() {
                let present: Vec<&str> = offered.iter().filter_map(|v| v.as_str()).collect();
                let missing: Vec<String> = harness
                    .tools(context.tools)
                    .into_iter()
                    // The shell is the one entry a harness may legitimately hold back: Claude Code
                    // does not list `Bash` among its offered tools, and the b10x loop publishes
                    // `run` only where the machine can confine an exec.
                    .filter(|named| {
                        named != "Bash" && named != "run" && !present.contains(&named.as_str())
                    })
                    .collect();
                if !missing.is_empty() {
                    outln!(
                        "note: state `{}` admits {} the session was not offered — {}. The step map \
                         and this harness disagree about the tool set; the model will be refused by \
                         the vendor rather than by the policy, and the turn is spent either way.",
                        context.state,
                        if missing.len() == 1 { "a tool" } else { "tools" },
                        missing.join(", ")
                    );
                }
            }
        }
        if event["event"] == "tool.requested" {
            tally.requested += 1;
        }
        // **`decision_required: false` is a fact and not a silence.** The b10x adapter sets it on
        // every call, beside `Seam::None`, because nothing on that loop adjudicates — so the
        // driver counts what it saw and answers nothing. Writing a `tool.decide` here would be
        // this process claiming a decision the wire says nobody made.
        if event["event"] == "tool.requested" && event["decision_required"] == true {
            tally.asked += 1;
            let call_id = event["call_id"].as_str().unwrap_or_default();
            let name = event["name"].as_str().unwrap_or_default();
            let deny = |reason: String| serde_json::json!({ "decision": "deny", "reason": reason });
            let decision = match decide_tool(context, name, &event["input"]) {
                Err(reason) => deny(format!("the driver's per-call policy refuses: {reason}")),
                // Nothing renders this call as an action — `Skill` and `WebSearch` are the two, and
                // [`action_for`] says why — so the engine is not consulted and the policy's allow
                // stands. Inventing a request would put an act nobody performed in the engine's
                // record, which is invariant 7's failure one layer up.
                Ok(()) => match action_for(name, &event["input"]) {
                    None => serde_json::json!({ "decision": "allow" }),
                    Some(request) => {
                        let verdict = authorize(&request);
                        if verdict.is_allowed() {
                            serde_json::json!({ "decision": "allow" })
                        } else {
                            deny(engine_refusal(&verdict))
                        }
                    }
                },
            };
            if decision["decision"] == "deny" {
                tally.denied += 1;
            }
            let command = serde_json::json!({
                "format": "metaharness.command/1",
                "id": format!("decide-{call_id}"),
                "command": "tool.decide",
                "call_id": call_id,
                "decision": decision,
            });
            // A write that fails means the child is gone; the caller's wait reports how.
            if commands
                .write_all(format!("{command}\n").as_bytes())
                .and_then(|()| commands.flush())
                .is_err()
            {
                break;
            }
        }
    }
    tally
}

/// What the driver was actually asked while one session ran.
///
/// # Why three counts and not one
///
/// A denial count on its own is only readable when something was asking. The claude arm answers
/// every call, so `denied: 0` there genuinely means *nothing this session did was refused by the
/// driver*. The b10x arm answers nothing — `Seam::None`, `decision_required: false` on every
/// `tool.requested` — so `denied: 0` there means *nobody asked*, and a report that printed the
/// same words for both would be reporting an adjudication that never happened. Two runs compared
/// on that number would be compared on an artefact of the instrument.
///
/// So [`Self::requested`] is what the session did, [`Self::asked`] is how much of it reached this
/// process at all, and [`Self::denied`] is what this process refused. `asked == 0` is the state
/// [`Self::line`] refuses to describe as a clean run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Adjudication {
    /// `tool.requested` events seen, whether or not one asked anything.
    requested: u32,
    /// Those that put a decision to this process.
    asked: u32,
    /// Those this process refused, by its own policy or by the engine's.
    denied: u32,
}

impl Adjudication {
    /// The line the run prints about one session's calls.
    ///
    /// A harness that does not adjudicate gets a sentence naming what bounded the run instead of a
    /// count of what this process refused, because the count is `0` for a reason that has nothing
    /// to do with the session's behaviour.
    fn line(self, harness: Harness, state: &StateId) -> String {
        if harness.adjudicates() {
            return format!(
                "note: state `{state}` put {} tool call(s) to the driver and {} were refused.",
                self.asked, self.denied
            );
        }
        format!(
            "note: state `{state}` observed {} tool call(s) and adjudicated none of them — the \
             `{}` loop publishes a toolset computed from what the machine can confine, so a tool \
             outside it does not exist rather than being refused. Nothing here says the run was \
             not refused anything; it says nobody asked this process. What the loop itself \
             refused is in the `{}` transcript beside this run, and the refusal specification for \
             this step is what checks it.",
            self.requested,
            harness.kind(),
            METAHARNESS_EVENT_FORMAT
        )
    }
}

/// The event stream both harnesses write, and the one both are checked from.
const METAHARNESS_EVENT_FORMAT: &str = "metaharness.event/1";

/// The per-call policy: the retired shell hooks, in the driver's own process.
///
/// This is the § 10.1 shape the hooks existed to approximate: the layer that sees a call's
/// *arguments* is the embedder, in Rust, and its verdict reaches the child through the
/// metaharness seam before the call runs. Three checks, first refusal wins, every reason written
/// for the model to act on rather than as a wall:
///
/// 1. **the driven surface** (`Bash`): one simple `protocol artifact|trace` invocation — no
///    pipes, no redirection, no substitution — and no shell at all in a state that does not
///    admit `command.execute`;
/// 2. **the per-state allowlist**: the tool must render from a capability this state admits,
///    which is what `--allowedTools` used to carry (and can no longer, because a bare
///    `--allowedTools` entry auto-approves the whole tool before any seam is consulted);
/// 3. **store integrity** (`Edit`/`Write`/`NotebookEdit`): every planning-store mutation crosses
///    the `protocol artifact` surface, in every state of every workflow.
fn decide_tool(
    context: &StepContext<'_>,
    tool: &str,
    input: &serde_json::Value,
) -> Result<(), String> {
    if tool == "Bash" {
        return driven_surface(context, input);
    }
    let offered = allowed_tools(context.tools);
    if !offered.iter().any(|name| name == tool) {
        return Err(format!(
            "`{tool}` is not offered in state `{}`; this state's tools are: {}",
            context.state,
            offered.join(", ")
        ));
    }
    match tool {
        "Edit" | "Write" | "NotebookEdit" => store_integrity(tool, input),
        _ => Ok(()),
    }
}

/// The planning store has one writer: `protocol artifact`.
///
/// All native file-writing tools are denied under the store. Bodies move through `artifact body`,
/// just as status and relations move through their own verbs; otherwise the same document still
/// has two writers and only one of them validates the store before committing its change.
fn store_integrity(tool: &str, input: &serde_json::Value) -> Result<(), String> {
    let target = match tool {
        "NotebookEdit" => input["notebook_path"].as_str().unwrap_or_default(),
        _ => input["file_path"].as_str().unwrap_or_default(),
    };
    if !target.contains(".engineering/planning/") {
        return Ok(());
    }
    Err(format!(
        "`{tool}` cannot write {target}: planning-store files are mutated only through `protocol \
         artifact`. Use `artifact body <id> --from <path|->` for prose, `artifact move` for \
         status, `artifact relate` for relations, and `artifact new` for creation."
    ))
}

/// The per-state shell surface: one simple invocation of `protocol artifact …` or
/// `protocol trace …`, exactly what the retired `driven-surface.sh` held the grant to.
///
/// The surface lives here and not in any document the run can reach, deliberately: a run that
/// could name its own allowed surface could widen it. Pattern-based and best-effort, as § 4.8
/// says — granting `command.execute` grants a superset of the shell's reach, and this narrows it.
fn driven_surface(context: &StepContext<'_>, input: &serde_json::Value) -> Result<(), String> {
    if !context.tools.shell_offered() {
        return Err(format!(
            "state `{}` does not admit `command.execute`, so this step holds no shell. Anything \
             a suite must observe is run by the driver as a `command` step and recorded with a \
             verifier's provenance, not with yours.",
            context.state
        ));
    }
    let command = input["command"].as_str().unwrap_or_default();
    if let Some(found) = composes(command) {
        return Err(format!(
            "the command composes or redirects, and this run admits one simple invocation at a \
             time: `{command}` — the `{found}` is unquoted. Run one call per Bash tool use; a \
             metacharacter inside quotes is an argument and is fine, so `grep -n \"a\\|b\" file` \
             is one invocation."
        ));
    }
    let mut words = command.split_whitespace();
    let program = words.next().unwrap_or_default();
    let verb = words.next().unwrap_or_default();
    let leaf = program.rsplit('/').next().unwrap_or(program);
    if READ_ONLY_PROGRAMS.contains(&leaf) {
        if context.tools.admits(&Capability::RepositoryRead)
            || context.tools.admits(&Capability::ArtifactRead)
        {
            return Ok(());
        }
        return Err(format!(
            "`{leaf}` reads the repository and state `{}` does not admit `repository.read`.",
            context.state
        ));
    }
    if leaf != "protocol" {
        return Err(format!(
            "`{}` is outside the surface this state admits. A driven step's shell exists so the \
             `protocol` CLI is reachable; it is not a general shell. Build, test and inspection \
             commands are `command` steps the driver runs, and their records carry a verifier's \
             provenance rather than yours.",
            if program.is_empty() {
                "(nothing)"
            } else {
                program
            }
        ));
    }
    if verb != "artifact" && verb != "trace" {
        return Err(format!(
            "`protocol {}` is outside the surface this state admits: `protocol artifact …` and \
             `protocol trace …`. Driving a run from inside a driven step, or moving the store's \
             own governing documents, is not this step's business.",
            if verb.is_empty() { "(no verb)" } else { verb }
        ));
    }
    Ok(())
}

/// The harness name that selects the metaharness executor.
const METAHARNESS_HARNESS: &str = "metaharness";

/// The harness name that selects the b10x loop.
const B10X_HARNESS: &str = "b10x";

/// The binary every `llm` step is spawned through.
const METAHARNESS_BINARY: &str = "metaharness";

/// The loop `metaharness run b10x` spawns, which has to be installed separately.
const B10X_BINARY: &str = "b10x-harness";

/// Which harness an `llm` step is spawned through, and the only place the two differ.
///
/// **§ 4.9 point 3's seam, with a second implementation in it at last.** The design says a second
/// harness is *a second free function chosen by this name*, not a trait added before there is
/// anything to design one against; `story:shell-echo-harness` proved the shape with a fake
/// executor and this is the first real one. What varies between the two is exactly three things —
/// the `metaharness run` kind, the naming table a shared capability decision renders into, and
/// whether the seam puts a decision to this process at all — and they are enumerated here so a
/// third harness has to answer the same three questions rather than discover them.
///
/// What deliberately does **not** vary: [`metaharness_operations`], which is the neutral
/// vocabulary the frame and the refusal specification are written in, and
/// `aep_driver::tool::tool_config`, which decides what a capability admits. A harness that decided
/// for itself could quietly re-admit a shell the state never granted, which is the one thing point
/// 2 exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Harness {
    /// Claude Code, driven through `metaharness run claude` in ask mode: every call is put to this
    /// process and answered before it runs.
    ClaudeCode,
    /// The b10x loop, spawned through `metaharness run b10x` and **observed**.
    ///
    /// It adjudicates nothing, by design and not by omission: the loop's published toolset is
    /// computed from what the machine can confine, so a tool outside the surface does not exist
    /// rather than being refused, and a seam that adjudicated its calls would put the driven arm's
    /// treatment back on top of the arm that exists to measure its absence.
    B10x,
}

impl Harness {
    /// The harness a step's `harness:` field names, or nothing this build can invoke.
    fn named(name: &str) -> Option<Self> {
        match name {
            // `claude-code` names the vendor and `metaharness` is the name the executor first
            // landed under; both reach the same invocation.
            LlmStep::DEFAULT_HARNESS | METAHARNESS_HARNESS => Some(Self::ClaudeCode),
            B10X_HARNESS => Some(Self::B10x),
            _ => None,
        }
    }

    /// Every name this build invokes, for a refusal that lists them rather than hinting.
    const NAMES: [&'static str; 3] = [LlmStep::DEFAULT_HARNESS, METAHARNESS_HARNESS, B10X_HARNESS];

    /// The `metaharness run` kind.
    fn kind(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::B10x => B10X_HARNESS,
        }
    }

    /// This harness's own names for an admitted capability set.
    ///
    /// The rendering half of § 4.9 point 2. Both arms read the same [`ToolConfig`] and neither
    /// decides anything about it.
    fn tools(self, config: &ToolConfig) -> Vec<String> {
        match self {
            Self::ClaudeCode => allowed_tools(config),
            Self::B10x => b10x_tools(config),
        }
    }

    /// Whether this harness's seam puts a decision to the driver before a call runs.
    ///
    /// `false` for b10x, and the run report has to say so in those words rather than reporting a
    /// denial count of zero: *nobody asked me* and *nothing was refused* are different findings
    /// and only one of them is about the run.
    fn adjudicates(self) -> bool {
        matches!(self, Self::ClaudeCode)
    }
}

/// The b10x loop's tool names for an admitted capability set.
///
/// The second naming table, and the reason § 4.9 point 2 puts the *decision* somewhere else: this
/// function reads the same [`ToolConfig`] [`allowed_tools`] reads and renames its answer. Nothing
/// here consults a capability the shared decision did not already admit.
///
/// The names are `b10x-harness-tools`' own catalogue entries — `file_read`, `file_write`,
/// `file_edit`, `dir_list`, `search`, `run` — read from `entry_names()` there rather than invented
/// here. Three neutral operations the Claude Code table renders have **no entry at all** in that
/// catalogue and are therefore rendered by nothing:
///
/// | operation | Claude Code | b10x |
/// |---|---|---|
/// | `web.read` | `WebFetch`, `WebSearch` | *no entry* — the loop has no web tool |
/// | `skill.load` | `Skill` | *no entry* — the loop has no skill mechanism |
/// | `subagent.spawn` | never offered | *no entry*, and never offered either |
///
/// A capability this table cannot render is **not** silently downgraded: `network.read` stays
/// admitted by the policy and the session simply has no tool for it, which the session-start audit
/// in [`answer_events`] reports against the same list. Rendering it as something else would be the
/// second, weaker policy point 2 forbids.
fn b10x_tools(config: &ToolConfig) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        tools.extend(["dir_list", "file_read", "search"].map(ToOwned::to_owned));
    }
    if config.admits(&Capability::RepositoryWrite) {
        tools.extend(["file_edit", "file_write"].map(ToOwned::to_owned));
    }
    if config.shell_offered() {
        // `run` and not a shell: the entry takes an argv list, composes nothing and starts only a
        // declared program. See [`b10x_argv`] for why the declaration travels on the launch.
        tools.push("run".to_owned());
    }
    tools.sort();
    tools.dedup();
    tools
}

/// Refuses a run before it is allocated when the seam's binary is not installed.
///
/// **A launch-time check for a launch-time fact.** Without it the missing binary is discovered at
/// the first `llm` step, as a [`StepOutcome::NoVerdict`] — by which point the run has a directory,
/// an id, the store lock and a snapshot, and the report says *no verdict* for something that was
/// never a verdict: nothing was observed because nothing was ever run. `NoVerdict` is D5's
/// `Unknown` and this is not unknown, it is decidable from `PATH` before a cent or a lock is spent.
///
/// Scoped to maps that have an `llm` step, because that is the only kind of step that spawns it: a
/// map of `command` and `operator` steps drives correctly on a machine with no vendor and no
/// metaharness, and refusing that run would be refusing work the driver can do.
///
/// The refusal answers the question it creates, which is this repository's posture for every
/// refusal — it names the one command that installs the binary.
/// Whether the map has any step that spawns a model, which is what both launch pre-flights are about.
fn has_llm_steps(map: &StepMap) -> bool {
    llm_step_count(map) > 0
}

/// How many, for a refusal that says how much is at stake.
fn llm_step_count(map: &StepMap) -> usize {
    map.states
        .values()
        .flat_map(|state| state.steps.iter())
        .filter(|step| matches!(step, Step::Llm(_)))
        .count()
}

fn metaharness_preflight(map: &StepMap) -> Option<String> {
    let llm_steps = llm_step_count(map);
    if llm_steps == 0 || on_path(METAHARNESS_BINARY) {
        return None;
    }
    Some(format!(
        "this map has {llm_steps} `llm` step(s) and `{METAHARNESS_BINARY}` is not on PATH.\n\
         \n\
         Every `llm` step is spawned through `{METAHARNESS_BINARY} run <harness>`, whichever \
         harness the step names: on `{}` the step's surface travels as a sealed frame document \
         and this process answers every tool call the session makes. There is no path around it \
         — the bare vendor argv was retired with `epic:metaharness-migration`, because a second \
         way to launch a session is a second policy to forget.\n\
         \n\
         Install it with `cargo install --path crates/metaharness-cli` from a metaharness checkout, \
         or drive a map whose steps are all `command` and `operator` steps, which needs neither.",
        LlmStep::DEFAULT_HARNESS
    ))
}

/// Every *this machine cannot run it today* pre-flight, in the order they are answered.
///
/// Three checks, and the order is the one a person can act on: the seam's binary, then the CLI a
/// driven session reaches the store through, then everything a `harness: b10x` step needs. Each is
/// decidable before a run id, a lock, a snapshot or a model bill exists, which is the whole
/// argument for them being here rather than at the first `llm` step.
///
/// The evidence-coverage check is deliberately **not** folded in and runs before this: it is
/// decidable from the two documents alone and says *this map can never finish this plan* on every
/// machine, so a real coverage gap must not be hidden behind a binary that happens to be missing.
///
/// The read-only note is printed rather than returned, because it refuses nothing: a b10x step in
/// a state that only reads is legitimate work.
fn machine_preflights(map: &StepMap, project: &Path, b10x: &B10xOptions) -> Option<String> {
    if let Some(refusal) = metaharness_preflight(map) {
        return Some(refusal);
    }
    if has_llm_steps(map) {
        if let Some(refusal) = protocol_on_the_session_path() {
            return Some(refusal);
        }
    }
    if let Some(refusal) = b10x_preflight(map, b10x) {
        return Some(refusal);
    }
    if let Some(note) = b10x_read_only_note(map, project) {
        outln!("note: {note}");
    }
    None
}

/// How many `llm` steps name the b10x loop.
fn b10x_step_count(map: &StepMap) -> usize {
    map.states
        .values()
        .flat_map(|state| state.steps.iter())
        .filter(|step| matches!(step, Step::Llm(step) if step.harness == B10X_HARNESS))
        .count()
}

/// Whether the installed `metaharness` publishes an adapter for this kind.
///
/// **The one pre-flight here that spawns, and the exception is argued rather than assumed.**
/// [`on_path`] refuses to run a binary to find out whether it exists, because that is a side
/// effect in a check and because a binary that exists and then fails is a different finding. This
/// asks a different question — *does this install know the kind at all* — which `PATH` cannot
/// answer and which nothing else can either: an adapter is compiled in, so an older binary under
/// the same name and the same `--version` refuses `b10x` as an invalid argument at the first step.
///
/// `capabilities` is the verb metaharness publishes for exactly this. Its own design says it
/// *"exists so an embedder can refuse early rather than discovering mid-run that a tier is
/// absent"*, and it is one of the three verbs that "work with no model and no credential": it
/// prints a value, reaches no network and spends nothing. Everything is discarded — the answer
/// wanted is the exit status.
fn metaharness_knows(kind: &str) -> bool {
    Process::new(METAHARNESS_BINARY)
        .arg("capabilities")
        .arg(kind)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Refuses a run whose `llm` steps name a harness this machine cannot spawn.
///
/// The third of the launch-time pre-flights and the same argument as the two beside it: a map that
/// names `b10x` on a machine with no b10x loop, no endpoint or no model is decidable from the
/// documents and the filesystem, and finding it out at the first `llm` step costs a run id, the
/// store lock, a snapshot and — on the arms that get that far — a model bill, for a
/// [`StepOutcome::NoVerdict`] that is not unknown at all.
///
/// Four checks, first refusal wins, each naming what to install or declare. The two that are
/// decidable anywhere come first — an invocation that names no endpoint is wrong on every machine
/// — and the two about *this* machine follow, so a missing loop cannot mask a missing flag.
fn b10x_preflight(map: &StepMap, options: &B10xOptions) -> Option<String> {
    let steps = b10x_step_count(map);
    if steps == 0 {
        return None;
    }
    // **The two facts about the invocation come first, and the order is load-bearing** — the same
    // lesson `start`'s coverage check records. These are decidable on every machine; the two below
    // them say *this machine cannot run it today*. With the machine checks first, a run that named
    // no endpoint at all would read as fine wherever the loop happened to be missing, and the test
    // asserting it is refused would pass vacuously in CI.
    if options.endpoint.is_none() {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and no \
             `--b10x-endpoint` was given.\n\
             \n\
             The loop is pointed at an endpoint by its caller and has no service of its own to \
             fall back on, and metaharness refuses to default one: a default would aim a driven \
             run at somebody's production API the first time the flag was forgotten. It is a fact \
             about this machine rather than about the work, which is why the step map cannot \
             carry it.\n\
             \n\
             Pass the gateway's root URL as `--b10x-endpoint`, and the model it serves as \
             `--b10x-model`."
        ));
    }
    if options.model.is_none() {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and no \
             `--b10x-model` was given. The endpoint serves several and the loop picks none."
        ));
    }
    let path = session_path();
    let installed = path
        .split(':')
        .any(|directory| Path::new(directory).join(B10X_BINARY).is_file());
    if !installed {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and \
             `{B10X_BINARY}` is not on the `PATH` the run will give its child.\n\
             \n\
             That `PATH` is `{path}` — **constructed by metaharness, not inherited** (H3) — so a \
             loop the operator can run is not automatically one the run can. It is the same \
             constructed `PATH` the `protocol` CLI has to be installed onto, and for the same \
             reason.\n\
             \n\
             Install it where the run will find it:\n\
             \n\
                 cargo install --path crates/harness-cli --root ~/.local\n\
             \n\
             from a `beyond10x/harness` checkout, or drive this map's `llm` steps on \
             `{}` instead.",
            LlmStep::DEFAULT_HARNESS
        ));
    }
    if !metaharness_knows(B10X_HARNESS) {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and the \
             installed `{METAHARNESS_BINARY}` does not publish an adapter for it.\n\
             \n\
             The adapter is compiled in, so an install predating it carries the same name and the \
             same `--version` and refuses `{B10X_HARNESS}` as an invalid argument at the first \
             step — after the run id, the lock and the snapshot. `{METAHARNESS_BINARY} \
             capabilities {B10X_HARNESS}` is the question that was asked and it did not answer.\n\
             \n\
             Reinstall it from a metaharness checkout that has the adapter:\n\
             \n\
                 cargo install --path crates/metaharness-cli --root ~/.local"
        ));
    }
    None
}

/// What a driven b10x session will not be able to do here, said before it is paid for.
///
/// **Not a refusal, because a read-only session is legitimate work.** A `specify` or a `review`
/// state that admits `repository.read` and nothing else drives perfectly well on this arm. What
/// would be wrong is a `implement` state discovering it, one turn at a time, in a session that was
/// told it had `file_write`.
///
/// The rule is metaharness's and it is a naming rule: substrate represents a workspace only when
/// its directory name starts with `ws_`, and a confined launch over a directory it cannot adopt is
/// refused rather than degraded. A driven run's working directory is the operator's repository, so
/// no driven b10x session is confined, so the loop publishes only the three reading entries — the
/// toolset is computed from what the machine can confine, and unconfined that is reading.
///
/// It is a note and not a refusal for a second reason: what a state admits is decided per state by
/// the engine at run time, and a pre-flight reading a map cannot know whether any state will reach
/// for a write.
fn b10x_read_only_note(map: &StepMap, working_directory: &Path) -> Option<String> {
    if b10x_step_count(map) == 0 {
        return None;
    }
    Some(format!(
        "a driven `{B10X_HARNESS}` session is **read-only** over {}. The loop publishes the tools \
         the machine can confine, substrate represents a workspace only when its directory name \
         starts with `ws_`, and a governed tree is the operator's repository — so no driven \
         session of this arm is confined and `file_write`, `file_edit` and `run` are not \
         published to it. A state that admits `repository.write` will say so at session start, in \
         the note beside this one.",
        working_directory.display()
    ))
}

/// What a run was started with, written beside its cursor so `resume` does not have to be told again.
///
/// **The printed resume line did not work, and that is the whole reason this exists.** A stopped
/// run prints `resume with: protocol drive resume <run>`; that command re-read none of `--map`,
/// `--task`, `--pause-on-approval` or `--plugin-dir`, so an operator who typed exactly what the
/// driver told them to type got a different run — a different map, no pause, no plugin — or an
/// error. It was recorded as F-W4.2-4 on 2026-08-24 and answered by observation: *the line as
/// printed does not work*.
///
/// Beside the cursor rather than inside it: the cursor is `aep.driver-cursor/1`, a published
/// document about **where a run is**, and how it was launched is not that. A missing or unreadable
/// launch record is not an error — a run started before this existed resumes exactly as it did,
/// from the flags the caller passes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Launch {
    /// The task document, as given.
    task: Option<PathBuf>,
    /// The map, as given — a path or an id, whichever the caller used.
    map: Option<String>,
    /// The project directory the run was started against.
    project: Option<PathBuf>,
    /// The document tree.
    root: Option<PathBuf>,
    /// Whether the run may stop at an approval.
    pause_on_approval: bool,
    /// The plugin directories the sessions loaded.
    plugin_dirs: Vec<PathBuf>,
    /// What a `harness: b10x` step was pointed at.
    ///
    /// `#[serde(default)]` on every field of it, so a run started before this existed reads an
    /// empty value and resumes exactly as it did — which for a map with no b10x step is what it
    /// had anyway.
    #[serde(default)]
    b10x: B10xOptions,
}

impl Launch {
    /// `<run>/launch.json`.
    fn path(run_directory: &Path) -> PathBuf {
        run_directory.join("launch.json")
    }

    /// Writes it, and says nothing if it cannot: a run that walks and does not record how it was
    /// launched is worse off at resume time and is not wrong now.
    fn write(&self, run_directory: &Path) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::path(run_directory), text + "\n");
        }
    }

    /// Reads it, or `None` for a run started before this existed.
    fn read(run_directory: &Path) -> Option<Self> {
        let text = fs::read_to_string(Self::path(run_directory)).ok()?;
        serde_json::from_str(&text).ok()
    }
}

/// The `PATH` a driven session will actually have, which is **not** this process's.
///
/// metaharness constructs the child environment rather than inheriting it — `env_clear()` then an
/// allowlist, plus a `PATH` computed as `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin`
/// (`metaharness-claude`'s `child_path`, which that crate makes public precisely so a pre-flight
/// can resolve a binary *the way the spawn will*). So a `target/debug` on the operator's `PATH`
/// reaches this process and never the session, and exporting one before `protocol drive` changes
/// nothing about what the model can run.
///
/// Replicated here rather than depended on: this repository takes `entity-runtime` and nothing
/// else, and one shared constant across that boundary would be a dependency in the direction
/// `adr/0002` refuses. `a_session_path_matches_what_metaharness_constructs` pins the two together,
/// so a change on that side fails here rather than in a paid run.
fn session_path() -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => {
            format!("{home}/.local/bin:/usr/local/bin:/usr/bin:/bin")
        }
        _ => "/usr/local/bin:/usr/bin:/bin".to_owned(),
    }
}

/// Refuses a run whose `llm` steps are told to use `protocol` when the session will not have it.
///
/// **Run `W4-3/1`, 2026-08-28, is why, and it cost $1.03 to find out.** The map's steps say *record
/// it in the planning store*, and the store's only route is the `protocol` CLI — the state's shell
/// exists for that and admits nothing else. The session ran `protocol artifact --help` and got
/// `exit 127, command not found`, four times across two states, because the constructed `PATH`
/// holds no `target/debug`. Every guard held and the run was simply unable to do its work.
///
/// It is the same shape as the metaharness pre-flight above and sits beside it for the same reason:
/// a run that cannot do its work should not own a run id, a lock and a model bill to discover that.
fn protocol_on_the_session_path() -> Option<String> {
    let path = session_path();
    let found = path.split(':').any(|directory| {
        let candidate = Path::new(directory).join("protocol");
        candidate.is_file()
    });
    if found {
        return None;
    }
    Some(format!(
        "a driven `llm` step reaches the planning store through the `protocol` CLI, and the \n\
         session's `PATH` does not hold it.\n\
         \n\
         That `PATH` is `{path}` — **constructed by metaharness, not inherited**, so exporting \n\
         `target/debug` before this command changes what *this* process can run and nothing about \n\
         what the model can. A run started anyway walks its states, is refused `protocol` by the \n\
         shell with `exit 127`, and submits nothing: run `W4-3/1` did exactly that on 2026-08-28 \n\
         for $1.03.\n\
         \n\
         Install this build where the session will find it — `--root ~/.local`, because cargo's \n\
         own default is `$CARGO_HOME/bin` and that directory is **not** on the constructed \n\
         `PATH` either:\n\
         \n\
             cargo install --path crates/protocol-cli --root ~/.local\n\
         \n\
         Install rather than symlink `target/debug`: a later `cargo build` replaces that binary \n\
         with a different version than the one this run is recorded against, and a run whose \n\
         evidence was produced by a build nobody can name is the defect `version-check` exists for."
    ))
}

/// Whether `program` is on `PATH` as a file that is there to be executed.
///
/// A lookup and never a spawn: running the binary to find out whether it exists is a side effect in
/// a pre-flight, and a binary that exists and then fails is a different finding — that one is a
/// step with no verdict, which is what the retry budget is for.
///
/// `pub(crate)` because [`crate::eval`] drives the same binary as a tool and asks the same question
/// before spending anything. One lookup, so the two verbs cannot disagree about whether it is
/// installed.
pub(crate) fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

/// The format tag the frame document carries, as the metaharness design § 5.5 spells it.
const METAHARNESS_FRAME_FORMAT: &str = "metaharness.frame/1";

/// The metaharness operations for an admitted capability set.
///
/// The same decisions as [`allowed_tools`], spelled in metaharness's § 5.2 vocabulary instead of
/// the vendor's: the protocol decides what a capability admits, both tables only render it, and
/// `subagent.spawn` is never offered for the same reason `Task` never is.
fn metaharness_operations(config: &ToolConfig) -> Vec<&'static str> {
    let mut operations: Vec<&'static str> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        operations.extend(["file.read", "dir.list", "search"]);
    }
    if config.admits(&Capability::RepositoryWrite) {
        operations.extend(["file.write", "file.edit"]);
    }
    if config.admits(&Capability::NetworkRead) {
        operations.push("web.read");
    }
    if config.shell_offered() {
        operations.push("shell");
    }
    if config.skills_offered() {
        operations.push("skill.load");
    }
    operations.sort_unstable();
    operations.dedup();
    operations
}

/// Every operation the table can render, whatever a policy admits.
///
/// Computed by asking [`metaharness_operations`] about a configuration that admits everything,
/// rather than written out again. Two lists would drift, and the one that drifts is the one nobody
/// looks at: a hand-written vocabulary missing `file.edit` would emit a specification that never
/// checks for an edit and reports green.
fn every_operation() -> Vec<&'static str> {
    metaharness_operations(&ToolConfig::new(TOOL_CANDIDATES.iter().cloned().collect()))
}

/// The operations this step's policy did **not** admit.
///
/// # What this is for, and what it is not
///
/// Gap register `:40`. Design § 4.8 row 3 promised the per-state tool set would be *audited*: the
/// allowlist at session launch, the hook over the same derived set, and an expectation kind reading
/// back what the session was actually given. `env.tool_available` shipped and then showed it reads
/// the harness's tool **inventory**, not the session's allow rules — the committed fixture was
/// launched with nine allowed tools and lists thirty-two.
///
/// The record that would settle it is the harness's to write and does not exist. This is the other
/// route the register names, and it is **strictly weaker**, which is why it says so out loud: it
/// catches a tool that was offered *and used*, and cannot see one that was offered and never
/// reached for. A refused operation that never appears in the transcript is the same evidence as an
/// operation nobody wanted. What it does close is the case that matters — a run that did something
/// its state was not allowed to do now fails a check instead of passing unexamined.
fn refused_operations(config: &ToolConfig) -> Vec<&'static str> {
    let admitted = metaharness_operations(config);
    every_operation()
        .into_iter()
        .filter(|operation| !admitted.contains(operation))
        .collect()
}

/// The step's refused operations as a `trace.spec/1` document.
///
/// One `tool.absent` row per refused operation, keyed by the **neutral operations** vocabulary and
/// never by a vendor's tool names. Naming tools here would make the specification decidable against
/// one harness and silently vacuous against every other — a row saying `tools: [Edit, Write]` selects
/// nothing at all on a harness that spells a write `workspace_write`, and reports green for it.
///
/// `severity: gate` and `on_unknown: gap`: a transcript that cannot say whether a refused
/// operation happened is not evidence that it did not. The whole point is to stop reading silence
/// as compliance.
fn refusal_specification(
    state: &aep_domain::ids::StateId,
    index: usize,
    config: &ToolConfig,
) -> Option<serde_json::Value> {
    let expectations: Vec<serde_json::Value> = refused_operations(config)
        .into_iter()
        .map(|operation| {
            serde_json::json!({
                // Dashes, not the dots the operation is spelled with: an expectation id is
                // lowercase letters, digits and dashes, and the checker refuses anything else.
                "id": format!("refused-{}", operation.replace('.', "-")),
                "statement": format!(
                    "step {index} of `{state}` was not admitted `{operation}`, \
                     so the run must not contain one"
                ),
                "severity": "gate",
                "on_unknown": "gap",
                "expect": { "tool.absent": { "operations": [operation] } },
            })
        })
        .collect();
    // `None` when nothing was refused, because `trace-spec/1` refuses a specification with no
    // expectations — *"a report with no content reads exactly like a report with no gaps"* — and
    // that rule is right and older than this. Absence is still readable: the frame document for the
    // same step is written unconditionally, so a frame with no refusal file beside it means this
    // state was admitted everything, and no frame at all means the step never ran.
    if expectations.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "format": "trace-spec/1",
        // One `/`, between a namespace and a name: `driver/<state>-<index>`.
        "id": format!("driver/{}-{index}", state.to_string().replace('.', "-")),
        "title": format!("what step {index} of `{state}` was not allowed to do"),
        "expectations": expectations,
    }))
}

/// The step as a sealed `metaharness.frame/1` document.
///
/// Built as plain JSON and sealed by the document's own rule — SHA-256, hex, over the compact
/// serialization with keys sorted at every level (`serde_json`'s default map order) and the
/// `digest` and `format` fields absent — so this binary produces byte-for-byte what metaharness
/// verifies, without linking its crates. The obligations and reaching lines are the engine's own
/// words, verbatim, on the same rule as the prompt: a summary here would be the only place the
/// summary existed.
fn metaharness_frame(
    context: &StepContext<'_>,
    workflow_id: &str,
    workflow_version: &str,
) -> serde_json::Value {
    let line = |text: &String| serde_json::json!({ "text": text, "asked_by": null });
    let mut frame = serde_json::json!({
        "workflow": { "id": workflow_id, "version": workflow_version },
        "node": { "id": context.state.to_string() },
        "step": {
            "workflow": workflow_id,
            "state": context.state.to_string(),
            "index": context.index,
            "attempt": context.attempt,
        },
        "prior": [],
        "obligations": context.requirements.iter().map(line).collect::<Vec<_>>(),
        "reaching": context.reaching.iter().map(line).collect::<Vec<_>>(),
        "next": [],
        "handoff": { "handoff": "none" },
        "operations": metaharness_operations(context.tools)
            .iter()
            .map(|operation| serde_json::json!({ "op": operation }))
            .collect::<Vec<_>>(),
        "entities": null,
    });
    let digest = {
        use sha2::{Digest as _, Sha256};
        let bytes = serde_json::to_vec(&frame).expect("a frame value serialises");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    let object = frame.as_object_mut().expect("a frame is an object");
    object.insert("digest".into(), digest.into());
    object.insert("format".into(), METAHARNESS_FRAME_FORMAT.into());
    frame
}

/// A minted frame as the bytes that go on disk.
///
/// Pretty-printed with a trailing newline, which is exactly what metaharness's own
/// `Frame::to_document` writes, so the two producers of this file agree byte for byte and a
/// document minted here can be diffed against one minted there. Split out of the write so the
/// committed golden under `fixtures/` is *these* bytes and not a second rendering of them: a golden
/// produced by a path the driver does not take would pin the test and not the driver.
///
/// # Errors
///
/// When the frame will not serialise, which for a value built by [`metaharness_frame`] would be a
/// defect here rather than anything a caller did.
fn frame_document(frame: &serde_json::Value) -> Result<String, String> {
    let text = serde_json::to_string_pretty(frame)
        .map_err(|error| format!("the frame would not serialise: {error}"))?;
    Ok(format!("{text}\n"))
}

/// The `metaharness run claude` invocation for one step.
///
/// `--cwd` is the metaharness a6 declaration: the session works in the governed tree, and
/// metaharness attests the two hermetic rows that costs instead of claiming them. `--decisions
/// frame` makes metaharness the per-call decider from the frame's admitted set. The plugins
/// still travel for their skills; their hooks read a step context this launch does not carry and
/// no-op, which is the intended shape — one policy, one enforcer.
fn metaharness_argv(
    frame: &Path,
    working_directory: &Path,
    plugin_dirs: &[PathBuf],
    prompt: &str,
) -> Vec<String> {
    let mut argv = vec![
        METAHARNESS_BINARY.to_owned(),
        "run".to_owned(),
        "claude".to_owned(),
        "--hermetic".to_owned(),
        "--cwd".to_owned(),
        working_directory.display().to_string(),
        "--frame".to_owned(),
        frame.display().to_string(),
        "--decisions".to_owned(),
        "ask".to_owned(),
        "-p".to_owned(),
        prompt.to_owned(),
    ];
    for directory in plugin_dirs {
        argv.push("--plugin-dir".to_owned());
        argv.push(directory.display().to_string());
    }
    argv
}

/// One write scope as `--write-scope`'s grammar spells it.
///
/// Total and written out rather than taken off the type's `Serialize`, so a rule the map gains
/// later fails to compile here instead of reaching an argv as an empty word. It coincides with the
/// map's own kebab-case wire form and that is not an accident worth relying on silently —
/// `the_write_scope_words_are_the_ones_the_step_map_is_written_in` asserts the two agree.
fn write_scope_word(scope: WriteScope) -> &'static str {
    match scope {
        WriteScope::Allowed => "allowed",
        WriteScope::PartialOnly => "partial-only",
        WriteScope::Denied => "denied",
    }
}

/// The `metaharness run b10x` invocation for one step.
///
/// # Why there is no `--frame` here, and why that is not a shortcut
///
/// The claude arm's surface travels twice — the sealed frame document *and* a per-call answer —
/// because F9 says a frame whose text reaches the model while nothing enforces it tells the model
/// *"strictly only these operations"* and makes it false. metaharness enforces that rule on its own
/// side: `required_commands` adds `tool.decide` to any spec carrying a frame, the b10x adapter
/// refuses `tool.decide` because nothing on that loop ever asks, and so **`metaharness run b10x
/// --frame …` is refused before a model is reached**. `--decisions observe` is refused for the same
/// reason, and `--decisions frame` — the default, and what this argv leaves in place — is the only
/// one the adapter admits.
///
/// The frame is still *minted and written* beside the transcript for a b10x step. It is the record
/// of what the step was, in the neutral vocabulary both arms are checked in, and the refusal
/// specification beside it is derived from the same [`ToolConfig`]. What changes is that on this
/// arm the document is evidence about the step rather than an instruction to a seam.
///
/// # What the surface travels as instead
///
/// `--write-scope` and `--context`, which are the b10x-only spec fields that exist for exactly this
/// — the loop has no seam, so *"for that kind the scope has to travel to the tools"*. Both come
/// off the step map's own `scope:` and `context:` keys, which no other executor reads.
///
/// # What is deliberately not asked for, because it cannot be had over a governed tree
///
/// No `--substrate-embedded`, no `--substrate` and no `--cgroup-root`. substrate represents a
/// workspace only when its directory name starts with `ws_`, and the working directory of a driven
/// run is the operator's repository — metaharness refuses a confined launch over a directory it
/// cannot adopt rather than degrading it. So a driven b10x session is **read-only**: the loop
/// publishes what the machine can confine, and with no confinement that is the three reading
/// entries. Asking for confinement here would turn every driven b10x step into a launch refusal;
/// not asking for it makes the limitation visible where it can be acted on, in [`b10x_preflight`]
/// before the run and in the session-start audit during it.
fn b10x_argv(
    options: &B10xOptions,
    working_directory: &Path,
    scope: &[ScopeRule],
    context_files: &[String],
    prompt: &str,
) -> Vec<String> {
    let mut argv = vec![
        METAHARNESS_BINARY.to_owned(),
        "run".to_owned(),
        B10X_HARNESS.to_owned(),
        "--hermetic".to_owned(),
        "--cwd".to_owned(),
        working_directory.display().to_string(),
        "--model-endpoint".to_owned(),
        options.endpoint.clone().unwrap_or_default(),
        "--model".to_owned(),
        options.model.clone().unwrap_or_default(),
        "--credentials".to_owned(),
        // `operator-login` is the flag's default and names nothing on this loop, which refuses it
        // rather than launching a run with no credential under a flag that claims one.
        if options.api_key { "api-key" } else { "none" }.to_owned(),
    ];
    for rule in scope {
        for path in &rule.paths {
            argv.push("--write-scope".to_owned());
            // `<glob>=<allowed|partial-only|denied>`, ordered, first match wins — which is why the
            // rules are pushed in the order the map wrote them and never sorted.
            argv.push(format!("{path}={}", write_scope_word(rule.write)));
        }
    }
    for file in context_files {
        argv.push("--context".to_owned());
        argv.push(file.clone());
    }
    argv.push("-p".to_owned());
    argv.push(prompt.to_owned());
    argv
}

/// The instant the driver just observed something, from the wall clock.
///
/// The driver runs the program and reads its exit status, so *now* is the truthful observation
/// time — this is the one case where the two times an evidence record carries legitimately
/// coincide, and it is stated rather than assumed. It lives in `protocol-cli` and not in a pure
/// crate for the reason the store lock does: reading ambient OS state is this binary's job.
fn observed_now() -> ObservedAt {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    ObservedAt::new(Timestamp::from_epoch_millis(millis))
}

/// Reads the record a verifier wrote for itself, and submits what the document says.
///
/// The other half of `mint`, and the reason both exist. `mint` builds a record from an exit status,
/// which is honest for a suite and impossible for a check whose record carries digests and counts:
/// a `trace_conformance` minted from `exit 0` would state a specification digest nobody computed.
/// So a verifier that can write its own record does, and the driver's whole job here is to read it
/// — which is the same thing `protocol evaluate --evidence` does with a file a person points at.
///
/// Three refusals, each of them D5's `Unknown` rather than a failing verdict:
///
/// * **no document** — the program was to write one and did not, so nothing was observed;
/// * **more than one record** — a step establishes one thing, and picking one of several would be
///   the driver choosing what the run is about;
/// * **an approval, or anything a person is recorded as having produced** — invariant 7 at this
///   layer. A run's own step must not be able to hand the engine a human's approval read out of a
///   file; that record enters through a person and `protocol evaluate --evidence`, never here.
fn read_record(
    declared: &str,
    mapping: &EvidenceMapping,
    command: &str,
    context: &StepContext<'_>,
) -> StepOutcome {
    let path = match expand(declared, context) {
        Ok(path) => PathBuf::from(path),
        Err(reason) => return StepOutcome::NoVerdict { reason },
    };
    let no_verdict = |reason: String| StepOutcome::NoVerdict { reason };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            return no_verdict(format!(
                "`{command}` was to write a `{}` record at {} and {error}, so nothing was observed",
                mapping.kind.as_str(),
                path.display()
            ))
        }
    };
    let origin = path.display().to_string();
    let inputs = match aep_schema::parse::evidence_list(&text, Some(&origin)) {
        Ok(inputs) => inputs,
        Err(error) => {
            return no_verdict(format!(
                "the record `{command}` wrote does not read: {error}"
            ))
        }
    };
    let held = inputs.len();
    let Some(input) = inputs.into_iter().next().filter(|_| held == 1) else {
        return no_verdict(format!(
            "a step establishes one thing, and the record `{command}` wrote at {} holds {held}",
            path.display()
        ));
    };
    if input.evidence.kind() != mapping.kind {
        return no_verdict(format!(
            "the step declares `{}` and the record `{command}` wrote is a `{}`",
            mapping.kind.as_str(),
            input.evidence.kind().as_str()
        ));
    }
    if matches!(input.evidence, Evidence::Approval(_))
        || matches!(input.producer, Producer::Human { .. })
    {
        return no_verdict(format!(
            "the record at {} is an approval or is recorded as a person's, and a driven step \
             cannot submit one: an approval reaches an execution through a person running \
             `protocol evaluate --evidence`",
            path.display()
        ));
    }
    StepOutcome::Observed(Box::new(crate::submission(input)))
}

/// Turns a verdict into the evidence the map said it establishes.
///
/// The per-kind rule, in one place: three kinds carry a verdict and can therefore say *no*; `diff`
/// has no failing form — a `ChangeSet` cannot state that no change happened — so a failed
/// observation of one is an absence rather than a `False`, and absence is spelled *submit nothing*.
fn mint(
    mapping: &EvidenceMapping,
    passed: bool,
    command: &str,
    observed_at: ObservedAt,
) -> Option<EvidenceSubmission> {
    let evidence = match mapping.kind {
        EvidenceKind::TestResult => {
            let suite = mapping.suite.clone().unwrap_or(TestSuite::Unit);
            Evidence::TestResult(if passed {
                TestResult::passing(suite, 1)
            } else {
                TestResult::failing(suite, 0, 1)
            })
        }
        EvidenceKind::StaticAnalysis => Evidence::StaticAnalysis(StaticAnalysisResult {
            tool: mapping.tool.clone(),
            errors: usize::from(!passed),
            warnings: 0,
        }),
        EvidenceKind::ContractResult => Evidence::ContractResult(ContractResult {
            checked: 1,
            failed: usize::from(!passed),
            breaking_changes: 0,
            consumer: None,
            provider: None,
        }),
        // The counts are zero because nothing read them: an exit status carries no numbers, and a
        // fabricated count is worse than a missing one — the engine cannot tell an invented number
        // apart from an observed one. What the record establishes is `diff.exists`, which is what
        // the shipped workflow's guard reads.
        EvidenceKind::Diff if passed => Evidence::Diff(ChangeSet {
            files_changed: 0,
            lines_added: 0,
            lines_removed: 0,
            revision_before: None,
            revision_after: None,
            paths: Vec::new(),
        }),
        _ => return None,
    };

    let mut submission = EvidenceSubmission::new(
        evidence,
        // A verifier produced it, because a verifier produced it: the driver ran the program and
        // read its exit status. Nothing about a model's opinion of the run enters the record, which
        // is how `independent: true` is honestly satisfied.
        Producer::Verifier {
            verifier: mapping.verifier.clone(),
        },
        // And the observation happened when the program ran, which for this driver is now. That is
        // the honest value here and it is passed in rather than read here, so the one place this
        // binary reads a wall clock stays countable.
        observed_at,
    );
    submission.subject.clone_from(&mapping.subject);
    submission.provenance = Provenance {
        command: Some(command.to_owned()),
        tool: mapping.tool.clone().or_else(|| tool_of(&mapping.verifier)),
        ..Provenance::default()
    };
    Some(submission)
}

/// The tool a verifier names, when it names one.
fn tool_of(verifier: &Verifier) -> Option<ToolRef> {
    match verifier {
        Verifier::ExternalTool(tool) => Some(tool.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::capability::Environment;
    use aep_domain::ids::StateId;
    use aep_driver::executor::StepAttempt;

    fn config(capabilities: &[Capability]) -> ToolConfig {
        ToolConfig::new(capabilities.iter().cloned().collect())
    }

    /// One `llm` step, as a step map that names the second harness would produce it.
    ///
    /// The harness is spelled as a literal rather than through a constant, deliberately: this is
    /// the string a step map author writes, and a test that read it out of the same constant the
    /// selector reads would pass whatever that constant said.
    fn b10x_step() -> LlmStep {
        LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: "b10x".to_owned(),
            skills: Vec::new(),
            prompt: "do the thing".to_owned(),
        }
    }

    /// A step context with nothing outstanding, for a test that is about the surface.
    fn step_context<'a>(
        tools: &'a ToolConfig,
        state: &'a StateId,
        task: &'a aep_domain::task::Task,
    ) -> StepContext<'a> {
        StepContext {
            task,
            state,
            index: 0,
            attempt: 1,
            tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &[],
            reaching: &[],
            preceding_llm: None,
        }
    }

    /// The task a prompt test's run is driving.
    ///
    /// `derived_from` is populated because the identity line names the artifacts, and a fixture
    /// without one would let the line pass by saying nothing.
    fn driven_task() -> aep_domain::task::Task {
        aep_schema::parse::task(
            "id: T-1\nkind: feature\nobjective: drive something\nprotocol: aep/1\n\
             profile: test.standard\nderived_from: [story:the-one-being-driven]\n",
            None,
        )
        .expect("the fixture task parses")
    }

    /// The prompt the driver would build for one `llm` step.
    fn prompt_with_skills(skills: &[&str]) -> String {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: skills.iter().map(ToString::to_string).collect(),
            prompt: "do the thing".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "specify".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        prompt_for(&step, &context)
    }

    /// The session's `PATH` is metaharness's constructed one, and this pre-flight resolves on it.
    ///
    /// Pinned against that crate's `child_path` by construction rather than by dependency — the
    /// arrow in `adr/0002` runs one way and metaharness is not on it. If they drift, a driven run
    /// is refused when it would have worked, or worse, started when it cannot: both are cheaper to
    /// find here than at $1 a state.
    #[test]
    fn a_session_path_matches_what_metaharness_constructs() {
        let path = session_path();
        assert!(
            path.ends_with("/usr/local/bin:/usr/bin:/bin"),
            "metaharness's BASE_PATH is the tail of the constructed PATH: {path}"
        );
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                assert!(
                    path.starts_with(&format!("{home}/.local/bin:")),
                    "and `$HOME/.local/bin` is the head, which is the only directory an operator \
                     can install into without root: {path}"
                );
            }
        }
        assert!(
            !path.contains("target/debug"),
            "the session never sees this repository's build directory, which is the whole point: \
             {path}"
        );
        assert!(
            !path.contains(".cargo/bin"),
            "nor cargo's own install root — which is why the refusal says `--root ~/.local`: {path}"
        );
    }

    /// The prompt names the state's tools, from the same value the policy refuses on.
    ///
    /// **Run `W4-3/1`, 2026-08-29.** Every session spent a turn calling a tool it did not have —
    /// and the first attempt was a `ToolSearch` for `Grep` and `Glob`, which is a session trying to
    /// *load* what it had been told existed. Nothing told it. `decide_tool` prints the admitted set
    /// in its refusal, so the surface was knowable the whole time and only reachable by being
    /// refused: an allowlist a session learns by trial is an allowlist that costs a turn per state.
    ///
    /// Rendered from `context.tools` rather than from a second list, because two lists drift and
    /// the model would then trust the wrong one.
    #[test]
    fn the_prompt_names_the_tools_the_state_admits_and_the_policy_agrees() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "do the thing".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::RepositoryWrite]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };

        let prompt = prompt_for(&step, &context);
        let admitted = allowed_tools(&tools);
        assert!(!admitted.is_empty(), "the fixture admits something to name");
        for tool in &admitted {
            assert!(
                prompt.contains(tool.as_str()),
                "`{tool}` is admitted and the session is not told: {prompt}"
            );
        }
        assert!(
            prompt.contains("there are no others"),
            "the list is stated as closed, or it reads as a suggestion: {prompt}"
        );

        // The two must come from one source. A tool the prompt names and the policy refuses would
        // be worse than saying nothing — it would be an instruction to do what will be denied.
        for tool in &admitted {
            assert!(
                decide_tool(&context, tool, &serde_json::json!({})).is_ok()
                    || tool == "Bash"
                    || matches!(tool.as_str(), "Edit" | "Write" | "NotebookEdit"),
                "the prompt names `{tool}` and the policy refuses it"
            );
        }
    }

    /// A step that names `b10x` is told **that** harness's tool names, not Claude Code's.
    ///
    /// The rendering half of § 4.9 point 2, in the place it is most expensive to get wrong. The
    /// decision about which capabilities admit which operations is shared and is
    /// `aep_driver::tool::tool_config`'s; only the naming table is the harness's. A prompt that
    /// named `Read`, `Edit` and `Bash` to the b10x loop would be naming six tools that do not
    /// exist in its catalogue — which is exactly the class of waste this executor was added to
    /// remove, reintroduced by the driver itself.
    ///
    /// The b10x names are `b10x_harness_tools::entry_names`', read from the loop's own catalogue:
    /// `file_read`, `file_write`, `file_edit`, `dir_list`, `search`, `run`.
    #[test]
    fn a_b10x_step_is_told_the_b10x_catalogues_names_and_never_claude_codes() {
        let prompt = prompt_for(
            &b10x_step(),
            &step_context(
                &config(&[
                    Capability::RepositoryRead,
                    Capability::RepositoryWrite,
                    Capability::CommandExecution,
                ]),
                &"implement".parse().expect("a state id"),
                &driven_task(),
            ),
        );
        for named in [
            "file_read",
            "file_write",
            "file_edit",
            "dir_list",
            "search",
            "run",
        ] {
            assert!(
                prompt.contains(named),
                "`{named}` is in the b10x catalogue and this state admits it: {prompt}"
            );
        }
        for vendor in ["`Read`", "`Edit`", "`Write`", "`Glob`", "`Grep`", "`Bash`"] {
            assert!(
                !prompt.contains(vendor),
                "{vendor} is Claude Code's name and no b10x session has one: {prompt}"
            );
        }
    }

    /// A metacharacter inside quotes is an argument; outside them it composes.
    ///
    /// **Found by run `A3` within minutes of admitting the readers, and it was my own defect.**
    /// `grep -n "StolenLock\|took_lock_from" crates/` is one invocation whose `|` belongs to grep,
    /// and the bare-character scan refused it three times in one state. Admitting a tool and then
    /// refusing the natural way to use it is worse than not admitting it: the session is told two
    /// things and cannot tell which to believe.
    #[test]
    fn a_metacharacter_inside_quotes_is_an_argument_and_outside_them_it_composes() {
        for one_invocation in [
            r#"grep -n "StolenLock\|took_lock_from" crates/"#,
            r"grep -n 'a;b' file",
            r#"grep -E "fn (drive|resume)" src/run.rs"#,
            r"rg 'x > y' crates",
            r"grep -n '$(whoami)' file",
            r"grep -n '`date`' file",
            "protocol artifact list",
        ] {
            assert_eq!(
                composes(one_invocation),
                None,
                "`{one_invocation}` is one invocation: its metacharacters are quoted"
            );
        }

        for composed in [
            "protocol artifact list && protocol artifact graph",
            "protocol artifact list | head",
            "grep -rn x . > out.txt",
            "cat a; rm b",
            r#"echo "$(whoami)""#,
            r#"echo "`date`""#,
            r#"grep -n "a\|b" file | wc -l"#,
        ] {
            assert!(
                composes(composed).is_some(),
                "`{composed}` composes and must be refused"
            );
        }
    }

    /// A state that admits reading can read at scale, and still cannot write by any route.
    ///
    /// **The gap this closes.** `repository.read` renders `Glob` and `Grep`; Claude Code 2.1.247
    /// offers neither, and its own error tells the model to *search file contents with `grep` via
    /// the Bash tool instead* — which `driven_surface` refused. So a driven session was told to do
    /// the one thing the driver denied, and run `W4-3/1` spent 19 calls discovering that and never
    /// searched anything.
    ///
    /// The widening is safe only because composition and redirection are refused before this rule
    /// is reached, so this asserts both halves: the readers are admitted, and every route from a
    /// reader to a written byte is still closed.
    #[test]
    fn a_reading_state_may_read_at_scale_and_still_cannot_write_by_any_route() {
        let _step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "find it".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        let bash = |command: &str| {
            decide_tool(&context, "Bash", &serde_json::json!({ "command": command }))
        };

        for reading in [
            "grep -rn DriverOptions crates/",
            "rg --files crates/aep-driver",
            "ls .engineering/planning",
            "cat README.md",
            "head -40 crates/aep-driver/src/run.rs",
            "wc -l Cargo.toml",
        ] {
            assert!(
                bash(reading).is_ok(),
                "`{reading}` only reads and the state admits reading"
            );
        }

        // Every route from a reader to a byte on disk. The first four are refused by the
        // composition rule; the rest are programs that write with no help from a shell.
        for writing in [
            "grep -rn x crates/ > out.txt",
            "cat a.md >> b.md",
            "ls | tee out.txt",
            "cat a && rm b",
            "sed -i s/a/b/ Cargo.toml",
            "awk '{print > \"out\"}' a",
            "find . -delete",
            "xargs rm",
            "sh -c 'rm -rf x'",
            "env rm x",
        ] {
            assert!(
                bash(writing).is_err(),
                "`{writing}` reaches a write and must be refused"
            );
        }

        // And a state with no read capability gets none of them.
        let blind = config(&[Capability::CommandExecution]);
        let deaf = StepContext {
            tools: &blind,
            ..context
        };
        assert!(
            decide_tool(
                &deaf,
                "Bash",
                &serde_json::json!({ "command": "grep -r x ." })
            )
            .is_err(),
            "a state that does not admit reading does not get a reader"
        );
    }

    /// The prompt states every rule the shell will refuse on, and the policy agrees with it.
    ///
    /// **Measured on run `W4-3/1`, 2026-08-29: 28 of 174 tool calls — 16% of everything the run did
    /// — were refused, and every one was a rule the session could have been told.** Eleven were a
    /// program outside the surface, ten were a composed command, four were a tool the harness does
    /// not have, three were a tool the state does not admit. They recurred in every state from the
    /// first to the last, because being refused teaches one call and the next session starts fresh.
    ///
    /// This asserts the prompt names both shell rules, and — the part that matters — that the
    /// *examples it gives* are genuinely refused by `driven_surface`. A prompt that warned about a
    /// command the policy allows would train the session out of something it may do.
    #[test]
    fn the_prompt_states_the_shell_rules_the_policy_will_refuse_on() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "do the thing".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        let prompt = prompt_for(&step, &context);

        assert!(
            prompt.contains("one simple invocation per call"),
            "the composed-command rule is stated: {prompt}"
        );
        assert!(
            prompt.contains("protocol artifact") && prompt.contains("protocol trace"),
            "and the two verb families the surface admits: {prompt}"
        );

        // Everything the prompt tells the session not to do must actually be refused. Otherwise the
        // instruction is a superstition the run pays for in capability.
        let refused = |command: &str| {
            decide_tool(&context, "Bash", &serde_json::json!({ "command": command })).is_err()
        };
        // `ls` and `cat` were on this list until the readers were admitted, and this test is
        // where that change had to be argued: what is forbidden is what *writes* or what runs a
        // program the surface never admitted, not what reads.
        for forbidden in [
            "protocol artifact list && protocol artifact graph",
            "protocol artifact list | head",
            "git status",
            "cargo test --workspace",
            "sed -i s/a/b/ Cargo.toml",
            "protocol --help",
        ] {
            assert!(
                refused(forbidden),
                "the prompt warns against `{forbidden}` and the policy permits it"
            );
        }
        // And the one thing it tells the session it *may* do has to work.
        assert!(
            decide_tool(
                &context,
                "Bash",
                &serde_json::json!({ "command": "protocol artifact list" })
            )
            .is_ok(),
            "the prompt's own example is refused by the policy"
        );
    }

    /// The session is told which task the run drives, before it is told anything else.
    ///
    /// **Run `W4-3/1`, 2026-08-28, is why.** The map's `receive` prompt says *read the task under
    /// `.engineering/`* — a map is written once and driven many times, so it cannot say more. By
    /// then that directory held three task documents from three runs. The session read `task.yaml`,
    /// which is `W4-1`, described a different objective entirely, found the intake for it already
    /// in the store and reported that its work was done. It created nothing. The engine's cursor
    /// said `W4-3` throughout, and the next state went further wrong: 62 mentions of the wrong
    /// story against 10 of the right one.
    ///
    /// Nothing was violated — the guards held, the store was untouched, every transition was the
    /// engine's. The run was simply about something else than its own audit trail said, which is
    /// worse than a run that fails, because everything downstream is *about* something and nothing
    /// says what.
    ///
    /// The identity leads the prompt, so it is the subject of every sentence after it, and it names
    /// artifacts rather than a path — a path has to be read correctly, an id is what the store
    /// answers to.
    #[test]
    fn the_prompt_names_the_task_the_run_drives_before_the_maps_own_words() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "Read the task under `.engineering/` and record what is asked for.".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "receive".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };

        let prompt = prompt_for(&step, &context);
        let identity = prompt
            .split("Read the task under")
            .next()
            .expect("the map's own prompt follows the identity");
        assert!(
            identity.contains("`T-1`"),
            "the run's task is named before the step's own words: {prompt}"
        );
        assert!(
            identity.contains("story:the-one-being-driven"),
            "and so is what it is derived from, because that is what the store answers to: {prompt}"
        );
        assert!(
            identity.contains("belongs to another run"),
            "and the other task documents are ruled out by name, which is the whole defect: {prompt}"
        );
        assert!(
            prompt.starts_with("This run drives task"),
            "it leads, so it is the subject of every sentence after it: {prompt}"
        );
    }

    /// What the step is trying to reach reaches the step, under a heading of its own.
    ///
    /// Run `W4-1/1` spent $8.36 in `establish_verifiers` writing checks the guard out of that state
    /// then refused, because the prompt carried `Evaluation::requirements` — what must hold *while
    /// in* the state — and never `Evaluation::transitions[].requirements`, which is what the state
    /// is trying to reach. The two lines are asserted apart rather than together: a prompt that
    /// merged them would tell a step that its outgoing guard is already in force here, which is a
    /// different instruction.
    #[test]
    fn an_unmet_outgoing_guard_is_named_in_the_prompt_under_the_reaching_heading() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "write the checks".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "establish_verifiers".parse().expect("a state id");
        let requirements = vec!["✓ artifact story (any) [state establish_verifiers]".to_owned()];
        let reaching = vec![
            "-> implement: guard: test.exists".to_owned(),
            "-> implement: ✗ test.first_result == failed [principle test-driven]".to_owned(),
        ];
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/W4-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };

        let prompt = prompt_for(&step, &context);
        let (held, reached) = prompt
            .split_once("What this state is trying to reach")
            .expect("the reaching lines are under their own heading");
        assert!(
            held.contains("artifact story (any)"),
            "what must hold here stays under its own heading: {prompt}"
        );
        for line in &reaching {
            assert!(
                reached.contains(line.as_str()),
                "`{line}` is what the state is trying to reach and belongs in the prompt: {prompt}"
            );
            assert!(
                !held.contains(line.as_str()),
                "`{line}` guards the way out and must not read as a rule in force here: {prompt}"
            );
        }
    }

    /// A scratch directory under this crate's target directory, named for the test that asked.
    fn scratch(name: &str) -> PathBuf {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/drive-records")
            .join(name);
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("the scratch directory is writable");
        directory
    }

    /// A `trace_conformance` document of the shape `protocol trace evidence` writes.
    const TRACE_RECORD: &str = "\
- kind: trace_conformance
  specification: driven-eval/honest-step
  spec_digest: c2114acdc5782176f7149da41bf1baab6266305ce77d31f813da9de8f93e7aeb
  transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b
  status: passed
  expectations_total: 12
  expectations_gapped: 0
  expectations_unknown: 0
  observed_at: 1787355862391
  producer:
    producer: verifier
    verifier: trace-checker
";

    /// The record a verifier wrote is submitted as the verifier's, with nothing minted here.
    ///
    /// `trace_conformance` is not in `EvidenceMapping::MINTABLE` and must never be: its record
    /// carries a specification digest, a transcript digest and three counts, and an exit status
    /// carries none of them. So the check writes the document and the driver reads it — and the
    /// producer that arrives at the engine is the checker's, not this binary's, which is what makes
    /// the record admissible at all.
    #[test]
    fn a_record_a_verifier_wrote_is_submitted_as_that_verifiers_and_never_minted_here() {
        let directory = scratch("trace");
        let record = directory.join("trace-implement.yaml");
        std::fs::write(&record, TRACE_RECORD).expect("the record is writable");
        let mapping = EvidenceMapping {
            kind: EvidenceKind::TraceConformance,
            verifier: Verifier::TraceChecker,
            suite: None,
            subject: None,
            tool: None,
            record: Some("{run_directory}/trace-implement.yaml".to_owned()),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 1,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: Some(StepAttempt {
                index: 0,
                attempt: 1,
            }),
        };

        let outcome = read_record(
            mapping.record.as_deref().expect("a declared record"),
            &mapping,
            "protocol trace evidence",
            &context,
        );
        let StepOutcome::Observed(submission) = outcome else {
            panic!("a record that reads is a verdict: {outcome:?}");
        };
        assert_eq!(
            submission.evidence.kind(),
            EvidenceKind::TraceConformance,
            "what the document says it is, is what is submitted"
        );
        assert!(
            matches!(
                submission.producer,
                Producer::Verifier {
                    verifier: Verifier::TraceChecker
                }
            ),
            "the producer is the checker's own: {:?}",
            submission.producer
        );

        // `{transcript}` is a run-time fact, so a step that names one in a run where no `llm` step
        // has run is D5's `Unknown` rather than a verdict about a file that is not there.
        let empty: Vec<String> = Vec::new();
        let unrun = StepContext {
            task: &task,
            state: &state,
            index: 1,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };
        let outcome = expand("{transcript}", &unrun).expect_err("there is no transcript to name");
        assert!(outcome.contains("transcript"), "{outcome}");
    }

    /// Invariant 7 at the layer a `record:` path opens: a run cannot submit a person's approval.
    ///
    /// The path a step writes to is a path a step can also write *to*, and an approval read out of
    /// a file would unlock a capability gate with a document the run itself could have authored.
    /// The engine's capability check matches on the decision and not on who granted it, so the
    /// refusal has to be here.
    #[test]
    fn an_approval_read_out_of_a_file_is_refused_however_well_formed_it_is() {
        let directory = scratch("approval");
        let record = directory.join("approval.yaml");
        std::fs::write(
            &record,
            "- kind: approval\n  approval: release\n  decision: granted\n  \
             observed_at: 1787355862391\n  producer:\n    producer: human\n    id: a-person\n",
        )
        .expect("the record is writable");
        let mapping = EvidenceMapping {
            kind: EvidenceKind::Approval,
            verifier: Verifier::HumanApproval,
            suite: None,
            subject: None,
            tool: None,
            record: Some("{run_directory}/approval.yaml".to_owned()),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "review".parse().expect("a state id");
        let empty: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };

        let outcome = read_record(
            mapping.record.as_deref().expect("a declared record"),
            &mapping,
            "cat approval.yaml",
            &context,
        );
        let StepOutcome::NoVerdict { reason } = outcome else {
            panic!("an approval read out of a file is refused: {outcome:?}");
        };
        assert!(
            reason.contains("approval"),
            "the refusal says what it refused: {reason}"
        );
    }

    /// A record the verifier was to write and did not is D5's `Unknown`, and so is one that does
    /// not read.
    ///
    /// The case `story:evidence-producers-for-the-driven-map` made load-bearing. Three of the four
    /// kinds that map now produces arrive through `record:`, and the failure mode a producer has
    /// that a `cargo test` step does not is *the verb ran and wrote nothing usable*: a store the
    /// checker refused to choose from, a path a rename broke, a half-written file. None of those is
    /// a failing verdict — the run has observed nothing — and submitting a `failed` record for one
    /// would be the driver inventing an observation, which is invariant 7 a layer above the engine.
    ///
    /// Both roads are checked because they fail at different depths: a missing file never reaches
    /// the parser, and a malformed one fails inside it.
    #[test]
    fn a_record_that_is_missing_or_does_not_read_submits_nothing_and_says_why() {
        let directory = scratch("absent-record");
        let mapping = EvidenceMapping {
            kind: EvidenceKind::Specification,
            verifier: Verifier::ExternalTool("protocol".parse().expect("a tool reference")),
            suite: None,
            subject: None,
            tool: None,
            record: Some("{run_directory}/specification.yaml".to_owned()),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "adversarial_verify".parse().expect("a state id");
        let empty: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 3,
            attempt: 1,
            tools: &tools,
            run_directory: &directory,
            requirements: &empty,
            reaching: &empty,
            preceding_llm: None,
        };
        let read = || {
            read_record(
                mapping.record.as_deref().expect("a declared record"),
                &mapping,
                "protocol specification evidence",
                &context,
            )
        };

        // Nothing was written: the verb refused to choose between two specifications in force, or
        // the path in the map no longer names what the verb writes.
        let StepOutcome::NoVerdict { reason } = read() else {
            panic!("a record that is not there is not a verdict");
        };
        assert!(
            reason.contains("specification") && reason.contains("nothing was observed"),
            "the refusal names the kind that is owed and says nothing was observed, so a person \
             reading the run knows the step did not fail — it did not run: {reason}"
        );

        // Written, and not a document. Half a file is the shape a killed verb leaves behind.
        std::fs::write(
            directory.join("specification.yaml"),
            "- kind: specification\n  satisfied: ",
        )
        .expect("the record is writable");
        let StepOutcome::NoVerdict { reason } = read() else {
            panic!("a record that does not parse is not a verdict");
        };
        assert!(
            reason.contains("does not read"),
            "the refusal says the document is unreadable rather than reporting a failed \
             specification: {reason}"
        );
    }

    /// A step map's `skills:` list is a request to the model, not a command-line flag.
    ///
    /// The skill reaches the session by being asked for, and the `Skill` tool answers; nothing
    /// about the invocation carries it, which is what keeps a skill list from becoming a second
    /// tool surface.
    #[test]
    fn a_steps_skills_are_asked_for_in_the_prompt() {
        let prompt = prompt_with_skills(&["planning"]);
        assert!(
            prompt.contains("Load the `planning` skill"),
            "the step's skill has to be asked for somewhere: {prompt}"
        );
    }

    // ------------------------------------------------------------ the per-call policy

    /// The fixture task, borrowed for a context that outlives this call.
    ///
    /// Leaked rather than threaded through every caller: it is one small value per test binary,
    /// and the alternative is a lifetime parameter on two helpers that exist to shorten tests.
    fn task_ref() -> &'static aep_domain::task::Task {
        Box::leak(Box::new(driven_task()))
    }

    /// One context for the policy tests.
    fn policy_context<'a>(state: &'a StateId, tools: &'a ToolConfig) -> StepContext<'a> {
        StepContext {
            task: task_ref(),
            state,
            index: 0,
            attempt: 1,
            tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &[],
            reaching: &[],
            preceding_llm: None,
        }
    }

    /// The retired `driven-surface.sh`, case for case: the grant is held to one simple
    /// `protocol artifact|trace` invocation, and a state with no shell says so by name.
    #[test]
    fn the_shell_surface_is_one_simple_protocol_invocation() {
        let state: StateId = "implement".parse().expect("a state id");
        let shell = config(&[Capability::CommandExecution]);
        let context = policy_context(&state, &shell);
        let bash = |command: &str| {
            decide_tool(&context, "Bash", &serde_json::json!({ "command": command }))
        };

        assert!(bash("protocol artifact list").is_ok());
        assert!(bash("protocol trace check t.jsonl").is_ok());
        assert!(bash("/usr/local/bin/protocol artifact list").is_ok());

        assert!(
            bash("protocol artifact list | tee out").is_err(),
            "composition"
        );
        assert!(
            bash("protocol artifact list; rm -rf /").is_err(),
            "chaining"
        );
        assert!(bash("protocol artifact list > out").is_err(), "redirection");
        assert!(bash("protocol artifact $(cat x)").is_err(), "substitution");
        assert!(bash("cargo test").is_err(), "another program");
        assert!(bash("protocol drive run").is_err(), "another verb");
        assert!(bash("").is_err(), "an empty command");

        let no_shell = config(&[Capability::RepositoryRead]);
        let context = policy_context(&state, &no_shell);
        let refusal = decide_tool(
            &context,
            "Bash",
            &serde_json::json!({ "command": "protocol artifact list" }),
        )
        .expect_err("no shell in this state");
        assert!(
            refusal.contains("does not admit `command.execute`"),
            "{refusal}"
        );
    }

    /// Every native file writer is refused under the planning store; the CLI is its sole writer.
    #[test]
    fn the_planning_stores_frontmatter_is_the_clis() {
        let state: StateId = "implement".parse().expect("a state id");
        let writing = config(&[Capability::RepositoryWrite, Capability::RepositoryRead]);
        let context = policy_context(&state, &writing);
        let store_file = "/repo/.engineering/planning/story/one.md";

        let write = serde_json::json!({ "file_path": store_file, "content": "x" });
        assert!(decide_tool(&context, "Write", &write).is_err());
        let notebook = serde_json::json!({ "notebook_path": store_file });
        assert!(decide_tool(&context, "NotebookEdit", &notebook).is_err());

        let edit = |old: &str, new: &str| {
            decide_tool(
                &context,
                "Edit",
                &serde_json::json!({ "file_path": store_file, "old_string": old, "new_string": new }),
            )
        };
        let refusal = edit("a body sentence", "a better body sentence")
            .expect_err("even body edits cross the CLI surface");
        assert!(refusal.contains("artifact body"), "{refusal}");
        assert!(edit("---", "-- -").is_err());
        assert!(edit("a line", "status: done").is_err());

        let elsewhere = serde_json::json!({ "file_path": "/repo/src/lib.rs", "content": "x" });
        assert!(decide_tool(&context, "Write", &elsewhere).is_ok());
    }

    /// The allowlist that used to ride on `--allowedTools`, now a decision with a reason: a tool
    /// no admitted capability renders to is denied naming the state's actual surface.
    #[test]
    fn a_tool_outside_the_states_surface_is_denied_with_the_surface_named() {
        let state: StateId = "specify".parse().expect("a state id");
        let reading = config(&[Capability::RepositoryRead]);
        let context = policy_context(&state, &reading);

        assert!(decide_tool(&context, "Read", &serde_json::json!({})).is_ok());
        assert!(decide_tool(&context, "Skill", &serde_json::json!({})).is_ok());
        let refusal = decide_tool(&context, "Edit", &serde_json::json!({ "file_path": "/x" }))
            .expect_err("no write capability in this state");
        assert!(
            refusal.contains("not offered in state `specify`"),
            "{refusal}"
        );
        assert!(
            decide_tool(&context, "Task", &serde_json::json!({})).is_err(),
            "a subagent is never offered"
        );
    }

    // ------------------------------------------------------------ the metaharness executor

    /// One `StepContext` for the frame tests, with the engine's lines present.
    fn metaharness_context<'a>(
        state: &'a StateId,
        tools: &'a ToolConfig,
        requirements: &'a [String],
        reaching: &'a [String],
    ) -> StepContext<'a> {
        StepContext {
            task: task_ref(),
            state,
            index: 2,
            attempt: 3,
            tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements,
            reaching,
            preceding_llm: None,
        }
    }

    #[test]
    fn the_metaharness_operations_mirror_the_allowed_tools_decisions() {
        let reading = config(&[Capability::RepositoryRead]);
        assert_eq!(
            metaharness_operations(&reading),
            ["dir.list", "file.read", "search", "skill.load"]
        );
        assert!(!metaharness_operations(&reading).contains(&"shell"));

        let shell = config(&[Capability::CommandExecution]);
        assert!(metaharness_operations(&shell).contains(&"shell"));

        let everything = config(&[
            Capability::RepositoryRead,
            Capability::RepositoryWrite,
            Capability::CommandExecution,
            Capability::NetworkRead,
        ]);
        assert!(
            !metaharness_operations(&everything).contains(&"subagent.spawn"),
            "a subagent's tool set is derived by nothing in these decisions"
        );
    }

    /// Gap register `:40`. The document the driver writes has to be one `protocol trace check`
    /// can actually read, or it is a file nobody consumes that looks like an audit.
    ///
    /// Read back through `trace_domain::raw::read_spec` — the same door the CLI uses — rather than
    /// eyeballed as JSON.
    #[test]
    fn the_refusal_specification_is_a_specification_the_checker_reads() {
        let state: aep_domain::ids::StateId = "implement".parse().expect("a state id");
        let read_only = ToolConfig::new([Capability::RepositoryRead].into_iter().collect());
        let document =
            refusal_specification(&state, 0, &read_only).expect("a read-only state refuses things");
        let text = serde_json::to_string(&document).expect("renders");

        let spec = trace_domain::raw::read_spec(&text)
            .expect("the driver must write a specification the checker can read");

        let refused: Vec<&str> = spec
            .expectations
            .iter()
            .map(|expectation| expectation.id.as_str())
            .collect();
        assert!(
            refused.contains(&"refused-file-write"),
            "a read-only state must refuse writing: {refused:?}"
        );
        assert!(
            refused.contains(&"refused-shell"),
            "and a shell: {refused:?}"
        );
        assert!(
            !refused.iter().any(|id| id.ends_with("file-read")),
            "and must not refuse what it admitted: {refused:?}"
        );
        assert!(
            !refused.iter().any(|id| id.ends_with("skill-load")),
            "skills are always offered, so refusing them would be a row that can only fail: \
             {refused:?}"
        );
    }

    /// The complement is computed from the one table, so the two cannot drift.
    #[test]
    fn admitted_and_refused_operations_partition_the_vocabulary() {
        for config in [
            ToolConfig::default(),
            ToolConfig::new([Capability::RepositoryRead].into_iter().collect()),
            ToolConfig::new(TOOL_CANDIDATES.iter().cloned().collect()),
        ] {
            let admitted = metaharness_operations(&config);
            let refused = refused_operations(&config);
            let mut together: Vec<&str> = admitted.iter().chain(refused.iter()).copied().collect();
            together.sort_unstable();
            let mut all = every_operation();
            all.sort_unstable();
            assert_eq!(
                together, all,
                "every operation is either admitted or refused, and never both or neither"
            );
        }
    }

    /// A fully permissive state writes no specification, and that is `trace-spec/1`'s rule rather
    /// than a shortcut.
    ///
    /// The format refuses a specification with no expectations — *"a report with no content reads
    /// exactly like a report with no gaps"* — which is the same argument for not writing one. What
    /// keeps absence readable is the **frame**: it is written unconditionally, so a frame with no
    /// refusal file beside it means this state was admitted everything, and no frame at all means
    /// the step never ran.
    #[test]
    fn a_state_that_admits_everything_writes_no_specification() {
        let state: aep_domain::ids::StateId = "implement".parse().expect("a state id");
        let everything = ToolConfig::new(TOOL_CANDIDATES.iter().cloned().collect());
        assert!(refusal_specification(&state, 0, &everything).is_none());

        // And the empty document would indeed have been refused, so this is the format's rule and
        // not a preference.
        let empty = serde_json::json!({
            "format": "trace-spec/1",
            "id": "driver/implement-0",
            "expectations": [],
        });
        assert!(
            trace_domain::raw::read_spec(&empty.to_string()).is_err(),
            "an empty specification judges nothing and must not be writable"
        );
    }

    /// The seal is the metaharness § 5.5 rule, reproduced here without its crates: SHA-256 over
    /// the compact key-sorted serialization with `digest` and `format` absent. A document this
    /// test passes is a document metaharness's parser accepts byte-for-byte; one it fails is a
    /// run refused before a cent is spent.
    #[test]
    fn the_frame_document_is_sealed_by_the_rule_metaharness_verifies() {
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements = vec!["the suite is red before the implementation".to_owned()];
        let reaching = vec!["to verify: the suite is green".to_owned()];
        let context = metaharness_context(&state, &tools, &requirements, &reaching);

        let frame = metaharness_frame(&context, "development/default", "1");
        assert_eq!(frame["format"], METAHARNESS_FRAME_FORMAT);

        let mut unsealed = frame.clone();
        let object = unsealed.as_object_mut().expect("an object");
        let stated = object.remove("digest").expect("a digest");
        object.remove("format");
        let recomputed = {
            use sha2::{Digest as _, Sha256};
            let bytes = serde_json::to_vec(&unsealed).expect("serialises");
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            format!("{:x}", hasher.finalize())
        };
        assert_eq!(stated, serde_json::Value::String(recomputed));
    }

    /// The engine's lines travel verbatim, on the same rule as the prompt: the frame is the only
    /// place they exist for the seam, and a summary here would be the only summary.
    #[test]
    fn the_frame_carries_the_engines_lines_and_the_steps_coordinates() {
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "specify".parse().expect("a state id");
        let requirements = vec!["an approved specification exists".to_owned()];
        let reaching = vec!["to implement: the suite is red".to_owned()];
        let context = metaharness_context(&state, &tools, &requirements, &reaching);

        let frame = metaharness_frame(&context, "development/default", "1");
        assert_eq!(frame["node"]["id"], "specify");
        assert_eq!(frame["step"]["index"], 2);
        assert_eq!(frame["step"]["attempt"], 3);
        assert_eq!(frame["workflow"]["version"], "1");
        assert_eq!(
            frame["obligations"][0]["text"],
            "an approved specification exists"
        );
        assert_eq!(
            frame["reaching"][0]["text"],
            "to implement: the suite is red"
        );
        assert_eq!(frame["handoff"]["handoff"], "none");
        let operations: Vec<&str> = frame["operations"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|entry| entry["op"].as_str().expect("a name"))
            .collect();
        assert_eq!(
            operations,
            ["dir.list", "file.read", "search", "skill.load"]
        );
    }

    #[test]
    fn the_metaharness_argv_drives_the_seam_with_the_declared_directory_and_frame() {
        let argv = metaharness_argv(
            Path::new("/runs/T-1/1/transcripts/implement-2-3.frame.json"),
            Path::new("/operator/repo"),
            &[PathBuf::from("/plugins/claude-code")],
            "do the thing",
        );
        assert_eq!(argv[0], "metaharness");
        assert_eq!(argv[1], "run");
        assert_eq!(argv[2], "claude");
        let has = |flag: &str, value: &str| {
            argv.windows(2)
                .any(|pair| pair[0] == flag && pair[1] == value)
        };
        assert!(has("--cwd", "/operator/repo"));
        assert!(has(
            "--frame",
            "/runs/T-1/1/transcripts/implement-2-3.frame.json"
        ));
        assert!(has("--decisions", "ask"));
        assert!(has("-p", "do the thing"));
        assert!(has("--plugin-dir", "/plugins/claude-code"));
        assert!(argv.contains(&"--hermetic".to_owned()));
    }

    /// The b10x argv is the launch that loop refuses least, and it carries no frame.
    ///
    /// **Three of these assertions are about what is absent, and the absences are the design.**
    /// `metaharness run b10x --frame …` is refused before a model is reached — a frame's
    /// enforcement rides on `tool.decide`, which the b10x adapter refuses because nothing on that
    /// loop ever asks — and `--decisions observe` is refused for the same reason, so the launch
    /// leaves the default in place. `--substrate-embedded` is absent because substrate adopts a
    /// workspace only when its directory name starts with `ws_`, and a governed tree is the
    /// operator's repository: asking would turn every driven b10x step into a launch refusal.
    ///
    /// What is present instead is the surface travelling as the two spec fields that exist for a
    /// harness with no seam — the step's `scope:` as `--write-scope` in the order it was written,
    /// and its `context:` as `--context`.
    #[test]
    fn the_b10x_argv_carries_the_scope_and_never_the_frame_that_loop_would_refuse() {
        let mut step = b10x_step();
        step.scope = vec![
            ScopeRule {
                paths: vec![".engineering/planning/**".to_owned()],
                write: WriteScope::PartialOnly,
            },
            ScopeRule {
                paths: vec!["**".to_owned()],
                write: WriteScope::Denied,
            },
        ];
        step.context = vec!["AGENTS.md".to_owned()];
        let options = B10xOptions {
            endpoint: Some("http://127.0.0.1:8080".to_owned()),
            model: Some("a-model".to_owned()),
            api_key: false,
        };

        let argv = b10x_argv(
            &options,
            Path::new("/operator/repo"),
            &step.scope,
            &step.context,
            "do the thing",
        );
        assert_eq!(argv[0], "metaharness");
        assert_eq!(argv[1], "run");
        assert_eq!(argv[2], "b10x");
        let has = |flag: &str, value: &str| {
            argv.windows(2)
                .any(|pair| pair[0] == flag && pair[1] == value)
        };
        assert!(has("--cwd", "/operator/repo"));
        assert!(has("--model-endpoint", "http://127.0.0.1:8080"));
        assert!(has("--model", "a-model"));
        assert!(has("--credentials", "none"), "{argv:?}");
        assert!(has("-p", "do the thing"));
        assert!(argv.contains(&"--hermetic".to_owned()));

        // Ordered, first match wins, so the map's own order is the argv's order.
        let scopes: Vec<&String> = argv
            .windows(2)
            .filter(|pair| pair[0] == "--write-scope")
            .map(|pair| &pair[1])
            .collect();
        assert_eq!(
            scopes,
            [".engineering/planning/**=partial-only", "**=denied"]
        );
        assert!(has("--context", "AGENTS.md"));

        for refused in [
            "--frame",
            "--decisions",
            "--substrate-embedded",
            "--plugin-dir",
        ] {
            assert!(
                !argv.iter().any(|word| word == refused),
                "`{refused}` is either refused by the b10x adapter or means nothing to it: {argv:?}"
            );
        }

        // The credential is a choice and not a silence: `operator-login` is the flag's default and
        // names nothing on this loop, which refuses it rather than launching unauthenticated.
        let authenticated = b10x_argv(
            &B10xOptions {
                api_key: true,
                ..options
            },
            Path::new("/operator/repo"),
            &[],
            &[],
            "do the thing",
        );
        assert!(authenticated
            .windows(2)
            .any(|pair| pair[0] == "--credentials" && pair[1] == "api-key"));
    }

    /// One capability decision, two naming tables, and no second decision anywhere.
    ///
    /// § 4.9 point 2's load-bearing assertion, driven from `aep_driver::tool::tool_config` rather
    /// than from a hand-built [`ToolConfig`] so the shared half is genuinely the shared function.
    /// A second harness that re-decided could quietly re-admit a shell the state never granted,
    /// which is what makes this the guard rather than the name comparison.
    #[test]
    fn the_shared_tool_decision_renders_into_two_vocabularies_and_is_taken_once() {
        use aep_domain::capability::CapabilityPolicy;
        use aep_driver::tool::tool_config;

        let everything = tool_config(&CapabilityPolicy::allowing(TOOL_CANDIDATES.iter().cloned()));
        assert!(
            !everything.is_empty(),
            "the widest policy admits something, or every assertion below is vacuous"
        );
        let b10x = b10x_tools(&everything);
        let claude = allowed_tools(&everything);
        assert!(!b10x.is_empty() && !claude.is_empty());
        for named in &b10x {
            assert!(
                !claude.contains(named),
                "`{named}` is in both tables, so one of them is not a rendering of its own harness"
            );
        }
        // The three entries § 4.9 point 2 decides rather than leaves to an implementer.
        assert!(
            !everything.subagents_offered() && !b10x.iter().any(|named| named.contains("agent")),
            "no subagent spawner is ever rendered, whatever is admitted"
        );
        for (capabilities, admitted) in [
            (vec![Capability::RepositoryRead], false),
            (
                vec![Capability::RepositoryRead, Capability::CommandExecution],
                true,
            ),
        ] {
            let config = tool_config(&CapabilityPolicy::allowing(capabilities.clone()));
            assert_eq!(
                b10x_tools(&config).contains(&"run".to_owned()),
                admitted,
                "the exec entry is offered iff `command.execute` is admitted, and \
                 {capabilities:?} admits it: {admitted}"
            );
        }
        // `web.read` and `skill.load` have no entry in that loop's catalogue. Not downgraded to
        // something else and not silently dropped from the shared decision: the capability stays
        // admitted and the session simply has no tool, which the session-start audit reports.
        let networked = tool_config(&CapabilityPolicy::allowing([
            Capability::RepositoryRead,
            Capability::NetworkRead,
        ]));
        assert!(
            networked.admits(&Capability::NetworkRead),
            "the shared decision still admits it"
        );
        assert_eq!(
            b10x_tools(&networked),
            ["dir_list", "file_read", "search"],
            "and this table renders nothing for it, because the catalogue has no entry"
        );
    }

    /// The `--write-scope` words are the words a step map is written in.
    ///
    /// Two spellings of one rule is one spelling that drifts, and the drift here is silent: a rule
    /// rendered as an unknown word is a rule metaharness refuses at launch, or worse, one it reads
    /// as a different rule.
    #[test]
    fn the_write_scope_words_are_the_ones_the_step_map_is_written_in() {
        for scope in [
            WriteScope::Allowed,
            WriteScope::PartialOnly,
            WriteScope::Denied,
        ] {
            let written = serde_json::to_value(scope).expect("a scope serialises");
            assert_eq!(
                written.as_str().expect("a string"),
                write_scope_word(scope),
                "the argv word and the document word are one word"
            );
        }
    }

    /// An observed session's calls are counted and never reported as an adjudication.
    ///
    /// **The failure this exists to make impossible.** The b10x adapter sets
    /// `decision_required: false` on every `tool.requested` and `Seam::None` beside it, so a
    /// driver that folded that stream into the claude arm's report would print *0 refused* — and
    /// two arms compared on that number would be compared on an artefact of the instrument rather
    /// than on what the runs did. What is refused on this arm is refused by the toolset, before a
    /// call exists to be counted.
    ///
    /// Both directions are asserted, because a report that said *nobody asked* on every run would
    /// be exactly as wrong in the other direction.
    #[test]
    fn an_observed_session_is_counted_and_never_reported_as_a_clean_adjudication() {
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "specify".parse().expect("a state id");
        let task = driven_task();
        let context = step_context(&tools, &state, &task);
        let observed = format!(
            "{}\n{}\n",
            serde_json::json!({
                "format": METAHARNESS_EVENT_FORMAT,
                "event": "tool.requested",
                "decision_required": false,
                "seam": "none",
                "call_id": "call-1",
                "name": "file_read",
                "input": { "path": "AGENTS.md" },
            }),
            serde_json::json!({
                "format": METAHARNESS_EVENT_FORMAT,
                "event": "tool.requested",
                "decision_required": false,
                "seam": "none",
                "call_id": "call-2",
                "name": "search",
                "input": { "path": "." },
            })
        );

        let mut commands: Vec<u8> = Vec::new();
        let mut transcript: Vec<u8> = Vec::new();
        let mut authorize = |_: &ActionRequest| -> Decision {
            panic!("an observed stream asks the engine nothing, because nobody asked the driver")
        };
        let tally = answer_events(
            Harness::B10x,
            &context,
            observed.as_bytes(),
            &mut commands,
            &mut transcript,
            &mut authorize,
        );

        assert_eq!(
            tally,
            Adjudication {
                requested: 2,
                asked: 0,
                denied: 0
            }
        );
        assert!(
            commands.is_empty(),
            "answering a call nobody put would be this process claiming a decision the wire says \
             nobody made: {}",
            String::from_utf8_lossy(&commands)
        );
        assert_eq!(
            String::from_utf8_lossy(&transcript),
            observed,
            "every event line reaches the transcript, decided or not"
        );

        let line = tally.line(Harness::B10x, &state);
        assert!(
            line.contains("observed 2 tool call(s) and adjudicated none"),
            "the count of what happened is reported: {line}"
        );
        assert!(
            line.contains("nobody asked this process"),
            "and it is distinguished from nothing having been refused: {line}"
        );
        assert!(
            !line.contains("were refused"),
            "a denial count here would read as a verdict about the run: {line}"
        );

        // The other direction: an arm that does adjudicate reports the counts, because there the
        // zero genuinely means nothing was refused.
        let adjudicated = Adjudication {
            requested: 5,
            asked: 5,
            denied: 1,
        }
        .line(Harness::ClaudeCode, &state);
        assert!(
            adjudicated.contains("put 5 tool call(s) to the driver and 1 were refused"),
            "{adjudicated}"
        );
    }

    /// A map naming a harness this machine cannot spawn is refused before a run id or a lock.
    ///
    /// The same shape as the two pre-flights beside it, and the same argument: this is decidable
    /// from the map and the filesystem, and discovering it at the first `llm` step means a run
    /// directory, an id, the store lock and a snapshot for a `NoVerdict` about something that was
    /// never run.
    ///
    /// Asserted on the two checks that need no process — a map with no b10x step is silent, and a
    /// map with one that declares no endpoint is refused naming the flag. The two spawning checks
    /// above them are not exercised here: a unit test that shelled out to whatever
    /// `{METAHARNESS_BINARY}` this machine happens to hold would report on the machine.
    #[test]
    fn a_map_naming_the_b10x_harness_is_refused_when_the_run_cannot_say_where_to_point_it() {
        let map = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/b10x\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do it\n\
             \x20       harness: b10x\n",
            None,
        )
        .expect("the map validates");
        assert_eq!(b10x_step_count(&map), 1, "the fixture reaches the rule");

        let claude = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/llm\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do it\n",
            None,
        )
        .expect("the map validates");
        assert!(
            b10x_preflight(&claude, &B10xOptions::default()).is_none(),
            "a map with no b10x step is not this pre-flight's business, whatever is installed"
        );

        // The two checks that read only the arguments, and they answer the same on every machine
        // — which is the reason they run first.
        let refusal = b10x_preflight(&map, &B10xOptions::default()).expect("refused");
        assert!(
            refusal.contains("--b10x-endpoint"),
            "the refusal names the flag that answers it: {refusal}"
        );
        let refusal = b10x_preflight(
            &map,
            &B10xOptions {
                endpoint: Some("http://127.0.0.1:8080".to_owned()),
                ..B10xOptions::default()
            },
        )
        .expect("refused");
        assert!(refusal.contains("--b10x-model"), "{refusal}");

        // With both declared, what is left is about this machine, so the assertion is about which
        // answer it gave rather than about which one it should have.
        let declared = B10xOptions {
            endpoint: Some("http://127.0.0.1:8080".to_owned()),
            model: Some("a-model".to_owned()),
            api_key: false,
        };
        let installed = session_path()
            .split(':')
            .any(|directory| Path::new(directory).join(B10X_BINARY).is_file());
        match b10x_preflight(&map, &declared) {
            None => assert!(
                installed && metaharness_knows(B10X_HARNESS),
                "the only reason to admit a b10x map is that both halves are installed"
            ),
            Some(refusal) => assert!(
                refusal.contains(B10X_BINARY) || refusal.contains("does not publish an adapter"),
                "an install predating the adapter is named as that rather than as a missing \
                 binary: {refusal}"
            ),
        }
    }

    // -------------------------------------------------- the golden the other repository replays

    /// The name of the committed cross-repository golden, under this crate's `fixtures/`.
    const GOLDEN: &str = "metaharness-frame-canonical.json";

    /// The one frame the golden is minted from, and the reason it can be committed at all.
    ///
    /// Nothing here reads a clock, an environment variable or anything off this machine, so the
    /// document is byte-identical wherever it is minted — a golden that varied with its producer
    /// would pin the producer and not the format. The `run_directory` a [`StepContext`] carries
    /// never reaches the frame, which is why the fixture holds no path at all; the workflow id and
    /// the state are document names this repository publishes, and the two lines are the engine's
    /// own vocabulary. There is deliberately nothing account-level in it: this file is public and
    /// is read by a repository that is not.
    ///
    /// The capability set is the widest a driven state gets, so the golden carries seven of the ten
    /// parameterless operations rather than a corner of the vocabulary.
    fn canonical_frame() -> serde_json::Value {
        let tools = config(&[
            Capability::RepositoryRead,
            Capability::RepositoryWrite,
            Capability::CommandExecution,
        ]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements = vec!["the suite is red before the implementation".to_owned()];
        let reaching = vec!["to verify: the suite is green".to_owned()];
        let task = driven_task();
        let context = StepContext {
            task: &task,
            state: &state,
            index: 2,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("."),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        metaharness_frame(&context, "development/default", "1")
    }

    /// Compares `produced` against the committed golden, or writes it when there is none.
    ///
    /// Written-when-absent and then failing, on `aep-render`'s rule: a regeneration is a reviewable
    /// diff and never a silent overwrite. There is deliberately **no** environment variable that
    /// accepts whatever the minter now produces — this file is another repository's input, and a
    /// golden that rewrites itself pins nothing on either side of the seam.
    fn golden(produced: &str) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(GOLDEN);
        let Ok(committed) = fs::read_to_string(&path) else {
            fs::create_dir_all(path.parent().expect("the golden has a directory"))
                .expect("the fixture directory is writable");
            fs::write(&path, produced).expect("the golden is writable");
            panic!(
                "no golden at {}; it has been written — review it and run again",
                path.display()
            );
        };
        if committed == produced {
            return;
        }
        let differs = committed
            .lines()
            .zip(produced.lines())
            .enumerate()
            .find(|(_, (want, got))| want != got)
            .map_or_else(
                || {
                    (
                        committed.lines().count().min(produced.lines().count()) + 1,
                        "<end of file>".to_owned(),
                        "<more lines>".to_owned(),
                    )
                },
                |(index, (want, got))| (index + 1, want.to_owned(), got.to_owned()),
            );
        let (line, want, got) = differs;
        panic!(
            "{} differs at line {line}\n  committed: {want}\n  produced:  {got}\n\
             delete the file and re-run to accept the new document — and say so in the story, \
             because metaharness replays these bytes",
            path.display()
        );
    }

    /// The golden is the bytes the driver writes, not a hand-typed copy of them.
    ///
    /// It is minted through [`metaharness_frame`] and rendered through [`frame_document`], which is
    /// the path `write_frame_document` takes; only the `fs::write` is missing. A fixture assembled
    /// any other way would drift from the driver in silence, and the contract test that reads it
    /// (`tests/metaharness_frame_contract.rs`) would then be certifying a document nothing sends.
    #[test]
    fn the_committed_golden_is_the_document_the_driver_would_write() {
        let document = frame_document(&canonical_frame()).expect("the frame renders");
        golden(&document);
    }

    /// Two mints of the same step agree byte for byte, or the golden could not be committed and the
    /// digest could not be cited across the process boundary it is only ever cited across.
    #[test]
    fn two_mints_of_the_same_step_are_the_same_document() {
        assert_eq!(
            frame_document(&canonical_frame()).expect("the frame renders"),
            frame_document(&canonical_frame()).expect("the frame renders")
        );
    }

    // ------------------------------------------------------------ the engine at decision time

    /// A protocol declaring more than the profile below grants, so a capability can be *known* and
    /// still not be granted — which is the state a `NotGranted` decision needs.
    const AUTHORIZE_PROTOCOL: &str = r"
id: aep
version: 1
title: Test protocol
capabilities: [repository.read, repository.write, command.execute, tests.execute]
evidence_kinds: [test_result, diff, approval]
verifiers: [test-runner, compiler, human-approval]
artifact_kinds: [story]
phases: [implementation]
observables:
  - 'task.**'
  - 'tests.**'
  - 'diff.**'
  - 'artifact.**'
  - 'evidence.**'
  - 'state.**'
  - 'workflow.**'
  - 'approvals.**'
";

    const AUTHORIZE_WORKFLOW: &str = r"
id: test/linear
version: 1
title: Linear
initial: implement
states:
  implement:
    title: Implement
    phases: [implementation]
  complete:
    title: Complete
    terminal: true
    phases: [implementation]
transitions:
  - from: implement
    to: complete
    when: diff.exists
";

    /// The profile that makes the fixture load-bearing: it grants `repository.read` and **not**
    /// `repository.write`, so a state whose rendered surface offers `Edit` is a state where the two
    /// layers disagree and the engine is the one that refuses.
    const AUTHORIZE_PROFILE: &str = r"
id: test.reading
title: Reading only
protocol: aep/1
workflow: test/linear
capabilities:
  allow: [repository.read]
completion:
  - diff.exists
";

    const AUTHORIZE_TASK: &str = r"
id: T-1
kind: feature
objective: drive something
protocol: aep/1
profile: test.reading
";

    /// An engine over those documents, and an execution of that task in `implement`.
    fn authorizing_execution() -> (Engine, aep_engine::execution::Execution) {
        use aep_engine::ProtocolEngine as _;
        let mut registry = Registry::new();
        registry
            .insert_protocol(
                aep_schema::parse::protocol(AUTHORIZE_PROTOCOL, None).expect("the protocol parses"),
            )
            .expect("the protocol is unique");
        registry
            .insert_workflow(
                aep_schema::parse::workflow(AUTHORIZE_WORKFLOW, None).expect("the workflow parses"),
            )
            .expect("the workflow is unique");
        registry
            .insert_profile(
                aep_schema::parse::profile(AUTHORIZE_PROFILE, None).expect("the profile parses"),
            )
            .expect("the profile is unique");
        let engine = Engine::new(registry);
        let execution = engine
            .initialize(aep_schema::parse::task(AUTHORIZE_TASK, None).expect("the task parses"))
            .expect("the task resolves");
        (engine, execution)
    }

    /// One `tool.requested` event line of the shape metaharness writes in ask mode.
    fn requested(tool: &str, input: &serde_json::Value) -> String {
        format!(
            "{}\n",
            serde_json::json!({
                "format": "metaharness.event/1",
                "event": "tool.requested",
                "decision_required": true,
                "call_id": "call-1",
                "name": tool,
                "input": input,
            })
        )
    }

    /// Runs one scripted call through the whole seam and returns the decision written back down
    /// stdin — the same object metaharness reads, never a summary of it.
    fn decide_through_the_seam(
        context: &StepContext<'_>,
        engine: &Engine,
        execution: &mut aep_engine::execution::Execution,
        tool: &str,
        input: &serde_json::Value,
    ) -> serde_json::Value {
        use aep_engine::ProtocolEngine as _;
        let mut commands: Vec<u8> = Vec::new();
        let mut transcript: Vec<u8> = Vec::new();
        {
            let mut authorize =
                |request: &ActionRequest| engine.authorize(&mut *execution, request);
            answer_events(
                Harness::ClaudeCode,
                context,
                requested(tool, input).as_bytes(),
                &mut commands,
                &mut transcript,
                &mut authorize,
            );
        }
        assert!(
            !transcript.is_empty(),
            "every event line reaches the transcript, decided or not"
        );
        serde_json::from_slice(&commands).expect("one `tool.decide` command line")
    }

    /// Every event the execution recorded, by name.
    fn event_names(execution: &aep_engine::execution::Execution) -> Vec<String> {
        execution
            .events()
            .iter()
            .map(|envelope| envelope.event.name().to_owned())
            .collect()
    }

    /// The gap the guide called *"a decision is in the run's record, not yet in the engine's"*,
    /// closed: the engine refuses the call **and** the refusal is in the execution's own events.
    ///
    /// The fixture reaches the state where the rule is load-bearing before asserting the outcome —
    /// the policy layer is asserted to *allow* this call first, because a test where both layers
    /// refuse would pass whether or not the engine was ever asked.
    #[test]
    fn a_call_the_engine_refuses_is_denied_and_the_refusal_is_in_the_executions_event_record() {
        let (engine, mut execution) = authorizing_execution();
        let state: StateId = "implement".parse().expect("a state id");
        let writing = config(&[Capability::RepositoryRead, Capability::RepositoryWrite]);
        let context = policy_context(&state, &writing);
        let input = serde_json::json!({
            "file_path": "/repo/src/lib.rs",
            "old_string": "a",
            "new_string": "b",
        });
        assert!(
            decide_tool(&context, "Edit", &input).is_ok(),
            "the policy layer admits this call, so the engine is the layer under test"
        );

        let command = decide_through_the_seam(&context, &engine, &mut execution, "Edit", &input);

        assert_eq!(command["command"], "tool.decide");
        assert_eq!(command["call_id"], "call-1");
        assert_eq!(command["decision"]["decision"], "deny");
        let reason = command["decision"]["reason"]
            .as_str()
            .expect("a denial says why");
        assert!(
            reason.contains("the engine refuses this call"),
            "the reason names the layer that refused: {reason}"
        );
        assert!(
            reason.contains("repository.write") && reason.contains("not_granted"),
            "and carries the engine's own words: {reason}"
        );

        let denied = execution
            .events()
            .iter()
            .find(|envelope| envelope.event.name() == "action_denied")
            .expect(
                "the refusal is in the execution's event record, which is what authorize is for",
            );
        let json = serde_json::to_value(&denied.event).expect("the event serialises");
        assert_eq!(json["capability"], "repository.write");
        assert_eq!(json["decision"], "not_granted");
        assert!(
            event_names(&execution).contains(&"action_requested".to_owned()),
            "the request is recorded beside the refusal: {:?}",
            event_names(&execution)
        );
    }

    /// Policy first, and a call it refuses never reaches the engine.
    ///
    /// The order matters in both directions: the argument-level rules are the only layer that can
    /// tell `protocol artifact list` from `cargo test`, and an engine asked about a call the driver
    /// already refused would record an action nobody was allowed to attempt.
    #[test]
    fn a_call_the_policy_refuses_is_attributed_to_the_policy_and_never_reaches_the_engine() {
        let (engine, mut execution) = authorizing_execution();
        let state: StateId = "implement".parse().expect("a state id");
        let shell = config(&[Capability::CommandExecution]);
        let context = policy_context(&state, &shell);
        let input = serde_json::json!({ "command": "cargo test" });

        let command = decide_through_the_seam(&context, &engine, &mut execution, "Bash", &input);

        assert_eq!(command["decision"]["decision"], "deny");
        let reason = command["decision"]["reason"]
            .as_str()
            .expect("a denial says why");
        assert!(
            reason.contains("the driver's per-call policy refuses"),
            "the reason names the layer that refused: {reason}"
        );
        assert!(
            !event_names(&execution).contains(&"action_requested".to_owned()),
            "a call the policy refused is not an action the engine was asked about: {:?}",
            event_names(&execution)
        );
    }

    /// The `None` arm of the table, exercised: a `Skill` load is admitted by the policy and the
    /// engine is not consulted, because no `ActionRequest` describes loading instructions.
    #[test]
    fn a_skill_load_is_admitted_without_the_engine_being_asked_to_invent_an_action() {
        let (engine, mut execution) = authorizing_execution();
        let state: StateId = "implement".parse().expect("a state id");
        let reading = config(&[Capability::RepositoryRead]);
        let context = policy_context(&state, &reading);
        let input = serde_json::json!({ "skill": "planning" });

        let command = decide_through_the_seam(&context, &engine, &mut execution, "Skill", &input);

        assert_eq!(command["decision"]["decision"], "allow");
        assert!(
            !event_names(&execution).contains(&"action_requested".to_owned()),
            "loading instructions is not an action, and the record must not claim one: {:?}",
            event_names(&execution)
        );
    }

    /// The table itself: which tool is which action, and what each therefore needs.
    ///
    /// Asserted as capabilities rather than as variants, because the capability is the only thing
    /// the engine decides on — and asserted on the *payload* too, so a request that reached the
    /// record naming the wrong file would fail here rather than mislead an audit.
    #[test]
    fn each_offered_tool_renders_as_the_action_it_is_and_two_render_as_none() {
        let needs = |tool: &str, input: serde_json::Value| {
            action_for(tool, &input).map(|request| request.required_capability().to_string())
        };
        let read = serde_json::json!({ "file_path": "/repo/src/lib.rs" });
        assert_eq!(needs("Read", read.clone()), Some("repository.read".into()));
        assert_eq!(
            needs("Grep", serde_json::json!({ "pattern": "fn main" })),
            Some("repository.read".into()),
            "a search with no path is a search of the working directory"
        );
        assert_eq!(needs("Edit", read.clone()), Some("repository.write".into()));
        assert_eq!(needs("Write", read), Some("repository.write".into()));
        assert_eq!(
            needs(
                "NotebookEdit",
                serde_json::json!({ "notebook_path": "/n.ipynb" })
            ),
            Some("repository.write".into())
        );
        assert_eq!(
            needs(
                "Bash",
                serde_json::json!({ "command": "protocol artifact list" })
            ),
            Some("command.execute".into())
        );
        assert_eq!(
            needs(
                "WebFetch",
                serde_json::json!({ "url": "https://example.test/" })
            ),
            Some("network.read".into())
        );

        assert!(
            action_for("Skill", &serde_json::json!({ "skill": "planning" })).is_none(),
            "loading instructions takes no action"
        );
        assert!(
            action_for("WebSearch", &serde_json::json!({ "query": "aep" })).is_none(),
            "a search names no URL, and a request stating one nobody asked for is a fiction"
        );

        let request = action_for(
            "Bash",
            &serde_json::json!({ "command": "protocol artifact list --kind story" }),
        )
        .expect("a shell call renders");
        assert_eq!(
            request.action.summary(),
            "run `protocol artifact list --kind story`",
            "what the engine records is the call that was made"
        );
        let request = action_for("Read", &serde_json::json!({ "file_path": "/repo/x.rs" }))
            .expect("a read renders");
        assert_eq!(request.action.summary(), "read /repo/x.rs");
    }

    /// The launch-time refusal, and the one case it must not fire in.
    ///
    /// A map of `command` steps drives on a machine with no metaharness and no vendor, so the check
    /// is scoped to maps that would spawn one. `PATH` is not manipulated here — the assertion is
    /// about what is checked, and the refusal's text is what an operator has to act on.
    #[test]
    fn a_map_with_an_llm_step_is_refused_at_launch_when_the_seams_binary_is_missing() {
        let commands_only = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/commands\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: command\n        run: [\"true\"]\n",
            None,
        )
        .expect("the map validates");
        assert!(
            metaharness_preflight(&commands_only).is_none(),
            "a map that spawns no session needs no seam binary, whatever is on PATH"
        );

        let with_llm = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/llm\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do the thing\n",
            None,
        )
        .expect("the map validates");
        match metaharness_preflight(&with_llm) {
            None => assert!(
                on_path(METAHARNESS_BINARY),
                "the only reason to allow an `llm` map is that the binary is installed"
            ),
            Some(refusal) => {
                assert!(
                    refusal.contains("cargo install --path crates/metaharness-cli"),
                    "a refusal answers the question it creates: {refusal}"
                );
                assert!(
                    refusal.contains("not on PATH"),
                    "and says what it found: {refusal}"
                );
            }
        }
    }

    #[test]
    fn the_rendering_offers_a_shell_only_when_the_capability_is_admitted() {
        let reading = config(&[Capability::RepositoryRead]);
        assert_eq!(allowed_tools(&reading), ["Glob", "Grep", "Read", "Skill"]);
        assert!(!allowed_tools(&reading).contains(&"Bash".to_owned()));

        let shell = config(&[Capability::CommandExecution]);
        assert!(allowed_tools(&shell).contains(&"Bash".to_owned()));
    }

    #[test]
    fn a_subagent_spawner_is_never_rendered_whatever_is_admitted() {
        let everything = config(&[
            Capability::RepositoryRead,
            Capability::RepositoryWrite,
            Capability::CommandExecution,
            Capability::NetworkRead,
            Capability::Deploy(Environment::Production),
        ]);
        assert!(
            !allowed_tools(&everything).contains(&"Task".to_owned()),
            "a subagent's tool set is derived by nothing in D1-D6, so it is a route around the \
             per-state allowlist"
        );
    }

    #[test]
    fn a_failing_command_mints_a_record_that_says_so_and_a_failed_diff_mints_nothing() {
        let mapping = EvidenceMapping {
            kind: EvidenceKind::TestResult,
            verifier: Verifier::TestRunner,
            suite: Some(TestSuite::Unit),
            subject: None,
            tool: None,
            record: None,
        };
        let failed = mint(&mapping, false, "cargo test", observed_now()).expect("a verdict");
        match &failed.evidence {
            Evidence::TestResult(result) => assert_eq!(result.failed, 1),
            other => panic!("expected a test result, got {other:?}"),
        }
        assert_eq!(
            failed.producer,
            Producer::Verifier {
                verifier: Verifier::TestRunner
            }
        );

        let diff = EvidenceMapping {
            kind: EvidenceKind::Diff,
            verifier: Verifier::parse("git").expect("a verifier"),
            suite: None,
            subject: None,
            tool: None,
            record: None,
        };
        assert!(
            mint(&diff, false, "git diff", observed_now()).is_none(),
            "a ChangeSet has no form that says no change happened, so the honest answer is to \
             submit nothing"
        );
    }
}
