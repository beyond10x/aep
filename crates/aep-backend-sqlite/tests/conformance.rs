//! P4 held to the same sixteen suites as every other backend, and to the promise only it makes.

use aep_backend_sqlite::SqliteBackend;

#[test]
fn the_sqlite_backend_conforms() {
    let store = SqliteBackend::in_memory().expect("a database");
    let report = aep_conformance::run(&store, aep_conformance::Level::Full);
    assert!(
        report.passed(),
        "SqliteBackend failed {} of {} checks:\n{}",
        report.failures(),
        report.checks(),
        report
            .failing_suites()
            .flat_map(
                |suite| suite
                    .checks
                    .iter()
                    .filter(|check| !check.passed)
                    .map(|check| format!(
                        "  {}: {}",
                        check.name,
                        check.detail.as_deref().unwrap_or("")
                    ))
            )
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_suites_that_pass_here_catch_a_backend_that_is_wrong() {
    // The guard. A suite that passes proves nothing until something proves the suite can fail — the
    // lesson of 2026-08-26, when a "rolls back both halves" test that asserted a pre-check refusal
    // was cited as evidence for two releases.
    //
    // If this test ever starts passing, `the_sqlite_backend_conforms` has stopped being evidence.
    for fault in [
        aep_conformance::Fault::ReplayApplies,
        aep_conformance::Fault::IgnoreExpectedRevision,
        aep_conformance::Fault::DropRejectionAudit,
    ] {
        let faulty = aep_conformance::FaultyBackend::new(
            SqliteBackend::in_memory().expect("a database"),
            fault,
        );
        let report = aep_conformance::run(&faulty, aep_conformance::Level::Full);
        assert!(
            !report.passed(),
            "the suites passed a backend injected with {fault:?}, so they are not evidence"
        );
    }
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
fn a_second_run_refuses_to_overwrite_what_the_first_one_stored() {
    // The defect this closes destroyed data silently. `MemoryBackend` mints identities from a
    // per-process counter, so run 2's first `CreateEntity` reuses run 1's identity; `persist` reads
    // the durable revision immediately before committing, so the expectation matched by
    // construction and `ON CONFLICT … DO UPDATE` overwrote the row. No conflict, no latch, exit 0.
    //
    // Hydration is P5 and is deliberately deferred. Deferring it does not license destruction.
    use aep_contract::command::CommandService;
    use aep_contract::testing::block_on;
    use entity_store::StateProvider as _;

    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("second-run.sqlite3");
    let _ = std::fs::remove_file(&path);

    let first = SqliteBackend::open(&path).expect("a database");
    block_on(first.execute(create("one", "req-1", "key-1", "cmd-1"))).expect("run 1 writes");
    drop(first);

    let second = SqliteBackend::open(&path).expect("the file reopens");
    let error = block_on(second.execute(create("two", "req-2", "key-2", "cmd-2")))
        .expect_err("run 2 must not write over run 1");
    assert!(
        error.to_string().contains("did not write it"),
        "and it says why: {error}"
    );
    assert!(
        second.latched().is_some(),
        "the backend latches rather than carrying on against a database it disagrees with"
    );

    // What run 1 stored is still there.
    let durable = entity_sqlite::SqliteStore::open(&path).expect("the file reopens");
    let held = durable
        .load("aep.entity", "01MEM0000000000000001")
        .expect("answers")
        .expect("run 1's entity is still there");
    assert_eq!(held.revision, 1);
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
