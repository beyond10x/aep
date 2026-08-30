//! The interaction contract over any `entity-store` provider.
//!
//! One adapter. `aep-backend-sqlite` is [`EntityBackend`] over `entity_sqlite::SqliteStore`; the
//! next durable backend is [`EntityBackend`] over the next provider `entity-runtime` ships, and it
//! passes the sixteen conformance suites because this adapter already does and the provider already
//! passed the runtime's. Wave F of `docs/plan/store-waves-f-g-h.md`: story 2 extracted this from
//! `SqliteBackend`, story 3 made events cross the seam, story 4 made a populated store readable.
//!
//! # The contract logic is not written twice
//!
//! Every command is handed to [`MemoryBackend`], and this type adds durability around it — the same
//! arrangement `aep-backend-markdown` uses, for the same reason. Idempotency, revision conflicts,
//! "a refusal still leaves an audit record", "nothing is ever physically deleted": each is a
//! decision whose wrong version looks right, and two implementations of them drift in exactly the
//! ways a suite run months apart discovers.
//!
//! # Why a provider from `entity-runtime` and not a store written here
//!
//! `entity-runtime`'s [`Store`] writes an instance and its events as one step — its `SqliteStore`
//! inside one transaction — with a conformance suite of its own and a test that tears a write. A
//! second store written here would be building, badly, the thing next door that is already tested
//! against the case that matters. The dependency arrow is the one `atlas/architecture/adr/0002`
//! already points: this workspace takes from `entity-runtime` and gives nothing back.
//!
//! # The projection seam
//!
//! What a provider holds for a contract entity is a [`Projection`]'s decision. The default,
//! [`Identity`], keeps every record the contract has under four entity types (below) and hydrates
//! from them — the shape a SQLite or Postgres plan takes. `aep-backend-markdown`'s projection keeps
//! the plan's own shape — one document per artifact, relations in frontmatter, the history in the
//! journal — and seeds from the documents on open. The adapter is the same type over both: apply in
//! [`MemoryBackend`], ask the projection where the result lands, diff against what the provider
//! held, seal the event, and submit the complete command through
//! [`AtomicBatchStore::commit_batch`].
//!
//! # What the [`Identity`] projection holds, and how a second process gets it back
//!
//! Four entity types, all the contract's records:
//!
//! | type | one instance per | fields |
//! |---|---|---|
//! | [`STORED_AS`] `aep.entity` | contract entity | the body, flat, plus [`METADATA_KEY`] — the `EntityMetadata` and the archived flag |
//! | [`RELATIONS_AS`] `aep.relation` | relation ever created | the `Relation`, and `removed` once it was |
//! | [`AUDIT_AS`] `aep.audit` | audit record, accepted or refused | the `AuditRecord` |
//! | [`APPLIED_AS`] `aep.applied` | idempotency key | the command id and the result a replay returns |
//!
//! [`EntityBackend::over`] **hydrates**: it asks the provider what it holds
//! (`StateProvider::ids`, `entity-runtime` 0.10.0) and installs every record in [`MemoryBackend`]
//! with its **stored** identity, so a second process sees every entity, relation and audit record
//! the first wrote, with the same ids, and a replayed command is recognised across processes.
//! History is rebuilt from the stored events. A store this adapter cannot read back wholly refuses
//! to open — naming the row — rather than answering about part of it, because a backend that
//! hydrated nine of ten entities and said nothing is the failure nobody finds.
//!
//! Identities are the store's: this adapter derives nothing from them (invariant 13), and
//! `MemoryBackend` mints new ones past whatever is already held rather than parsing the old ones.
//!
//! # Events cross the seam
//!
//! Every accepted command becomes one [`entity_core::DomainEvent`] per affected entity, written
//! with the instance in the same `commit` — so the provider's guarantee that *a state cannot move
//! without the event that explains it* (runtime R-83) protects a real event and not an empty list.
//! The event's type is the command's type name (`aep.status.move/v1` and so on), `from_state` and
//! `to_state` are the entity's `status` before and after, and `changed` is the fields the command
//! wrote. A refused command writes no event (R-84 across the seam) — its audit record still lands;
//! a replayed one writes nothing, as it writes no instance.
//!
//! **Who and when travel in the event's `payload`.** The runtime seals an event with a
//! [`Recording`] into an [`entity_store::Envelope`], but its providers store the bare `DomainEvent`
//! a `Decision` carries (`entity-runtime` `story:the-store-keeps-the-envelope`). So the seal's
//! fields are written into `payload`, with the contract's own instant (`at`, epoch milliseconds —
//! the seal's `recorded_at` is to the second) and the executor. That is what a rebuilt history is
//! made of. **What the command was decided on is the event's `args`** (`entity-runtime` 0.11.0,
//! R-110): the command's payload, verbatim — a `MoveStatus`'s `decided_on` account included.
//!
//! The record types carry no events of their own. R-83 is about an instance whose *state* moves;
//! a relation, an audit record and an idempotency entry are records of something that already
//! happened, and the event explaining them is the entity's.
//!
//! # One command, one publish
//!
//! A command is applied to detached memory and projection state. Every placement, audit record and
//! idempotency record becomes one ordered provider batch whose expectations come from the local
//! pre-command view. Only a successful batch publishes the detached state. A conflict or provider
//! failure therefore leaves both durable and local state unchanged; there is no stale in-memory
//! prefix to latch.

pub mod kernel;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Mutex;

use aep_backend_memory::store::{AppliedCommand, StoredEntity};
use aep_backend_memory::MemoryBackend;
use aep_contract::command::{CommandEnvelope, CommandOutcome, CommandResult, CommandService};
use aep_contract::error::{CommandError, QueryError};
use aep_contract::query::{
    AuditQuery, EntityEnvelope, EntityQuery, Page, QueryService, Relation, RelationQuery,
    RevisionRecord,
};
use aep_contract::registry::TypeDescriptor;
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::audit::{AuditKind, AuditRecord, ChangeRecord};
use aep_domain::command::Command;
use aep_domain::entity::{
    ActorRef, EntityId, EntityLocator, EntityMetadata, EntityRef, EntityRevision, EntityType,
};
use aep_domain::ids::{AuditId, EventId, IdempotencyKey, RelationId};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_store::{
    AtomicBatchStore, AtomicCommit, Envelope, Expect, Recording, Store, StoreError,
};
use serde_json::{json, Map, Value};

