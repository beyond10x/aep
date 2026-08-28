//! The adapter held to the sixteen suites over two providers, and shown to fail when it is wrong.
//!
//! Moved here from `aep-backend-sqlite` (wave F, story 2): the suites are a claim about the adapter,
//! and the adapter is one type over any provider. Running them over `MemoryStore` as well as
//! `SqliteStore` is what makes "any provider" a tested sentence rather than a generic parameter.

use aep_backend_entity::EntityBackend;
use aep_conformance::{Fault, FaultyBackend, Level};
use entity_store::Store;

/// Runs the sixteen suites at `Level::Full` and names every failing check.
fn assert_conforms<S: Store>(backend: &EntityBackend<S>, provider: &str) {
    let report = aep_conformance::run(backend, Level::Full);
    assert!(
        report.passed(),
        "EntityBackend<{provider}> failed {} of {} checks:\n{}",
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

/// The guard. A suite that passes proves nothing until something proves the suite can fail — the
/// lesson of 2026-08-26, when a "rolls back both halves" test that asserted a pre-check refusal was
/// cited as evidence for two releases. If this ever starts passing, the test beside it has stopped
/// being evidence.
fn assert_the_suites_catch_a_faulty<S: Store>(fresh: impl Fn() -> EntityBackend<S>) {
    for fault in [
        Fault::ReplayApplies,
        Fault::IgnoreExpectedRevision,
        Fault::DropRejectionAudit,
    ] {
        let faulty = FaultyBackend::new(fresh(), fault);
        let report = aep_conformance::run(&faulty, Level::Full);
        assert!(
            !report.passed(),
            "the suites passed an adapter injected with {fault:?}, so they are not evidence"
        );
    }
}

#[test]
fn the_adapter_over_a_memory_store_conforms() {
    assert_conforms(
        &EntityBackend::over(entity_store::MemoryStore::new()),
        "MemoryStore",
    );
}

#[test]
fn the_adapter_over_a_sqlite_store_conforms() {
    assert_conforms(
        &EntityBackend::over(entity_sqlite::SqliteStore::in_memory().expect("a database")),
        "SqliteStore",
    );
}

#[test]
fn the_suites_that_pass_over_a_memory_store_catch_an_adapter_that_is_wrong() {
    assert_the_suites_catch_a_faulty(|| EntityBackend::over(entity_store::MemoryStore::new()));
}

#[test]
fn the_suites_that_pass_over_a_sqlite_store_catch_an_adapter_that_is_wrong() {
    assert_the_suites_catch_a_faulty(|| {
        EntityBackend::over(entity_sqlite::SqliteStore::in_memory().expect("a database"))
    });
}

#[test]
fn a_latched_adapter_refuses_reads_as_well_as_writes() {
    // `Unwritable` accepts nothing, so the first command applies in memory and fails to land; from
    // then on the adapter must refuse to answer about state that is not durable — reads included,
    // which is the half the latch once missed.
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{ActorRef, EntityLocator, EntityRef, EntityType};
    use aep_domain::time::Timestamp;

    #[derive(Debug, Default)]
    struct Unwritable(entity_store::MemoryStore);
    impl entity_store::StateProvider for Unwritable {
        fn load(
            &self,
            entity: &str,
            id: &str,
        ) -> Result<Option<entity_core::EntityInstance>, entity_store::StoreError> {
            self.0.load(entity, id)
        }
    }
    impl entity_store::EventProvider for Unwritable {
        fn events(
            &self,
            entity: &str,
            id: &str,
        ) -> Result<Vec<entity_core::DomainEvent>, entity_store::StoreError> {
            self.0.events(entity, id)
        }
    }
    impl Store for Unwritable {
        fn commit(
            &mut self,
            _decision: &entity_core::Decision,
            _expect: entity_store::Expect,
        ) -> Result<(), entity_store::StoreError> {
            Err(entity_store::StoreError::Backend(
                "the disk is full".to_owned(),
            ))
        }
    }

    let backend = EntityBackend::over(Unwritable::default());
    let context = CommandContext::new(
        "req-1".parse().expect("a request id"),
        "key-1".parse().expect("an idempotency key"),
        ActorRef::parse("human:operator").expect("an actor"),
        "corr-1".parse().expect("a correlation id"),
        Timestamp::from_epoch_millis(1_700_000_000_000),
    );
    let envelope = CommandEnvelope::new(
        "cmd-1".parse().expect("a command id"),
        "aep.entity.create/v1",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").expect("a type"),
            locator: EntityLocator::parse("ep://beyond10x/plan/story/one").expect("a locator"),
            data: aep_domain::node::Node::Map(
                [("status".to_owned(), aep_domain::node::Node::from("draft"))]
                    .into_iter()
                    .collect(),
            ),
        }),
        context,
    );
    let error = block_on(backend.execute(envelope)).expect_err("the write cannot land");
    assert!(
        error.to_string().contains("the disk is full"),
        "the refusal carries the provider's reason: {error}"
    );
    assert!(backend.latched().is_some(), "and the adapter latches");

    let read = block_on(backend.get(
        &EntityRef::new("01MEM0000000000000001".parse().expect("an id")),
        aep_contract::QueryConsistency::Current,
    ));
    assert!(
        matches!(read, Err(aep_contract::QueryError::Unavailable { .. })),
        "a latched adapter does not answer a read from memory: {read:?}"
    );
}
