//! `protocol artifact` — planning in the markdown store.
//!
//! The verb family an agent uses to plan work: create an epic, decompose it into stories, move one
//! to `active`, ask what the board looks like. Everything it touches is a markdown file in
//! `.engineering/planning/`, so the plan is reviewable in a pull request and its history is
//! `git log` rather than an export.
//!
//! # What lives here and what does not
//!
//! This is the first module split of `main.rs`, and the line is drawn where it costs nothing:
//! `Format`, the output macros, `print_table`, `print_serialised` and `exit_code` stay in the crate
//! root and are reached as `crate::` items, because they are shared with every other verb. What
//! moved is the verb family itself — its arguments, its rendering and its refusals — which shares
//! no state with anything else in the binary.
//!
//! # Exit codes
//!
//! `0` when the answer is yes, `1` when the store or a lifecycle says no. A refusal is *rendered*
//! rather than returned as an error, on the same reasoning `ess diff` gives for a pair it will not
//! compare: "a story cannot go straight from draft to implemented" is an answer about the input,
//! not a failure of this program.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_markdown::{MarkdownStore, PlanningDocument, PlanningFrontmatter, StoreReport};
use aep_domain::artifact::{
    ArtifactGraph, ArtifactId, ArtifactKind, ArtifactLifecycle, ArtifactRef, ArtifactStatus,
    RelationKind,
};
use aep_project::project::project_directory;
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};

use crate::Format;

/// The directory inside `.engineering` that holds the plan.
const PLANNING_DIRECTORY: &str = "planning";

/// Where a new document's body is seeded from, relative to the document tree.
const TEMPLATE_DIRECTORY: &str = "artifacts/templates";

/// Where the plan is and which documents govern it.
///
/// Split from the rendering choice because one verb needs a different one: `protocol artifact
/// graph` renders `dot` or `json`, which are not values the shared [`Format`] has, and a verb
/// carrying two `--format` flags is not a verb. The same reasoning that gave `protocol ess graph`
/// its own `GraphFormat` — a value a verb cannot honour is worse than one it does not offer.
///
/// `--store` is resolved **lazily**, and that is deliberate: `protocol artifact kinds` and
/// `protocol artifact relations` answer from the vocabulary alone, and a command that refused to
/// list the relation names because the working directory is not a project would be refusing for a
/// reason that has nothing to do with the question.
#[derive(Debug, Clone, Args)]
pub(crate) struct StoreLocation {
    /// The planning store. Defaults to `<project>/.engineering/planning`.
    #[arg(long)]
    store: Option<PathBuf>,
    /// The document tree the lifecycles and templates come from. Defaults to the project's
    /// configured `protocols` source, or `.` when there is no project.
    #[arg(long)]
    root: Option<PathBuf>,
}

/// Where the plan is, which documents govern it, and how to print the answer.
#[derive(Debug, Args)]
pub(crate) struct StoreArgs {
    /// Where the plan is.
    #[command(flatten)]
    location: StoreLocation,
    /// How to render the result.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
}

impl StoreArgs {
    /// The lifecycles in force.
    fn lifecycles(&self) -> Result<aep_engine::Registry> {
        self.location.lifecycles()
    }

    /// The repository the store sits in.
    fn repository_root(&self) -> PathBuf {
        self.location.repository_root()
    }
}

impl StoreLocation {
    /// A location built by something other than a command line.
    ///
    /// Only tests need this: every shipped caller gets its `StoreLocation` from clap. It exists so
    /// a test of something that *holds* one — a served request, say — can be written without
    /// parsing a command line to get there.
    #[cfg(test)]
    pub(crate) fn at(store: Option<PathBuf>, root: Option<PathBuf>) -> Self {
        Self { store, root }
    }

    /// The repository the store sits in, so a workspace beside it can be found.
    ///
    /// `<repo>/.engineering/planning` is the store, so the repository is two directories up. An
    /// explicit `--store` somewhere else answers the same way, which is what lets one repository's
    /// verbs validate another's store.
    ///
    /// Without `--store` this used to answer `.`, the **working directory** — so the same command
    /// that exits 0 at the repository root exited 1 from `crates/aep-domain/`, reporting every
    /// cross-repository relation as undeclared because `.engineering/workspace.yaml` was looked for
    /// beside the subdirectory. `story:own-engineering-store` promises *run anywhere inside it, with
    /// no flag*, and that promise and a green `validate` could not both hold. It now walks up to the
    /// project the way discovery does, and falls back to `.` only when there is no project at all —
    /// which is the historical answer for a tree that is not one.
    fn repository_root(&self) -> PathBuf {
        if let Some(path) = self.store.as_ref() {
            return path
                .parent()
                .and_then(std::path::Path::parent)
                .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf);
        }
        std::env::current_dir()
            .ok()
            .and_then(|here| aep_project::project::discover(&here))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Where the plan is kept, as `--store` or `project.yaml` says, with paths resolved.
    ///
    /// `--store <dir>` is the markdown form and overrides the project
    /// (`story:store-selection-in-project-yaml`). Without it, the discovered project's `store:`
    /// decides — `markdown` by default, so no existing project changes meaning.
    fn plan(&self) -> Result<Plan> {
        if let Some(path) = &self.store {
            return Ok(Plan::Markdown { root: path.clone() });
        }
        Plan::discovered()
    }

    /// The document tree, from `--root`, the discovered project, or the historical `.` fallback.
    fn document_root(&self) -> Result<PathBuf> {
        if let Some(path) = &self.root {
            return Ok(path.clone());
        }
        let here = std::env::current_dir().context("reading the working directory")?;
        let Some(project) = aep_project::project::discover(&here) else {
            return Ok(PathBuf::from("."));
        };
        aep_project::project::load_paths(&project)
            .map(|paths| paths.protocols)
            .with_context(|| {
                format!(
                    "reading the protocol document source from {}/project.yaml",
                    project.join(project_directory()).display()
                )
            })
    }

    /// The lifecycles in force, loaded exactly as `protocol validate` loads them.
    ///
    /// A tree with no `artifacts/lifecycles/` is not an error — it yields an empty registry, and
    /// every kind then gets [`ArtifactLifecycle::permissive`]. That is what makes the store usable
    /// in a repository that has not adopted the document tree yet.
    fn lifecycles(&self) -> Result<aep_engine::Registry> {
        crate::load(&self.document_root()?)
    }
}

/// Where a plan is kept, resolved: what a verb opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Plan {
    /// Markdown documents under `root`.
    Markdown { root: PathBuf },
    /// One SQLite file.
    Sqlite { path: PathBuf },
    /// A PostgreSQL database.
    Postgres { url: String },
    /// Markdown documents under `root` and a replica, under a declared policy
    /// (`story:hybrid-backend`).
    Hybrid {
        root: PathBuf,
        replica: Replica,
        policy: aep_domain::project::HybridPolicy,
    },
}

/// The replica half of a hybrid plan. The local half is always the markdown documents: they are
/// the plan's shape, and the projection that writes them is the plan's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Replica {
    Sqlite(PathBuf),
    Postgres(String),
}

impl Replica {
    /// Where the replica is, for a message.
    fn describe(&self) -> String {
        match self {
            Self::Sqlite(path) => format!("the SQLite store {}", path.display()),
            Self::Postgres(url) => format!("the Postgres store {}", redact(url)),
        }
    }
}

impl Plan {
    /// The plan the project whose `.engineering/` is `engineering` names in its `project.yaml`.
    ///
    /// A project directory with no `project.yaml` — a fixture, or a repository that has adopted
    /// nothing yet — is the default configuration: markdown under `planning/`.
    pub(crate) fn for_project(engineering: &Path) -> Result<Self> {
        let config_path = engineering.join(aep_domain::project::PROJECT_FILE);
        if !config_path.exists() {
            return Ok(Self::Markdown {
                root: engineering.join(PLANNING_DIRECTORY),
            });
        }
        let text = std::fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?;
        let config = aep_schema::parse::project(&text, Some(&config_path.display().to_string()))
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        Self::from_config(&config.store.resolved(engineering), engineering)
    }

    /// The plan the project this was run in names — `project.yaml` found by walking up from here.
    pub(crate) fn discovered() -> Result<Self> {
        let here = std::env::current_dir().context("reading the working directory")?;
        let directory = project_directory();
        let project = aep_project::project::discover(&here).with_context(|| {
            format!(
                "no `--store` was given and no `{directory}/project.yaml` was found in {} \
                 or any parent; pass `--store <dir>` to say where the plan is",
                here.display()
            )
        })?;
        Self::for_project(&project.join(directory))
    }

    /// The plan a validated configuration names.
    fn from_config(store: &aep_domain::project::StoreConfig, engineering: &Path) -> Result<Self> {
        use aep_domain::project::StoreConfig;
        Ok(match store {
            StoreConfig::Markdown => Self::Markdown {
                root: engineering.join(PLANNING_DIRECTORY),
            },
            StoreConfig::Sqlite { path } => Self::Sqlite { path: path.clone() },
            StoreConfig::Postgres { url } => Self::Postgres { url: url.clone() },
            StoreConfig::Hybrid {
                policy,
                local,
                replica,
            } => {
                let StoreConfig::Markdown = **local else {
                    anyhow::bail!(
                        "`store: hybrid` keeps the plan's documents as its local half; this build \
                         opens `local: markdown` and `{}` is not that",
                        describe_config(local)
                    );
                };
                let replica = match &**replica {
                    StoreConfig::Sqlite { path } => Replica::Sqlite(path.clone()),
                    StoreConfig::Postgres { url } => Replica::Postgres(url.clone()),
                    other => anyhow::bail!(
                        "`store: hybrid` needs a replica this build can open — `sqlite: <path>` or \
                         `postgres: <url>` — and `{}` is not one",
                        describe_config(other)
                    ),
                };
                Self::Hybrid {
                    root: engineering.join(PLANNING_DIRECTORY),
                    replica,
                    policy: policy.clone(),
                }
            }
        })
    }

    /// Where the plan is, for a message.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Markdown { root } => root.display().to_string(),
            Self::Sqlite { path } => format!("the SQLite store {}", path.display()),
            Self::Postgres { url } => format!("the Postgres store {}", redact(url)),
            Self::Hybrid { root, replica, .. } => {
                format!(
                    "{} with its replica in {}",
                    root.display(),
                    replica.describe()
                )
            }
        }
    }

    /// The contract over a plan that keeps no documents: SQLite or Postgres, hydrated on open.
    ///
    /// A markdown or hybrid plan is not opened here — its backend needs the workspace and the
    /// ladders the verbs carry (`markdown_backend_for`, `hybrid_backend_for`), and reading it does
    /// not need a backend at all.
    fn open_backend(&self) -> Result<Option<PlanBackend>> {
        Ok(match self {
            Self::Markdown { .. } | Self::Hybrid { .. } => None,
            Self::Sqlite { path } => Some(PlanBackend::Sqlite(
                aep_backend_sqlite::SqliteBackend::open(path)
                    .map_err(|error| anyhow::anyhow!("{error}"))
                    .with_context(|| format!("opening the SQLite store at {}", path.display()))?,
            )),
            Self::Postgres { url } => Some(PlanBackend::Postgres(
                aep_backend_postgres::PostgresBackend::connect(url)
                    .map_err(|error| anyhow::anyhow!("{error}"))
                    .with_context(|| format!("connecting to {}", redact(url)))?,
            )),
        })
    }
}

/// The plan `protocol drive` rebuilds its artifact graph from, in whichever store the project names.
///
/// `--store <dir>` is the markdown override, as it is for every `protocol artifact` verb. A store
/// that cannot be read answers a report whose failures say so, which is what stops the run —
/// the driver treats a plan it cannot trust as `StoreBroken`, not as *blocked*.
pub(crate) struct DrivenPlan {
    plan: Plan,
    /// The project the plan belongs to, so the workspace manifest beside it can be read.
    project: PathBuf,
}

impl DrivenPlan {
    /// The plan for `project`, or the markdown store at `store` when one was given.
    pub(crate) fn for_project(store: Option<&Path>, project: &Path) -> Result<Self> {
        let plan = match store {
            Some(root) => Plan::Markdown {
                root: root.to_path_buf(),
            },
            None => Plan::for_project(&project.join(project_directory()))?,
        };
        Ok(Self {
            plan,
            project: project.to_path_buf(),
        })
    }
}

impl aep_driver::PlanSource for DrivenPlan {
    fn load(&self) -> StoreReport {
        match &self.plan {
            Plan::Markdown { root } => MarkdownStore::open(root.clone()).load(),
            Plan::Hybrid {
                root,
                replica,
                policy,
            } => match hybrid_documents(root, replica, policy) {
                Ok(report) => report,
                Err(error) => unreadable_plan(&self.plan, &error),
            },
            durable => match durable.open_backend() {
                Ok(Some(backend)) => match report_from_backend(&backend) {
                    Ok(report) => report,
                    Err(error) => unreadable_plan(durable, &error),
                },
                Ok(None) => unreachable!("a durable plan opens a backend"),
                Err(error) => unreadable_plan(durable, &error),
            },
        }
    }

    fn describe(&self) -> String {
        self.plan.describe()
    }

    /// What `<project>/.engineering/workspace.yaml` declares, so a relation into another
    /// repository is judged the same way `protocol artifact validate` judges it.
    fn declared_members(&self) -> Vec<aep_domain::workspace::MemberName> {
        declared_members(&self.project)
    }
}

/// The report of a plan that could not be read at all: one failure, naming the store and the cause.
fn unreadable_plan(plan: &Plan, error: &anyhow::Error) -> StoreReport {
    StoreReport {
        failures: vec![aep_backend_markdown::store::StoreFailure {
            path: PathBuf::from(plan.describe()),
            detail: format!("{error:#}"),
        }],
        ..StoreReport::default()
    }
}

/// A connection URL with its password, if any, left out of a message.
pub(crate) fn redact(url: &str) -> String {
    match (url.find("://"), url.rfind('@')) {
        (Some(scheme), Some(at)) if at > scheme => {
            let credentials = &url[scheme + 3..at];
            match credentials.split_once(':') {
                Some((user, _)) => format!("{}{user}:…{}", &url[..scheme + 3], &url[at..]),
                None => url.to_owned(),
            }
        }
        _ => url.to_owned(),
    }
}

/// A store configuration, as a message names it.
fn describe_config(store: &aep_domain::project::StoreConfig) -> String {
    use aep_domain::project::StoreConfig;
    match store {
        StoreConfig::Markdown => "markdown".to_owned(),
        StoreConfig::Sqlite { path } => format!("sqlite: {}", path.display()),
        StoreConfig::Postgres { url } => format!("postgres: {}", redact(url)),
        StoreConfig::Hybrid { .. } => "hybrid".to_owned(),
    }
}

/// The hybrid plan's documents, read through the composite's declared read path, without opening
/// the contract over it.
fn hybrid_documents(
    root: &Path,
    replica: &Replica,
    policy: &aep_domain::project::HybridPolicy,
) -> Result<StoreReport> {
    use aep_backend_hybrid::Composite;
    use aep_backend_markdown::projection::documents_of;

    let policy =
        aep_backend_hybrid::policy_from(policy).map_err(|error| anyhow::anyhow!("{error}"))?;
    let report = match replica {
        Replica::Sqlite(path) => {
            let store = entity_sqlite::SqliteStore::open(path)
                .with_context(|| format!("opening the replica at {}", path.display()))?;
            documents_of(&Composite::new(root, store, policy))
        }
        Replica::Postgres(url) => {
            let store = entity_postgres::PostgresStore::connect_no_tls(url)
                .with_context(|| format!("connecting to the replica at {}", redact(url)))?;
            documents_of(&Composite::new(root, store, policy))
        }
    };
    report
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("reading the hybrid plan at {}", root.display()))
}

/// The contract over a hybrid plan: the markdown projection over the composite, with the workspace
/// and the ladders the verbs carry, remembering every divergence written beside the plan.
fn hybrid_backend_for(
    args: &StoreLocation,
    root: &Path,
    replica: &Replica,
    policy: &aep_domain::project::HybridPolicy,
) -> Result<PlanBackend> {
    use aep_backend_hybrid::HybridBackend;

    let policy =
        aep_backend_hybrid::policy_from(policy).map_err(|error| anyhow::anyhow!("{error}"))?;
    let members = declared_members(&args.repository_root());
    let lifecycles = args.lifecycles()?.lifecycles().clone();
    let opened = match replica {
        Replica::Sqlite(path) => {
            let store = entity_sqlite::SqliteStore::open(path)
                .with_context(|| format!("opening the replica at {}", path.display()))?;
            HybridBackend::open(
                root,
                store,
                policy,
                members,
                clock_at_the_edge(),
                command_actor()?,
                lifecycles,
            )
            .map(PlanBackend::HybridSqlite)
        }
        Replica::Postgres(url) => {
            let store = entity_postgres::PostgresStore::connect_no_tls(url)
                .with_context(|| format!("connecting to the replica at {}", redact(url)))?;
            HybridBackend::open(
                root,
                store,
                policy,
                members,
                clock_at_the_edge(),
                command_actor()?,
                lifecycles,
            )
            .map(PlanBackend::HybridPostgres)
        }
    };
    opened
        .map_err(|error| anyhow::anyhow!("{error}"))
        .with_context(|| format!("opening the hybrid plan at {}", root.display()))
}

/// The backend a plan opens: one enum so every verb is written once over the contract.
enum PlanBackend {
    Markdown(aep_backend_markdown::backend::MarkdownBackend),
    Sqlite(aep_backend_sqlite::SqliteBackend),
    Postgres(aep_backend_postgres::PostgresBackend),
    HybridSqlite(aep_backend_hybrid::HybridBackend<entity_sqlite::SqliteStore>),
    HybridPostgres(aep_backend_hybrid::HybridBackend<entity_postgres::PostgresStore>),
}

impl PlanBackend {
    /// The event log of one entity, as the provider keeps it — what a plan without a journal
    /// answers `history` from.
    fn events_of(
        &self,
        id: &aep_domain::entity::EntityId,
    ) -> Result<Vec<entity_core::DomainEvent>> {
        let events = match self {
            Self::Markdown(backend) => backend.as_entity_backend().events_of(id),
            Self::Sqlite(backend) => backend.as_entity_backend().events_of(id),
            Self::Postgres(backend) => backend.as_entity_backend().events_of(id),
            Self::HybridSqlite(backend) => backend.as_entity_backend().events_of(id),
            Self::HybridPostgres(backend) => backend.as_entity_backend().events_of(id),
        };
        events.map_err(|error| anyhow::anyhow!("reading the event log: {error}"))
    }
}

impl aep_contract::command::CommandService for PlanBackend {
    type Command = aep_domain::command::Command;

    async fn execute(
        &self,
        envelope: aep_contract::command::CommandEnvelope<Self::Command>,
    ) -> Result<aep_contract::command::CommandResult, aep_contract::error::CommandError> {
        match self {
            Self::Markdown(backend) => backend.execute(envelope).await,
            Self::Sqlite(backend) => backend.execute(envelope).await,
            Self::Postgres(backend) => backend.execute(envelope).await,
            Self::HybridSqlite(backend) => backend.execute(envelope).await,
            Self::HybridPostgres(backend) => backend.execute(envelope).await,
        }
    }
}

impl aep_contract::query::QueryService for PlanBackend {
    type AuditRecord = aep_domain::audit::AuditRecord;

    async fn get(
        &self,
        reference: &aep_domain::entity::EntityRef,
        consistency: aep_contract::QueryConsistency,
    ) -> Result<aep_contract::query::EntityEnvelope, aep_contract::error::QueryError> {
        match self {
            Self::Markdown(backend) => backend.get(reference, consistency).await,
            Self::Sqlite(backend) => backend.get(reference, consistency).await,
            Self::Postgres(backend) => backend.get(reference, consistency).await,
            Self::HybridSqlite(backend) => backend.get(reference, consistency).await,
            Self::HybridPostgres(backend) => backend.get(reference, consistency).await,
        }
    }

    async fn resolve(
        &self,
        locator: &aep_domain::entity::EntityLocator,
    ) -> Result<aep_domain::entity::EntityId, aep_contract::error::QueryError> {
        match self {
            Self::Markdown(backend) => backend.resolve(locator).await,
            Self::Sqlite(backend) => backend.resolve(locator).await,
            Self::Postgres(backend) => backend.resolve(locator).await,
            Self::HybridSqlite(backend) => backend.resolve(locator).await,
            Self::HybridPostgres(backend) => backend.resolve(locator).await,
        }
    }

    async fn query(
        &self,
        query: &aep_contract::query::EntityQuery,
    ) -> Result<
        aep_contract::query::Page<aep_contract::query::EntityEnvelope>,
        aep_contract::error::QueryError,
    > {
        match self {
            Self::Markdown(backend) => backend.query(query).await,
            Self::Sqlite(backend) => backend.query(query).await,
            Self::Postgres(backend) => backend.query(query).await,
            Self::HybridSqlite(backend) => backend.query(query).await,
            Self::HybridPostgres(backend) => backend.query(query).await,
        }
    }

    async fn relations(
        &self,
        query: &aep_contract::query::RelationQuery,
    ) -> Result<
        aep_contract::query::Page<aep_contract::query::Relation>,
        aep_contract::error::QueryError,
    > {
        match self {
            Self::Markdown(backend) => backend.relations(query).await,
            Self::Sqlite(backend) => backend.relations(query).await,
            Self::Postgres(backend) => backend.relations(query).await,
            Self::HybridSqlite(backend) => backend.relations(query).await,
            Self::HybridPostgres(backend) => backend.relations(query).await,
        }
    }

    async fn history(
        &self,
        reference: &aep_domain::entity::EntityRef,
    ) -> Result<Vec<aep_contract::query::RevisionRecord>, aep_contract::error::QueryError> {
        match self {
            Self::Markdown(backend) => backend.history(reference).await,
            Self::Sqlite(backend) => backend.history(reference).await,
            Self::Postgres(backend) => backend.history(reference).await,
            Self::HybridSqlite(backend) => backend.history(reference).await,
            Self::HybridPostgres(backend) => backend.history(reference).await,
        }
    }

    async fn audit(
        &self,
        query: &aep_contract::query::AuditQuery,
    ) -> Result<aep_contract::query::Page<Self::AuditRecord>, aep_contract::error::QueryError> {
        match self {
            Self::Markdown(backend) => backend.audit(query).await,
            Self::Sqlite(backend) => backend.audit(query).await,
            Self::Postgres(backend) => backend.audit(query).await,
            Self::HybridSqlite(backend) => backend.audit(query).await,
            Self::HybridPostgres(backend) => backend.audit(query).await,
        }
    }

    async fn describe_type(
        &self,
        entity_type: &aep_domain::entity::EntityType,
    ) -> Result<aep_contract::registry::TypeDescriptor, aep_contract::error::QueryError> {
        match self {
            Self::Markdown(backend) => backend.describe_type(entity_type).await,
            Self::Sqlite(backend) => backend.describe_type(entity_type).await,
            Self::Postgres(backend) => backend.describe_type(entity_type).await,
            Self::HybridSqlite(backend) => backend.describe_type(entity_type).await,
            Self::HybridPostgres(backend) => backend.describe_type(entity_type).await,
        }
    }
}

/// A plan, opened: the backend every write goes through and the documents every read answers from.
///
/// A markdown plan's documents are its files, read as they always were; a SQLite or Postgres plan
/// has none, so its documents are built from the contract's entities — the same mapping the
/// markdown projection applies on a write — and every read verb is then one function over a
/// `StoreReport`, whichever store it came from (`story:store-selection-in-project-yaml`).
struct Opened {
    plan: Plan,
    /// The contract over the plan. Absent for a markdown plan opened to read: building it hydrates
    /// the store and refuses a plan that does not build a graph, and `validate` exists to report
    /// exactly that plan rather than be refused by it.
    backend: Option<PlanBackend>,
    report: StoreReport,
    /// The files, for a markdown plan: the journal, the drift check and the path of a document.
    files: Option<MarkdownStore>,
}