/// The entity type every contract entity is stored under in the provider.
///
/// One type, because the contract's entity *types* are data — `design`, `story`, whatever an
/// adopter declares — and encoding them as `entity-core` definitions would make adding a type a
/// schema migration. The contract's own type lives in the instance's metadata.
pub const STORED_AS: &str = "aep.entity";

/// The entity type every relation is stored under.
pub const RELATIONS_AS: &str = "aep.relation";

/// The entity type every audit record is stored under.
pub const AUDIT_AS: &str = "aep.audit";

/// The entity type every applied command — the idempotency memory — is stored under.
pub const APPLIED_AS: &str = "aep.applied";

/// The reserved key in an `aep.entity` instance's fields that carries the contract's metadata.
///
/// `$`-prefixed so no frontmatter key an adopter writes collides with it; `entity-core` reads `$`
/// only in rule templates, never in a field name.
pub const METADATA_KEY: &str = "$aep";

/// The lifecycle state every record instance is stored in.
const RECORDED: &str = "recorded";

/// The lifecycle state a relation instance moves to when the relation is removed.
const REMOVED: &str = "removed";

/// What the shell knows about a command and the kernel does not, taken before the envelope is
/// handed to the contract and consumed.
///
/// The four `Recording` values are the ones `entity-runtime` R-86 says an event log needs and the
/// kernel must not invent; every one of them is on the command envelope already.
struct Provenance {
    /// The command's type name, which becomes the event type.
    command_type: String,
    /// When, which flow, what caused it, who.
    recording: Recording,
    /// The contract's own instant, to the millisecond, for a rebuilt `RevisionRecord`.
    at: Timestamp,
    /// What ran, when that differs from who authorised it.
    executor: Option<ActorRef>,
    /// The idempotency key, so the applied-command record can be found and persisted.
    key: IdempotencyKey,
    /// The command's payload — what the operation was decided on — which becomes the event's
    /// `args` (`entity-runtime` R-110). For a `MoveStatus` that is the target, the status moved
    /// to, the revision expected and the `decided_on` account; for an update, the fields written.
    args: Map<String, Value>,
    /// The provider coordinate and revision the command's local pre-command view held.
    optimistic: Option<((String, String), Expect)>,
}

impl Provenance {
    fn of(envelope: &CommandEnvelope<Command>) -> Self {
        let args = match serde_json::to_value(&envelope.payload) {
            Ok(Value::Object(map)) => map,
            Ok(other) => object(json!({ "payload": other })),
            // Serialising a command does not fail in practice; if it ever does, the failure is
            // written down rather than swallowed — `.ok()` on this value is how a provenance was
            // silently lost once already (`story:journal-backed-store`).
            Err(error) => object(json!({ "unserialisable": error.to_string() })),
        };
        Self {
            command_type: envelope.command_type.clone(),
            recording: Recording {
                recorded_at: envelope.context.issued_at.iso_8601(),
                correlation: envelope.context.correlation_id.to_string(),
                causation: envelope.command_id.to_string(),
                actor: Some(envelope.context.actor.to_string()),
            },
            at: envelope.context.issued_at,
            executor: envelope.context.executor.clone(),
            key: envelope.context.idempotency_key.clone(),
            args,
            optimistic: None,
        }
    }

    /// One event for one placement, sealed, with the seal written into its payload.
    fn event(
        &self,
        placement: &Placement,
        from_state: Option<String>,
        changed: Map<String, Value>,
    ) -> DomainEvent {
        let mut event = DomainEvent {
            entity: placement.entity.clone(),
            version: 1,
            id: placement.id.clone(),
            revision: placement.revision,
            event_type: self.command_type.clone(),
            from_state,
            to_state: placement.state.clone(),
            changed,
            args: self.args.clone(),
            payload: Value::Null,
        };
        // Sealed by the runtime's own `Recording`, so the event id is the derived
        // `<entity>:<id>@<revision>#<index>~<args>` every other shell would derive — and then the
        // seal is written into the event, because the seal is the only part of an envelope the
        // providers do not keep (see the module doc).
        let sealed = self.recording.seal(std::slice::from_ref(&event));
        let mut payload = recorded(&sealed[0], self.at, self.executor.as_ref());
        if let (Value::Object(payload), Some(note)) = (&mut payload, &placement.note) {
            payload.insert("change".to_owned(), note.clone());
        }
        event.payload = payload;
        event
    }
}

/// The sealed envelope's fields and the contract's instant and executor, as the event's payload.
///
/// `executor` is always written — `null` when absent — because an absent key and *nothing else
/// ran* would otherwise read alike, and only one of them is a claim. What the command was decided
/// on is not here: that is the event's own `args`.
fn recorded(sealed: &Envelope<DomainEvent>, at: Timestamp, executor: Option<&ActorRef>) -> Value {
    json!({
        "event_id": sealed.event_id,
        "recorded_at": sealed.recorded_at,
        "at": at.epoch_millis(),
        "correlation": sealed.correlation,
        "causation": sealed.causation,
        "actor": sealed.actor,
        "executor": executor.map(ToString::to_string),
    })
}

/// The fields `after` holds that `before` did not, or held differently.
///
/// A field `before` had and `after` lacks is not representable: the contract's `UpdateEntity`
/// merges and never removes, so the case does not arise today, and an event format that could say
/// "removed" is the runtime's to add (`DomainEvent::changed` is a map of values).
fn changed_between(before: &Map<String, Value>, after: &Map<String, Value>) -> Map<String, Value> {
    after
        .iter()
        .filter(|(name, value)| before.get(*name) != Some(*value))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// What the contract holds beside an entity's body, carried under [`METADATA_KEY`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredMetadata {
    metadata: EntityMetadata,
    archived: bool,
}

