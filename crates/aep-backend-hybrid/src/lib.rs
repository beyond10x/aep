//! The plan kept twice — markdown for pull requests, a replica for tooling — under a declared policy.
//!
//! `story:hybrid-backend` (P6), as wave H re-scoped it. The composite is `entity-runtime`'s
//! [`Hybrid`]: a local [`MarkdownProvider`] and a replica `R: Store`, with whose copy wins, where a
//! read goes first, what a silent replica does and what a losing write becomes all **declared** in
//! `project.yaml` (`store: hybrid`, `story:store-selection-in-project-yaml`) and none of them
//! defaulted (`store-v0.1.md` R-106). The contract over it is the adapter, shaped by the plan's own
//! projection: `EntityBackend<Composite<R>, MarkdownProjection>`.
//!
//! # What the atomicity guarantee is
//!
//! Contract commands and hydration use the policy's declared [`Authority`] directly. The authority
//! receives the complete [`AtomicBatchStore`] transaction first; only an accepted authority batch
//! is projected to the other side. A replica failure is a [`Divergence`] and never turns an
//! authority success into a reported command failure. `catch_up` remains the explicit replay path
//! and merges nothing.
//!
//! # Divergences survive the process
//!
//! `protocol artifact` runs one process per verb. A divergence recorded by a write lives in the
//! [`Hybrid`] that saw it, so it is written to [`DIVERGENCES`] beside the plan after every command
//! and handed back with [`Hybrid::remember`] on the next open — where `protocol artifact
//! divergences` lists it and `protocol artifact catch-up` ([`catch_up`]) replays it.
//!
//! # What a hybrid plan reads
//!
//! Through the composite's `Store` traits, so the declared read path governs hydration too. The one
//! thing a `Store` cannot answer — which kinds there are — comes from the local side's directories
//! ([`PlanStore::kinds`]): the files are the plan's shape, and the replica holds the same instances.

use std::path::{Path, PathBuf};

use aep_backend_entity::EntityBackend;
use aep_backend_markdown::projection::MarkdownProjection;
use aep_backend_markdown::provider::{MarkdownProvider, PlanStore};
use aep_contract::command::{CommandEnvelope, CommandResult, CommandService};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{
    AuditQuery, EntityEnvelope, EntityQuery, Page, QueryService, Relation, RelationQuery,
    RevisionRecord,
};
use aep_contract::registry::TypeDescriptor;
use aep_contract::QueryConsistency;
use aep_domain::audit::AuditRecord;
use aep_domain::command::Command;
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::project::HybridPolicy;
use aep_domain::time::Timestamp;
use aep_domain::workspace::MemberName;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_remote::Hybrid;
use entity_store::{
    AtomicBatchStore, AtomicCommit, EventProvider, Expect, StateProvider, Store, StoreError,
};

/// The runtime's record of one disagreement, its four-word policy and the words themselves,
/// re-exported so a caller of this crate names one `entity-runtime` pin: the workspace's.
pub use entity_remote::{
    Authority, Divergence, OnDivergence, Policy, ReadPath, StoreSide, WhenUnreachable,
};

/// The file beside the plan that holds every divergence not yet caught up, one JSON object per line.
pub const DIVERGENCES: &str = "divergences.jsonl";

/// The runtime's composite over the plan's provider and a replica, as a plan-shaped store.
///
/// A newtype because the orphan rule asks for one: [`MarkdownProjection`] hydrates from any
/// [`PlanStore`], and neither that trait nor [`Hybrid`] is this crate's to implement one for the
/// other. Every `Store` call is the hybrid's; the plan-shaped questions are answered by the local
/// side.
#[derive(Debug)]
pub struct Composite<R> {
    local: MarkdownProvider,
    replica: R,
    policy: Policy,
    divergences: Vec<Divergence>,
}

