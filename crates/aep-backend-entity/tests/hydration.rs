//! Wave F, story 4: a populated store is read back on open, and the foreign-row refusal retires.
//!
//! Every test is two processes in one: a backend writes and is dropped, a second backend opens the
//! same file. What the second one answers is compared with what the first one answered, never with
//! what the test thinks it should be.

use std::collections::BTreeMap;

use aep_backend_entity::{EntityBackend, AUDIT_AS, METADATA_KEY, RELATIONS_AS, STORED_AS};
use aep_contract::command::{CommandContext, CommandEnvelope, CommandOutcome, CommandService};
use aep_contract::query::{AuditQuery, QueryService, RelationQuery};
use aep_contract::testing::block_on;
use aep_contract::QueryConsistency;
use aep_domain::artifact::RelationKind;
use aep_domain::command::{
    Command, CreateEntity, CreateRelation, MoveStatus, RemoveRelation, UpdateEntity,
};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::ids::RelationId;
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_sqlite::SqliteStore;
use entity_store::{StateProvider as _, Store as _};

const AT: u64 = 1_700_000_000_000;

fn envelope(command: Command, n: u32) -> CommandEnvelope<Command> {
    let context = CommandContext::new(
        format!("req-{n}").parse().expect("a request id"),
        format!("key-{n}").parse().expect("an idempotency key"),
        ActorRef::parse("human:operator").expect("an actor"),
        "corr-1".parse().expect("a correlation id"),
        Timestamp::from_epoch_millis(AT + u64::from(n)),
    );
    let kind = command.kind().as_str();
    CommandEnvelope::new(
        format!("cmd-{n}").parse().expect("a command id"),
        kind,
        command,
        context,
    )
}

fn text(pairs: &[(&str, &str)]) -> Node {
    Node::Map(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), Node::from(*value)))
            .collect(),
    )
}

fn create(name: &str) -> Command {
    Command::CreateEntity(CreateEntity {
        entity_type: EntityType::parse("aep.story/v1").expect("a type"),
        locator: EntityLocator::parse(&format!("ep://beyond10x/plan/story/{name}"))
            .expect("a locator"),
        data: text(&[("status", "draft"), ("title", name)]),
    })
}

fn rename(target: &EntityId, title: &str) -> Command {
    Command::UpdateEntity(UpdateEntity {
        target: EntityRef::new(target.clone()),
        changes: BTreeMap::from([("title".to_owned(), Node::from(title))]),
    })
}

fn propose(target: &EntityId) -> Command {
    Command::MoveStatus(MoveStatus {
        target: EntityRef::new(target.clone()),
        to: "proposed".to_owned(),
        expected_revision: None,
        decided_on: Some(text(&[("recorded", "test_result=1")])),
    })
}

fn relate(source: &EntityId, target: &EntityId) -> Command {
    Command::CreateRelation(CreateRelation {
        kind: RelationKind::Decomposes,
        source: EntityRef::new(source.clone()),
        target: EntityRef::new(target.clone()),
    })
}

fn unrelate(relation: &RelationId) -> Command {
    Command::RemoveRelation(RemoveRelation {
        relation: relation.clone(),
    })
}

fn fresh_file(name: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.sqlite3"));
    let _ = std::fs::remove_file(&path);
    path
}

fn open(path: &std::path::Path) -> EntityBackend<SqliteStore> {
    EntityBackend::over(SqliteStore::open(path).expect("a database")).expect("the store opens")
}

/// Everything a process can be asked about a plan, gathered so two processes can be compared.
#[derive(Debug, PartialEq, Eq)]
struct View {
    entities: Vec<aep_contract::query::EntityEnvelope>,
    relations: Vec<aep_contract::query::Relation>,
    audit: Vec<aep_domain::audit::AuditRecord>,
    history: Vec<Vec<aep_contract::query::RevisionRecord>>,
    len: usize,
}

