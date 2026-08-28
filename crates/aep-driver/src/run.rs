//! The loop: seven engine calls in order, and outside the engine only what the answers permit.
//!
//! Per iteration, in this order:
//!
//! 1. **the store** — `MarkdownStore::load()`, checking `is_clean()` **before** `graph()` (F7);
//! 2. **restore-or-init** — the freshly built graph goes to `Engine::restore`, or to
//!    `initialize_with_artifacts` on the first iteration of a new run;
//! 3. **evaluate** — the engine's picture of what is owed and what may move;
//! 4. **route** — [`crate::route::next_step`] says run a step, transition, or stop;
//! 5. **persist** — the snapshot and the cursor, after every step.
//!
//! There is one thing the loop does with the engine that it does not *call*: it **lends** it. An
//! `llm` step is handed `Engine::authorize` over the live execution — a
//! [`StepAuthorizer`](crate::executor::StepAuthorizer) — because a model's tool call is decided
//! while the step runs, and the engine's record of that decision has to be written then rather
//! than reconstructed from a log afterwards.
//!
//! # D2: the graph is rebuilt every iteration, and nothing is cached
//!
//! The rebuild **is** the store's integrity check, which is what buys the cost: a full read and
//! parse of every planning document plus a full plan re-resolution, per iteration. Both are pure
//! CPU over local files with no clock and no network, and both are linear. A cache is refused for
//! the reason an index file is: a cached membership list is a second copy of the membership list,
//! and a second copy is a second thing that can disagree with the first. A `command` step can
//! create an artifact, so rebuilding once per *state* would evaluate the next step of that state
//! against a store one write behind.
//!
//! The asymmetry with the registry is chosen rather than accidental (F8): the **registry** is
//! loaded once per invocation and the **store** is rebuilt per iteration, so a mid-run edit to
//! `workflows/` is not picked up while a mid-run edit to the planning store is. D1's cursor pins the
//! workflow for the life of the run precisely so a governing document cannot move under it.
//!
//! # A broken store stops the run, and it is not `Blocked`
//!
//! `StoreReport::graph()` returns `Ok` for a store that has quietly lost a document: a file that
//! failed to parse never reaches the graph, it lands in `report.failures`. That is right for
//! *reading* — a listing of nine artifacts beats a refusal because the tenth file has a typo — and
//! wrong for gating, because `artifact.story.count` then drops by one and a **completion gate** is
//! evaluated against a fact base that shrank because of a typo. So `is_clean()` is consulted first,
//! the store's own failures go into the report verbatim, and the status is
//! [`RunStatus::StoreBroken`] — never `Blocked`, which is the engine's word for *the protocol says
//! no*, and a store with a typo in it is not that (F7).
//!
//! # Two documents, two owners
//!
//! `snapshot.json` is the engine's and `cursor.json` is the driver's, written side by side after
//! every step. A driver that stored its cursor inside the engine's snapshot would be a driver that
//! had quietly forked the snapshot format. Each is written to a fixed temporary name and renamed
//! over its target, so a crash mid-write leaves the previous document intact; a fixed name is safe
//! because the store lock (D6) guarantees one writer.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use aep_backend_markdown::store::StoreReport;
use aep_backend_markdown::MarkdownStore;
use aep_domain::action::ActionRequest;
use aep_domain::artifact::ArtifactGraph;
use aep_domain::error::ValidationErrors;
use aep_domain::ids::{StateId, TaskId};
use aep_domain::task::Task;
use aep_driver_spec::cursor::{DriverCursor, RunId, RunStatus};
use aep_driver_spec::map::{Step, StepMap};
use aep_engine::evaluate::{Evaluation, Requirement};
use aep_engine::execution::Execution;
use aep_engine::policy::effective_policy;
use aep_engine::resolve::resolve;
use aep_engine::{
    Clock, CompletionExplanation, Engine, ProtocolEngine, ProtocolError, Snapshot, TransitionResult,
};

use crate::executor::{StepAttempt, StepContext, StepExecutors, StepOutcome};
use crate::route::{next_step, NextStep};
use crate::tool::tool_config;

/// The engine version a run is pinned to.
///
/// This crate's own package version, which is the workspace version `aep-engine` shares — the two
/// move together by construction, so there is no second number to keep in step. The cursor records
/// it because `Snapshot` carries `deny_unknown_fields`: a field a future engine adds makes an
/// *older* driver refuse a *newer* snapshot as a deserialization error, at the least informative
/// possible moment. One field turns that into *"this snapshot was written by engine X and this
/// driver links engine Y"* (review finding **F20**).
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the driver's snapshot is written to, inside the run directory.
const SNAPSHOT_FILE: &str = "snapshot.json";