/// What the memory store held before a command, so what the command added can be found afterwards
/// without teaching this adapter which commands create relations or audit records.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Every relation held before.
    pub relations: BTreeSet<RelationId>,
    /// How many audit records were held before.
    pub audit: usize,
    /// Whether the command's idempotency key was already remembered.
    pub applied: bool,
}

/// One instance a command's result lands as, in the provider's own coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The provider's entity type — `aep.entity` for [`Identity`], the kind for a plan.
    pub entity: String,
    /// The provider's identity — the contract's id for [`Identity`], the name for a plan.
    pub id: String,
    /// The lifecycle state written.
    pub state: String,
    /// The revision written — and the event's.
    pub revision: u64,
    /// The instance's fields, whole.
    pub fields: Map<String, Value>,
    /// What the projection wants recorded about this write, carried in the event's payload under
    /// `change` — a plan's journal spells a move, a relation and a recorded observation differently.
    pub note: Option<Value>,
    /// Write even when the fields and state did not change: an observation *about* a document
    /// appends an event at the document's current revision.
    pub always: bool,
}

/// One record written beside the placements: a relation, an audit record, an applied command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The provider's entity type.
    pub entity: String,
    /// The provider's identity.
    pub id: String,
    /// The lifecycle state written.
    pub state: String,
    /// The fields, or `None` for "what is held, marked removed" — the one update a record gets.
    pub fields: Option<Map<String, Value>>,
}

/// How a contract entity lands in a provider, and how memory is filled from one on open.
///
/// The adapter owns the contract logic, the latch, the seal and the commit; the projection owns
/// the shape. [`Identity`] is the shape of a store that holds the contract's records as they are.
/// A plan-shaped projection lives beside the plan's own provider, in `aep-backend-markdown`.
pub trait Projection<S: Store> {
    /// Fills `inner` from `store` — a second process's view of what the first wrote.
    ///
    /// # Errors
    ///
    /// If the store cannot be read, or holds something this projection cannot read back — refused
    /// rather than skipped, naming the row.
    fn hydrate(&mut self, store: &S, inner: &MemoryBackend) -> Result<(), CommandError>;

    /// Notes what a command is before the contract consumes it.
    ///
    /// # Errors
    ///
    /// If the command carries something the projection cannot write — a move's account that is not
    /// a provenance, say.
    fn before(&mut self, envelope: &CommandEnvelope<Command>) -> Result<(), CommandError>;

    /// Where an accepted command's result lands: one placement per instance to write.
    ///
    /// # Errors
    ///
    /// A refusal the projection makes on its own account — a status the plan's ladder does not
    /// permit — is returned as it is, not latched: nothing durable disagrees yet.
    fn placements(
        &mut self,
        store: &S,
        inner: &MemoryBackend,
        result: &CommandResult,
    ) -> Result<Vec<Placement>, CommandError>;

    /// The provider's coordinates for an entity — where its instance and its events are — when
    /// the store has a shape for it. `None` for an entity this projection does not write (a
    /// conformance suite's, in a plan), whose history is then the in-memory one.
    fn coordinates(&self, inner: &MemoryBackend, id: &EntityId) -> Option<(String, String)>;

    /// The ladders this store's kinds are held to, when it holds any — what `describe_type` renders
    /// and what a move is decided against, from one object.
    fn lifecycles(&self) -> Option<&aep_domain::artifact::LifecycleRegistry>;

    /// The records to write beside the placements, from what changed since `before`.
    ///
    /// # Errors
    ///
    /// As for [`Projection::placements`].
    fn records(
        &mut self,
        store: &S,
        inner: &MemoryBackend,
        before: &Snapshot,
        key: &IdempotencyKey,
    ) -> Result<Vec<Record>, CommandError>;
}

/// The projection that keeps the contract's records as they are, under four entity types.
///
/// Every contract entity is an `aep.entity` instance under its own id, with its body flat and its
/// metadata under [`METADATA_KEY`]; relations, audit records and applied commands are instances of
/// their own types; opening hydrates all four. The shape a SQLite or Postgres plan takes. With
/// [`Identity::with_lifecycles`], `describe_type` reports the ladder of every planning kind.
///
/// **An observation is an event on the entity it is about.** A relation is a record of its own and
/// evidence changes nothing, so the contract names no affected entity for either — but the entity
/// the edge starts at, and the entity the evidence is about, each get an event at their current
/// revision saying so. That is what lets a history read from the entity's log say what a plan's
/// journal says (`protocol artifact history` over SQLite), and what lets a second process count the
/// evidence on hand from the log alone.
#[derive(Debug, Clone, Default)]
pub struct Identity {
    lifecycles: Option<aep_domain::artifact::LifecycleRegistry>,
    /// The entity the command in flight is an observation about, noted in `before` and written in
    /// `placements`: a relation's source, evidence's target.
    observing: Option<EntityId>,
}

impl Identity {
    /// The identity shape, reporting `lifecycles` through `describe_type`.
    #[must_use]
    pub fn with_lifecycles(lifecycles: aep_domain::artifact::LifecycleRegistry) -> Self {
        Self {
            lifecycles: Some(lifecycles),
            observing: None,
        }
    }
}

/// One placement for `id` as the contract holds it now — an observation: same state, same revision,
/// the fields as they stand, written regardless.
fn observation_of(inner: &MemoryBackend, id: &EntityId) -> Result<Placement, CommandError> {
    let stored = inner
        .with_store(|store| store.entity(id).cloned())
        .ok_or_else(|| CommandError::Conflict {
            reason: format!("the entity `{id}` could not be read back after the command"),
        })?;
    Ok(Placement {
        entity: STORED_AS.to_owned(),
        id: id.to_string(),
        state: status_of(&stored.data),
        revision: stored.metadata.revision.get(),
        fields: pack(&stored),
        note: None,
        always: true,
    })
}

impl<S: Store> Projection<S> for Identity {
    fn hydrate(&mut self, store: &S, inner: &MemoryBackend) -> Result<(), CommandError> {
        hydrate(store, inner)
    }

