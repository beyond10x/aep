//! The interaction contract over a SQLite file.
//!
//! **P4**, and the first backend in this workspace that is a database. One file, no server, and the
//! obvious next durability step after the markdown store — which is the plan's own store and shaped
//! like a plan. This one is shaped like nothing in particular, which is the point: it holds whatever
//! the contract holds, including entities a conformance suite invents.
//!
//! # The contract logic is not written twice
//!
//! Every command is handed to [`MemoryBackend`], and this type adds durability around it — the same
//! arrangement `aep-backend-markdown` uses, for the same reason. Idempotency, revision conflicts,
//! "a refusal still leaves an audit record", "nothing is ever physically deleted": each is a
//! decision whose wrong version looks right, and two implementations of them drift in exactly the
//! ways a suite run months apart discovers.
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
//!
//! # The window, stated
//!
//! A command is applied and **then** written, because what must be written is the result of
//! applying it. If the write fails, this backend latches and every later call refuses rather than
//! answering from memory about a database that no longer agrees. Closing that properly needs a
//! durable intent log, which is `P6`.

use std::sync::Mutex;

use aep_backend_memory::MemoryBackend;
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
use aep_domain::node::Node;

/// The entity type every contract entity is stored under in the database.
///
/// One type, because the contract's entity *types* are data — `design`, `story`, whatever an
/// adopter declares — and encoding them as `entity-core` definitions would make adding a type a
/// schema migration. The contract's own type lives in the row's body.
const STORED_AS: &str = "aep.entity";

/// The contract, over a SQLite file.
pub struct SqliteBackend {
    inner: MemoryBackend,
    durable: Mutex<entity_sqlite::SqliteStore>,
    latched: Mutex<Option<String>>,
}

impl std::fmt::Debug for SqliteBackend {
    /// Hand-written because `SqliteStore` holds a connection and does not derive `Debug`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqliteBackend")
            .field("entities", &self.inner.len())
            .field("latched", &self.latched())
            .finish_non_exhaustive()
    }
}

impl SqliteBackend {
    /// Opens the database at `path`, hydrating the contract from what it holds.
    ///
    /// # Errors
    ///
    /// If the database cannot be opened or read.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, CommandError> {
        let durable =
            entity_sqlite::SqliteStore::open(path).map_err(|error| CommandError::Conflict {
                reason: format!("the database could not be opened: {error}"),
            })?;
        Ok(Self {
            inner: MemoryBackend::new(),
            durable: Mutex::new(durable),
            latched: Mutex::new(None),
        })
    }

    /// An in-memory database, for a test that wants the same code path without a file.
    ///
    /// # Errors
    ///
    /// If the database cannot be created.
    pub fn in_memory() -> Result<Self, CommandError> {
        let durable =
            entity_sqlite::SqliteStore::in_memory().map_err(|error| CommandError::Conflict {
                reason: format!("the database could not be created: {error}"),
            })?;
        Ok(Self {
            inner: MemoryBackend::new(),
            durable: Mutex::new(durable),
            latched: Mutex::new(None),
        })
    }

    /// The fault that made this backend untrustworthy, if one has happened.
    pub fn latched(&self) -> Option<String> {
        self.latched
            .lock()
            .expect("the latch is not poisoned")
            .clone()
    }

    /// How many entities the contract holds.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` when it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn check_latch(&self) -> Result<(), CommandError> {
        match self.latched() {
            None => Ok(()),
            Some(detail) => Err(CommandError::Conflict {
                reason: format!(
                    "this backend is not answering: a write failed and the database no longer \
                     matches what it holds — {detail}. Reopen it once the cause is fixed."
                ),
            }),
        }
    }

    fn latch(&self, detail: impl AsRef<str>) -> CommandError {
        let detail = detail.as_ref();
        *self.latched.lock().expect("the latch is not poisoned") = Some(detail.to_owned());
        CommandError::Conflict {
            reason: format!("the contract moved in memory but not in the database: {detail}"),
        }
    }

    /// Writes every entity a command touched into the database.
    fn persist(&self, result: &CommandResult) -> Result<(), CommandError> {
        // A replay changed nothing, so there is nothing to write. Writing anyway would push the
        // same revision at a store that already holds it, which its own optimistic check would
        // rightly refuse — and this backend would latch over a command that did no harm.
        if result.outcome == aep_contract::command::CommandOutcome::Replayed {
            return Ok(());
        }
        for reference in &result.affected {
            let envelope = block_on(self.inner.get(
                &EntityRef::new(reference.id.clone()),
                QueryConsistency::Current,
            ))
            .map_err(|error| self.latch(format!("the entity could not be read back: {error}")))?;

            let instance = entity_core::EntityInstance {
                entity: STORED_AS.to_owned(),
                version: 1,
                id: reference.id.to_string(),
                lifecycle_state: status_of(&envelope.data),
                revision: reference.revision.get(),
                fields: as_fields(&envelope.data),
            };
            let decision = entity_core::Decision {
                instance,
                events: Vec::new(),
            };

            // The expectation is **read**, not assumed. Deriving it from the contract's revision
            // ("it must be at mine minus one") assumes every command bumps by exactly one and that
            // nothing else ever writes — and the first assumption is already false for a command
            // that touches an entity twice. Reading it keeps the optimistic check honest: what it
            // still catches is the database moving underneath this process between the read and the
            // write, which is what `Expect` is for.
            let mut durable = self.durable.lock().expect("the database is not poisoned");
            let held =
                entity_store::StateProvider::load(&*durable, STORED_AS, &reference.id.to_string())
                    .map_err(|error| self.latch(error.to_string()))?;
            let expect = held.map_or(entity_store::Expect::Absent, |held| {
                entity_store::Expect::Revision(held.revision)
            });
            entity_store::Store::commit(&mut *durable, &decision, expect)
                .map_err(|error| self.latch(error.to_string()))?;
        }
        Ok(())
    }
}

