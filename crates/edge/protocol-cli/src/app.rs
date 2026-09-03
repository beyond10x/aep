// Shared implementation of the canonical command and its compatibility alias.
//
// Every subcommand is a thin shell over the library: the CLI parses arguments, loads documents and
// renders results. It decides nothing, which is the point — if `protocol evaluate` says a transition
// is blocked, a harness calling the same engine gets the same answer.
//
// Exit codes: `0` success, `1` the documents or the execution say no, `2` bad usage, `3` nobody
// found out.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_memory::{seed, MemoryBackend};
use aep_contract::consistency::QueryConsistency;
use aep_contract::query::{AuditQuery, EntityQuery, QueryService, RelationQuery};
use aep_contract::testing::block_on;
use aep_domain::action::{Action, ActionRequest, ProductionMutate};
use aep_domain::artifact::ArtifactGraph;
use aep_domain::capability::Capability;
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef};
use aep_domain::task::Task;
use aep_domain::time::Timestamp;
use aep_engine::engine::{EvidenceSubmission, ProtocolEngine, TransitionResult};
use aep_engine::{Engine, Registry};
use aep_project::load_tree_report;
use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};

/// Who the entity surface seeds as.
///
/// Fixed, and a `service:` actor rather than a person: nobody authorised these writes, the CLI made
/// them to have something to answer about.
const SEED_ACTOR: &str = "service:protocol-cli";

/// When the entity surface seeds.
///
/// Fixed so two runs over the same manifest produce byte-identical output. A wall clock here would
/// make every `--format json` diff noise.
const SEED_AT: Timestamp = Timestamp::EPOCH;

/// Reference CLI for the Agentic Engineering Protocol.
#[derive(Debug, Parser)]
#[command(
    name = "protocol",
    bin_name = "protocol",
    about,
    version,
    disable_help_subcommand = true
)]
struct Cli {
    /// What to do.
    #[command(subcommand)]
    command: Command,
}

/// How to render results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    /// Human-readable lines.
    Text,
    /// YAML, for another tool to read.
    Yaml,
    /// JSON, for another tool to read.
    Json,
}

/// Where the documents are.
#[derive(Debug, Args)]
struct RootArgs {
    /// The document tree to load.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
}

