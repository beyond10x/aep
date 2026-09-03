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
//! # Two documents, two owners, one committed generation
//!
//! `snapshot.json` is the engine's and `cursor.json` is the driver's. They remain separate owned
//! documents, but each persist writes both into an immutable, digest-sealed generation and then
//! atomically replaces `state.json`, the only authoritative pointer. A crash before the pointer
//! moves leaves the previous pair current. Top-level copies remain for older tooling and are never
//! read while the pointer exists.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use aep_backend_markdown::store::StoreReport;
use aep_backend_markdown::MarkdownStore;
use aep_domain::action::ActionRequest;
use aep_domain::artifact::ArtifactGraph;
use aep_domain::entity::ActorRef;
use aep_domain::error::ValidationErrors;
use aep_domain::evidence::{ApprovalDecision, Evidence};
use aep_domain::ids::{StateId, TaskId};
use aep_domain::task::Task;
use aep_driver_spec::cursor::{
    DriverCursor, InFlightAttempt, OperatorAnswer, OwedAnswer, RunId, RunStatus, StolenLock,
};
use aep_driver_spec::map::{Step, StepMap};
use aep_engine::evaluate::{Evaluation, Requirement};
use aep_engine::execution::Execution;
use aep_engine::policy::effective_policy;
use aep_engine::resolve::resolve;
use aep_engine::{
    Clock, CompletionExplanation, Engine, ProtocolEngine, ProtocolError, Snapshot, TransitionResult,
};
use sha2::{Digest, Sha256};

use crate::attest::{self, Admission};
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

/// The atomic pointer to the one snapshot/cursor generation readers may observe.
const CURRENT_GENERATION_FILE: &str = "state.json";

/// Immutable state generations live below this directory.
const GENERATIONS_DIRECTORY: &str = "generations";

/// How an operator resolves an attempt whose dispatch outlived its durable outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InFlightResolution {
    /// Dispatch the exact persisted attempt again.
    Retry(String),
    /// Record that the persisted attempt has no verdict, then continue under its retry policy.
    RecordNoVerdict,
}

/// The two ways out of a refused resume, named in the refusal.
const ROUTES_OUT: &str = "the routes out are `--restart`, which allocates a new run id and \
                          re-observes the evidence, or reverting the document that moved";

/// How a run is bounded, what it may do without a person, and where its task was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverOptions {
    /// The document the `task` argument of [`drive`] and [`resume`] was read from.
    ///
    /// What `{task}` expands to in a `command` step
    /// ([`StepContext::task_document`](crate::executor::StepContext::task_document)), and the only
    /// way a step map can name the document *this* run was started from rather than whichever one
    /// a verb discovers.
    ///
    /// Here rather than beside `task:` in the signature because it is not always knowable: a
    /// caller may parse a task out of bytes that were never a file, and an `Option` says so where
    /// a seventh parameter would make every such caller invent a path. `None` is *this run was not
    /// started from a document*, and a `{task}` in the map is then D5's `Unknown` rather than a
    /// guess.
    ///
    /// **Absolute**, by that field's contract: a `command` step runs in the project directory, and
    /// a relative path given on a command line is relative to wherever it was typed.
    pub task_document: Option<PathBuf>,
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
    /// The one non-human actor whose approval may answer an `operator` step of this run.
    ///
    /// Opt-in and named, exactly as `--pause-on-approval` is opt-in: without it only a person's
    /// approval counts, which is what every run did before the flag existed. It does not answer
    /// anything itself — the run still stops at the step, and the named actor records its
    /// approval while the run is stopped — it says whose answer the resume may count
    /// ([`crate::attest`]). Never this run's own actor, and the launch refuses the attempt.
    pub approver: Option<ActorRef>,
    /// The lock this call superseded on its way in, when it superseded one.
    ///
    /// An **input**, for the same reason this crate is handed a [`crate::lock::LockState`] rather
    /// than probing for one: `lock.json` belongs to `aep-cli`, along with the run directory it
    /// grants, and a driver that opened the lock file would be a driver reading ambient OS state
    /// (review finding **F19**). What arrives here is the three values the caller already read out
    /// of the lock it took.
    ///
    /// `None` means *this call took nothing from anybody*, which says nothing about what the run did
    /// on an earlier call: a theft already in the cursor is never cleared by a later clean
    /// acquisition, or the record would be erasable by resuming the run once more on a free lock.
    pub stolen_lock: Option<StolenLock>,
    /// Explicit resolution for the attempt a crashed invocation left in flight.
    pub in_flight_resolution: Option<InFlightResolution>,
}

