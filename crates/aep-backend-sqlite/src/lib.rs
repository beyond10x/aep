//! The interaction contract over a SQLite file.
//!
//! **P4**, and the first backend in this workspace that is a database. One file, no server, and the
//! obvious next durability step after the markdown store — which is the plan's own store and shaped
//! like a plan. This one is shaped like nothing in particular, which is the point: it holds whatever
//! the contract holds, including entities a conformance suite invents.
//!
//! # This crate is a type, not a second adapter
//!
//! [`SqliteBackend`] is [`EntityBackend`] over `entity_sqlite::SqliteStore` — wave F, story 2 of
//! `docs/plan/store-waves-f-g-h.md` extracted everything that was not SQLite-specific into
//! `aep-backend-entity`, and nothing in the old implementation was except the two constructors.
//! The contract logic lives once in `aep-backend-memory`; the durability logic — apply, then commit
//! with a **read** expectation, latch on failure, latch covers reads — lives once in the adapter;
//! the sixteen suites and the faulty-backend guard run against the adapter over this store in
//! `aep-backend-entity`'s own tests. What this crate keeps is what only SQLite has: a path, an
//! in-memory mode, and the tests that open the file a second time to see what landed.
//!
//! # Why `entity-sqlite` and not a second hand-written store
//!
//! `entity-runtime`'s store writes an instance and its events inside one transaction — one `BEGIN`,
//! both writes, one `COMMIT` — with a busy timeout so a second writer waits rather than being told
//! the system is broken. It has a conformance suite of its own and a test that tears a write. A
//! second transactional store written here would be building, badly, the thing next door that is
//! already tested against the case that matters.
//!
//! The dependency arrow is the one `atlas/architecture/adr/0002` already points: this workspace
//! takes from `entity-runtime` and gives nothing back.

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
use entity_sqlite::SqliteStore;

/// The entity type every contract entity is stored under in the database.
///
/// Re-exported from the adapter so a reader of this crate, and its tests, name the same string.
pub use aep_backend_entity::STORED_AS;

/// The contract, over a SQLite file.
///
/// A newtype rather than a type alias because the constructors are inherent methods, and an
/// inherent method can only be written in the crate that owns the type. Every trait method forwards.
#[derive(Debug)]
pub struct SqliteBackend(EntityBackend<SqliteStore>);

impl SqliteBackend {
    /// Opens the database at `path`.
    ///
    /// Nothing is read back yet — hydration is F4 (`story:sqlite-hydrates-on-open`). Until then a
    /// populated file is protected by the adapter's foreign-row refusal, not by hydration.
    ///
    /// # Errors
    ///
    /// If the database cannot be opened.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, CommandError> {
        let durable = SqliteStore::open(path).map_err(|error| CommandError::Conflict {
            reason: format!("the database could not be opened: {error}"),
        })?;
        Ok(Self(EntityBackend::over(durable)))
    }

    /// An in-memory database, for a test that wants the same code path without a file.
    ///
    /// # Errors
    ///
    /// If the database cannot be created.
    pub fn in_memory() -> Result<Self, CommandError> {
        let durable = SqliteStore::in_memory().map_err(|error| CommandError::Conflict {
            reason: format!("the database could not be created: {error}"),
        })?;
        Ok(Self(EntityBackend::over(durable)))
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
    pub fn as_entity_backend(&self) -> &EntityBackend<SqliteStore> {
        &self.0
    }
}

impl CommandService for SqliteBackend {
    type Command = Command;

    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        self.0.execute(envelope).await
    }
}

impl QueryService for SqliteBackend {
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