impl Opened {
    /// The contract, for a verb that writes or asks the store what it recorded.
    fn backend(&self) -> Result<&PlanBackend> {
        self.backend
            .as_ref()
            .context("the plan was opened to read its documents, not to command it")
    }

    /// Why an id names nothing here.
    fn missing(&self, id: &ArtifactId) -> String {
        match &self.files {
            Some(store) => missing(store, id),
            None => format!("{} holds no `{id}`", self.plan.describe()),
        }
    }

    /// Where a document is, for a message: a path for a markdown plan, the plan otherwise.
    fn path_of(&self, id: &ArtifactId) -> String {
        match &self.files {
            Some(store) => store
                .root()
                .join(store.relative_path_for(id))
                .display()
                .to_string(),
            None => format!("{} in {}", relative_path_for(id), self.plan.describe()),
        }
    }

    /// The evidence recorded about `id`, by kind, wherever this plan keeps its records.
    fn evidence_on_hand(
        &self,
        id: &ArtifactId,
    ) -> Result<aep_backend_markdown::kernel::EvidenceOnHand> {
        match &self.files {
            Some(store) => Ok(aep_backend_markdown::journal::evidence_on_hand(
                store.root(),
                id,
            )),
            None => evidence_from_events(self.backend()?, id),
        }
    }
}

/// `<kind>/<name>.md`, the path a document has in every plan whether or not it is a file.
fn relative_path_for(id: &ArtifactId) -> String {
    format!("{}/{}.md", id.namespace(), id.name())
}

/// Opens the plan `args` names and reads its documents.
///
/// `with_backend` is what a verb that writes, or asks the contract, passes: the backend is built —
/// and a markdown plan that cannot be read cleanly is refused rather than warned about, because a
/// write into a plan with an unreadable document is a write into a plan nobody can see whole.
fn open(args: &StoreLocation, with_backend: bool) -> Result<Opened> {
    let plan = args.plan()?;
    match &plan {
        Plan::Markdown { root } => {
            let store = MarkdownStore::open(root.clone());
            let report = store.load();
            if with_backend {
                require_clean(&store, &report)?;
            } else {
                warn_unclean(&report);
            }
            let backend = with_backend
                .then(|| markdown_backend_for(args, root))
                .transpose()?
                .map(PlanBackend::Markdown);
            Ok(Opened {
                plan,
                backend,
                report,
                files: Some(store),
            })
        }
        Plan::Hybrid {
            root,
            replica,
            policy,
        } => {
            // The documents through the composite's declared read path; the files, for the journal
            // and a document's path, are the local half — the authority or the replica of it, and
            // in either case where the plan's own journal is written.
            let report = hybrid_documents(root, replica, policy)?;
            let backend = with_backend
                .then(|| hybrid_backend_for(args, root, replica, policy))
                .transpose()?;
            let files = Some(MarkdownStore::open(root.clone()));
            Ok(Opened {
                plan,
                backend,
                report,
                files,
            })
        }
        Plan::Sqlite { .. } | Plan::Postgres { .. } => {
            let backend = plan
                .open_backend()?
                .context("a SQLite or Postgres plan opens a backend")?;
            let report = report_from_backend(&backend)?;
            Ok(Opened {
                plan,
                backend: Some(backend),
                report,
                files: None,
            })
        }
    }
}

/// The documents a store that keeps none would hold: one per entity addressed into this plan.
fn report_from_backend(backend: &PlanBackend) -> Result<StoreReport> {
    use aep_contract::query::{EntityQuery, QueryService, RelationQuery};
    use aep_contract::testing::block_on;
    use aep_domain::entity::EntityRef;

    let page = block_on(backend.query(&EntityQuery {
        organisation: Some(aep_backend_markdown::backend::ORGANISATION.to_owned()),
        space: Some(aep_backend_markdown::backend::SPACE.to_owned()),
        ..EntityQuery::default()
    }))
    .map_err(|error| anyhow::anyhow!("reading the plan: {error}"))?;

    let artifact_of = |locator: &aep_domain::entity::EntityLocator| {
        ArtifactId::new(format!("{}:{}", locator.kind(), locator.key())).ok()
    };
    let mut report = StoreReport::default();
    for envelope in &page.items {
        let Some(id) = artifact_of(&envelope.metadata.locator) else {
            continue;
        };
        let edges = block_on(backend.relations(&RelationQuery {
            source: Some(EntityRef::new(envelope.metadata.id.clone())),
            ..RelationQuery::default()
        }))
        .map_err(|error| anyhow::anyhow!("reading the plan's edges: {error}"))?;
        let mut relations = Vec::new();
        for relation in &edges.items {
            let target =
                block_on(backend.get(&relation.target, aep_contract::QueryConsistency::Current))
                    .map_err(|error| anyhow::anyhow!("reading an edge's target: {error}"))?;
            if let Some(target) = artifact_of(&target.metadata.locator) {
                relations.push(aep_domain::artifact::ArtifactRelation::new(
                    relation.kind,
                    ArtifactRef::new(target, None),
                ));
            }
        }
        let Some(document) = aep_backend_markdown::projection::document_from_entity(
            id.clone(),
            &envelope.data,
            envelope.metadata.revision.get(),
            &relations,
        ) else {
            continue;
        };
        report.files_read += 1;
        report.documents.insert(
            id.clone(),
            aep_backend_markdown::store::StoredDocument {
                relative_path: relative_path_for(&id),
                document,
            },
        );
    }
    Ok(report)
}

/// How much evidence is on hand about `id`, in a plan without a journal: counted from the entity's
/// own events, as `history` reads them.
///
/// Not from the audit trail: an accepted command's record names its subject and nothing about what
/// kind of evidence it recorded — the contract keeps `decision` for refusals — so the trail could
/// only ever say *something* was recorded. The event carries the command's `args` (`kind`,
/// `source`), or the projection's note, and the `Identity` shape writes one on the target at its
/// unchanged revision precisely so this count survives the process that recorded it.
fn evidence_from_events(
    backend: &PlanBackend,
    id: &ArtifactId,
) -> Result<aep_backend_markdown::kernel::EvidenceOnHand> {
    use aep_backend_markdown::journal::Change;
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::entity::EntityLocator;

    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        id.namespace(),
        id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))?;
    let target = block_on(backend.resolve(&locator))
        .map_err(|error| anyhow::anyhow!("`{id}` is not in this store: {error}"))?;
    let mut counted = aep_backend_markdown::kernel::EvidenceOnHand::new();
    for event in backend.events_of(&target)? {
        if let Some(entry) = entry_from_event(backend, id, &event) {
            if let Change::Evidence { kind, .. } = entry.change {
                *counted.entry(kind).or_default() += 1;
            }
        }
    }
    Ok(counted)
}

/// What can be done with the plan.
#[derive(Debug, Subcommand)]
pub(crate) enum ArtifactCommand {
    /// Create a plan item, and write it.
    New(NewArgs),
    /// Move a plan item to another status, if its kind's lifecycle permits.
    ///
    /// Refused moves are printed with **every** status the artifact could have moved to instead,
    /// because a refusal that does not answer the question it creates sends the reader to the
    /// lifecycle file to work it out.
    Move {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
        /// The status to move it to, such as `active`.
        #[arg(long)]
        to: String,
        /// Evidence presented for the move, as `<kind>=<count>`, repeatable.
        ///
        /// Needed only for a rung whose lifecycle document declares a `requires:` entry. The
        /// planning store holds markdown, not evidence records, so what is on hand comes from the
        /// caller — the same shape the kernel demands of a clock.
        ///
        /// **This establishes that a count was presented, not that the records are sound.** It names
        /// no test, no run and no artifact, and nothing can go and check it — so a move that uses it
        /// says so on the way out and the journal records the move as resting on an assertion.
        ///
        /// `protocol artifact evidence` is the alternative and the better one: it records an
        /// observation *about* an artifact with a source and an instant, `move` finds it without
        /// being told, and evidence about one story is worth nothing to another. This flag stays for
        /// evidence that lives outside the store — a CI run nobody recorded is still real, and
        /// refusing it would only push people to record a fiction.
        #[arg(long = "evidence", value_name = "KIND=COUNT")]
        evidence: Vec<String>,
        /// The instant to judge a dated rung against, ISO-8601. Defaults to now.
        ///
        /// Needed only for a rung whose lifecycle document declares a `when:` entry. The clock is
        /// read **here**, at the edge, and never inside a decision — which is what lets the same
        /// move be replayed a year later and give the same answer.
        #[arg(long = "at", value_name = "INSTANT")]
        at: Option<String>,
        /// Walk the ladder's intermediate rungs to get there, journalling each hop.
        ///
        /// `draft -> proposed -> active` is two commands per story on every wave (`8cffc110#184`,
        /// and `9da4f51c#3303`, a `python` loop issuing four commands for each of eight stories).
        /// This is one, and it is still N moves in the journal, because a walk that recorded one
        /// hop would be a history that says a story was never proposed.
        ///
        /// It crosses only rungs **nothing guards**: a rung with a `requires:` or a `when:` stops
        /// the walk with that rung's own refusal, because arriving somewhere by not being asked is
        /// exactly what an evidence gate is against.
        #[arg(long)]
        via: bool,
    },
    /// Add an edge from one plan item to another.
    ///
    /// Two spellings, one act. `relate <id> <relation> <target>` is the three-positional form;
    /// `relate <id> <relation>:<target>` is the one `new --relate` already takes, split at the
    /// **first** colon exactly as that flag is. Both issue the same command and journal the same
    /// entry — `relate story:x serves:vision:O2` used to be refused for want of a third positional
    /// while the same words were accepted at `new`, which is one spelling too many for one edge.
    Relate {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact the edge starts at.
        id: String,
        /// What the edge means, such as `decomposes`. `protocol artifact relations` lists them.
        ///
        /// May carry the target after a colon — `decomposes:epic:passwordless` — in which case
        /// the third positional is left off.
        relation: String,
        /// The artifact the edge points at, when the relation does not already name it.
        target: Option<String>,
    },
    /// Replace, extend or re-section a plan item's markdown body, preserving CLI-owned frontmatter.
    ///
    /// Three ways to arrive at a body and one door to write it. Without a flag `--from` is the
    /// **whole** body, which is the original verb; `--append` adds to what is there; `--section`
    /// replaces the prose under one `##` heading, or adds that section at the end when the document
    /// has no such heading. Each is one `update` in the journal.
    ///
    /// The reason the last two exist is what happened without them: five sessions patched the
    /// store with `python`, heredocs and `cat >>`, because *append a section* and *replace a
    /// section* had no verb and the whole-body rewrite was the only thing on offer — and a hand
    /// edit skips the journal, which is the record every other write leaves.
    Body {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
        /// Read the body — whole, appended or sectioned — from this UTF-8 file; `-` reads stdin.
        #[arg(long, value_name = "PATH")]
        from: PathBuf,
        /// Add what `--from` holds to the end of the body instead of replacing it.
        #[arg(long, conflicts_with = "section")]
        append: bool,
        /// Replace the prose under this `##` heading, or add the section at the end.
        #[arg(long, value_name = "HEADING")]
        section: Option<String>,
    },
    /// Change one frontmatter field, through the door every other write passes.
    ///
    /// The verb `body` had and frontmatter did not: nothing changed a title, a summary, an owner or
    /// a tag, so every session that needed one reached for an editor or a `python` frontmatter
    /// splitter — and `11727595#818` shows what that costs, a hand-patched `revision:` caught as
    /// drift after `edit`, `update`, `set` and `write` had each been tried and refused as unknown
    /// verbs.
    ///
    /// **What it will not change**: `status`, which is `move`'s and carries a lifecycle decision;
    /// `revision`, which is the store's own count of writes; and `id` and `kind`, which are the
    /// artifact's identity. Each is refused by name rather than by being an unrecognised flag.
    Set {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
        /// The title, which is what a listing shows.
        #[arg(long, allow_hyphen_values = true)]
        title: Option<String>,
        /// A one-line summary.
        #[arg(long, allow_hyphen_values = true)]
        summary: Option<String>,
        /// Who owns it.
        #[arg(long, allow_hyphen_values = true)]
        owner: Option<String>,
        /// A label to add. Repeat for more than one.
        #[arg(long = "tag", allow_hyphen_values = true)]
        tag: Vec<String>,
        /// A label to remove. Repeat for more than one.
        #[arg(long = "untag", allow_hyphen_values = true)]
        untag: Vec<String>,
        /// Not a field this verb changes; `move` does, and records why.
        #[arg(long, hide = true, allow_hyphen_values = true)]
        status: Option<String>,
        /// Not a field this verb changes; the store counts its own writes.
        #[arg(long, hide = true, allow_hyphen_values = true)]
        revision: Option<String>,
        /// Not a field this verb changes; an artifact's identity is fixed at `new`.
        #[arg(long = "id", hide = true, allow_hyphen_values = true)]
        identity: Option<String>,
        /// Not a field this verb changes; an artifact's identity is fixed at `new`.
        #[arg(long, hide = true, allow_hyphen_values = true)]
        kind: Option<String>,
    },
    /// Print one artifact: its frontmatter fields, then its body.
    ///
    /// The verb for an id in hand, and the one this family did not have. `list` prints the whole
    /// plan, `board` arranges it, `history` prints the event log, `explain` answers what made a
    /// status happen — and `body` *writes*. Somebody holding `story:passkey-login` and wanting to
    /// read it had nothing to type, and reached for `show`, because that is what every other tool
    /// calls it.
    ///
    /// The body is printed **verbatim**. A verb that summarised the prose would be a second and
    /// worse `explain`, and the reason to run this is to see what the document actually says.
    Show {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
        /// Print the body bytes and nothing else — no labels, no trailing newline this verb added.
        ///
        /// What `body --from` would write back unchanged, so a body can be read out, edited and
        /// handed back without a frontmatter splitter in between. Refused with a machine format:
        /// *the bytes and nothing else* is a promise a JSON wrapper breaks.
        #[arg(long = "body-only")]
        body_only: bool,
    },
    /// List the plan, one line per artifact.
    List {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// Only this kind, and the kinds that specialise it.
        #[arg(long)]
        kind: Option<String>,
        /// Only this status.
        #[arg(long)]
        status: Option<String>,
    },
    /// Show what is blocked, grouped by the single thing blocking it.
    ///
    /// The question a backlog cannot answer: *what is stopped, on what type of thing, and on which
    /// item*. Five stories waiting on one decision are one row here with five lines under it —
    /// one conversation to have — where a list of five parked stories is five conversations
    /// somebody has to start individually.
    ///
    /// A `blocks` edge counts until the artifact declaring it reaches the end of its own
    /// lifecycle, so `protocol artifact move <blocker> --to cleared` is how something is
    /// unblocked, and the journal keeps the record that it was ever stuck.
    ///
    /// Always exits 0. This is a report: an exit code that moved with the count would make every
    /// plan holding a blocker a red build.
    Blocked {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// Only blockers of this type, such as `credential`.
        #[arg(long = "type", value_name = "TYPE")]
        category: Option<String>,
    },
    /// Show the plan as status columns.
    Board {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// Only this kind, and the kinds that specialise it.
        #[arg(long)]
        kind: Option<String>,
    },
    /// Print the plan's graph.
    ///
    /// `dot` is for `dot -Tsvg`; `json` is the artifact graph itself, for a consumer that would
    /// otherwise have to parse a diagram to get at it. The same split `protocol ess graph` makes.
    Graph {
        /// Where the plan is.
        #[command(flatten)]
        store: StoreLocation,
        /// How to render the graph.
        #[arg(long, value_enum, default_value_t = PlanningGraphFormat::Dot)]
        format: PlanningGraphFormat,
    },
    /// Show what happened to one artifact, oldest first.
    ///
    /// Read from the store's own journal rather than from git: a rename is a guess in a repository
    /// log, a squash loses the moves, and neither answers *which of these was a status move*
    /// without parsing markdown out of a patch.
    History {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
    },
    /// Explain a status: what the store admitted before each move, and what it was about.
    ///
    /// The audit question three months later — *what made this done* — answered out of the store
    /// rather than out of the repository's log (`story:completion-audit-join`). Per status the
    /// artifact reached: the move, the instant, the revision it left the artifact at, and every
    /// evidence record admitted since the previous move, each named against **the revision the
    /// artifact was at when the record was admitted** — so a later edit to the body cannot make an
    /// old record look like it was about the new text.
    ///
    /// A status reached with no record is marked rather than left blank, in the words
    /// `protocol artifact validate` uses: a move on somebody's assertion is legal, and what it must
    /// not be is indistinguishable from one the store holds a record for.
    ///
    /// `protocol explain` is a different question — how a policy decided — and this is deliberately
    /// not it.
    Explain {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
    },
    /// List the divergences a hybrid plan has recorded: writes one side took and the other did not.
    ///
    /// Only a `store: hybrid` plan has any. The list is what `catch-up` replays, and the exit code
    /// says whether anything is outstanding.
    Divergences {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
    },
    /// Replay a hybrid plan's recorded divergences at the side that has not seen them.
    ///
    /// The runtime's catch-up (`store-v0.1.md` R-108): what the authority holds **now** is
    /// replayed, nothing is merged, and a replica that moved on its own stays outstanding for a
    /// person.
    CatchUp {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
    },
    /// Record evidence about an artifact, so a later move can be decided on it.
    ///
    /// The alternative this replaces is `move --evidence test_result=1`, a number that names no
    /// test, no run and no artifact. This names all three, is appended and never edited, and is
    /// found by `move` rather than typed at it. The store still cannot *verify* that the run
    /// happened — but a claim with a subject, a source and an instant is one somebody can go and
    /// check, and a bare count is not.
    Evidence {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// What the evidence is about, such as `story:passkey-login`.
        id: String,
        /// The kind of observation, such as `test_result` or `approval`.
        #[arg(long)]
        kind: String,
        /// Where it came from — `task check`, a CI run URL, a person's name.
        #[arg(long)]
        source: String,
        /// Where to go and look: a URL, a run id, a file path.
        #[arg(long = "ref", value_name = "REFERENCE")]
        reference: Option<String>,
        /// When it was observed, ISO-8601. Defaults to now, read at the edge.
        #[arg(long, value_name = "INSTANT")]
        at: Option<String>,
    },
    /// Check the whole plan: every file, every edge, every status.
    ///
    /// Three classes of problem, accumulated into one list rather than reported one run at a time:
    /// a file that cannot be read or sits where its id does not put it; an edge that points at
    /// nothing, a cycle, or an id declared twice; and a status the kind's lifecycle does not have.
    Validate {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// Exit 1 on what this verb otherwise only reports.
        ///
        /// Three classes are reported and deliberately not failed on: a status reached on somebody's
        /// assertion rather than on a record, a document that predates the event log, and drift
        /// between a document and what the log says was written to it. Each is legal in a store
        /// people are working in — refusing an assertion outright would stop anybody closing a
        /// story on the day a runner is down — and each is a thing a *gate* has every reason to
        /// refuse. Without this flag the output and the exit code are exactly what they were.
        #[arg(long)]
        strict: bool,
    },
    /// List the artifact kinds, marking the ones that are planning rather than output.
    ///
    /// The compiled vocabulary, plus every kind the document tree declares a lifecycle for, plus
    /// one row for the open `<type>-blocker` family — which is what makes this the verb that
    /// answers *what can I create* rather than *what did this binary ship knowing*.
    Kinds {
        /// How to render, and which tree the lifecycles come from. `--store` is not read: this
        /// answers from the vocabulary and the documents, not from the plan.
        #[command(flatten)]
        store: StoreArgs,
    },
    /// List the relation vocabulary, with what each edge means.
    Relations {
        /// How to render. `--store` is not read: this answers from the vocabulary.
        #[command(flatten)]
        store: StoreArgs,
    },
    /// Show one kind's lifecycle: where it starts, and what may follow what.
    Lifecycle {
        /// Where the documents are and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The kind, such as `story`.
        kind: String,
    },
}

/// What `protocol artifact new` needs.
///
/// Its own struct rather than nine fields on the variant, because the verb's handler would
/// otherwise take nine arguments and say nothing more than the struct does.
#[derive(Debug, Args)]
pub(crate) struct NewArgs {
    /// Where the plan is and how to render.
    #[command(flatten)]
    store: StoreArgs,
    /// The kind, such as `story`. Aliases such as `adr` are accepted.
    kind: String,
    /// The name, which becomes the id's name part and the file's stem.
    name: String,
    /// The title, which is what a listing shows.
    ///
    /// Takes a value that begins with `-`. A title or a summary is prose somebody wrote, and prose
    /// starts with a dash often enough — `--summary "--strict is now a flag"` cost `114c2340#196` a
    /// retry, and the workaround, `--summary=…`, is a thing you have to already know.
    #[arg(long, allow_hyphen_values = true)]
    title: String,
    /// A one-line summary.
    ///
    /// Takes a value that begins with `-`, for the reason `--title` does.
    #[arg(long, allow_hyphen_values = true)]
    summary: Option<String>,
    /// Who owns it.
    #[arg(long)]
    owner: Option<String>,
    /// A label. Repeat for more than one.
    #[arg(long = "tag")]
    tag: Vec<String>,
    /// An edge, written `<relation>:<artifact-id>`, such as `derived_from:epic:passwordless`.
    ///
    /// Split at the **first** colon, which is unambiguous because no relation name contains one
    /// and every artifact id does.
    #[arg(long = "relate")]
    relate: Vec<String>,
    /// Read the complete body from this UTF-8 file; `-` reads standard input.
    ///
    /// Without it the kind's template is the body, to be replaced later with `body`. That later
    /// step does not exist for an immutable kind — a `review-result` refuses every edit after the
    /// fact, which is what makes it evidence — so for one of those the body has to arrive with the
    /// record or never arrive at all, and this is how it arrives.
    #[arg(long, value_name = "PATH")]
    from: Option<PathBuf>,
    /// The evidence kind this artifact is stopping anybody from producing, such as `test_result`.
    ///
    /// The join between a blocker and an evidence gate: a rung wants a `test_result`, the job that
    /// would produce one cannot mint a token, and this is where the store records *which* fact is
    /// missing and why. Only meaningful together with `--relate blocks:<id>`, and
    /// `protocol artifact validate` says so.
    #[arg(long, value_name = "EVIDENCE-KIND")]
    withholds: Option<String>,
}

/// How to render the plan's graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum PlanningGraphFormat {
    /// Graphviz DOT, for `dot -Tsvg`.
    Dot,
    /// The artifact graph itself.
    Json,
}

