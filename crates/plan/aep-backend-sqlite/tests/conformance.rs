//! What only SQLite can show: the file, read back through a second handle.
//!
//! The sixteen suites and the faulty-backend guard moved to `aep-backend-entity` (wave F, story 2),
//! where they run over this store and over `MemoryStore` through the one adapter. What stays here is
//! the promise only a file makes, and one check that the newtype forwards the whole contract.

use aep_backend_sqlite::{SqliteBackend, STORED_AS};

#[test]
fn the_newtype_forwards_the_whole_contract() {
    // Not a copy of the adapter's conformance test: that one asserts the adapter; this one asserts
    // that `SqliteBackend`'s hand-written forwarding of `CommandService` and `QueryService` reaches
    // every method, which a forwarding mistake would fail and a type alias could not have.
    let backend = SqliteBackend::in_memory().expect("a database");
    let report = aep_conformance::run(&backend, aep_conformance::Level::Full);
    assert!(
        report.passed(),
        "SqliteBackend forwards incompletely; {} of {} checks failed",
        report.failures(),
        report.checks()
    );
}

#[test]
fn what_the_contract_accepted_is_in_the_database() {
    // The whole point of a database backend, and the thing the suites cannot show: they run against
    // the contract, and the contract's state lives in memory while the process runs. This reads the
    // file back through a *second* handle, so nothing in the backend under test is asked whether
    // its own write happened.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
    use aep_domain::time::Timestamp;
    use entity_store::StateProvider as _;

    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("durable.sqlite3");
    let _ = std::fs::remove_file(&path);
    let store = SqliteBackend::open(&path).expect("a database");

    let at = Timestamp::from_epoch_millis(1_700_000_000_000);
    let actor = ActorRef::parse("human:operator").expect("an actor");
    let payload = Command::CreateEntity(CreateEntity {
        entity_type: EntityType::parse("aep.story/v1").expect("a type"),
        locator: EntityLocator::parse("ep://beyond10x/plan/story/one").expect("a locator"),
        data: aep_domain::node::Node::Map(
            [("status".to_owned(), aep_domain::node::Node::from("draft"))]
                .into_iter()
                .collect(),
        ),
    });
    let context = CommandContext::new(
        "req-1".parse().expect("a request id"),
        "key-1".parse().expect("an idempotency key"),
        actor,
        "corr-1".parse().expect("a correlation id"),
        at,
    );
    let envelope = CommandEnvelope::new(
        "cmd-1".parse().expect("a command id"),
        "aep.entity.create/v1",
        payload,
        context,
    );
    let result = block_on(store.execute(envelope)).expect("the command is permitted");
    let created = result.affected.first().expect("one entity was affected");

    let durable = entity_sqlite::SqliteStore::open(&path).expect("the file reopens");
    let held = durable
        .load("aep.entity", &created.id.to_string())
        .expect("the store answers")
        .expect("and holds it");
    assert_eq!(held.lifecycle_state, "draft");
    assert_eq!(held.revision, created.revision.get());
}

#[test]
fn a_second_run_reads_the_first_back_and_continues_past_it() {
    // The refusal this replaces ("the database already holds `…`, and this backend did not write
    // it") existed because `open` read nothing back and minted identities from a fresh counter,
    // so run 2's first entity collided with run 1's. Hydration makes run 2 see run 1's entity under
    // its own identity and mint the next one past it. The file, read through a second handle, holds
    // both.
    use aep_contract::command::CommandService;
    use aep_contract::testing::block_on;
    use entity_store::StateProvider as _;

    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("second-run.sqlite3");
    let _ = std::fs::remove_file(&path);

    let first = SqliteBackend::open(&path).expect("a database");
    let one = block_on(first.execute(create("one", "req-1", "key-1", "cmd-1")))
        .expect("run 1 writes")
        .affected[0]
        .id
        .clone();
    drop(first);

    let second = SqliteBackend::open(&path).expect("the file reopens");
    assert_eq!(second.len(), 1, "run 2 sees what run 1 wrote");
    let two = block_on(second.execute(create("two", "req-2", "key-2", "cmd-2")))
        .expect("run 2 continues")
        .affected[0]
        .id
        .clone();
    assert_ne!(one, two, "a fresh identity, not run 1's reused");
    assert!(second.latched().is_none());

    let durable = entity_sqlite::SqliteStore::open(&path).expect("the file reopens");
    assert_eq!(
        durable.ids(STORED_AS).expect("answers"),
        [one.to_string(), two.to_string()],
        "both runs' entities are in the file"
    );
}

/// One `CreateEntity` command at `name`, with identifiers of its own.
fn create(
    name: &str,
    request: &str,
    key: &str,
    command: &str,
) -> aep_contract::command::CommandEnvelope<aep_domain::command::Command> {
    use aep_contract::command::{CommandContext, CommandEnvelope};
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
    use aep_domain::time::Timestamp;

    let context = CommandContext::new(
        request.parse().expect("a request id"),
        key.parse().expect("an idempotency key"),
        ActorRef::parse("human:operator").expect("an actor"),
        "corr-1".parse().expect("a correlation id"),
        Timestamp::from_epoch_millis(1_700_000_000_000),
    );
    CommandEnvelope::new(
        command.parse().expect("a command id"),
        "aep.entity.create/v1",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").expect("a type"),
            locator: EntityLocator::parse(&format!("ep://beyond10x/plan/story/{name}"))
                .expect("a locator"),
            data: aep_domain::node::Node::Map(
                [("status".to_owned(), aep_domain::node::Node::from("draft"))]
                    .into_iter()
                    .collect(),
            ),
        }),
        context,
    )
}