/// The inputs an execution needs.
#[derive(Debug, Args)]
struct ExecutionArgs {
    /// The document tree to load. Inside a project, this comes from `.engineering/project.yaml`.
    #[arg(long)]
    root: Option<PathBuf>,
    /// The task document. Inside a project, this comes from `.engineering/task.yaml`.
    #[arg(long)]
    task: Option<PathBuf>,
    /// An artifact manifest. Inside a project, this comes from `.engineering/artifacts.yaml`.
    #[arg(long)]
    artifacts: Option<PathBuf>,
    /// Evidence to submit before evaluating, as a list of submissions.
    #[arg(long)]
    evidence: Vec<PathBuf>,
    /// A snapshot to resume from.
    #[arg(long)]
    state: Option<PathBuf>,
    /// Advance the execution as far as the evidence permits before reporting.
    #[arg(long)]
    advance: bool,
    /// How to render the result.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

/// What the entity and audit surface needs in order to answer.
///
/// A source of artifacts is required because the backend is in-memory: without one there is
/// nothing to answer about. There are two, and exactly one must be given.
///
/// * `--artifacts` — a manifest, which is how a project that keeps its planning somewhere else
///   points at what exists. `examples/development-passkeys` is that arrangement: the stories live
///   in Linear and the manifest names them.
/// * `--planning` — a markdown planning store, which is how a project that keeps its planning
///   *here* says the same thing. The store is read, every document becomes an artifact located at
///   its own file, and the graph that comes out is fed to exactly the same seeder.
///
/// Neither is privileged, and the entity surface cannot tell them apart once seeded — which is the
/// point of routing both through [`ArtifactGraph`] rather than teaching this command two sources.
#[derive(Debug, Args)]
#[command(group(
    clap::ArgGroup::new("artifact-source").required(true).args(["artifacts", "planning"])
))]
struct BackendArgs {
    /// The document tree; a relative `--artifacts` or `--planning` path is resolved against it.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// The artifact manifest to seed the in-memory backend from.
    #[arg(long)]
    artifacts: Option<PathBuf>,
    /// A markdown planning store to seed the in-memory backend from, instead of a manifest.
    #[arg(long)]
    planning: Option<PathBuf>,
    /// The organisation the seeded locators live under.
    #[arg(long, default_value = "local")]
    organisation: String,
    /// The space the seeded locators live under.
    #[arg(long, default_value = "manifest")]
    space: String,
    /// How to render the result.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

/// Which backend `protocol conformance` holds to the suites.
///
/// Its own enum rather than a free string so an unknown backend is a usage error naming the three
/// that exist, and so the report can say which one answered in the words the caller typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ConformanceBackend {
    /// The in-memory reference implementation. The default, so no existing invocation changes
    /// meaning; it keeps nothing, so it takes no `--store`.
    Memory,
    /// The markdown planning store, `aep-backend-markdown`, at `--store <dir>` — or a scratch
    /// directory when none is given, because the suites write.
    Markdown,
    /// The SQLite backend, `aep-backend-sqlite`, at `--store <file>` — or an in-memory database
    /// when none is given.
    Sqlite,
    /// The Postgres backend, `aep-backend-postgres`, at `--store <url>` — required, because there
    /// is no scratch server to invent. The suites write; give it a database of their own.
    Postgres,
    /// The hybrid, `aep-backend-hybrid`: the markdown plan at `--store <dir>` — or a scratch
    /// directory — with an in-memory SQLite replica, the markdown side the authority, divergences
    /// recorded. The composite held to the same sixteen suites (`story:hybrid-backend`).
    Hybrid,
    /// The kind of store the project this is run in configured (`store:` in `project.yaml`), held
    /// to the suites on a scratch instance of it: a scratch directory for `markdown`, an in-memory
    /// database for `sqlite`, a schema of its own on the configured server for `postgres`, a
    /// scratch directory with an in-memory replica under the project's own policy for `hybrid`.
    /// The suites write, and the project's plan is not theirs to write into.
    Project,
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Check that a document tree is structurally and semantically valid.
    Validate {
        /// The document tree to load.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// An artifact manifest to validate as well.
        #[arg(long)]
        artifacts: Option<PathBuf>,
        /// How to render the result.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Also write what this validation found as a `verification` evidence document.
        ///
        /// The claim is [`DOCUMENT_TREE_VALID`] and the verifier is this tool. It is the same walk
        /// the report above describes, written in the shape `protocol evaluate --evidence` reads,
        /// so a driven step can declare `kind: verification` with a `record:` and submit what the
        /// validator found rather than a verdict minted from an exit status.
        ///
        /// A tree with problems still writes a record — one saying `failed`, naming what it found.
        /// The verdict belongs in the document; refusing to write one would be the validator
        /// deciding that bad news is not an observation.
        #[arg(long, value_name = "PATH")]
        evidence: Option<PathBuf>,
    },
    /// Resolve a task into an execution plan.
    Resolve(ExecutionArgs),
    /// Show what a protocol, principle, workflow or profile declares.
    Inspect {
        /// The document tree to load.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// What to inspect, such as `aep/1`, `test-driven` or `development.standard`.
        reference: Option<String>,
        /// How to render the result.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Evaluate an execution: what is owed, what is permitted, what is missing.
    Evaluate(ExecutionArgs),
    /// Explain a decision, or why a task is incomplete.
    Explain {
        /// The execution inputs.
        #[command(flatten)]
        execution: ExecutionArgs,
        /// A capability to ask about, such as `production.write`.
        #[arg(long)]
        action: Option<String>,
    },
    /// Browse the plan in a browser, and move artifacts along their ladders from it.
    ///
    /// A board is a shape and a terminal prints lines, so triage is what the CLI is worst at. This
    /// answers the same facts `artifact board`, `show` and `explain` print, and takes status moves
    /// back through the same decision `artifact move` makes.
    ///
    /// **It binds `127.0.0.1` and there is no flag that widens it.** Reaching it from another
    /// machine is `ssh -L`. The URL it prints carries a token for the run; a request without that
    /// token is refused, which is what stops another page in the same browser writing to the store.
    /// It is not authentication, and the module says so.
    Serve {
        /// Where the plan is, and which documents govern it.
        #[command(flatten)]
        location: planning::StoreLocation,
        /// The port to listen on. `0` takes whatever the operating system offers.
        #[arg(long, default_value_t = 8899)]
        port: u16,
        /// Answer reads and refuse every transition.
        #[arg(long)]
        read_only: bool,
    },
    /// Plan work in the markdown planning store: epics, stories, tasks and how they relate.
    ///
    /// The store is a directory of markdown files — one artifact per file, YAML frontmatter, free
    /// markdown body — under `.engineering/planning/` by default. It is the first durable store in
    /// this repository, and it is durable in the way that matters for a plan: the diff of a status
    /// move is one line, and `git log` already knows who made it.
    ///
    /// # These verbs write, and they have no `--out`
    ///
    /// A deliberate departure from `ess generate`, `ess synthesize` and `ess conform synthesize`,
    /// which all refuse to write without `--out` because a verb that scatters files over a working
    /// tree the first time somebody tries it is a verb nobody tries twice. That argument does not
    /// reach here, and the difference is worth stating rather than leaving as an inconsistency:
    ///
    /// * those verbs write a **tree** whose shape the caller cannot predict; these write **exactly
    ///   one file**, at a path the id determines — `story:passkey-login` is
    ///   `story/passkey-login.md`, always;
    /// * they write into whatever directory is passed; these write inside a directory that was
    ///   **opted into**, either by `--store` or by the project's own `.engineering/planning/`;
    /// * and `protocol artifact new` is not a preview of anything. A plan item you did not want is
    ///   retired with `protocol artifact move <id> --to archived`, which keeps its record — never
    ///   with `rm`, which `validate` reports as a deletion no command made. Neither is true of a
    ///   synthesised workspace.
    ///
    /// `new` still refuses to overwrite an existing document, which is the part of the `--out`
    /// argument that does apply.
    Artifact {
        /// What to do with the plan.
        #[command(subcommand)]
        command: planning::ArtifactCommand,
    },
    /// Read a repository that already exists into the protocol's own terms.
    ///
    /// Every other verb here starts from a document somebody wrote; these start from a repository
    /// somebody built, which is the state an adopter is actually in. `scan` reads and interprets
    /// nothing — it emits located facts for an agent to judge, and writes nothing. `init` writes the
    /// project file. `openapi` drafts a domain from a contract that already exists, and names every
    /// decision it could not take rather than omitting it.
    Reverse {
        /// What to read.
        #[command(subcommand)]
        command: reverse::ReverseCommand,
    },
    /// Report whether this checkout is in a state the other verbs will accept.
    ///
    /// One line per check — the binary's version, the project file, the protocol source it names,
    /// the planning store, each plugin directory given, and the checkout's newest release tag —
    /// each `ok`, `warn` or `fail`. Exit `1` on any `fail`.
    ///
    /// It fixes nothing, which is what makes it safe to run first: a checker that also repairs
    /// cannot be run to find out what is wrong. It reads no clock and opens no connection, so a
    /// pinned protocol source is checked for shape and for a cached snapshot and never fetched.
    Doctor(doctor::DoctorArgs),
    /// Judge an agent run against a typed specification, or report what is in one.
    ///
    /// The transcript comes from a harness that has already finished — these verbs never start an
    /// agent, never call a model and never reach a network. They read a file and evaluate typed
    /// predicates over it, which is what makes a verdict reproducible: the same transcript and the
    /// same specification produce the same report on any machine, at any load, on any day.
    ///
    /// The third observation domain in this repository, after an authored specification and a
    /// scanned cluster, and it takes the same shape on purpose: a content-addressed IR, an
    /// authored expectation document, and `ok`/`gap`/`unk` verdicts where the third value means
    /// *the adapter did not understand the event*.
    Trace {
        /// What to do with it.
        #[command(subcommand)]
        command: trace::TraceCommand,
    },
    /// Turn a contract runner's own record into evidence the engine reads.
    ///
    /// The consumer/provider contract — *does the published interface still behave as its consumers
    /// were told?* — and specifically the record an outside runner prints about it. metaharness
    /// contract-tests each `metaharness ⇄ vendor` adapter and emits the outcome in the
    /// `contract_result` shape this repository defines, which is a shared vocabulary rather than a
    /// dependency: no crate crosses that boundary, because this repository is public and that one is
    /// not.
    ///
    /// Not to be confused with `protocol conformance`, which asks whether a **backend** implements
    /// `aep-contract` — storage, commands, queries, audit. Neither subsumes the other and the only
    /// thing they share is the word.
    Contract {
        /// What to do with a record.
        #[command(subcommand)]
        command: contract::ContractCommand,
    },
    /// Check the properties this repository's own decisions rest on, and write the record.
    ///
    /// `principles/verification/property-based-testing.yaml` owes every code task an independent
    /// `property_test_result` from a `property-tester`, and a `property_test_result` carries a
    /// property name, a case count and a seed — none of which an exit status holds. So the check
    /// runs in this process and writes its own document, which a step map reads back through
    /// `evidence.record:`.
    Property {
        /// What to do with them.
        #[command(subcommand)]
        command: property::PropertyCommand,
    },
    /// Decide a specification's requirements against the evidence a run has admitted.
    ///
    /// `principles/development/spec-driven.yaml` owes `specification.satisfied` before completion,
    /// and only a `specification` record projects it. What counts as a requirement in a markdown
    /// artifact is this verb's own decision and is stated in full in its module documentation: a
    /// list item under a `Requirements` or `Acceptance` heading, satisfied when the predicate it
    /// names is observed `True`.
    Specification {
        /// What to do with one.
        #[command(subcommand)]
        command: specification::SpecificationCommand,
    },
    /// Assemble what many checked runs said into one table of facts.
    ///
    /// The evaluation programme's deliverable. Its runs come in three arms — raw instructions, the
    /// shipped plugin, a driven run whose calls an enforcer decides — against more than one
    /// harness, and each leaves a run manifest beside the record `protocol trace check` wrote about
    /// its transcript. This verb counts, per harness × arm × workflow and per expectation, how many
    /// facts held, how many were contradicted and how many nobody could find out.
    ///
    /// **It computes no score**, and refuses to: the only ways to fold three columns into one
    /// number are to count an unobservable expectation as a pass, which is the collapse invariant 5
    /// exists to refuse, or as a failure, which blames an agent for a field a harness stopped
    /// recording.
    ///
    /// Not to be confused with `protocol evaluate`, which asks the engine what one task owes and
    /// what it is permitted. This verb decides nothing and reads no protocol document; the only
    /// thing the two share is a stem.
    Eval {
        /// What to do with a set of runs.
        #[command(subcommand)]
        command: eval::EvalCommand,
    },
    /// Walk a workflow: run the steps a step map declares, and do only what the engine permits.
    ///
    /// The reference driver. It makes the engine's calls in order, executes the three kinds of step
    /// that touch the world — a program, a model, a person — and records what it did. It evaluates
    /// no gate itself: a driver that could evaluate a gate would be a second protocol
    /// implementation with none of the conformance suites, and the first time the two disagreed the
    /// one nobody tested would win.
    Drive {
        /// What to do with a run.
        #[command(subcommand)]
        command: drive::DriveCommand,
    },
    /// Draw a workflow, and a run over it.
    ///
    /// The engine answers *may this move?* in words; this answers *where is it?* in a picture —
    /// the states down the page, the guards beside the arrows, and, when a run is drawn over them,
    /// where it is, where it has been, what it produced and why it stopped. It evaluates nothing:
    /// every overlay it draws was decided by the engine and read out of a run directory.
    Workflow {
        /// What to do with it.
        #[command(subcommand)]
        command: render::WorkflowCommand,
    },
    /// Ask the reference backend about the entities an artifact manifest or planning store holds.
    Entity {
        /// Which question to ask about them.
        #[command(subcommand)]
        command: EntityCommand,
    },
    /// Show the audit trail, oldest first.
    ///
    /// The backend is in-memory, so this run seeds it from `--artifacts` or `--planning` and then
    /// reads: what you see is the seeding itself, not a durable past.
    Audit {
        /// Where the artifacts come from and how to render.
        #[command(flatten)]
        backend: BackendArgs,
        /// Only records from this activity; the seeding run is `seed-manifest`.
        #[arg(long)]
        correlation: Option<String>,
        /// Only records about one entity, by locator or identity.
        #[arg(long)]
        entity: Option<String>,
        /// Only refused attempts — what something tried to do and was stopped from doing.
        #[arg(long)]
        rejected: bool,
    },
    /// Describe an entity type: what it is, whether it may change, and what may target it.
    ///
    /// This is how a harness asks what a design *is* rather than hard-coding it. The source is
    /// still seeded, because the answer comes from the same backend that holds the entities.
    Describe {
        /// Where the artifacts come from and how to render.
        #[command(flatten)]
        backend: BackendArgs,
        /// The type to describe, such as `aep.design/v1`.
        entity_type: String,
    },
    /// Read the dated claims a document makes, and say which of them nobody has looked at since.
    ///
    /// The observation half of evidence horizons. `scan` reads human-written markdown for the
    /// annotation convention a claim is written in; `inspect` reads an evidence file of the kind
    /// `protocol evaluate --evidence` submits. Neither writes anything, neither resolves a plan and
    /// neither decides a gate: they report what a document says about when somebody last looked.
    Evidence {
        /// Which question to ask.
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Answer across the repositories a workspace names, rather than only this one.
    ///
    /// A project file says what *this* repository runs under; `.engineering/workspace.yaml` says
    /// which repositories one command should answer across. A story here blocked by a story
    /// somewhere else can then say so, and one board shows the work rather than three.
    ///
    /// A repository without a workspace file is not a broken workspace — it is a repository that
    /// answers only for itself, which is the ordinary case.
    Workspace {
        /// Which question to ask.
        #[command(subcommand)]
        command: workspace::WorkspaceCommand,
    },
    /// Inspect built-in schemas or work with project-owned JSON Schema contracts.
    Schema {
        /// What to do. A built-in schema name such as `workflow` remains accepted.
        #[command(subcommand)]
        command: Option<schema::SchemaCommand>,
    },
    /// Check a storage backend against the AEP contract suites.
    ///
    /// The question is whether a **backend** implements `aep-contract` — commands, queries, audit,
    /// idempotency, consistency — and the answer is about storage, not about any system you have
    /// specified.
    ///
    /// The other conformance verb answers a different question. `protocol ess conform` asks whether
    /// an **implementation** satisfies an executable system specification: whether `CreateInvoice`
    /// with a negative amount is refused, whether a paid invoice can still be cancelled. Design §42
    /// calls this one contract conformance and that one semantic conformance; neither subsumes the
    /// other, and a backend passing here says nothing about a system passing there.
    ///
    /// Runs against the backend `--backend` names — `memory`, the reference implementation, unless
    /// told otherwise — and the report's first line says which one answered, because a report that
    /// does not name what it ran against is a report somebody will attribute to the wrong thing:
    /// this verb was hard-coded to `memory` for two releases while a story ticked "runs against the
    /// markdown store". `--store` says where a durable backend lives. **The suites write**, so a
    /// durable backend given no `--store` gets a scratch store, and one pointed at a plan you keep
    /// will append the suites' commands to that plan's journal.
    ///
    /// `--inject` deliberately breaks one property, to show that the suite responsible for it
    /// actually fails — a suite that passes everything tells you nothing.
    Conformance {
        /// How much of the contract to check: core, audited or full.
        #[arg(long, default_value = "full")]
        level: String,
        /// Run one suite by name instead of a whole level.
        #[arg(long)]
        suite: Option<String>,
        /// Break one property on purpose, to see which suite catches it.
        #[arg(long)]
        inject: Option<String>,
        /// Which backend to hold to the suites.
        #[arg(long, value_enum, default_value_t = ConformanceBackend::Memory)]
        backend: ConformanceBackend,
        /// Where a durable backend lives: a directory for `markdown`, a file for `sqlite`, a URL
        /// for `postgres`. Not with `project`, whose store `project.yaml` names.
        #[arg(long)]
        store: Option<PathBuf>,
        /// How to render the result.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
}

/// The two questions the evidence surface answers.
///
/// Both take `--at`, and every test passes it: a report whose answer depends on the day it is run
/// is a report that cannot be checked into a repository. Without it, `--at` is today.
#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// Scan markdown documents for dated claims, and report coverage beside the classification.
    ///
    /// # The coverage line is the point
    ///
    /// A scanner over human-written documents needs a coverage claim of its own. This one counts
    /// annotation-shaped occurrences *without* the parser and compares that number with the records
    /// the parser produced; a divergence means an annotation is present, correct, legible to a
    /// human and invisible to the gate. The comparison is one line and it belongs in the gate
    /// rather than in an investigation — on a real corpus it is what surfaced 15 unwatched
    /// annotations out of 160.
    Scan {
        /// The markdown files to read. A directory is read one level deep for `*.md`.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// The day to classify against, such as `2026-09-01`. Defaults to today.
        #[arg(long, value_name = "DATE")]
        at: Option<String>,
        /// How many days of remaining life still count as `expiring`.
        #[arg(long, default_value_t = 0)]
        warn_days: u32,
        /// Exit non-zero when the parser found fewer records than there are occurrences.
        ///
        /// Coverage only. An expired record is a normal, expected finding — a corpus with none is a
        /// corpus nobody has kept — so it is `--fail-on-expired` that judges it, and the two are
        /// separate flags because they answer different questions: *is the gate blind?* and *is the
        /// claim stale?*.
        #[arg(long)]
        strict: bool,
        /// Exit non-zero when any record is past its horizon.
        #[arg(long)]
        fail_on_expired: bool,
        /// How to render the report.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Read an evidence file and report, per record, when somebody last looked.
    ///
    /// Reads the same document `protocol evaluate --evidence` submits and puts every record to the
    /// engine's own future-observation comparison, so the two verbs answer identically about one
    /// file: an observation written as a calendar date is refused only once that day has begun in
    /// no timezone, one written as epoch milliseconds is compared exactly, and either refusal
    /// names the record rather than the document. The check that makes a scheduled re-check
    /// unwritable is therefore available before anything is submitted to an engine.
    ///
    /// The one place the two verbs differ is `--at`, which pins the comparison to a chosen day
    /// instead of the wall clock.
    Inspect {
        /// The evidence files to read.
        #[arg(required = true)]
        evidence: Vec<PathBuf>,
        /// The day to age the records against. Defaults to today.
        ///
        /// It also pins the future-observation check, to the **end** of that day — reading a
        /// record the day it was written is the verb's primary use, and a record stamped 14:07 is
        /// inside its day rather than ahead of its first millisecond. Without it the check runs
        /// against the wall clock, which is the instant `protocol evaluate` submits against.
        #[arg(long, value_name = "DATE")]
        at: Option<String>,
        /// A horizon to apply for the report, such as `7d`.
        ///
        /// **Report only.** It is a what-if applied to a printed table: it reaches no requirement,
        /// no evaluation and no document, and nothing it prints can extend the life of a record.
        /// The horizon that decides a gate is declared on a requirement, in a reviewed document,
        /// and is re-read on every resolve.
        #[arg(long, value_name = "DAYS")]
        horizon: Option<String>,
        /// How to render the report.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
}

/// The questions the entity surface answers.
///
/// Every one of them seeds an in-memory backend from `--artifacts` or `--planning` and then reads
/// it back. Whichever source is given, the artifacts arrive as one [`ArtifactGraph`] and nothing
/// downstream can tell which it was. Nothing
/// here is durable: `protocol entity history` shows this run's seeding, and running it again
/// produces the same answer rather than a longer history.
#[derive(Debug, Subcommand)]
enum EntityCommand {
    /// List every entity the source seeds, with its type, locator and revision.
    ///
    /// The backend is in-memory: this run seeds it from `--artifacts` or `--planning` and then
    /// answers. Every entity here was created moments ago by this process.
    List {
        /// Where the artifacts come from and how to render.
        #[command(flatten)]
        backend: BackendArgs,
        /// Only entities of this type, such as `aep.design/v1`.
        #[arg(long = "type")]
        entity_type: Option<String>,
    },
    /// Print one entity, addressed by locator or by identity.
    ///
    /// The backend is in-memory: this run seeds it from `--artifacts` or `--planning` and then
    /// answers. Exits 1 when nothing the source seeds is addressed by what was asked for.
    Get {
        /// Where the artifacts come from and how to render.
        #[command(flatten)]
        backend: BackendArgs,
        /// A locator such as `ep://local/manifest/design/passkeys-auth`, or an entity identity.
        reference: String,
    },
    /// Show an entity's revision records, oldest first.
    ///
    /// The backend is in-memory: what this shows is *the seeding*, not a durable past. Every
    /// entity is therefore at revision 1, and running the command again does not lengthen it.
    History {
        /// Where the artifacts come from and how to render.
        #[command(flatten)]
        backend: BackendArgs,
        /// A locator or an entity identity.
        reference: String,
    },
    /// Show what an entity points at, or — with `--incoming` — what points at it.
    ///
    /// The backend is in-memory: this run seeds it from `--artifacts` or `--planning` and then
    /// answers. The edges are the source's own `relations`, stored as relation commands.
    Relations {
        /// Where the artifacts come from and how to render.
        #[command(flatten)]
        backend: BackendArgs,
        /// A locator or an entity identity.
        reference: String,
        /// Answer "what points at this?" instead.
        #[arg(long)]
        incoming: bool,
    },
}

/// Writes to standard output, treating a closed pipe as a normal end rather than a crash.
///
/// Rust's `println!` panics when the reader goes away, so `protocol inspect | head -3` ends in a
/// stack trace instead of three lines. A consumer that stopped reading is not an error this program
/// has anything to say about, so it exits quietly.
fn write_out(text: &str, newline: bool) {
    use std::io::Write;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let outcome = if newline {
        writeln!(handle, "{text}")
    } else {
        write!(handle, "{text}")
    };
    if let Err(error) = outcome {
        if error.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("error: cannot write to stdout: {error}");
        std::process::exit(1);
    }
}

/// `println!`, but a closed pipe ends the program quietly.
///
/// Written `crate::write_out` rather than `write_out` so the macro works from a submodule too. A
/// `macro_rules!` macro is textually scoped: it is in scope for everything declared after it,
/// including [`planning`], and an unqualified call inside it would resolve against *that* module's
/// namespace rather than this one's.
macro_rules! outln {
    () => { crate::write_out("", true) };
    ($($arg:tt)*) => { crate::write_out(&format!($($arg)*), true) };
}

/// `print!`, but a closed pipe ends the program quietly.
macro_rules! out {
    ($($arg:tt)*) => { crate::write_out(&format!($($arg)*), false) };
}

// Declared here, below the macros, and not at the top with the `use` statements: `macro_rules!` is
// textually scoped, so a module declared above this point could not call `outln!`. The first module
// split of this file, and the criterion for the next one is the same as this one's — a verb family
// with its own store, its own vocabulary and no shared state with the rest.
mod planning;
mod serve;

// The second module split, on the same criterion: a verb family with its own observation
// domain, its own vocabulary and no shared state with the rest of the binary. It brings its own
// `--format` enum too, because a check report has two useful renderings and not three.
mod trace;

// The third, on the same criterion again: a verb family with its own store — a run directory — its
// own vocabulary, and no shared state with the rest. It is also where the three things that touch
// the world live, which is why they are here and not in `aep-driver`.
mod drive;

// The fourth, and it holds the same boundary the third does: `aep-render` decides what a picture
// looks like, and the poll loop, the rasteriser shell-out and the read of a run directory are here
// because that crate is scanned for exactly those things.
mod render;

// The fifth. Its input is one JSON object an outside contract runner printed, its vocabulary is the
// `contract_result` payload `aep-domain` already defines, and it shares nothing with the rest — the
// smallest a verb family in this binary has ever been, because the shared vocabulary did most of the
// work before the module existed.
mod contract;

// The sixth. Its input is a pair of documents per run — a manifest this repository defines and the
// check report `protocol trace check` writes — its vocabulary is the three arms of the evaluation
// programme, and it shares nothing with the rest. It is also where the one rule that programme has
// about its own output lives: counts of facts, never a score.
mod eval;
mod money;
mod redaction;
mod workspace;

// The seventh, and the only one whose output is another component's input. It reads the same
// workflow `render` draws and writes the document `b10x-harness-flow` plans, which is a projection
// and deliberately not an equivalence - see the module for what it drops and why. It answers, for
// free, whether a workflow fits a notation before anything is paid to run one under it.
mod flow;

// Project-owned JSON Schema contracts. The schema documents are the source of truth; this module
// only discovers their project registry, validates instances and writes deterministic projections.
mod schema;

// The eighth, and the first whose input is a repository rather than a document. It shares the
// criterion the others were split on — its own vocabulary, its own output shape, no shared state —
// and adds one of its own: it is the only module here that reads a tree it does not govern, so the
// rules that keep it honest (fixed enumeration order, no clock, no network, stdout is data) are
// stated in it rather than assumed from the rest.
mod reverse;

// Not a verb family at all — one verb, and the only one here that answers about the *installation*
// rather than about a document. It is a module because its six checks each reuse a different part
// of this binary (the project loader, the source locator, the planning store's own validation, the
// driver's plugin rule, the tag listing) and a verb that reaches that widely does not belong inline
// in the dispatcher.
mod doctor;

// Not a verb family: the envelope every producer added after
// `story:evidence-producers-for-the-driven-map` puts around a record, said once. See the module for
// why the three older minting verbs are deliberately not migrated onto it.
mod evidence_doc;

// The ninth verb family. Its input is the workspace's own three-valued core, its vocabulary is
// `property_test_result`, and it shares nothing with the rest: a property checker that runs in this
// process and writes down what it measured, because a `property_test_result` minted from an exit
// status would state a case count nobody read.
mod property;

// The tenth. Its inputs are a specification artifact's body and the evidence a run has admitted,
// its vocabulary is `specification`, and it shares nothing with the rest. It is the one producer
// here that decides a *document* against a run rather than code against a suite.
mod specification;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(1)
        }
    }
}

/// Runs the CLI, returning the process exit code.
fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate {
            root,
            artifacts,
            format,
            evidence,
        } => validate(&root, artifacts.as_deref(), format, evidence.as_deref()),
        Command::Resolve(args) => resolve(&args),
        Command::Inspect {
            root,
            reference,
            format,
        } => inspect(&root, reference.as_deref(), format),
        Command::Evaluate(args) => evaluate(&args, None),
        Command::Explain { execution, action } => evaluate(&execution, action.as_deref()),
        Command::Artifact { command } => planning::run(command),
        Command::Serve {
            location,
            port,
            read_only,
        } => serve::run(&location, port, read_only),
        Command::Trace { command } => trace::run(command),
        Command::Contract { command } => contract::run(command),
        Command::Property { command } => property::run(command),
        Command::Specification { command } => specification::run(command),
        Command::Eval { command } => eval::run(command),
        Command::Drive { command } => drive::run(command),
        Command::Workflow { command } => render::run(command),
        Command::Entity { command } => entity(&command),
        Command::Audit {
            backend,
            correlation,
            entity,
            rejected,
        } => audit(
            &backend,
            correlation.as_deref(),
            entity.as_deref(),
            rejected,
        ),
        Command::Describe {
            backend,
            entity_type,
        } => describe(&backend, &entity_type),
        Command::Evidence { command } => match command {
            EvidenceCommand::Scan {
                paths,
                at,
                warn_days,
                strict,
                fail_on_expired,
                format,
            } => evidence_scan(
                &paths,
                observation_day(at.as_deref())?,
                warn_days,
                strict,
                fail_on_expired,
                format,
            ),
            EvidenceCommand::Inspect {
                evidence,
                at,
                horizon,
                format,
            } => {
                let day = observation_day(at.as_deref())?;
                evidence_inspect(
                    &evidence,
                    day,
                    future_reference(at.as_deref(), day),
                    horizon.as_deref(),
                    format,
                )
            }
        },
        Command::Reverse { command } => reverse::run(command),
        Command::Doctor(args) => doctor::run(&args),
        Command::Workspace { command } => workspace::run(command),
        Command::Schema { command } => schema::run(command),
        Command::Conformance {
            level,
            suite,
            inject,
            backend,
            store,
            format,
        } => conformance(
            &level,
            suite.as_deref(),
            inject.as_deref(),
            backend,
            store.as_deref(),
            format,
        ),
    }
}