/// `protocol artifact`
pub(crate) fn run(command: ArtifactCommand) -> Result<ExitCode> {
    match command {
        ArtifactCommand::New(args) => create(&args),
        ArtifactCommand::Move {
            store,
            id,
            to,
            evidence,
            at,
            via,
        } => move_status(&store, &id, &to, &evidence, at.as_deref(), via),
        ArtifactCommand::Relate {
            store,
            id,
            relation,
            target,
        } => relate(&store, &id, &relation, target.as_deref()),
        ArtifactCommand::Body {
            store,
            id,
            from,
            append,
            section,
        } => replace_body(
            &store,
            &id,
            &from,
            &BodyEdit::of(append, section.as_deref()),
        ),
        ArtifactCommand::Set {
            store,
            id,
            title,
            summary,
            owner,
            tag,
            untag,
            status,
            revision,
            identity,
            kind,
        } => set(
            &store,
            &id,
            &Fields {
                title,
                summary,
                owner,
                tag,
                untag,
            },
            &[
                ("status", status),
                ("revision", revision),
                ("id", identity),
                ("kind", kind),
            ],
        ),
        ArtifactCommand::Show {
            store,
            id,
            body_only,
        } => show(&store, &id, body_only),
        ArtifactCommand::List {
            store,
            kind,
            status,
        } => list(&store, kind.as_deref(), status.as_deref()),
        ArtifactCommand::Board { store, kind } => board(&store, kind.as_deref()),
        ArtifactCommand::Blocked { store, category } => blocked(&store, category.as_deref()),
        ArtifactCommand::Graph { store, format } => graph(&store, format),
        ArtifactCommand::Validate { store, strict } => validate(&store, strict),
        ArtifactCommand::History { store, id } => history(&store, &id),
        ArtifactCommand::Explain { store, id } => explain(&store, &id),
        ArtifactCommand::Divergences { store } => divergences(&store),
        ArtifactCommand::CatchUp { store } => catch_up(&store),
        ArtifactCommand::Evidence {
            store,
            id,
            kind,
            source,
            reference,
            at,
        } => record_evidence(
            &store,
            &id,
            &kind,
            &source,
            reference.as_deref(),
            at.as_deref(),
        ),
        ArtifactCommand::Kinds { store } => kinds(&store),
        ArtifactCommand::Relations { store } => relations(&store),
        ArtifactCommand::Lifecycle { store, kind } => lifecycle(&store, &kind),
    }
}

/// The members the workspace beside this store declares, if there is one.
///
/// A store with no workspace file declares no members, so every member-qualified target is a
/// dangling edge — which is what makes a misspelled member name a defect rather than a crossing
/// nobody can check.
fn declared_members(root: &Path) -> Vec<aep_domain::workspace::MemberName> {
    // `load_workspace` joins the project directory itself; joining it here too looked right and
    // pointed at `.engineering/.engineering/workspace.yaml`, so every member read as undeclared and
    // every crossing as a dangling edge.
    aep_project::project::load_workspace(root).map_or_else(
        |_| Vec::new(),
        |workspace| {
            workspace.map_or_else(Vec::new, |workspace| {
                workspace
                    .members
                    .iter()
                    .map(|member| member.name.clone())
                    .collect()
            })
        },
    )
}

/// The artifact graph a planning store describes, for the entity surface to seed from.
///
/// Refuses an unreadable store rather than seeding a partial one: an entity surface answering
/// about nine of ten artifacts, with nothing saying which one is missing, is worse than an error.
pub(crate) fn graph_at(root: &Path) -> Result<ArtifactGraph> {
    let store = MarkdownStore::open(root);
    let report = store.load();
    require_clean(&store, &report)?;
    let repository = root
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new("."));
    report
        .graph_in_workspace(declared_members(repository))
        .map_err(|errors| anyhow::anyhow!("{errors}"))
        .with_context(|| format!("reading the planning store at {}", root.display()))
}

/// The instant, as the contract spells one.
///
/// Read here and handed in, never inside a crate that decides anything: a record that dated itself
/// could not be replayed.
pub(crate) fn clock_at_the_edge() -> aep_domain::time::Timestamp {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    aep_domain::time::Timestamp::from_epoch_millis(millis)
}

/// The environment variable a caller declares its actor in.
///
/// Named the way `AEP_DRIVE_PLUGIN_DIR` is: the `AEP_` prefix is this CLI's, and the rest says
/// what the value is. Written by `protocol drive` onto every session it launches
/// (`crate::drive::session_env`) and read here on every store write, so the two ends of the
/// declaration are one constant rather than two string literals.
pub(crate) const ACTOR_ENV: &str = "AEP_ACTOR";

/// Who the command is from.
///
/// Reads [`ACTOR_ENV`] and falls back to `human:<$USER>`. See [`actor_from`] for what each answer
/// means; this half exists only to fetch the two variables, so the deciding half is a function of
/// its arguments and can be tested without writing to an environment every other test shares.
pub(crate) fn command_actor() -> Result<aep_domain::entity::ActorRef> {
    actor_from(
        std::env::var(ACTOR_ENV).ok().as_deref(),
        std::env::var("USER").ok().as_deref(),
    )
}

/// Who a store write is attributed to, given what the caller declared and who is logged in.
///
/// **Three answers, and the middle one is the point.**
///
/// * `declared` is `Some` and parses — that is the actor, whatever it says. A driven session is
///   handed `agent:<execution id>`, so `protocol artifact move` run from inside a run is
///   journalled as the run's act; before this, every write in the store said `human:<$USER>` and
///   the journal could not tell an agent's move from the operator's own.
/// * `declared` is `Some` and does **not** parse — including the empty string, which is what a
///   harness that forgot to fill the variable in leaves behind — and the command is **refused,
///   naming the variable and the value**. Falling back here is the one thing this must not do: it
///   would attribute an agent's write to whoever was logged in, which is exactly the defect.
/// * `declared` is `None` — nothing changes. `human:<$USER>`, sanitised, as it has always been.
///
/// The store still cannot *verify* an identity, and this does not pretend otherwise: it is a
/// declaration, as strong as the rest of the provenance model and no stronger (gap register
/// **D-3**, attestation by signature, stays proposed). What it buys is that an agent can declare
/// something other than a person's name.
fn actor_from(declared: Option<&str>, user: Option<&str>) -> Result<aep_domain::entity::ActorRef> {
    if let Some(declared) = declared {
        return aep_domain::entity::ActorRef::parse(declared).map_err(|error| {
            anyhow::anyhow!(
                "{ACTOR_ENV} is set to `{declared}`, which is not a usable actor: {error}. Unset \
                 it to write as `human:$USER`"
            )
        });
    }
    // An empty `USER` reads the same as no `USER`: an actor named after nobody is not a name.
    let name = user.filter(|name| !name.is_empty()).unwrap_or("unknown");
    let sanitised: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    aep_domain::entity::ActorRef::parse(&format!("human:{sanitised}"))
        .map_err(|error| anyhow::anyhow!("`{name}` is not a usable actor: {error}"))
}

/// The contract backend over this store.
///
/// Hydrated per invocation. That is the cost of one write path rather than two: the store is read,
/// its artifacts become entities through `CreateEntity` commands, and the command this verb issues
/// is decided against them. A CLI that wrote the file directly would skip all of it, which is
/// exactly what D-P1 was.
fn markdown_backend_for(
    args: &StoreLocation,
    root: &Path,
) -> Result<aep_backend_markdown::backend::MarkdownBackend> {
    aep_backend_markdown::backend::MarkdownBackend::open(
        root,
        declared_members(&args.repository_root()),
        clock_at_the_edge(),
        command_actor()?,
        // The ladders this store's kinds declare. Without them the backend falls back to the
        // permissive lifecycle and a status copied out of a command is a transition nothing
        // checked.
        args.lifecycles()?.lifecycles().clone(),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
    .with_context(|| format!("opening the planning store at {}", root.display()))
}

/// One command envelope, with identifiers derived from `name` so a replay is recognisable.
///
/// Shared by every planning verb: four copies of this were four places for the idempotency key to
/// be built differently, and an idempotency key that differs between two calls of the same verb is
/// a replay the contract cannot recognise.
fn envelope_for(
    name: &str,
    correlation: &str,
    wire_name: &'static str,
    payload: aep_domain::command::Command,
    at: aep_domain::time::Timestamp,
) -> Result<aep_contract::command::CommandEnvelope<aep_domain::command::Command>> {
    use aep_contract::command::{CommandContext, CommandEnvelope};

    let safe = name.replace([':', '/'], "-");
    let context = CommandContext::new(
        format!("req-{safe}")
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        safe.parse().map_err(|error| anyhow::anyhow!("{error}"))?,
        command_actor()?,
        correlation
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        at,
    );
    Ok(CommandEnvelope::new(
        format!("cmd-{safe}")
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        wire_name,
        payload,
        context,
    ))
}

/// What a planning document states, as an entity body.
///
/// Status, title, summary, owner, tags — and the **prose**, under `BODY_KEY`. Everything the
/// document says about itself travels as data, so the command carries the whole document and the
/// backend stays the only thing that writes one.
fn entity_body(
    document: &PlanningDocument,
) -> std::collections::BTreeMap<String, aep_domain::node::Node> {
    use aep_domain::node::Node;

    let front = &document.frontmatter;
    let mut data: std::collections::BTreeMap<String, Node> = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from(front.status.as_str()));
    for (key, value) in [
        ("title", front.title.as_deref()),
        ("summary", front.summary.as_deref()),
        ("owner", front.owner.as_deref()),
    ] {
        if let Some(value) = value {
            data.insert(key.to_owned(), Node::from(value));
        }
    }
    if !front.tags.is_empty() {
        data.insert(
            "tags".to_owned(),
            Node::Seq(
                front
                    .tags
                    .iter()
                    .map(|tag| Node::from(tag.as_str()))
                    .collect(),
            ),
        );
    }
    if let Some(withholds) = front.withholds {
        data.insert("withholds".to_owned(), Node::from(withholds.as_str()));
    }
    data.insert(
        aep_backend_markdown::backend::BODY_KEY.to_owned(),
        Node::from(document.body.as_str()),
    );
    data
}

/// Issues the `CreateEntity` command that produces `document`, and returns where it landed.
///
/// The entity carries what the document states — status, title, summary, owner, tags and the
/// **prose**, under `BODY_KEY`. The backend writes the file; nothing here does.
fn write_through_a_command(opened: &Opened, document: &PlanningDocument) -> Result<String> {
    use aep_contract::command::CommandService;
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{EntityLocator, EntityType};
    use aep_domain::node::Node;

    let backend = opened.backend()?;
    let front = &document.frontmatter;
    let data = entity_body(document);

    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        front.id.namespace(),
        front.id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{}` cannot be given an address: {error}", front.id))?;

    // **Every target resolved before anything is written.** They used to be resolved inside the
    // loop, after the create had been committed and journalled — so a typo in `--relate` left the
    // caller told the command failed and holding an artifact without the edge they asked for.
    for relation in &front.relations {
        let target = relation.target.id();
        block_on(QueryService::resolve(
            backend,
            &EntityLocator::new(
                aep_backend_markdown::backend::ORGANISATION,
                aep_backend_markdown::backend::SPACE,
                target.namespace(),
                target.name(),
            )
            .map_err(|error| anyhow::anyhow!("`{target}` cannot be given an address: {error}"))?,
        ))
        .map_err(|error| {
            anyhow::anyhow!(
                "`{}` would point at `{target}`, which this store does not hold: {error}",
                front.id
            )
        })?;
    }

    let name = format!("new-{}", front.id);
    let envelope = envelope_for(
        &name,
        "protocol-artifact-new",
        "aep.entity.create/v1",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse(&format!("aep.{}/v1", front.id.namespace()))
                .map_err(|error| anyhow::anyhow!("{error}"))?,
            locator,
            data: Node::Map(data),
        }),
        clock_at_the_edge(),
    )?;

    let result = block_on(backend.execute(envelope)).map_err(|error| {
        let detail = error.to_string();
        if detail.contains("already addresses an entity") {
            anyhow::anyhow!(
                "`{}` already exists at {} — creating over a document is how a plan item's body \
                 disappears, and there is no undo in a tool that has not committed anything",
                front.id,
                relative_path_for(&front.id)
            )
        } else {
            anyhow::anyhow!("{detail}")
        }
    })?;
    let Some(created) = result.affected.first() else {
        anyhow::bail!("creating `{}` reported no entity", front.id);
    };

    // The edges, each its own command — the same one `protocol artifact relate` issues, because
    // an edge created at birth and an edge added later are the same act.
    let _ = created;
    for relation in &front.relations {
        relate_through_a_command(backend, &front.id, relation.kind, relation.target.id())?;
    }

    Ok(opened.path_of(&front.id))
}

/// `protocol artifact new`
fn create(args: &NewArgs) -> Result<ExitCode> {
    let kind = ArtifactKind::parse(&args.kind).map_err(|error| anyhow::anyhow!("{error}"))?;
    // The id's namespace is the kind's *canonical* name, whichever spelling was typed, so `adr` and
    // `architecture-decision-record` produce one id rather than two for one thing.
    let id = ArtifactId::new(format!("{}:{}", kind.as_str(), args.name))
        .map_err(|error| anyhow::anyhow!("{error}"))?;

    let document_root = args.store.location.document_root()?;
    let registry = crate::load(&document_root)?;
    let status = registry
        .lifecycles()
        .for_kind(&kind)
        .map_or(ArtifactStatus::Draft, |lifecycle| lifecycle.initial.clone());

    let mut frontmatter = PlanningFrontmatter::new(id.clone(), kind.clone(), status.clone());
    frontmatter.title = Some(args.title.clone());
    frontmatter.summary.clone_from(&args.summary);
    frontmatter.owner.clone_from(&args.owner);
    frontmatter.tags = args.tag.iter().cloned().collect();
    if let Some(value) = &args.withholds {
        frontmatter.withholds = Some(
            aep_domain::evidence::EvidenceKind::parse(value)
                .map_err(|error| anyhow::anyhow!("{error}"))?,
        );
    }
    for value in &args.relate {
        let (relation, target) = parse_relation(value)?;
        frontmatter
            .relations
            .push(aep_domain::artifact::ArtifactRelation::new(
                relation, target,
            ));
    }

    // The body arrives with the record when the caller has one. It matters most for the kind that
    // cannot take one later: `body` is refused on a `review-result` by design, so without this the
    // only review `new` could record was the template.
    let body = match &args.from {
        Some(from) => read_body(from)?,
        None => template(&document_root, &kind).unwrap_or_else(|| format!("# {}\n", args.title)),
    };
    let document = PlanningDocument::new(frontmatter, body);

    // **Through a command, not through the store.** This is what D-P1 was: a second write path is a
    // second place for idempotency, revision checks and the audit record to be forgotten, and
    // invariant 14 gives state change exactly one door. The document above is what the command has
    // to produce, not what gets written — `MarkdownBackend` writes it, from the entity.
    let opened = open(&args.store.location, true)?;
    let path = write_through_a_command(&opened, &document)?;
    let relative = relative_path_for(&id);

    match args.store.format {
        Format::Text => outln!("created {id} ({status}) at {path}"),
        Format::Yaml | Format::Json => crate::print_serialised(
            &Created {
                id: id.to_string(),
                kind: kind.to_string(),
                status: status.as_str().to_owned(),
                path: relative,
            },
            args.store.format,
        )?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Issues the `MoveStatus` command that records one decided move.
///
/// The decision is not taken here and not taken by the backend — it was taken by the engine against
/// the kind's lifecycle document before this is called. What travels is the move and its account.
fn move_through_a_command(
    backend: &PlanBackend,
    id: &ArtifactId,
    to: &ArtifactStatus,
    decided_on: &aep_backend_markdown::journal::Provenance,
) -> Result<()> {
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, MoveStatus};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        id.namespace(),
        id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))?;
    let target = block_on(QueryService::resolve(backend, &locator))
        .map_err(|error| anyhow::anyhow!("`{id}` is not in this store: {error}"))?;

    // **As text, not as a `Node` tree.** `Node`'s numbers are floating point, so an evidence count
    // of `1` travels out and comes back as `1.0`, and a count that is not an integer is not a
    // count. Both ends used to swallow that with `.ok()`, which turned a decoding failure into a
    // move that looked exactly as well founded as one nobody had any evidence for.
    let account = Some(aep_domain::node::Node::from(
        serde_json::to_string(decided_on)
            .map_err(|error| anyhow::anyhow!("the account could not be written: {error}"))?
            .as_str(),
    ));

    let at = clock_at_the_edge();
    let name = format!("move-{id}").replace([':', '/'], "-");
    let context = CommandContext::new(
        format!("req-{name}")
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        format!("{name}-{to}-{}", at.epoch_millis())
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        command_actor()?,
        "protocol-artifact-move"
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        at,
    );
    block_on(
        backend.execute(CommandEnvelope::new(
            format!("cmd-{name}-{}", at.epoch_millis())
                .parse()
                .map_err(|error| anyhow::anyhow!("{error}"))?,
            "aep.status.move/v1",
            Command::MoveStatus(MoveStatus {
                target: EntityRef::new(target),
                to: to.as_str().to_owned(),
                expected_revision: None,
                decided_on: account,
            }),
            context,
        )),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(())
}

/// What the graph rules say about a store, each finding as `validate` would print it.
///
/// A store whose documents do not build a graph at all answers **nothing** rather than everything:
/// a dangling edge is a defect `validate` reports and is not a reason to refuse an unrelated status
/// move, and comparing "no graph" to "no graph" the way a caller compares two of these would
/// otherwise turn every such store into a refusal.
fn lifecycle_findings(
    report: &StoreReport,
    repository: &Path,
    lifecycles: &aep_domain::artifact::LifecycleRegistry,
) -> Vec<String> {
    report
        .graph_in_workspace(declared_members(repository))
        .map(|graph| {
            graph
                .validate_lifecycles(lifecycles)
                .as_slice()
                .iter()
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `protocol artifact move`
/// What a caller is asking a move to do.
///
/// Bundled rather than passed as six arguments, so a caller cannot silently swap `to` for `now` —
/// both are `&str`, and the compiler would have nothing to say about it.
struct MoveRequest<'a> {
    /// The artifact to move.
    id: &'a ArtifactId,
    /// The status it is asked to reach, as the caller spelled it.
    to: &'a str,
    /// Evidence the caller asserted, which is kept apart from what the store recorded.
    asserted: aep_backend_markdown::kernel::EvidenceOnHand,
    /// The instant the edge read, so a dated rung is judged against one moment.
    now: &'a str,
    /// Walk the ladder's own route rather than requiring one hop.
    via: bool,
}

/// What a move did, and — when it stopped — why.
///
/// Both halves, because `--via` walks several rungs: a walk that commits two hops and is refused at
/// the third **made two real moves**, and a return type that carried only the refusal would report a
/// store that does not exist. The caller needs the hops before it needs the reason.
#[derive(Debug)]
struct MoveOutcome {
    /// The hops that were written, in the order they were written.
    made: Vec<Moved>,
    /// What the decision rested on, kept so a caller can say whether it leaned on an assertion.
    decided_on: aep_backend_markdown::journal::Provenance,
    /// Why the walk stopped, when it did. `None` is every requested hop made.
    refusal: Option<MoveStopped>,
}

/// Why a walk stopped.
///
/// Separate from [`anyhow::Error`] on the distinction the CLI already draws in its exit codes and an
/// HTTP caller draws in its status codes: an `Err` is *this is not a question* — no such artifact, a
/// status no ladder declares, a store that will not read — and a `MoveStopped` is *the answer is no*.
#[derive(Debug)]
enum MoveStopped {
    /// The kind's ladder refused the rung, and said what it would have permitted.
    Refused {
        /// Where the artifact stood when it was refused.
        from: ArtifactStatus,
        /// The refusal, carrying every legal target.
        refusal: Box<aep_backend_markdown::document::MoveRefusal>,
    },
    /// `--via` reached a guarded rung. A walk crosses rungs nothing guards.
    GuardedRungOnAWalk {
        /// Where the artifact stood.
        from: ArtifactStatus,
        /// The rung that is guarded.
        rung: ArtifactStatus,
        /// The rung's own refusal, when it had one to give.
        refusal: Option<Box<aep_backend_markdown::document::MoveRefusal>>,
    },
    /// The ladder permits it and the store it would leave does not validate.
    WouldNotValidate {
        /// Where the artifact stood.
        from: ArtifactStatus,
        /// Where this hop would have taken it.
        to: ArtifactStatus,
        /// The finding the would-be store reports, which it did not report before.
        finding: String,
    },
}

/// Decides a move against the kind's ladder and writes the hops it permits.
///
/// Every rule lives here and none of it prints: the ladder, `--via`'s walk, the guarded-rung rule,
/// the store-wide re-validation, and the command that writes. Two callers share it — the CLI verb,
/// which renders the outcome as lines and an exit code, and anything else that needs the same
/// decision to answer the same way. A second caller that assembled these steps itself would be a
/// second protocol, which is the thing `Command::MoveStatus` refuses to become.
///
/// The clock is **not** read here. It arrives from the edge, so the instant that decided is the one
/// the caller can print.
fn decide_and_move(
    opened: &mut Opened,
    registry: &aep_engine::Registry,
    repository: &Path,
    asked: MoveRequest<'_>,
) -> Result<MoveOutcome> {
    let MoveRequest {
        id,
        to,
        asserted,
        now,
        via,
    } = asked;
    // What the whole store already reports, before this move. Taken here because the decision below
    // mutates the document in place, and *new* is the only interesting word in the comparison: a
    // store that was already reporting a finding must not have this move refused for it.
    let before = lifecycle_findings(&opened.report, repository, registry.lifecycles());

    // Evidence recorded *about this artifact* is found rather than typed. Both origins are kept
    // apart all the way through the decision and into the journal, so the history can say what the
    // move rested on — see `journal::Provenance`. Built here rather than passed in, so a caller
    // cannot hand over a provenance that disagrees with the evidence the decision used.
    let decided_on = aep_backend_markdown::journal::Provenance {
        recorded: opened.evidence_on_hand(id)?,
        asserted,
    };
    let evidence = decided_on.total();

    let not_here = opened.missing(id);
    let stored = opened
        .report
        .documents
        .get_mut(id)
        .with_context(|| not_here)?;
    let (standing, ladder, hops) = plan_the_walk(&stored.document, registry, to, via)?;

    // **A walk crosses rungs nothing guards, and stops at the first one that is.** `--via` is for
    // the ceremony a ladder makes somebody type — `draft → proposed → active` is two commands per
    // story on every wave (`8cffc110#184`) — and not for getting past a rung that asks for
    // something. A guarded rung is refused in the words that rung would have used.
    if let Some(stopped) = guarded_rung_on_the_walk(
        &hops,
        &ladder,
        &stored.document,
        registry,
        &evidence,
        now,
        &standing,
    ) {
        return Ok(MoveOutcome {
            made: Vec::new(),
            decided_on,
            refusal: Some(stopped),
        });
    }

    let mut made: Vec<Moved> = Vec::new();
    for rung in &hops {
        // Recomputed per hop, because the store the next hop would leave is the store this one
        // left. The comparison is still *new since a moment ago*, which is the only useful one.
        let before = if made.is_empty() {
            before.clone()
        } else {
            lifecycle_findings(&opened.report, repository, registry.lifecycles())
        };
        let not_here = opened.missing(id);
        let path = opened.path_of(id);
        let stored = opened
            .report
            .documents
            .get_mut(id)
            .with_context(|| not_here)?;
        let from = stored.document.frontmatter.status.clone();
        let relative = stored.relative_path.clone();

        if let Err(refusal) =
            stored
                .document
                .move_status(rung.clone(), registry.lifecycles(), &evidence, Some(now))
        {
            return Ok(MoveOutcome {
                made,
                decided_on,
                refusal: Some(MoveStopped::Refused { from, refusal }),
            });
        }
        let document = stored.document.clone();

        // **The rules the store would be judged by, run on the store this hop would leave.** The
        // ladder says a move is legal; the *graph* says whether the result is a plan that
        // validates, and until now only `validate` asked it — so `move --to active` succeeded and
        // `validate` immediately reported `[empty_declaration] … is active and serves no
        // objective`, which is one command creating work for the next one (`114c2340#92`,
        // `4d4c15a4#149`). The document has been moved in memory and nothing has been written, so
        // `opened.report` *is* the would-be store.
        let after = lifecycle_findings(&opened.report, repository, registry.lifecycles());
        if let Some(finding) = after
            .iter()
            .find(|finding| !before.contains(*finding) && finding.contains(&id.to_string()))
        {
            return Ok(MoveOutcome {
                made,
                decided_on,
                refusal: Some(MoveStopped::WouldNotValidate {
                    from,
                    to: rung.clone(),
                    finding: finding.clone(),
                }),
            });
        }

        // Through the command the vocabulary gained for this. The engine has already decided the
        // move above, against the kind's ladder and the evidence presented; what crosses here is
        // the decision and the account it rested on. `MarkdownBackend` writes the file and
        // journals it — **once per hop**, so a walk leaves the same record two commands would.
        let _ = relative;
        move_through_a_command(opened.backend()?, id, rung, &decided_on)?;
        made.push(Moved {
            id: id.to_string(),
            from: from.as_str().to_owned(),
            to: rung.as_str().to_owned(),
            revision: document.frontmatter.revision,
            path,
        });
    }

    Ok(MoveOutcome {
        made,
        decided_on,
        refusal: None,
    })
}