    fn before(&mut self, envelope: &CommandEnvelope<Command>) -> Result<(), CommandError> {
        self.observing = match &envelope.payload {
            Command::CreateRelation(create) => Some(create.source.id.clone()),
            Command::RecordEvidence(record) => Some(record.target.id.clone()),
            _ => None,
        };
        Ok(())
    }

    fn coordinates(&self, _inner: &MemoryBackend, id: &EntityId) -> Option<(String, String)> {
        Some((STORED_AS.to_owned(), id.to_string()))
    }

    fn lifecycles(&self) -> Option<&aep_domain::artifact::LifecycleRegistry> {
        self.lifecycles.as_ref()
    }

    fn placements(
        &mut self,
        _store: &S,
        inner: &MemoryBackend,
        result: &CommandResult,
    ) -> Result<Vec<Placement>, CommandError> {
        let mut placements = result
            .affected
            .iter()
            .map(|reference| observation_of(inner, &reference.id))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(observed) = self.observing.take() {
            if !placements
                .iter()
                .any(|placement| placement.id == observed.to_string())
            {
                placements.push(observation_of(inner, &observed)?);
            }
        }
        Ok(placements)
    }

    fn records(
        &mut self,
        _store: &S,
        inner: &MemoryBackend,
        before: &Snapshot,
        key: &IdempotencyKey,
    ) -> Result<Vec<Record>, CommandError> {
        let (created, removed, audit, applied) = inner.with_store(|store| {
            let now: BTreeMap<RelationId, Relation> = store
                .relations()
                .map(|relation| (relation.id.clone(), relation.clone()))
                .collect();
            let created: Vec<Relation> = now
                .values()
                .filter(|relation| !before.relations.contains(&relation.id))
                .cloned()
                .collect();
            let removed: Vec<RelationId> = before
                .relations
                .iter()
                .filter(|id| !now.contains_key(*id))
                .cloned()
                .collect();
            let audit: Vec<AuditRecord> = store.audit()[before.audit..].to_vec();
            let applied = if before.applied {
                None
            } else {
                store.applied(key).cloned()
            };
            (created, removed, audit, applied)
        });
        let mut records = Vec::new();
        for relation in created {
            records.push(Record {
                entity: RELATIONS_AS.to_owned(),
                id: relation.id.to_string(),
                state: RECORDED.to_owned(),
                fields: Some(object(json!({ "relation": relation, "removed": false }))),
            });
        }
        for id in removed {
            records.push(Record {
                entity: RELATIONS_AS.to_owned(),
                id: id.to_string(),
                state: REMOVED.to_owned(),
                fields: None,
            });
        }
        for record in audit {
            records.push(Record {
                entity: AUDIT_AS.to_owned(),
                id: record.audit_id.to_string(),
                state: RECORDED.to_owned(),
                fields: Some(object(json!({ "record": record }))),
            });
        }
        if let Some(applied) = applied {
            records.push(Record {
                entity: APPLIED_AS.to_owned(),
                id: key.to_string(),
                state: RECORDED.to_owned(),
                fields: Some(object(
                    json!({ "command_id": applied.command_id, "result": applied.result }),
                )),
            });
        }
        Ok(records)
    }
}

/// The contract, over a provider `S`, shaped by a projection `P`.
pub struct EntityBackend<S, P = Identity> {
    inner: MemoryBackend,
    durable: Mutex<S>,
    projection: Mutex<P>,
    latched: Mutex<Option<String>>,
}

impl<S, P> fmt::Debug for EntityBackend<S, P> {
    /// Hand-written because a provider may hold a connection and not derive `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EntityBackend")
            .field("entities", &self.inner.len())
            .field("latched", &self.latched())
            .finish_non_exhaustive()
    }
}

impl<S, P> EntityBackend<S, P> {
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

    /// The backend the contract logic lives in, for a caller that needs to look.
    pub const fn inner(&self) -> &MemoryBackend {
        &self.inner
    }

    /// Runs `read` against the provider.
    ///
    /// For a test that wants to see what landed without opening a second handle — which a
    /// `MemoryStore` cannot offer. Read-only by signature; every write goes through
    /// [`CommandService::execute`], and that is invariant 14.
    ///
    /// # Panics
    ///
    /// If the provider's lock is poisoned, which means a previous call panicked while holding it.
    pub fn with_store<T>(&self, read: impl FnOnce(&S) -> T) -> T {
        let durable = self.durable.lock().expect("the provider is not poisoned");
        read(&durable)
    }

    /// Runs `read` against the projection, for what only it knows — a plan's unprojected entities.
    ///
    /// # Panics
    ///
    /// If the projection's lock is poisoned.
    pub fn with_projection<T>(&self, read: impl FnOnce(&P) -> T) -> T {
        let projection = self
            .projection
            .lock()
            .expect("the projection is not poisoned");
        read(&projection)
    }
}

/// A store failure while opening, as the contract spells it.
fn could_not_read(error: &StoreError) -> CommandError {
    CommandError::Conflict {
        reason: format!("the store could not be read: {error}"),
    }
}

/// A row this adapter cannot make sense of. Refused by name: see the module doc.
fn unreadable(entity: &str, id: &str, detail: impl fmt::Display) -> CommandError {
    CommandError::Conflict {
        reason: format!(
            "the store holds a `{entity}` row `{id}` this backend cannot read back ({detail}), so it \
             refuses to open rather than answer about part of what is there"
        ),
    }
}

impl<S: Store> EntityBackend<S, Identity> {
    /// The contract over `store`, holding everything the store holds, in the [`Identity`] shape.
    ///
    /// # Errors
    ///
    /// As [`EntityBackend::with_projection`].
    pub fn over(store: S) -> Result<Self, CommandError> {
        Self::shaped(store, Identity::default())
    }
}