/// `protocol conformance`
fn conformance(
    level: &str,
    suite: Option<&str>,
    inject: Option<&str>,
    backend: ConformanceBackend,
    store: Option<&Path>,
    format: Format,
) -> Result<ExitCode> {
    let level = conformance_level(level)?;

    let fault = match inject {
        None => None,
        Some(name) => Some(parse_fault(name)?),
    };

    let ConformanceTarget {
        backend,
        store,
        scratch_schema,
        hybrid_policy,
    } = conformance_target(backend, store)?;
    let store = store.as_deref();

    // One arm per backend, each saying in the report what it ran against. The durable ones without
    // `--store` get a scratch store rather than a refusal, because the suites write and a person
    // asking "does this backend conform?" should not first have to invent a place for it to write.
    let report = match backend {
        ConformanceBackend::Memory => {
            if let Some(path) = store {
                anyhow::bail!(
                    "`--backend memory` keeps nothing, so `--store {}` would have no effect; drop \
                     it, or name a durable backend (`markdown`, `sqlite`)",
                    path.display()
                );
            }
            run_against(
                aep_backend_memory::MemoryBackend::new(),
                fault,
                level,
                suite,
            )?
            .ran_against("memory")
        }
        ConformanceBackend::Sqlite => {
            if let Some(path) = store {
                let backend = aep_backend_sqlite::SqliteBackend::open(path)
                    .map_err(|error| anyhow::anyhow!("{error}"))
                    .with_context(|| format!("opening the SQLite store at {}", path.display()))?;
                run_against(backend, fault, level, suite)?
                    .ran_against(format!("sqlite ({})", path.display()))
            } else {
                let backend = aep_backend_sqlite::SqliteBackend::in_memory()
                    .map_err(|error| anyhow::anyhow!("{error}"))?;
                run_against(backend, fault, level, suite)?
                    .ran_against("sqlite (in-memory database)")
            }
        }
        ConformanceBackend::Postgres => {
            let Some(url) = store.and_then(Path::to_str) else {
                anyhow::bail!(
                    "`--backend postgres` needs `--store <url>`: there is no scratch server to \
                     invent, and the suites write — give them a database of their own"
                );
            };
            let (backend, where_) = match &scratch_schema {
                Some(schema) => (
                    aep_backend_postgres::PostgresBackend::connect_in_schema(url, schema),
                    format!("postgres ({}, schema {schema})", planning::redact(url)),
                ),
                None => (
                    aep_backend_postgres::PostgresBackend::connect(url),
                    format!("postgres ({})", planning::redact(url)),
                ),
            };
            let backend = backend
                .map_err(|error| anyhow::anyhow!("{error}"))
                .with_context(|| "connecting to the Postgres store".to_owned())?;
            run_against(backend, fault, level, suite)?.ran_against(where_)
        }
        ConformanceBackend::Markdown => {
            let (backend, root) = markdown_conformance_backend(store)?;
            run_against(backend, fault, level, suite)?
                .ran_against(format!("markdown ({})", root.display()))
        }
        ConformanceBackend::Hybrid => {
            let (backend, root, policy) =
                hybrid_conformance_backend(store, hybrid_policy.as_ref())?;
            run_against(backend, fault, level, suite)?.ran_against(format!(
                "hybrid ({}, in-memory SQLite replica, authority {policy})",
                root.display()
            ))
        }
        ConformanceBackend::Project => {
            unreachable!("`project` was resolved to the store the project names above")
        }
    };

    match format {
        Format::Text => {
            outln!("{report}");
            if let Some(fault) = fault {
                outln!(
                    "injected fault: {} — expected to be caught by the `{}` suite",
                    fault.describe(),
                    fault.caught_by()
                );
            }
        }
        Format::Yaml | Format::Json => print_serialised(&report, format)?,
    }

    Ok(exit_code(report.passed()))
}

