//! Command-wide durability and optimistic concurrency regressions.

use std::path::PathBuf;

use aep_backend_entity::EntityBackend;
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::consistency::QueryConsistency;
use aep_contract::query::QueryService;
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity, UpdateEntity};
use aep_domain::entity::{ActorRef, EntityRef, EntityRevision, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_sqlite::SqliteStore;
use entity_store::{
    AtomicBatchStore, AtomicCommit, EventProvider, Expect, MemoryStore, StateProvider, Store,
    StoreError,
};

fn file(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    path
}

fn envelope(
    id: &str,
    payload: Command,
    expected: Option<EntityRevision>,
) -> CommandEnvelope<Command> {
    let command_type = payload.kind().as_str().to_owned();
    let target = payload.target();
    let mut envelope = CommandEnvelope::new(
        id.parse().expect("command id"),
        command_type,
        payload,
        CommandContext::new(
            format!("request-{id}").parse().expect("request id"),
            format!("key-{id}").parse().expect("idempotency key"),
            ActorRef::parse("human:atomic").expect("actor"),
            "correlation-atomic".parse().expect("correlation"),
            Timestamp::from_epoch_millis(1_700_000_000_000),
        ),
    );
    envelope.target = target;
    envelope.expected_revision = expected;
    envelope
}

fn create() -> Command {
    Command::CreateEntity(CreateEntity {
        entity_type: EntityType::parse("aep.story/v1").expect("type"),
        locator: "ep://beyond10x/plan/story/atomic".parse().expect("locator"),
        data: Node::Map(
            [
                ("status".to_owned(), Node::from("draft")),
                ("title".to_owned(), Node::from("Initial")),
            ]
            .into(),
        ),
    })
}

fn rename(target: &EntityRef, title: &str) -> Command {
    Command::UpdateEntity(UpdateEntity {
        target: target.clone(),
        changes: [("title".to_owned(), Node::from(title))].into(),
    })
}

fn title(data: &Node) -> Option<&Node> {
    match data {
        Node::Map(fields) => fields.get("title"),
        _ => None,
    }
}

#[test]
fn a_writer_hydrated_before_a_newer_commit_cannot_overwrite_it() {
    let path = file("stale-writers");
    let first = EntityBackend::over(SqliteStore::open(&path).expect("database")).expect("opens");
    let created = block_on(first.execute(envelope("create", create(), None))).expect("created");
    let target = created.affected[0].unversioned();
    let stale = EntityBackend::over(SqliteStore::open(&path).expect("second handle"))
        .expect("stale writer hydrates revision one");

    block_on(first.execute(envelope(
        "fresh-update",
        rename(&target, "Fresh"),
        Some(EntityRevision::INITIAL),
    )))
    .expect("first writer advances the durable row");
    let error = block_on(stale.execute(envelope(
        "stale-update",
        rename(&target, "Stale"),
        Some(EntityRevision::INITIAL),
    )))
    .expect_err("the stale pre-command expectation loses");
    assert!(error.to_string().contains("expected revision 1"), "{error}");

    let stale_view = block_on(stale.get(&target, QueryConsistency::Current)).expect("old view");
    assert_eq!(title(&stale_view.data), Some(&Node::from("Initial")));
    let reopened = EntityBackend::over(SqliteStore::open(&path).expect("third handle"))
        .expect("durable state reopens");
    let durable = block_on(reopened.get(&target, QueryConsistency::Current)).expect("current row");
    assert_eq!(title(&durable.data), Some(&Node::from("Fresh")));
}

#[derive(Debug, Default)]
struct FailingBatch {
    inner: MemoryStore,
    received: usize,
}

impl StateProvider for FailingBatch {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        self.inner.load(entity, id)
    }

    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        self.inner.ids(entity)
    }
}

impl EventProvider for FailingBatch {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        self.inner.events(entity, id)
    }
}

impl Store for FailingBatch {
    fn commit(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        self.inner.commit(decision, expect)
    }
}

impl AtomicBatchStore for FailingBatch {
    fn commit_batch(&mut self, commits: &[AtomicCommit]) -> Result<(), StoreError> {
        self.received = commits.len();
        Err(StoreError::Backend("injected batch failure".to_owned()))
    }
}

#[test]
fn a_failed_provider_batch_publishes_neither_durable_nor_local_prefix() {
    let backend = EntityBackend::over(FailingBatch::default()).expect("opens");
    let error = block_on(backend.execute(envelope("create", create(), None)))
        .expect_err("the provider refuses the transaction");
    assert!(
        error.to_string().contains("injected batch failure"),
        "{error}"
    );
    assert!(backend.is_empty(), "candidate memory was not published");
    backend.with_store(|store| {
        assert!(
            store.received > 1,
            "the complete command arrived as one batch"
        );
        assert!(store.inner.ids("aep.entity").expect("ids").is_empty());
        assert!(store.inner.ids("aep.audit").expect("ids").is_empty());
        assert!(store.inner.ids("aep.applied").expect("ids").is_empty());
    });
}