impl<S: Store, P: Projection<S>> EntityBackend<S, P> {
    /// The event log of one entity, at the coordinates the projection gave it.
    ///
    /// What `protocol artifact history` reads over a plan without a journal: the same events the
    /// adapter rebuilds a history from, whichever shape the store holds them in — `aep.entity`
    /// under the contract's id for [`Identity`], the kind and the name for a plan. Empty for an
    /// entity the projection does not write.
    ///
    /// # Errors
    ///
    /// If the provider cannot read its log.
    pub fn events_of(&self, id: &EntityId) -> Result<Vec<DomainEvent>, entity_store::StoreError> {
        let Some((entity, key)) =
            self.with_projection(|projection| projection.coordinates(&self.inner, id))
        else {
            return Ok(Vec::new());
        };
        self.with_store(|store| store.events(&entity, &key))
    }

    /// The contract over `store`, shaped by `projection`, holding everything the store holds.
    ///
    /// Hydrates through the projection: the [`Identity`] shape installs every record under its
    /// stored identity; a plan's shape seeds from its documents. A second process opening the same
    /// store sees what the first wrote.
    ///
    /// # Errors
    ///
    /// If the store cannot be read, or holds something the projection cannot read back — refused
    /// rather than skipped, naming the row.
    pub fn shaped(store: S, mut projection: P) -> Result<Self, CommandError> {
        let inner = MemoryBackend::new();
        projection.hydrate(&store, &inner)?;
        Ok(Self {
            inner,
            durable: Mutex::new(store),
            projection: Mutex::new(projection),
            latched: Mutex::new(None),
        })
    }

    /// Refuses a read when a write has left memory and the store disagreeing.
    fn refuse_when_latched(&self) -> Result<(), QueryError> {
        match self.latched() {
            None => Ok(()),
            Some(detail) => Err(QueryError::Unavailable {
                reason: format!(
                    "this backend is not answering: a write failed and what it holds no longer \
                     matches what is stored — {detail}. Reopen it once the cause is fixed."
                ),
            }),
        }
    }

    fn check_latch(&self) -> Result<(), CommandError> {
        match self.latched() {
            None => Ok(()),
            Some(detail) => Err(CommandError::Conflict {
                reason: format!(
                    "this backend is not answering: a write failed and the store no longer matches \
                     what it holds — {detail}. Reopen it once the cause is fixed."
                ),
            }),
        }
    }

    fn snapshot(&self, key: &IdempotencyKey) -> Snapshot {
        self.inner.with_store(|store| Snapshot {
            relations: store.relations().map(|r| r.id.clone()).collect(),
            audit: store.audit().len(),
            applied: store.applied(key).is_some(),
        })
    }

    /// Builds the one provider transaction for a command from the pre-command durable view.
    fn commits(
        durable: &S,
        placements: &[Placement],
        records: &[Record],
        provenance: &Provenance,
    ) -> Result<Vec<AtomicCommit>, CommandError> {
        let mut held_by_key: BTreeMap<(String, String), Option<EntityInstance>> = BTreeMap::new();
        let mut commits = Vec::new();
        for placement in placements {
            let key = (placement.entity.clone(), placement.id.clone());
            if !held_by_key.contains_key(&key) {
                let loaded = durable
                    .load(&placement.entity, &placement.id)
                    .map_err(|error| provider_error(&error))?;
                held_by_key.insert(key.clone(), loaded.clone());
            }
            let held = held_by_key.get(&key).cloned().flatten();

            // Nothing to write when nothing changed: a command whose result is the document as it
            // already stood must not bump a revision and journal a change that did not happen.
            if !placement.always
                && held.as_ref().is_some_and(|held| {
                    held.fields == placement.fields && held.lifecycle_state == placement.state
                })
            {
                continue;
            }

            // The event is built from what the store held a moment ago and what the contract holds
            // now: `from_state` and `changed` are the difference, and for a creation there is no
            // "before", so `from_state` is `None` and `changed` is every field — which is what the
            // runtime's fold (`entity_core::rehydrate`) reads as a creation event.
            let (from_state, changed) = match &held {
                Some(before) => (
                    Some(before.lifecycle_state.clone()),
                    changed_between(&before.fields, &placement.fields),
                ),
                None => (None, placement.fields.clone()),
            };
            let event = provenance.event(placement, from_state, changed);
            let instance = EntityInstance {
                entity: placement.entity.clone(),
                version: 1,
                id: placement.id.clone(),
                lifecycle_state: placement.state.clone(),
                revision: placement.revision,
                fields: placement.fields.clone(),
            };
            let decision = Decision {
                instance: instance.clone(),
                events: vec![event],
            };

            let expect = provenance
                .optimistic
                .as_ref()
                .filter(|(coordinate, _)| coordinate == &key)
                .map_or_else(
                    || held.map_or(Expect::Absent, |held| Expect::Revision(held.revision)),
                    |(_, expect)| *expect,
                );
            commits.push(AtomicCommit::new(decision, expect));
            held_by_key.insert(key, Some(instance));
        }
        for record in records {
            let key = (record.entity.clone(), record.id.clone());
            if !held_by_key.contains_key(&key) {
                let loaded = durable
                    .load(&record.entity, &record.id)
                    .map_err(|error| provider_error(&error))?;
                held_by_key.insert(key.clone(), loaded);
            }
            let held = held_by_key.get(&key).cloned().flatten();
            let (expect, revision) = match &held {
                Some(held) => (Expect::Revision(held.revision), held.revision + 1),
                None => (Expect::Absent, 1),
            };
            let fields = record.fields.clone().unwrap_or_else(|| {
                let mut fields = held.map(|held| held.fields).unwrap_or_default();
                fields.insert("removed".to_owned(), Value::Bool(true));
                fields
            });
            let instance = EntityInstance {
                entity: record.entity.clone(),
                version: 1,
                id: record.id.clone(),
                lifecycle_state: record.state.clone(),
                revision,
                fields,
            };
            let decision = Decision {
                instance: instance.clone(),
                events: Vec::new(),
            };
            commits.push(AtomicCommit::new(decision, expect));
            held_by_key.insert(key, Some(instance));
        }
        Ok(commits)
    }
}

/// A provider error before candidate state has been published.
fn provider_error(error: &StoreError) -> CommandError {
    CommandError::Conflict {
        reason: format!("the store refused the atomic command: {error}"),
    }
}

