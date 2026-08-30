//! The store: entities, relations, history, audit and idempotency, in `BTreeMap`s.
//!
//! Deliberately boring. Its job is to be *obviously* correct so that the conformance suite is
//! testing the contract rather than this implementation, and to give anyone building a real backend
//! something to diff against.
//!
//! Two decisions worth knowing:
//!
//! * **Time comes from the caller.** Every command carries `issued_at`, so the store needs no clock
//!   and a replay produces identical timestamps. A backend that stamped its own would make an
//!   audit trail that cannot be reproduced.
//! * **Identifiers are generated from a counter, past whatever is already held.** They satisfy the
//!   opacity rule and are deterministic, which is what lets a conformance run be compared byte for
//!   byte. A store that was *hydrated* — filled from a durable provider with identities another
//!   process minted — holds ids the counter has not reached, so a mint that lands on a held id
//!   moves on rather than reusing it. Nothing parses an id to find out where the counter should
//!   be (invariant 13): the check is a lookup, and the cost is one extra tick per collision.

use std::collections::{BTreeMap, BTreeSet};

use aep_contract::consistency::ConsistencyToken;
use aep_contract::query::{Relation, RevisionRecord};
use aep_domain::audit::AuditRecord;
use aep_domain::domain_event::DomainEventEnvelope;
use aep_domain::entity::{EntityId, EntityLocator, EntityMetadata};
use aep_domain::ids::{AuditId, CommandId, EventId, IdempotencyKey, RelationId};
use aep_domain::node::Node;

/// One stored entity: its metadata and its untyped body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEntity {
    /// Identity, type, revision, provenance.
    pub metadata: EntityMetadata,
    /// The body, as the contract carries it.
    pub data: Node,
    /// Whether it has been archived. Archived entities remain readable — there is no delete.
    pub archived: bool,
}

/// What a previously-applied command produced, kept so a replay can return it unchanged.
#[derive(Debug, Clone)]
pub struct AppliedCommand {
    /// Which logical command it was.
    pub command_id: CommandId,
    /// What it returned the first time.
    pub result: aep_contract::command::CommandResult,
}

/// Everything the backend holds.
#[derive(Debug, Clone, Default)]
pub struct Store {
    entities: BTreeMap<EntityId, StoredEntity>,
    locators: BTreeMap<String, EntityId>,
    relations: BTreeMap<RelationId, Relation>,
    history: BTreeMap<EntityId, Vec<RevisionRecord>>,
    audit: Vec<AuditRecord>,
    /// The audit ids held, so a mint can tell a taken one; `audit` is in write order and stays so.
    audit_ids: BTreeSet<AuditId>,
    events: Vec<DomainEventEnvelope>,
    applied: BTreeMap<String, AppliedCommand>,
    sequence: u64,
}

impl Store {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the sequence and returns the new value, which every generated identifier uses.
    fn tick(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    /// The token describing the store's current point in history.
    pub fn token(&self) -> ConsistencyToken {
        ConsistencyToken::new(format!("seq-{:012}", self.sequence))
            .expect("a generated token is well formed")
    }

    /// `true` when the store has advanced at least as far as `token`.
    ///
    /// The store is immediately consistent, so this is only ever false for a token from a *different*
    /// store — which is worth catching rather than silently satisfying.
    pub fn has_reached(&self, token: &ConsistencyToken) -> bool {
        token
            .as_str()
            .strip_prefix("seq-")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|sequence| sequence <= self.sequence)
    }

    /// Generates an opaque entity identifier that no held entity carries.
    pub fn next_entity_id(&mut self) -> EntityId {
        loop {
            let sequence = self.tick();
            let id = EntityId::new(format!("01MEM{sequence:016}"))
                .expect("a generated identifier is well formed");
            if !self.entities.contains_key(&id) {
                return id;
            }
        }
    }