/// What the driver's cursor is written to, inside the run directory.
const CURSOR_FILE: &str = "cursor.json";

/// The two ways out of a refused resume, named in the refusal.
const ROUTES_OUT: &str = "the routes out are `--restart`, which allocates a new run id and \
                          re-observes the evidence, or reverting the document that moved";

/// How a run is bounded and what it may do without a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverOptions {
    /// The blunt bound on the whole loop.
    ///
    /// A third bound beside the per-state visit budget and the per-step retry budget, and the least
    /// informative of the three: it stops a run that is making progress nobody wants as well as one
    /// that is wedged. It exists because the other two bound *a state* and *a step*, and a workflow
    /// with many states can still walk further than an operator meant to pay for.
    pub max_iterations: u32,
    /// Whether the run may stop at an approval instead of refusing to start.
    ///
    /// Opt-in because it changes what a green exit means: without it exit 0 means *finished*, with
    /// it exit 0 means *finished or waiting*, and a caller has to choose to be told that.
    pub pause_on_approval: bool,
    /// Whether there is nobody at the keyboard.
    pub headless: bool,
}

impl Default for DriverOptions {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            pause_on_approval: false,
            headless: true,
        }
    }
}

/// Why a run could not proceed.
#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    /// The filesystem refused.
    #[error("{}: {source}", path.display())]
    Io {
        /// What was being read or written.
        path: PathBuf,
        /// What the filesystem said.
        #[source]
        source: io::Error,
    },

    /// A run document could not be read as the record it claims to be.
    #[error("{}: {detail}", path.display())]
    Malformed {
        /// Which document.
        path: PathBuf,
        /// What is wrong with it.
        detail: String,
    },

    /// The planning store could not be trusted, and no run had started to record it against.
    #[error("the planning store cannot be trusted:\n{0}")]
    Store(String),

    /// The engine refused.
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// The driver refused, and the message names what to do instead.
    #[error("{0}")]
    Refused(String),

    /// A document did not validate — the plan would not resolve, or the map does not fit it.
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
}

/// One run's directory: a path, plus the two records that live in it.
///
/// Never allocated here — `protocol-cli` allocates it after taking the store lock, and never
/// deletes or reuses one. `--restart` allocates a new run id, because a run directory that could be
/// reused is a history that can be overwritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDirectory {
    path: PathBuf,
}

impl RunDirectory {
    /// The run directory at `path`.
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Where it is.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where the driver's cursor lives.
    pub fn cursor_path(&self) -> PathBuf {
        self.path.join(CURSOR_FILE)
    }

    /// Where the engine's snapshot lives.
    pub fn snapshot_path(&self) -> PathBuf {
        self.path.join(SNAPSHOT_FILE)
    }

    /// `true` when a run has already been persisted here.
    pub fn has_cursor(&self) -> bool {
        self.cursor_path().exists()
    }

    /// Reads the driver's cursor.
    pub fn read_cursor(&self) -> Result<DriverCursor, DriveError> {
        read_json(&self.cursor_path())
    }

    /// Reads the engine's snapshot.
    pub fn read_snapshot(&self) -> Result<Snapshot, DriveError> {
        read_json(&self.snapshot_path())
    }

    /// Writes both records, creating the directory if it is not there.
    ///
    /// Pretty-printed, because the first thing anybody does with a stopped run is read its cursor.
    pub fn persist(&self, snapshot: &Snapshot, cursor: &DriverCursor) -> Result<(), DriveError> {
        fs::create_dir_all(&self.path).map_err(|source| DriveError::Io {
            path: self.path.clone(),
            source,
        })?;
        write_json(&self.snapshot_path(), snapshot)?;
        write_json(&self.cursor_path(), cursor)
    }

    /// Which run this directory is, read off its own path.
    ///
    /// `.engineering/runs/<task>/<n>`, which is `RunId::segments` — two segments rather than one
    /// flattened name, so a task's runs sit together and no separator has to be escaped out of an
    /// identifier that may legally contain `/` itself. A directory that does not have that shape is
    /// refused rather than guessed at: the alternative is inventing a run id, and a run id that
    /// disagrees with its own directory is a record nobody can join back up.
    pub fn run_id(&self, task: &TaskId) -> Result<RunId, DriveError> {
        let refuse = |detail: String| {
            Err(DriveError::Refused(format!(
                "the run directory {} {detail}; a run directory is `<task>/<n>`, such as \
                 `.engineering/runs/{task}/1`",
                self.path.display()
            )))
        };
        let Some(ordinal) = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse::<u32>().ok())
        else {
            return refuse("does not end in a run number".to_owned());
        };
        let owner = self
            .path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str());
        if owner != Some(task.as_str()) {
            return refuse(format!(
                "sits under `{}` and this run is of task `{task}`",
                owner.unwrap_or("<nothing>")
            ));
        }
        RunId::new(task, ordinal).map_err(|error| DriveError::Refused(error.to_string()))
    }
}