impl<R: Store> Composite<R> {
    /// The composite over the documents at `root` and `replica`, under `policy`.
    pub fn new(root: impl Into<PathBuf>, replica: R, policy: Policy) -> Self {
        Self {
            local: MarkdownProvider::open(root),
            replica,
            policy,
            divergences: Vec::new(),
        }
    }

    /// Every divergence the replica has not reconciled.
    pub fn divergences(&self) -> &[Divergence] {
        &self.divergences
    }

    /// Remembers one divergence from an earlier process.
    pub fn remember(&mut self, divergence: Divergence) {
        if !self.divergences.contains(&divergence) {
            self.divergences.push(divergence);
        }
    }

    /// The declared policy. Commands and contract reads use its authority field; the other words
    /// continue to govern explicit reconciliation outside the contract adapter.
    pub const fn policy(&self) -> Policy {
        self.policy
    }
}

impl<R: Store> StateProvider for Composite<R> {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        match self.policy.authority {
            Authority::Local => self.local.load(entity, id),
            Authority::Remote => self.replica.load(entity, id),
        }
    }

    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        match self.policy.authority {
            Authority::Local => self.local.ids(entity),
            Authority::Remote => self.replica.ids(entity),
        }
    }
}

impl<R: Store> EventProvider for Composite<R> {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        match self.policy.authority {
            Authority::Local => self.local.events(entity, id),
            Authority::Remote => self.replica.events(entity, id),
        }
    }
}

impl<R: AtomicBatchStore> Store for Composite<R> {
    fn commit(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        self.commit_batch(&[AtomicCommit::new(decision.clone(), expect)])
    }
}

impl<R: AtomicBatchStore> AtomicBatchStore for Composite<R> {
    fn commit_batch(&mut self, commits: &[AtomicCommit]) -> Result<(), StoreError> {
        let replica_failure = match self.policy.authority {
            Authority::Local => {
                self.local.commit_batch(commits)?;
                self.replica.commit_batch(commits).err()
            }
            Authority::Remote => {
                self.replica.commit_batch(commits)?;
                self.local.commit_batch(commits).err()
            }
        };
        if let Some(error) = replica_failure {
            for commit in commits {
                let instance = &commit.decision.instance;
                self.remember(Divergence {
                    entity: instance.entity.clone(),
                    id: instance.id.clone(),
                    local_revision: instance.revision,
                    source: match self.policy.authority {
                        Authority::Local => StoreSide::Local,
                        Authority::Remote => StoreSide::Remote,
                    },
                    destination: match self.policy.authority {
                        Authority::Local => StoreSide::Remote,
                        Authority::Remote => StoreSide::Local,
                    },
                    record_id: None,
                    detail: format!(
                        "the authority committed its batch and the replica refused: {error}"
                    ),
                });
            }
        }
        Ok(())
    }
}

impl<R: AtomicBatchStore> PlanStore for Composite<R> {
    fn root(&self) -> &Path {
        self.local.root()
    }

    fn kinds(&self) -> Result<Vec<String>, StoreError> {
        self.local.kinds()
    }
}

/// The runtime's policy, from the words `project.yaml` spells it in.
///
/// The words were already checked when the project file was read (`aep.project/1` refuses a hybrid
/// missing one, or spelling one wrong); this is the mapping, and a word this build does not know is
/// a refusal naming it rather than a default.
///
/// # Errors
///
/// If a word is not one this build maps.
pub fn policy_from(policy: &HybridPolicy) -> Result<Policy, CommandError> {
    let unknown = |field: &str, word: &str| CommandError::Conflict {
        reason: format!("`{field}: {word}` is not a word this build knows for a hybrid store"),
    };
    let authority = match policy.authority.as_str() {
        "local" => Authority::Local,
        "replica" => Authority::Remote,
        other => return Err(unknown("authority", other)),
    };
    let read_path = match policy.read.as_str() {
        "local-first" => ReadPath::LocalFirst,
        "replica-first" => ReadPath::RemoteFirst,
        "replica-only" => ReadPath::RemoteOnly,
        other => return Err(unknown("read", other)),
    };
    let when_unreachable = match policy.on_unreachable.as_str() {
        "refuse" => WhenUnreachable::Refuse,
        "serve-stale" => WhenUnreachable::ServeStale,
        other => return Err(unknown("on_unreachable", other)),
    };
    let on_divergence = match policy.on_divergence.as_str() {
        "refuse" => OnDivergence::Refuse,
        "record" => OnDivergence::RecordDivergence,
        other => return Err(unknown("on_divergence", other)),
    };
    Ok(Policy::new(
        authority,
        read_path,
        when_unreachable,
        on_divergence,
    ))
}