fn view(backend: &EntityBackend<SqliteStore>, ids: &[EntityId]) -> View {
    View {
        entities: ids
            .iter()
            .map(|id| {
                block_on(backend.get(&EntityRef::new(id.clone()), QueryConsistency::Current))
                    .expect("held")
            })
            .collect(),
        relations: block_on(backend.relations(&RelationQuery::default()))
            .expect("answers")
            .items,
        audit: block_on(backend.audit(&AuditQuery::default()))
            .expect("answers")
            .items,
        history: ids
            .iter()
            .map(|id| block_on(backend.history(&EntityRef::new(id.clone()))).expect("answers"))
            .collect(),
        len: backend.len(),
    }
}

/// Two stories, related, one renamed and proposed, one command refused — a small plan with every
/// record type in it. Returns the two identities.
fn a_small_plan(backend: &EntityBackend<SqliteStore>) -> (EntityId, EntityId) {
    let one = block_on(backend.execute(envelope(create("one"), 1)))
        .expect("created")
        .affected[0]
        .id
        .clone();
    let two = block_on(backend.execute(envelope(create("two"), 2)))
        .expect("created")
        .affected[0]
        .id
        .clone();
    block_on(backend.execute(envelope(relate(&one, &two), 3))).expect("related");
    block_on(backend.execute(envelope(rename(&one, "One, renamed"), 4))).expect("renamed");
    block_on(backend.execute(envelope(propose(&one), 5))).expect("proposed");
    let ghost = EntityId::new("01MEM9999999999999999".to_owned()).expect("an id");
    block_on(backend.execute(envelope(rename(&ghost, "ghost"), 6)))
        .expect_err("an entity nobody created cannot be renamed");
    (one, two)
}

#[test]
fn a_second_process_sees_everything_the_first_wrote_with_the_same_identities() {
    let path = fresh_file("hydrate-all");
    let first = open(&path);
    let (one, two) = a_small_plan(&first);
    let before = view(&first, &[one.clone(), two.clone()]);
    drop(first);

    let second = open(&path);
    let after = view(&second, &[one, two]);

    assert_eq!(before.len, 2, "the fixture holds two entities");
    assert_eq!(before.relations.len(), 1, "and one relation");
    assert_eq!(
        before.audit.len(),
        6,
        "and six audit records, one of them a refusal"
    );
    assert_eq!(
        before.history[0].len(),
        3,
        "and a history of three for `one`"
    );
    assert_eq!(
        after, before,
        "a second process answers exactly what the first did: entities with their metadata, \
         relations, audit records including the refusal, and history"
    );
}

#[test]
fn a_second_process_mints_identities_past_the_first_ones() {
    // The defect the foreign-row refusal guarded against, closed rather than refused: run 2 used to
    // mint `01MEM…0001` again and overwrite run 1's entity. Now it holds run 1's identities and
    // mints past them — without parsing them, which invariant 13 forbids.
    let path = fresh_file("hydrate-mint");
    let first = open(&path);
    let (one, two) = a_small_plan(&first);
    let relations_before = block_on(first.relations(&RelationQuery::default()))
        .expect("answers")
        .items;
    let audit_before = block_on(first.audit(&AuditQuery::default()))
        .expect("answers")
        .items
        .len();
    drop(first);

    let second = open(&path);
    let three = block_on(second.execute(envelope(create("three"), 7)))
        .expect("run 2 continues")
        .affected[0]
        .id
        .clone();
    assert!(
        three != one && three != two,
        "a fresh identity: {three} is not {one} or {two}"
    );
    block_on(second.execute(envelope(relate(&two, &three), 8))).expect("related");
    let relations_after = block_on(second.relations(&RelationQuery::default()))
        .expect("answers")
        .items;
    assert_eq!(relations_after.len(), 2);
    // Relations come back in id order and a fresh id may sort before the old one, so the claim is
    // about the sets: one id is new, and the old one is still there.
    let old: std::collections::BTreeSet<_> =
        relations_before.iter().map(|r| r.id.clone()).collect();
    let now: std::collections::BTreeSet<_> = relations_after.iter().map(|r| r.id.clone()).collect();
    assert!(now.is_superset(&old), "run 1's relation is still held");
    assert_eq!(
        now.difference(&old).count(),
        1,
        "and exactly one fresh relation identity was minted"
    );
    let audit_after = block_on(second.audit(&AuditQuery::default()))
        .expect("answers")
        .items;
    assert_eq!(audit_after.len(), audit_before + 2, "two more records");
    let ids: std::collections::BTreeSet<_> =
        audit_after.iter().map(|r| r.audit_id.clone()).collect();
    assert_eq!(ids.len(), audit_after.len(), "and no audit id reused");
    assert!(second.latched().is_none());

    // The file, through a second handle, holds all three entities.
    let durable = SqliteStore::open(&path).expect("reopens");
    assert_eq!(durable.ids(STORED_AS).expect("answers").len(), 3);
    assert_eq!(durable.ids(RELATIONS_AS).expect("answers").len(), 2);
    assert_eq!(
        durable.ids(AUDIT_AS).expect("answers").len(),
        audit_before + 2
    );
}