/// What a run did, and why it stopped.
#[derive(Debug, Clone)]
pub struct RunReport {
    /// The cursor as it was last persisted.
    pub cursor: DriverCursor,
    /// Every move the engine made, in order.
    pub transitions: Vec<(StateId, StateId)>,
    /// How many step attempts ran — a step retried once is two.
    pub steps_run: u32,
    /// How many pieces of evidence were submitted.
    pub evidence_submitted: u32,
    /// The engine's own reasons, verbatim.
    ///
    /// Never summarised and never re-worded. The engine's sentence is the one the workflow author
    /// can act on; a driver's paraphrase of it is a second vocabulary for the same fact.
    pub reasons: Vec<String>,
    /// What completion is still owed, verbatim, when there was an execution to ask.
    pub explanation: Option<CompletionExplanation>,
    /// The driver's own lines — a budget spent, a step with no verdict, a store that stopped
    /// parsing.
    pub notes: Vec<String>,
}

impl RunReport {
    /// Where the run got to.
    pub fn status(&self) -> RunStatus {
        self.cursor.status
    }
}

/// Where the artifact graph is rebuilt from at the top of every iteration.
///
/// The driver reads a plan and decides nothing about where it is kept: a markdown store is one, and
/// `protocol drive` hands it whichever store the project's `project.yaml` names
/// (`story:store-selection-in-project-yaml`). One method, because the loop asks one question —
/// *what does the plan say now* — and `StoreReport` is the answer in every store's terms,
/// failures included, so a store that stopped reading stops the run the same way a broken file does.
pub trait PlanSource {
    /// The plan's documents, read now.
    fn load(&self) -> StoreReport;

    /// Where the plan is, for a note in the run's report.
    fn describe(&self) -> String;

    /// The workspace members this plan is allowed to point at.
    ///
    /// A relation into another repository is a **declared** edge when the workspace manifest names
    /// the member and a dangling one otherwise, and only the caller knows which — the driver reads
    /// no manifest. Defaulting to none keeps a bare store's behaviour: nothing is declared, so a
    /// crossing edge is dangling, which is what a store with no workspace beside it means.
    ///
    /// It exists because the two readers disagreed: `protocol artifact validate` called this
    /// repository's own store valid while `protocol drive` refused to start on it, both correct
    /// about a different question and only one of them told the truth about the store.
    fn declared_members(&self) -> Vec<aep_domain::workspace::MemberName> {
        Vec::new()
    }
}

impl PlanSource for MarkdownStore {
    fn load(&self) -> StoreReport {
        Self::load(self)
    }

    fn describe(&self) -> String {
        self.root().display().to_string()
    }
}

/// Starts a new run, or continues one whose cursor is on disk.
///
/// Continuing is not resuming: [`resume`] checks the three pins and refuses when any moved, and a
/// caller that means *pick this run up again* should call it. This one continues a run in the same
/// invocation's terms, which is what a fresh `drive` over a directory it just wrote wants.
pub fn drive<C, S, X>(
    engine: &Engine<C>,
    task: &Task,
    store: &S,
    map: &StepMap,
    run: &RunDirectory,
    executors: &mut X,
    options: &DriverOptions,
) -> Result<RunReport, DriveError>
where
    C: Clock,
    S: PlanSource + ?Sized,
    X: StepExecutors,
{
    let (cursor, snapshot) = if run.has_cursor() {
        (Some(run.read_cursor()?), Some(run.read_snapshot()?))
    } else {
        (None, None)
    };
    Session {
        engine,
        task,
        store,
        map,
        directory: run,
        options,
    }
    .run(executors, cursor, snapshot)
}

