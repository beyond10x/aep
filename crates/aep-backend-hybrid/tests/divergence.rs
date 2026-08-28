//! A write the replica could not take is a divergence: recorded, written beside the plan, listed by
//! the next process, and replayed by `catch_up` when the replica is back. The third and fourth
//! acceptance lines of `story:hybrid-backend`, in the shape a one-process-per-command shell has.

use std::path::{Path, PathBuf};

use aep_backend_hybrid::{catch_up, read_divergences, HybridBackend, DIVERGENCES};
use aep_backend_markdown::backend::{ORGANISATION, SPACE};
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity};
use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_core::{Decision, DomainEvent, EntityInstance};
use entity_remote::{Authority, OnDivergence, Policy, ReadPath, WhenUnreachable};
use entity_sqlite::SqliteStore;
use entity_store::{EventProvider, Expect, StateProvider, Store, StoreError};

fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("divergence")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory");
    root
}

const POLICY: Policy = Policy::new(
    Authority::Local,
    ReadPath::LocalFirst,
    WhenUnreachable::ServeStale,
    OnDivergence::RecordDivergence,
);

fn at() -> Timestamp {
    Timestamp::from_epoch_millis(1_700_000_000_000)
}

fn actor() -> ActorRef {
    ActorRef::parse("human:divergence").expect("a well-formed actor")
}

/// A replica whose writes fail — the disk is full, the service is down — while its reads answer.
struct Down<R>(R);

impl<R: StateProvider> StateProvider for Down<R> {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        self.0.load(entity, id)
    }
    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        self.0.ids(entity)
    }
}

impl<R: EventProvider> EventProvider for Down<R> {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        self.0.events(entity, id)
    }
}

impl<R: Store> Store for Down<R> {
    fn commit(&mut self, _: &Decision, _: Expect) -> Result<(), StoreError> {
        Err(StoreError::Unreachable {
            provider: "replica".to_owned(),
            detail: "the replica is not taking writes".to_owned(),
        })
    }
}

/// One `CreateEntity` for `story:<name>`, through the contract.
fn create_story(backend: &impl CommandService<Command = Command>, name: &str) {
    let locator = EntityLocator::new(ORGANISATION, SPACE, "story", name).expect("an address");
    let mut data = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from("draft"));
    data.insert("title".to_owned(), Node::from("A story kept twice"));
    let context = CommandContext::new(
        format!("req-{name}").parse().expect("request id"),
        name.parse().expect("idempotency key"),
        actor(),
        format!("flow-{name}").parse().expect("correlation"),
        at(),
    );
    let envelope = CommandEnvelope::new(
        format!("cmd-{name}").parse().expect("command id"),
        "aep.entity.create/v1",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").expect("a type"),
            locator,
            data: Node::Map(data),
        }),
        context,
    );
    let result = block_on(backend.execute(envelope)).expect("the authority takes the write");
    assert_eq!(result.affected.len(), 1);
}

#[test]
fn a_write_the_replica_refused_is_recorded_beside_the_plan_and_caught_up_by_the_next_process() {
    let root = scratch("round-trip");
    let replica_file = root
        .parent()
        .expect("a parent")
        .join("round-trip-replica.sqlite3");
    let _ = std::fs::remove_file(&replica_file);

    // Process 1: the replica is down for writes. The authority takes the story; the divergence is
    // recorded and written beside the plan — the fact, not a `DELETE` or a silent retry.
    {
        let replica = Down(SqliteStore::open(&replica_file).expect("a database"));
        let backend = HybridBackend::open(
            &root,
            replica,
            POLICY,
            std::iter::empty(),
            at(),
            actor(),
            aep_domain::artifact::LifecycleRegistry::default(),
        )
        .expect("the plan opens");
        create_story(&backend, "kept-twice");
        assert!(
            root.join("story/kept-twice.md").is_file(),
            "the authority holds the document"
        );
        let recorded = backend.divergences();
        assert_eq!(recorded.len(), 1, "{recorded:?}");
        assert_eq!(recorded[0].entity, "story");
        assert_eq!(recorded[0].id, "kept-twice");
        assert!(
            recorded[0].detail.contains("not taking writes"),
            "{}",
            recorded[0].detail
        );
        assert!(root.join(DIVERGENCES).is_file(), "written beside the plan");
    }

    // Process 2: opens the plan and sees the divergence without having been told.
    {
        let replica = SqliteStore::open(&replica_file).expect("a database");
        let backend = HybridBackend::open(
            &root,
            replica,
            POLICY,
            std::iter::empty(),
            at(),
            actor(),
            aep_domain::artifact::LifecycleRegistry::default(),
        )
        .expect("the plan opens");
        assert_eq!(backend.divergences().len(), 1, "remembered from the file");
        assert_eq!(
            backend.len(),
            1,
            "and the story hydrated from the authority"
        );
    }

    // Process 3: the replica is back; catch-up replays and the file goes.
    let replica = SqliteStore::open(&replica_file).expect("a database");
    let outcome = catch_up(&root, replica, POLICY).expect("catch-up runs");
    assert_eq!(outcome.found, 1);
    assert_eq!(outcome.replayed(), 1, "{:?}", outcome.outstanding);
    assert!(outcome.outstanding.is_empty());
    assert!(
        !root.join(DIVERGENCES).exists(),
        "nothing outstanding, no file"
    );
    assert!(read_divergences(&root).expect("readable").is_empty());

    let replica = SqliteStore::open(&replica_file).expect("a database");
    let held = replica
        .load("story", "kept-twice")
        .expect("answers")
        .expect("the replica now holds the story");
    assert_eq!(held.lifecycle_state, "draft");
    assert_eq!(
        replica.events("story", "kept-twice").expect("events").len(),
        1,
        "and its creation event"
    );
}

#[test]
fn a_replica_that_moved_on_its_own_stays_outstanding_for_a_person() {
    // R-108: catch-up merges nothing.
    let root = scratch("conflict");
    let replica_file = root
        .parent()
        .expect("a parent")
        .join("conflict-replica.sqlite3");
    let _ = std::fs::remove_file(&replica_file);
    {
        let backend = HybridBackend::open(
            &root,
            Down(SqliteStore::open(&replica_file).expect("a database")),
            POLICY,
            std::iter::empty(),
            at(),
            actor(),
            aep_domain::artifact::LifecycleRegistry::default(),
        )
        .expect("the plan opens");
        create_story(&backend, "contested");
    }
    // Somebody wrote the same story into the replica by hand, at a revision the authority never
    // produced.
    let mut replica = SqliteStore::open(&replica_file).expect("a database");
    let mut foreign = replica
        .load("story", "contested")
        .expect("answers")
        .unwrap_or_else(|| {
            let local = aep_backend_markdown::provider::MarkdownProvider::open(&root);
            local
                .load("story", "contested")
                .expect("readable")
                .expect("the authority holds it")
        });
    foreign.revision = 7;
    replica
        .commit(
            &Decision {
                instance: foreign,
                events: Vec::new(),
            },
            Expect::Absent,
        )
        .expect("the replica takes the foreign write");

    let outcome = catch_up(&root, replica, POLICY).expect("catch-up runs");
    assert_eq!(outcome.found, 1);
    assert_eq!(outcome.outstanding.len(), 1, "not merged, not dropped");
    assert!(
        outcome.outstanding[0].detail.contains("moved on its own"),
        "{}",
        outcome.outstanding[0].detail
    );
    assert_eq!(
        read_divergences(&root).expect("readable").len(),
        1,
        "written back"
    );
}