/// Reads everything `store` holds into `inner`, with the stored identities.
fn hydrate<S: Store>(store: &S, inner: &MemoryBackend) -> Result<(), CommandError> {
    for id in store.ids(STORED_AS).map_err(|e| could_not_read(&e))? {
        let instance = store
            .load(STORED_AS, &id)
            .map_err(|e| could_not_read(&e))?
            .ok_or_else(|| unreadable(STORED_AS, &id, "listed, and `load` answers absent"))?;
        let stored = unpack(&instance).map_err(|detail| unreadable(STORED_AS, &id, detail))?;
        let events = store
            .events(STORED_AS, &id)
            .map_err(|e| could_not_read(&e))?;
        let history = events
            .iter()
            .map(revision_record)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|detail| unreadable(STORED_AS, &id, detail))?;
        inner.with_store_mut(|memory| {
            let entity_id = stored.metadata.id.clone();
            memory.insert_entity(stored);
            for record in history {
                memory.record_revision(&entity_id, record);
            }
        });
    }

    for id in store.ids(RELATIONS_AS).map_err(|e| could_not_read(&e))? {
        let instance = store
            .load(RELATIONS_AS, &id)
            .map_err(|e| could_not_read(&e))?
            .ok_or_else(|| unreadable(RELATIONS_AS, &id, "listed, and `load` answers absent"))?;
        if instance.fields.get("removed") == Some(&Value::Bool(true)) {
            continue;
        }
        let relation: Relation =
            field(&instance, "relation").map_err(|detail| unreadable(RELATIONS_AS, &id, detail))?;
        inner.with_store_mut(|memory| memory.insert_relation(relation));
    }

    // Audit ids are minted from one zero-padded counter, so sorted is the order they were written.
    for id in store.ids(AUDIT_AS).map_err(|e| could_not_read(&e))? {
        let instance = store
            .load(AUDIT_AS, &id)
            .map_err(|e| could_not_read(&e))?
            .ok_or_else(|| unreadable(AUDIT_AS, &id, "listed, and `load` answers absent"))?;
        let record: AuditRecord =
            field(&instance, "record").map_err(|detail| unreadable(AUDIT_AS, &id, detail))?;
        inner.with_store_mut(|memory| memory.record_audit(record));
    }

    for key in store.ids(APPLIED_AS).map_err(|e| could_not_read(&e))? {
        let instance = store
            .load(APPLIED_AS, &key)
            .map_err(|e| could_not_read(&e))?
            .ok_or_else(|| unreadable(APPLIED_AS, &key, "listed, and `load` answers absent"))?;
        let applied = AppliedCommand {
            command_id: field(&instance, "command_id")
                .map_err(|detail| unreadable(APPLIED_AS, &key, detail))?,
            result: field(&instance, "result")
                .map_err(|detail| unreadable(APPLIED_AS, &key, detail))?,
        };
        let key: IdempotencyKey = key
            .parse()
            .map_err(|error| unreadable(APPLIED_AS, &key, error))?;
        inner.with_store_mut(|memory| memory.remember(&key, applied));
    }
    Ok(())
}

/// One typed field of a record instance.
fn field<T: serde::de::DeserializeOwned>(
    instance: &EntityInstance,
    name: &str,
) -> Result<T, String> {
    let value = instance
        .fields
        .get(name)
        .ok_or_else(|| format!("no `{name}` field"))?;
    serde_json::from_value(value.clone()).map_err(|error| format!("`{name}`: {error}"))
}

/// The provider's fields for a contract entity: its body, flat, plus the metadata under
/// [`METADATA_KEY`].
fn pack(stored: &StoredEntity) -> Map<String, Value> {
    let mut fields = as_fields(&stored.data);
    fields.insert(
        METADATA_KEY.to_owned(),
        serde_json::to_value(StoredMetadata {
            metadata: stored.metadata.clone(),
            archived: stored.archived,
        })
        .unwrap_or_else(|error| json!({ "unserialisable": error.to_string() })),
    );
    fields
}

/// The inverse of [`pack`], refusing an instance whose metadata disagrees with it.
fn unpack(instance: &EntityInstance) -> Result<StoredEntity, String> {
    let mut fields = instance.fields.clone();
    let metadata = fields.remove(METADATA_KEY).ok_or_else(|| {
        format!("no `{METADATA_KEY}` metadata; this row was not written by this backend")
    })?;
    let StoredMetadata { metadata, archived } =
        serde_json::from_value(metadata).map_err(|error| format!("`{METADATA_KEY}`: {error}"))?;
    if metadata.id.to_string() != instance.id {
        return Err(format!(
            "the metadata names `{}` and the row is `{}`",
            metadata.id, instance.id
        ));
    }
    if metadata.revision.get() != instance.revision {
        return Err(format!(
            "the metadata is at revision {} and the row at {}",
            metadata.revision.get(),
            instance.revision
        ));
    }
    let data: Node = serde_json::from_value(Value::Object(fields))
        .map_err(|error| format!("the body does not read as a contract body: {error}"))?;
    if status_of(&data) != instance.lifecycle_state {
        return Err(format!(
            "the body says status `{}` and the row says `{}`",
            status_of(&data),
            instance.lifecycle_state
        ));
    }
    Ok(StoredEntity {
        metadata,
        data,
        archived,
    })
}

/// The history entry a stored event stands for, read from the seal in its payload.
fn revision_record(event: &DomainEvent) -> Result<RevisionRecord, String> {
    let seal = &event.payload;
    let at = seal.get("at").and_then(Value::as_u64).ok_or_else(|| {
        format!(
            "event at revision {} carries no `at` instant",
            event.revision
        )
    })?;
    let actor = seal
        .get("actor")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("event at revision {} carries no `actor`", event.revision))?;
    let executor = match seal.get("executor") {
        Some(Value::String(executor)) => Some(
            ActorRef::parse(executor)
                .map_err(|error| format!("event at revision {}: {error}", event.revision))?,
        ),
        _ => None,
    };
    let command_id = match seal.get("causation").and_then(Value::as_str) {
        Some(causation) => Some(
            causation
                .parse()
                .map_err(|error| format!("event at revision {}: {error}", event.revision))?,
        ),
        None => None,
    };
    Ok(RevisionRecord {
        revision: EntityRevision::new(event.revision)
            .map_err(|error| format!("event at revision {}: {error}", event.revision))?,
        at: Timestamp::from_epoch_millis(at),
        actor: ActorRef::parse(actor)
            .map_err(|error| format!("event at revision {}: {error}", event.revision))?,
        executor,
        command_id,
        audit_id: None,
    })
}

