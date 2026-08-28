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
//! # What the provider holds, and how a second process gets it back
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
//! the seal's `recorded_at` is to the second) and the executor, and a `MoveStatus`'s `decided_on`
//! account under `decided_on`. That is what a rebuilt history is made of.
//!
//! The record types carry no events of their own. R-83 is about an instance whose *state* moves;
//! a relation, an audit record and an idempotency entry are records of something that already
//! happened, and the event explaining them is the entity's.
//!
//! # The window, stated
//!
//! A command is applied and **then** written — the entity with its event first, then the records —
//! because what must be written is the result of applying it. If any write fails, this backend
//! latches and every later call, reads included, refuses rather than answering from memory about a
//! store that no longer agrees. Each `commit` is atomic; the sequence of them is not. Closing that
//! properly needs a durable intent log, which is `P6`.

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
use aep_domain::audit::AuditRecord;
use aep_domain::command::Command;
use aep_domain::entity::{
    ActorRef, EntityId, EntityLocator, EntityMetadata, EntityRef, EntityRevision, EntityType,
};
use aep_domain::ids::{IdempotencyKey, RelationId};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_store::{Envelope, Expect, Recording, Store, StoreError};
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
    /// A `MoveStatus`'s account of what the move rested on, when it carried one.
    decided_on: Option<Value>,
}

impl Provenance {
    fn of(envelope: &CommandEnvelope<Command>) -> Self {
        let decided_on = match &envelope.payload {
            Command::MoveStatus(move_status) => move_status.decided_on.as_ref().map(account_of),
            _ => None,
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
            decided_on,
        }
    }

    /// One event for one affected entity, sealed, with the seal written into its payload.
    fn event(
        &self,
        id: &str,
        revision: u64,
        from_state: Option<String>,
        to_state: String,
        changed: Map<String, Value>,
    ) -> DomainEvent {
        let mut event = DomainEvent {
            entity: STORED_AS.to_owned(),
            version: 1,
            id: id.to_owned(),
            revision,
            event_type: self.command_type.clone(),
            from_state,
            to_state,
            changed,
            payload: Value::Null,
        };
        // Sealed by the runtime's own `Recording`, so the event id is the derived
        // `<entity>:<id>@<revision>#<index>` every other shell would derive — and then the seal is
        // written into the event, because the seal is the only part of an envelope the providers
        // do not keep (see the module doc).
        let sealed = self.recording.seal(std::slice::from_ref(&event));
        event.payload = recorded(
            &sealed[0],
            self.at,
            self.executor.as_ref(),
            self.decided_on.as_ref(),
        );
        event
    }
}

/// A move's account as JSON. Serialising a `Node` does not fail in practice; if it ever does, the
/// failure is written down rather than swallowed — `.ok()` on this value is how a provenance was
/// silently lost once already (`story:journal-backed-store`).
fn account_of(node: &Node) -> Value {
    serde_json::to_value(node)
        .unwrap_or_else(|error| json!({ "unserialisable": error.to_string() }))
}

