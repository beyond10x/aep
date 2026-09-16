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

include!("drive_tests.rs");
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode, Stdio};

use aep_domain::capability::Capability;
use aep_domain::entity::ActorRef;
use aep_domain::evidence::{
    ChangeSet, ContractResult, Evidence, EvidenceKind, Producer, Provenance, StaticAnalysisResult,
    TestResult, TestSuite,
};
use aep_domain::ids::{ExecutionId, StateId, TaskId, ToolRef};
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_driver::coverage::CoverageReport;
use aep_driver::executor::{
    CommandStepExecutor, LlmStepExecutor, OperatorStepExecutor, StepAuthorizer, StepContext,
    StepOutcome,
};
use aep_driver::lock::{Liveness, LockState};
use aep_driver::run::{DriveError, DriverOptions, InFlightResolution, RunDirectory, RunReport};
use aep_driver_spec::cursor::{DriverCursor, RunId, RunStatus, StolenLock};
use aep_driver_spec::map::{
    placeholders_in, CommandStep, EvidenceMapping, LlmStep, OperatorStep, ScopeRule, Step, StepMap,
    WriteScope,
};
use aep_engine::engine::EvidenceSubmission;
use aep_engine::policy::Decision;
use aep_engine::{Engine, ProtocolEngine, Registry, TransitionResult};
use aep_project::project::project_directory;
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
// The one glob matcher in the workspace, and the one a step map's `scope:` is decided with. Taken
// from `trace-domain` rather than written again here for the reason `AGENTS.md` gives about a
// second copy of a rule: two matchers would disagree about `*` the first time either was touched,
// and this one is already property-tested against the paths the design writes
// (`crates/observe/trace-domain/src/matcher.rs`).
use trace_domain::matcher::glob_matches;

/// The directory inside `.engineering` that holds runs.
pub const RUNS_DIRECTORY: &str = "runs";


/// The one lock file per project.
///
/// **Per project, not per store**, and the two are not the same when `--store` is given: the path
/// comes from `runs_directory(&inputs.project)`, so two projects aimed at one store by an explicit
/// `--store` hold two different locks and both runs proceed. `--store` defaults to
/// `<project>/.engineering/planning`, so reaching that needs a deliberate override; the scope is
/// settled by `decision-blocker:store-lock-scope` and asserted by
/// `a_second_project_aimed_at_one_store_holds_its_own_lock_and_is_not_refused`.
///
/// **One fixed path, taken before any run id is allocated.** The reviewed first draft put the lock
/// inside `.engineering/runs/<run-id>/`, which is circular: two invocations counting the existing
/// directories at slightly different moments get `3` and `4`, and **both `create_new` succeed**,
/// because they are different paths — two live runs over one store, which is the option D6
/// explicitly rejected, reached by accident. Review finding **F2**.
pub const LOCK_FILE: &str = "lock.json";


/// The store-level pointer to the run that last held the lock.
pub const CURRENT_FILE: &str = "current";


/// The transcript directory inside a run.
pub const TRANSCRIPTS: &str = "transcripts";


/// Where one attempt at one `llm` step leaves its transcript.
///
/// One function rather than two spellings of the same format string: the step that *writes* the
/// transcript and the step that *checks* it are different steps of a map, and a checker pointed at
/// a path the writer never used would report that a session did nothing.
pub fn transcript_path(run_directory: &Path, state: &StateId, index: usize, attempt: u32) -> PathBuf {
    run_directory
        .join(TRANSCRIPTS)
        .join(format!("{state}-{index}-{attempt}.jsonl"))
}


/// Expands the placeholders a step map admits, or says which one it could not.
///
/// The vocabulary is `aep_driver_spec::map::CommandStep::PLACEHOLDERS` and an unknown name is
/// refused at load, so the only failures reachable here are the two that are facts about the
/// **run** rather than about the document: a `{transcript}` in a run where the `llm` step before it
/// has not run, and a `{task}` in a run that was not started from a task document. Neither is
/// decidable at load, and both are D5's `Unknown` rather than a guess.
///
/// `{task}` expands to the task document's path **as the driver resolved it**, which
/// `DriveLocation::inputs` makes absolute: a `command` step is spawned with the project directory
/// as its working directory, so a relative `--task` — resolved against whatever directory the
/// operator typed it in — is a path the child would open somewhere else or not at all.
pub fn expand(word: &str, context: &StepContext<'_>) -> Result<String, String> {
    let mut expanded = word.to_owned();
    for name in placeholders_in(word) {
        let value = match name {
            "run_directory" => context.run_directory.display().to_string(),
            "task" => {
                let Some(document) = context.task_document else {
                    return Err(format!(
                        "`{{task}}` is the task document this run was started from, and task \
                         `{}` was not read out of one",
                        context.task.id
                    ));
                };
                document.display().to_string()
            }
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


/// The one environment variable that may name a plugin directory (AGENTS.md invariant 12).
///
/// `pub(crate)` because `aep doctor` reports on the same directories this drives with, and a second
/// spelling of the name is how a rename leaves the preflight checking a variable nothing reads.
pub const PLUGIN_DIR_ENV: &str = "AEP_DRIVE_PLUGIN_DIR";


/// What can be done with a driven run.
#[derive(Debug, Subcommand)]
pub enum DriveCommand {
    /// Start a new run of a task, allocating a run id.
    Run(RunArgs),
    /// Report what the store's last run is doing, and who holds the lock.
    Status(StatusArgs),
    /// Continue a run that stopped, re-taking the store lock.
    Resume(ResumeArgs),
    /// Answer one `before-call` hook consultation from the native loop, on stdin.
    ///
    /// **The same content rule the vendor arm enforces in-process, reachable as a program.**
    /// `decide_tool` runs inside this process for a `claude` step because that arm's calls come
    /// back through the metaharness seam. The native loop decides in-process and consults hooks
    /// instead, so the rule has to be *spawnable*; this is the spelling that makes it so, and it
    /// calls the same `store_integrity_at` rather than restating it. A second copy of a rule is a
    /// second rule. Where a step may write at all is not asked here: that is the step map's
    /// `scope:`, which reaches this loop as `--write-scope`.
    ///
    /// Reads the `--hooks` protocol on stdin and answers with an exit status: `0` proceeds, `2`
    /// blocks with `{"reason": …}` on stdout. Not for people to run.
    #[command(hide = true)]
    Hook,
    /// Answer one `transition` hook consultation from the native loop, on stdin — the governor.
    ///
    /// **The engine, reachable as a program at a section boundary.** `b10x-harness workflow run`
    /// walks a flow `protocol workflow flow` projected from a workflow, and that projection is an
    /// ordering and not a government: no guard travels. The loop asks a `transition` hook before
    /// a section is entered and after it leaves, and this verb is what answers it from the engine
    /// — `evaluate` for entering, `transition` for leaving — so a native walk is governed by the
    /// same documents that govern a driven run, with no crate dependency in either direction.
    ///
    /// Reads the loop's `transition` document on stdin and answers with an exit status: `0`
    /// proceeds, `2` refuses with `{"reason": …}` on stdout, in the engine's own words. Positions
    /// the engine on a run's cursor when `--run` names one, and on the state the flow path names
    /// otherwise. Decides only; writes nothing and takes no lock.
    Transition(TransitionArgs),
}


/// The arguments of `protocol drive transition`.
#[derive(Debug, Args)]
pub struct TransitionArgs {
    /// Where the documents, the task and the store are.
    #[command(flatten)]
    pub location: DriveLocation,
    /// The run whose snapshot positions the engine, such as `AUTH-142/3`.
    ///
    /// Without it the engine is positioned on the state the flow path names — the section's first
    /// state on `enter`, its last on `leave` — over the store as it is now, which is what a native
    /// walk that has no run of its own gets.
    #[arg(long)]
    pub run: Option<String>,
}


/// Where the run's inputs are.
#[derive(Debug, Clone, Args)]
pub struct DriveLocation {
    /// The project directory — the one holding `.engineering`. Discovered when omitted.
    #[arg(long)]
    pub project: Option<PathBuf>,
    /// The document tree. Comes from the project when omitted.
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// The task document. Comes from the project when omitted.
    #[arg(long)]
    pub task: Option<PathBuf>,
    /// The planning store, as a markdown directory. Defaults to the store `project.yaml` names —
    /// `.engineering/planning/` unless it says `store: sqlite` or `store: postgres`.
    #[arg(long)]
    pub store: Option<PathBuf>,
    /// The step map: a file, or the id of one in the document tree.
    #[arg(long)]
    pub map: Option<String>,
    /// A plugin directory to load into every `llm` step's session. Repeatable.
    ///
    /// **W3.4's integration seam, and the reason it is a flag rather than a constant.** The
    /// plugin's `hooks/hooks.json` is the driver's enforcement arm — the layer that sees a tool's
    /// *arguments*, which `--allowedTools` cannot — and a session that never loaded the plugin
    /// never loaded the hooks. Where the plugin lives is a property of the machine, not of the
    /// protocol, so it is named here rather than guessed at. `AEP_DRIVE_PLUGIN_DIR` supplies it
    /// when the flag is absent, which is what lets an eval script set it once for a whole run.
    #[arg(long)]
    pub plugin_dir: Vec<PathBuf>,
}


/// The arguments of `protocol drive run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Where the run's inputs are.
    #[command(flatten)]
    pub location: DriveLocation,
    /// The maximum this driven run may reserve for model sessions, in US dollars.
    ///
    /// Required when the selected map contains an `llm` step. The cap is checked before every
    /// metaharness spawn; a cap applied after a session exits would be a receipt, not a bound.
    #[arg(long, value_name = "USD")]
    pub budget_usd: Option<String>,
    /// The conservative charge reserved before each model session, in US dollars.
    ///
    /// Required with `--budget-usd`, rather than defaulted: when a transcript cannot state cost,
    /// only the operator can say what one more launch is allowed to count as.
    #[arg(long, value_name = "USD", requires = "budget_usd")]
    pub assume_usd_per_run: Option<String>,
    /// Run until the first thing a person owes, then persist and exit 0.
    #[arg(long)]
    pub pause_on_approval: bool,
    /// The one non-human actor whose recorded approval may answer an `operator` step.
    ///
    /// `agent:<name>`. It answers nothing itself: the run still stops at the step, the named
    /// actor records its approval against the run's snapshot while the run is stopped, and the
    /// resume counts it — and refuses it by name when it is this run's own actor. A person's
    /// approval is admissible without being named, on every run. Needs `--pause-on-approval`,
    /// because an answer that arrives while the run is stopped needs a run that can stop.
    #[arg(long, value_name = "ACTOR", requires = "pause_on_approval")]
    pub approver: Option<ActorRef>,
    /// Stop after this many loop iterations, whatever the state of the run.
    #[arg(long, default_value_t = 25)]
    pub max_iterations: u32,
    /// Take the store lock from a holder that is provably dead.
    #[arg(long)]
    pub take_lock: bool,
    /// Start even though the map cannot produce evidence the plan will demand.
    ///
    /// **This weakens no rule the engine enforces.** The pre-flight it turns off is an *economic*
    /// check, not a protocol one: without it a run walks every state and blocks at the guard that
    /// wanted the record, which for `W4-2/1` cost $31.46 and 76 minutes. With this flag the gap is
    /// still printed and the run still blocks at that guard — the caller has simply said they know,
    /// which is the position somebody driving a run to a `--pause-on-approval` stop and supplying
    /// the record by hand is legitimately in.
    #[arg(long)]
    pub allow_evidence_gap: bool,
}


/// The arguments of `protocol drive status`.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Where the run's inputs are.
    #[command(flatten)]
    pub location: DriveLocation,
    /// Which run to report on. The store's current one when omitted.
    #[arg(long)]
    pub run: Option<String>,
}


/// The arguments of `protocol drive resume`.
#[derive(Debug, Args)]
pub struct ResumeArgs {
    /// The run to continue, such as `AUTH-142/3`.
    pub run: String,
    /// Where the run's inputs are.
    #[command(flatten)]
    pub location: DriveLocation,
    /// Run until the first thing a person owes, then persist and exit 0.
    #[arg(long)]
    pub pause_on_approval: bool,
    /// The one non-human actor whose recorded approval may answer an `operator` step.
    ///
    /// As on `run`. Remembered from the launch when omitted, so a resume admits whoever the run
    /// was started admitting; given here, it replaces that for this resume and the ones after.
    #[arg(long, value_name = "ACTOR")]
    pub approver: Option<ActorRef>,
    /// Stop after this many loop iterations, whatever the state of the run.
    #[arg(long, default_value_t = 25)]
    pub max_iterations: u32,
    /// Narrow the run's remembered dollar cap.
    ///
    /// A resume may lower the original cap and may not raise it. The per-session assumption is
    /// remembered unchanged from the launch.
    #[arg(long, value_name = "USD")]
    pub budget_usd: Option<String>,
    /// Take the store lock from a holder that is provably dead.
    #[arg(long)]
    pub take_lock: bool,
    /// Retry the unresolved outside attempt with this exact persisted attempt id.
    #[arg(
        long,
        value_name = "ATTEMPT",
        conflicts_with = "record_in_flight_no_verdict"
    )]
    pub retry_in_flight: Option<String>,
    /// Resolve an uncertain outside attempt as having produced no verdict.
    #[arg(long, conflicts_with = "retry_in_flight")]
    pub record_in_flight_no_verdict: bool,
}