impl Default for DriverOptions {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            pause_on_approval: false,
            headless: true,
            approver: None,
            task_document: None,
            stolen_lock: None,
            in_flight_resolution: None,
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

/// One run's directory: a path, plus the committed generations that live in it.
///
/// Never allocated here — `aep-cli` allocates it after taking the store lock, and never
/// deletes or reuses one. `--restart` allocates a new run id, because a run directory that could be
/// reused is a history that can be overwritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDirectory {
    path: PathBuf,
}

/// One immutable pair, sealed by the digests in this manifest.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GenerationManifest {
    format: String,
    generation: u64,
    snapshot_sha256: String,
    cursor_sha256: String,
}

/// The only mutable publication point for a run's state.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GenerationPointer {
    format: String,
    generation: u64,
    manifest_sha256: String,
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
        self.current_generation_path().exists()
            || self.cursor_path().exists()
            || self.snapshot_path().exists()
    }

    /// Reads the driver's cursor.
    pub fn read_cursor(&self) -> Result<DriverCursor, DriveError> {
        self.read_pair().map(|(cursor, _)| cursor)
    }

    /// Reads the engine's snapshot.
    pub fn read_snapshot(&self) -> Result<Snapshot, DriveError> {
        self.read_pair().map(|(_, snapshot)| snapshot)
    }

    /// Where the atomic state-generation pointer lives.
    pub fn current_generation_path(&self) -> PathBuf {
        self.path.join(CURRENT_GENERATION_FILE)
    }

    /// Reads one authenticated snapshot/cursor pair, migrating a complete legacy pair first.
    pub fn read_pair(&self) -> Result<(DriverCursor, Snapshot), DriveError> {
        if self.current_generation_path().exists() {
            return self.read_generation();
        }
        let cursor_exists = self.cursor_path().exists();
        let snapshot_exists = self.snapshot_path().exists();
        if cursor_exists != snapshot_exists {
            return Err(DriveError::Refused(format!(
                "run state in {} is incomplete: legacy snapshot and cursor must either both exist or both be absent",
                self.path.display()
            )));
        }
        if !cursor_exists {
            return Err(DriveError::Refused(format!(
                "run state in {} has no committed generation",
                self.path.display()
            )));
        }
        let cursor = read_json(&self.cursor_path())?;
        let snapshot = read_json(&self.snapshot_path())?;
        self.persist(&snapshot, &cursor)?;
        Ok((cursor, snapshot))
    }

    /// Writes and seals both records, then atomically publishes their generation.
    ///
    /// Pretty-printed, because the first thing anybody does with a stopped run is read its cursor.
    pub fn persist(&self, snapshot: &Snapshot, cursor: &DriverCursor) -> Result<(), DriveError> {
        fs::create_dir_all(&self.path).map_err(|source| DriveError::Io {
            path: self.path.clone(),
            source,
        })?;
        let snapshot_bytes = json_bytes(&self.snapshot_path(), snapshot)?;
        let cursor_bytes = json_bytes(&self.cursor_path(), cursor)?;
        let generation = self.next_generation()?;
        let generations = self.path.join(GENERATIONS_DIRECTORY);
        fs::create_dir_all(&generations).map_err(|source| DriveError::Io {
            path: generations.clone(),
            source,
        })?;
        let writing = generations.join(format!(".{generation}.writing"));
        if writing.exists() {
            fs::remove_dir_all(&writing).map_err(|source| DriveError::Io {
                path: writing.clone(),
                source,
            })?;
        }
        fs::create_dir(&writing).map_err(|source| DriveError::Io {
            path: writing.clone(),
            source,
        })?;
        write_bytes(&writing.join(SNAPSHOT_FILE), &snapshot_bytes)?;
        write_bytes(&writing.join(CURSOR_FILE), &cursor_bytes)?;
        let manifest = GenerationManifest {
            format: "aep.driver-state-generation/1".to_owned(),
            generation,
            snapshot_sha256: sha256(&snapshot_bytes),
            cursor_sha256: sha256(&cursor_bytes),
        };
        let manifest_path = writing.join("manifest.json");
        let manifest_bytes = json_bytes(&manifest_path, &manifest)?;
        write_bytes(&manifest_path, &manifest_bytes)?;
        let committed = generations.join(generation.to_string());
        fs::rename(&writing, &committed).map_err(|source| DriveError::Io {
            path: committed,
            source,
        })?;
        let pointer = GenerationPointer {
            format: "aep.driver-state-current/1".to_owned(),
            generation,
            manifest_sha256: sha256(&manifest_bytes),
        };
        write_json(&self.current_generation_path(), &pointer)?;

        // Compatibility projections for older readers. They are never read when the pointer
        // exists, so a crash between these writes cannot expose a mixed authoritative pair.
        write_bytes(&self.snapshot_path(), &snapshot_bytes)?;
        write_bytes(&self.cursor_path(), &cursor_bytes)
    }

    /// Reads and verifies the generation selected by the atomic pointer.
    fn read_generation(&self) -> Result<(DriverCursor, Snapshot), DriveError> {
        let pointer_path = self.current_generation_path();
        let pointer: GenerationPointer = read_json(&pointer_path)?;
        if pointer.format != "aep.driver-state-current/1" {
            return Err(DriveError::Malformed {
                path: pointer_path,
                detail: format!("unknown run-state pointer format `{}`", pointer.format),
            });
        }
        let directory = self
            .path
            .join(GENERATIONS_DIRECTORY)
            .join(pointer.generation.to_string());
        let manifest_path = directory.join("manifest.json");
        let manifest_bytes = read_bytes(&manifest_path)?;
        if sha256(&manifest_bytes) != pointer.manifest_sha256 {
            return Err(DriveError::Refused(format!(
                "run state generation {} has a manifest digest mismatch",
                pointer.generation
            )));
        }
        let manifest: GenerationManifest = parse_json(&manifest_path, &manifest_bytes)?;
        if manifest.format != "aep.driver-state-generation/1"
            || manifest.generation != pointer.generation
        {
            return Err(DriveError::Refused(format!(
                "run state generation {} does not identify itself",
                pointer.generation
            )));
        }
        let snapshot_path = directory.join(SNAPSHOT_FILE);
        let cursor_path = directory.join(CURSOR_FILE);
        let snapshot_bytes = read_bytes(&snapshot_path)?;
        let cursor_bytes = read_bytes(&cursor_path)?;
        if sha256(&snapshot_bytes) != manifest.snapshot_sha256
            || sha256(&cursor_bytes) != manifest.cursor_sha256
        {
            return Err(DriveError::Refused(format!(
                "run state generation {} does not match its snapshot/cursor digests",
                pointer.generation
            )));
        }
        Ok((
            parse_json(&cursor_path, &cursor_bytes)?,
            parse_json(&snapshot_path, &snapshot_bytes)?,
        ))
    }

    /// The next generation after every committed or abandoned numeric directory.
    fn next_generation(&self) -> Result<u64, DriveError> {
        let generations = self.path.join(GENERATIONS_DIRECTORY);
        if !generations.exists() {
            return Ok(1);
        }
        let entries = fs::read_dir(&generations).map_err(|source| DriveError::Io {
            path: generations.clone(),
            source,
        })?;
        let maximum = entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        Ok(maximum + 1)
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
        let (cursor, snapshot) = run.read_pair()?;
        (Some(cursor), Some(snapshot))
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
    let (cursor, snapshot) = run.read_pair()?;
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
    retry_in_flight: Option<InFlightAttempt>,
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
    /// The cursor this call walks with: the one carried in, or a fresh one.
    ///
    /// A resume reads its cursor off disk rather than building one, so the theft this call made
    /// on its way in is folded into it here — and only when there *was* one. An unconditional
    /// assignment would clear a recorded theft on the next resume over a free lock, which is the
    /// one direction `took_lock_from` never moves in.
    fn cursor_for(
        &self,
        carried: Option<DriverCursor>,
        run_id: &RunId,
        execution: &Execution,
    ) -> DriverCursor {
        match carried {
            Some(mut existing) => {
                if let Some(stolen) = &self.options.stolen_lock {
                    existing.took_lock_from = Some(stolen.clone());
                }
                existing
            }
            None => fresh_cursor(
                run_id,
                execution,
                self.map,
                self.options.stolen_lock.clone(),
            ),
        }
    }

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
            let mut cursor = self.cursor_for(carried.take(), &run_id, &execution);
            self.check_agreement(&cursor, &execution)?;
            if !checked {
                self.resolve_in_flight(&mut cursor, &execution, &mut tally)?;
            }
            if !checked {
                self.check_map(&execution)?;
                checked = true;
            }
            // A run stopped at an `operator` step reads what arrived while it was stopped before
            // it does anything else, so that whoever answered is in the record before the run
            // walks on — or the run stops again, saying who would be admissible.
            if let Some(owed) = cursor.owed.take() {
                if let Some(report) = self.settle(owed, &execution, &mut cursor, &mut tally)? {
                    return Ok(report);
                }
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

    /// Refuses a silent replay, or applies the operator's explicit resolution once per invocation.
    fn resolve_in_flight(
        &self,
        cursor: &mut DriverCursor,
        execution: &Execution,
        tally: &mut Tally,
    ) -> Result<(), DriveError> {
        let Some(in_flight) = cursor.in_flight.clone() else {
            return Ok(());
        };
        match &self.options.in_flight_resolution {
            Some(InFlightResolution::Retry(named)) if named == &in_flight.id => {
                tally.progress.notes.push(format!(
                    "operator authorised retry of unresolved attempt `{}`",
                    in_flight.id
                ));
                tally.retry_in_flight = Some(in_flight);
                Ok(())
            }
            Some(InFlightResolution::Retry(named)) => Err(DriveError::Refused(format!(
                "`--retry-in-flight` named `{named}`, but this run's unresolved attempt is `{}`",
                in_flight.id
            ))),
            Some(InFlightResolution::RecordNoVerdict) => {
                tally.progress.notes.push(format!(
                    "unresolved attempt `{}` was explicitly recorded with no verdict",
                    in_flight.id
                ));
                cursor.in_flight = None;
                self.directory.persist(&execution.snapshot(), cursor)
            }
            None => Err(DriveError::Refused(format!(
                "attempt `{}` at step {} of `{}` may have run, but no outcome was committed; retry it with `--retry-in-flight {}` or record the uncertainty with `--record-in-flight-no-verdict`",
                in_flight.id, in_flight.step, in_flight.state, in_flight.id
            ))),
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
                 there — a person answers by recording an approval and resuming, and \
                 `--approver agent:<name>` admits one named agent's recorded approval as well — \
                 or run interactively. Reaching this at all means the plan owes an approval that \
                 the pre-flight scan did not see"
            )));
        }

        // The breaker is read *before* the attempt, because the whole point is not to make it. A
        // step skipped here costs nothing; a step attempted against a dependency that has been down
        // since the first state costs a timeout and produces one more line saying the same thing.
        if let Step::Command(command) = step {
            if let (Some(dependency), Some(threshold)) =
                (command.depends_on.as_deref(), command.circuit_breaker)
            {
                if cursor.circuit_is_open(dependency, threshold) {
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

        let attempt = self.begin_attempt(execution, cursor, &state, index, tally)?;
        let outcome = self.execute(executors, execution, &state, index, evaluation, cursor);
        cursor.in_flight = None;
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
                        let failures = cursor.record_circuit_failure(dependency);
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
            StepOutcome::BudgetExhausted { reason } => {
                return self.stop_spend_budget(cursor, execution, tally, reason);
            }
            StepOutcome::Paused { reason } => {
                return self
                    .pause(&reason, index, step, execution, cursor, tally)
                    .map(Some);
            }
        }
        Ok(None)
    }

    /// Stops immediately when the executor refused a paid effect before launch.
    fn stop_spend_budget(
        &self,
        cursor: &DriverCursor,
        execution: &Execution,
        tally: &mut Tally,
        reason: String,
    ) -> Result<Option<RunReport>, DriveError> {
        // This is not a model failure and cannot change by consuming the step's retry budget.
        tally.progress.notes.push(reason);
        let progress = std::mem::take(&mut tally.progress);
        self.finish(
            cursor.clone(),
            execution,
            RunStatus::BudgetExhausted,
            progress,
        )
        .map(Some)
    }

    /// Publishes the attempt marker before dispatch, reusing an explicitly authorised id exactly.
    fn begin_attempt(
        &self,
        execution: &Execution,
        cursor: &mut DriverCursor,
        state: &StateId,
        index: usize,
        tally: &mut Tally,
    ) -> Result<u32, DriveError> {
        let attempt = match tally.retry_in_flight.take() {
            Some(in_flight) if in_flight.state == *state && in_flight.step == index => {
                in_flight.attempt
            }
            Some(in_flight) => {
                return Err(DriveError::Refused(format!(
                    "unresolved attempt `{}` points at step {} of `{}`, but the cursor routes to step {index} of `{state}`",
                    in_flight.id, in_flight.step, in_flight.state
                )))
            }
            None => cursor.begin_attempt(state, index).attempt,
        };
        self.directory.persist(&execution.snapshot(), cursor)?;
        Ok(attempt)
    }

    /// Stops the run at an `operator` step, remembering what is owed.
    fn pause(
        &self,
        reason: &str,
        index: usize,
        step: &Step,
        execution: &Execution,
        cursor: &mut DriverCursor,
        tally: &mut Tally,
    ) -> Result<RunReport, DriveError> {
        let state = cursor.state.clone();
        let label = step.label();
        tally.progress.notes.push(format!(
            "step {index} of `{state}` ({label}) is waiting for an answer: {reason}"
        ));
        // What is owed, remembered so the resume can say who answered it — or that
        // nobody did. `evidence_before` is the record's length now: everything after it
        // arrives while nothing of this run is executing.
        cursor.owed = Some(OwedAnswer {
            state: state.clone(),
            step: index,
            prompt: match step {
                Step::Operator(operator) => operator.prompt.clone(),
                _ => label,
            },
            evidence_before: execution.recorded_evidence().len(),
        });
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
        self.finish(
            cursor.clone(),
            execution,
            RunStatus::AwaitingOperator,
            progress,
        )
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
        // Cloned rather than borrowed off the execution: an `llm` step's authorizer takes the
        // execution mutably for the length of the step, and the context is alive across it.
        let execution_id = execution.id().clone();
        let context = StepContext {
            task: self.task,
            // Off the options rather than off the task: a validated `Task` carries no path, by
            // invariant 10, so the document it was read from is the caller's to state.
            task_document: self.options.task_document.as_deref(),
            execution: &execution_id,
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
    /// Two documents with two owners still carry the same location. Even an internally consistent
    /// generation is refused when their semantic positions disagree.
    /// Reads what arrived while the run was stopped at an `operator` step, and settles it.
    ///
    /// Three outcomes, and the asymmetry between the last two is the point:
    ///
    /// * an **admissible** approval arrived — a person's, or the named agent's — and the cursor
    ///   records who answered ([`DriverCursor::answers`]); the run carries on;
    /// * an approval arrived and **none is admissible** — an agent nobody named, the run's own
    ///   actor, a denial — and the run stops again, saying what was found and who would count.
    ///   Walking on here is what the step exists to prevent: a run that approved its own
    ///   specification would satisfy a principle by writing to the document the principle is
    ///   about;
    /// * **nothing** arrived. With an approver named, the operator asked for a recorded answer and
    ///   there is none, so the run stops again. With none named, the run carries on exactly as it
    ///   did before this existed — a person who moved the artifact the prompt named and resumed
    ///   is that route, and the guard on the way out is what decides whether they did — and the
    ///   report says, in one line, that the record holds nobody's answer. Before this line a run
    ///   that walked past an approval with nothing recorded was indistinguishable from one that
    ///   was approved.
    fn settle(
        &self,
        owed: OwedAnswer,
        execution: &Execution,
        cursor: &mut DriverCursor,
        tally: &mut Tally,
    ) -> Result<Option<RunReport>, DriveError> {
        let own = self.own_actors(execution);
        let named = self.options.approver.as_ref();
        let records = execution.recorded_evidence();
        let arrived = &records[owed.evidence_before.min(records.len())..];
        let mut refused: Vec<String> = Vec::new();
        for recorded in arrived {
            let Evidence::Approval(approval) = &recorded.record.value else {
                continue;
            };
            if approval.decision != ApprovalDecision::Granted {
                refused.push(format!(
                    "approval `{}` by {} was denied, not granted",
                    approval.approval, approval.approver
                ));
                continue;
            }
            match attest::admit(&approval.approver, named, &own) {
                Admission::Admitted => {
                    let by = approval.approver.to_string();
                    tally.progress.notes.push(format!(
                        "step {} of `{}` was answered by {by}: approval `{}` granted",
                        owed.step, owed.state, approval.approval
                    ));
                    cursor.answers.push(OperatorAnswer {
                        state: owed.state,
                        step: owed.step,
                        by,
                        approval: approval.approval.to_string(),
                        evidence: recorded.record.id.to_string(),
                    });
                    return Ok(None);
                }
                Admission::Refused { reason } => {
                    refused.push(format!("approval `{}` — {reason}", approval.approval));
                }
            }
        }

        if refused.is_empty() && named.is_none() {
            tally.progress.notes.push(format!(
                "step {} of `{}` was owed an answer and this run's record holds nobody's: nothing \
                 was recorded while the run was stopped. The run carries on and the guard on the \
                 way out decides, as it did before; whoever answered is not in this record",
                owed.step, owed.state
            ));
            return Ok(None);
        }
        let found = if refused.is_empty() {
            "nothing was recorded while the run was stopped".to_owned()
        } else {
            refused.join("; ")
        };
        tally.progress.notes.push(format!(
            "step {} of `{}` is still owed an answer ({}): {found}. Admissible: {}",
            owed.step,
            owed.state,
            owed.prompt,
            attest::admissible(named)
        ));
        cursor.owed = Some(owed);
        let progress = std::mem::take(&mut tally.progress);
        self.finish(
            cursor.clone(),
            execution,
            RunStatus::AwaitingOperator,
            progress,
        )
        .map(Some)
    }

    /// Every actor this run itself is, in the vocabulary an approval's producer is read in.
    ///
    /// The execution, the task, and the harness each `llm` step runs under: an approval carrying
    /// any of these as its producer is the run approving its own work. Declared identities, as
    /// strong as the record and no stronger — see [`crate::attest`].
    ///
    /// The execution's actor comes from [`attest::session_actor`] rather than being spelled again
    /// here, because that is the same function `aep-cli` hands to each `llm` step's session
    /// in `AEP_ACTOR`. The two have to agree or a run could approve its own work under the very
    /// name it writes to the store under.
    fn own_actors(&self, execution: &Execution) -> Vec<ActorRef> {
        let mut actors: Vec<ActorRef> = attest::session_actor(execution.id()).into_iter().collect();
        let mut names: Vec<String> = vec![self.task.id.to_string()];
        for entry in self.map.states.values() {
            for step in &entry.steps {
                if let Step::Llm(llm) = step {
                    names.push(llm.harness.clone());
                }
            }
        }
        actors.extend(
            names
                .into_iter()
                .filter_map(|name| ActorRef::parse(&format!("agent:{name}")).ok()),
        );
        actors.sort();
        actors.dedup();
        actors
    }

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
///
/// `stolen` is the lock this run superseded on its way in, as its caller read it. It is written
/// here rather than after the run, so a run that supersedes a lock and then blocks, breaks its store
/// or spends its budget without executing a step still leaves the theft in the record — which is
/// precisely the case somebody goes looking for.
fn fresh_cursor(
    run: &RunId,
    execution: &Execution,
    map: &StepMap,
    stolen: Option<StolenLock>,
) -> DriverCursor {
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
        in_flight: None,
        circuit_failures: BTreeMap::new(),
        iterations: 0,
        status: RunStatus::Running,
        reasons: Vec::new(),
        took_lock_from: stolen,
        owed: None,
        answers: Vec::new(),
    };
    // Counted on entry, and the initial state is an entry. A budget that only counted re-entries
    // would let a one-state workflow run forever.
    cursor.record_visit(&initial);
    cursor
}

/// Reads one JSON record.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, DriveError> {
    let bytes = read_bytes(path)?;
    parse_json(path, &bytes)
}

/// Reads bytes while preserving the path in an IO refusal.
fn read_bytes(path: &Path) -> Result<Vec<u8>, DriveError> {
    fs::read(path).map_err(|source| DriveError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Parses JSON bytes while preserving the path in a malformed-record refusal.
fn parse_json<T: serde::de::DeserializeOwned>(path: &Path, bytes: &[u8]) -> Result<T, DriveError> {
    serde_json::from_slice(bytes).map_err(|source| DriveError::Malformed {
        path: path.to_path_buf(),
        detail: source.to_string(),
    })
}

/// Serializes one pretty-printed JSON record to its canonical persisted bytes.
fn json_bytes<T: serde::Serialize>(path: &Path, value: &T) -> Result<Vec<u8>, DriveError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| DriveError::Malformed {
        path: path.to_path_buf(),
        detail: source.to_string(),
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// SHA-256 as lowercase hexadecimal text.
fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        })
}

/// Writes exact bytes through a fixed temporary name.
fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), DriveError> {
    let writing = path.with_extension("writing");
    fs::write(&writing, bytes).map_err(|source| DriveError::Io {
        path: writing.clone(),
        source,
    })?;
    fs::rename(&writing, path).map_err(|source| DriveError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Writes one JSON record, through a fixed temporary name.
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), DriveError> {
    write_bytes(path, &json_bytes(path, value)?)
}