/// The history a log stands for: one record per revision, the first event that reached it.
///
/// An observation about an entity is an event at the entity's current revision (a plan's evidence
/// record); it is a fact about the entity, not a revision of it, so it does not add a record.
fn history_of(events: &[DomainEvent]) -> Result<Vec<RevisionRecord>, String> {
    let mut by_revision: BTreeMap<u64, RevisionRecord> = BTreeMap::new();
    for event in events {
        if by_revision.contains_key(&event.revision) {
            continue;
        }
        by_revision.insert(event.revision, revision_record(event)?);
    }
    Ok(by_revision.into_values().collect())
}

/// The accepted-command record an event stands for, about `subject`.
///
/// Identities are derived from the seal's event id, digested into the identifier charset; a record
/// rebuilt twice from one event has one identity. An observation (no change, same state before and
/// after) carries no change record, as the contract's own rule for a record that changed nothing.
fn audit_of(event: &DomainEvent, subject: &EntityId) -> Option<AuditRecord> {
    let seal = event.payload.as_object()?;
    let event_id = seal.get("event_id")?.as_str()?;
    let digest = format!("{:016x}", digest(event_id));
    let at = Timestamp::from_epoch_millis(seal.get("at")?.as_u64()?);
    let actor = ActorRef::parse(seal.get("actor")?.as_str()?).ok()?;
    let correlation = seal.get("correlation")?.as_str()?.parse().ok()?;
    let mut record = AuditRecord::new(
        AuditId::new(format!("aud-log-{digest}")).ok()?,
        AuditKind::CommandAccepted,
        at,
        actor,
        correlation,
    );
    record.executor = match seal.get("executor") {
        Some(Value::String(executor)) => ActorRef::parse(executor).ok(),
        _ => None,
    };
    record.subject = Some(EntityRef::new(subject.clone()));
    record.command_id = seal
        .get("causation")
        .and_then(Value::as_str)
        .and_then(|causation| causation.parse().ok());
    record.event_id = EventId::new(format!("evt-log-{digest}")).ok();
    let observation =
        event.changed.is_empty() && event.from_state.as_deref() == Some(event.to_state.as_str());
    if !observation {
        record.change = Some(ChangeRecord {
            entity: EntityRef::new(subject.clone()),
            before: event
                .from_state
                .as_ref()
                .and_then(|_| EntityRevision::new(event.revision.saturating_sub(1)).ok()),
            after: EntityRevision::new(event.revision).ok(),
            command: Some(event.event_type.clone()),
            payload: None,
            redacted: false,
            redaction_reason: None,
        });
    }
    Some(record)
}

/// The same filters `aep-backend-memory` applies, so a record from the log is held to the query the
/// way a record from memory is.
fn audit_matches(record: &AuditRecord, query: &AuditQuery) -> bool {
    query.entity.as_ref().is_none_or(|entity| {
        record
            .subject
            .as_ref()
            .is_some_and(|subject| subject.id == entity.id)
    }) && query
        .correlation_id
        .as_ref()
        .is_none_or(|correlation| &record.correlation_id == correlation)
        && query
            .command_id
            .as_ref()
            .is_none_or(|command| record.command_id.as_ref() == Some(command))
        && query
            .actor
            .as_ref()
            .is_none_or(|actor| &record.actor == actor)
        && query
            .kind
            .as_ref()
            .is_none_or(|kind| record.kind.as_str() == kind)
        && query.since.is_none_or(|since| record.occurred_at >= since)
        && query.until.is_none_or(|until| record.occurred_at < until)
        && (!query.rejected_only || record.is_rejection())
}