/// The divergences written beside the plan at `root`, oldest first. None when there is no file.
///
/// # Errors
///
/// If the file exists and cannot be read, or holds a line that is not a divergence — refused rather
/// than skipped, because a divergence that silently stopped being counted is the failure this file
/// exists against.
pub fn read_divergences(root: &Path) -> Result<Vec<Divergence>, CommandError> {
    let path = root.join(DIVERGENCES);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(CommandError::Conflict {
                reason: format!("reading {}: {error}", path.display()),
            })
        }
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|error| CommandError::Conflict {
                reason: format!(
                    "{} holds a line that is not a divergence: {error}",
                    path.display()
                ),
            })
        })
        .collect()
}

/// Writes `divergences` as the whole of the file beside the plan at `root`, or removes it when
/// there are none — so the file's presence is itself the answer to *is anything outstanding*.
///
/// # Errors
///
/// If the file cannot be written.
pub fn write_divergences(root: &Path, divergences: &[Divergence]) -> Result<(), CommandError> {
    let path = root.join(DIVERGENCES);
    let failed = |error: std::io::Error| CommandError::Conflict {
        reason: format!("writing {}: {error}", path.display()),
    };
    if divergences.is_empty() {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(failed(error)),
        };
    }
    let mut lines = String::new();
    for divergence in divergences {
        lines.push_str(&serde_json::to_string(divergence).map_err(|error| {
            CommandError::Conflict {
                reason: format!("serialising a divergence: {error}"),
            }
        })?);
        lines.push('\n');
    }
    std::fs::create_dir_all(root).map_err(failed)?;
    std::fs::write(&path, lines).map_err(failed)
}

/// The contract over a plan kept in markdown and in a replica.
#[derive(Debug)]
pub struct HybridBackend<R: AtomicBatchStore>(EntityBackend<Composite<R>, MarkdownProjection>);

impl<R: AtomicBatchStore> HybridBackend<R> {
    /// Opens the plan at `root` with `replica` under `policy`, hydrating the contract from what the
    /// composite holds and remembering every divergence an earlier process wrote beside the plan.
    ///
    /// `members`, `at`, `actor` and `lifecycles` are the markdown projection's, as for
    /// `MarkdownBackend::open`.
    ///
    /// # Errors
    ///
    /// If either side cannot be read, if the plan does not build a graph, or if the divergence
    /// file cannot be read.
    pub fn open(
        root: impl AsRef<Path>,
        replica: R,
        policy: Policy,
        members: impl IntoIterator<Item = MemberName>,
        at: Timestamp,
        actor: ActorRef,
        lifecycles: aep_domain::artifact::LifecycleRegistry,
    ) -> Result<Self, CommandError> {
        let root = root.as_ref();
        let mut composite = Composite::new(root, replica, policy);
        for divergence in read_divergences(root)? {
            composite.remember(divergence);
        }
        let projection = MarkdownProjection::new(members, at, actor, lifecycles);
        Ok(Self(EntityBackend::shaped(composite, projection)?))
    }

    /// Every divergence outstanding: recorded by this process or handed back from the file.
    pub fn divergences(&self) -> Vec<Divergence> {
        self.0
            .with_store(|composite| composite.divergences().to_vec())
    }

    /// The policy in force.
    pub fn policy(&self) -> Policy {
        self.0.with_store(Composite::policy)
    }