/// Where the artifact stands, the ladder it stands on, and the rungs a move would cross.
///
/// The target is read *after* the artifact, because what a status name may be is decided by the
/// ladder this kind declares and not by a list compiled into this binary. `ArtifactStatus` is an
/// open vocabulary; the ladder is what keeps it open to authors and closed to typos.
///
/// Without `--via` the walk is one rung — the one the caller named. With it, the ladder's own
/// shortest route there, and `rungs_between` is breadth-first over `BTreeSet`s so two routes of
/// equal length resolve the same way on every machine.
fn plan_the_walk(
    document: &PlanningDocument,
    registry: &aep_engine::Registry,
    to: &str,
    via: bool,
) -> Result<(ArtifactStatus, ArtifactLifecycle, Vec<ArtifactStatus>)> {
    let standing = document.frontmatter.status.clone();
    let kind = document.frontmatter.kind.clone();
    let to = parse_status_in(to, &kind, registry.lifecycles())?;

    let permissive = ArtifactLifecycle::permissive();
    let ladder = registry
        .lifecycles()
        .for_kind(&kind)
        .unwrap_or(&permissive)
        .clone();
    let hops = if via {
        rungs_between(&ladder, &standing, &to).unwrap_or_else(|| vec![to.clone()])
    } else {
        vec![to]
    };
    Ok((standing, ladder, hops))
}

/// The first guarded rung a walk would cross, when there is one.
///
/// **A walk crosses rungs nothing guards, and stops at the first one that is.** `--via` is for the
/// ceremony a ladder makes somebody type — `draft → proposed → active` is two commands per story on
/// every wave (`8cffc110#184`) — and not for getting past a rung that asks for something. Only the
/// rungs *before* the last are checked here: the last is the one the caller named, and it is
/// answered by moving to it.
fn guarded_rung_on_the_walk(
    hops: &[ArtifactStatus],
    ladder: &ArtifactLifecycle,
    document: &PlanningDocument,
    registry: &aep_engine::Registry,
    evidence: &aep_backend_markdown::kernel::EvidenceOnHand,
    now: &str,
    standing: &ArtifactStatus,
) -> Option<MoveStopped> {
    for rung in hops.iter().take(hops.len().saturating_sub(1)) {
        if ladder.requirements_for(rung).is_empty() && ladder.timing_for(rung).is_none() {
            continue;
        }
        // The probe carries the caller's own evidence, so the refusal is true: a caller who
        // presented nothing reads that rung's "not yet earned", and one who presented enough reads
        // that the walk is what is refused, not the evidence.
        let mut probe = document.clone();
        let refusal = probe
            .move_status(rung.clone(), registry.lifecycles(), evidence, Some(now))
            .err();
        return Some(MoveStopped::GuardedRungOnAWalk {
            from: standing.clone(),
            rung: rung.clone(),
            refusal,
        });
    }
    None
}

/// Moves an artifact along its kind's ladder, and says what it did.
///
/// The decision is [`decide_and_move`]'s; this reads the clock at the edge, opens the store, and
/// renders the outcome as the lines and exit code the terminal expects.
fn move_status(
    args: &StoreArgs,
    id: &str,
    to: &str,
    evidence: &[String],
    at: Option<&str>,
    via: bool,
) -> Result<ExitCode> {
    let asserted = parse_evidence(evidence)?;
    // The clock, read once, here. `aep-domain` has no clock and neither does the backend; this is
    // the edge, and the instant it read is printed with any dated refusal so a reader can see which
    // moment decided.
    let now = match at {
        Some(given) => given.to_owned(),
        None => now_at_the_edge(),
    };
    let id = artifact_id(id)?;
    let registry = args.lifecycles()?;

    let mut opened = open(&args.location, true)?;
    let repository = args.repository_root();

    let outcome = decide_and_move(
        &mut opened,
        &registry,
        &repository,
        MoveRequest {
            id: &id,
            to,
            asserted,
            now: &now,
            via,
        },
    )?;

    // Reported before the refusal, because a walk that made two hops and stopped at the third made
    // two real moves and the reader has to see them first.
    report_moves(args, &id, &outcome.made, &outcome.decided_on)?;
    let Some(stopped) = outcome.refusal else {
        return Ok(ExitCode::SUCCESS);
    };
    match stopped {
        MoveStopped::Refused { from, refusal }
        | MoveStopped::GuardedRungOnAWalk {
            from,
            refusal: Some(refusal),
            ..
        } => outln!("{id} is {from}; {refusal}"),
        MoveStopped::GuardedRungOnAWalk {
            from,
            rung,
            refusal: None,
        } => outln!(
            "{id} is {from}; `--via` walks rungs nothing guards, and {rung} is guarded — \
             move it there on its own, with what that rung asks for"
        ),
        MoveStopped::WouldNotValidate { from, to, finding } => {
            outln!("{id} would move {from} -> {to}, and the store would not validate:");
            outln!("  - {finding}");
        }
    }
    Ok(crate::exit_code(false))
}

/// The rungs from `from` to `to`, `to` last and `from` left out, or `None` when the ladder has no
/// route.
///
/// Breadth-first, so the answer is the **shortest** walk, and over `BTreeSet`s, so two routes of
/// equal length resolve the same way on every machine — a status move that depended on hash order
/// would be a different plan on a different day.
fn rungs_between(
    ladder: &ArtifactLifecycle,
    from: &ArtifactStatus,
    to: &ArtifactStatus,
) -> Option<Vec<ArtifactStatus>> {
    if from == to {
        return None;
    }
    let mut seen: BTreeSet<ArtifactStatus> = BTreeSet::new();
    let mut queue: std::collections::VecDeque<Vec<ArtifactStatus>> =
        std::collections::VecDeque::new();
    seen.insert(from.clone());
    queue.push_back(Vec::new());
    while let Some(walked) = queue.pop_front() {
        let at = walked.last().unwrap_or(from);
        for next in ladder.transitions.get(at).into_iter().flatten() {
            if !seen.insert(next.clone()) {
                continue;
            }
            let mut extended = walked.clone();
            extended.push(next.clone());
            if next == to {
                return Some(extended);
            }
            queue.push_back(extended);
        }
    }
    None
}

/// Prints the moves that were made, in the format the caller asked for.
///
/// Shared by the successful walk and by the refusal that stops one part-way, because a hop that was
/// committed is a hop the caller has to be told about — a refusal printed alone would leave a story
/// somewhere nobody said it was.
fn report_moves(
    args: &StoreArgs,
    id: &ArtifactId,
    made: &[Moved],
    decided_on: &aep_backend_markdown::journal::Provenance,
) -> Result<()> {
    if made.is_empty() {
        return Ok(());
    }
    match args.format {
        Format::Text => {
            for moved in made {
                outln!(
                    "{id} moved {} -> {} (revision {})",
                    moved.from,
                    moved.to,
                    moved.revision
                );
            }
            if decided_on.leans_on_an_assertion() {
                let asserted: Vec<String> = decided_on
                    .asserted
                    .iter()
                    .map(|(kind, count)| format!("{}={count}", kind.as_str()))
                    .collect();
                outln!(
                    "  decided partly on asserted evidence nothing checks: {}",
                    asserted.join(", ")
                );
                outln!("  `protocol artifact evidence {id} --kind <kind> --source <where>` records it instead");
            }
        }
        // One move is one object, as it has always been; a walk is the list it actually was.
        // Collapsing a walk into its last hop would report a plan that never happened.
        Format::Yaml | Format::Json => match made {
            [only] => crate::print_serialised(only, args.format)?,
            several => crate::print_serialised(&several, args.format)?,
        },
    }
    Ok(())
}

/// Issues the `CreateRelation` command that adds one edge.
///
/// The journal entry comes from `MarkdownBackend`, which is what makes the record and the write one
/// act rather than two things a verb has to remember to do in order.
fn relate_through_a_command(
    backend: &PlanBackend,
    source: &ArtifactId,
    relation: RelationKind,
    target: &ArtifactId,
) -> Result<()> {
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateRelation};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let address = |id: &ArtifactId| {
        EntityLocator::new(
            aep_backend_markdown::backend::ORGANISATION,
            aep_backend_markdown::backend::SPACE,
            id.namespace(),
            id.name(),
        )
        .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))
    };
    let resolve = |id: &ArtifactId| -> Result<aep_domain::entity::EntityId> {
        block_on(QueryService::resolve(backend, &address(id)?))
            .map_err(|error| anyhow::anyhow!("`{id}` is not in this store: {error}"))
    };

    let name = format!("{source}-{relation}-{target}").replace([':', '/'], "-");
    let context = CommandContext::new(
        format!("req-{name}")
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        format!("rel-{name}")
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        command_actor()?,
        "protocol-artifact-relate"
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        clock_at_the_edge(),
    );
    block_on(
        backend.execute(CommandEnvelope::new(
            format!("cmd-{name}")
                .parse()
                .map_err(|error| anyhow::anyhow!("{error}"))?,
            "aep.relation.create/v1",
            Command::CreateRelation(CreateRelation {
                kind: relation,
                source: EntityRef::new(resolve(source)?),
                target: EntityRef::new(resolve(target)?),
            }),
            context,
        )),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(())
}

/// `protocol artifact relate`
///
/// `target` is absent when the caller wrote the edge as one word — `<relation>:<target>` — which is
/// the spelling `new --relate` takes, and is split here by the very same [`parse_relation`], so the
/// two verbs cannot drift apart in what they accept.
fn relate(args: &StoreArgs, id: &str, relation: &str, target: Option<&str>) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    let (relation, target) = match target {
        Some(target) => (
            RelationKind::parse(relation).map_err(|error| anyhow::anyhow!("{error}"))?,
            ArtifactRef::parse(target).map_err(|error| anyhow::anyhow!("{error}"))?,
        ),
        None => parse_relation(relation)?,
    };

    let mut opened = open(&args.location, true)?;

    if !opened.report.documents.contains_key(target.id()) {
        bail!(
            "{} does not hold `{}`, so `{id} {relation} {target}` would be an edge to nothing",
            opened.plan.describe(),
            target.id()
        );
    }

    let not_here = opened.missing(&id);
    let stored = opened
        .report
        .documents
        .get_mut(&id)
        .with_context(|| not_here)?;
    if !stored.document.add_relation(relation, target.clone()) {
        outln!("{id} already declares {relation} {target}; nothing to do");
        return Ok(ExitCode::SUCCESS);
    }
    let relative = stored.relative_path.clone();
    let document = stored.document.clone();

    // Checked before it is written, not after: a cycle is only visible from the whole graph, and a
    // store that has to be repaired by hand after an edge went in is a store people stop using.
    if let Err(errors) = opened
        .report
        .graph_in_workspace(declared_members(&args.repository_root()))
    {
        outln!("`{id} {relation} {target}` would not build a graph:");
        for error in errors.as_slice() {
            outln!("  - {error}");
        }
        return Ok(crate::exit_code(false));
    }

    // Through a command. The edge the contract creates is what `MarkdownBackend` projects into
    // frontmatter, so the document above is what this verb had to check a graph against — not what
    // gets written.
    let _ = relative;
    relate_through_a_command(opened.backend()?, &id, relation, target.id())?;
    match args.format {
        Format::Text => outln!(
            "{id} {relation} {target} (revision {})",
            document.frontmatter.revision
        ),
        Format::Yaml | Format::Json => crate::print_serialised(
            &Related {
                id: id.to_string(),
                relation: relation.as_str(),
                target: target.to_string(),
                revision: document.frontmatter.revision,
            },
            args.format,
        )?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Issues an `UpdateEntity` command carrying `changes`.
///
/// The one door for a structural change. `MarkdownBackend` writes the file and journals it, so the
/// record and the write are one act rather than two things a verb has to remember to do in order.
fn update_through_a_command(
    backend: &PlanBackend,
    id: &ArtifactId,
    changes: impl IntoIterator<Item = (String, aep_domain::node::Node)>,
    correlation: &str,
) -> Result<()> {
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, UpdateEntity};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        id.namespace(),
        id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))?;
    let target = block_on(QueryService::resolve(backend, &locator))
        .map_err(|error| anyhow::anyhow!("`{id}` is not in this store: {error}"))?;

    let at = clock_at_the_edge();
    let name = format!("{correlation}-{id}").replace([':', '/'], "-");
    let context = CommandContext::new(
        format!("req-{name}")
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        format!("upd-{name}-{}", at.epoch_millis())
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        command_actor()?,
        correlation
            .parse()
            .map_err(|error| anyhow::anyhow!("{error}"))?,
        at,
    );
    block_on(
        backend.execute(CommandEnvelope::new(
            format!("cmd-{name}-{}", at.epoch_millis())
                .parse()
                .map_err(|error| anyhow::anyhow!("{error}"))?,
            "aep.entity.update/v1",
            Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(target),
                changes: changes.into_iter().collect(),
            }),
            context,
        )),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(())
}

/// The complete body a verb was handed: a file, or standard input when `from` is `-`.
///
/// Shared by `new --from` and `body --from`, so the two moments a body can reach the store read it
/// the same way, and a body that arrives at birth is the bytes it would have been a revision later.
fn read_body(from: &Path) -> Result<String> {
    if from == Path::new("-") {
        let mut body = String::new();
        std::io::stdin()
            .read_to_string(&mut body)
            .context("reading the body from standard input")?;
        Ok(body)
    } else {
        std::fs::read_to_string(from)
            .with_context(|| format!("reading the body from {}", from.display()))
    }
}

/// Which of the three ways `body` was asked to arrive at a new body.
///
/// One enum rather than two booleans on the handler, because *replace the whole thing*, *add to the
/// end* and *replace one section* are three states and two booleans are four.
#[derive(Debug, Clone)]
enum BodyEdit {
    /// `--from` is the whole body.
    Whole,
    /// `--from` goes on the end of what is there.
    Append,
    /// `--from` is the prose under one `##` heading.
    Section(String),
}

impl BodyEdit {
    /// What the flags asked for. `--append` and `--section` are mutually exclusive at the parser.
    fn of(append: bool, section: Option<&str>) -> Self {
        match (append, section) {
            (_, Some(heading)) => Self::Section(heading.to_owned()),
            (true, None) => Self::Append,
            (false, None) => Self::Whole,
        }
    }

    /// How the journal's correlation names this write.
    fn correlation(&self) -> &'static str {
        match self {
            Self::Whole => "protocol-artifact-body",
            Self::Append => "protocol-artifact-body-append",
            Self::Section(_) => "protocol-artifact-body-section",
        }
    }

    /// What the verb says it did.
    fn past_tense(&self) -> String {
        match self {
            Self::Whole => "body replaced".to_owned(),
            Self::Append => "body appended to".to_owned(),
            Self::Section(heading) => format!("`## {heading}` written"),
        }
    }
}

/// The body `edit` makes of `existing` given the bytes the caller handed in.
///
/// No markdown parser, deliberately: a heading is a line that starts with `##`, which is what the
/// documents in this store actually are, and a parser here would be a second opinion about what a
/// planning document is. The section runs from its heading to the next heading at the same level or
/// above, or to the end.
fn edited_body(existing: &str, arriving: &str, edit: &BodyEdit) -> String {
    match edit {
        BodyEdit::Whole => arriving.to_owned(),
        BodyEdit::Append => {
            if existing.trim().is_empty() {
                return arriving.to_owned();
            }
            format!(
                "{}\n\n{}",
                existing.trim_end(),
                arriving.trim_start_matches('\n')
            )
        }
        BodyEdit::Section(heading) => section_written(existing, heading, arriving),
    }
}

/// `existing` with the prose under `## <heading>` replaced by `arriving`, or that section added.
fn section_written(existing: &str, heading: &str, arriving: &str) -> String {
    let wanted = format!("## {}", heading.trim());
    let lines: Vec<&str> = existing.lines().collect();
    let Some(at) = lines
        .iter()
        .position(|line| line.trim_end() == wanted.as_str())
    else {
        // No such heading: the section is added at the end, which is what a caller asking for a
        // section a document does not have meant. Refusing would send them back to a heredoc.
        let head = if existing.trim().is_empty() {
            String::new()
        } else {
            format!("{}\n\n", existing.trim_end())
        };
        return format!("{head}{wanted}\n\n{}\n", arriving.trim());
    };
    // The next heading at this level or above ends the section; anything deeper is inside it.
    let ends_at = lines
        .iter()
        .enumerate()
        .skip(at + 1)
        .find(|(_, line)| {
            let trimmed = line.trim_start();
            trimmed.starts_with("# ") || trimmed.starts_with("## ")
        })
        .map_or(lines.len(), |(index, _)| index);

    let mut out = String::new();
    for line in &lines[..=at] {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, "\n{}", arriving.trim());
    if ends_at < lines.len() {
        let _ = writeln!(out);
        for line in &lines[ends_at..] {
            let _ = writeln!(out, "{line}");
        }
    }
    out
}

/// `protocol artifact body`
fn replace_body(args: &StoreArgs, id: &str, from: &Path, edit: &BodyEdit) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    let arriving = read_body(from)?;
    // **An empty body is not a body.** `11727595#10819`: `body --from -` on empty standard input
    // wrote nothing over the prose and bumped the revision, which is a document destroyed and a
    // record saying somebody meant to. The flag is named because the flag is what went wrong — a
    // pipe that produced nothing, a file that was not the one intended.
    if arriving.trim().is_empty() {
        anyhow::bail!(
            "`--from {}` holds no body; a planning document is its prose, and writing an empty one \
             over it is not an edit anything can undo",
            from.display()
        );
    }

    let mut opened = open(&args.location, true)?;
    let not_here = opened.missing(&id);
    let stored = opened
        .report
        .documents
        .get_mut(&id)
        .with_context(|| not_here)?;
    let body = edited_body(&stored.document.body, &arriving, edit);
    if !stored.document.replace_body(body) {
        outln!("{id} already has those body bytes; nothing to do");
        return Ok(ExitCode::SUCCESS);
    }

    let relative = stored.relative_path.clone();
    let document = stored.document.clone();
    // Through a command, carrying the prose as data — which is the whole reason `BODY_KEY` exists.
    // A verb that wrote the body directly is the second write path invariant 14 forbids, and a body
    // is the one thing a planning document is *for*.
    let _ = relative;
    update_through_a_command(
        opened.backend()?,
        &id,
        [(
            aep_backend_markdown::backend::BODY_KEY.to_owned(),
            aep_domain::node::Node::from(document.body.as_str()),
        )],
        edit.correlation(),
    )?;
    let path = opened.path_of(&id);
    match args.format {
        Format::Text => outln!(
            "{id} {} (revision {}) at {path}",
            edit.past_tense(),
            document.frontmatter.revision
        ),
        Format::Yaml | Format::Json => crate::print_serialised(
            &BodyReplaced {
                id: id.to_string(),
                revision: document.frontmatter.revision,
                path: relative,
            },
            args.format,
        )?,
    }
    Ok(ExitCode::SUCCESS)
}

/// The frontmatter fields `set` may change.
///
/// Its own struct rather than five arguments on the handler, for the reason [`NewArgs`] is one.
#[derive(Debug)]
struct Fields {
    /// The new title, when one was given.
    title: Option<String>,
    /// The new summary, when one was given.
    summary: Option<String>,
    /// The new owner, when one was given.
    owner: Option<String>,
    /// Labels to add.
    tag: Vec<String>,
    /// Labels to remove.
    untag: Vec<String>,
}

impl Fields {
    /// `true` when the caller named no field at all.
    fn nothing_named(&self) -> bool {
        self.title.is_none()
            && self.summary.is_none()
            && self.owner.is_none()
            && self.tag.is_empty()
            && self.untag.is_empty()
    }
}

/// Why `set` will not change a field, in the words a reader can act on.
///
/// Each is a flag `set` accepts and refuses, rather than one `clap` reports as unrecognised: the
/// person typing `--status` has a question — *how do I change a status* — and `unexpected argument`
/// answers none of it.
fn not_a_field_set_changes(name: &str) -> String {
    match name {
        "status" => "`status` is not a field `set` changes: a status is a decision taken against \
                     the kind's lifecycle, and `protocol artifact move <id> --to <status>` is what \
                     takes it and records what it rested on"
            .to_owned(),
        "revision" => {
            "`revision` is not a field `set` changes: it is the store's own count of the \
                       writes it made, and a document claiming one no write produced is what \
                       `protocol artifact validate` reports as a forged revision"
                .to_owned()
        }
        other => format!(
            "`{other}` is not a field `set` changes: an artifact's id and kind are its identity, \
             fixed at `protocol artifact new` — create the artifact the new name calls for and \
             relate this one to it"
        ),
    }
}