/// FNV-1a, 64-bit — an identity component, not a security boundary, and not worth a dependency.
fn digest(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// A JSON object, or an empty one — the value handed in is always built as an object here.
fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
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

/// The entity body, as the provider's field map.
fn as_fields(body: &Node) -> Map<String, Value> {
    match serde_json::to_value(body) {
        Ok(Value::Object(map)) => map,
        Ok(other) => {
            let mut map = Map::new();
            map.insert("body".to_owned(), other);
            map
        }
        Err(_) => Map::new(),
    }
}

impl<S: AtomicBatchStore, P: Projection<S> + Clone> CommandService for EntityBackend<S, P> {
    type Command = Command;

    // The `async` belongs to the contract. This body completes without awaiting: every provider
    // `entity-runtime` ships is synchronous, and making this otherwise would pull a runtime into a
    // crate that needs none.
    #[allow(clippy::unused_async_trait_impl)]
    async fn execute(
        &self,
        envelope: CommandEnvelope<Self::Command>,
    ) -> Result<CommandResult, CommandError> {
        self.check_latch()?;
        let mut provenance = Provenance::of(&envelope);
        let candidate = self.inner.fork();
        let mut projection = self
            .projection
            .lock()
            .expect("the projection is not poisoned")
            .clone();
        if let Some(target) = envelope.target.as_ref() {
            let expected = self
                .inner
                .with_store(|store| store.entity(&target.id).map(|held| held.metadata.revision));
            if let (Some(coordinate), Some(expected)) =
                (projection.coordinates(&self.inner, &target.id), expected)
            {
                provenance.optimistic = Some((coordinate, Expect::Revision(expected.get())));
            }
        }
        projection.before(&envelope)?;
        let before = self.snapshot(&provenance.key);
        let outcome = block_on(candidate.execute(envelope));
        if outcome
            .as_ref()
            .is_ok_and(|result| result.outcome == CommandOutcome::Replayed)
        {
            return outcome;
        }
        let mut placements = Vec::new();
        if let Ok(result) = &outcome {
            let durable = self.durable.lock().expect("the provider is not poisoned");
            placements = projection.placements(&durable, &candidate, result)?;
        }
        // Whatever the contract decided, the records it wrote about deciding reach the store: a
        // refusal is recorded (invariant 15), so its record is too.
        let records = {
            let durable = self.durable.lock().expect("the provider is not poisoned");
            projection.records(&durable, &candidate, &before, &provenance.key)?
        };
        {
            let mut durable = self.durable.lock().expect("the provider is not poisoned");
            let commits = Self::commits(&durable, &placements, &records, &provenance)?;
            durable
                .commit_batch(&commits)
                .map_err(|error| provider_error(&error))?;
        }
        self.inner.replace_with(&candidate);
        *self
            .projection
            .lock()
            .expect("the projection is not poisoned") = projection;
        outcome
    }
}

impl<S: Store, P: Projection<S>> QueryService for EntityBackend<S, P> {
    type AuditRecord = AuditRecord;

    #[allow(clippy::unused_async_trait_impl)]
    async fn get(
        &self,
        reference: &EntityRef,
        consistency: QueryConsistency,
    ) -> Result<EntityEnvelope, QueryError> {
        // The latch covers reads too, and the module doc always said it did: "every later call
        // refuses rather than serving state that is not durable". It covered writes only once, so a
        // latched backend went on answering from memory about a store it no longer matched — which
        // is the one thing the latch exists to stop.
        self.refuse_when_latched()?;
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

    /// # From the log, not from this process
    ///
    /// The events the provider holds for the entity are the history (wave G, story 3): a second
    /// process answers the same records the first does, and the in-memory `RevisionRecord`s are a
    /// cache of the log rather than its source. An entity the projection has no coordinates for,
    /// or one with no events yet — a plan's document that predates its provider — answers from
    /// memory, which is what it answered before.
    #[allow(clippy::unused_async_trait_impl)]
    async fn history(&self, reference: &EntityRef) -> Result<Vec<RevisionRecord>, QueryError> {
        self.refuse_when_latched()?;
        if let Some((entity, id)) =
            self.with_projection(|projection| projection.coordinates(&self.inner, &reference.id))
        {
            let events = self
                .with_store(|store| store.events(&entity, &id))
                .map_err(|error| QueryError::Unavailable {
                    reason: format!("the event log could not be read: {error}"),
                })?;
            let records = history_of(&events).map_err(|detail| QueryError::Unavailable {
                reason: format!("the event log could not be read back as a history: {detail}"),
            })?;
            if !records.is_empty() {
                return Ok(records);
            }
        }
        block_on(self.inner.history(reference))
    }

    /// # From the log as well
    ///
    /// What this process recorded, and — for every entity the projection can locate — an accepted
    /// record for each event in the log whose command this process did not see. A refusal leaves
    /// no event, so refusals are this process's; the [`Identity`] projection hydrates every audit
    /// record on open, so for it the log adds nothing.
    #[allow(clippy::unused_async_trait_impl)]
    async fn audit(&self, query: &AuditQuery) -> Result<Page<Self::AuditRecord>, QueryError> {
        self.refuse_when_latched()?;
        // Aggregate the complete answer first. Paginating memory and then appending records rebuilt
        // from provider events makes every continuation apply to only half of the result set.
        let mut unpaged = query.clone();
        unpaged.limit = None;
        unpaged.after = None;
        let mut items = block_on(self.inner.audit(&unpaged))?.items;
        let known: BTreeSet<_> = items
            .iter()
            .filter_map(|record| record.command_id.clone())
            .collect();
        let subjects: Vec<EntityId> = match &query.entity {
            Some(entity) => vec![entity.id.clone()],
            None => self
                .inner
                .with_store(|store| store.entities().map(|e| e.metadata.id.clone()).collect()),
        };
        let mut extra = Vec::new();
        for subject in subjects {
            let Some((entity, id)) =
                self.with_projection(|projection| projection.coordinates(&self.inner, &subject))
            else {
                continue;
            };
            let events = self
                .with_store(|store| store.events(&entity, &id))
                .map_err(|error| QueryError::Unavailable {
                    reason: format!("the event log could not be read: {error}"),
                })?;
            for event in &events {
                let Some(record) = audit_of(event, &subject) else {
                    continue;
                };
                let seen = record
                    .command_id
                    .as_ref()
                    .is_some_and(|command| known.contains(command));
                if !seen && audit_matches(&record, query) {
                    extra.push(record);
                }
            }
        }
        extra.sort_by_key(|record| record.occurred_at);
        items.extend(extra);
        items.sort_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.audit_id.cmp(&right.audit_id))
        });
        Page::paginate(items, query.limit, query.after.as_ref())
    }

    /// # The ladder, from the definition the kernel decides with
    ///
    /// Wave H, story 2 (D-P5): the in-memory backend describes a kind's commands and relations and
    /// leaves `lifecycle` empty, because ladders are plan data it does not hold. The projection
    /// holds them, and the descriptor is [`kernel::describe`] over the same [`EntityDefinition`]
    /// a move is decided against — so a harness that reads which statuses a story may hold reads
    /// what the store will enforce.
    ///
    /// [`EntityDefinition`]: entity_core::EntityDefinition
    #[allow(clippy::unused_async_trait_impl)]
    async fn describe_type(&self, entity_type: &EntityType) -> Result<TypeDescriptor, QueryError> {
        let mut descriptor = block_on(self.inner.describe_type(entity_type))?;
        if descriptor.lifecycle.is_none() {
            let kind = aep_domain::artifact::ArtifactKind::NAMED
                .iter()
                .find(|kind| &kind.entity_type() == entity_type);
            if let Some(kind) = kind {
                descriptor.lifecycle = self.with_projection(|projection| {
                    projection
                        .lifecycles()
                        .and_then(|ladders| ladders.for_kind(kind))
                        .and_then(|lifecycle| kernel::describe(Some(kind), lifecycle))
                });
            }
        }
        Ok(descriptor)
    }
}