/// Runs one `protocol drive` verb.
pub fn run(command: DriveCommand) -> Result<ExitCode> {
    match command {
        DriveCommand::Run(args) => start_with_host(&args, &CommandOnlyHost),
        DriveCommand::Status(args) => status(&args),
        DriveCommand::Resume(args) => resume_with_host(&args, &CommandOnlyHost),
        DriveCommand::Hook => bail!("use `metaharness aep drive hook` for the native execution adapter"),
        DriveCommand::Transition(_) => bail!("use `metaharness aep drive transition` for the native execution adapter"),
    }
}


/// The project this was run in, or a refusal naming what to pass instead.
pub fn discover_project() -> Result<PathBuf> {
    let here = std::env::current_dir().context("reading the working directory")?;
    let directory = project_directory();
    aep_project::project::discover(&here).with_context(|| {
        format!(
            "no `--project` was given and no `{directory}/project.yaml` was found in {} or \
             any parent",
            here.display()
        )
    })
}


/// A path a child process can open, whatever directory it is started in.
///
/// [`std::path::absolute`] and not [`Path::canonicalize`]: this must not touch the filesystem or
/// resolve a symlink. A task document reached through a symlinked worktree is the document the
/// operator named, and rewriting it to the link's target would put a path in a run's record that
/// the operator never typed. A path the platform refuses to absolutize is left as it was — a
/// working relative path is better than a lost one.
pub fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}


/// Everything a run needs, resolved from flags or from the project it was run in.
pub struct Inputs {
    /// The project directory — the one holding `.engineering`.
    pub project: PathBuf,
    /// The documents in force.
    pub registry: Registry,
    /// The task being driven.
    pub task: Task,
    /// The document [`Inputs::task`] was read from, absolute.
    ///
    /// What `{task}` expands to. Absolute because a `command` step is spawned with the project
    /// directory as its working directory and `--task` is relative to the operator's own: the two
    /// are the same directory often enough that a relative path would work in testing and open the
    /// wrong document — or nothing — in a run started from anywhere else.
    pub task_document: PathBuf,
    /// The planning store the artifact graph is rebuilt from every iteration.
    pub store: crate::planning::DrivenPlan,
    /// The step map driving the run.
    pub map: StepMap,
    /// Where the step map came from, for a report.
    pub map_origin: String,
    /// The plugin directories every `llm` step's session loads.
    pub plugin_dirs: Vec<PathBuf>,
}