    /// The fault that made this backend untrustworthy, if one has happened.
    pub fn latched(&self) -> Option<String> {
        self.0.latched()
    }

    /// How many entities the contract holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The adapter this is an instantiation of, for a caller that wants the generic surface.
    pub const fn as_entity_backend(&self) -> &EntityBackend<Composite<R>, MarkdownProjection> {
        &self.0
    }

    /// Writes the outstanding divergences beside the plan, so the next process sees them.
    fn persist_divergences(&self) -> Result<(), CommandError> {
        let (root, divergences) = self.0.with_store(|composite| {
            (
                composite.root().to_path_buf(),
                composite.divergences().to_vec(),
            )
        });
        write_divergences(&root, &divergences)
    }
}

impl<R: AtomicBatchStore> CommandService for HybridBackend<R> {
    type Command = Command;

    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        let outcome = self.0.execute(envelope).await;
        // Whatever the command's outcome, what diverged reaches the file: a divergence is recorded
        // on an accepted write under `record` and on a refused one under `refuse`, and either is
        // lost with the process unless written now. A file that cannot be written is said, even
        // over an accepted command — the command landed and the record of its divergence did not,
        // which is the one thing the caller must not be left believing otherwise.
        self.persist_divergences()?;
        outcome
    }
}

impl<R: AtomicBatchStore> QueryService for HybridBackend<R> {
    type AuditRecord = AuditRecord;

    async fn get(
        &self,
        reference: &EntityRef,
        consistency: QueryConsistency,
    ) -> Result<EntityEnvelope, QueryError> {
        self.0.get(reference, consistency).await
    }

    async fn resolve(&self, locator: &EntityLocator) -> Result<EntityId, QueryError> {
        self.0.resolve(locator).await
    }

    async fn query(&self, query: &EntityQuery) -> Result<Page<EntityEnvelope>, QueryError> {
        self.0.query(query).await
    }

    async fn relations(&self, query: &RelationQuery) -> Result<Page<Relation>, QueryError> {
        self.0.relations(query).await
    }

    async fn history(&self, reference: &EntityRef) -> Result<Vec<RevisionRecord>, QueryError> {
        self.0.history(reference).await
    }

    async fn audit(&self, query: &AuditQuery) -> Result<Page<Self::AuditRecord>, QueryError> {
        self.0.audit(query).await
    }

    async fn describe_type(&self, entity_type: &EntityType) -> Result<TypeDescriptor, QueryError> {
        self.0.describe_type(entity_type).await
    }
}

/// What a catch-up did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchUp {
    /// How many divergences were outstanding before.
    pub found: usize,
    /// The ones still outstanding after, with why — a replica that moved on its own, a side that
    /// could not be read — for a person. Written back beside the plan.
    pub outstanding: Vec<Divergence>,
}

impl CatchUp {
    /// How many were replayed, or found already caught up.
    #[must_use]
    pub fn replayed(&self) -> usize {
        self.found.saturating_sub(self.outstanding.len())
    }
}

/// Replays every divergence written beside the plan at `root` at the side that has not seen it.
///
/// Over the bare composite rather than the contract, because a catch-up is the runtime's
/// reconciliation (`store-v0.1.md` R-108) and not a command: nothing is decided, and the adapter's
/// one door stays the one door for what is. What stays outstanding is written back; a reconciliation
/// that cleared its own file on a partial success would report success and lose the rest.
///
/// # Errors
///
/// If the divergence file cannot be read or written.
pub fn catch_up<R: Store>(
    root: &Path,
    replica: R,
    policy: Policy,
) -> Result<CatchUp, CommandError> {
    let mut hybrid = Hybrid::new(MarkdownProvider::open(root), replica, policy);
    let recorded = read_divergences(root)?;
    let found = recorded.len();
    for divergence in recorded {
        hybrid.remember(divergence);
    }
    hybrid.catch_up();
    let outstanding = hybrid.divergences().to_vec();
    write_divergences(root, &outstanding)?;
    Ok(CatchUp { found, outstanding })
}