/// The contract's status key, as `aep-backend-memory` spells it.
fn status_of(body: &Node) -> String {
    match body {
        Node::Map(fields) => match fields.get("status") {
            Some(Node::Text(status)) => status.clone(),
            _ => "unknown".to_owned(),
        },
        _ => "unknown".to_owned(),
    }
}

/// The entity body, as the durable store's field map.
fn as_fields(body: &Node) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::to_value(body) {
        Ok(serde_json::Value::Object(map)) => map,
        Ok(other) => {
            let mut map = serde_json::Map::new();
            map.insert("body".to_owned(), other);
            map
        }
        Err(_) => serde_json::Map::new(),
    }
}

impl CommandService for SqliteBackend {
    type Command = Command;

    // The `async` belongs to the contract. This body completes without awaiting: SQLite through
    // `rusqlite` is synchronous, and making it otherwise would pull a runtime into a crate that
    // needs none.
    #[allow(clippy::unused_async_trait_impl)]
    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        self.check_latch()?;
        let result = block_on(self.inner.execute(envelope))?;
        self.persist(&result)?;
        Ok(result)
    }
}

impl QueryService for SqliteBackend {
    type AuditRecord = AuditRecord;

    #[allow(clippy::unused_async_trait_impl)]
    async fn get(
        &self,
        reference: &EntityRef,
        consistency: QueryConsistency,
    ) -> Result<EntityEnvelope, QueryError> {
        block_on(self.inner.get(reference, consistency))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn resolve(&self, locator: &EntityLocator) -> Result<EntityId, QueryError> {
        block_on(self.inner.resolve(locator))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn query(&self, query: &EntityQuery) -> Result<Page<EntityEnvelope>, QueryError> {
        block_on(self.inner.query(query))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn relations(&self, query: &RelationQuery) -> Result<Page<Relation>, QueryError> {
        block_on(self.inner.relations(query))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn history(&self, reference: &EntityRef) -> Result<Vec<RevisionRecord>, QueryError> {
        block_on(self.inner.history(reference))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn audit(&self, query: &AuditQuery) -> Result<Page<Self::AuditRecord>, QueryError> {
        block_on(self.inner.audit(query))
    }

    #[allow(clippy::unused_async_trait_impl)]
    async fn describe_type(&self, entity_type: &EntityType) -> Result<TypeDescriptor, QueryError> {
        block_on(self.inner.describe_type(entity_type))
    }
}
