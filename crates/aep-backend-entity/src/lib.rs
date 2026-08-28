//! The interaction contract over any `entity-store` provider.
//!
//! One adapter. `aep-backend-sqlite` is [`EntityBackend`] over `entity_sqlite::SqliteStore`; the
//! next durable backend is [`EntityBackend`] over the next provider `entity-runtime` ships, and it
//! passes the sixteen conformance suites because this adapter already does and the provider already
//! passed the runtime's. Wave F, story 2 of `docs/plan/store-waves-f-g-h.md`: this crate is the
//! generic form of what `SqliteBackend` was, extracted so that events (F3) and hydration (F4) are
//! written once.
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
//! # Events cross the seam
//!
//! Every accepted command becomes one [`entity_core::DomainEvent`] per affected entity, written
//! with the instance in the same `commit` — so the provider's guarantee that *a state cannot move
//! without the event that explains it* (runtime R-83) protects a real event and not an empty list,
//! which is what it protected before wave F, story 3. The event's type is the command's type name
//! (`aep.status.move/v1` and so on), `from_state`/`to_state` are the entity's `status` before and
//! after, and `changed` is the fields the command wrote. A refused command writes nothing (R-84
//! across the seam); a replayed one writes nothing, as it writes no instance.
//!
//! **Who and when travel in the event's `payload`.** The runtime seals an event with a
//! [`Recording`] — `recorded_at`, `correlation`, `causation`, `actor` — into an
//! [`entity_store::Envelope`], but its providers store the bare `DomainEvent` a `Decision` carries
//! (`entity-sqlite` writes `decision.events`; `entity-cli` prints the sealed envelopes and commits
//! the decision). So the sealed envelope's five fields are written into `payload`, and a
//! `MoveStatus`'s `decided_on` account beside them, under `decided_on`. That is where a second
//! process finds *who moved a story and when*, and it is a decision this crate records rather than
//! assumes: the plan's default put the account in `causation`, and `causation` does not reach the
//! file.
//!
//! # The window, stated
//!
//! A command is applied and **then** written, because what must be written is the result of
//! applying it. If the write fails, this backend latches and every later call — reads included —
//! refuses rather than answering from memory about a store that no longer agrees. Closing that
//! properly needs a durable intent log, which is `P6`.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Mutex;

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
use aep_domain::entity::{EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::node::Node;
use entity_core::DomainEvent;
use entity_store::{Envelope, Expect, Recording, Store};
use serde_json::{json, Map, Value};

/// The entity type every contract entity is stored under in the provider.
///
/// One type, because the contract's entity *types* are data — `design`, `story`, whatever an
/// adopter declares — and encoding them as `entity-core` definitions would make adding a type a
/// schema migration. The contract's own type lives in the instance's fields.
pub const STORED_AS: &str = "aep.entity";

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
            decided_on,
        }
    }
}

/// A move's account as JSON. Serialising a `Node` does not fail in practice; if it ever does, the
/// failure is written down rather than swallowed — `.ok()` on this value is how a provenance was
/// silently lost once already (`story:journal-backed-store`).
fn account_of(node: &Node) -> Value {
    serde_json::to_value(node)
        .unwrap_or_else(|error| json!({ "unserialisable": error.to_string() }))
}

/// The contract, over a provider `S`.
pub struct EntityBackend<S> {
    inner: MemoryBackend,
    durable: Mutex<S>,
    latched: Mutex<Option<String>>,
    /// The identities this backend has written, so it can tell its own rows from somebody else's.
    ///
    /// # What this stops, and it is not hypothetical
    ///
    /// [`EntityBackend::over`] builds a fresh [`MemoryBackend`], which mints identities from a
    /// **per-process counter** — the first `CreateEntity` of every run is `01MEM…0001`. A second
    /// run against the same store therefore reuses run 1's identity, and `persist` reads the
    /// durable revision immediately before committing, so the expectation matches by construction
    /// and the provider overwrites the instance. No conflict, no latch, exit 0, and somebody's data
    /// gone.
    ///
    /// Hydration is F4's work (`story:sqlite-hydrates-on-open`) and retires this set. Deferring it
    /// does not license destroying what is already there.
    written: Mutex<BTreeSet<String>>,
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

impl<S: Store> EntityBackend<S> {
    /// The contract over `store`, holding nothing until a command is executed.
    ///
    /// Nothing is read back from the provider yet — that is F4. Until then a populated store is
    /// protected by the foreign-row refusal in `persist`, not by hydration.
    pub fn over(store: S) -> Self {
        Self {
            inner: MemoryBackend::new(),
            durable: Mutex::new(store),
            latched: Mutex::new(None),
            written: Mutex::new(BTreeSet::new()),
        }
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
            let envelope = block_on(self.inner.get(
                &EntityRef::new(reference.id.clone()),
                QueryConsistency::Current,
            ))
            .map_err(|error| self.latch(format!("the entity could not be read back: {error}")))?;

            let to_state = status_of(&envelope.data);
            let fields = as_fields(&envelope.data);

            // The expectation is **read**, not assumed. Deriving it from the contract's revision
            // ("it must be at mine minus one") assumes every command bumps by exactly one and that
            // nothing else ever writes — and the first assumption is already false for a command
            // that touches an entity twice. Reading it keeps the optimistic check honest: what it
            // still catches is the store moving underneath this process between the read and the
            // write, which is what `Expect` is for.
            let key = reference.id.to_string();
            let mut durable = self.durable.lock().expect("the provider is not poisoned");
            let held = durable
                .load(STORED_AS, &key)
                .map_err(|error| self.latch(error.to_string()))?;
            // **An instance this backend never wrote is somebody else's.** See `written`:
            // identities come from a per-process counter, so a second run's first entity collides
            // with the first run's, and the expectation below would match by construction and
            // overwrite it. This is the check that turns silent destruction into a refusal.
            let mine = self
                .written
                .lock()
                .expect("the written set is not poisoned")
                .contains(&key);
            if held.is_some() && !mine {
                drop(durable);
                return Err(self.latch(format!(
                    "the store already holds `{key}`, and this backend did not write it. It cannot \
                     yet read a store back, so it would mint this identity from a fresh counter and \
                     overwrite what is there. Hydration is F4; point this at an empty store until \
                     then."
                )));
            }

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
            let decision = entity_core::Decision {
                instance: entity_core::EntityInstance {
                    entity: STORED_AS.to_owned(),
                    version: 1,
                    id: key.clone(),
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
            drop(durable);
            self.written
                .lock()
                .expect("the written set is not poisoned")
                .insert(key);
        }
        Ok(())
    }
}

impl Provenance {
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
        event.payload = recorded(&sealed[0], self.decided_on.as_ref());
        event
    }
}

/// The sealed envelope's five fields and the move's account, as the event's payload.
///
/// `decided_on` is always written — `null` when the command carried no account — because an absent
/// key and *nothing was decided on* would otherwise read alike, and only one of them is a claim.
fn recorded(sealed: &Envelope<DomainEvent>, decided_on: Option<&Value>) -> Value {
    json!({
        "event_id": sealed.event_id,
        "recorded_at": sealed.recorded_at,
        "correlation": sealed.correlation,
        "causation": sealed.causation,
        "actor": sealed.actor,
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
        let result = block_on(self.inner.execute(envelope))?;
        self.persist(&result, &provenance)?;
        Ok(result)
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