/// `protocol artifact set`
///
/// Frontmatter through the same door as prose. `refused` carries the flags this verb accepts only
/// in order to say why it will not honour them; each is `None` on every call that meant anything.
fn set(
    args: &StoreArgs,
    id: &str,
    fields: &Fields,
    refused: &[(&str, Option<String>)],
) -> Result<ExitCode> {
    for (name, given) in refused {
        if given.is_some() {
            anyhow::bail!("{}", not_a_field_set_changes(name));
        }
    }
    if fields.nothing_named() {
        anyhow::bail!(
            "nothing to set; name a field, such as `--title`, `--summary`, `--owner`, `--tag` or \
             `--untag`"
        );
    }
    let id = artifact_id(id)?;

    let opened = open(&args.location, true)?;
    let stored = opened
        .report
        .documents
        .get(&id)
        .with_context(|| opened.missing(&id))?;
    let front = &stored.document.frontmatter;

    // Only what differs travels. A command carrying a title the document already has is a write
    // with nothing in it and a revision nobody can explain — the same reasoning `replace_body`
    // gives for identical bytes.
    let mut changes: Vec<(String, aep_domain::node::Node)> = Vec::new();
    let mut named: Vec<String> = Vec::new();
    for (key, wanted, held) in [
        ("title", fields.title.as_deref(), front.title.as_deref()),
        (
            "summary",
            fields.summary.as_deref(),
            front.summary.as_deref(),
        ),
        ("owner", fields.owner.as_deref(), front.owner.as_deref()),
    ] {
        if let Some(wanted) = wanted {
            if held != Some(wanted) {
                changes.push((key.to_owned(), aep_domain::node::Node::from(wanted)));
                named.push(key.to_owned());
            }
        }
    }
    if !fields.tag.is_empty() || !fields.untag.is_empty() {
        let mut tags = front.tags.clone();
        for tag in &fields.tag {
            tags.insert(tag.clone());
        }
        for tag in &fields.untag {
            tags.remove(tag);
        }
        if tags != front.tags {
            changes.push((
                "tags".to_owned(),
                aep_domain::node::Node::Seq(
                    tags.iter()
                        .map(|tag| aep_domain::node::Node::from(tag.as_str()))
                        .collect(),
                ),
            ));
            named.push("tags".to_owned());
        }
    }

    if changes.is_empty() {
        outln!("{id} already reads that way; nothing to do");
        return Ok(ExitCode::SUCCESS);
    }

    // The revision the write will land at. The backend counts it as `existing + 1` and nothing
    // here writes the document, so this is what the file says a moment later — read back rather
    // than guessed only if that stops being true.
    let revision = front.revision.saturating_add(1);
    let relative = stored.relative_path.clone();
    update_through_a_command(opened.backend()?, &id, changes, "protocol-artifact-set")?;

    let path = opened.path_of(&id);
    match args.format {
        Format::Text => outln!(
            "{id} {} set (revision {revision}) at {path}",
            named.join(", ")
        ),
        Format::Yaml | Format::Json => crate::print_serialised(
            &FieldsSet {
                id: id.to_string(),
                fields: named,
                revision,
                path: relative,
            },
            args.format,
        )?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol artifact show`
///
/// One artifact, printed: the frontmatter fields a reader asks about, then the body as the store
/// holds it. Read through [`open`] like every other read, so markdown, SQLite, Postgres and a
/// hybrid answer the same way and nothing here goes near a file.
///
/// The plan is opened **without** a backend, as `list` and `board` are: this verb answers from the
/// documents, and building the contract would refuse a markdown plan on account of some *other*
/// document being unreadable — a refusal that has nothing to do with the id that was asked for.
///
/// What it does not print is `extra`, the frontmatter keys this format does not name. They are a
/// markdown document's own, kept so a round trip loses nothing, and a plan that keeps no documents
/// has never been told about them — so printing them would make this verb answer differently
/// depending on where the plan is kept, which is the one thing every verb here refuses to do.
/// One stored document as the shape both `show` and a served read answer with.
///
/// Lifted so the two cannot drift: a field added here appears in the terminal and in the browser at
/// once, and a field added to only one of them is the defect this shape exists to prevent.
fn shown_from(stored: &aep_backend_markdown::StoredDocument) -> Shown {
    let frontmatter = &stored.document.frontmatter;
    Shown {
        id: frontmatter.id.to_string(),
        kind: frontmatter.kind.to_string(),
        status: frontmatter.status.as_str().to_owned(),
        title: frontmatter.title.clone(),
        summary: frontmatter.summary.clone(),
        owner: frontmatter.owner.clone(),
        tags: frontmatter.tags.iter().cloned().collect(),
        relations: frontmatter
            .relations
            .iter()
            .map(|relation| ShownRelation {
                relation: relation.kind.as_str(),
                target: relation.target.to_string(),
            })
            .collect(),
        withholds: frontmatter.withholds.map(|kind| kind.as_str().to_owned()),
        revision: frontmatter.revision,
        body: stored.document.body.clone(),
    }
}

fn show(args: &StoreArgs, id: &str, body_only: bool) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    if body_only && args.format != Format::Text {
        anyhow::bail!(
            "`--body-only` prints the body bytes and nothing else, so it has no `--format {}` \
             rendering; drop one of the two",
            format!("{:?}", args.format).to_lowercase()
        );
    }
    let opened = open(&args.location, false)?;
    let stored = opened
        .report
        .documents
        .get(&id)
        .with_context(|| opened.missing(&id))?;
    let shown = shown_from(stored);

    // The bytes and nothing else — no labels, no blank line, no newline this verb added. What
    // `body --from` would write straight back, which is what makes *read it, edit it, hand it
    // back* a thing to type rather than a frontmatter splitter to write.
    if body_only {
        out!("{}", shown.body);
        return Ok(ExitCode::SUCCESS);
    }

    match args.format {
        Format::Text => {
            let mut rows = vec![
                vec!["id".to_owned(), shown.id.clone()],
                vec!["kind".to_owned(), shown.kind.clone()],
                vec!["status".to_owned(), shown.status.clone()],
            ];
            // Absent is not empty. A label with nothing after it reads as a field set to the empty
            // string, which is a different document from one that never set it.
            for (label, value) in [
                ("title", shown.title.as_deref()),
                ("summary", shown.summary.as_deref()),
                ("owner", shown.owner.as_deref()),
                ("withholds", shown.withholds.as_deref()),
            ] {
                if let Some(value) = value {
                    rows.push(vec![label.to_owned(), value.to_owned()]);
                }
            }
            if !shown.tags.is_empty() {
                rows.push(vec!["tags".to_owned(), shown.tags.join(", ")]);
            }
            // One edge per line, labelled once: a document with six relations on one line is a line
            // nobody reads to the end.
            for (index, relation) in shown.relations.iter().enumerate() {
                let label = if index == 0 { "relations" } else { "" };
                rows.push(vec![
                    label.to_owned(),
                    format!("{} {}", relation.relation, relation.target),
                ]);
            }
            rows.push(vec!["revision".to_owned(), shown.revision.to_string()]);
            crate::print_table(&rows);

            if !shown.body.is_empty() {
                outln!();
                // `out!`, not `outln!`: the body is the document's own bytes and this verb adds
                // none to them. A body written without a closing newline prints without one.
                out!("{}", shown.body);
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&shown, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol artifact list`
fn list(args: &StoreArgs, kind: Option<&str>, status: Option<&str>) -> Result<ExitCode> {
    let opened = open(&args.location, false)?;
    let ladders = ladders_or_none(args);
    let blocked = blockers_by_target(&opened.report, ladders.lifecycles());
    let listed = select(&opened.report, &blocked, kind, status)?;

    match args.format {
        Format::Text => crate::print_table(
            &listed
                .iter()
                .map(|entry| {
                    let mut row = vec![
                        entry.id.clone(),
                        entry.kind.clone(),
                        entry.status.clone(),
                        entry.title.clone().unwrap_or_default(),
                    ];
                    // A fifth cell only where there is something to say, so an unblocked row ends
                    // at its title rather than in two spaces of nothing.
                    if let Some(marker) = blocked_marker(&entry.blocked_by) {
                        row.push(marker);
                    }
                    row
                })
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => crate::print_serialised(&listed, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// The lifecycles in force, or none of them.
///
/// A document tree that cannot be read is not a reason a *listing* should stop answering — `list`
/// and `board` never needed one before. Without ladders every `blocks` edge reads as in force
/// until the blocker is `archived`, which errs towards saying something is stuck: the failure this
/// story exists against is a parked item that looks like a moving one, and the opposite mistake is
/// visible the moment somebody reads it.
fn ladders_or_none(args: &StoreArgs) -> aep_engine::Registry {
    args.lifecycles().unwrap_or_default()
}

/// `protocol artifact board`
fn board(args: &StoreArgs, kind: Option<&str>) -> Result<ExitCode> {
    let opened = open(&args.location, false)?;
    let ladders = ladders_or_none(args);
    let blocked = blockers_by_target(&opened.report, ladders.lifecycles());
    let listed = select(&opened.report, &blocked, kind, None)?;
    let columns = columns_for(&listed, ladders.lifecycles());

    match args.format {
        Format::Text => {
            for (index, column) in columns.iter().enumerate() {
                if index > 0 {
                    outln!();
                }
                outln!("{} ({})", column.status, column.artifacts.len());
                for entry in &column.artifacts {
                    // The marker rides on the card, not on the column: a blocked item is still
                    // `active` — that is precisely the complaint — so moving it to a column of its
                    // own would be inventing a status the ladder does not have.
                    let marker = blocked_marker(&entry.blocked_by)
                        .map_or_else(String::new, |marker| format!("  [{marker}]"));
                    outln!(
                        "  {}  {}{}",
                        entry.id,
                        entry.title.clone().unwrap_or_default(),
                        marker
                    );
                }
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&columns, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Where the compiled vocabulary puts a rung, and after every rung it names, the ones it cannot.
///
/// The **only** thing `ArtifactStatus::ALL` decides on the board: which of two rungs neither of
/// which leads to the other is printed first. It decides no rung's existence — that is the whole
/// point of `story:board-columns-come-from-the-ladders` — and it drops nothing.
fn precedence(status: &str) -> (usize, String) {
    (
        ArtifactStatus::ALL
            .iter()
            .position(|known| known.as_str() == status)
            .unwrap_or(usize::MAX),
        status.to_owned(),
    )
}

/// Whether `from` reaches `target` by following one or more edges of `leads_to`.
///
/// `reaches(g, rung, rung)` therefore asks whether `rung` is on a cycle, which is the question both
/// callers actually have: [`ladder_order`] cuts only rungs that are, and [`board_order`] refuses
/// only the constraints that would put one there.
fn reaches(leads_to: &BTreeMap<String, BTreeSet<String>>, from: &str, target: &str) -> bool {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut pending: Vec<&str> = vec![from];
    while let Some(rung) = pending.pop() {
        for onwards in leads_to.get(rung).into_iter().flatten() {
            if onwards == target {
                return true;
            }
            if seen.insert(onwards.as_str()) {
                pending.push(onwards.as_str());
            }
        }
    }
    false
}

/// One ladder's rungs, in the order that document puts them in.
///
/// A topological sort — Kahn's, over this ladder's `transitions` — so a rung is printed only once
/// every rung that leads to it has been. That is what "ladder order" means and a walk outwards from
/// `initial` is not: `archived` is one hop from `draft` in `artifacts/lifecycles/task.yaml` *and*
/// the place a task stops, so ordering by distance from the start puts the end of the ladder third.
///
/// Two rungs neither of which leads to the other are separated by [`precedence`], and the result is
/// a *total* order over this ladder's rungs — which is what makes it something [`board_order`] can
/// merge, and what keeps `accepted` ahead of `rejected` in
/// `artifacts/lifecycles/architecture-decision-record.yaml`, where `proposed: [accepted, rejected]`
/// gives the pair no edge either way.
///
/// **Ladders have cycles.** `proposed -> draft` is in most of the documents in this repository, so
/// a pass where nothing is ready is an ordinary ladder and not a malformed one. Two rules decide
/// where such a pass cuts, and both exist because the obvious alternative was tried and printed
/// something worse:
///
/// 1. **Only a rung on a cycle may be cut**, hence `reaches(remaining, rung, rung)`. Cutting the
///    best of *every* stuck rung breaks an edge that no cycle forced anybody to break:
///    `intake -> triage`, `triage -> {rework, approved}`, `rework -> triage` has one cycle,
///    `{triage, rework}`, and `approved` is not in it — yet `approved` outranks both by
///    [`precedence`], so it was printed second, ahead of the only rung that reaches it, for two
///    backwards edges where one is unavoidable.
/// 2. **The cut lands where a reader entered**: a rung an already-printed rung leads to, then the
///    ladder's own `initial`, then [`precedence`]. Cutting at whichever rung was closest to ready
///    instead printed this repository's own board `proposed`, `active`, `draft`, with sixty-six
///    artifacts in the column that came third — `draft` is waited on by both `proposed` and
///    `in_review`, so it never wins that race.
///
/// One rung leaves the graph on every pass, so this terminates whatever the documents say.
fn ladder_order(ladder: &ArtifactLifecycle) -> Vec<String> {
    let mut waiting: BTreeMap<String, BTreeSet<String>> = ladder
        .statuses()
        .iter()
        .map(|rung| (rung.as_str().to_owned(), BTreeSet::new()))
        .collect();
    let mut leads_to: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (from, targets) in &ladder.transitions {
        for to in targets.iter().filter(|to| *to != from) {
            leads_to
                .entry(from.as_str().to_owned())
                .or_default()
                .insert(to.as_str().to_owned());
            waiting
                .entry(to.as_str().to_owned())
                .or_default()
                .insert(from.as_str().to_owned());
        }
    }

    let initial = ladder.initial.as_str().to_owned();
    let mut ordered: Vec<String> = Vec::with_capacity(waiting.len());
    while !waiting.is_empty() {
        // The graph the cut asks its question of is the one that is left: an edge out of a rung
        // already printed cannot hold anything up, and counting it would read a settled rung as
        // still being on a cycle.
        let remaining: BTreeMap<String, BTreeSet<String>> = waiting
            .keys()
            .map(|rung| {
                let onwards = leads_to
                    .get(rung)
                    .into_iter()
                    .flatten()
                    .filter(|to| waiting.contains_key(*to))
                    .cloned()
                    .collect();
                (rung.clone(), onwards)
            })
            .collect();
        let entered = |rung: &String| {
            ordered.iter().any(|done| {
                leads_to
                    .get(done)
                    .is_some_and(|targets| targets.contains(rung))
            })
        };
        // Ready rungs first and in precedence order, which is Kahn's algorithm; the cut keys below
        // are read only on a pass where nothing is ready, and only for a rung on a cycle, where no
        // edge has an opinion left to give.
        let key = |rung: &String| {
            let stuck = !waiting[rung].is_empty();
            (
                usize::from(stuck),
                usize::from(stuck && !reaches(&remaining, rung, rung)),
                usize::from(stuck && !entered(rung)),
                usize::from(stuck && *rung != initial),
                precedence(rung),
            )
        };
        let Some(next) = waiting.keys().min_by_key(|rung| key(rung)).cloned() else {
            break;
        };
        waiting.remove(&next);
        for onwards in leads_to.get(&next).into_iter().flatten() {
            if let Some(before) = waiting.get_mut(onwards) {
                before.remove(&next);
            }
        }
        ordered.push(next);
    }
    ordered
}

/// The rungs on the board, in the order the ladders themselves put them in.
///
/// Every ladder present is compiled to its own order by [`ladder_order`] — the order
/// `protocol artifact lifecycle <kind>` describes — and those orders are then **merged**: a ladder
/// is honoured in full unless honouring it would contradict one merged before it. Merging rather
/// than concatenating is what keeps a ladder's own columns where they were when something unrelated
/// is filed; appending one order per kind instead made the answer depend on which kind sorted
/// first.
///
/// **Two ladders can disagree, and then one of them loses.** `checklist` running
/// draft -> proposed -> active and `escalation` running active -> waiting -> proposed are each
/// perfectly ordinary, and their union has a cycle; no list of columns can honour both. The
/// merge is taken in **kind order**, so which one loses depends on the names of the kinds present
/// and on nothing else — not on how many artifacts are on the board, not on what was filed last,
/// so the columns do not move when the store does. Sorting the raw union of every ladder's edges
/// instead let one artifact of a second kind reorder the first kind's columns, because the cut fell
/// on the other ladder's rung. `--kind` narrows to one ladder, where the question cannot arise.
///
/// The compiled vocabulary is merged **last**, as one more sequence, so it separates two rungs
/// exactly when no ladder on the board has said anything about the pair: a `mystery` at `draft` and
/// a `review-result` at `active`, where `artifacts/lifecycles/review-result.yaml` is the only
/// document either kind has and it names neither `draft` nor `proposed`. That is the acceptance's
/// "the compiled list is used for nothing but the default ordering of the statuses it knows", and
/// it is why a rung no ladder named is interleaved by [`precedence`] rather than appended after
/// everything.
fn board_order(ladders: &[&ArtifactLifecycle], held: &BTreeSet<String>) -> Vec<String> {
    let mut sequences: Vec<Vec<String>> = ladders.iter().copied().map(ladder_order).collect();
    let mut rungs: BTreeSet<String> = held.clone();
    for sequence in &sequences {
        rungs.extend(sequence.iter().cloned());
    }
    sequences.push(
        ArtifactStatus::ALL
            .iter()
            .map(|known| known.as_str().to_owned())
            .filter(|known| rungs.contains(known))
            .collect(),
    );

    // Every pair a sequence puts in an order, not only its neighbours: a pair skipped for
    // contradicting an earlier ladder should cost that one pair, and not the order of everything
    // after it.
    let mut after: BTreeMap<String, BTreeSet<String>> = rungs
        .iter()
        .map(|rung| (rung.clone(), BTreeSet::new()))
        .collect();
    for sequence in &sequences {
        for (index, earlier) in sequence.iter().enumerate() {
            for later in sequence.iter().skip(index + 1) {
                if earlier != later && !reaches(&after, later, earlier) {
                    if let Some(onwards) = after.get_mut(earlier) {
                        onwards.insert(later.clone());
                    }
                }
            }
        }
    }

    let mut waiting: BTreeMap<String, BTreeSet<String>> = rungs
        .iter()
        .map(|rung| (rung.clone(), BTreeSet::new()))
        .collect();
    for (earlier, laters) in &after {
        for later in laters {
            if let Some(before) = waiting.get_mut(later) {
                before.insert(earlier.clone());
            }
        }
    }
    let mut ordered: Vec<String> = Vec::with_capacity(waiting.len());
    // `after` is acyclic by construction, so a source always exists and the second key never
    // decides anything; it is there so that this loop ends on any input rather than only on the
    // ones the construction promises.
    while let Some(next) = waiting
        .keys()
        .min_by_key(|rung| (usize::from(!waiting[*rung].is_empty()), precedence(rung)))
        .cloned()
    {
        waiting.remove(&next);
        for onwards in after.get(&next).into_iter().flatten() {
            if let Some(before) = waiting.get_mut(onwards) {
                before.remove(&next);
            }
        }
        ordered.push(next);
    }
    ordered
}

/// The board's columns: the ladders of the kinds on the board, in ladder order, and every rung
/// anything is actually on.
///
/// The set is **not** [`ArtifactStatus::ALL`]. That list is the vocabulary this crate was
/// *compiled* with, and the status vocabulary is open — so a `blocker` at `open`, a rung
/// `artifacts/lifecycles/blocker.yaml` declares and no release here names, had a card on the board
/// and no column to put it in. The ladders decide which columns exist, exactly as they decide
/// which moves are legal.
///
/// `--kind` needs no case of its own: it narrows what is on the board, and the columns are read
/// off what is on the board.
///
/// **Every card lands in a column, and this function has no failure mode.** A rung the board was
/// handed an artifact on and no ladder it read names still gets one, in the place [`precedence`]
/// puts it. `board` is `list` regrouped, not `list` filtered, and reading the ladders is
/// best-effort by design — see [`ladders_or_none`] — so a tree with no `artifacts/` directory, or
/// one document in it with a typo, would otherwise take an adopter's rung off the board and its
/// cards with it, at exit 0 and without a word, while `list` still prints them. That silent drop is
/// the defect this verb was filed against; it must not come back through the path where no ladder
/// is declared at all. Losing an order is a nuisance and losing a card is a lie. A kind whose name
/// this binary cannot parse loses its ladder for the same reason and by the same rule — its cards
/// still have rungs, and its rungs still have columns.
///
/// A rung a ladder declares and nothing is on still gets no column: an empty column is something a
/// reader has to skip on every glance.
fn columns_for(
    listed: &[Listed],
    lifecycles: &aep_domain::artifact::LifecycleRegistry,
) -> Vec<Column> {
    let kinds: BTreeSet<ArtifactKind> = listed
        .iter()
        .filter_map(|entry| ArtifactKind::parse(&entry.kind).ok())
        .collect();
    let ladders: Vec<&ArtifactLifecycle> = kinds
        .iter()
        .filter_map(|kind| lifecycles.for_kind(kind))
        .collect();
    let held: BTreeSet<String> = listed.iter().map(|entry| entry.status.clone()).collect();

    board_order(&ladders, &held)
        .into_iter()
        .map(|status| Column {
            artifacts: listed
                .iter()
                .filter(|entry| entry.status == status)
                .cloned()
                .collect(),
            status,
        })
        .filter(|column| !column.artifacts.is_empty())
        .collect()
}

/// `protocol artifact blocked`
/// Whether any ladder in force governs a blocker, which is what makes `blocked` a question at all.
///
/// The bare `blocker` and every `<type>-blocker` count, because the family is open and a store may
/// have declared a ladder for a type this binary has never heard of.
fn declares_a_blocker_ladder(lifecycles: &aep_domain::artifact::LifecycleRegistry) -> bool {
    lifecycles.iter().any(|(kind, _)| kind.is_blocker())
}

fn blocked(args: &StoreArgs, category: Option<&str>) -> Result<ExitCode> {
    let opened = open(&args.location, false)?;
    let ladders = ladders_or_none(args);
    let by_target = blockers_by_target(&opened.report, ladders.lifecycles());

    // Turned inside out: the map above answers *what is blocking this*, and the question here is
    // *what is this blocking*. One pass, keyed by the blocker, so several artifacts stopped by one
    // thing arrive as one group rather than as a list a reader has to collate by eye.
    let mut grouped: BTreeMap<String, Vec<Stopped>> = BTreeMap::new();
    for (target, blockers) in &by_target {
        let Some(stored) = opened.report.documents.get(target) else {
            continue;
        };
        let front = &stored.document.frontmatter;
        for blocking in blockers {
            grouped
                .entry(blocking.blocker.clone())
                .or_default()
                .push(Stopped {
                    id: front.id.to_string(),
                    kind: front.kind.to_string(),
                    status: front.status.as_str().to_owned(),
                    title: front.title.clone(),
                });
        }
    }

    let mut blockages: Vec<Blockage> = Vec::new();
    for (blocker, stopped) in grouped {
        let id = artifact_id(&blocker)?;
        let Some(stored) = opened.report.documents.get(&id) else {
            continue;
        };
        let front = &stored.document.frontmatter;
        let of_type = blocking_type(&front.kind);
        if category.is_some_and(|wanted| wanted != of_type) {
            continue;
        }
        blockages.push(Blockage {
            blocker,
            category: of_type,
            kind: front.kind.to_string(),
            status: front.status.as_str().to_owned(),
            title: front.title.clone(),
            withholds: front.withholds.map(|kind| kind.as_str().to_owned()),
            blocks: stopped,
        });
    }

    match args.format {
        Format::Text => {
            if blockages.is_empty() {
                // **Two different answers, and they used to be one.** `nothing is blocked` is a
                // report about the store; a store whose pin predates `artifacts/kinds/blocker.yaml`
                // has no blocker ladder to resolve, so the mechanism this verb reports on does not
                // exist there — and answering `nothing is blocked` made a missing feature look like
                // good news (`431986de#7007`, and the operator at `#7024`: "what are you talking
                // about blockers").
                if declares_a_blocker_ladder(ladders.lifecycles()) {
                    outln!("nothing is blocked");
                } else {
                    outln!(
                        "this store's lifecycles declare no blocker kind; `protocol artifact kinds` lists what can be created"
                    );
                }
            }
            for (index, blockage) in blockages.iter().enumerate() {
                if index > 0 {
                    outln!();
                }
                let withheld = blockage
                    .withholds
                    .as_ref()
                    .map_or_else(String::new, |kind| format!(", withholding {kind}"));
                outln!(
                    "{}  {}  {}{}  {}",
                    blockage.blocker,
                    blockage.category,
                    blockage.status,
                    withheld,
                    blockage.title.clone().unwrap_or_default()
                );
                for stopped in &blockage.blocks {
                    outln!(
                        "  blocks {}  {}  {}",
                        stopped.id,
                        stopped.status,
                        stopped.title.clone().unwrap_or_default()
                    );
                }
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&blockages, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol artifact graph`
fn graph(args: &StoreLocation, format: PlanningGraphFormat) -> Result<ExitCode> {
    let opened = open(args, false)?;
    let graph = match opened
        .report
        .graph_in_workspace(declared_members(&args.repository_root()))
    {
        Ok(graph) => graph,
        Err(errors) => {
            outln!("the plan does not build a graph:");
            for error in errors.as_slice() {
                outln!("  - {error}");
            }
            return Ok(crate::exit_code(false));
        }
    };

    match format {
        PlanningGraphFormat::Dot => out!("{}", dot(&graph)),
        PlanningGraphFormat::Json => crate::print_serialised(&graph, Format::Json)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// The plan as Graphviz DOT.
///
/// Ids need no escaping: [`ArtifactId`] accepts only alphanumerics and `-_./`, so neither a quote
/// nor a backslash can reach the output. Said here because the alternative is a reader wondering
/// whether it was forgotten.
fn dot(graph: &ArtifactGraph) -> String {
    let mut rendered = String::from("digraph planning {\n  rankdir=LR;\n  node [shape=box];\n");
    for artifact in graph.artifacts() {
        let _ = writeln!(
            rendered,
            "  \"{}\" [label=\"{}\\n{} · {}\"];",
            artifact.id, artifact.id, artifact.kind, artifact.status
        );
    }
    for artifact in graph.artifacts() {
        for relation in &artifact.relations {
            let _ = writeln!(
                rendered,
                "  \"{}\" -> \"{}\" [label=\"{}\"];",
                artifact.id,
                relation.target.id(),
                relation.kind
            );
        }
    }
    rendered.push_str("}\n");
    rendered
}

/// What the log says about the documents, for `validate`.
struct LogFindings {
    /// Reconciliation and drift findings, each a problem.
    problems: Vec<String>,
    /// The drift findings alone, for the report's own field.
    drift: Vec<String>,
    /// The forged-revision findings alone — a revision no logged write produced.
    forged: Vec<String>,
    /// The deletion findings alone.
    deleted: Vec<String>,
    /// Documents with no events at all.
    pre_provider: usize,
}

/// The documents against the event log (wave G, story 4), and the journal against the files.
///
/// A frontmatter field that disagrees with what the events say is **drift** — an edit made in an
/// editor; a revision higher than any event records is a **forged revision**, which no write could
/// have produced; events with no document are a **deletion**; a document with no events predates
/// the provider and is none of them. The journal's older reconciliation covers the same log's
/// status and revision by entry, so a document either check names is not named twice, and an
/// orphan the log knows as deleted is said once, as that.
fn log_findings(
    root: &Path,
    report: &aep_backend_markdown::store::StoreReport,
    held: &std::collections::BTreeMap<ArtifactId, (aep_domain::artifact::ArtifactStatus, u64)>,
) -> LogFindings {
    let drift = aep_backend_markdown::drift::detect(root, &report.documents);
    // A forged revision is a revision finding too, so the journal's own reconciliation must not
    // report it a second time in its older words.
    let drifted: std::collections::BTreeSet<_> = drift
        .drift
        .iter()
        .map(|d| d.artifact.clone())
        .chain(drift.forged.iter().map(|f| f.artifact.clone()))
        .collect();
    let deleted: std::collections::BTreeSet<_> =
        drift.deleted.iter().map(|d| d.artifact.clone()).collect();
    let mut problems: Vec<String> = aep_backend_markdown::journal::reconcile(root, held)
        .iter()
        .filter(|finding| match finding {
            aep_backend_markdown::journal::Drift::Disagrees { artifact, .. } => {
                !drifted.contains(artifact)
            }
            aep_backend_markdown::journal::Drift::Orphan { artifact, .. } => {
                !deleted.contains(artifact)
            }
        })
        .map(ToString::to_string)
        .collect();
    let drift_findings: Vec<String> = drift.drift.iter().map(ToString::to_string).collect();
    let forged_findings: Vec<String> = drift.forged.iter().map(ToString::to_string).collect();
    let deleted_findings: Vec<String> = drift.deleted.iter().map(ToString::to_string).collect();
    problems.extend(drift_findings.iter().cloned());
    problems.extend(forged_findings.iter().cloned());
    problems.extend(deleted_findings.iter().cloned());
    LogFindings {
        problems,
        drift: drift_findings,
        forged: forged_findings,
        deleted: deleted_findings,
        pre_provider: drift.pre_provider,
    }
}

/// `protocol artifact validate`
///
/// `strict` turns each *reported* class into an exit code, and changes nothing else: the same lines
/// are printed, in the same order, whether or not it is set. That split is deliberate and is
/// `story:completion-needs-evidence`'s recorded position — a store somebody is working in must be
/// able to hold a status closed on an assertion, and a gate must be able to refuse one — so the
/// caller who wants the second says so rather than the tool deciding for both.
fn validate(args: &StoreArgs, strict: bool) -> Result<ExitCode> {
    let opened = open(&args.location, false)?;
    let report = &opened.report;
    let registry = args.lifecycles()?;

    let mut problems: Vec<String> = report.failures.iter().map(ToString::to_string).collect();
    match report.graph_in_workspace(declared_members(&args.repository_root())) {
        Ok(graph) => problems.extend(
            graph
                .validate_lifecycles(registry.lifecycles())
                .as_slice()
                .iter()
                .map(ToString::to_string),
        ),
        Err(errors) => problems.extend(errors.as_slice().iter().map(ToString::to_string)),
    }

    // The journal against the files. A status the journal does not account for, and an entry naming
    // an artifact this store does not hold, are the two ways the record can be wrong — and on
    // 2026-08-26 both happened here in one day, with `validate` reporting every file valid through
    // both, because it read the files and the files were fine.
    let held: std::collections::BTreeMap<_, _> = report
        .documents
        .values()
        .map(|stored| {
            (
                stored.document.frontmatter.id.clone(),
                (
                    stored.document.frontmatter.status.clone(),
                    stored.document.frontmatter.revision,
                ),
            )
        })
        .collect();
    // The journal and the event log are a markdown plan's; a SQLite or Postgres plan keeps its
    // history in the store and the contract answers it, so there is no second record to reconcile.
    let (drift_findings, forged_findings, deleted_findings, pre_provider) = match &opened.files {
        Some(store) => {
            let log = log_findings(store.root(), report, &held);
            problems.extend(log.problems);
            (log.drift, log.forged, log.deleted, log.pre_provider)
        }
        None => (Vec::new(), Vec::new(), Vec::new(), 0),
    };

    // Closed on somebody's word, and the store knows the difference. A move whose provenance is
    // `asserted` reached this status because a caller said the evidence existed; one that is
    // `recorded` reached it because the store held a record. Both are legal — refusing an assertion
    // outright would stop anybody closing a story the day a runner is down — and reporting only the
    // second as evidence is what makes the first honest rather than invisible.
    let entries = opened
        .files
        .as_ref()
        .map(|store| aep_backend_markdown::journal::read(store.root()).0)
        .unwrap_or_default();
    let mut asserted: Vec<String> = Vec::new();
    for entry in &entries {
        if let aep_backend_markdown::journal::Change::Moved { to, decided_on, .. } = &entry.change {
            if decided_on.leans_on_an_assertion() {
                asserted.push(format!(
                    "{} reached {to} on an assertion rather than a record — the evidence was \
                     claimed, not held",
                    entry.artifact
                ));
            }
        }
    }

    let summary = Summary {
        store: opened.plan.describe(),
        files_read: report.files_read,
        artifacts: report.documents.len(),
        problems: problems.clone(),
        closed_on_an_assertion: asserted.clone(),
        drift: drift_findings,
        forged: forged_findings,
        deleted: deleted_findings,
        pre_provider,
    };

    match args.format {
        Format::Text => print_validation(&summary, strict),
        Format::Yaml | Format::Json => crate::print_serialised(&summary, args.format)?,
    }
    Ok(crate::exit_code(
        summary.problems.is_empty() && (!strict || strictly_refused(&summary).is_empty()),
    ))
}

/// What `validate` prints for a person: the counts, the classes it reports, then the verdict.
///
/// The lines are the same whether or not `strict` is set — it adds one, naming which reported class
/// decided the exit code, because an exit code with no line above it is a gate nobody can debug.
fn print_validation(summary: &Summary, strict: bool) {
    outln!(
        "{} file(s) in {}: {} artifact(s)",
        summary.files_read,
        summary.store,
        summary.artifacts
    );
    // A normal condition, said out loud: a document with no events cannot be checked against its
    // log, and a reader should know how many of those there are.
    if summary.pre_provider > 0 {
        outln!("{} document(s) predate the event log", summary.pre_provider);
    }
    // Reported, and deliberately **not** counted as a problem. Refusing an assertion outright would
    // stop anybody closing a story on the day a runner is down, which is the day it matters most.
    // What it must not be is invisible.
    if !summary.closed_on_an_assertion.is_empty() {
        outln!(
            "{} closed on an assertion:",
            summary.closed_on_an_assertion.len()
        );
        for note in &summary.closed_on_an_assertion {
            outln!("  - {note}");
        }
    }
    if summary.problems.is_empty() {
        outln!("valid");
    } else {
        outln!("{} problem(s):", summary.problems.len());
        for problem in &summary.problems {
            outln!("  - {problem}");
        }
    }
    if strict {
        let refusing = strictly_refused(summary);
        if !refusing.is_empty() {
            outln!(
                "--strict: refusing on {}",
                render_list(&refusing.iter().map(String::as_str).collect::<Vec<&str>>())
            );
        }
    }
}

/// The classes `--strict` fails on, named, in the order the report prints them.
///
/// Drift, a forged revision and a deletion are already problems, so they already fail; they are
/// here anyway, because a caller reading *why* a strict run refused should not have to know which
/// of the classes happened to be counted twice — and because a future edit that stopped counting
/// one as a problem must not quietly stop `--strict` refusing it.
fn strictly_refused(summary: &Summary) -> Vec<String> {
    let mut refusing = Vec::new();
    for (label, count) in [
        (
            "closed on an assertion",
            summary.closed_on_an_assertion.len(),
        ),
        ("predating the event log", summary.pre_provider),
        ("drifted", summary.drift.len()),
        ("forged revision", summary.forged.len()),
        ("deleted", summary.deleted.len()),
    ] {
        if count > 0 {
            refusing.push(format!("{count} {label}"));
        }
    }
    refusing
}

/// The row `kinds` prints for one kind.
///
/// A blocker reads `planning` whatever [`ArtifactKind::is_planning`] says, and that is not a
/// disagreement with the domain: that predicate answers *intent or output* over the compiled
/// vocabulary, and a `<type>-blocker` is an `Other` it has never been told about. A blocker is a
/// record of why work is stopped, which is intent, and printing it as `output` would put it on the
/// wrong side of the only column this table has.
fn kind_row(kind: &ArtifactKind, note: Option<String>) -> KindRow {
    let planning = kind.is_planning() || kind.is_blocker();
    KindRow {
        kind: kind.as_str().to_owned(),
        layer: if planning { "planning" } else { "output" },
        planning,
        note,
    }
}

/// `protocol artifact kinds`
///
/// **The compiled vocabulary is not the whole answer, and used to be printed as though it were.**
/// `ArtifactKind::NAMED` is what this binary knows; a store's `artifacts/lifecycles/*.yaml` may
/// declare kinds beside it — `protocol artifact lifecycle third-party-blocker` answered while
/// `kinds | grep -i block` returned nothing at all (`fcf5873a#361`) — and the blocker family is
/// **open**: any `<type>-blocker` is a kind, so no list can enumerate it and a row that says so is
/// the only honest way to put it in a table.
///
/// The lifecycles are read best-effort, as `board` reads them: a tree that cannot be read is not a
/// reason to stop answering the part of the question that comes from the vocabulary.
fn kinds(args: &StoreArgs) -> Result<ExitCode> {
    let mut listed: Vec<KindRow> = ArtifactKind::NAMED
        .iter()
        .map(|kind| kind_row(kind, None))
        .collect();

    let compiled: BTreeSet<String> = listed.iter().map(|row| row.kind.clone()).collect();
    let ladders = ladders_or_none(args);
    let mut declared: Vec<&ArtifactKind> = ladders
        .lifecycles()
        .iter()
        .map(|(kind, _)| kind)
        .filter(|kind| !compiled.contains(kind.as_str()))
        .collect();
    declared.sort_by_key(|kind| kind.as_str().to_owned());
    for kind in declared {
        listed.push(kind_row(
            kind,
            Some("this store's lifecycles declare it".to_owned()),
        ));
    }

    // The family no list can hold. `blocker_type` splits `<type>-blocker` and nothing enumerates
    // the types, because the type is whatever would clear the blockage.
    listed.push(KindRow {
        kind: format!("<type>-{}", aep_domain::artifact::BLOCKER),
        layer: "planning",
        planning: true,
        note: Some("open family: credential-blocker, decision-blocker, …".to_owned()),
    });

    match args.format {
        Format::Text => crate::print_table(
            &listed
                .iter()
                .map(|entry| {
                    let mut row = vec![entry.kind.clone(), entry.layer.to_owned()];
                    if let Some(note) = &entry.note {
                        row.push(format!("({note})"));
                    }
                    row
                })
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => crate::print_serialised(&listed, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol artifact relations`
fn relations(args: &StoreArgs) -> Result<ExitCode> {
    let listed: Vec<RelationRow> = RelationKind::ALL
        .iter()
        .map(|relation| RelationRow {
            relation: relation.as_str(),
            meaning: meaning(*relation),
            inverse: relation.inverse_label(),
        })
        .collect();

    match args.format {
        Format::Text => crate::print_table(
            &listed
                .iter()
                .map(|entry| vec![entry.relation.to_owned(), entry.meaning.to_owned()])
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => crate::print_serialised(&listed, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol artifact lifecycle`
fn lifecycle(args: &StoreArgs, kind: &str) -> Result<ExitCode> {
    let kind = ArtifactKind::parse(kind).map_err(|error| anyhow::anyhow!("{error}"))?;
    let registry = args.lifecycles()?;
    let permissive = ArtifactLifecycle::permissive();
    let declared = registry.lifecycles().for_kind(&kind).cloned();
    let lifecycle = declared.clone().unwrap_or(permissive);

    let view = Lifecycle {
        kind: kind.to_string(),
        declared: declared.is_some(),
        initial: lifecycle.initial.as_str().to_owned(),
        transitions: lifecycle
            .transitions
            .iter()
            .map(|(from, to)| {
                (
                    from.as_str().to_owned(),
                    to.iter().map(|status| status.as_str().to_owned()).collect(),
                )
            })
            .collect(),
    };

    match args.format {
        Format::Text => {
            if view.declared {
                outln!("{} starts at {}", view.kind, view.initial);
            } else {
                outln!(
                    "{} declares no lifecycle, so every status and every move is permitted",
                    view.kind
                );
            }
            for (from, to) in &view.transitions {
                let to: Vec<&str> = to.iter().map(String::as_str).collect();
                outln!("  {from} -> {}", render_list(&to));
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&view, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------------------------

/// Refuses to write while any document in the store cannot be read.
///
/// A mutation is a read followed by a write, and the read has to have worked. Two files claiming
/// one id is the case that makes this more than caution: whichever one this command chose to
/// rewrite, the other would still be there afterwards saying something different.
fn require_clean(store: &MarkdownStore, report: &StoreReport) -> Result<()> {
    if report.is_clean() {
        return Ok(());
    }
    let mut detail = format!(
        "{} planning document(s) in {} cannot be used, so nothing was written:",
        report.failures.len(),
        store.root().display()
    );
    for failure in &report.failures {
        let _ = write!(detail, "\n  - {failure}");
    }
    detail.push_str("\nfix them, or run `protocol artifact validate` for the whole list");
    bail!("{detail}")
}

/// Says on standard error that a read answered from part of the store.
///
/// A warning rather than a refusal, because a listing of nine artifacts is more useful than a
/// refusal over the tenth — and on standard error, so `--format json` stays a document.
fn warn_unclean(report: &StoreReport) {
    if !report.is_clean() {
        eprintln!(
            "warning: {} planning document(s) could not be read and are missing from this \
             answer; run `protocol artifact validate`",
            report.failures.len()
        );
    }
}

/// Why an id names nothing here.
fn missing(store: &MarkdownStore, id: &ArtifactId) -> String {
    format!(
        "{} holds no `{id}`; it would be at {}",
        store.root().display(),
        store.relative_path_for(id)
    )
}

/// Parses an artifact id, or says what one looks like.
fn artifact_id(value: &str) -> Result<ArtifactId> {
    ArtifactId::new(value).map_err(|error| anyhow::anyhow!("{error}"))
}

/// `protocol artifact evidence`
///
/// Records rather than decides. Nothing is gated here, no status moves, and a rung's `requires:` is
/// not consulted — recording evidence and acting on it are separate acts on purpose, because a
/// The instant a verb was told to record, as the contract spells one.
///
/// `--at` names an instant somebody observed. It travels on the command, because the clock is read
/// at the edge and handed in — a backend that stamped its own would silently ignore the flag.
///
/// # Errors
///
/// If the text is not an instant this build can read.
fn instant(text: &str) -> Result<aep_domain::time::Timestamp> {
    aep_domain::time::CivilDate::parse(text)
        .map(aep_domain::time::CivilDate::to_timestamp)
        .or_else(|_| second_instant(text).ok_or(()))
        .or_else(|()| {
            text.parse::<u64>()
                .map(aep_domain::time::Timestamp::from_epoch_millis)
                .map_err(|_| ())
        })
        .map_err(|()| anyhow::anyhow!("`{text}` is not an instant this build can read"))
}

/// `YYYY-MM-DDTHH:MM:SSZ` — the form `now_at_the_edge` writes — as a timestamp.
///
/// `story:evidence-verb-refuses-its-own-default-instant`: the edge produced an instant to the
/// second and every reader accepted a date or epoch milliseconds, so `protocol artifact evidence`
/// without `--at` refused the very value it had just defaulted to.
fn second_instant(text: &str) -> Option<aep_domain::time::Timestamp> {
    let (date, time) = text.split_once('T')?;
    let time = time.strip_suffix('Z')?;
    let mut parts = time.split(':');
    let (hours, minutes, seconds) = (
        parts.next()?.parse::<u64>().ok()?,
        parts.next()?.parse::<u64>().ok()?,
        parts.next()?.parse::<u64>().ok()?,
    );
    if parts.next().is_some() || hours > 23 || minutes > 59 || seconds > 60 {
        return None;
    }
    let day = aep_domain::time::CivilDate::parse(date)
        .ok()?
        .to_timestamp();
    Some(aep_domain::time::Timestamp::from_epoch_millis(
        day.epoch_millis() + (hours * 3600 + minutes * 60 + seconds) * 1000,
    ))
}

/// Issues the `RecordEvidence` command that records one observation.
fn evidence_through_a_command(
    backend: &PlanBackend,
    id: &ArtifactId,
    kind: aep_domain::evidence::EvidenceKind,
    source: &str,
    reference: Option<&str>,
    at: aep_domain::time::Timestamp,
) -> Result<()> {
    use aep_contract::command::CommandService;
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, RecordEvidence};
    use aep_domain::entity::{EntityLocator, EntityRef};

    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        id.namespace(),
        id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))?;
    let target = block_on(QueryService::resolve(backend, &locator))
        .map_err(|error| anyhow::anyhow!("`{id}` is not in this store: {error}"))?;

    let envelope = envelope_for(
        &format!(
            "evidence-{id}-{kind}-{}",
            clock_at_the_edge().epoch_millis()
        ),
        "protocol-artifact-evidence",
        "aep.evidence.record/v1",
        Command::RecordEvidence(RecordEvidence {
            target: EntityRef::new(target),
            kind: kind.to_string(),
            source: source.to_owned(),
            reference: reference.map(ToOwned::to_owned),
        }),
        at,
    )?;
    block_on(backend.execute(envelope)).map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(())
}

/// command that recorded evidence *and* moved the artifact would make the evidence a formality of
/// the move rather than a thing that existed before it.
fn record_evidence(
    args: &StoreArgs,
    id: &str,
    kind: &str,
    source: &str,
    reference: Option<&str>,
    at: Option<&str>,
) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    // Built rather than wrapped in a `context`, which would put the advice in front of the list it
    // is advice about. The refusal has to **end** with the two kinds, because that is the part a
    // reader who did not find their word in fifteen names still needs.
    let kind = aep_domain::evidence::EvidenceKind::parse(kind.trim())
        .map_err(|error| anyhow::anyhow!("{error}. {}", nearest_evidence_kinds()))?;
    if source.trim().is_empty() {
        anyhow::bail!(
            "evidence needs a source; write where it came from, such as --source 'task check'"
        );
    }
    let at = at.map_or_else(now_at_the_edge, str::to_owned);

    let opened = open(&args.location, true)?;
    // The artifact must exist. Evidence about nothing is not evidence, and a typo'd id would
    // otherwise sit in the journal looking like a record until somebody wondered why a move refused.
    let stored = opened
        .report
        .documents
        .get(&id)
        .with_context(|| opened.missing(&id))?;

    // Through a command, like every other write. An evidence record is the **input to the
    // evidence-gated move decision**, so a verb appending one directly writes the thing the
    // decision reads without passing the door every other write passes — the same deviation D-P1
    // was, and invisible to a scan looking only for store writes.
    let _ = stored;
    evidence_through_a_command(
        opened.backend()?,
        &id,
        kind,
        source,
        reference,
        instant(&at)?,
    )?;

    let on_hand = opened.evidence_on_hand(&id)?;
    match args.format {
        Format::Text => {
            outln!("{id}: {} recorded from {source}", kind.as_str());
            let held: Vec<String> = on_hand
                .iter()
                .map(|(kind, count)| format!("{}={count}", kind.as_str()))
                .collect();
            outln!("  on hand: {}", held.join(", "));
        }
        Format::Yaml | Format::Json => crate::print_serialised(&on_hand, args.format)?,
    }
    Ok(crate::exit_code(true))
}

/// `protocol artifact history`
fn history(args: &StoreArgs, id: &str) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    let plan = args.location.plan()?;
    let Plan::Markdown { root } = &plan else {
        // A SQLite or Postgres plan has no journal; a hybrid has one on its local half, and reads
        // its history through the composite's read path instead, as it reads everything else.
        return history_from_the_contract(args, &id);
    };
    let (entries, unreadable) = aep_backend_markdown::journal::history(root, &id);
    print_history(args.format, &id, &entries, unreadable)
}

/// One recorded divergence, as `divergences` and `catch-up` print it.
#[derive(Debug, serde::Serialize)]
struct DivergenceLine {
    entity: String,
    id: String,
    local_revision: u64,
    detail: String,
}

impl From<&aep_backend_hybrid::Divergence> for DivergenceLine {
    fn from(divergence: &aep_backend_hybrid::Divergence) -> Self {
        Self {
            entity: divergence.entity.clone(),
            id: divergence.id.clone(),
            local_revision: divergence.local_revision,
            detail: divergence.detail.clone(),
        }
    }
}

/// What `divergences` reports.
#[derive(Debug, serde::Serialize)]
struct DivergencesReport {
    store: String,
    authority: String,
    divergences: Vec<DivergenceLine>,
}

/// The hybrid plan `args` names, or the reason a verb about divergences does not apply.
fn hybrid_plan(
    args: &StoreLocation,
) -> Result<(PathBuf, Replica, aep_domain::project::HybridPolicy)> {
    match args.plan()? {
        Plan::Hybrid {
            root,
            replica,
            policy,
        } => Ok((root, replica, policy)),
        other => anyhow::bail!(
            "{} is not a hybrid plan; a divergence is what a `store: hybrid` records when one of \
             its two sides took a write the other did not",
            other.describe()
        ),
    }
}

/// `protocol artifact divergences`: what a hybrid plan has recorded and not yet caught up.
fn divergences(args: &StoreArgs) -> Result<ExitCode> {
    let (root, replica, policy) = hybrid_plan(&args.location)?;
    let recorded =
        aep_backend_hybrid::read_divergences(&root).map_err(|error| anyhow::anyhow!("{error}"))?;
    let report = DivergencesReport {
        store: format!(
            "{} with its replica in {}",
            root.display(),
            replica.describe()
        ),
        authority: policy.authority.clone(),
        divergences: recorded.iter().map(DivergenceLine::from).collect(),
    };
    match args.format {
        Format::Text => {
            if report.divergences.is_empty() {
                outln!("no divergences recorded; authority: {}", report.authority);
            } else {
                outln!(
                    "{} divergence(s) recorded; authority: {} — `protocol artifact catch-up` replays \
                     them",
                    report.divergences.len(),
                    report.authority
                );
                for line in &report.divergences {
                    outln!(
                        "  {}:{} at revision {}: {}",
                        line.entity,
                        line.id,
                        line.local_revision,
                        line.detail
                    );
                }
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&report, args.format)?,
    }
    Ok(crate::exit_code(report.divergences.is_empty()))
}

/// What `catch-up` reports.
#[derive(Debug, serde::Serialize)]
struct CatchUpReport {
    store: String,
    authority: String,
    found: usize,
    replayed: usize,
    outstanding: Vec<DivergenceLine>,
}

/// `protocol artifact catch-up`: replays a hybrid plan's divergences at the side that missed them.
fn catch_up(args: &StoreArgs) -> Result<ExitCode> {
    let (root, replica, policy) = hybrid_plan(&args.location)?;
    let runtime_policy =
        aep_backend_hybrid::policy_from(&policy).map_err(|error| anyhow::anyhow!("{error}"))?;
    let outcome = match &replica {
        Replica::Sqlite(path) => {
            let store = entity_sqlite::SqliteStore::open(path)
                .with_context(|| format!("opening the replica at {}", path.display()))?;
            aep_backend_hybrid::catch_up(&root, store, runtime_policy)
        }
        Replica::Postgres(url) => {
            let store = entity_postgres::PostgresStore::connect_no_tls(url)
                .with_context(|| format!("connecting to the replica at {}", redact(url)))?;
            aep_backend_hybrid::catch_up(&root, store, runtime_policy)
        }
    }
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    let report = CatchUpReport {
        store: format!(
            "{} with its replica in {}",
            root.display(),
            replica.describe()
        ),
        authority: policy.authority.clone(),
        found: outcome.found,
        replayed: outcome.replayed(),
        outstanding: outcome
            .outstanding
            .iter()
            .map(DivergenceLine::from)
            .collect(),
    };
    match args.format {
        Format::Text => {
            outln!(
                "{} divergence(s) found, {} replayed, {} outstanding; authority: {}",
                report.found,
                report.replayed,
                report.outstanding.len(),
                report.authority
            );
            for line in &report.outstanding {
                outln!(
                    "  {}:{} at revision {}: {}",
                    line.entity,
                    line.id,
                    line.local_revision,
                    line.detail
                );
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&report, args.format)?,
    }
    Ok(crate::exit_code(report.outstanding.is_empty()))
}

/// `protocol artifact history` over a plan with no journal: the event log, read as journal entries.
///
/// A SQLite or Postgres plan keeps what the journal keeps — who, when, which revision, what changed
/// — as the runtime's events (`entity-runtime` R-110: the command's payload travels as the event's
/// `args`). Read back through the same vocabulary, so a history printed over one store is the
/// history printed over another (`story:store-selection-in-project-yaml`). An event this cannot
/// read as an entry is counted and said, as an unreadable journal line is.
fn history_from_the_contract(args: &StoreArgs, id: &ArtifactId) -> Result<ExitCode> {
    let opened = open(&args.location, true)?;
    let (entries, unreadable) = entries_from_the_contract(&opened, id)?;
    print_history(args.format, id, &entries, unreadable)
}

/// Every journal entry one artifact's event log stands for, oldest first, and how many events this
/// build could not read as one.
///
/// The **only** reading path `explain` has, in every store, and the one `history` takes wherever
/// there is no journal file to read instead. A question the store answers must have one answer: a
/// second way of reading the same events is a second answer waiting to drift from the first.
fn entries_from_the_contract(
    opened: &Opened,
    id: &ArtifactId,
) -> Result<(Vec<aep_backend_markdown::journal::Entry>, usize)> {
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::entity::EntityLocator;

    let backend = opened.backend()?;
    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        id.namespace(),
        id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))?;
    let entity = block_on(backend.resolve(&locator)).with_context(|| opened.missing(id))?;

    let mut entries = Vec::new();
    let mut unreadable = 0;
    for event in &backend.events_of(&entity)? {
        match entry_from_event(backend, id, event) {
            Some(entry) => entries.push(entry),
            None => unreadable += 1,
        }
    }
    Ok((entries, unreadable))
}

/// `protocol artifact explain`
///
/// The question a reviewer asks three months later — *what made this done* — answered by the store
/// rather than by commit archaeology (`story:completion-audit-join`). Every store answers it the
/// same way, because it is read through the contract in all of them and never from the files.
fn explain(args: &StoreArgs, id: &str) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    let opened = open(&args.location, true)?;
    let stored = opened
        .report
        .documents
        .get(&id)
        .with_context(|| opened.missing(&id))?;
    let (entries, unreadable) = entries_from_the_contract(&opened, &id)?;
    let (reached, recorded_since) = joined(&entries);
    let ladders = ladders_or_none(args);
    let blocked_by = blockers_by_target(&opened.report, ladders.lifecycles())
        .remove(&id)
        .unwrap_or_default();
    let next = next_rungs(
        &stored.document.frontmatter,
        ladders.lifecycles(),
        &opened.evidence_on_hand(&id)?,
    );
    let explained = Explained {
        artifact: id,
        store: opened.plan.describe(),
        status: stored.document.frontmatter.status.to_string(),
        revision: stored.document.frontmatter.revision,
        blocked_by,
        reached,
        recorded_since,
        next,
        unreadable,
    };
    print_explanation(args.format, &explained)
}

/// Where this artifact may go next, and what each of those rungs costs against what it holds.
///
/// **The question `explain` created and did not answer.** It said what happened and, when nothing
/// had, `no status move is recorded` — so the requirement of the *next* rung was learnt by being
/// refused by `move`, twice (`11727595#3402`). A rung's price is a line in a lifecycle document and
/// there is no reason a reader should have to be refused to see it.
///
/// A kind whose lineage declares no ladder gets no lines: the permissive lifecycle makes every
/// status reachable from every other, and forty rows of *anything is legal* is not an answer.
fn next_rungs(
    frontmatter: &PlanningFrontmatter,
    lifecycles: &aep_domain::artifact::LifecycleRegistry,
    held: &aep_backend_markdown::kernel::EvidenceOnHand,
) -> Vec<NextRung> {
    let Some(ladder) = lifecycles.for_kind(&frontmatter.kind) else {
        return Vec::new();
    };
    ladder
        .transitions
        .get(&frontmatter.status)
        .into_iter()
        .flatten()
        .map(|status| NextRung {
            status: status.as_str().to_owned(),
            needs: ladder
                .requirements_for(status)
                .iter()
                .map(|requirement| Need {
                    kind: requirement.evidence.as_str().to_owned(),
                    at_least: requirement.at_least,
                    held: held.get(&requirement.evidence).copied().unwrap_or_default(),
                })
                .collect(),
        })
        .collect()
}

/// The statuses `entries` account for, each joined to the records admitted since the previous one,
/// and whatever has been recorded since the last of them.
///
/// The join is **log order**, deliberately, and not a comparison of instants: `at` is when the
/// caller says they looked, which a caller may legitimately back-date, and a record back-dated
/// behind a move it was written after did not make that move. What a move rested on is what the
/// store had already admitted when the move was made.
///
/// One-to-many, which is `story:completion-audit-join`'s own default: a story satisfied by a suite
/// and by a review has two records, and forcing a choice between them would lose one.
fn joined(entries: &[aep_backend_markdown::journal::Entry]) -> (Vec<Reached>, Vec<Admitted>) {
    use aep_backend_markdown::journal::Change;

    let mut reached = Vec::new();
    let mut since: Vec<Admitted> = Vec::new();
    for entry in entries {
        match &entry.change {
            Change::Evidence {
                kind,
                source,
                reference,
            } => since.push(Admitted {
                kind: kind.as_str().to_owned(),
                source: source.clone(),
                reference: reference.clone(),
                at: entry.at.clone(),
                revision: entry.revision,
            }),
            Change::Moved {
                from,
                to,
                decided_on,
            } => {
                let rested_on = std::mem::take(&mut since);
                let on_nothing_recorded = rested_on
                    .is_empty()
                    .then(|| nothing_recorded(decided_on).to_owned());
                reached.push(Reached {
                    from: from.to_string(),
                    to: to.to_string(),
                    at: entry.at.clone(),
                    revision: entry.revision,
                    rested_on,
                    on_nothing_recorded,
                });
            }
            Change::Created { .. } | Change::Related { .. } | Change::BodyReplaced => {}
        }
    }
    (reached, since)
}

/// What a move rested on when the store holds no record for it, in the words `validate` uses.
///
/// Both readings are legal and neither is invisible. A move on a bare count is one somebody claimed
/// and nothing can check; a move on nothing at all is a rung that asked for nothing, and reporting
/// it as an assertion would put words in a caller's mouth.
fn nothing_recorded(decided_on: &aep_backend_markdown::journal::Provenance) -> &'static str {
    if decided_on.leans_on_an_assertion() {
        "asserted — no record: the evidence was claimed, not held"
    } else {
        "no record: nothing was recorded about how this was decided"
    }
}

/// Prints an explanation: the same lines whichever store answered it.
fn print_explanation(format: Format, explained: &Explained) -> Result<ExitCode> {
    match format {
        Format::Text => {
            // Said out loud rather than folded into the answer, exactly as `history` says it: a
            // shorter account reported as if it were complete is the quiet failure this verb exists
            // against.
            if explained.unreadable > 0 {
                outln!(
                    "{} event(s) could not be read and are not accounted for below",
                    explained.unreadable
                );
            }
            outln!(
                "{} in {}: {}, revision {}",
                explained.artifact,
                explained.store,
                explained.status,
                explained.revision
            );
            // Before the history, not after it: what is stopping this *now* is the thing a
            // reader came for, and a page of past moves is what they would have to scroll past.
            for blocking in &explained.blocked_by {
                let withheld = blocking
                    .withholds
                    .as_ref()
                    .map_or_else(String::new, |kind| format!(", withholding {kind}"));
                outln!(
                    "  blocked by {} ({}){}",
                    blocking.blocker,
                    blocking.category,
                    withheld
                );
            }
            if explained.reached.is_empty() {
                outln!("  no status move is recorded");
            }
            for step in &explained.reached {
                outln!(
                    "  {} -> {}  {}  (revision {})",
                    step.from,
                    step.to,
                    step.at,
                    step.revision
                );
                if let Some(note) = &step.on_nothing_recorded {
                    outln!("    {note}");
                }
                for record in &step.rested_on {
                    outln!("    {}", describe_record(record));
                }
            }
            if !explained.recorded_since.is_empty() {
                outln!("  recorded since, and not yet the reason for a move:");
                for record in &explained.recorded_since {
                    outln!("    {}", describe_record(record));
                }
            }
            // Last, and one line per rung: the reader who got here wanted the history, and the
            // reader who got here to find out what to do next wanted this. Both are served by it
            // being at the end rather than by neither being served at all.
            for rung in &explained.next {
                if rung.needs.is_empty() {
                    outln!("  next: {} needs no record", rung.status);
                }
                for need in &rung.needs {
                    outln!(
                        "  next: {} needs {} {} record(s); held: {}",
                        rung.status,
                        need.at_least,
                        need.kind,
                        need.held
                    );
                }
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(explained, format)?,
    }
    Ok(crate::exit_code(true))
}

/// One record on one line: what it is, where it came from, when it was observed — and the revision
/// the artifact was at when it was admitted, which is the text it was actually about.
fn describe_record(record: &Admitted) -> String {
    let reference = record
        .reference
        .as_deref()
        .map_or_else(String::new, |reference| format!(" ({reference})"));
    format!(
        "{} from {}{}, observed {}, admitted at revision {}",
        record.kind, record.source, reference, record.at, record.revision
    )
}

/// The journal entry a plan's event stands for, when the command it carries is one a plan issues.
fn entry_from_event(
    backend: &PlanBackend,
    id: &ArtifactId,
    event: &entity_core::DomainEvent,
) -> Option<aep_backend_markdown::journal::Entry> {
    use aep_backend_markdown::journal::{Change, Entry};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::entity::{EntityId, EntityRef};

    let seal = event.payload.as_object()?;
    let at = seal.get("recorded_at")?.as_str()?.to_owned();
    let actor = seal.get("actor")?.as_str()?.to_owned();
    let entry = |change: Change| {
        Some(Entry {
            at: at.clone(),
            actor: actor.clone(),
            artifact: id.clone(),
            kind: id.namespace().parse().ok()?,
            revision: event.revision,
            change,
        })
    };
    // The plan's own projection writes what changed into the event, in the journal's vocabulary —
    // and it is the only reliable account of an edge's target for a plan whose entity identities
    // are minted per process (a markdown or hybrid plan re-seeds on open, so the `args` name an
    // entity id from the process that wrote). What follows is the reading for a store whose events
    // carry no such note: the `Identity` shape of a SQLite or Postgres plan, whose ids are stored.
    if let Some(note) = seal.get("change") {
        if let Ok(change) = serde_json::from_value::<Change>(note.clone()) {
            return entry(change);
        }
    }
    let args = &event.args;
    let text = |key: &str| args.get(key).and_then(serde_json::Value::as_str);
    let change = match text("command")? {
        "create-entity" => Change::Created {
            status: event.to_state.parse().ok()?,
        },
        "move-status" => Change::Moved {
            from: event.from_state.as_deref()?.parse().ok()?,
            to: event.to_state.parse().ok()?,
            decided_on: args
                .get("decided_on")
                .map(provenance_of)
                .unwrap_or_default(),
        },
        "create-relation" => {
            let target: EntityId = args.get("target")?.as_str()?.parse().ok()?;
            let target = block_on(backend.get(
                &EntityRef::new(target),
                aep_contract::QueryConsistency::Current,
            ))
            .ok()?;
            Change::Related {
                relation: serde_json::from_value(args.get("kind")?.clone()).ok()?,
                target: format!(
                    "{}:{}",
                    target.metadata.locator.kind(),
                    target.metadata.locator.key()
                ),
            }
        }
        "update-entity" => Change::BodyReplaced,
        "record-evidence" => Change::Evidence {
            kind: text("kind")?.parse().ok()?,
            source: text("source")?.to_owned(),
            reference: text("reference").map(str::to_owned),
        },
        _ => return None,
    };
    entry(change)
}

/// The account a move's `decided_on` argument carries, in either shape it travels in.
///
/// It travels as **JSON text**: `Node`'s numbers are floating point and a count of `1` that comes
/// back as `1.0` is not a count, so the command carries a string rather than a map (the markdown
/// projection reads it with `from_str` for exactly that reason). A store whose events are the
/// command's `args` therefore hands a `Value::String` back here.
///
/// Reading only the object shape defaulted every one of those to *nothing was recorded*, so over a
/// SQLite or Postgres plan a move made on somebody's bare `--evidence` count was indistinguishable
/// from a move the store held a record for — the one distinction `Provenance` exists to keep, lost
/// in the store that has no journal file to fall back on. Both shapes are read now, and neither is
/// guessed at: an account that parses as neither is no account, which is the honest reading of a
/// line this build does not understand.
fn provenance_of(account: &serde_json::Value) -> aep_backend_markdown::journal::Provenance {
    match account {
        serde_json::Value::String(text) => serde_json::from_str(text).unwrap_or_default(),
        other => serde_json::from_value(other.clone()).unwrap_or_default(),
    }
}

/// Prints a history: the same lines whichever store answered it.
fn print_history(
    format: Format,
    id: &ArtifactId,
    entries: &[aep_backend_markdown::journal::Entry],
    unreadable: usize,
) -> Result<ExitCode> {
    // Said out loud rather than folded into the count. A journal is append-only and long-lived, so
    // one half-written line from a killed process must not make the rest unreadable — but a shorter
    // history reported as if it were complete is exactly the quiet failure this file exists against.
    if unreadable > 0 {
        outln!("{unreadable} journal line(s) could not be read and are not counted below");
    }

    match format {
        Format::Text => {
            if entries.is_empty() {
                outln!("{id}: nothing recorded");
            }
            for entry in entries {
                outln!(
                    "{}  {}  {} (revision {})",
                    entry.at,
                    entry.actor,
                    entry.change,
                    entry.revision
                );
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&entries, format)?,
    }
    Ok(crate::exit_code(true))
}

/// Superseded, 2026-08-26, and recorded rather than quietly deleted.
///
/// `fn journal` wrote a journal entry beside each verb's own write, and `every_write_verb_is_journalled`
/// scanned this file for a write that had no such call near it. Both were the mitigation for
/// deviation **D-P1**, which existed because the verbs wrote the store directly.
///
/// They do not. Every verb issues a command and `MarkdownBackend` writes the file **and** journals
/// it as one act, so a write that records nothing is not something a verb can forget — it is not
/// something a verb does. The last holdout was `protocol artifact evidence`, which appended to the
/// journal directly; an evidence record is the input to the evidence-gated move decision, so a verb
/// writing one behind the contract was writing the thing the decision reads. It goes through
/// `aep.evidence.record/v1` now.
///
/// What pins it is `no_planning_verb_writes_to_the_store_except_through_a_command`, whose predicate
/// covers the journal as well as the store, with a guard that calls that same predicate.
const _SUPERSEDED_JOURNAL_HELPER: () = ();

/// Who is doing this, as they are willing to say — superseded by [`actor_from`].
///
/// It read `$USER` and nothing else, which is why every write in this store said `human:<$USER>`
/// however it was made. The store still cannot verify an identity; what changed is that a caller
/// can now declare one, and the declaration is parsed rather than interpolated.
const _SUPERSEDED_ACTOR_HELPER: () = ();

/// The instant this invocation ran, ISO-8601, read from the system clock.
///
/// The only clock in the planning path, and it is in the shell by construction: `aep-domain` has a
/// banned-token scan that would refuse one, and a decision that read the clock itself could not be
/// replayed. Its answer is an *argument* to the move, printed with any dated refusal.
pub(crate) fn now_at_the_edge() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    // Civil date from a Unix second, without a date library: days since the epoch, then the
    // proleptic Gregorian calendar. `aep-domain` cannot take one and neither should this path.
    let (days, rest) = (
        i64::try_from(seconds / 86_400).unwrap_or(0),
        seconds % 86_400,
    );
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// Howard Hinnant's `civil_from_days`, which is the standard way to do this in integer arithmetic.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = u64::try_from(z - era * 146_097).unwrap_or(0);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = i64::try_from(yoe).unwrap_or(0) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parses `<kind>=<count>` pairs into the counts a requirement is checked against.
///
/// A kind nobody names is a typo, not an empty count: silently reading `test_reslt=1` as *no test
/// results* would refuse the move for a reason the author cannot see, which is the failure the
/// three-valued refusal exists to end.
/// What a refused evidence kind says after the list of the ones that exist.
///
/// **A list of fifteen names is not an answer.** The parser already prints them, and the two things
/// sessions actually reached for and did not find are an observation of a live system
/// (`431986de#6957` wrote `measurement`) and a dependency on another repository's work
/// (`e70b8018 s1#694` wrote `cross_repo_dependency`). Both have a kind; neither guess is its name,
/// and neither is findable by scanning a list for a word you do not have.
///
/// The two kinds are named through the enum rather than as text, so a release that removed either
/// fails to compile here instead of shipping a refusal that recommends a kind nobody can use.
fn nearest_evidence_kinds() -> String {
    use aep_domain::evidence::EvidenceKind;
    format!(
        "for an observation of a running system use `{}`; for a relation to another store's \
         artifact use `{}`",
        EvidenceKind::HealthObservation.as_str(),
        EvidenceKind::Artifact.as_str()
    )
}

fn parse_evidence(pairs: &[String]) -> Result<aep_backend_markdown::kernel::EvidenceOnHand> {
    let mut counts = aep_backend_markdown::kernel::EvidenceOnHand::new();
    for pair in pairs {
        let (kind, count) = pair.split_once('=').with_context(|| {
            format!("`{pair}` is not evidence; write it as <kind>=<count>, such as test_result=1")
        })?;
        let kind = aep_domain::evidence::EvidenceKind::parse(kind.trim())
            .map_err(|error| anyhow::anyhow!("{error}. {}", nearest_evidence_kinds()))?;
        let count: usize = count
            .trim()
            .parse()
            .with_context(|| format!("`{count}` is not a count of {} records", kind.as_str()))?;
        *counts.entry(kind).or_default() += count;
    }
    Ok(counts)
}

/// Parses a status name for a read — a filter, where any well-formed name is a fair question.
///
/// Asking to list artifacts in a status nothing holds is an empty answer, not an error, so this
/// checks the shape and nothing else. Writing one is a different matter: see [`parse_status_in`].
fn parse_status(value: &str) -> Result<ArtifactStatus> {
    ArtifactStatus::parse(value).map_err(|error| anyhow::anyhow!("{error}"))
}

/// Parses a status name for a **write**, against the ladder the kind declares.
///
/// This is where the open vocabulary is kept honest. `ArtifactStatus` will carry any well-formed
/// name, so a lifecycle document can add `correction-owed` without a release here — but a name no
/// ladder declares is a typo, and a typo that becomes a status is a process nobody wrote. The
/// refusal lists what this kind may actually hold, which is more useful than a list of what the
/// binary happens to know.
fn parse_status_in(
    value: &str,
    kind: &ArtifactKind,
    lifecycles: &aep_domain::artifact::LifecycleRegistry,
) -> Result<ArtifactStatus> {
    let status = parse_status(value)?;
    let Some(lifecycle) = lifecycles.for_kind(kind) else {
        // No ladder anywhere in this kind's lineage: the store is permissive here, and refusing a
        // status because nobody wrote a ladder for `runbook` would make it unusable.
        return Ok(status);
    };
    if lifecycle.permits(&status) {
        return Ok(status);
    }
    let declared: Vec<String> = lifecycle
        .statuses()
        .iter()
        .map(|status| status.as_str().to_owned())
        .collect();
    let declared: Vec<&str> = declared.iter().map(String::as_str).collect();
    anyhow::bail!(
        "`{value}` is not a status a {kind} may hold; its lifecycle declares {}. \
         Add the rung to that lifecycle document if it belongs there — it is a line, not a release",
        render_list(&declared)
    )
}

/// Parses `<relation>:<artifact-id>`.
fn parse_relation(value: &str) -> Result<(RelationKind, ArtifactRef)> {
    let (relation, target) = value.split_once(':').with_context(|| {
        format!(
            "`{value}` is not an edge; write `<relation>:<artifact-id>`, such as \
             `derived_from:epic:passwordless`"
        )
    })?;
    Ok((
        RelationKind::parse(relation).map_err(|error| anyhow::anyhow!("{error}"))?,
        ArtifactRef::parse(target).map_err(|error| anyhow::anyhow!("{error}"))?,
    ))
}

/// Every blocker still in force, keyed by the artifact it stops.
///
/// **What is still blocking is decided by the ladder, not by a status name written here.** A
/// `blocks` edge holds until the artifact declaring it reaches the end of its own lifecycle — for
/// a blocker that is `cleared`, because `artifacts/lifecycles/blocker.yaml` gives that rung no
/// successor — or is `archived`, this vocabulary's retirement and the only answer available in a
/// tree that declares no ladders at all. That is what makes unblocking *a move like any other*:
/// `protocol artifact move <blocker> --to cleared` lifts it, the journal keeps the record that
/// something was ever stuck, and nothing had to be edited out of a file.
fn blockers_by_target(
    report: &StoreReport,
    lifecycles: &aep_domain::artifact::LifecycleRegistry,
) -> BTreeMap<ArtifactId, Vec<Blocking>> {
    let mut found: BTreeMap<ArtifactId, Vec<Blocking>> = BTreeMap::new();
    for stored in report.documents.values() {
        let front = &stored.document.frontmatter;
        if front.status == ArtifactStatus::Archived {
            continue;
        }
        if lifecycles
            .for_kind(&front.kind)
            .is_some_and(|ladder| ladder.is_terminal(&front.status))
        {
            continue;
        }
        for relation in front.targets(RelationKind::Blocks) {
            found
                .entry(relation.target.id().clone())
                .or_default()
                .push(Blocking {
                    blocker: front.id.to_string(),
                    category: blocking_type(&front.kind),
                    withholds: front.withholds.map(|kind| kind.as_str().to_owned()),
                });
        }
    }
    found
}

/// What a listing says about a blocker that carries no type.
///
/// Named rather than left blank: an empty cell reads as *not blocked* at a glance, which is the
/// exact confusion the typed blocker exists to remove.
const UNTYPED: &str = "untyped";

/// What a listing calls the type of one blocking artifact.
///
/// A blocker's type is the part of its kind before `-blocker`. A bare `blocker` has none and reads
/// [`UNTYPED`]. Anything else that happens to declare a `blocks` edge — a story, a task — reads as
/// its **own kind**, because calling a story untyped would say it is a blocker whose author forgot
/// to say which sort, and it is not a blocker at all.
fn blocking_type(kind: &ArtifactKind) -> String {
    if let Some(of_type) = kind.blocker_type() {
        return of_type.to_owned();
    }
    if kind.is_blocker() {
        return UNTYPED.to_owned();
    }
    kind.as_str().to_owned()
}

/// The `blocked: …` cell a listing shows, or nothing when the artifact is moving.
fn blocked_marker(blockers: &[Blocking]) -> Option<String> {
    if blockers.is_empty() {
        return None;
    }
    let mut types: Vec<&str> = blockers
        .iter()
        .map(|blocking| blocking.category.as_str())
        .collect();
    types.sort_unstable();
    types.dedup();
    Some(format!("blocked: {}", types.join(", ")))
}

/// The artifacts a listing verb was asked for, in id order.
fn select(
    report: &StoreReport,
    blocked: &BTreeMap<ArtifactId, Vec<Blocking>>,
    kind: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Listed>> {
    let kind = match kind {
        Some(value) => {
            Some(ArtifactKind::parse(value).map_err(|error| anyhow::anyhow!("{error}"))?)
        }
        None => None,
    };
    let status = match status {
        Some(value) => Some(parse_status(value)?),
        None => None,
    };

    Ok(report
        .documents
        .values()
        .filter(|stored| {
            let frontmatter = &stored.document.frontmatter;
            // `is_a`, not equality, so `--kind design` answers for an `architecture-design` the way
            // `ArtifactGraph::of_kind` does. One question, one answer, wherever it is asked.
            kind.as_ref()
                .is_none_or(|wanted| frontmatter.kind.is_a(wanted))
                && status
                    .as_ref()
                    .is_none_or(|wanted| &frontmatter.status == wanted)
        })
        .map(|stored| Listed {
            id: stored.document.frontmatter.id.to_string(),
            kind: stored.document.frontmatter.kind.to_string(),
            status: stored.document.frontmatter.status.as_str().to_owned(),
            title: stored.document.frontmatter.title.clone(),
            path: stored.relative_path.clone(),
            relations: stored
                .document
                .frontmatter
                .relations
                .iter()
                .map(|relation| ShownRelation {
                    relation: relation.kind.as_str(),
                    target: relation.target.to_string(),
                })
                .collect(),
            blocked_by: blocked
                .get(&stored.document.frontmatter.id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect())
}

/// The template body for a new artifact of `kind`, when the tree has one.
///
/// Alias-aware, because the templates are named the way a person writes the kind: the file for an
/// `architecture-decision-record` is `adr.md`. The canonical spelling is tried first, so a tree
/// that adds `architecture-decision-record.md` wins over the shorter name rather than being
/// ignored.
fn template(root: &Path, kind: &ArtifactKind) -> Option<String> {
    let alias = match kind {
        ArtifactKind::ArchitectureDecisionRecord => Some("adr"),
        ArtifactKind::ExecutableSystemSpecification => Some("ess"),
        ArtifactKind::ProductRequirements => Some("prd"),
        ArtifactKind::Specification => Some("spec"),
        ArtifactKind::ReviewResult => Some("review"),
        _ => None,
    };
    [Some(kind.as_str()), alias]
        .into_iter()
        .flatten()
        .find_map(|name| {
            std::fs::read_to_string(root.join(TEMPLATE_DIRECTORY).join(format!("{name}.md"))).ok()
        })
}

/// What one relation means, as `artifacts/relations/relations.yaml` states it.
///
/// Duplicated from that document rather than read out of it, and the duplication is deliberate:
/// this verb answers about the vocabulary the *binary* implements, which is
/// [`RelationKind`], and a tree without an `artifacts/` directory would otherwise make
/// `protocol artifact relations` print nothing. `delivers` has no entry in that file — the sentence
/// here matches its declaration in `aep-domain`.
fn meaning(relation: RelationKind) -> &'static str {
    match relation {
        RelationKind::InformedBy => {
            "Shaped by, without being derived from — read and taken into account."
        }
        RelationKind::DerivedFrom => {
            "Produced from a higher-level artifact, whose intent it carries down."
        }
        RelationKind::Decomposes => "Breaks a larger artifact into smaller work.",
        RelationKind::Specifies => "States the required behaviour of something.",
        RelationKind::Designs => "Proposes how to satisfy something.",
        RelationKind::Implements => "Realises something in the system.",
        RelationKind::Decides => "Records a decision taken within something.",
        RelationKind::Reviews => "Assesses something.",
        RelationKind::Verifies => "Establishes that something holds.",
        RelationKind::Blocks => "Prevents progress on something.",
        RelationKind::DependsOn => "Needs something else first.",
        RelationKind::Supersedes => "Replaces something, which becomes superseded.",
        RelationKind::Delivers => "Produces the outcome something asked for.",
        RelationKind::Serves => {
            "Moves an objective the collection has set — a `vision` artifact — and says which."
        }
    }
}

/// A comma-separated list, or `nothing` when there is none.
fn render_list(values: &[&str]) -> String {
    if values.is_empty() {
        return "nothing".to_owned();
    }
    values.join(", ")
}

// ---------------------------------------------------------------------------------------------
// What the machine-readable formats carry
// ---------------------------------------------------------------------------------------------

/// One artifact in a listing.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Listed {
    id: String,
    kind: String,
    status: String,
    title: Option<String>,
    path: String,
    /// Its outgoing edges, `[]` for an artifact that has none.
    ///
    /// **Never `null` and never absent.** `3130470e#132` broke on exactly that: a documented `jq`
    /// shape reading `.relations[]` failed on the first artifact with no edges, because a key that
    /// disappears is a key every consumer has to write a branch for. Same reasoning as `blocked_by`
    /// beside it, and the same shape `show` prints.
    relations: Vec<ShownRelation>,
    /// Every blocker still in force against it, empty for an artifact nothing stops.
    ///
    /// Always written, never omitted when empty: `active` and `active but parked on a credential`
    /// have to be different documents to a machine as well as to a reader, and a key that
    /// disappears is one every consumer writes a branch for.
    blocked_by: Vec<Blocking>,
}

/// One blocker still in force against an artifact, as a listing shows it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Blocking {
    /// The artifact doing the blocking.
    blocker: String,
    /// What would clear it: the blocker kind's type, or [`UNTYPED`].
    #[serde(rename = "type")]
    category: String,
    /// The evidence kind it is stopping anybody from producing, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    withholds: Option<String>,
}

/// One thing stopping work, with everything it is stopping — what `blocked` prints.
///
/// Grouped by the blocker rather than by the blocked, which is the whole point of the verb: five
/// items waiting on one decision are one conversation to have, and a list keyed the other way makes
/// a reader join them by eye.
#[derive(Debug, serde::Serialize)]
struct Blockage {
    /// The artifact doing the blocking.
    blocker: String,
    /// What would clear it.
    #[serde(rename = "type")]
    category: String,
    /// Its own kind, so an untyped blocker still says what it is.
    kind: String,
    /// Where its own lifecycle has got to.
    status: String,
    /// Its title.
    title: Option<String>,
    /// The evidence kind it is stopping anybody from producing.
    #[serde(skip_serializing_if = "Option::is_none")]
    withholds: Option<String>,
    /// What it is stopping, in id order.
    blocks: Vec<Stopped>,
}

/// One artifact a blocker is stopping.
#[derive(Debug, serde::Serialize)]
struct Stopped {
    id: String,
    kind: String,
    status: String,
    title: Option<String>,
}

/// One artifact, whole: what `show` prints.
///
/// Every field is serialised whether or not it is set, which is the opposite of what the text
/// rendering does and deliberate: a machine format whose keys come and go is one every consumer has
/// to write a branch for, while a person reading a labelled block is served by the absent labels
/// being absent.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Shown {
    id: String,
    kind: String,
    status: String,
    title: Option<String>,
    summary: Option<String>,
    owner: Option<String>,
    tags: Vec<String>,
    relations: Vec<ShownRelation>,
    /// The evidence kind this artifact is stopping anybody from producing.
    withholds: Option<String>,
    revision: u64,
    /// The markdown body, exactly as the store holds it.
    body: String,
}

/// One outgoing edge, as `show` prints it.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ShownRelation {
    relation: &'static str,
    target: String,
}

/// One status column of the board.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Column {
    // Owned rather than `&'static str`: a column is named by a rung of a ladder, and a lifecycle
    // document may have invented that name, which no `'static` slice in this binary can hold.
    status: String,
    artifacts: Vec<Listed>,
}

/// What `new` wrote.
#[derive(Debug, serde::Serialize)]
struct Created {
    id: String,
    kind: String,
    // Owned rather than `&'static str`: a status name may be one a lifecycle document invented,
    // which no `'static` slice can hold.
    status: String,
    path: String,
}

/// What `move` did.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Moved {
    id: String,
    from: String,
    to: String,
    revision: u64,
    path: String,
}

/// One evidence record, joined to an artifact through that artifact's event log.
///
/// The join is a **stored fact**, not a path: `reference` is a string the recorder wrote down, and
/// deleting whatever it points at leaves this record exactly where it was. A CI log that rotates
/// away must not take the account of what closed a story with it.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Admitted {
    kind: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    /// When the observation was made, as the recorder gave it.
    at: String,
    /// The revision the artifact was at **when the record was admitted** — so a later edit cannot
    /// make an old record look like it was about the new text.
    revision: u64,
}

/// One status an artifact reached, and what the store holds about why.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Reached {
    from: String,
    to: String,
    at: String,
    /// The revision the artifact was at after the move.
    revision: u64,
    /// The records admitted before this move and after the previous one. Possibly several: a story
    /// satisfied by a suite and by a review rested on both.
    rested_on: Vec<Admitted>,
    /// Present exactly when `rested_on` is empty, saying which kind of claim the move rested on
    /// instead. A status reached on nobody's record is legal and must not be invisible.
    #[serde(skip_serializing_if = "Option::is_none")]
    on_nothing_recorded: Option<String>,
}

/// What `explain` answers: what made this artifact what it is.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Explained {
    artifact: ArtifactId,
    store: String,
    status: String,
    revision: u64,
    /// Every blocker still in force against it.
    ///
    /// The answer to the question the rest of this verb raises and cannot settle: *there is no
    /// record for the next move — why not*. A blocker naming a withheld evidence kind says which
    /// fact is missing and what would have to happen for it to exist, which is a thing a person
    /// can go and do.
    blocked_by: Vec<Blocking>,
    reached: Vec<Reached>,
    /// Records admitted after the last move: held, and not yet the reason for anything.
    recorded_since: Vec<Admitted>,
    /// Where it may go next, and what each of those rungs asks for. Empty for a kind whose lineage
    /// declares no ladder, where *anything is legal* is the honest answer and not a useful one.
    next: Vec<NextRung>,
    unreadable: usize,
}

/// One rung an artifact may move to next, with what that rung costs.
#[derive(Debug, serde::Serialize)]
pub(crate) struct NextRung {
    status: String,
    /// What the ladder asks for to reach it — empty for a rung that asks nothing.
    needs: Vec<Need>,
}

/// One evidence requirement of a rung, against what the artifact holds now.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Need {
    kind: String,
    at_least: usize,
    /// Records of that kind the store holds **about this artifact** — the number `move` reads, not
    /// a count of everything the store knows.
    held: usize,
}

/// What `relate` did.
#[derive(Debug, serde::Serialize)]
struct Related {
    id: String,
    relation: &'static str,
    target: String,
    revision: u64,
}

/// What `body` replaced.
#[derive(Debug, serde::Serialize)]
struct BodyReplaced {
    id: String,
    revision: u64,
    path: String,
}

/// What `set` changed, and what the document reads at afterwards.
#[derive(Debug, serde::Serialize)]
struct FieldsSet {
    id: String,
    /// The frontmatter keys this write carried, in the order the verb reads them.
    fields: Vec<String>,
    revision: u64,
    path: String,
}

/// What `validate` found.
#[derive(Debug, serde::Serialize)]
struct Summary {
    store: String,
    files_read: usize,
    artifacts: usize,
    problems: Vec<String>,
    /// Statuses reached because a caller said the evidence existed, not because the store held it.
    ///
    /// Reported and **not** counted as a problem: refusing an assertion outright would stop
    /// anybody closing a story on the day a runner is down, which is the day it matters most. What
    /// it must not be is invisible.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    closed_on_an_assertion: Vec<String>,
    /// Documents whose frontmatter disagrees with their last event. Counted as problems.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    drift: Vec<String>,
    /// Documents claiming a revision no logged write produced. Counted as problems.
    ///
    /// Detection, not enforcement: nothing here refuses the write that made one, because refusing
    /// it needs to know who wrote the document, which is gap register **D-3** and still proposed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    forged: Vec<String>,
    /// Documents the event log knows and the store no longer holds. Counted as problems.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    deleted: Vec<String>,
    /// Documents with no events at all — they predate the provider, which is not a defect.
    pre_provider: usize,
}

/// One kind in the vocabulary.
#[derive(Debug, serde::Serialize)]
struct KindRow {
    kind: String,
    layer: &'static str,
    planning: bool,
    /// Where the row came from when it did not come from the compiled list, or what the row stands
    /// for when it stands for a family rather than a name.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

/// One relation in the vocabulary.
#[derive(Debug, serde::Serialize)]
struct RelationRow {
    relation: &'static str,
    meaning: &'static str,
    inverse: &'static str,
}

/// One kind's ladder.
#[derive(Debug, serde::Serialize)]
struct Lifecycle {
    kind: String,
    declared: bool,
    initial: String,
    transitions: BTreeMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declared actor is taken as declared, so a driven write is the run's and not a person's.
    ///
    /// Arguments rather than `std::env::set_var`: the process environment is shared by every test
    /// in this binary and they run in parallel, so a test that set `AEP_ACTOR` would decide what a
    /// concurrent test read.
    #[test]
    fn a_declared_actor_is_who_the_write_is_from_and_an_undeclared_one_is_the_logged_in_person() {
        assert_eq!(
            actor_from(Some("agent:W4-3.1"), Some("operator"))
                .expect("a well-formed actor")
                .to_string(),
            "agent:W4-3.1",
            "the declaration wins over `$USER`, or a driven move is journalled as the operator's"
        );
        assert_eq!(
            actor_from(Some("system"), Some("operator"))
                .expect("a well-formed actor")
                .to_string(),
            "system"
        );
        assert_eq!(
            actor_from(None, Some("operator"))
                .expect("a well-formed actor")
                .to_string(),
            "human:operator",
            "nothing declared is the behaviour that was there before, unchanged"
        );
        assert_eq!(
            actor_from(None, Some("Ada Lovelace"))
                .expect("a well-formed actor")
                .to_string(),
            "human:Ada-Lovelace",
            "and a login name that is not an actor name is still sanitised into one"
        );
        assert_eq!(
            actor_from(None, None)
                .expect("a well-formed actor")
                .to_string(),
            "human:unknown"
        );
    }

    /// A declaration nobody can read is refused, never quietly replaced by a person's name.
    #[test]
    fn a_malformed_declared_actor_is_refused_naming_the_variable_and_never_defaulted() {
        for value in ["robot:hal", "alice", "", "agent:"] {
            let error = actor_from(Some(value), Some("operator"))
                .expect_err("a declaration that does not parse is a refusal")
                .to_string();
            assert!(
                error.contains(ACTOR_ENV),
                "the refusal names the variable so the caller knows what to fix: {error}"
            );
            assert!(
                !error.contains("human:operator"),
                "and never falls back to whoever is logged in: {error}"
            );
        }
    }

    #[test]
    fn an_edge_argument_splits_at_the_first_colon_only() {
        // `derived_from:epic:passwordless` has three segments and one meaning: the target id keeps
        // its own colon. Splitting at the last would produce the relation `derived_from:epic`.
        let (relation, target) =
            parse_relation("derived_from:epic:passwordless").expect("a well-formed edge");
        assert_eq!(relation, RelationKind::DerivedFrom);
        assert_eq!(target.to_string(), "epic:passwordless");
    }

    /// Superseded, 2026-08-26, and recorded rather than quietly deleted.
    ///
    /// `every_write_verb_is_journalled` scanned this file for a `store.update` or `store.create`
    /// with no `journal(` call near it. It was the mitigation for deviation **D-P1**: writes went
    /// to the store directly, so the only thing that could see an unrecorded one was a source scan.
    ///
    /// There are no direct writes left. Every verb issues a command and `MarkdownBackend` writes the
    /// file **and** journals it as one act, so a write that records nothing is no longer something a
    /// verb can forget to do — it is not something a verb does at all. The guarantee moved rather
    /// than went away, and what pins it now is
    /// `no_planning_verb_writes_to_the_store_except_through_a_command` in `tests/planning_cli.rs`,
    /// with its own planted-write guard beside it.
    ///
    /// Left as a note because a check that disappears from a file reads exactly like a check nobody
    /// thought was needed.
    const _SUPERSEDED_JOURNAL_SCAN: () = ();
}

// ---------------------------------------------------------------------------------------------
// What another surface may ask this one
//
// `protocol serve` answers a browser with the same facts the terminal prints, and it reaches them
// through the functions below and through nothing else. Each is the compute half of a verb whose
// other half is printing: `board_of` is `board` without the lines, `shown_of` is `show` without
// them, and `moved_by` is `move` without them. Keeping the seam here rather than widening a dozen
// private items means there is exactly one list of what a second reader may see, and it is this.
// ---------------------------------------------------------------------------------------------

/// The board, as columns, for a caller that will render them itself.
pub(crate) fn board_of(location: &StoreLocation, kind: Option<&str>) -> Result<Vec<Column>> {
    let args = StoreArgs {
        location: location.clone(),
        format: Format::Json,
    };
    let opened = open(location, false)?;
    let ladders = ladders_or_none(&args);
    let blocked = blockers_by_target(&opened.report, ladders.lifecycles());
    let listed = select(&opened.report, &blocked, kind, None)?;
    Ok(columns_for(&listed, ladders.lifecycles()))
}

/// One artifact: its fields, its edges and its body.
pub(crate) fn shown_of(location: &StoreLocation, id: &str) -> Result<Shown> {
    let id = artifact_id(id)?;
    let opened = open(location, false)?;
    let stored = opened
        .report
        .documents
        .get(&id)
        .with_context(|| opened.missing(&id))?;
    Ok(shown_from(stored))
}

/// Where one artifact may go next, what each rung costs, and what it already holds.
pub(crate) fn explained_of(location: &StoreLocation, id: &str) -> Result<Explained> {
    let args = StoreArgs {
        location: location.clone(),
        format: Format::Json,
    };
    let id = artifact_id(id)?;
    let opened = open(location, true)?;
    let stored = opened
        .report
        .documents
        .get(&id)
        .with_context(|| opened.missing(&id))?;
    let (entries, unreadable) = entries_from_the_contract(&opened, &id)?;
    let (reached, recorded_since) = joined(&entries);
    let ladders = ladders_or_none(&args);
    let blocked_by = blockers_by_target(&opened.report, ladders.lifecycles())
        .remove(&id)
        .unwrap_or_default();
    let next = next_rungs(
        &stored.document.frontmatter,
        ladders.lifecycles(),
        &opened.evidence_on_hand(&id)?,
    );
    Ok(Explained {
        artifact: id,
        store: opened.plan.describe(),
        status: stored.document.frontmatter.status.to_string(),
        revision: stored.document.frontmatter.revision,
        blocked_by,
        reached,
        recorded_since,
        next,
        unreadable,
    })
}

/// Which plan is being served, so a reader can see it before they move anything in it.
pub(crate) fn store_of(location: &StoreLocation) -> Result<Served> {
    let opened = open(location, false)?;
    Ok(Served {
        store: opened.plan.describe(),
        artifacts: opened.report.documents.len(),
        unreadable: opened.report.failures.len(),
    })
}

/// Moves an artifact, and answers with what it did or why it did not.
///
/// The decision is [`decide_and_move`]'s, which is the same one the terminal gets. A second caller
/// that assembled the steps itself would skip the store-wide re-validation and the guarded-rung
/// rule, and would write a provenance of its own shape — three divergences, all silent, in the
/// write path of a governed store.
pub(crate) fn moved_by(
    location: &StoreLocation,
    id: &str,
    to: &str,
    now: &str,
) -> Result<ServedMove> {
    let args = StoreArgs {
        location: location.clone(),
        format: Format::Json,
    };
    let id = artifact_id(id)?;
    let registry = args.lifecycles()?;
    let mut opened = open(location, true)?;
    let repository = args.repository_root();
    let outcome = decide_and_move(
        &mut opened,
        &registry,
        &repository,
        MoveRequest {
            id: &id,
            to,
            asserted: aep_backend_markdown::kernel::EvidenceOnHand::new(),
            now,
            via: false,
        },
    )?;
    let leans_on_an_assertion = outcome.decided_on.leans_on_an_assertion();
    let refusal = outcome.refusal.map(|stopped| match stopped {
        MoveStopped::Refused { from, refusal } => ServedRefusal {
            from: from.as_str().to_owned(),
            refused: Some(*refusal),
            guarded_rung: None,
            finding: None,
        },
        MoveStopped::GuardedRungOnAWalk {
            from,
            rung,
            refusal,
        } => ServedRefusal {
            from: from.as_str().to_owned(),
            refused: refusal.map(|boxed| *boxed),
            guarded_rung: Some(rung.as_str().to_owned()),
            finding: None,
        },
        MoveStopped::WouldNotValidate { from, to, finding } => ServedRefusal {
            from: from.as_str().to_owned(),
            refused: None,
            guarded_rung: Some(to.as_str().to_owned()),
            finding: Some(finding),
        },
    });
    Ok(ServedMove {
        made: outcome.made,
        leans_on_an_assertion,
        refusal,
    })
}

/// Which plan a reader is looking at.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Served {
    /// The store, in the words `explain` uses for it.
    store: String,
    /// How many artifacts it holds.
    artifacts: usize,
    /// How many files in it would not read, which a reader should know before trusting a count.
    unreadable: usize,
}

/// What a move did, for a caller that is not a terminal.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ServedMove {
    /// The hops that were written. Present even when the move was refused, because a walk that
    /// committed a hop and then stopped changed the store.
    made: Vec<Moved>,
    /// Whether the decision rested partly on a count nothing checks.
    leans_on_an_assertion: bool,
    /// Why it stopped, when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<ServedRefusal>,
}

impl ServedMove {
    /// Whether the move was refused, so a caller can pick a status code without reading the shape.
    pub(crate) fn was_refused(&self) -> bool {
        self.refusal.is_some()
    }
}

/// Why a move stopped, flattened into one shape a reader can branch on.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ServedRefusal {
    /// Where the artifact stood when it stopped.
    from: String,
    /// The ladder's own refusal, carrying every status it would have permitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    refused: Option<aep_backend_markdown::document::MoveRefusal>,
    /// The rung a walk would not cross, or the rung that would have left the store invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    guarded_rung: Option<String>,
    /// What the would-be store would have reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    finding: Option<String>,
}