/// The same loop, resuming: reads the cursor and the snapshot, checks the three pins, and refuses
/// when any moved.
///
/// Fail closed, naming both values. `Execution::restore` checks only that the snapshot's task
/// matches the plan and that its *state name* still exists, so a workflow that renamed nothing and
/// rewrote every guard restores cleanly and silently re-governs the run. The cursor is what closes
/// that — which is why the test for this refusal also asserts `Engine::restore` *would* have
/// accepted the same snapshot.
pub fn resume<C, S, X>(
    engine: &Engine<C>,
    task: &Task,
    store: &S,
    map: &StepMap,
    run: &RunDirectory,
    executors: &mut X,
    options: &DriverOptions,
) -> Result<RunReport, DriveError>
where
    C: Clock,
    S: PlanSource + ?Sized,
    X: StepExecutors,
{
    let cursor = run.read_cursor()?;
    let snapshot = run.read_snapshot()?;
    let plan = resolve(task, engine.registry())?;
    let workflow = format!("{}/{}", plan.workflow.id, plan.workflow.version);
    if let Some(refusal) = cursor.resume_refusal(&workflow, &map.id, &map.digest(), ENGINE_VERSION)
    {
        return Err(DriveError::Refused(format!("{refusal}; {ROUTES_OUT}")));
    }
    Session {
        engine,
        task,
        store,
        map,
        directory: run,
        options,
    }
    .run(executors, Some(cursor), Some(snapshot))
}

/// Everything one call was given, so the loop can be a sequence of short steps.
struct Session<'a, C: Clock, S: PlanSource + ?Sized> {
    engine: &'a Engine<C>,
    task: &'a Task,
    store: &'a S,
    map: &'a StepMap,
    directory: &'a RunDirectory,
    options: &'a DriverOptions,
}

/// What a run has done so far, before it knows how it ends.
#[derive(Debug, Default)]
struct Progress {
    transitions: Vec<(StateId, StateId)>,
    steps_run: u32,
    evidence_submitted: u32,
    reasons: Vec<String>,
    notes: Vec<String>,
}

/// The loop's own mutable state: what has happened, and how badly the current step is going.
#[derive(Debug, Default)]
struct Tally {
    progress: Progress,
    streak: Streak,
    breaker: Breaker,
}

/// How many times each outside dependency has failed in this run, and which have been given up on.
///
/// Gap register `:78`. The retry budget and this are the two halves of the same question asked at
/// different scopes, and conflating them is the mistake the tier exists to avoid. Retry bounds
/// **this step keeps crashing** and resets when the run moves on. A breaker bounds **this dependency
/// keeps failing** and does *not* reset, across every step that names it — because the third call to
/// an API that has been down for ten minutes is not new information, it is ten more seconds of
/// timeout and one more indistinguishable line in the log.
///
/// Counted per dependency name rather than per step: two steps that call the same service share its
/// fate, and that sharing is the entire difference between a breaker and a retry budget.
#[derive(Debug, Default)]
struct Breaker {
    failures: std::collections::BTreeMap<String, u32>,
}

impl Breaker {
    /// Records one failure of a dependency, returning the total so far in this run.
    fn record(&mut self, dependency: &str) -> u32 {
        let count = self.failures.entry(dependency.to_owned()).or_default();
        *count += 1;
        *count
    }

    /// Whether this dependency has failed enough times that the run should stop attempting it.
    fn is_open(&self, dependency: &str, threshold: u32) -> bool {
        self.failures.get(dependency).copied().unwrap_or(0) >= threshold
    }
}

/// Consecutive attempts at one step that produced no verdict.
///
/// Separate from the cursor's attempt count, and the difference is D5's *"spent, not reset"* read
/// precisely. The **cursor** counts every attempt at `<state>#<index>` for the life of the run and
/// never resets, so *"green on the third try"* stays in the record. The **budget** bounds
/// *consecutive failures at this step*, so a step that succeeded on its second visit over a
/// back-edge is not refused for the attempt it spent on its first. A resume starts a fresh streak:
/// resuming is a person's deliberate act, and the cursor still holds every attempt that came before.
#[derive(Debug, Default)]
struct Streak {
    at: Option<(StateId, usize)>,
    count: u32,
}

impl Streak {
    /// Records one attempt with no verdict, returning how many in a row that is.
    fn record(&mut self, state: &StateId, index: usize) -> u32 {
        let here = (state.clone(), index);
        if self.at.as_ref() != Some(&here) {
            self.at = Some(here);
            self.count = 0;
        }
        self.count += 1;
        self.count
    }

    /// Forgets the streak, because the run moved on.
    fn clear(&mut self) {
        self.at = None;
        self.count = 0;
    }
}