/// The markdown backend the suites run against: at `store`, or a scratch directory when none was
/// given, because the suites write.
///
/// Permissive ladders and no workspace members: the suites are about the contract and durability,
/// not about any particular ladder, and a real ladder would refuse moves the suites are entitled to
/// make — the same choice `aep-backend-markdown`'s own conformance test takes.
fn markdown_conformance_backend(
    store: Option<&Path>,
) -> Result<(aep_backend_markdown::backend::MarkdownBackend, PathBuf)> {
    let root = scratch_or(store)?;
    let backend = aep_backend_markdown::backend::MarkdownBackend::open(
        &root,
        std::iter::empty(),
        planning::clock_at_the_edge(),
        planning::command_actor()?,
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
    .with_context(|| format!("opening the markdown store at {}", root.display()))?;
    Ok((backend, root))
}

/// A conformance level by name, or every name it could have been.
fn conformance_level(level: &str) -> Result<aep_conformance::Level> {
    use aep_conformance::Level;

    Level::parse(level).with_context(|| {
        format!(
            "`{level}` is not a conformance level; expected one of {}",
            Level::ALL
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// The backend the suites run against, with `project` resolved to the store `project.yaml` names.
///
/// On a scratch instance of it: the suites write, and the question is whether the *kind* of store
/// the project chose conforms, not whether its plan survives being written into. For `postgres`
/// that is a schema of this process's own on the configured server, returned as the third element.
fn conformance_target(
    backend: ConformanceBackend,
    store: Option<&Path>,
) -> Result<ConformanceTarget> {
    let ConformanceBackend::Project = backend else {
        return Ok(ConformanceTarget {
            backend,
            store: store.map(Path::to_path_buf),
            scratch_schema: None,
            hybrid_policy: None,
        });
    };
    if let Some(path) = store {
        anyhow::bail!(
            "`--backend project` reads the store from `project.yaml`, so `--store {}` would be a \
             second answer; drop it, or name the backend",
            path.display()
        );
    }
    let mut target = ConformanceTarget {
        backend: ConformanceBackend::Markdown,
        store: None,
        scratch_schema: None,
        hybrid_policy: None,
    };
    match planning::Plan::discovered()? {
        planning::Plan::Markdown { .. } => {}
        planning::Plan::Sqlite { .. } => target.backend = ConformanceBackend::Sqlite,
        planning::Plan::Postgres { url } => {
            target.backend = ConformanceBackend::Postgres;
            target.store = Some(PathBuf::from(url));
            target.scratch_schema = Some(format!("aep_conformance_{}", std::process::id()));
        }
        planning::Plan::Hybrid { policy, .. } => {
            target.backend = ConformanceBackend::Hybrid;
            target.hybrid_policy = Some(policy);
        }
    }
    Ok(target)
}

/// What `conformance_target` resolved: the backend, where it lives, and what only some kinds carry.
struct ConformanceTarget {
    backend: ConformanceBackend,
    /// A directory, a file or a URL, as the backend reads it.
    store: Option<PathBuf>,
    /// A Postgres schema of this process's own, when the project's server is being borrowed.
    scratch_schema: Option<String>,
    /// The project's four words, for a hybrid.
    hybrid_policy: Option<aep_domain::project::HybridPolicy>,
}

/// The hybrid the suites run against: the markdown plan at `store` or a scratch directory, an
/// in-memory SQLite replica, under `policy` or — when none was configured — the markdown side as
/// the authority with divergences recorded. Returns the authority word for the report.
fn hybrid_conformance_backend(
    store: Option<&Path>,
    policy: Option<&aep_domain::project::HybridPolicy>,
) -> Result<(
    aep_backend_hybrid::HybridBackend<entity_sqlite::SqliteStore>,
    PathBuf,
    String,
)> {
    let root = scratch_or(store)?;
    let default = aep_domain::project::HybridPolicy {
        authority: "local".to_owned(),
        read: "local-first".to_owned(),
        on_unreachable: "refuse".to_owned(),
        on_divergence: "record".to_owned(),
    };
    let configured = policy.unwrap_or(&default);
    let policy =
        aep_backend_hybrid::policy_from(configured).map_err(|error| anyhow::anyhow!("{error}"))?;
    let authority = configured.authority.clone();
    let replica =
        entity_sqlite::SqliteStore::in_memory().map_err(|error| anyhow::anyhow!("{error}"))?;
    let backend = aep_backend_hybrid::HybridBackend::open(
        &root,
        replica,
        policy,
        std::iter::empty(),
        planning::clock_at_the_edge(),
        planning::command_actor()?,
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
    .with_context(|| format!("opening the hybrid plan at {}", root.display()))?;
    Ok((backend, root, authority))
}

/// `store`, or a scratch directory of this process's own, because the suites write.
fn scratch_or(store: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = store {
        return Ok(path.to_path_buf());
    }
    let scratch = std::env::temp_dir().join(format!("protocol-conformance-{}", std::process::id()));
    std::fs::create_dir_all(&scratch)
        .with_context(|| format!("creating a scratch store at {}", scratch.display()))?;
    Ok(scratch)
}

/// What is wrong with the `project.yaml` of the project this was run in, when there is one.
///
/// `protocol validate` is where a project's configuration is refused as a whole — a `store: hybrid`
/// missing one of its four policy words names the word here (`aep.project/1`, runtime R-106
/// enforced at our edge), rather than at the first verb that happened to open the plan. No project
/// found is no problem: the document tree is what was asked about.
fn project_file_problems() -> Vec<String> {
    let Ok(here) = std::env::current_dir() else {
        return Vec::new();
    };
    let Some(project) = aep_project::project::discover(&here) else {
        return Vec::new();
    };
    let path = project
        .join(aep_project::project::project_directory())
        .join(aep_domain::project::PROJECT_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => return vec![format!("{}: {error}", path.display())],
    };
    match aep_schema::parse::project(&text, Some(&path.display().to_string())) {
        Ok(_) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

/// Runs the suites against `backend`, with `fault` injected when one was asked for.
fn run_against<B: aep_conformance::Backend>(
    backend: B,
    fault: Option<aep_conformance::Fault>,
    level: aep_conformance::Level,
    suite: Option<&str>,
) -> Result<aep_conformance::ConformanceReport> {
    match fault {
        None => run_conformance(&backend, level, suite),
        Some(fault) => {
            let faulty = aep_conformance::FaultyBackend::new(backend, fault);
            run_conformance(&faulty, level, suite)
        }
    }
}

/// Runs a level, or one named suite within it.
fn run_conformance<B: aep_conformance::Backend>(
    backend: &B,
    level: aep_conformance::Level,
    suite: Option<&str>,
) -> Result<aep_conformance::ConformanceReport> {
    match suite {
        None => Ok(aep_conformance::run(backend, level)),
        Some(name) => {
            let report = aep_conformance::run_suite(backend, name).with_context(|| {
                format!(
                    "`{name}` is not a suite; known suites are {}",
                    aep_conformance::suites::all()
                        .iter()
                        .map(|suite| suite.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
            Ok(aep_conformance::ConformanceReport {
                level,
                suites: vec![report],
                ran_against: None,
            })
        }
    }
}

/// Parses a fault name, such as `replay-applies`.
fn parse_fault(name: &str) -> Result<aep_conformance::Fault> {
    // `replay-applies`, `replay_applies` and `ReplayApplies` all name the same fault; separators are
    // a spelling choice, not part of the name.
    let normalised = name.replace(['-', '_'], "").to_ascii_lowercase();
    aep_conformance::Fault::ALL
        .iter()
        .copied()
        .find(|fault| format!("{fault:?}").to_ascii_lowercase() == normalised)
        .with_context(|| {
            format!(
                "`{name}` is not a fault; known faults are {}",
                aep_conformance::Fault::ALL
                    .iter()
                    .map(|fault| kebab(&format!("{fault:?}")))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Renders a `CamelCase` name in kebab-case, for command-line use.
fn kebab(value: &str) -> String {
    let mut rendered = String::new();
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                rendered.push('-');
            }
            rendered.push(character.to_ascii_lowercase());
        } else {
            rendered.push(character);
        }
    }
    rendered
}

/// What can be done with a specification.
/// The observation time a verifier was given, or now.
pub(crate) fn observation_time(written: Option<&str>) -> Result<aep_domain::time::ObservedAt> {
    match written {
        Some(value) => {
            let at = aep_domain::time::CivilDate::parse(value)
                .map(aep_domain::CivilDate::to_timestamp)
                .or_else(|error| {
                    value
                        .parse::<u64>()
                        .map(aep_domain::time::Timestamp::from_epoch_millis)
                        .map_err(|_| error)
                })
                .with_context(|| {
                    format!("`{value}` is not a date such as 2026-08-30 or epoch milliseconds")
                })?;
            Ok(aep_domain::time::ObservedAt::new(at))
        }
        None => Ok(now_observed()),
    }
}

/// Reads the wall clock at the CLI edge as an observation time.
pub(crate) fn now_observed() -> aep_domain::time::ObservedAt {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    aep_domain::time::ObservedAt::new(aep_domain::time::Timestamp::from_epoch_millis(millis))
}

/// `protocol validate`
/// The claim `protocol validate --evidence` establishes.
///
/// A claim of its own rather than one of the eleven the convention lists (AGENTS.md § *Conventions*:
/// reuse before inventing), and the reason is that reusing `invariant` would be a false claim:
/// `verification.invariant.passed` is what `principles/verification/invariant-checking.yaml`,
/// `principles/development/design-by-contract.yaml` and `profiles/development-critical.yaml` read,
/// and none of them means *the document tree loads and cross-validates*. A validator that answered
/// those three by walking YAML would satisfy an obligation nobody discharged, which is the failure
/// mode the whole evidence programme exists to prevent. The claim is singular, as the convention
/// requires.
const DOCUMENT_TREE_VALID: &str = "document-tree-valid";

/// The verifier class `protocol validate` signs its record as.
///
/// An external tool named `protocol`, the same spelling `drivers/development/checks.yaml` already
/// uses for this binary's own verbs. `EvidenceKind::Verification::default_verifiers` names
/// `policy-engine` and `model-checker`, and this is neither: `default_verifiers` is a table of
/// defaults rather than of constraints, and claiming to be a model checker would be a stronger
/// statement about method than a document walk supports.
fn validator() -> aep_domain::verification::Verifier {
    aep_domain::verification::Verifier::ExternalTool(
        aep_domain::ids::ToolRef::new("protocol").expect("`protocol` is a tool reference"),
    )
}

/// Writes what a validation found as a `verification` evidence document.
///
/// Extracted from [`validate`] rather than inlined because the payload has one rule worth reading
/// on its own: the counterexamples are the problems **as the validator wrote them**, capped, and a
/// tree with no problems carries none. Nothing here re-words a refusal — a validation error that
/// reads differently in an evidence record than it does on the terminal is a second opinion about
/// the same walk.
fn write_validation_evidence(
    root: &Path,
    problems: &[String],
    format: Format,
    out: &Path,
) -> Result<()> {
    // Enough to act on, and not the whole list: a record is read by a person deciding what to fix,
    // and a tree with four hundred problems hands back four hundred lines of YAML nobody reads.
    // The count is not lost — `status` says it failed and the report on the terminal has them all.
    const NAMED: usize = 10;

    let counterexamples = problems
        .iter()
        .take(NAMED)
        .map(|problem| aep_domain::verification::Counterexample {
            verifier: validator(),
            property: aep_domain::ids::ClaimId::new(DOCUMENT_TREE_VALID).ok(),
            note: Some(problem.clone()),
            ..aep_domain::verification::Counterexample::default()
        })
        .collect();

    let record = aep_domain::evidence::VerificationRecord {
        claim: aep_domain::ids::ClaimId::new(DOCUMENT_TREE_VALID)
            .context("`document-tree-valid` is a claim id")?,
        verifier: validator(),
        status: if problems.is_empty() {
            aep_domain::verification::VerificationStatus::Passed
        } else {
            aep_domain::verification::VerificationStatus::Failed
        },
        subject: None,
        counterexamples,
    };

    let minted = evidence_doc::MintedEvidence::new(
        aep_domain::evidence::Evidence::Verification(record),
        validator(),
        // The walk happened in this process, in this second, which is the one case where a default
        // of *now* is the honest value rather than a freshness claim nobody made.
        now_observed(),
    )
    .obtained_by(format!(
        "protocol validate --root {} --evidence {}",
        root.display(),
        out.display()
    ))
    .reading(root.display().to_string());

    evidence_doc::emit(&minted, format, Some(out))
}

fn validate(
    root: &Path,
    artifacts: Option<&Path>,
    format: Format,
    evidence: Option<&Path>,
) -> Result<ExitCode> {
    let outcome = load_tree_report(root);
    let mut problems: Vec<String> = outcome.failures.iter().map(ToString::to_string).collect();
    problems.extend(project_file_problems());

    if let Some(path) = artifacts {
        let graph = read_artifacts(path)?;
        let lifecycle_errors = graph.validate_lifecycles(outcome.registry.lifecycles());
        problems.extend(lifecycle_errors.as_slice().iter().map(ToString::to_string));
    }

    let summary = Summary {
        files_read: outcome.files_read,
        protocols: outcome.registry.protocols().count(),
        principles: outcome.registry.principles().count(),
        workflows: outcome.registry.workflows().count(),
        profiles: outcome.registry.profiles().count(),
        lifecycles: outcome.registry.lifecycles().len(),
        step_maps: outcome.drivers.len(),
        problems: problems.clone(),
    };

    match format {
        Format::Text => {
            outln!(
                "{} file(s): {} protocol(s), {} principle(s), {} workflow(s), {} profile(s), {} \
                 lifecycle(s), {} step map(s)",
                summary.files_read,
                summary.protocols,
                summary.principles,
                summary.workflows,
                summary.profiles,
                summary.lifecycles,
                summary.step_maps
            );
            if problems.is_empty() {
                outln!("valid");
            } else {
                outln!("{} problem(s):", problems.len());
                for problem in &problems {
                    outln!("  - {problem}");
                }
            }
        }
        Format::Yaml | Format::Json => print_serialised(&summary, format)?,
    }

    if let Some(out) = evidence {
        // Written whatever the verdict, and before the exit code is decided: a step that declares
        // `record:` has its document read and its exit status ignored, so a validator that wrote no
        // record on a red tree would leave the run with nothing observed instead of with a `failed`
        // record naming what it found.
        write_validation_evidence(root, &problems, format, out)?;
    }

    Ok(exit_code(problems.is_empty()))
}

/// What `validate` reports.
#[derive(Debug, serde::Serialize)]
struct Summary {
    files_read: usize,
    protocols: usize,
    principles: usize,
    workflows: usize,
    profiles: usize,
    lifecycles: usize,
    step_maps: usize,
    problems: Vec<String>,
}

/// What an execution needs, from flags or from the project the command was run in.
struct Inputs {
    registry: Registry,
    task: Task,
    artifacts: ArtifactGraph,
    /// Where these came from, for a report.
    origin: String,
}

/// Resolves execution inputs: explicit flags first, then the project this was run in.
///
/// The order matters. A flag is an instruction; discovery is a convenience. Silently preferring the
/// project would make `--task other.yaml` do something other than what it says.
fn inputs(args: &ExecutionArgs) -> Result<Inputs> {
    if let (Some(task), root) = (&args.task, &args.root) {
        let root = root.clone().unwrap_or_else(|| PathBuf::from("."));
        let registry = load(&root)?;
        let artifacts = match &args.artifacts {
            Some(path) => read_artifacts(path)?,
            None => ArtifactGraph::new(),
        };
        return Ok(Inputs {
            registry,
            task: read_task(task)?,
            artifacts,
            origin: format!("{} and {}", root.display(), task.display()),
        });
    }

    let here = std::env::current_dir().context("reading the working directory")?;
    let directory = aep_project::project::project_directory();
    let root = aep_project::project::discover(&here).with_context(|| {
        format!(
            "no `{directory}/project.yaml` in {} or any parent, and no --task was given",
            here.display()
        )
    })?;
    let project =
        aep_project::project::load(&root).map_err(|errors| anyhow::anyhow!("{errors}"))?;

    // A flag still overrides what the project says, so a one-off run needs no edit to the project.
    let task = match &args.task {
        Some(path) => read_task(path)?,
        None => project
            .require_task()
            .map_err(|reason| anyhow::anyhow!("{reason}"))?
            .clone(),
    };
    let artifacts = match &args.artifacts {
        Some(path) => read_artifacts(path)?,
        None => project.artifacts,
    };

    Ok(Inputs {
        registry: project.registry,
        task,
        artifacts,
        origin: format!("project {}", root.display()),
    })
}

/// `protocol resolve`
fn resolve(args: &ExecutionArgs) -> Result<ExitCode> {
    let Inputs {
        registry,
        task,
        origin,
        ..
    } = inputs(args)?;
    let plan = aep_engine::resolve(&task, &registry)
        .map_err(|errors| anyhow::anyhow!("{errors}"))
        .context("the task cannot be resolved")?;

    match args.format {
        Format::Text => {
            outln!("inputs      {origin}");
            outln!("task        {} ({})", plan.task.id, plan.task.kind);
            outln!("objective   {}", plan.task.objective);
            outln!("protocol    {}", plan.protocol.reference());
            outln!("profile     {}", plan.profile.id);
            outln!(
                "workflow    {} (initial: {})",
                plan.workflow.id,
                plan.workflow.initial
            );
            outln!(
                "principles  {}",
                plan.principles
                    .iter()
                    .map(|principle| principle.id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !plan.dropped_principles.is_empty() {
                outln!(
                    "dropped     {}",
                    plan.dropped_principles
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            outln!("obligations {}", plan.obligations.len());
            outln!("capabilities");
            for (capability, decision) in plan.capability_summary() {
                // `Display` for the decision writes directly, which ignores a width specifier, so
                // the padding has to happen on an owned string.
                outln!("  {:<18} {capability}", decision.to_string());
            }
        }
        Format::Yaml | Format::Json => print_serialised(&plan, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol inspect`
fn inspect(root: &Path, reference: Option<&str>, format: Format) -> Result<ExitCode> {
    let registry = load(root)?;

    let Some(reference) = reference else {
        for protocol in registry.protocols() {
            outln!("protocol   {}", protocol.reference());
        }
        for principle in registry.principles() {
            outln!("principle  {}  {}", principle.id, principle.title);
        }
        for workflow in registry.workflows() {
            outln!("workflow   {}  {}", workflow.id, workflow.title);
        }
        for profile in registry.profiles() {
            outln!("profile    {}  {}", profile.id, profile.title);
        }
        return Ok(ExitCode::SUCCESS);
    };

    if let Ok(protocol_ref) = reference.parse() {
        if let Ok(protocol) = registry.resolved_protocol(&protocol_ref) {
            return print_document(&protocol, format).map(|()| ExitCode::SUCCESS);
        }
    }
    if let Ok(principle_ref) = reference.parse() {
        if let Some(principle) = registry.principle(&principle_ref) {
            return print_document(principle, format).map(|()| ExitCode::SUCCESS);
        }
    }
    if let Ok(workflow_ref) = reference.parse() {
        if let Some(workflow) = registry.workflow(&workflow_ref) {
            return print_document(workflow, format).map(|()| ExitCode::SUCCESS);
        }
    }
    if let Ok(profile_ref) = reference.parse() {
        if let Ok(profile) = registry.resolved_profile(&profile_ref) {
            return print_document(&profile, format).map(|()| ExitCode::SUCCESS);
        }
    }

    bail!("nothing in {} declares `{reference}`", root.display())
}

/// `protocol evaluate` and `protocol explain`
fn evaluate(args: &ExecutionArgs, action: Option<&str>) -> Result<ExitCode> {
    let Inputs {
        registry,
        task,
        artifacts,
        origin,
    } = inputs(args)?;

    let engine = Engine::new(registry);
    let mut execution = match &args.state {
        Some(path) => {
            let text =
                fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
            let snapshot = serde_yaml::from_str(&text)
                .with_context(|| format!("parsing the snapshot in {}", path.display()))?;
            engine
                .restore(task, artifacts, snapshot)
                .context("restoring the execution")?
        }
        None => engine
            .initialize_with_artifacts(task, artifacts)
            .map_err(|error| anyhow::anyhow!("{error}"))
            .context("initialising the execution")?,
    };

    // One record two hours ahead used to discard the other 214: the first failure propagated with
    // `?` and the run produced no evaluation at all. A future observation is still a refusal — it
    // is invariant 7 and nothing here downgrades it to a warning — but it refuses *that record*,
    // by file position and by the date as written, and the rest of the document is still
    // submitted. Every other refusal the engine can make is a fact about the document as a whole
    // and still stops the run.
    let mut refused: Vec<String> = Vec::new();
    for path in &args.evidence {
        let origin = path.display().to_string();
        for (ordinal, submission) in read_evidence(path)?.into_iter().enumerate() {
            let observed_at = submission.observed_at;
            match engine.submit_evidence(&mut execution, submission) {
                Ok(_) => {}
                Err(aep_engine::error::ProtocolError::ObservationInFuture { now, .. }) => {
                    refused.push(future_observation_refusal(
                        &origin,
                        ordinal + 1,
                        observed_at,
                        now,
                    ));
                }
                Err(error) => {
                    return Err(anyhow::anyhow!("{error}"))
                        .with_context(|| format!("submitting evidence from {origin}"));
                }
            }
        }
    }

    if args.advance {
        // Advance until nothing more can move, stopping at the first state seen twice. A workflow
        // with a back-edge — `verify -> implement` in `adp/default` — would otherwise ping-pong
        // until the loop bound, which looks like progress and is not: no evidence arrives in here,
        // so the second visit can only repeat the first.
        let mut seen = vec![execution.state_id().clone()];
        while let Ok(TransitionResult::Moved { to, .. }) = engine.transition(&mut execution) {
            if seen.contains(&to) {
                break;
            }
            seen.push(to);
        }
    }

    if let Some(action) = action {
        let request = action_request(action)?;
        let decision = engine.authorize(&mut execution, &request);
        let explanation = Engine::<aep_engine::SystemClock>::explain_decision(&decision);
        match args.format {
            Format::Text => outln!("{explanation}"),
            Format::Yaml | Format::Json => print_serialised(&explanation, args.format)?,
        }
        report_refusals(&refused);
        return Ok(exit_code(decision.is_allowed() && refused.is_empty()));
    }

    let evaluation = engine.evaluate(&execution);
    match args.format {
        Format::Text => {
            outln!("inputs      {origin}");
            outln!(
                "state       {} ({})",
                evaluation.state,
                evaluation.state_title
            );
            if !evaluation.requirements.is_empty() {
                outln!("owed here");
                for requirement in &evaluation.requirements {
                    outln!("  {}", requirement.line());
                }
            }
            outln!("transitions");
            if evaluation.transitions.is_empty() {
                outln!("  (none: this state is terminal)");
            }
            for transition in &evaluation.transitions {
                let mark = if transition.permitted {
                    "permitted"
                } else {
                    "blocked"
                };
                outln!("  {} -> {} [{mark}]", evaluation.state, transition.to);
                for reason in transition.unmet() {
                    outln!("      {reason}");
                }
            }
            outln!("{}", engine.explain_completion(&execution));
        }
        Format::Yaml | Format::Json => print_serialised(&evaluation, args.format)?,
    }

    // Exit 0: the report was produced. Whether the execution is blocked is in the report, and a
    // harness that wants to branch on it reads `blocked` or `is_complete` from the JSON — a blocked
    // execution is the normal case, not an error.
    //
    // A refused record is not that case. The evaluation is still printed, because an evaluation
    // missing one fact is worth more than no evaluation at all, and the exit code still says the
    // run did not read everything it was given.
    report_refusals(&refused);
    Ok(exit_code(refused.is_empty()))
}

/// Prints per-record refusals where a person reading a report will still see them.
fn report_refusals(refusals: &[String]) {
    for refusal in refusals {
        eprintln!("{refusal}");
    }
}

/// `protocol entity`
fn entity(command: &EntityCommand) -> Result<ExitCode> {
    match command {
        EntityCommand::List {
            backend,
            entity_type,
        } => entity_list(backend, entity_type.as_deref()),
        EntityCommand::Get { backend, reference } => entity_get(backend, reference),
        EntityCommand::History { backend, reference } => entity_history(backend, reference),
        EntityCommand::Relations {
            backend,
            reference,
            incoming,
        } => entity_relations(backend, reference, *incoming),
    }
}

/// `protocol entity list`
fn entity_list(args: &BackendArgs, entity_type: Option<&str>) -> Result<ExitCode> {
    let backend = seeded(args)?;

    let mut query = EntityQuery::default();
    if let Some(name) = entity_type {
        query.entity_type = Some(name.parse().map_err(|error| anyhow::anyhow!("{error}"))?);
    }
    let page = block_on(backend.query(&query)).map_err(|error| anyhow::anyhow!("{error}"))?;

    match args.format {
        Format::Text => print_table(
            &page
                .items
                .iter()
                .map(|entity| {
                    vec![
                        entity.metadata.id.to_string(),
                        entity.metadata.entity_type.to_string(),
                        entity.metadata.locator.to_string(),
                        format!("r{}", entity.metadata.revision),
                    ]
                })
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => print_serialised(&page.items, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol entity get`
fn entity_get(args: &BackendArgs, reference: &str) -> Result<ExitCode> {
    let backend = seeded(args)?;
    let target = resolve_entity(&backend, reference)?;
    let entity = block_on(backend.get(&target, QueryConsistency::Current))
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    match args.format {
        Format::Text => {
            outln!("id         {}", entity.metadata.id);
            outln!("type       {}", entity.metadata.entity_type);
            outln!("locator    {}", entity.metadata.locator);
            outln!("revision   {}", entity.metadata.revision);
            outln!(
                "created    {} by {}",
                entity.metadata.created_at,
                entity.metadata.provenance.created_by
            );
            outln!("body");
            let body = serde_yaml::to_string(&entity.data).context("rendering the body")?;
            for line in body.lines() {
                outln!("  {line}");
            }
        }
        Format::Yaml | Format::Json => print_serialised(&entity, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol entity history`
fn entity_history(args: &BackendArgs, reference: &str) -> Result<ExitCode> {
    let backend = seeded(args)?;
    let target = resolve_entity(&backend, reference)?;
    let history = block_on(backend.history(&target)).map_err(|error| anyhow::anyhow!("{error}"))?;

    match args.format {
        Format::Text => print_table(
            &history
                .iter()
                .map(|record| {
                    vec![
                        format!("r{}", record.revision),
                        record.at.to_string(),
                        record.actor.to_string(),
                        record
                            .command_id
                            .as_ref()
                            .map_or_else(|| "-".to_owned(), ToString::to_string),
                    ]
                })
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => print_serialised(&history, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol entity relations`
fn entity_relations(args: &BackendArgs, reference: &str, incoming: bool) -> Result<ExitCode> {
    let backend = seeded(args)?;
    let target = resolve_entity(&backend, reference)?;

    let query = if incoming {
        RelationQuery::to(target)
    } else {
        RelationQuery::from(target)
    };
    let page = block_on(backend.relations(&query)).map_err(|error| anyhow::anyhow!("{error}"))?;

    match args.format {
        Format::Text => {
            let mut rows = Vec::new();
            for relation in &page.items {
                // The other end, since one end is what was asked about.
                let other = if incoming {
                    &relation.source
                } else {
                    &relation.target
                };
                rows.push(vec![
                    relation.kind.to_string(),
                    if incoming { "<-" } else { "->" }.to_owned(),
                    other.id.to_string(),
                    locator_of(&backend, other),
                ]);
            }
            print_table(&rows);
        }
        Format::Yaml | Format::Json => print_serialised(&page.items, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol audit`
fn audit(
    args: &BackendArgs,
    correlation: Option<&str>,
    entity: Option<&str>,
    rejected: bool,
) -> Result<ExitCode> {
    let backend = seeded(args)?;

    let mut query = AuditQuery {
        rejected_only: rejected,
        ..AuditQuery::default()
    };
    if let Some(correlation) = correlation {
        query.correlation_id = Some(
            correlation
                .parse()
                .map_err(|error| anyhow::anyhow!("{error}"))?,
        );
    }
    if let Some(entity) = entity {
        query.entity = Some(resolve_entity(&backend, entity)?);
    }
    let page = block_on(backend.audit(&query)).map_err(|error| anyhow::anyhow!("{error}"))?;

    match args.format {
        Format::Text => print_table(
            &page
                .items
                .iter()
                .map(|record| {
                    vec![
                        record.audit_id.to_string(),
                        record.kind.as_str().to_owned(),
                        record.occurred_at.to_string(),
                        record.actor.to_string(),
                        record
                            .command_id
                            .as_ref()
                            .map_or_else(|| "-".to_owned(), ToString::to_string),
                        record
                            .subject
                            .as_ref()
                            .map_or_else(|| "-".to_owned(), |subject| subject.id.to_string()),
                    ]
                })
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => print_serialised(&page.items, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol describe`
fn describe(args: &BackendArgs, entity_type: &str) -> Result<ExitCode> {
    let backend = seeded(args)?;
    let entity_type = entity_type
        .parse()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let descriptor = block_on(backend.describe_type(&entity_type))
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    match args.format {
        Format::Text => {
            outln!("type       {}", descriptor.entity_type);
            outln!("summary    {}", descriptor.summary);
            outln!(
                "mutable    {}",
                if descriptor.mutable { "yes" } else { "no" }
            );
            if !descriptor.commands.is_empty() {
                outln!("commands");
                for command in &descriptor.commands {
                    let guard = if command.revision_guarded {
                        "revision-guarded"
                    } else {
                        "unguarded"
                    };
                    outln!(
                        "  {:<28} {guard:<17} {}",
                        command.command_type,
                        command.summary
                    );
                }
            }
            if !descriptor.relations.is_empty() {
                outln!("relations");
                for relation in &descriptor.relations {
                    outln!("  {}", relation.kind);
                }
            }
        }
        Format::Yaml | Format::Json => print_serialised(&descriptor, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Seeds an in-memory backend from the manifest, so there is something to answer about.
///
/// Every invocation starts from nothing: the backend keeps no state between runs, which is why the
/// seeding is visible in the history and the audit trail rather than hidden.
fn seeded(args: &BackendArgs) -> Result<MemoryBackend> {
    let (source, graph) = match (&args.artifacts, &args.planning) {
        (Some(manifest), None) => {
            let path = resolved(&args.root, manifest);
            let graph = read_artifacts(&path)?;
            (path, graph)
        }
        (None, Some(store)) => {
            let path = resolved(&args.root, store);
            let graph = planning::graph_at(&path)?;
            (path, graph)
        }
        // Unreachable through the command line: the `artifact-source` group makes clap refuse both
        // and neither. Spelled out rather than `unreachable!`, because the group and this match are
        // two declarations of one rule and nothing checks that they agree.
        _ => bail!(
            "give exactly one of `--artifacts <manifest>` and `--planning <store>`: the backend \
             is in-memory, so it needs one source of artifacts and cannot merge two"
        ),
    };

    let actor: ActorRef = SEED_ACTOR
        .parse()
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    let backend = MemoryBackend::new();
    seed::from_manifest(
        &backend,
        &graph,
        &args.organisation,
        &args.space,
        SEED_AT,
        &actor,
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
    .with_context(|| format!("seeding the backend from {}", source.display()))?;
    Ok(backend)
}

/// Resolves a relative path against the document tree, and leaves an absolute one alone.
fn resolved(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

/// Resolves a locator or a raw identity to an entity that exists.
///
/// Both spellings are accepted because both are how people arrive: a locator is what an
/// organisation knows a thing by, an identity is what a previous command printed.
fn resolve_entity(backend: &MemoryBackend, reference: &str) -> Result<EntityRef> {
    if let Ok(locator) = reference.parse::<EntityLocator>() {
        return match block_on(backend.resolve(&locator)) {
            Ok(id) => Ok(EntityRef::new(id)),
            Err(_) => bail!("nothing seeded from this manifest is addressed by `{reference}`"),
        };
    }
    if let Ok(id) = reference.parse::<EntityId>() {
        let target = EntityRef::new(id);
        if block_on(backend.get(&target, QueryConsistency::Current)).is_ok() {
            return Ok(target);
        }
        bail!("no entity seeded from this manifest has the identity `{reference}`");
    }
    bail!("`{reference}` is neither a locator (`ep://…`) nor an entity identity")
}

/// The locator an entity is addressed by, for output that names both ends of an edge.
fn locator_of(backend: &MemoryBackend, reference: &EntityRef) -> String {
    block_on(backend.get(reference, QueryConsistency::Current)).map_or_else(
        |_| "-".to_owned(),
        |entity| entity.metadata.locator.to_string(),
    )
}

/// Prints one record per line, in columns wide enough for the widest cell.
///
/// Aligned because the surface exists to be scanned: a reader looking for one design among sixty
/// entities finds it by column position, not by reading every line.
fn print_table(rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = Vec::new();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            let width = cell.chars().count();
            match widths.get_mut(index) {
                Some(current) => *current = (*current).max(width),
                None => widths.push(width),
            }
        }
    }

    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                if index + 1 == row.len() {
                    cell.clone()
                } else {
                    format!("{cell:width$}", width = widths[index])
                }
            })
            .collect();
        outln!("{}", cells.join("  "));
    }
}

/// Loads a document tree, failing if anything is wrong with it.
fn load(root: &Path) -> Result<Registry> {
    Ok(load_documents(root)?.registry)
}

/// Loads both semantic documents and edge-owned harness projections.
fn load_documents(root: &Path) -> Result<aep_project::load::LoadOutcome> {
    let outcome = load_tree_report(root);
    if outcome.failures.is_empty() {
        return Ok(outcome);
    }
    let detail = outcome
        .failures
        .iter()
        .map(|failure| format!("  - {failure}"))
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "{} document problem(s) in {}:\n{detail}",
        outcome.failures.len(),
        root.display()
    )
}

/// Reads a task document.
pub(crate) fn read_task(path: &Path) -> Result<Task> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let origin = path.display().to_string();
    aep_schema::parse::task(&text, Some(&origin)).map_err(|error| anyhow::anyhow!("{error}"))
}

/// Reads an artifact manifest.
fn read_artifacts(path: &Path) -> Result<ArtifactGraph> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let origin = path.display().to_string();
    aep_schema::parse::artifact_manifest(&text, Some(&origin))
        .map_err(|error| anyhow::anyhow!("{error}"))
}

/// Reads a list of evidence submissions.
fn read_evidence(path: &Path) -> Result<Vec<EvidenceSubmission>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let origin = path.display().to_string();
    let inputs = aep_schema::parse::evidence_list(&text, Some(&origin))
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(inputs.into_iter().map(submission).collect())
}

/// Turns a parsed evidence input into a submission.
fn submission(input: aep_schema::parse::EvidenceInput) -> EvidenceSubmission {
    let mut submission = EvidenceSubmission::new(input.evidence, input.producer, input.observed_at);
    submission.subject = input.about;
    if let Some(provenance) = input.provenance {
        submission.provenance = provenance;
    }
    submission
}

/// Builds a stand-in action for a capability named on the command line.
///
/// `explain --action` asks about a *capability*, so the CLI wraps it in the simplest action that
/// requires it. The decision depends on the capability, not on the action's details.
fn action_request(capability: &str) -> Result<ActionRequest> {
    let capability: Capability = capability
        .parse()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let action = match &capability {
        Capability::RepositoryRead => Action::RepositoryRead(aep_domain::action::RepositoryRead {
            paths: vec![".".to_owned()],
        }),
        Capability::RepositoryWrite => {
            Action::RepositoryWrite(aep_domain::action::RepositoryWrite {
                paths: vec![".".to_owned()],
                intent: None,
            })
        }
        Capability::TestExecution => Action::TestExecute(aep_domain::action::TestExecute {
            suite: aep_domain::evidence::TestSuite::Unit,
            selector: None,
        }),
        Capability::CommandExecution => {
            Action::CommandExecute(aep_domain::action::CommandExecute {
                program: "true".to_owned(),
                args: Vec::new(),
            })
        }
        Capability::NetworkRead(_) | Capability::NetworkWrite => {
            Action::NetworkRequest(aep_domain::action::NetworkRequest {
                url: "https://example.test/".to_owned(),
                intent: if capability == Capability::NetworkWrite {
                    aep_domain::action::NetworkIntent::Write
                } else {
                    aep_domain::action::NetworkIntent::Read
                },
            })
        }
        Capability::TelemetryRead => Action::TelemetryQuery(aep_domain::action::TelemetryQuery {
            query: "up".to_owned(),
            service: None,
        }),
        Capability::ProductionRead | Capability::ProductionWrite => {
            Action::ProductionMutate(ProductionMutate {
                target: "state".to_owned(),
                change: None,
            })
        }
        Capability::Deploy(environment) => Action::Deploy(aep_domain::action::Deploy {
            environment: environment.clone(),
            revision: "HEAD".to_owned(),
            strategy: None,
        }),
        Capability::Rollback(environment) => Action::Rollback(aep_domain::action::Rollback {
            environment: environment.clone(),
            to_revision: None,
        }),
        Capability::SecretRead => Action::SecretRead(aep_domain::action::SecretRead {
            secret: "secret".to_owned(),
        }),
        Capability::ArtifactRead | Capability::ArtifactWrite => {
            Action::ArtifactWrite(aep_domain::action::ArtifactWrite {
                artifact: "design:example"
                    .parse()
                    .map_err(|error| anyhow::anyhow!("{error}"))?,
                kind: aep_domain::artifact::ArtifactKind::Design,
            })
        }
        Capability::ReviewRequest => Action::ReviewRequest(aep_domain::action::ReviewRequest {
            subject: "design:example"
                .parse()
                .map_err(|error| anyhow::anyhow!("{error}"))?,
            reviewer: None,
        }),
        Capability::ApprovalRequest => {
            Action::ApprovalRequest(aep_domain::action::ApprovalRequest {
                approval: "production-change"
                    .parse()
                    .map_err(|error| anyhow::anyhow!("{error}"))?,
                reason: None,
            })
        }
        other => bail!("`{other}` cannot be asked about directly yet"),
    };
    Ok(ActionRequest::new(action))
}

/// Prints a validated document in the requested format.
fn print_document<T: serde::Serialize>(document: &T, format: Format) -> Result<()> {
    match format {
        Format::Text | Format::Yaml => {
            out!(
                "{}",
                serde_yaml::to_string(document).context("rendering the document")?
            );
        }
        Format::Json => print_serialised(document, format)?,
    }
    Ok(())
}

/// Prints a value as YAML or JSON.
fn print_serialised<T: serde::Serialize>(value: &T, format: Format) -> Result<()> {
    match format {
        Format::Json => {
            outln!(
                "{}",
                serde_json::to_string_pretty(value).context("rendering as JSON")?
            );
        }
        _ => out!(
            "{}",
            serde_yaml::to_string(value).context("rendering as YAML")?
        ),
    }
    Ok(())
}

/// The instant a future-observation refusal is decided against.
///
/// Two verbs read one evidence document and must answer identically about it, so both put the
/// caller's `observed_at` to [`aep_domain::time::ObservedAt::is_after`] — the engine's own
/// comparison — rather than each carrying a rule of its own. What differs is only which instant
/// they compare against:
///
/// * **no `--at`**: the wall clock, which is exactly what `protocol evaluate` submits against;
/// * **`--at <day>`**: the last millisecond of that day. A pinned day is a what-if, and the
///   permissive end of it is the one that keeps the verb's primary use working — reading a record
///   the day it was written, whose instant is somewhere inside the day, not at its first
///   millisecond.
fn future_reference(written: Option<&str>, day: aep_domain::time::CivilDate) -> Timestamp {
    /// One day, less a millisecond: the last instant that belongs to a pinned day.
    const LAST_MILLISECOND_OF_A_DAY: u64 = 86_400_000 - 1;

    match written {
        Some(_) => Timestamp::from_epoch_millis(
            day.to_timestamp()
                .epoch_millis()
                .saturating_add(LAST_MILLISECOND_OF_A_DAY),
        ),
        None => now_observed().timestamp(),
    }
}

/// One record's refusal, in the words both verbs print.
///
/// The position is the point. The message an adopter pasted named an epoch pair and nothing else,
/// so the record it was about was not identifiable among 215 — and the file it came from produced
/// no evaluation at all. This names the file, which record in it, and the observation *as the
/// caller wrote it*.
fn future_observation_refusal(
    origin: &str,
    ordinal: usize,
    observed_at: aep_domain::time::ObservedAt,
    now: Timestamp,
) -> String {
    format!(
        "{origin}: record {ordinal}: the observation time {observed_at} has not happened yet; \
         the clock reads {} ({now})",
        now.iso_8601()
    )
}

/// `0` when the answer is yes, `1` when it is no.
/// The day a report classifies against: what was asked for, or today.
fn observation_day(written: Option<&str>) -> Result<aep_domain::time::CivilDate> {
    match written {
        Some(value) => aep_domain::time::CivilDate::parse(value)
            .with_context(|| format!("`{value}` is not a date such as 2026-09-01")),
        None => Ok(aep_domain::time::CivilDate::from_timestamp(
            now_observed().timestamp(),
        )),
    }
}

/// One document's coverage and its records, ready to render.
#[derive(serde::Serialize)]
struct ScannedFile {
    file: String,
    raw_occurrences: usize,
    records: usize,
    divergence: usize,
    claims: Vec<ScannedClaim>,
    rejections: Vec<ScannedRejection>,
}

#[derive(serde::Serialize)]
struct ScannedClaim {
    line: usize,
    date: String,
    horizon: String,
    malformed: bool,
    state: &'static str,
    days: u32,
    claim: String,
}

#[derive(serde::Serialize)]
struct ScannedRejection {
    line: usize,
    reason: String,
    text: String,
}

#[derive(serde::Serialize)]
struct ScanReport {
    at: String,
    warn_days: u32,
    files: Vec<ScannedFile>,
    totals: ScanTotals,
}

#[derive(serde::Serialize)]
struct ScanTotals {
    raw_occurrences: usize,
    records: usize,
    divergence: usize,
    ok: usize,
    expiring: usize,
    expired: usize,
    malformed: usize,
}

/// `protocol evidence scan`
fn evidence_scan(
    paths: &[PathBuf],
    at: aep_domain::time::CivilDate,
    warn_days: u32,
    strict: bool,
    fail_on_expired: bool,
    format: Format,
) -> Result<ExitCode> {
    let mut files = Vec::new();
    let mut totals = ScanTotals {
        raw_occurrences: 0,
        records: 0,
        divergence: 0,
        ok: 0,
        expiring: 0,
        expired: 0,
        malformed: 0,
    };

    for path in markdown_files(paths)? {
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let scan = aep_backend_markdown::claim::scan_at(&text, at);
        let mut claims = Vec::new();
        for record in &scan.records {
            let state = record.state(at, warn_days);
            match state {
                aep_backend_markdown::claim::ClaimState::Ok { .. } => totals.ok += 1,
                aep_backend_markdown::claim::ClaimState::Expiring { .. } => totals.expiring += 1,
                aep_backend_markdown::claim::ClaimState::Expired { .. } => totals.expired += 1,
            }
            if record.malformed {
                totals.malformed += 1;
            }
            claims.push(ScannedClaim {
                line: record.line,
                date: record.date.to_string(),
                horizon: record.horizon.to_string(),
                malformed: record.malformed,
                state: state.label(),
                days: state.days(),
                claim: record.claim.clone(),
            });
        }
        totals.raw_occurrences += scan.raw_occurrences;
        totals.records += scan.records.len();
        totals.divergence += scan.divergence();
        files.push(ScannedFile {
            file: path.display().to_string(),
            raw_occurrences: scan.raw_occurrences,
            records: scan.records.len(),
            divergence: scan.divergence(),
            claims,
            rejections: scan
                .rejections
                .iter()
                .map(|rejection| ScannedRejection {
                    line: rejection.line,
                    reason: rejection.reason.to_string(),
                    text: rejection.text.clone(),
                })
                .collect(),
        });
    }

    let report = ScanReport {
        at: at.to_string(),
        warn_days,
        files,
        totals,
    };

    match format {
        Format::Json => outln!(
            "{}",
            serde_json::to_string_pretty(&report).context("rendering the scan")?
        ),
        Format::Yaml => out!(
            "{}",
            serde_yaml::to_string(&report).context("rendering the scan")?
        ),
        Format::Text => render_scan(&report),
    }

    let blind = strict && report.totals.divergence > 0;
    let stale = fail_on_expired && report.totals.expired > 0;
    Ok(exit_code(!blind && !stale))
}

/// The human rendering: one line per claim, and the coverage line per file.
fn render_scan(report: &ScanReport) {
    for file in &report.files {
        outln!(
            "{} — {} record(s) from {} occurrence(s){}",
            file.file,
            file.records,
            file.raw_occurrences,
            if file.divergence == 0 {
                String::new()
            } else {
                format!(", {} NOT PARSED", file.divergence)
            }
        );
        for claim in &file.claims {
            outln!(
                "  {:>4}  {:8} {:>3}d  {} {}{}",
                claim.line,
                claim.state,
                claim.days,
                claim.date,
                if claim.malformed { "[malformed] " } else { "" },
                truncated(&claim.claim)
            );
        }
        for rejection in &file.rejections {
            outln!("  {:>4}  refused  {}", rejection.line, rejection.reason);
        }
    }
    let totals = &report.totals;
    outln!(
        "{} occurrence(s), {} record(s), {} unparsed — {} ok, {} expiring, {} expired, {} malformed (at {})",
        totals.raw_occurrences,
        totals.records,
        totals.divergence,
        totals.ok,
        totals.expiring,
        totals.expired,
        totals.malformed,
        report.at
    );
}

/// A claim, short enough for one terminal line.
fn truncated(claim: &str) -> String {
    const LIMIT: usize = 68;
    if claim.chars().count() <= LIMIT {
        return claim.to_owned();
    }
    let kept: String = claim.chars().take(LIMIT - 1).collect();
    format!("{kept}…")
}

/// Every markdown file named, expanding a directory one level.
fn markdown_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = fs::read_dir(path)
                .with_context(|| format!("reading {}", path.display()))?
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|entry| entry.extension().is_some_and(|kind| kind == "md"))
                .collect();
            found.sort();
            files.extend(found);
            continue;
        }
        files.push(path.clone());
    }
    Ok(files)
}

/// One submitted record, aged.
#[derive(serde::Serialize)]
struct InspectedRecord {
    file: String,
    kind: String,
    observed_at: String,
    age_days: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<&'static str>,
    producer: String,
}

/// `protocol evidence inspect`
///
/// Two references, and they answer different questions. `at` is the day the report **ages**
/// against, so a horizon boundary reads in whole days exactly as a document scanner reads it.
/// `reference` is the instant a **future** observation is refused against, and it is the engine's
/// own comparison so that this verb and `protocol evaluate` cannot disagree about one file.
fn evidence_inspect(
    paths: &[PathBuf],
    at: aep_domain::time::CivilDate,
    reference: Timestamp,
    horizon: Option<&str>,
    format: Format,
) -> Result<ExitCode> {
    let horizon = match horizon {
        Some(written) => Some(
            aep_domain::time::Horizon::parse(written)
                .with_context(|| format!("`{written}` is not a horizon such as `7d`"))?,
        ),
        None => None,
    };
    let now = at.to_timestamp();

    let mut records = Vec::new();
    let mut future = Vec::new();
    for path in paths {
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let origin = path.display().to_string();
        let inputs = aep_schema::parse::evidence_list(&text, Some(&origin))?;
        for (ordinal, input) in inputs.into_iter().enumerate() {
            let observed = input.observed_at;
            // The engine's comparison, made here so the two verbs cannot answer differently about
            // one document: a calendar date is refused only once it has begun in no timezone, an
            // epoch value is compared exactly, and the record that fails is named.
            if observed.is_after(reference) {
                future.push(future_observation_refusal(
                    &origin,
                    ordinal + 1,
                    observed,
                    reference,
                ));
            }
            records.push(InspectedRecord {
                file: origin.clone(),
                kind: input.evidence.kind().to_string(),
                observed_at: observed.day().to_string(),
                age_days: observed.age_days(now),
                state: horizon.map(|horizon| {
                    if horizon.covers(observed.timestamp(), now) {
                        "ok"
                    } else {
                        "expired"
                    }
                }),
                producer: input.producer.to_string(),
            });
        }
    }

    match format {
        Format::Json => outln!(
            "{}",
            serde_json::to_string_pretty(&records).context("rendering the records")?
        ),
        Format::Yaml => out!(
            "{}",
            serde_yaml::to_string(&records).context("rendering the records")?
        ),
        Format::Text => {
            for record in &records {
                outln!(
                    "{:24} {} {:>4}d old  {}  {}",
                    record.kind,
                    record.observed_at,
                    record.age_days,
                    record.state.unwrap_or("-"),
                    record.producer
                );
            }
            outln!("{} record(s), aged at {}", records.len(), at);
        }
    }
    for refusal in &future {
        eprintln!("{refusal}");
    }
    Ok(exit_code(future.is_empty()))
}

fn exit_code(ok: bool) -> ExitCode {
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

// ---------------------------------------------------------------------------------------------
// `protocol infra` — the observation half. The bundle is produced elsewhere; here it is a file.
// ---------------------------------------------------------------------------------------------

/// What can be done with an observation bundle.
/// The website's CLI reference, held against the CLI itself.
///
/// The gate's website step is `npm run build`, and Docusaurus resolves links rather than claims: a
/// page that *describes* a CLI which has moved underneath it builds green for ever. On 2026-08-30,
/// at 0.33.0, seven of seventy-seven verbs had no entry — the whole workspace family
/// among them, eight releases after `website/docs/status/roadmap.md` told readers it had shipped.
///
/// The verb list comes from `clap` rather than from parsing `--help`, so it is the same tree the
/// binary dispatches on and cannot drift from it by a rendering change.
#[cfg(test)]
mod cli_reference {
    use clap::CommandFactory as _;

    /// The page a reader goes to for the verb list, relative to this crate.
    const REFERENCE: &str = "../../../website/docs/reference/cli.md";

    /// Every leaf verb, spelled as a reader would type it.
    ///
    /// A leaf, because a parent that only groups is not something anybody runs: `aep artifact`
    /// alone is an error message. `help` is clap's, not ours, and a hidden command is deliberately
    /// not part of the documented surface.
    fn leaves(command: &clap::Command, spelled: &str, out: &mut Vec<String>) {
        let mut branched = false;
        for sub in command.get_subcommands() {
            if sub.get_name() == "help" || sub.is_hide_set() {
                continue;
            }
            branched = true;
            leaves(sub, &format!("{spelled} {}", sub.get_name()), out);
        }
        if !branched {
            out.push(spelled.to_owned());
        }
    }

    /// The verbs the page does not spell anywhere.
    ///
    /// A substring match, deliberately: the page writes a verb inside a longer synopsis — flags,
    /// value hints, alternatives — so anything stricter would demand the reference be written in a
    /// shape nobody wants to read.
    fn absent_from<'a>(verbs: &'a [String], page: &str) -> Vec<&'a String> {
        verbs.iter().filter(|verb| !page.contains(*verb)).collect()
    }

    /// The rule is load-bearing only when a verb is genuinely missing, so the fixture makes one
    /// missing. Without this, `every_verb_…` would pass identically if `absent_from` returned an
    /// empty list unconditionally.
    #[test]
    fn a_verb_the_reference_does_not_spell_is_reported_by_name() {
        let verbs = vec![
            "aep artifact list".to_owned(),
            "aep workspace crossings".to_owned(),
        ];
        let page = "| `aep artifact list [--kind …]` | the plan, one line per artifact |";

        let missing = absent_from(&verbs, page);
        assert_eq!(
            missing,
            vec![&"aep workspace crossings".to_owned()],
            "the documented verb must pass and the undocumented one must be named"
        );
    }

    #[test]
    fn every_verb_the_cli_answers_has_an_entry_in_the_reference() {
        let mut verbs = Vec::new();
        leaves(&super::Cli::command(), "aep", &mut verbs);
        assert!(
            verbs.len() > 50,
            "the walk found only {} verbs, so it is walking the wrong tree rather than passing",
            verbs.len()
        );

        let page = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(REFERENCE),
        )
        .expect("reading the CLI reference page");

        let missing = absent_from(&verbs, &page);
        assert!(
            missing.is_empty(),
            "{} of {} verbs have no entry in {REFERENCE}:\n{}\n\nA verb a reader cannot find is a \
             verb that did not ship for them. Add a row to the surface it belongs to.",
            missing.len(),
            verbs.len(),
            missing
                .iter()
                .map(|verb| format!("  {verb}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
