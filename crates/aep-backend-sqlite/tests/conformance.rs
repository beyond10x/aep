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