impl<C: Clock, S: PlanSource + ?Sized> Session<'_, C, S> {
    /// The loop.
    fn run<X: StepExecutors>(
        &self,
        executors: &mut X,
        cursor: Option<DriverCursor>,
        snapshot: Option<Snapshot>,
    ) -> Result<RunReport, DriveError> {
        let run_id = self.directory.run_id(&self.task.id)?;
        let mut carried = cursor;
        let mut snapshot = snapshot;
        let mut tally = Tally::default();
        let mut checked = false;

        // Iterations spent in *this* call, beside the cursor's lifetime count.
        let mut here: u32 = 0;
        loop {
            let graph = match self.graph() {
                Ok(graph) => graph,
                Err(failures) => {
                    return self.stop_broken_store(
                        carried,
                        snapshot.as_ref(),
                        tally.progress,
                        failures,
                    )
                }
            };

            let mut execution = match snapshot.take() {
                Some(previous) => self.engine.restore(self.task.clone(), graph, previous)?,
                None => self
                    .engine
                    .initialize_with_artifacts(self.task.clone(), graph)?,
            };
            let mut cursor = match carried.take() {
                Some(existing) => existing,
                None => fresh_cursor(&run_id, &execution, self.map),
            };
            self.check_agreement(&cursor, &execution)?;
            if !checked {
                self.check_map(&execution)?;
                checked = true;
            }

            let evaluation = self.engine.evaluate(&execution);
            cursor.iterations += 1;
            // **This invocation's iterations, not the run's.** `cursor.iterations` is the run's
            // lifetime count and stays that way — it is what a reader asks *how far did this run
            // get*. The bound is a bound on **this call**, because that is what an operator typing
            // `--max-iterations 40` is asking for. Comparing the flag against the lifetime count
            // meant a resumed run had spent its budget before evaluating anything: `W4-2/1`'s
            // first resume returned `budget-exhausted`, `steps 0 run`, having done nothing, and the
            // operator had no way to see why from the flag they passed (F-W4.2-4).
            here += 1;
            if here > self.options.max_iterations {
                tally.progress.notes.push(format!(
                    "this call stopped after {} iteration(s), which is `max_iterations`; the run \
                     has spent {} in total and the state it was in is `{}`",
                    self.options.max_iterations, cursor.iterations, cursor.state
                ));
                tally.progress.reasons.extend(evaluation.blocking_reasons());
                return self.finish(
                    cursor,
                    &execution,
                    RunStatus::BudgetExhausted,
                    tally.progress,
                );
            }

            match next_step(self.map, &cursor) {
                NextStep::VisitBudgetExhausted { state, budget } => {
                    tally.progress.notes.push(format!(
                        "state `{state}` has been entered {} times and its visit budget is \
                         {budget}; the run is cycling rather than progressing",
                        cursor.visits_of(&state)
                    ));
                    tally.progress.reasons.extend(evaluation.blocking_reasons());
                    return self.finish(
                        cursor,
                        &execution,
                        RunStatus::BudgetExhausted,
                        tally.progress,
                    );
                }
                NextStep::Transition => {
                    if let Some(report) = self.advance(&mut execution, &mut cursor, &mut tally)? {
                        return Ok(report);
                    }
                }
                NextStep::Run { index } => {
                    if let Some(report) = self.step(
                        executors,
                        &mut execution,
                        &mut cursor,
                        &evaluation,
                        index,
                        &mut tally,
                    )? {
                        return Ok(report);
                    }
                }
            }

            let taken = execution.snapshot();
            self.directory.persist(&taken, &cursor)?;
            snapshot = Some(taken);
            carried = Some(cursor);
        }
    }

    /// Checks the step map against the plan the task resolved to, once per invocation.
    ///
    /// D1 phase two, run **before the first step executes**. The protocol in force comes from the
    /// **task**, which no document loader has seen, so this cannot be folded into load-time
    /// validation: a loader that guessed would let a map validate and then fail at the transition
    /// that needed the evidence — the most expensive possible moment, halfway through a run that
    /// has already spent a token budget.
    fn check_map(&self, execution: &Execution) -> Result<(), DriveError> {
        let plan = execution.plan();
        let errors = self.map.check_run(&plan.protocol, &plan.workflow);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DriveError::Validation(errors))
        }
    }

    /// Asks the engine to move, and folds the answer in.
    ///
    /// The routing is entirely the engine's: a failing suite is `False` and the **workflow** takes
    /// the back-edge. A driver that decided that for itself would be a second protocol
    /// implementation with none of the conformance suites.
    fn advance(
        &self,
        execution: &mut Execution,
        cursor: &mut DriverCursor,
        tally: &mut Tally,
    ) -> Result<Option<RunReport>, DriveError> {
        match self.engine.transition(execution)? {
            TransitionResult::Moved { from, to, .. } => {
                tally.progress.transitions.push((from, to.clone()));
                cursor.state = to.clone();
                cursor.step = 0;
                // Counted on **entry**, including re-entry over a back-edge: that is the cycle the
                // visit budget exists to bound.
                cursor.record_visit(&to);
                tally.streak.clear();
                Ok(None)
            }
            TransitionResult::Completed { .. } => {
                let progress = std::mem::take(&mut tally.progress);
                self.finish(cursor.clone(), execution, RunStatus::Completed, progress)
                    .map(Some)
            }
            TransitionResult::Blocked { reasons, .. } => {
                // Nothing moves and no step of this state is left to change that: a second attempt
                // would read the same store and reach the same answer, so looping would be polling.
                tally.progress.reasons.extend(reasons);
                let progress = std::mem::take(&mut tally.progress);
                self.finish(cursor.clone(), execution, RunStatus::Blocked, progress)
                    .map(Some)
            }
        }
    }

    /// Runs one step and folds its outcome in, returning a report when the run stops here.
    fn step<X: StepExecutors>(
        &self,
        executors: &mut X,
        execution: &mut Execution,
        cursor: &mut DriverCursor,
        evaluation: &Evaluation,
        index: usize,
        tally: &mut Tally,
    ) -> Result<Option<RunReport>, DriveError> {
        let state = cursor.state.clone();
        let step = &self.map.steps_for(&state)[index];
        let (label, budget, kind) = (step.label(), step.retry_budget(), step.kind());

        if matches!(step, Step::Operator(_))
            && self.options.headless
            && !self.options.pause_on_approval
        {
            return Err(DriveError::Refused(format!(
                "step {index} of `{state}` is an `operator` step and nobody is at the keyboard: \
                 {label}. Pass `--pause-on-approval` to run until the first approval and stop \
                 there, or run interactively. Reaching this at all means the plan owes an approval \
                 that the pre-flight scan did not see"
            )));
        }

        // The breaker is read *before* the attempt, because the whole point is not to make it. A
        // step skipped here costs nothing; a step attempted against a dependency that has been down
        // since the first state costs a timeout and produces one more line saying the same thing.
        if let Step::Command(command) = step {
            if let (Some(dependency), Some(threshold)) =
                (command.depends_on.as_deref(), command.circuit_breaker)
            {
                if tally.breaker.is_open(dependency, threshold) {
                    tally.progress.notes.push(format!(
                        "step {index} of `{state}` ({label}) was not attempted: `{dependency}` has \
                         failed {threshold} time(s) in this run and its circuit is open. Nothing \
                         was observed here, so nothing was recorded either"
                    ));
                    cursor.step += 1;
                    tally.streak.clear();
                    return Ok(None);
                }
            }
        }

        let attempt = cursor.record_attempt(&state, index);
        let outcome = self.execute(executors, execution, &state, index, evaluation, cursor);
        tally.progress.steps_run += 1;

        match outcome {
            StepOutcome::Observed(submission) => {
                self.engine.submit_evidence(execution, *submission)?;
                tally.progress.evidence_submitted += 1;
                cursor.step += 1;
                tally.streak.clear();
            }
            StepOutcome::Nothing => {
                cursor.step += 1;
                tally.streak.clear();
            }
            StepOutcome::NoVerdict { reason } => {
                // D5: nothing was observed, so nothing is submitted. Submitting a failing record
                // for a suite that never ran would fabricate an observation and send an agent to
                // fix code nobody ran.
                tally.progress.notes.push(format!(
                    "attempt {attempt} at step {index} of `{state}` ({label}) produced no verdict: \
                     {reason}"
                ));
                // A no-verdict is a failure *of the dependency* when the step named one, and that
                // count is what the next step naming it reads.
                if let Step::Command(command) = &self.map.steps_for(&state)[index] {
                    if let Some(dependency) = command.depends_on.as_deref() {
                        let failures = tally.breaker.record(dependency);
                        if command
                            .circuit_breaker
                            .is_some_and(|threshold| failures >= threshold)
                        {
                            tally.progress.notes.push(format!(
                                "`{dependency}` has now failed {failures} time(s); its circuit is \
                                 open and later steps that need it will not be attempted"
                            ));
                        }
                    }
                }
                let spent = tally.streak.record(&state, index);
                if spent > budget {
                    tally.progress.notes.push(format!(
                        "step {index} of `{state}` has spent its {kind} retry budget of {budget}, \
                         and no evidence was submitted for any attempt"
                    ));
                    tally.progress.reasons.extend(evaluation.blocking_reasons());
                    let progress = std::mem::take(&mut tally.progress);
                    return self
                        .finish(
                            cursor.clone(),
                            execution,
                            RunStatus::BudgetExhausted,
                            progress,
                        )
                        .map(Some);
                }
            }
            StepOutcome::Paused { reason } => {
                tally.progress.notes.push(format!(
                    "step {index} of `{state}` ({label}) is waiting for a person: {reason}"
                ));
                // The pause **is** this step's completion, so the cursor moves past it. The design
                // says a paused run "resumes" (§ 4.6), and a cursor left pointing at the step that
                // paused does not resume: it re-presents the same question to the same person on
                // every resume, and no map with an `operator` step before its last state could ever
                // move past one. What the person was asked for is decided by the guard on the way
                // out, not by asking again — a person who did nothing meets a `TransitionBlocked`
                // naming exactly what is still owed. A back-edge re-entry sets `step` to 0, so
                // re-entering the state asks again, which is the case where asking twice is right.
                cursor.step += 1;
                let progress = std::mem::take(&mut tally.progress);
                return self
                    .finish(
                        cursor.clone(),
                        execution,
                        RunStatus::AwaitingOperator,
                        progress,
                    )
                    .map(Some);
            }
        }
        Ok(None)
    }

    /// Builds the step's context and hands it to the executor for its kind.
    ///
    /// The execution is borrowed mutably for one reason: an `llm` step's tool calls are decided
    /// while the step runs, and `Engine::authorize` writes each decision into the execution's own
    /// event record. The loop is the only holder of both the engine and the live execution, so the
    /// authorizer is lent from here and lives no longer than the step.
    fn execute<X: StepExecutors>(
        &self,
        executors: &mut X,
        execution: &mut Execution,
        state: &StateId,
        index: usize,
        evaluation: &Evaluation,
        cursor: &DriverCursor,
    ) -> StepOutcome {
        // Read back rather than passed in: `record_attempt` has already counted this attempt, so
        // the cursor is the one place that knows which attempt every step of this state is on —
        // this one and the `llm` step below it.
        let attempt = cursor.attempts_at(state, index);
        // Per state, not per run: `effective_policy` grants the state's capabilities on top of the
        // plan's, so the tools that exist in `implement` are not the tools that exist in `review`.
        let tools = tool_config(&effective_policy(execution));
        let requirements: Vec<String> = evaluation
            .requirements
            .iter()
            .map(Requirement::line)
            .collect();
        // What guards the way *out*, which is a different question from what must hold here and
        // was never asked before: `Evaluation::requirements` is the in-state list, and the outgoing
        // guard lives on `Evaluation::transitions`. Unmet lines only — `unmet()` is empty for a
        // permitted transition — and lines the in-state list already carries are dropped, because
        // an obligation owed here is evaluated against every outgoing transition as well.
        let reaching: Vec<String> = evaluation
            .transitions
            .iter()
            .flat_map(|transition| {
                let to = transition.to.clone();
                transition
                    .unmet()
                    .into_iter()
                    .map(move |line| (to.clone(), line))
            })
            .filter(|(_, line)| !requirements.contains(line))
            .map(|(to, line)| format!("-> {to}: {line}"))
            .collect();
        // The `llm` step this one follows, so a command step can be about the session before it.
        // The nearest one and not the first: a state may run two, and a checker pointed at the
        // wrong transcript reports on a session that was asked for something else.
        let steps = self.map.steps_for(state);
        let preceding_llm = steps[..index]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, step)| matches!(step, Step::Llm(_)))
            .map(|(index, _)| StepAttempt {
                index,
                attempt: cursor.attempts_at(state, index),
            })
            // Zero attempts means the step was never run in this run or any it resumed from, so
            // there is nothing it wrote to be about.
            .filter(|preceding| preceding.attempt > 0);
        let context = StepContext {
            task: self.task,
            state,
            index,
            attempt,
            tools: &tools,
            run_directory: self.directory.path(),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm,
        };
        match &steps[index] {
            Step::Command(command) => executors.run_command(command, &context),
            // Only the `llm` step is lent the engine, and the asymmetry is the point: a `command`
            // step is the driver's own invocation of a program the map names, decided before the
            // run started by the pre-flight scan, while an `llm` step's calls are a model's and are
            // decided one at a time while it runs.
            Step::Llm(llm) => {
                let mut authorize =
                    |request: &ActionRequest| self.engine.authorize(execution, request);
                executors.run_llm(llm, &context, &mut authorize)
            }
            Step::Operator(operator) => executors.run_operator(operator, &context),
        }
    }

    /// The artifact graph, or the store's own failures verbatim.
    ///
    /// `is_clean()` first (F7): a file that did not parse is not in the graph to be wrong about.
    fn graph(&self) -> Result<ArtifactGraph, Vec<String>> {
        let report = self.store.load();
        if !report.is_clean() {
            return Err(report.failures.iter().map(ToString::to_string).collect());
        }
        report
            .graph_in_workspace(self.store.declared_members())
            .map_err(|errors| {
                errors
                    .as_slice()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
    }

    /// Refuses when the cursor and the snapshot disagree about where the run is.
    ///
    /// Two documents with two owners, so they can be edited apart. Which one is right is not
    /// guessable, and carrying on would run one state's steps against another state's evidence.
    fn check_agreement(
        &self,
        cursor: &DriverCursor,
        execution: &Execution,
    ) -> Result<(), DriveError> {
        if cursor.state == *execution.state_id() {
            return Ok(());
        }
        Err(DriveError::Refused(format!(
            "the cursor in {} says this run is in `{}` and the snapshot beside it says `{}`; \
             {ROUTES_OUT}",
            self.directory.path().display(),
            cursor.state,
            execution.state_id()
        )))
    }

    /// Persists the final state of a run and reports it.
    fn finish(
        &self,
        mut cursor: DriverCursor,
        execution: &Execution,
        status: RunStatus,
        progress: Progress,
    ) -> Result<RunReport, DriveError> {
        cursor.status = status;
        cursor.reasons.clone_from(&progress.reasons);
        self.directory.persist(&execution.snapshot(), &cursor)?;
        Ok(RunReport {
            cursor,
            transitions: progress.transitions,
            steps_run: progress.steps_run,
            evidence_submitted: progress.evidence_submitted,
            reasons: progress.reasons,
            explanation: Some(self.engine.explain_completion(execution)),
            notes: progress.notes,
        })
    }

    /// Stops on a store that cannot be trusted, leaving a run directory that resumes.
    ///
    /// The driver does not carry on with the last good graph — that is a run evaluating against a
    /// store that no longer exists. A run that had not started yet has no snapshot to persist, so
    /// there the failures come back as an error instead of a report.
    fn stop_broken_store(
        &self,
        cursor: Option<DriverCursor>,
        snapshot: Option<&Snapshot>,
        mut progress: Progress,
        failures: Vec<String>,
    ) -> Result<RunReport, DriveError> {
        let (Some(mut cursor), Some(snapshot)) = (cursor, snapshot) else {
            return Err(DriveError::Store(failures.join("\n")));
        };
        progress.notes.push(format!(
            "the planning store stopped parsing, so no evaluation could be trusted; {} file(s) \
             below {} are reported verbatim",
            failures.len(),
            self.store.describe()
        ));
        progress.reasons.extend(failures);
        cursor.status = RunStatus::StoreBroken;
        cursor.reasons.clone_from(&progress.reasons);
        self.directory.persist(snapshot, &cursor)?;
        Ok(RunReport {
            cursor,
            transitions: progress.transitions,
            steps_run: progress.steps_run,
            evidence_submitted: progress.evidence_submitted,
            reasons: progress.reasons,
            explanation: None,
            notes: progress.notes,
        })
    }
}

/// The cursor a run starts with, pinned to the three things a resume checks.
fn fresh_cursor(run: &RunId, execution: &Execution, map: &StepMap) -> DriverCursor {
    let plan = execution.plan();
    let initial = execution.state_id().clone();
    let mut cursor = DriverCursor {
        run: run.clone(),
        task: plan.task.id.clone(),
        execution: execution.id().clone(),
        workflow: format!("{}/{}", plan.workflow.id, plan.workflow.version),
        map: map.id.clone(),
        map_digest: map.digest(),
        engine_version: ENGINE_VERSION.to_owned(),
        state: initial.clone(),
        step: 0,
        visits: BTreeMap::new(),
        attempts: BTreeMap::new(),
        iterations: 0,
        status: RunStatus::Running,
        reasons: Vec::new(),
        took_lock_from: None,
    };
    // Counted on entry, and the initial state is an entry. A budget that only counted re-entries
    // would let a one-state workflow run forever.
    cursor.record_visit(&initial);
    cursor
}

/// Reads one JSON record.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, DriveError> {
    let text = fs::read_to_string(path).map_err(|source| DriveError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| DriveError::Malformed {
        path: path.to_path_buf(),
        detail: source.to_string(),
    })
}

/// Writes one JSON record, through a fixed temporary name.
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), DriveError> {
    let text = serde_json::to_string_pretty(value).map_err(|source| DriveError::Malformed {
        path: path.to_path_buf(),
        detail: source.to_string(),
    })?;
    let writing = path.with_extension("json.writing");
    fs::write(&writing, format!("{text}\n")).map_err(|source| DriveError::Io {
        path: writing.clone(),
        source,
    })?;
    fs::rename(&writing, path).map_err(|source| DriveError::Io {
        path: path.to_path_buf(),
        source,
    })
}