    /// Generates a relation identifier that no held relation carries.
    pub fn next_relation_id(&mut self) -> RelationId {
        loop {
            let sequence = self.tick();
            let id = RelationId::new(format!("rel-{sequence:012}"))
                .expect("a generated identifier is well formed");
            if !self.relations.contains_key(&id) {
                return id;
            }
        }
    }

    /// Generates an audit identifier that no held record carries.
    pub fn next_audit_id(&mut self) -> AuditId {
        loop {
            let sequence = self.tick();
            let id = AuditId::new(format!("aud-{sequence:012}"))
                .expect("a generated identifier is well formed");
            if !self.audit_ids.contains(&id) {
                return id;
            }
        }
    }

    /// Generates an event identifier.
    pub fn next_event_id(&mut self) -> EventId {
        let sequence = self.tick();
        EventId::new(format!("evt-{sequence:012}")).expect("a generated identifier is well formed")
    }

    /// Inserts a new entity and indexes its locator.
    pub fn insert_entity(&mut self, entity: StoredEntity) {
        self.locators.insert(
            entity.metadata.locator.to_string(),
            entity.metadata.id.clone(),
        );
        self.entities.insert(entity.metadata.id.clone(), entity);
    }

    /// The entity with this identity.
    pub fn entity(&self, id: &EntityId) -> Option<&StoredEntity> {
        self.entities.get(id)
    }

    /// The entity with this identity, mutably.
    pub fn entity_mut(&mut self, id: &EntityId) -> Option<&mut StoredEntity> {
        self.entities.get_mut(id)
    }

    /// Every entity, in identity order.
    pub fn entities(&self) -> impl Iterator<Item = &StoredEntity> {
        self.entities.values()
    }

    /// The identity a locator resolves to.
    pub fn resolve(&self, locator: &EntityLocator) -> Option<&EntityId> {
        self.locators.get(&locator.to_string())
    }

    /// `true` when a locator is already taken, which is what stops two entities sharing an address.
    pub fn locator_taken(&self, locator: &EntityLocator) -> bool {
        self.locators.contains_key(&locator.to_string())
    }

    /// Records a relation.
    pub fn insert_relation(&mut self, relation: Relation) {
        self.relations.insert(relation.id.clone(), relation);
    }

    /// Removes a relation, returning it.
    pub fn remove_relation(&mut self, id: &RelationId) -> Option<Relation> {
        self.relations.remove(id)
    }

    /// Every relation, in identifier order.
    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }

    /// Appends a revision record to an entity's history.
    pub fn record_revision(&mut self, id: &EntityId, record: RevisionRecord) {
        self.history.entry(id.clone()).or_default().push(record);
    }

    /// An entity's history, oldest first.
    pub fn history(&self, id: &EntityId) -> &[RevisionRecord] {
        self.history.get(id).map_or(&[], Vec::as_slice)
    }

    /// Appends an audit record.
    pub fn record_audit(&mut self, record: AuditRecord) {
        self.audit_ids.insert(record.audit_id.clone());
        self.audit.push(record);
    }

    /// Every audit record, in the order it was written.
    pub fn audit(&self) -> &[AuditRecord] {
        &self.audit
    }

    /// Appends a domain event.
    pub fn record_event(&mut self, event: DomainEventEnvelope) {
        self.events.push(event);
    }

    /// Every domain event, in the order it occurred.
    pub fn events(&self) -> &[DomainEventEnvelope] {
        &self.events
    }

    /// What a previously-applied command with this idempotency key produced.
    pub fn applied(&self, key: &IdempotencyKey) -> Option<&AppliedCommand> {
        self.applied.get(key.as_str())
    }

    /// Remembers what a command produced, so a replay returns the same answer.
    pub fn remember(&mut self, key: &IdempotencyKey, applied: AppliedCommand) {
        self.applied.insert(key.as_str().to_owned(), applied);
    }

    /// How many entities are stored.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// `true` when nothing is stored.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
