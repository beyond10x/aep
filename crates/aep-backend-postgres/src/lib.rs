//! The interaction contract over PostgreSQL.
//!
//! **P5**, re-scoped by wave H of `docs/plan/store-waves-f-g-h.md`: the backend an organisation
//! actually runs — concurrent writers, real transactions, the backup story it already has — as a
//! *type*. [`PostgresBackend`] is [`EntityBackend`] over `entity_postgres::PostgresStore`, exactly
//! as `aep-backend-sqlite` is the adapter over `SqliteStore`. Nothing here decides anything: the
//! contract logic lives in `aep-backend-memory`, the durability logic in the adapter, and the
//! transaction, the row lock and the two-writers guarantee in the runtime's provider.
//!
//! # What two writers get
//!
//! Two processes open one database, hydrate, and both move one artifact from the revision they
//! read. The provider serialises them: the first lands; the second's commit is refused with the
//! revision it lost to, and the command returns a conflict naming that revision. Its detached
//! candidate is never published, so the adapter retains the pre-command view and does not latch;
//! reopening reads what the winner wrote. Never a silent last-writer-wins.
//!
//! # A gate that says when it did not run
//!
//! This workspace's gate reaches no network. The tests here run when `ENTITY_POSTGRES_URL` names a
//! server — each in a schema of its own — and the gate's `postgres-check` step prints
//! `postgres-check: skipped, ENTITY_POSTGRES_URL unset` when it does not, so a green gate cannot
//! read as a tested backend. CI sets it, against a service container.

use std::collections::BTreeSet;
use std::sync::Mutex;

use aep_backend_entity::{EntityBackend, Identity, APPLIED_AS, METADATA_KEY, RELATIONS_AS};
use aep_contract::command::{CommandEnvelope, CommandResult, CommandService};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{
    AuditQuery, EntityEnvelope, EntityQuery, Page, QueryService, Relation, RelationQuery,
    RevisionRecord,
};
use aep_contract::registry::TypeDescriptor;
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::audit::AuditRecord;
use aep_domain::command::Command;
use aep_domain::entity::{EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::ids::RelationId;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_postgres::PostgresStore;
use entity_query::DocumentQuery;
use entity_store::{
    AtomicBatchStore, AtomicCommit, EventProvider, Expect, MemoryStore, StateProvider, Store,
    StoreError,
};
use serde_json::json;

/// The entity type every contract entity is stored under, re-exported from the adapter.
pub use aep_backend_entity::STORED_AS;

/// The contract, over a PostgreSQL database.
///
/// A newtype rather than a type alias because the constructors are inherent methods, and an
/// inherent method can only be written in the crate that owns the type. Every trait method forwards.
#[derive(Debug)]
pub struct PostgresBackend(EntityBackend<PostgresStore>);

impl PostgresBackend {
    /// Connects to `url`, prepares the schema, and hydrates the contract from everything it holds.
    ///
    /// # Errors
    ///
    /// If the server cannot be reached, the schema cannot be established, or the database holds a
    /// row the adapter cannot read back — refused by name rather than skipped.
    pub fn connect(url: &str) -> Result<Self, CommandError> {
        let durable =
            PostgresStore::connect_no_tls(url).map_err(|error| CommandError::Conflict {
                reason: format!("the database could not be opened: {error}"),
            })?;
        Ok(Self(EntityBackend::over(durable)?))
    }

    /// As [`Self::connect`], keeping everything under `schema` — several plans in one database, or
    /// a test that wants a store of its own on a shared server.
    ///
    /// # Errors
    ///
    /// As [`Self::connect`].
    pub fn connect_in_schema(url: &str, schema: &str) -> Result<Self, CommandError> {
        let durable = PostgresStore::connect_in_schema(url, schema).map_err(|error| {
            CommandError::Conflict {
                reason: format!("the database could not be opened: {error}"),
            }
        })?;
        Ok(Self(EntityBackend::over(durable)?))
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
    pub const fn as_entity_backend(&self) -> &EntityBackend<PostgresStore> {
        &self.0
    }
}

/// A fresh, dependency-scoped command session over PostgreSQL.
///
/// Unlike [`PostgresBackend`], this backend never hydrates the complete provider. Each command
/// opens one outer transaction, locks its idempotency record and direct dependencies, evaluates the
/// existing entity adapter over that bounded view, then commits the captured batch through the same
/// transaction. Semantic refusals are values inside the successful outer transaction so their
/// audit record commits; provider failures roll the transaction back.
pub struct SessionPostgresBackend {
    durable: Mutex<PostgresStore>,
}

impl std::fmt::Debug for SessionPostgresBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionPostgresBackend")
            .finish_non_exhaustive()
    }
}

impl SessionPostgresBackend {
    /// Connects without hydrating provider rows.
    pub fn connect(url: &str) -> Result<Self, CommandError> {
        let durable =
            PostgresStore::connect_no_tls(url).map_err(|error| provider_command_error(&error))?;
        Ok(Self {
            durable: Mutex::new(durable),
        })
    }

    /// Connects in one schema without hydrating provider rows.
    pub fn connect_in_schema(url: &str, schema: &str) -> Result<Self, CommandError> {
        let durable = PostgresStore::connect_in_schema(url, schema)
            .map_err(|error| provider_command_error(&error))?;
        Ok(Self {
            durable: Mutex::new(durable),
        })
    }
}

fn provider_command_error(error: &StoreError) -> CommandError {
    CommandError::Conflict {
        reason: format!("the database refused the command session: {error}"),
    }
}

