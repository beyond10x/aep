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
//! revision it lost to, the adapter **latches** — its memory and the database disagree — and the
//! command returns a conflict naming that revision. The loser reopens and reads what the winner
//! wrote. Never a silent last-writer-wins.
//!
//! # A gate that says when it did not run
//!
//! This workspace's gate reaches no network. The tests here run when `ENTITY_POSTGRES_URL` names a
//! server — each in a schema of its own — and the gate's `postgres-check` step prints
//! `postgres-check: skipped, ENTITY_POSTGRES_URL unset` when it does not, so a green gate cannot
//! read as a tested backend. CI sets it, against a service container.

use aep_backend_entity::EntityBackend;
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
use aep_domain::entity::{EntityId, EntityLocator, EntityRef, EntityType};
use entity_postgres::PostgresStore;

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
        let durable = PostgresStore::connect(url).map_err(|error| CommandError::Conflict {
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