/// The sealed envelope's fields, the contract's instant and executor, and the move's account, as
/// the event's payload.
///
/// `decided_on` and `executor` are always written — `null` when absent — because an absent key and
/// *nothing was decided on* would otherwise read alike, and only one of them is a claim.
fn recorded(
    sealed: &Envelope<DomainEvent>,
    at: Timestamp,
    executor: Option<&ActorRef>,
    decided_on: Option<&Value>,
) -> Value {
    json!({
        "event_id": sealed.event_id,
        "recorded_at": sealed.recorded_at,
        "at": at.epoch_millis(),
        "correlation": sealed.correlation,
        "causation": sealed.causation,
        "actor": sealed.actor,
        "executor": executor.map(ToString::to_string),
        "decided_on": decided_on.cloned().unwrap_or(Value::Null),
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
struct Snapshot {
    relations: BTreeSet<RelationId>,
    audit: usize,
    applied: bool,
}

/// The contract, over a provider `S`.
pub struct EntityBackend<S> {
    inner: MemoryBackend,
    durable: Mutex<S>,
    latched: Mutex<Option<String>>,
}

impl<S> fmt::Debug for EntityBackend<S> {
    /// Hand-written because a provider may hold a connection and not derive `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EntityBackend")
            .field("entities", &self.inner.len())
            .field("latched", &self.latched())
            .finish_non_exhaustive()
    }
}

impl<S> EntityBackend<S> {
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

impl<S: Store> EntityBackend<S> {
    /// The contract over `store`, holding everything the store holds.
    ///
    /// Hydrates: every `aep.entity` with its history, every relation not since removed, every audit
    /// record and every applied command are installed in memory with their stored identities. A
    /// second process opening the same store sees what the first wrote.
    ///
    /// # Errors
    ///
    /// If the store cannot be read, or holds a row this adapter cannot read back — a listed id that
    /// does not load, an entity with no [`METADATA_KEY`], metadata disagreeing with the instance, an
    /// event with no seal. Refused rather than skipped, naming the row.
    pub fn over(store: S) -> Result<Self, CommandError> {
        let inner = MemoryBackend::new();
        hydrate(&store, &inner)?;
        Ok(Self {
            inner,
            durable: Mutex::new(store),
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

    fn latch(&self, detail: impl AsRef<str>) -> CommandError {
        let detail = detail.as_ref();
        *self.latched.lock().expect("the latch is not poisoned") = Some(detail.to_owned());
        CommandError::Conflict {
            reason: format!("the contract moved in memory but not in the store: {detail}"),
        }
    }

    fn snapshot(&self, key: &IdempotencyKey) -> Snapshot {
        self.inner.with_store(|store| Snapshot {
            relations: store.relations().map(|r| r.id.clone()).collect(),
            audit: store.audit().len(),
            applied: store.applied(key).is_some(),
        })
    }

    /// Writes every entity a command touched into the provider, each with the event that explains
    /// the write.
    fn persist(&self, result: &CommandResult, provenance: &Provenance) -> Result<(), CommandError> {
        // A replay changed nothing, so there is nothing to write. Writing anyway would push the
        // same revision at a store that already holds it, which its own optimistic check would
        // rightly refuse — and this backend would latch over a command that did no harm.
        if result.outcome == CommandOutcome::Replayed {
            return Ok(());
        }
        for reference in &result.affected {
            let stored = self
                .inner
                .with_store(|store| store.entity(&reference.id).cloned())
                .ok_or_else(|| {
                    self.latch(format!(
                        "the entity `{}` could not be read back after the command",
                        reference.id
                    ))
                })?;
            let to_state = status_of(&stored.data);
            let fields = pack(&stored);
            let key = reference.id.to_string();

            // The expectation is **read**, not assumed. Deriving it from the contract's revision
            // ("it must be at mine minus one") assumes every command bumps by exactly one and that
            // nothing else ever writes — and the first assumption is already false for a command
            // that touches an entity twice. Reading it keeps the optimistic check honest: what it
            // still catches is the store moving underneath this process between the read and the
            // write, which is what `Expect` is for.
            let mut durable = self.durable.lock().expect("the provider is not poisoned");
            let held = durable
                .load(STORED_AS, &key)
                .map_err(|error| self.latch(error.to_string()))?;

            // The event is built from what the store held a moment ago and what the contract holds
            // now: `from_state` and `changed` are the difference, and for a creation there is no
            // "before", so `from_state` is `None` and `changed` is every field — which is what the
            // runtime's fold (`entity_core::rehydrate`) reads as a creation event.
            let (from_state, changed) = match &held {
                Some(before) => (
                    Some(before.lifecycle_state.clone()),
                    changed_between(&before.fields, &fields),
                ),
                None => (None, fields.clone()),
            };
            let event = provenance.event(
                &key,
                reference.revision.get(),
                from_state,
                to_state.clone(),
                changed,
            );
            let decision = Decision {
                instance: EntityInstance {
                    entity: STORED_AS.to_owned(),
                    version: 1,
                    id: key,
                    lifecycle_state: to_state,
                    revision: reference.revision.get(),
                    fields,
                },
                events: vec![event],
            };

            let expect = held.map_or(Expect::Absent, |held| Expect::Revision(held.revision));
            durable
                .commit(&decision, expect)
                .map_err(|error| self.latch(error.to_string()))?;
        }
        Ok(())
    }

    /// Writes what a command added beside the entities: relations created or removed, audit
    /// records (a refusal's too), and the applied-command entry a replay is recognised by.
    fn persist_records(&self, before: &Snapshot, key: &IdempotencyKey) -> Result<(), CommandError> {
        let (created, removed, audit, applied) = self.inner.with_store(|store| {
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

        let mut durable = self.durable.lock().expect("the provider is not poisoned");
        // `fields: None` means "what is held, marked removed" — the one update a record ever gets.
        let mut write =
            |entity: &str, id: String, state: &str, fields: Option<Map<String, Value>>| {
                let held = durable
                    .load(entity, &id)
                    .map_err(|error| self.latch(error.to_string()))?;
                let (expect, revision) = match &held {
                    Some(held) => (Expect::Revision(held.revision), held.revision + 1),
                    None => (Expect::Absent, 1),
                };
                let fields = fields.unwrap_or_else(|| {
                    let mut fields = held.map(|held| held.fields).unwrap_or_default();
                    fields.insert("removed".to_owned(), Value::Bool(true));
                    fields
                });
                let decision = Decision {
                    instance: EntityInstance {
                        entity: entity.to_owned(),
                        version: 1,
                        id,
                        lifecycle_state: state.to_owned(),
                        revision,
                        fields,
                    },
                    events: Vec::new(),
                };
                durable
                    .commit(&decision, expect)
                    .map_err(|error| self.latch(error.to_string()))
            };

        for relation in created {
            write(
                RELATIONS_AS,
                relation.id.to_string(),
                RECORDED,
                Some(object(json!({ "relation": relation, "removed": false }))),
            )?;
        }
        for id in removed {
            write(RELATIONS_AS, id.to_string(), REMOVED, None)?;
        }
        for record in audit {
            write(
                AUDIT_AS,
                record.audit_id.to_string(),
                RECORDED,
                Some(object(json!({ "record": record }))),
            )?;
        }
        if let Some(applied) = applied {
            write(
                APPLIED_AS,
                key.to_string(),
                RECORDED,
                Some(object(
                    json!({ "command_id": applied.command_id, "result": applied.result }),
                )),
            )?;
        }
        Ok(())
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

impl<S: Store> CommandService for EntityBackend<S> {
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
        let provenance = Provenance::of(&envelope);
        let before = self.snapshot(&provenance.key);
        let outcome = block_on(self.inner.execute(envelope));
        if let Ok(result) = &outcome {
            self.persist(result, &provenance)?;
        }
        // Whatever the contract decided, the records it wrote about deciding reach the store: a
        // refusal is recorded (invariant 15), so its record is too.
        self.persist_records(&before, &provenance.key)?;
        outcome
    }
}

impl<S: Store> QueryService for EntityBackend<S> {
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
