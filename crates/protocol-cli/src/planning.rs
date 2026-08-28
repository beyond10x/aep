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

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aep_backend_markdown::{MarkdownStore, PlanningDocument, PlanningFrontmatter, StoreReport};
use aep_domain::artifact::{
    ArtifactGraph, ArtifactId, ArtifactKind, ArtifactLifecycle, ArtifactRef, ArtifactStatus,
    RelationKind,
};
use aep_engine::project::project_directory;
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
#[derive(Debug, Args)]
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
    /// The repository the store sits in, so a workspace beside it can be found.
    ///
    /// `<repo>/.engineering/planning` is the store, so the repository is two directories up. An
    /// explicit `--store` somewhere else answers the same way, which is what lets one repository's
    /// verbs validate another's store.
    fn repository_root(&self) -> PathBuf {
        self.store.as_ref().map_or_else(
            || PathBuf::from("."),
            |path| {
                path.parent()
                    .and_then(std::path::Path::parent)
                    .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf)
            },
        )
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
        let Some(project) = aep_engine::project::discover(&here) else {
            return Ok(PathBuf::from("."));
        };
        aep_engine::project::load_paths(&project)
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
        let project = aep_engine::project::discover(&here).with_context(|| {
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
        Ok(Self { plan })
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
            let store = entity_postgres::PostgresStore::connect(url)
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
            let store = entity_postgres::PostgresStore::connect(url)
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
    },
    /// Add an edge from one plan item to another.
    Relate {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact the edge starts at.
        id: String,
        /// What the edge means, such as `decomposes`. `protocol artifact relations` lists them.
        relation: String,
        /// The artifact the edge points at.
        target: String,
    },
    /// Replace a plan item's markdown body while preserving CLI-owned frontmatter.
    Body {
        /// Where the plan is and how to render.
        #[command(flatten)]
        store: StoreArgs,
        /// The artifact, such as `story:passkey-login`.
        id: String,
        /// Read the complete replacement body from this UTF-8 file; `-` reads standard input.
        #[arg(long, value_name = "PATH")]
        from: PathBuf,
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
    },
    /// List the artifact kinds, marking the ones that are planning rather than output.
    Kinds {
        /// How to render. `--store` is not read: this answers from the vocabulary.
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
    #[arg(long)]
    title: String,
    /// A one-line summary.
    #[arg(long)]
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
        } => move_status(&store, &id, &to, &evidence, at.as_deref()),
        ArtifactCommand::Relate {
            store,
            id,
            relation,
            target,
        } => relate(&store, &id, &relation, &target),
        ArtifactCommand::Body { store, id, from } => replace_body(&store, &id, &from),
        ArtifactCommand::List {
            store,
            kind,
            status,
        } => list(&store, kind.as_deref(), status.as_deref()),
        ArtifactCommand::Board { store, kind } => board(&store, kind.as_deref()),
        ArtifactCommand::Graph { store, format } => graph(&store, format),
        ArtifactCommand::Validate { store } => validate(&store),
        ArtifactCommand::History { store, id } => history(&store, &id),
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
    aep_engine::project::load_workspace(root).map_or_else(
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

/// Who the command is from.
///
/// `human:` because a person typed it. The store cannot verify an identity, and a field that looks
/// verified and is not is worse than one that plainly is not.
pub(crate) fn command_actor() -> Result<aep_domain::entity::ActorRef> {
    let name = actor_of(None);
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
    for value in &args.relate {
        let (relation, target) = parse_relation(value)?;
        frontmatter
            .relations
            .push(aep_domain::artifact::ArtifactRelation::new(
                relation, target,
            ));
    }

    let body = template(&document_root, &kind).unwrap_or_else(|| format!("# {}\n", args.title));
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

/// `protocol artifact move`
fn move_status(
    args: &StoreArgs,
    id: &str,
    to: &str,
    evidence: &[String],
    at: Option<&str>,
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

    // Evidence recorded *about this artifact* is found rather than typed. Both origins are kept
    // apart all the way through the decision and into the journal, so the history can say what the
    // move rested on — see `journal::Provenance`.
    let decided_on = aep_backend_markdown::journal::Provenance {
        recorded: opened.evidence_on_hand(&id)?,
        asserted,
    };
    let evidence = decided_on.total();

    let not_here = opened.missing(&id);
    let stored = opened
        .report
        .documents
        .get_mut(&id)
        .with_context(|| not_here)?;
    let from = stored.document.frontmatter.status.clone();

    // The target is read *after* the artifact, because what a status name may be is decided by the
    // ladder this kind declares and not by a list compiled into this binary. `ArtifactStatus` is an
    // open vocabulary; the ladder is what keeps it open to authors and closed to typos.
    let kind = stored.document.frontmatter.kind.clone();
    let to = parse_status_in(to, &kind, registry.lifecycles())?;

    if let Err(refusal) =
        stored
            .document
            .move_status(to.clone(), registry.lifecycles(), &evidence, Some(&now))
    {
        outln!("{id} is {from}; {refusal}");
        return Ok(crate::exit_code(false));
    }

    let relative = stored.relative_path.clone();
    let document = stored.document.clone();
    // Through the command the vocabulary gained for this. The engine has already decided the move
    // above, against the kind's ladder and the evidence presented; what crosses here is the
    // decision and the account it rested on. `MarkdownBackend` writes the file and journals it.
    let _ = relative;
    move_through_a_command(opened.backend()?, &id, &to, &decided_on)?;
    let path = opened.path_of(&id);

    match args.format {
        Format::Text => {
            outln!(
                "{id} moved {from} -> {to} (revision {})",
                document.frontmatter.revision
            );
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
        Format::Yaml | Format::Json => crate::print_serialised(
            &Moved {
                id: id.to_string(),
                from: from.as_str().to_owned(),
                to: to.as_str().to_owned(),
                revision: document.frontmatter.revision,
                path,
            },
            args.format,
        )?,
    }
    Ok(ExitCode::SUCCESS)
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
fn relate(args: &StoreArgs, id: &str, relation: &str, target: &str) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    let relation = RelationKind::parse(relation).map_err(|error| anyhow::anyhow!("{error}"))?;
    let target = ArtifactRef::parse(target).map_err(|error| anyhow::anyhow!("{error}"))?;

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

/// `protocol artifact body`
fn replace_body(args: &StoreArgs, id: &str, from: &Path) -> Result<ExitCode> {
    let id = artifact_id(id)?;
    let body = if from == Path::new("-") {
        let mut body = String::new();
        std::io::stdin()
            .read_to_string(&mut body)
            .context("reading the replacement body from standard input")?;
        body
    } else {
        std::fs::read_to_string(from)
            .with_context(|| format!("reading the replacement body from {}", from.display()))?
    };

    let mut opened = open(&args.location, true)?;
    let not_here = opened.missing(&id);
    let stored = opened
        .report
        .documents
        .get_mut(&id)
        .with_context(|| not_here)?;
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
        "protocol-artifact-body",
    )?;
    let path = opened.path_of(&id);
    match args.format {
        Format::Text => outln!(
            "{id} body replaced (revision {}) at {path}",
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

/// `protocol artifact list`
fn list(args: &StoreArgs, kind: Option<&str>, status: Option<&str>) -> Result<ExitCode> {
    let opened = open(&args.location, false)?;
    let listed = select(&opened.report, kind, status)?;

    match args.format {
        Format::Text => crate::print_table(
            &listed
                .iter()
                .map(|entry| {
                    vec![
                        entry.id.clone(),
                        entry.kind.clone(),
                        entry.status.clone(),
                        entry.title.clone().unwrap_or_default(),
                    ]
                })
                .collect::<Vec<_>>(),
        ),
        Format::Yaml | Format::Json => crate::print_serialised(&listed, args.format)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// `protocol artifact board`
fn board(args: &StoreArgs, kind: Option<&str>) -> Result<ExitCode> {
    let opened = open(&args.location, false)?;
    let listed = select(&opened.report, kind, None)?;
    // Every status, in the vocabulary's own order, and only the ones with something in them: an
    // empty column is a column a reader has to skip on every glance.
    let columns: Vec<Column> = ArtifactStatus::ALL
        .iter()
        .map(|status| Column {
            status: status.as_str(),
            artifacts: listed
                .iter()
                .filter(|entry| entry.status == status.as_str())
                .cloned()
                .collect(),
        })
        .filter(|column| !column.artifacts.is_empty())
        .collect();

    match args.format {
        Format::Text => {
            for (index, column) in columns.iter().enumerate() {
                if index > 0 {
                    outln!();
                }
                outln!("{} ({})", column.status, column.artifacts.len());
                for entry in &column.artifacts {
                    outln!(
                        "  {}  {}",
                        entry.id,
                        entry.title.clone().unwrap_or_default()
                    );
                }
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&columns, args.format)?,
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
    /// The deletion findings alone.
    deleted: Vec<String>,
    /// Documents with no events at all.
    pre_provider: usize,
}

/// The documents against the event log (wave G, story 4), and the journal against the files.
///
/// A frontmatter field that disagrees with what the events say is **drift** — an edit made in an
/// editor; events with no document are a **deletion**; a document with no events predates the
/// provider and is neither. The journal's older reconciliation covers the same log's status and
/// revision by entry, so a document the drift check names is not named twice, and an orphan the
/// log knows as deleted is said once, as that.
fn log_findings(
    root: &Path,
    report: &aep_backend_markdown::store::StoreReport,
    held: &std::collections::BTreeMap<ArtifactId, (aep_domain::artifact::ArtifactStatus, u64)>,
) -> LogFindings {
    let drift = aep_backend_markdown::drift::detect(root, &report.documents);
    let drifted: std::collections::BTreeSet<_> =
        drift.drift.iter().map(|d| d.artifact.clone()).collect();
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
    let deleted_findings: Vec<String> = drift.deleted.iter().map(ToString::to_string).collect();
    problems.extend(drift_findings.iter().cloned());
    problems.extend(deleted_findings.iter().cloned());
    LogFindings {
        problems,
        drift: drift_findings,
        deleted: deleted_findings,
        pre_provider: drift.pre_provider,
    }
}

/// `protocol artifact validate`
fn validate(args: &StoreArgs) -> Result<ExitCode> {
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
    let (drift_findings, deleted_findings, pre_provider) = match &opened.files {
        Some(store) => {
            let log = log_findings(store.root(), report, &held);
            problems.extend(log.problems);
            (log.drift, log.deleted, log.pre_provider)
        }
        None => (Vec::new(), Vec::new(), 0),
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
        deleted: deleted_findings,
        pre_provider,
    };

    match args.format {
        Format::Text => {
            outln!(
                "{} file(s) in {}: {} artifact(s)",
                summary.files_read,
                summary.store,
                summary.artifacts
            );
            // A normal condition, said out loud: a document with no events cannot be checked
            // against its log, and a reader should know how many of those there are.
            if summary.pre_provider > 0 {
                outln!("{} document(s) predate the event log", summary.pre_provider);
            }
            // Reported, and deliberately **not** counted as a problem. Refusing an assertion
            // outright would stop anybody closing a story on the day a runner is down, which is the
            // day it matters most. What it must not be is invisible.
            if !asserted.is_empty() {
                outln!("{} closed on an assertion:", asserted.len());
                for note in &asserted {
                    outln!("  - {note}");
                }
            }
            if problems.is_empty() {
                outln!("valid");
            } else {
                outln!("{} problem(s):", problems.len());
                for problem in &problems {
                    outln!("  - {problem}");
                }
            }
        }
        Format::Yaml | Format::Json => crate::print_serialised(&summary, args.format)?,
    }
    Ok(crate::exit_code(problems.is_empty()))
}

/// `protocol artifact kinds`
fn kinds(args: &StoreArgs) -> Result<ExitCode> {
    let listed: Vec<KindRow> = ArtifactKind::NAMED
        .iter()
        .map(|kind| KindRow {
            kind: kind.as_str().to_owned(),
            layer: if kind.is_planning() {
                "planning"
            } else {
                "output"
            },
            planning: kind.is_planning(),
        })
        .collect();

    match args.format {
        Format::Text => crate::print_table(
            &listed
                .iter()
                .map(|entry| vec![entry.kind.clone(), entry.layer.to_owned()])
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
    let kind = aep_domain::evidence::EvidenceKind::parse(kind.trim())
        .with_context(|| format!("`{kind}` is not a kind of evidence"))?;
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
            let store = entity_postgres::PostgresStore::connect(url)
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
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::entity::EntityLocator;

    let opened = open(&args.location, true)?;
    let backend = opened.backend()?;
    let locator = EntityLocator::new(
        aep_backend_markdown::backend::ORGANISATION,
        aep_backend_markdown::backend::SPACE,
        id.namespace(),
        id.name(),
    )
    .map_err(|error| anyhow::anyhow!("`{id}` cannot be given an address: {error}"))?;
    let entity = block_on(backend.resolve(&locator)).with_context(|| opened.missing(id))?;
    let events = backend.events_of(&entity)?;

    let mut entries = Vec::new();
    let mut unreadable = 0;
    for event in &events {
        match entry_from_event(backend, id, event) {
            Some(entry) => entries.push(entry),
            None => unreadable += 1,
        }
    }
    print_history(args.format, id, &entries, unreadable)
}

/// The journal entry a plan's event stands for, when the command it carries is one a plan issues.
fn entry_from_event(
    backend: &PlanBackend,
    id: &ArtifactId,
    event: &entity_core::DomainEvent,
) -> Option<aep_backend_markdown::journal::Entry> {
    use aep_backend_markdown::journal::{Change, Entry, Provenance};
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
                .cloned()
                .map_or_else(Provenance::default, |account| {
                    serde_json::from_value(account).unwrap_or_default()
                }),
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

/// Who is doing this, as they are willing to say.
///
/// The store cannot verify an identity, so this is free text and the journal's own documentation
/// says as much. A field that looks verified and is not is worse than one that plainly is not.
fn actor_of(given: Option<&str>) -> String {
    given.map_or_else(
        || std::env::var("USER").unwrap_or_else(|_| "unknown".to_owned()),
        str::to_owned,
    )
}

/// The instant this invocation ran, ISO-8601, read from the system clock.
///
/// The only clock in the planning path, and it is in the shell by construction: `aep-domain` has a
/// banned-token scan that would refuse one, and a decision that read the clock itself could not be
/// replayed. Its answer is an *argument* to the move, printed with any dated refusal.
fn now_at_the_edge() -> String {
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
fn parse_evidence(pairs: &[String]) -> Result<aep_backend_markdown::kernel::EvidenceOnHand> {
    let mut counts = aep_backend_markdown::kernel::EvidenceOnHand::new();
    for pair in pairs {
        let (kind, count) = pair.split_once('=').with_context(|| {
            format!("`{pair}` is not evidence; write it as <kind>=<count>, such as test_result=1")
        })?;
        let kind = aep_domain::evidence::EvidenceKind::parse(kind.trim())
            .map_err(|error| anyhow::anyhow!("{error}"))?;
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

/// The artifacts a listing verb was asked for, in id order.
fn select(report: &StoreReport, kind: Option<&str>, status: Option<&str>) -> Result<Vec<Listed>> {
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
struct Listed {
    id: String,
    kind: String,
    status: String,
    title: Option<String>,
    path: String,
}

/// One status column of the board.
#[derive(Debug, serde::Serialize)]
struct Column {
    status: &'static str,
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
struct Moved {
    id: String,
    from: String,
    to: String,
    revision: u64,
    path: String,
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