#[derive(Debug, Default)]
struct ScopedStore {
    inner: MemoryStore,
    committed: Vec<AtomicCommit>,
}

impl ScopedStore {
    fn preload(
        &mut self,
        instance: EntityInstance,
        events: Vec<DomainEvent>,
    ) -> Result<(), StoreError> {
        self.inner
            .commit(&Decision::legacy_import(instance, events), Expect::Absent)
    }
}

impl StateProvider for ScopedStore {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        self.inner.load(entity, id)
    }

    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        self.inner.ids(entity)
    }
}

impl EventProvider for ScopedStore {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        self.inner.events(entity, id)
    }
}

impl Store for ScopedStore {
    fn commit(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        self.inner.commit(decision, expect)?;
        self.committed
            .push(AtomicCommit::new(decision.clone(), expect));
        Ok(())
    }
}

impl AtomicBatchStore for ScopedStore {
    fn commit_batch(&mut self, commits: &[AtomicCommit]) -> Result<(), StoreError> {
        self.inner.commit_batch(commits)?;
        self.committed.extend_from_slice(commits);
        Ok(())
    }
}

fn command_dependencies(command: &Command) -> (BTreeSet<EntityId>, Option<RelationId>) {
    let mut entities = BTreeSet::new();
    let relation = match command {
        Command::UpdateEntity(command) => {
            entities.insert(command.target.id.clone());
            None
        }
        Command::CreateRelation(command) => {
            entities.insert(command.source.id.clone());
            entities.insert(command.target.id.clone());
            None
        }
        Command::RemoveRelation(command) => Some(command.relation.clone()),
        Command::ArchiveEntity(command) => {
            entities.insert(command.target.id.clone());
            None
        }
        Command::SupersedeEntity(command) => {
            entities.insert(command.target.id.clone());
            entities.insert(command.successor.id.clone());
            None
        }
        Command::SubmitDesignReview(command) => {
            entities.insert(command.design.id.clone());
            None
        }
        Command::ApproveDesign(command) => {
            entities.insert(command.design.id.clone());
            entities.insert(command.review.id.clone());
            None
        }
        Command::AcceptAdr(command) => {
            entities.insert(command.adr.id.clone());
            if let Some(superseded) = &command.supersedes {
                entities.insert(superseded.id.clone());
            }
            None
        }
        Command::MoveStatus(command) => {
            entities.insert(command.target.id.clone());
            None
        }
        Command::RecordEvidence(command) => {
            entities.insert(command.target.id.clone());
            None
        }
        _ => None,
    };
    (entities, relation)
}

fn preload(
    session: &mut entity_postgres::PostgresSession<'_>,
    scoped: &mut ScopedStore,
    entity: &str,
    id: &str,
) -> Result<(), StoreError> {
    if let Some(instance) = session.load_for_update(entity, id)? {
        let events = session.events(entity, id)?;
        scoped.preload(instance, events)?;
    }
    Ok(())
}

impl CommandService for SessionPostgresBackend {
    type Command = Command;

    #[allow(clippy::unused_async_trait_impl)]
    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        let mut durable = self
            .durable
            .lock()
            .expect("the PostgreSQL command-session lock is not poisoned");
        let result = durable
            .with_transaction(|session| {
                let mut scoped = ScopedStore::default();
                let sequence = session.reserve_sequence("aep.command-identities", 32)?;
                let sequence_floor = (1_u64 << 48).saturating_add(sequence);

                let key = envelope.context.idempotency_key.to_string();
                preload(session, &mut scoped, APPLIED_AS, &key)?;

                let (entities, relation) = command_dependencies(&envelope.payload);
                for id in entities {
                    preload(session, &mut scoped, STORED_AS, id.as_str())?;
                }
                if let Some(relation) = relation {
                    preload(session, &mut scoped, RELATIONS_AS, relation.as_str())?;
                }

                if let Command::CreateEntity(create) = &envelope.payload {
                    let locator = create.locator.to_string();
                    session.lock_identity("aep.locator", &locator)?;
                    let query = DocumentQuery::for_entity(STORED_AS)
                        .matching(
                            METADATA_KEY,
                            json!({ "metadata": { "locator": create.locator } }),
                        )
                        .with_limit(2);
                    let page = session.query_documents(&query).map_err(|error| {
                        StoreError::Backend(format!("querying an entity locator: {error}"))
                    })?;
                    if page.items.len() > 1 {
                        return Err(StoreError::Backend(format!(
                            "locator `{locator}` names more than one stored entity"
                        )));
                    }
                    for instance in page.items {
                        let events = session.events(&instance.entity, &instance.id)?;
                        scoped.preload(instance, events)?;
                    }
                }

                let backend =
                    EntityBackend::shaped(scoped, Identity::with_sequence_floor(sequence_floor))
                        .map_err(|error| StoreError::Backend(error.to_string()))?;
                let outcome = block_on(backend.execute(envelope));
                let commits = backend.with_store(|store| store.committed.clone());
                for commit in &commits {
                    if commit.expect == Expect::Absent {
                        session.lock_identity(
                            &commit.decision.instance.entity,
                            &commit.decision.instance.id,
                        )?;
                    }
                }
                session.commit_batch(&commits)?;
                Ok(outcome)
            })
            .map_err(|error| provider_command_error(&error))?;
        result
    }
}

impl CommandService for PostgresBackend {
    type Command = Command;

    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        self.0.execute(envelope).await
    }
}

impl QueryService for PostgresBackend {
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