#[test]
fn a_replay_across_processes_is_recognised() {
    // Idempotency memory is a record like any other, so a command retried after the process that
    // first applied it has exited is still a replay — the same answer, nothing applied twice.
    let path = fresh_file("hydrate-replay");
    let first = open(&path);
    let created = block_on(first.execute(envelope(create("one"), 1))).expect("created");
    drop(first);

    let second = open(&path);
    let replayed =
        block_on(second.execute(envelope(create("one"), 1))).expect("a replay is accepted");
    assert_eq!(replayed.outcome, CommandOutcome::Replayed);
    assert_eq!(replayed.affected, created.affected, "the same answer");
    assert_eq!(second.len(), 1, "and nothing applied twice");
}

#[test]
fn a_removed_relation_stays_removed_after_reopen() {
    let path = fresh_file("hydrate-unrelate");
    let first = open(&path);
    let (one, two) = a_small_plan(&first);
    let relation = block_on(first.relations(&RelationQuery::default()))
        .expect("answers")
        .items[0]
        .id
        .clone();
    block_on(first.execute(envelope(unrelate(&relation), 7))).expect("removed");
    assert!(block_on(first.relations(&RelationQuery::default()))
        .expect("answers")
        .items
        .is_empty());
    drop(first);

    let second = open(&path);
    assert!(
        block_on(second.relations(&RelationQuery::default()))
            .expect("answers")
            .items
            .is_empty(),
        "the removal survived the reopen"
    );
    // Nothing is physically deleted: the relation's record is still in the file, marked removed.
    let durable = SqliteStore::open(&path).expect("reopens");
    let held = durable
        .load(RELATIONS_AS, relation.as_ref())
        .expect("answers")
        .expect("still there");
    assert_eq!(held.revision, 2);
    assert_eq!(held.fields["removed"], serde_json::Value::Bool(true));
    let _ = (one, two);
}

#[test]
fn a_row_this_backend_cannot_read_back_refuses_the_open_by_name() {
    // The guard on hydration. A store holding an `aep.entity` row without the contract's metadata
    // was not written by this backend; skipping it would answer about part of the store and say
    // nothing, which is the failure nobody finds.
    let path = fresh_file("hydrate-planted");
    let first = open(&path);
    a_small_plan(&first);
    drop(first);

    let mut durable = SqliteStore::open(&path).expect("reopens");
    let mut fields = serde_json::Map::new();
    fields.insert("status".to_owned(), serde_json::Value::from("draft"));
    durable
        .commit(
            &entity_core::Decision::legacy_import(
                entity_core::EntityInstance {
                    entity: STORED_AS.to_owned(),
                    version: 1,
                    id: "planted-by-hand".to_owned(),
                    lifecycle_state: "draft".to_owned(),
                    revision: 1,
                    fields,
                },
                Vec::new(),
            ),
            entity_store::Expect::Absent,
        )
        .expect("the provider accepts any instance");
    drop(durable);

    let error = EntityBackend::over(SqliteStore::open(&path).expect("reopens"))
        .expect_err("a store with a row this backend cannot read must not open");
    let message = error.to_string();
    assert!(
        message.contains("planted-by-hand"),
        "names the row: {message}"
    );
    assert!(
        message.contains(METADATA_KEY),
        "and what is missing: {message}"
    );
}