impl DriveLocation {
    /// Resolves the run's inputs.
    pub fn inputs(&self) -> Result<Inputs> {
        let project = match &self.project {
            Some(path) => path.clone(),
            None => discover_project()?,
        };

        // The registry is loaded **once per invocation** and the store is rebuilt **per
        // iteration**, and the asymmetry is chosen rather than accidental (review finding F8): a
        // mid-run edit to `workflows/` is not picked up, because the cursor pins the workflow for
        // the life of the run precisely so a governing document cannot move under it; a mid-run
        // edit to the planning store *is*, because that is the work happening.
        let (registry, drivers) = if let Some(root) = &self.root {
            let loaded = crate::load_documents(root)?;
            (loaded.registry, loaded.drivers)
        } else {
            let loaded = aep_project::project::load(&project)
                .map_err(|errors| anyhow::anyhow!("{errors}"))?;
            (loaded.registry, loaded.drivers)
        };

        // Both halves of the answer, together: what is being driven, and which file said so. The
        // second is what `{task}` expands to, and it is resolved here — beside the read — so there
        // is no second reading of *which document is this run's task* to drift from the first.
        let (task, task_document) = if let Some(path) = &self.task {
            (crate::read_task(path)?, absolute(path))
        } else {
            let loaded = aep_project::project::load(&project)
                .map_err(|errors| anyhow::anyhow!("{errors}"))?;
            let document = absolute(&loaded.paths.task);
            let task = loaded
                .task
                .context("the project names no task, and no `--task` was given")?;
            (task, document)
        };

        let store = crate::planning::DrivenPlan::for_project(self.store.as_deref(), &project)?;

        let (map, map_origin) = self.step_map(&registry, &drivers, &task)?;

        let plugin_dirs = self.plugin_dirs(&project);

        Ok(Inputs {
            project,
            registry,
            task,
            task_document,
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
    #[must_use]
    pub fn remembering(&self, launch: Option<&Launch>, project: &Path) -> Self {
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

    /// The plugin directories a session loads: explicit flags, then the environment.
    ///
    /// The environment is a fallback and never an addition — a caller that named directories meant
    /// those directories, and silently appending one from the ambient environment is how a run
    /// ends up enforcing something its own command line does not mention.
    ///
    /// AEP bundles no plugin sources, so there is no repository-relative fallback. The external
    /// agentplugins checkout or installation is authority the operator must name.
    pub fn plugin_dirs(&self, _project: &Path) -> Vec<PathBuf> {
        if !self.plugin_dir.is_empty() {
            return self.plugin_dir.clone();
        }
        if let Some(value) = std::env::var_os(PLUGIN_DIR_ENV) {
            return vec![PathBuf::from(value)];
        }
        Vec::new()
    }

    /// The step map: the file named by `--map`, the map with that id, or the only one that fits.
    pub fn step_map(
        &self,
        registry: &Registry,
        drivers: &aep_project::load::DriverRegistry,
        task: &Task,
    ) -> Result<(StepMap, String)> {
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
            let map = drivers
                .get(&id)
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
        let fitting: Vec<&StepMap> = drivers
            .iter()
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
pub fn start_with_host(args: &RunArgs, host: &dyn ExecutionHost) -> Result<ExitCode> {
    let inputs = args.location.inputs()?;
    let runs = runs_directory(&inputs.project)?;

    let engine = Engine::new(inputs.registry.clone()).with_ess_conformance_v2_reader(std::sync::Arc::new(aep_ess_evidence::CountStageReader)).with_ess_conformance_coverage_reader(std::sync::Arc::new(aep_ess_evidence::CoverageReader));
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

    let prepared = host.prepare(&inputs, None, args.budget_usd.as_deref(), args.assume_usd_per_run.as_deref())?;
    if prepared.protocol_binary.is_none() {
        if let Some(refusal) = protocol_command_preflight(&inputs.map) { bail!("{refusal}"); }
    }

    // D3(c): the headless pre-flight, static and decidable and run before anything executes.
    if let Some(code) = refuse_owed(&plan, &inputs.map, args.pause_on_approval) {
        return Ok(code);
    }
    if let Some(code) = refuse_approver(args.approver.as_ref(), &inputs.task.id, &inputs.map) {
        return Ok(code);
    }

    let lock = take_lock(&runs, args.take_lock)?;
    let run_id = allocate_run(&runs, &inputs.task.id)?;
    lock.record_run(&run_id)?;
    let directory = RunDirectory::at(run_path(&runs, &run_id));
    fs::create_dir_all(directory.path())
        .with_context(|| format!("creating {}", directory.path().display()))?;
    // How this run was launched, so `resume` needs none of it again and the line this command
    // prints is a line that works.
    let launch = Launch {
        task: args.location.task.clone(),
        task_document: Some(inputs.task_document.clone()),
        map: args.location.map.clone(),
        project: Some(inputs.project.clone()),
        root: args.location.root.clone(),
        pause_on_approval: args.pause_on_approval,
        approver: args.approver.clone(),
        plugin_dirs: inputs.plugin_dirs.clone(),
        extra: prepared.launch_fields.clone(),
    };
    launch.write_required(directory.path())?;
    fs::write(runs.join(CURRENT_FILE), format!("{run_id}\n"))
        .with_context(|| format!("writing {}", runs.join(CURRENT_FILE).display()))?;

    let options = DriverOptions {
        max_iterations: args.max_iterations,
        pause_on_approval: args.pause_on_approval,
        headless: true,
        approver: args.approver.clone(),
        task_document: Some(inputs.task_document.clone()),
        // The theft travels into the cursor the driver writes, rather than only onto stdout below:
        // a note in the terminal lives exactly as long as the scrollback, which is not where
        // anybody looks a week later when two runs turn out to have overlapped.
        stolen_lock: lock.stolen().cloned(),
        in_flight_resolution: None,
    };
    let context = ExecutorContext {
        working_directory: inputs.project.clone(),
        run_directory: directory.path().to_path_buf(),
        plugin_dirs: inputs.plugin_dirs.clone(),
        workflow_id: inputs.map.workflow.id().to_string(),
        workflow_version: inputs.map.workflow.major().to_string(),
    };
    let protocol_binary = prepared.protocol_binary.clone();
    let llm = (prepared.executor)(&context, false)?;
    let mut executors = CliExecutors {
        working_directory: context.working_directory,
        run_directory: context.run_directory,
        approver: args.approver.clone(),
        protocol_binary,
        llm,
    };
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
    let outcome = finish_with_command(report, &run_id, &inputs.map_origin, host.resume_command());
    lock.release();
    outcome
}


/// `protocol drive resume`
pub fn resume_with_host(args: &ResumeArgs, host: &dyn ExecutionHost) -> Result<ExitCode> {
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
    let mut launch = Launch::read(directory.path());
    let location = args.location.remembering(launch.as_ref(), &project);
    let inputs = location.inputs()?;
    let pause_on_approval =
        args.pause_on_approval || launch.as_ref().is_some_and(|l| l.pause_on_approval);
    // Whose answer counts, remembered from the launch for the same reason the map is: a resume
    // that admitted a different approver would be a run whose second half was governed by a
    // policy its own record does not name. Given here, it replaces the remembered one.
    let approver = args
        .approver
        .clone()
        .or_else(|| launch.as_ref().and_then(|l| l.approver.clone()));

    let prepared = host.prepare(&inputs, launch.as_ref(), args.budget_usd.as_deref(), None)?;
    // What `{task}` expands to, taken from the launch record whenever the caller did not name a
    // task on this invocation. Resolving it again here would resolve it against *this* process's
    // working directory, so a resume typed from somewhere else would hand a `command` step a
    // different path than the run's own earlier steps got — one run, two documents, and neither
    // the step nor its record saying which. A flag still wins, exactly as it does in
    // `remembering`: an operator who names a task on a resume means that task.
    let task_document = if args.location.task.is_some() {
        inputs.task_document.clone()
    } else {
        launch
            .as_ref()
            .and_then(|l| l.task_document.clone())
            .unwrap_or_else(|| inputs.task_document.clone())
    };

    // The same pre-flights `run` does, and a resume needs them just as much: a resume re-takes the
    // lock, so discovering the missing binary mid-step costs a lock and an attempt in the cursor of
    // a run that was already stopped once.
    if prepared.protocol_binary.is_none() {
        if let Some(refusal) = protocol_command_preflight(&inputs.map) { bail!("{refusal}"); }
    }

    // A paused run holds no lock, because the pause has no bound — so a resume must **re-take** it,
    // and must refuse when another run now holds it. The first draft said a pause releases and
    // never said a resume re-acquires, which left the obvious assumption to produce two live runs.
    let lock = take_lock(&runs, args.take_lock)?;
    lock.record_run(&run_id)?;

    if let Some(record) = launch.as_mut() {
        record.extra.extend(prepared.launch_fields.clone());
        record.write_required(directory.path())?;
    }

    if let Some(code) = refuse_approver(approver.as_ref(), &inputs.task.id, &inputs.map) {
        return Ok(code);
    }
    if approver.is_some() && !pause_on_approval {
        outln!(
            "`--approver` names whose recorded approval may answer an `operator` step while the \
             run is stopped, and this run cannot stop: pass `--pause-on-approval` as well"
        );
        return Ok(ExitCode::from(1));
    }

    let engine = Engine::new(inputs.registry.clone()).with_ess_conformance_v2_reader(std::sync::Arc::new(aep_ess_evidence::CountStageReader)).with_ess_conformance_coverage_reader(std::sync::Arc::new(aep_ess_evidence::CoverageReader));
    let options = DriverOptions {
        max_iterations: args.max_iterations,
        pause_on_approval,
        headless: true,
        approver: approver.clone(),
        task_document: Some(task_document),
        // A resume re-takes the lock through the same `take_lock`, so it can supersede one too. The
        // most recent supersession is the answer to *which lock did this run take*; a resume that
        // stole nothing carries `None`, and the driver leaves any earlier theft where it is.
        stolen_lock: lock.stolen().cloned(),
        in_flight_resolution: if let Some(id) = &args.retry_in_flight {
            Some(InFlightResolution::Retry(id.clone()))
        } else if args.record_in_flight_no_verdict {
            Some(InFlightResolution::RecordNoVerdict)
        } else {
            None
        },
    };
    let context = ExecutorContext {
        working_directory: inputs.project.clone(),
        run_directory: directory.path().to_path_buf(),
        plugin_dirs: inputs.plugin_dirs.clone(),
        workflow_id: inputs.map.workflow.id().to_string(),
        workflow_version: inputs.map.workflow.major().to_string(),
    };
    let protocol_binary = prepared.protocol_binary.clone();
    let llm = (prepared.executor)(&context, true)?;
    let mut executors = CliExecutors {
        working_directory: context.working_directory,
        run_directory: context.run_directory,
        approver,
        protocol_binary,
        llm,
    };
    let report = aep_driver::run::resume(
        &engine,
        &inputs.task,
        &inputs.store,
        &inputs.map,
        &directory,
        &mut executors,
        &options,
    );
    let outcome = finish_with_command(report, &run_id, &inputs.map_origin, host.resume_command());
    lock.release();
    outcome
}


/// `protocol drive status`
pub fn status(args: &StatusArgs) -> Result<ExitCode> {
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
pub fn print_cursor(cursor: &DriverCursor) {
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
    if let Some(owed) = &cursor.owed {
        outln!(
            "owed       step {} of {}: {}",
            owed.step,
            owed.state,
            owed.prompt
        );
    }
    for answer in &cursor.answers {
        outln!(
            "answered   step {} of {} by {} (approval `{}`, evidence {})",
            answer.step,
            answer.state,
            answer.by,
            answer.approval,
            answer.evidence
        );
    }
    for reason in &cursor.reasons {
        outln!("           {reason}");
    }
}


/// Why `--approver` may not name this actor for this run, or `None` when it may.
///
/// Checked before a run id, a lock and a model bill exist, for the same reason every other
/// pre-flight is: a person, `system`, a service, and the run's own actors — the task, the
/// execution it will be given, and the harness each `llm` step runs under — can never answer.
pub fn approver_refusal(named: &ActorRef, task: &TaskId, map: &StepMap) -> Option<String> {
    let mut own_names: Vec<String> = vec![task.to_string()];
    for entry in map.states.values() {
        for step in &entry.steps {
            if let Step::Llm(llm) = step {
                own_names.push(llm.harness.clone());
            }
        }
    }
    let mut own: Vec<ActorRef> = own_names
        .iter()
        .filter_map(|name| ActorRef::parse(&format!("agent:{name}")).ok())
        .collect();
    // The execution id is `<task>.<ordinal>`, and the ordinal is not known until the engine
    // mints it: refuse the whole family by prefix rather than let `agent:T-1.2` through.
    if let ActorRef::Agent(name) = named {
        if name
            .strip_prefix(&format!("{task}."))
            .is_some_and(|ordinal| {
                !ordinal.is_empty() && ordinal.chars().all(|c| c.is_ascii_digit())
            })
        {
            own.push(named.clone());
        }
    }
    aep_driver::attest::naming_refusal(named, &own)
        .map(|reason| format!("`--approver {named}` is refused: {reason}"))
}


/// Renders a finished run and chooses the exit code.
pub fn finish(
    report: Result<RunReport, DriveError>,
    run: &RunId,
    map_origin: &str,
) -> Result<ExitCode> {
    finish_with_command(report, run, map_origin, "protocol drive resume")
}

/// Render the continuation command owned by the execution host.
fn finish_with_command(
    report: Result<RunReport, DriveError>,
    run: &RunId,
    map_origin: &str,
    resume_command: &str,
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
        outln!("resume with: {resume_command} {run}");
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
pub fn report_evidence_gap(report: &CoverageReport, origin: &str, allowed: bool) {
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


/// Refuses a headless start that would reach something only a person can answer.
///
/// D3(c): static, decidable, and before anything executes. Prints every entry with the document
/// that asked for it, and the two flags that change the answer.
pub fn refuse_owed(
    plan: &aep_domain::plan::ExecutionPlan,
    map: &StepMap,
    pause_on_approval: bool,
) -> Option<ExitCode> {
    let owed = owed_to_a_person(plan, map);
    if owed.is_empty() || pause_on_approval {
        return None;
    }
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
         flag that answers one — nothing below the driver checks who granted an approval, so the \
         refusal has to be the driver's — but there is one that says whose answer counts: a \
         person's always does, and `--approver agent:<name>` admits one named agent's recorded \
         approval as well, never this run's own."
    );
    Some(ExitCode::from(1))
}


/// Refuses an `--approver` this run may not admit, printing why.
pub fn refuse_approver(named: Option<&ActorRef>, task: &TaskId, map: &StepMap) -> Option<ExitCode> {
    let refusal = approver_refusal(named?, task, map)?;
    outln!("{refusal}");
    Some(ExitCode::from(1))
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
pub fn owed_to_a_person(plan: &aep_domain::plan::ExecutionPlan, map: &StepMap) -> Vec<String> {
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
pub fn reachable_states(workflow: &aep_domain::workflow::Workflow) -> BTreeSet<StateId> {
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
pub fn runs_directory(project: &Path) -> Result<PathBuf> {
    let runs = project.join(project_directory()).join(RUNS_DIRECTORY);
    fs::create_dir_all(&runs).with_context(|| format!("creating {}", runs.display()))?;
    Ok(runs)
}


/// The directory of one run.
pub fn run_path(runs: &Path, run: &RunId) -> PathBuf {
    let [task, ordinal] = run.segments();
    runs.join(task).join(ordinal)
}


/// The next run id for a task: one more than the highest this runs directory has any record of.
///
/// Allocated **after** the lock is taken, which is the whole of review finding F2.
///
/// **The floor is the listing *and* `current`, because a directory listing is not a history.** The
/// first draft took the highest existing directory and stated *"a run directory is never deleted
/// and never reused"* as though the second clause followed from the first. Nothing on disk stops
/// anybody deleting one, and run directories hold transcripts — they are the largest thing a driven
/// repository accumulates, so an operator reclaiming the newest one is the ordinary case. Deleting
/// it handed its id straight back out, to a different run, while [`CURRENT_FILE`] still named it:
/// two runs answering to one id, and every evidence record, cursor `took_lock_from` and report that
/// mentioned it made ambiguous after the fact.
///
/// `current` is written on every `run` and names the newest ever allocated, so it survives the
/// deletion of the directory it points at and raises the floor above it. **The limit is stated
/// rather than hidden:** an operator who deletes `current` as well has removed the last record that
/// this id was ever used, and the count restarts from the listing. Recovering an id from a deleted
/// run's evidence would mean reading the whole store on every allocation, for a case that is a
/// person deleting a pointer file rather than a directory of transcripts.
pub fn allocate_run(runs: &Path, task: &TaskId) -> Result<RunId> {
    let directory = runs.join(task.as_str());
    let mut highest = highest_named_by_current(runs, task);
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


/// The ordinal [`CURRENT_FILE`] names, when it names a run of `task`.
///
/// `0` for every way of not knowing — no file, an unreadable one, one that does not parse, one that
/// names another task — because this only ever raises a floor. A pointer this process cannot read
/// is not a reason to refuse to start a run; it is a reason to fall back on the listing, which is
/// what the allocator did before this existed.
pub fn highest_named_by_current(runs: &Path, task: &TaskId) -> u32 {
    fs::read_to_string(runs.join(CURRENT_FILE))
        .ok()
        .and_then(|text| text.trim().parse::<RunId>().ok())
        .filter(|run| run.task() == task.as_str())
        .map_or(0, |run| run.ordinal())
}


/// What `lock.json` holds.
///
/// **No timestamp.** Staleness is decided by liveness rather than by a number somebody wrote into a
/// file: any age threshold has to exceed the longest legitimate step, and the longest legitimate
/// step is an `operator` step waiting for a person, which has no bound. A driver that broke a lock
/// after two hours would break exactly the runs that paused correctly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockFile {
    /// The run it granted, once one has been allocated.
    pub run: Option<String>,
    /// The process holding it.
    pub pid: u32,
    /// The host that process is on.
    pub host: String,
    /// The driver that took it.
    pub driver: String,
}


/// A lock this process holds, and the run it took it from if it took one.
pub struct HeldLock {
    /// Lock file owned by this guard.
    pub path: PathBuf,
    /// Previous holder superseded by an explicit lock takeover.
    pub stolen: Option<StolenLock>,
    /// Whether the file has already been removed, so [`Drop`] does not remove it twice.
    pub released: bool,
}


impl HeldLock {
    /// Records the run id inside the lock, so a refusal can name it without a second read.
    pub fn record_run(&self, run: &RunId) -> Result<()> {
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
    pub fn stolen(&self) -> Option<&StolenLock> {
        self.stolen.as_ref()
    }

    /// Releases the lock.
    ///
    /// Called on every exit path the driver controls, including the approval pause and budget
    /// exhaustion: a paused run does not hold a lock, because the pause has no bound. What a paused
    /// run keeps is `current`, so resuming is one word.
    ///
    /// Kept as an explicit call, and consuming, even though [`Drop`] now does the same thing: the
    /// ordinary end of a run releasing its lock is a decision the code should state, and a reader
    /// who finds only a scope ending has to prove the absence of a later use to know the lock is
    /// gone. `Drop` is the floor under the paths nobody chose.
    pub fn release(mut self) {
        self.remove();
    }

    /// Removes the lock file, once.
    ///
    /// Idempotent by the flag rather than by `remove_file`'s own tolerance of a missing file,
    /// because those are different things: a second `remove_file` after a successful first one can
    /// delete a lock **another process** took in the window between them, which is the one failure
    /// the lock exists to prevent.
    pub fn remove(&mut self) {
        if !self.released {
            self.released = true;
            let _ = fs::remove_file(&self.path);
        }
    }
}


/// The lock is released when the value goes out of scope, however it goes out of scope.
///
/// [`HeldLock::release`] is called on every exit path the driver *chose*, and the leak was in the
/// paths it did not: between `take_lock` and the release, `protocol drive run` has four fallible
/// steps of its own — `allocate_run`, [`HeldLock::record_run`], `create_dir_all` of the run
/// directory and the write of `current` — and `resume` has two early returns as well. Any of them
/// left `lock.json` on disk naming a pid that had already exited, so the operator's next `drive`
/// was refused by a run that never started, and told to `--resume` it.
///
/// A `Drop` rather than a repaired call at each site because the set of exit paths is not closed:
/// the next `?` added between the two lines would reintroduce it, and would look correct.
impl Drop for HeldLock {
    fn drop(&mut self) {
        self.remove();
    }
}


/// A lock somebody else holds.
pub struct Holder {
    /// Recorded identity of the current lock holder.
    pub file: LockFile,
    /// The runs directory the lock sits in, so the holding run's own cursor can be found.
    pub runs: PathBuf,
}


impl Holder {
    /// The holder as the router sees it — a value, never a probe.
    pub fn state(&self) -> LockState {
        LockState {
            run: self
                .file
                .run
                .clone()
                .unwrap_or_else(|| "<unallocated>".to_owned()),
            pid: self.file.pid,
            host: self.file.host.clone(),
            liveness: liveness(&self.file),
            state: self.holder_state(),
        }
    }

    /// What the holding run is doing, read from **its own** `cursor.json`.
    ///
    /// The state comes from the cursor and never from `lock.json`. A lock file is written once when
    /// the lock is taken; the state changes after every step of the run it describes, so a state in
    /// the lock file would be wrong for most of that run's life — and a stale copy of a live fact is
    /// worse than no copy, because it is a fact the operator will act on.
    ///
    /// Every way this can fail answers `None`, and none of them is an error: no run id allocated yet
    /// (the window between `create_new` and [`HeldLock::record_run`]), no run directory, no cursor,
    /// or a cursor that will not parse. This process is reading **somebody else's** file, a refusal
    /// is the answer either way, and a `bail!` here would let one corrupt document in one run
    /// directory end every subsequent invocation against the store.
    pub fn holder_state(&self) -> Option<String> {
        let run: RunId = self.file.run.as_deref()?.parse().ok()?;
        RunDirectory::at(run_path(&self.runs, &run))
            .read_cursor()
            .ok()
            .map(|cursor| cursor.state.to_string())
    }
}


/// Reads the lock, when there is one.
pub fn read_lock(runs: &Path) -> Result<Option<Holder>> {
    let path = runs.join(LOCK_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    let file: LockFile =
        serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    Ok(Some(Holder {
        file,
        runs: runs.to_path_buf(),
    }))
}


/// Takes the store lock, or refuses and names the holder.
pub fn take_lock(runs: &Path, force: bool) -> Result<HeldLock> {
    let path = runs.join(LOCK_FILE);
    let mine = LockFile {
        run: None,
        pid: std::process::id(),
        host: host(),
        driver: format!("aep-cli {}", env!("CARGO_PKG_VERSION")),
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
            if let Err(error) = handle.write_all(body.as_bytes()) {
                // `create_new` has already reserved the path and the body never arrived, so what is
                // on disk is a zero-byte `lock.json` that **no `HeldLock` exists to `Drop`** — the
                // one leak the `Drop` impl below cannot reach, because there is nothing to drop.
                // The residue is worse than a leaked lock: a lock file that does not parse is read
                // by every verb, `--take-lock` included, so the documented route out is refused by
                // the thing it is the route out of.
                //
                // Removed here rather than left for a later reader, on the one failure this process
                // is still running to act on. A crash or a SIGKILL between the two syscalls still
                // leaves the file, and making *that* residue recoverable is a different fix in
                // `read_lock` — it belongs to whoever reads a lock, not to whoever writes one.
                drop(handle);
                let _ = fs::remove_file(&path);
                return Err(error).with_context(|| format!("writing {}", path.display()));
            }
            return Ok(HeldLock {
                path,
                stolen: None,
                released: false,
            });
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
        released: false,
    })
}


/// Whether the process named in a lock is alive, dead, or somebody else's problem.
///
/// **Liveness, never age.** A pid on another host says nothing to this one's process table, so a
/// lock naming another host is never stale here whatever the local table says.
pub fn liveness(file: &LockFile) -> Liveness {
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
pub fn host() -> String {
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


/// This CLI's own name, which is what a `command` step writes when it means *this build*.
pub const PROTOCOL_BINARY: &str = "protocol";


/// The run's record of which binary each `command` step attempt actually spawned.
///
/// Beside the cursor rather than inside it, for [`Launch`]'s reason: the cursor is
/// `aep.driver-cursor/1`, a published document about **where a run is**, and which binary answered
/// a step is not that. One JSON object per line and append-only, so an attempt that took the
/// process down with it still left its line behind — which is the attempt a reader is looking for.
pub const COMMANDS_FILE: &str = "commands.jsonl";


/// How a `command` step's program was turned into something to spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Resolution {
    /// Spawned exactly as the map wrote it — every program that is not this CLI.
    AsWritten,
    /// `protocol`, spawned as the binary this driver **is**.
    Driver,
    /// `protocol`, and this process could not name its own binary, so `PATH` decided.
    PathFallback,
}


/// One `command` step attempt, as the map wrote it and as it was spawned.
#[derive(Debug, serde::Serialize)]
pub struct CommandRun<'a> {
    /// The workflow state the step belongs to.
    pub state: String,
    /// Which step of that state's list it is.
    pub index: usize,
    /// Which attempt at it this was, counting from `1`.
    pub attempt: u32,
    /// The program the map wrote.
    pub program: &'a str,
    /// The program that was spawned.
    pub ran: &'a str,
    /// How the second was got from the first.
    pub resolved: Resolution,
}


/// What a `command` step's program resolves to, and the line that says so.
pub struct Resolved {
    /// The program to spawn.
    pub program: String,
    /// How it was arrived at.
    pub resolution: Resolution,
    /// What a reader has to be told, when the answer is not simply what the map wrote.
    pub note: Option<String>,
}


/// Resolves a `command` step's program: `protocol` is the binary this driver **is**.
///
/// **Run `W4-3/1`, 2026-08-28, is why.** Step 4 of `verify` was
/// `protocol property evidence --out …/property.yaml`. A `command` step is spawned by the driver
/// with the *driver's* environment, so the name resolved against the operator's own `PATH`, where
/// the first `protocol` was a 0.28.0 install predating the `property` verb. The step ran a binary
/// older than the map executing it, wrote no record, and the driver correctly reported *nothing
/// was observed* — three times, for real money, with the cause invisible in the message.
///
/// [`std::env::current_exe`] removes the failure rather than reporting it, and it buys the
/// agreement the run's whole evidence trail is recorded against: a record produced by a binary
/// nobody can name is the defect `version-check` exists for. It is keyed on the **file name**, so
/// `/usr/local/bin/protocol` is the same request written longer, and on nothing else — `cargo`,
/// `bash` and `git` are tools the driver finds the way it always did.
///
/// A process that cannot name its own binary falls back to the old behaviour and says so rather
/// than refusing: a run that cannot introspect itself is not a run that should stop. The refusal
/// for the case where that fallback is *known* to be wrong is [`protocol_command_preflight`], and
/// it happens before the run owns a run id.
pub fn resolve_program(written: &str) -> Resolved {
    let names_this_cli = Path::new(written)
        .file_name()
        .is_some_and(|name| name == PROTOCOL_BINARY);
    if !names_this_cli {
        return Resolved {
            program: written.to_owned(),
            resolution: Resolution::AsWritten,
            note: None,
        };
    }
    match std::env::current_exe() {
        Ok(executable) => {
            let program = executable.display().to_string();
            let note = Some(format!(
                "`{written}` is this driver's own build, {program} ({}); the driver's PATH was \
                 not consulted",
                env!("CARGO_PKG_VERSION")
            ));
            Resolved {
                program,
                resolution: Resolution::Driver,
                note,
            }
        }
        Err(error) => Resolved {
            program: written.to_owned(),
            resolution: Resolution::PathFallback,
            note: Some(format!(
                "`{written}` was resolved on the driver's PATH: this process cannot name its own \
                 binary ({error}), so the build that answered may not be the {} this run is \
                 recorded against",
                env!("CARGO_PKG_VERSION")
            )),
        },
    }
}


impl CliExecutors {
    /// Appends this attempt's line to the run's record of which binary answered.
    ///
    /// Best-effort, for [`Launch::write`]'s reason: a run that walks and cannot write down which
    /// binary it used leaves a reader worse off and is not wrong *now*, and a step refused because
    /// its bookkeeping file would not open is a refusal about nothing the protocol cares about.
    pub fn record_command(&self, context: &StepContext<'_>, written: &str, resolved: &Resolved) {
        let entry = CommandRun {
            state: context.state.to_string(),
            index: context.index,
            attempt: context.attempt,
            program: written,
            ran: &resolved.program,
            resolved: resolved.resolution,
        };
        let Ok(line) = serde_json::to_string(&entry) else {
            return;
        };
        let path = self.run_directory.join(COMMANDS_FILE);
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            use std::io::Write as _;
            let _ = writeln!(file, "{line}");
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
        let resolved = self.protocol_binary.as_ref().filter(|_| Path::new(&words[0]).file_name().is_some_and(|name| name == "aep" || name == "protocol")).map_or_else(|| resolve_program(&words[0]), |binary| Resolved { program: binary.display().to_string(), resolution: Resolution::Driver, note: Some("the host's selected AEP planning executable".to_owned()) });
        // The argv as **spawned**, not as written, because every message below quotes it and the
        // whole defect this closes was a message that named `protocol` while a namesake ran.
        let rendered = std::iter::once(resolved.program.as_str())
            .chain(words[1..].iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        self.record_command(context, &words[0], &resolved);
        // A `command` step is this process's own child, so the declared actor genuinely arrives:
        // a step map whose `run:` is a `protocol artifact …` writes to the store as the run, not
        // as whoever typed `protocol drive run`.
        let outcome = Process::new(&resolved.program)
            .args(&words[1..])
            .current_dir(&self.working_directory)
            .envs(session_env(context.execution))
            .env(
                crate::planning::COMMAND_IDENTITY_ENV,
                driver_command_identity(context),
            )
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
        // The step's own note, above its output: which binary answered, and — when that is not
        // simply what the map wrote — why. A reader of one log can tell a step that ran the
        // driver's own build from one that ran something it found.
        let mut body = format!("# ran: {rendered}\n");
        if let Some(note) = &resolved.note {
            body.push_str("# ");
            body.push_str(note);
            body.push('\n');
        }
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
        outln!();
        outln!(
            "who may answer: {}. Record the approval against this run's snapshot with `protocol \
             evaluate --evidence <file> --state <run>/snapshot.json`, or do what the prompt says, \
             then resume.",
            aep_driver::attest::admissible(self.approver.as_ref())
        );
        StepOutcome::Paused {
            reason: format!("an operator step in {} is owed an answer", context.state),
        }
    }
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
pub fn composes(command: &str) -> Option<char> {
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
pub const READ_ONLY_PROGRAMS: &[&str] = &["grep", "rg", "ls", "cat", "head", "tail", "wc"];


/// The engine's refusal as one line the model can act on.
///
/// The engine's own `DecisionExplanation` is four lines and belongs in a terminal; a `tool.decided`
/// event carries one reason string. Nothing is re-worded — the operation, the capability, the
/// decision, the document that decided and what is missing are all the engine's — and the layer is
/// named, so the event stream says who refused.
pub fn engine_refusal(decision: &Decision) -> String {
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


/// One `llm` step's declared write surface, as the seam reads it.
///
/// Two values, because a glob is only decidable against a path once you know what it is relative
/// to: the map writes `crates/**`, the vendor sends `/operator/repo/crates/govern/aep-domain/src/lib.rs`,
/// and a rule matched against the second answers about a repository nobody named.
#[derive(Debug, Clone, Copy)]
pub struct WriteSurface<'a> {
    /// The step's `scope:`, in the order the map wrote it — first match wins, so it is never
    /// sorted.
    ///
    /// Empty is **no scope declared**, which is not the same as a scope that allows everything
    /// (`aep_driver_spec::map::LlmStep::scope`). A map that said nothing restricts nothing here,
    /// and the way to restrict a step is to say so in the map.
    pub scope: &'a [ScopeRule],
    /// The working tree the scope's globs are written against.
    pub root: &'a Path,
}


/// The step map's `scope:`, enforced at the seam this arm has.
///
/// **The rule is read from the declaration; it is not written here.** Where a driven step may
/// write is a property of the work, so it belongs in a document a person can read —
/// `drivers/development/default.yaml` — rather than in a Rust function spelled in one vendor's
/// tool names, which is what it was for a year and which every other arm walked straight past.
/// The native execution host hands the same rules to its loop as `--write-scope` and enforces them
/// in its own tools; this is the vendor arm's half of the one declaration.
///
/// # Granularity is the harness's fact, and this is the harness
///
/// [`WriteScope::PartialOnly`] says *part of a file may be changed; a whole file may never be
/// replaced*, and deliberately names no operation — which of a harness's tools replace a whole
/// file is that harness's business, as `WriteScope`'s own documentation says. Here it is:
/// `Write` and `NotebookEdit` replace one, `Edit` changes part of one.
///
/// # Why falling off the end of a scope is not a decision
///
/// Validation refuses a scope whose last rule does not name `**`
/// (`aep_driver_spec::map::validated_scope`), so a declared scope has an answer for every path and
/// this function cannot reach its end with a rule left to find. The `None` arm is what a
/// hand-built scope in a test would hit, and it allows rather than inventing a verdict the
/// document does not contain.
pub fn declared_write(
    surface: WriteSurface<'_>,
    tool: &str,
    input: &serde_json::Value,
) -> Result<(), String> {
    if surface.scope.is_empty() {
        return Ok(());
    }
    let target = match tool {
        "NotebookEdit" => input["notebook_path"].as_str().unwrap_or_default(),
        _ => input["file_path"].as_str().unwrap_or_default(),
    };
    let subject = scope_subject(surface.root, target);
    let Some(rule) = surface
        .scope
        .iter()
        .find(|rule| rule.paths.iter().any(|glob| glob_matches(glob, subject)))
    else {
        return Ok(());
    };
    let matched = rule.paths.join("`, `");
    // The guarded arm is the granularity one and it comes before the catch-all, so a
    // `partial-only` rule refuses the two tools that replace a whole file and admits `Edit`.
    match rule.write {
        WriteScope::Denied => Err(format!(
            "`{tool}` cannot write `{subject}`: this step's declared write scope answers `denied` \
             for it, on the rule `{matched}`. This step may write {}. Everything else is changed \
             through the verb that owns it — a planning artifact through `protocol plan \
             artifact` (`new`, `body`, `move`, `relate`), which is why a file writer is denied \
             there.",
            writable(surface.scope)
        )),
        WriteScope::PartialOnly if tool == "Write" || tool == "NotebookEdit" => Err(format!(
            "`{tool}` replaces the whole of `{subject}`, and this step's declared write scope \
             answers `partial-only` for it, on the rule `{matched}`: part of a file may be \
             changed, a whole file may never be replaced. Make the change with `Edit`."
        )),
        WriteScope::Allowed | WriteScope::PartialOnly => Ok(()),
    }
}


/// The globs a scope leaves writable, for a refusal to name.
///
/// A refusal that says only *no* costs the next turn finding out where *yes* is, and the answer is
/// already in the declaration this refusal came from.
pub fn writable(scope: &[ScopeRule]) -> String {
    let allowed: Vec<&str> = scope
        .iter()
        .filter(|rule| rule.write != WriteScope::Denied)
        .flat_map(|rule| rule.paths.iter().map(String::as_str))
        .collect();
    if allowed.is_empty() {
        "nothing: every rule of its scope is `denied`".to_owned()
    } else {
        format!("`{}`", allowed.join("`, `"))
    }
}


/// The path a scope's globs are written against.
///
/// A step map writes `crates/**`, relative to the working tree. Claude Code sends the absolute
/// path it opened, so stripping the tree is what makes the two the same subject. A path that is
/// **not** under the tree is handed over unchanged rather than mangled: every validated scope ends
/// in a `**` catch-all, so an outsider still gets an answer instead of slipping through a
/// prefix that did not match.
pub fn scope_subject<'a>(root: &Path, target: &'a str) -> &'a str {
    let Some(root) = root.to_str() else {
        return target;
    };
    let root = root.trim_end_matches('/');
    match target.strip_prefix(root) {
        Some(rest) if rest.is_empty() => rest,
        Some(rest) => rest.strip_prefix('/').unwrap_or(target),
        None => target,
    }
}


/// Which side of a section the loop is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    /// Entering the section's first state.
    Enter,
    /// Leaving the section's last state.
    Leave,
}


/// What the engine said.
#[derive(Debug)]
pub enum Answer {
    /// The requested boundary is permitted.
    Proceed,
    /// The boundary is denied for this reason.
    Refuse(String),
}


/// Positions the engine and asks it.
///
/// **Enter** asks whether the engine may be in the section's first state: it is there already —
/// the driven run's cursor is on it — or a transition into it is permitted now. Otherwise the
/// refusal is that transition's unmet requirements, in the engine's words, or the plain fact that
/// the workflow declares no move from where the engine is to where the flow wants to go.
///
/// **Leave** asks whether the engine may leave the section's last state: `transition` on a copy of
/// the execution moves or completes, and the copy is dropped. `Blocked` is the refusal, reasons and
/// all. Nothing is persisted: a governor answers a question; the run that walks is the loop's.
pub fn answer(args: &TransitionArgs, path: &str, moment: Moment) -> Result<Answer> {
    let (engine, mut execution, positioned_by_run) = position(args, path, moment)?;
    let wanted = flow_state(path, moment);
    let workflow = &execution.plan().workflow;
    let Some(wanted) = wanted else {
        // The root is the flow's own container, and the loop asks about it first (design 0003
        // § 3: "the root is a group and is gated like one"). Entering it is entering the
        // workflow's initial state, which is where a fresh engine already is: proceed. Leaving it
        // is the whole walk done, and with a run to stand on the engine says whether the task may
        // move on from where the cursor is; without one there is nothing to position on, and the
        // answer is that nothing is owed *here* — the sections inside were governed one by one.
        // The first paid walk (2026-08-29, `native-eval.IudJuv`) was refused at `enter root` and
        // ran nothing, which is how this branch came to exist.
        return Ok(match (moment, positioned_by_run) {
            (Moment::Enter, _) | (Moment::Leave, false) => Answer::Proceed,
            (Moment::Leave, true) => match engine.transition(&mut execution) {
                Ok(TransitionResult::Moved { .. } | TransitionResult::Completed { .. }) => {
                    Answer::Proceed
                }
                Ok(TransitionResult::Blocked { state, reasons }) => {
                    Answer::Refuse(format!("{state}: {}", reasons.join("; ")))
                }
                Err(error) => Answer::Refuse(format!("the engine refused: {error}")),
            },
        });
    };
    if !workflow.states.contains_key(&wanted) {
        return Ok(Answer::Refuse(format!(
            "the flow path `{path}` names `{wanted}`, which is not a state of workflow `{}`",
            workflow.id
        )));
    }

    match moment {
        Moment::Enter => {
            if execution.state_id() == &wanted {
                return Ok(Answer::Proceed);
            }
            if !positioned_by_run {
                // Without a run there is no "where the engine is" but the state the path names,
                // which it is already on.
                return Ok(Answer::Proceed);
            }
            let evaluation = engine.evaluate(&execution);
            match evaluation
                .transitions
                .iter()
                .find(|transition| transition.to == wanted)
            {
                Some(transition) if transition.permitted => Ok(Answer::Proceed),
                Some(transition) => Ok(Answer::Refuse(format!(
                    "{} -> {}: {}",
                    evaluation.state,
                    wanted,
                    transition.unmet().join("; ")
                ))),
                None => Ok(Answer::Refuse(format!(
                    "the run is in `{}` and workflow `{}` declares no move from there to `{wanted}`",
                    evaluation.state, workflow.id
                ))),
            }
        }
        Moment::Leave => {
            if execution.state_id() != &wanted {
                return Ok(Answer::Refuse(format!(
                    "the flow is leaving `{wanted}` and the run's cursor is in `{}`: the two \
                     disagree about where the work is, and a governor does not guess",
                    execution.state_id()
                )));
            }
            match engine.transition(&mut execution) {
                Ok(TransitionResult::Moved { .. } | TransitionResult::Completed { .. }) => {
                    Ok(Answer::Proceed)
                }
                Ok(TransitionResult::Blocked { state, reasons }) => Ok(Answer::Refuse(format!(
                    "{state}: {}",
                    if reasons.is_empty() {
                        "nothing may move yet".to_owned()
                    } else {
                        reasons.join("; ")
                    }
                ))),
                Err(error) => Ok(Answer::Refuse(format!("the engine refused: {error}"))),
            }
        }
    }
}


/// The state a flow node path stands for at this moment.
///
/// `protocol workflow flow` emits every state as a group named for it and a retreat as a group
/// `<first>-to-<last>` (or `<state>-again` for a one-state retreat) holding them; the root is
/// `root`. The loop asks at group boundaries only, so the leaf of a path here is a state, a
/// retreat or the root. Entering a retreat is entering its first state, leaving it is leaving its
/// last — the same answers the states inside give one by one.
pub fn flow_state(path: &str, moment: Moment) -> Option<StateId> {
    let leaf = path.rsplit('.').next().unwrap_or(path);
    if leaf.is_empty() || leaf == "root" {
        return None;
    }
    let name = if let Some(state) = leaf.strip_suffix("-again") {
        state
    } else if let Some((first, last)) = leaf.split_once("-to-") {
        match moment {
            Moment::Enter => first,
            Moment::Leave => last,
        }
    } else {
        leaf
    };
    name.parse().ok()
}


/// The engine and an execution to ask it about.
///
/// With `--run`, the run's snapshot over the store as it is now — the same restore the driver does
/// at the top of every iteration. Without it, a fresh execution walked to the state the path names,
/// over the same store. The `bool` says which, because the two answer `enter` differently.
pub fn position(
    args: &TransitionArgs,
    path: &str,
    moment: Moment,
) -> Result<(Engine, aep_engine::Execution, bool)> {
    let project = match &args.location.project {
        Some(named) => named.clone(),
        None => discover_project()?,
    };
    let (location, snapshot) = match &args.run {
        Some(named) => {
            let runs = runs_directory(&project)?;
            let run_id: RunId = named.parse().map_err(|error| anyhow::anyhow!("{error}"))?;
            let directory = RunDirectory::at(run_path(&runs, &run_id));
            if !directory.path().is_dir() {
                bail!("no run {run_id} in {}", runs.display());
            }
            let launch = Launch::read(directory.path());
            let snapshot = directory
                .read_snapshot()
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            (
                args.location.remembering(launch.as_ref(), &project),
                Some(snapshot),
            )
        }
        None => (args.location.remembering(None, &project), None),
    };
    let inputs = location.inputs()?;
    let report = aep_driver::run::PlanSource::load(&inputs.store);
    if !report.is_clean() {
        bail!(
            "the store is not readable: {}",
            report
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
    let graph = report
        .graph_in_workspace(aep_driver::run::PlanSource::declared_members(&inputs.store))
        .map_err(|errors| {
            anyhow::anyhow!(
                "{}",
                errors
                    .as_slice()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;
    let engine = Engine::new(inputs.registry.clone()).with_ess_conformance_v2_reader(std::sync::Arc::new(aep_ess_evidence::CountStageReader)).with_ess_conformance_coverage_reader(std::sync::Arc::new(aep_ess_evidence::CoverageReader));
    if let Some(snapshot) = snapshot {
        let execution = engine
            .restore(inputs.task.clone(), graph, snapshot)
            .map_err(|error| anyhow::anyhow!("{error}"))
            .context("restoring the run's execution")?;
        return Ok((engine, execution, true));
    }
    let mut execution = engine
        .initialize_with_artifacts(inputs.task.clone(), graph)
        .map_err(|error| anyhow::anyhow!("{error}"))
        .context("initialising an execution")?;
    if let Some(state) = flow_state(path, moment) {
        if execution.plan().workflow.states.contains_key(&state) && execution.state_id() != &state {
            execution
                .enter_state(state)
                .map_err(|error| anyhow::anyhow!("{error}"))?;
        }
    }
    Ok((engine, execution, false))
}


/// The rule itself, over a path and the strings the caller has already extracted.
///
/// # Why this half is code and the other half is a declaration
///
/// *No whole-file replacement under the store* is a question about a **path** and a
/// **granularity**, and both are things a scope can say — so it says them, in
/// `drivers/development/default.yaml`, where a person reading the workflow can find the rule that
/// governs the run. [`declared_write`] enforces it on this arm and `--write-scope` enforces it on
/// the native one, from that one declaration.
///
/// *Text crossing the closing `---`* is a question about the **content** of an edit: which part of
/// a file this is, not which file. No scope grammar expresses it — the eval corpus says the same
/// thing about a transcript, that this half is "not transcript-decidable from a path"
/// (`conformance/eval/development-honest/expectations.trace.yaml`) — so it stays here, and the
/// store's directory appears below as the **address of the documents whose frontmatter has a
/// machine owner**, never as a rule about where a step may write.
///
/// # Split out because the two arms spell the argument differently and the rule does not
///
/// The vendor's `Edit` carries `old_string`/`new_string`; the native loop's `file_edit` carries
/// `old`/`new`, and its `NotebookEdit` does not exist at all. A shared rule that reached into a
/// `Value` for a key name was a rule that silently allowed everything on whichever arm it guessed
/// wrong about — which is exactly what happened once: the hook read `file_path`, the loop sent
/// `path`, the target came back empty, and `revision: 99` was written to a planning document by a
/// hook that reported success. The extraction belongs to whoever knows the call shape; the
/// decision belongs here.
pub fn store_integrity_at(target: &str, edits: &[(&str, &str)]) -> Result<(), String> {
    if !target.contains(".engineering/planning/") {
        return Ok(());
    }
    for (field, text) in edits {
        if text.lines().any(|line| line.trim() == "---") {
            return Err(format!(
                "the edit's `{field}` crosses the `---` frontmatter fence of {target}. Edit only \
                 below the closing fence; the frontmatter is the CLI's — `protocol plan \
                 artifact move` for status, `artifact relate` for relations, `artifact new` for \
                 creation, and `artifact body <id> --from <path|->` for the prose underneath."
            ));
        }
    }
    Ok(())
}


/// The per-state shell surface: one simple invocation of `protocol artifact …` or
/// `protocol trace …`, exactly what the retired `driven-surface.sh` held the grant to.
///
/// The surface lives here and not in any document the run can reach, deliberately: a run that
/// could name its own allowed surface could widen it. Pattern-based and best-effort, as § 4.8
/// says — granting `command.execute` grants a superset of the shell's reach, and this narrows it.
pub fn driven_surface(context: &StepContext<'_>, input: &serde_json::Value) -> Result<(), String> {
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
    let mut verb = words.next().unwrap_or_default();
    // The CLI's first level is the four area names, and every verb under them keeps its flat
    // spelling as a hidden alias — so `protocol plan artifact new` and `protocol artifact new` are
    // one command and this surface has to admit both. Skipping the area word rather than listing
    // the grouped spellings keeps the rule about *which verb*, which is what it was always about:
    // `protocol drive run` is still refused, because after the area word the verb is `run`.
    if crate::AREAS.contains(&verb) {
        verb = words.next().unwrap_or_default();
    }
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
            "`protocol {}` is outside the surface this state admits: `protocol plan artifact …` \
             and `protocol observe trace …`, by either spelling. Driving a run from inside a \
             driven step, or moving the store's own governing documents, is not this step's \
             business.",
            if verb.is_empty() { "(no verb)" } else { verb }
        ));
    }
    Ok(())
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
pub fn has_llm_steps(map: &StepMap) -> bool {
    llm_step_count(map) > 0
}


/// How many, for a refusal that says how much is at stake.
pub fn llm_step_count(map: &StepMap) -> usize {
    map.states
        .values()
        .flat_map(|state| state.steps.iter())
        .filter(|step| matches!(step, Step::Llm(_)))
        .count()
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
pub struct Launch {
    /// The task document, as given.
    pub task: Option<PathBuf>,
    /// The task document, as the run **resolved** it: absolute, and filled in when discovery
    /// rather than a flag found it.
    ///
    /// A second field beside the first rather than a replacement for it, because they answer
    /// different questions: `task` is what to pass to a resume, and this is what `{task}` expanded
    /// to. A resume that recomputed the second would expand a different path whenever it ran from
    /// a different directory than the launch did — a driven step named one document and the resume
    /// of the same run named another — so it is remembered, exactly as `--map` and the b10x
    /// options are.
    ///
    /// `#[serde(default)]`, so a run started before this existed resumes as it did: `None` here
    /// means the resume resolves it itself.
    #[serde(default)]
    pub task_document: Option<PathBuf>,
    /// The map, as given — a path or an id, whichever the caller used.
    pub map: Option<String>,
    /// The project directory the run was started against.
    pub project: Option<PathBuf>,
    /// The document tree.
    pub root: Option<PathBuf>,
    /// Whether the run may stop at an approval.
    pub pause_on_approval: bool,
    /// The one non-human actor whose recorded approval the run admits at an `operator` step.
    ///
    /// `#[serde(default)]`, so a run started before this existed resumes admitting a person only,
    /// which is what it admitted when it started.
    #[serde(default)]
    pub approver: Option<ActorRef>,
    /// The plugin directories the sessions loaded.
    pub plugin_dirs: Vec<PathBuf>,
    /// Host-owned launch fields, retained verbatim when a neutral reader updates a run.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}


impl Launch {
    /// `<run>/launch.json`.
    pub fn path(run_directory: &Path) -> PathBuf {
        run_directory.join("launch.json")
    }

    /// Writes it, and says nothing if it cannot: a run that walks and does not record how it was
    /// launched is worse off at resume time and is not wrong now.
    pub fn write(&self, run_directory: &Path) {
        let _ = self.write_required(run_directory);
    }

    /// Atomically writes a launch record when a caller cannot safely proceed without it.
    pub fn write_required(&self, run_directory: &Path) -> Result<()> {
        let path = Self::path(run_directory);
        let next = path.with_extension("json.next");
        let text = serde_json::to_string_pretty(self).context("rendering the launch record")?;
        fs::write(&next, text + "\n")
            .with_context(|| format!("writing the launch record at {}", next.display()))?;
        fs::rename(&next, &path).with_context(|| {
            format!(
                "publishing the launch record from {} to {}",
                next.display(),
                path.display()
            )
        })
    }

    /// Reads it, or `None` for a run started before this existed.
    pub fn read(run_directory: &Path) -> Option<Self> {
        let text = fs::read_to_string(Self::path(run_directory)).ok()?;
        serde_json::from_str(&text).ok()
    }
}


/// What the driver adds to the environment of every process it starts for a step.
///
/// **One variable: who the step is, when it writes to the planning store.** `command_actor()`
/// stamped `human:<$USER>` on every `artifact new`, `move`, `body`, `relate` and `evidence`,
/// whoever made it — so a driven session that ran `protocol artifact move <spec> approved` was
/// journalled as the operator's own move and the store could not tell an agent's write from a
/// person's. It is [`aep_driver::attest::session_actor`] and not a second spelling of
/// `agent:<execution>`, because the same value is what
/// [`aep_driver::attest::admit`] refuses an approval from: a run that wrote under one name and
/// was refused under another could approve its own work.
///
/// Empty when the execution id cannot be spelled as an actor name, and the session then writes as
/// the operator did before. Declaring nothing is honest; declaring a mangled name is not.
///
/// # What this reaches, and what it does not
///
/// A `command` step is spawned by this process, so it inherits the variable and a `protocol`
/// invocation in a step map is attributed to the run. An **`llm` step's session is not**:
/// `metaharness run` is spawned here and receives it, but metaharness constructs its child's
/// environment rather than inheriting one — `env_clear()` and a fixed allowlist (`INHERITED_KEYS`,
/// seven names, in `metaharness-claude`'s launch; `PATH` plus a credential in the `b10x` adapter's)
/// — and it publishes no flag that admits another variable. So the model's own
/// `protocol artifact move` is still journalled as `human:<$USER>`, and closing that is a flag on
/// that side of the boundary, not an edit on this one (`story:the-store-knows-who-wrote-it`,
/// § *Out of Scope*).
pub fn session_env(execution: &ExecutionId) -> Vec<(String, String)> {
    aep_driver::attest::session_actor(execution)
        .map(|actor| (crate::planning::ACTOR_ENV.to_owned(), actor.to_string()))
        .into_iter()
        .collect()
}

/// Derives one stable ordinary-mutation identity from the persisted driven action.
///
/// The attempt number is intentionally absent: retrying the same state/index must reach the same
/// reservation and provider idempotency keys. A different execution, state, or action index gets
/// a different identity before the subprocess starts.
fn driver_command_identity(context: &StepContext<'_>) -> String {
    let index = u64::try_from(context.index)
        .unwrap_or(u64::MAX)
        .to_be_bytes()
        .to_vec();
    let digest = aep_contract::migration::digest_parts_v1(
        "aep.planning-driver-action/1",
        &[
            context.execution.as_str().as_bytes().to_vec(),
            context.state.as_str().as_bytes().to_vec(),
            index,
        ],
    )
    .expect("bounded driver action identity inputs fit canonical framing");
    format!(
        "driver-{}",
        digest.as_wire().trim_start_matches("sha256:")
    )
}


/// Refuses a run whose `command` steps say `protocol` when this driver cannot guarantee they get it.
///
/// The third pre-flight, and it answers a question the other two do not.
/// The execution host separately checks the **session's** `PATH` — the one Metaharness
/// constructs for an `llm` step. A `command` step is spawned by the *driver*, with the *driver's*
/// environment, and that difference is exactly why run `W4-3/1`'s failure got past a guard that
/// looked like it covered this: two `PATH`s, one of them checked.
///
/// [`resolve_program`] normally removes the question — a step that says `protocol` gets
/// `current_exe()`. This fires only on the branch where that is unavailable, because then the
/// fallback is the very lookup that produced the defect, and whether it is safe is decidable here:
/// if the `PATH` `protocol` *is* this build, nothing is at stake and the run proceeds.
pub fn protocol_command_preflight(map: &StepMap) -> Option<String> {
    let steps = protocol_command_steps(map);
    if steps == 0 || std::env::current_exe().is_ok() {
        return None;
    }
    protocol_command_refusal(steps, protocol_version_on_path().as_deref())
}


/// How many `command` steps of the map invoke this CLI, by the same file-name rule the executor uses.
pub fn protocol_command_steps(map: &StepMap) -> usize {
    map.states
        .values()
        .flat_map(|state| state.steps.iter())
        .filter(|step| match step {
            Step::Command(command) => Path::new(command.program())
                .file_name()
                .is_some_and(|name| name == PROTOCOL_BINARY),
            _ => false,
        })
        .count()
}


/// The refusal itself, given what this process could learn about the `protocol` it would fall back to.
///
/// Separated from the two lookups because neither is reachable from a test: `current_exe()` does
/// not fail on a machine a test suite runs on, so the *message* — which is the whole product of a
/// pre-flight — would otherwise be checked by nobody. `installed` is `None` when there is no
/// `protocol` on the driver's `PATH` at all, which is the same finding with no version to quote.
pub fn protocol_command_refusal(steps: usize, installed: Option<&str>) -> Option<String> {
    let ours = env!("CARGO_PKG_VERSION");
    let disagreement = match installed {
        // Agreement is not a finding: the fallback would spawn this very build.
        Some(version) if version == ours => return None,
        Some(version) => format!("that one reports `{version}` and this build is `{ours}`"),
        None => format!("there is no `protocol` on that `PATH` at all, and this build is `{ours}`"),
    };
    Some(format!(
        "this map has {steps} `command` step(s) that invoke `protocol`, and this driver cannot \n\
         name its own binary: `current_exe()` is unavailable here, so such a step falls back to \n\
         the first `protocol` on the driver's `PATH` — and {disagreement}.\n\
         \n\
         A `command` step is spawned **by the driver, with the driver's environment**, so the \n\
         `PATH` that decides is the shell you typed `protocol drive` in — *not* the session \n\
         `PATH` metaharness constructs for an `llm` step. \n\
         `cargo install --path crates/edge/aep-cli --root ~/.local` is the fix for that other \n\
         `PATH`, which looks in `$HOME/.local/bin`; it fixes this one only if that directory \n\
         comes first in your own.\n\
         \n\
         A step that runs a binary older than the map executing it writes nothing and is recorded \n\
         as *no verdict*, with the cause invisible in the message: run `W4-3/1` spent a step's \n\
         whole retry budget on exactly that on 2026-08-28, against a `protocol` four releases \n\
         stale.\n\
         \n\
         Put this build first on the `PATH` you drive from:\n\
         \n\
             cargo install --path crates/edge/aep-cli --root ~/.local\n\
             export PATH=\"$HOME/.local/bin:$PATH\"\n\
         \n\
         or drive a map whose `command` steps name no `protocol`."
    ))
}


/// What the first `protocol` on the driver's own `PATH` says it is, when there is one.
///
/// A spawn, where [`on_path`] is deliberately only a lookup — because the question is different.
/// *Does a file exist* is decidable without running it; *which build is it* is not, and
/// `--version` is the one question this CLI answers by printing and exiting. It is asked only on
/// the branch where this process could not name its own binary, which is the branch where the
/// answer decides whether the run can be trusted at all.
pub fn protocol_version_on_path() -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    let candidate = std::env::split_paths(&paths)
        .map(|directory| directory.join(PROTOCOL_BINARY))
        .find(|candidate| candidate.is_file())?;
    let output = Process::new(candidate).arg("--version").output().ok()?;
    let printed = String::from_utf8_lossy(&output.stdout);
    // `clap`'s `--version` is `protocol <semver>`; the last word is the number, and a line that
    // has no words at all is a binary that answered nothing rather than a version.
    printed
        .lines()
        .next()?
        .split_whitespace()
        .next_back()
        .map(ToOwned::to_owned)
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
pub fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}


/// One write scope as `--write-scope`'s grammar spells it.
///
/// Total and written out rather than taken off the type's `Serialize`, so a rule the map gains
/// later fails to compile here instead of reaching an argv as an empty word. It coincides with the
/// map's own kebab-case wire form and that is not an accident worth relying on silently —
/// `the_write_scope_words_are_the_ones_the_step_map_is_written_in` asserts the two agree.
///
/// Shared because the flow projection renders the same scope into a projected flow node, and a
/// projection that spelled a scope differently from the run it describes would be a document that
/// looks like the thing it is not.
pub fn write_scope_word(scope: WriteScope) -> &'static str {
    match scope {
        WriteScope::Allowed => "allowed",
        WriteScope::PartialOnly => "partial-only",
        WriteScope::Denied => "denied",
    }
}


/// The instant the driver just observed something, from the wall clock.
///
/// The driver runs the program and reads its exit status, so *now* is the truthful observation
/// time — this is the one case where the two times an evidence record carries legitimately
/// coincide, and it is stated rather than assumed. It lives in `aep-cli` and not in a pure
/// crate for the reason the store lock does: reading ambient OS state is this binary's job.
pub fn observed_now() -> ObservedAt {
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
pub fn read_record(
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
    let inputs = match aep_schema::parse::evidence_list_with_readers(&text, Some(&origin), &crate::ess_readers()) {
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
pub fn mint(
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
pub fn tool_of(verifier: &Verifier) -> Option<ToolRef> {
    match verifier {
        Verifier::ExternalTool(tool) => Some(tool.clone()),
        _ => None,
    }
}


/// Resolved invocation paths supplied to an execution host.
pub struct ExecutorContext {
    /// Project checkout.
    pub working_directory: PathBuf,
    /// Existing run directory.
    pub run_directory: PathBuf,
    /// Explicit plugin inputs.
    pub plugin_dirs: Vec<PathBuf>,
    /// Resolved workflow identity.
    pub workflow_id: String,
    /// Workflow major version.
    pub workflow_version: String,
}

/// Host preparation performs no run mutation or model launch.
pub trait ExecutionHost {
    /// The continuation command for a run this host starts or resumes.
    fn resume_command(&self) -> &'static str {
        "protocol drive resume"
    }

    /// Validate this invocation before the lock or run id exists.
    fn prepare(&self, inputs: &Inputs, previous: Option<&Launch>, budget: Option<&str>, charge: Option<&str>) -> Result<PreparedExecution>;
}

/// Factory called after the neutral launch record has been made durable.
pub type ExecutorFactory = Box<dyn FnOnce(&ExecutorContext, bool) -> Result<Box<dyn LlmStepExecutor>>>;

/// Validated host configuration and its deferred executor.
pub struct PreparedExecution {
    /// Host-owned fields in the existing launch record.
    pub launch_fields: std::collections::BTreeMap<String, serde_json::Value>,
    /// Exact planning executable selected by an external host.
    pub protocol_binary: Option<PathBuf>,
    /// Construct the executor after the run directory exists.
    pub executor: ExecutorFactory,
}

/// AEP's standalone command never launches an external model adapter.
pub struct CommandOnlyHost;
impl ExecutionHost for CommandOnlyHost {
    fn prepare(&self, inputs: &Inputs, _: Option<&Launch>, _: Option<&str>, _: Option<&str>) -> Result<PreparedExecution> {
        if has_llm_steps(&inputs.map) {
            bail!("this map contains model-backed steps; use `metaharness aep drive run` or `metaharness aep drive resume`; AEP allocated no run and launched nothing");
        }
        Ok(PreparedExecution {
            launch_fields: std::collections::BTreeMap::default(),
            protocol_binary: None,
            executor: Box::new(|_, _| Ok(Box::new(NoModelExecutor))),
        })
    }
}
struct NoModelExecutor;
impl LlmStepExecutor for NoModelExecutor {
    fn run_llm(&mut self, _: &LlmStep, _: &StepContext<'_>, _: StepAuthorizer<'_>) -> StepOutcome {
        StepOutcome::NoVerdict { reason: "no model executor was supplied".to_owned() }
    }
}

struct CliExecutors {
    working_directory: PathBuf,
    run_directory: PathBuf,
    approver: Option<ActorRef>,
    protocol_binary: Option<PathBuf>,
    llm: Box<dyn LlmStepExecutor>,
}
impl LlmStepExecutor for CliExecutors {
    fn run_llm(&mut self, step: &LlmStep, context: &StepContext<'_>, authorize: StepAuthorizer<'_>) -> StepOutcome {
        self.llm.run_llm(step, context, authorize)
    }
}
