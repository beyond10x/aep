//! P5 held to the same sixteen suites as every other backend, under a server, when there is one —
//! and to the promise only a server-backed store makes: two processes writing one artifact.

use std::sync::atomic::{AtomicUsize, Ordering};

use aep_backend_postgres::{PostgresBackend, SessionPostgresBackend};
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity, UpdateEntity};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;

static SCHEMAS: AtomicUsize = AtomicUsize::new(0);

/// The server, or `None` after saying why the test is not running.
fn url() -> Option<String> {
    match std::env::var("ENTITY_POSTGRES_URL") {
        Ok(url) if !url.trim().is_empty() => Some(url),
        _ => {
            eprintln!("skipped: ENTITY_POSTGRES_URL unset, so no server to hold this backend to");
            None
        }
    }
}

fn schema(label: &str) -> String {
    format!(
        "aep_test_{}_{}_{label}",
        std::process::id(),
        SCHEMAS.fetch_add(1, Ordering::Relaxed)
    )
}

fn open(url: &str, schema: &str) -> PostgresBackend {
    PostgresBackend::connect_in_schema(url, schema).unwrap_or_else(|error| {
        panic!("ENTITY_POSTGRES_URL is set and the server refused: {error}")
    })
}

fn drop_schema(url: &str, schema: &str) {
    let mut store = entity_postgres::PostgresStore::connect_no_tls(url).expect("a connection");
    store.drop_schema(schema).expect("dropped");
}

#[test]
fn the_postgres_backend_conforms_and_the_suites_catch_a_faulty_one() {
    let Some(url) = url() else { return };
    let schema = schema("conforms");
    let backend = open(&url, &schema);
    let report = aep_conformance::run(&backend, aep_conformance::Level::Full);
    assert!(
        report.passed(),
        "PostgresBackend failed {} of {} checks:\n{}",
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
    drop(backend);
    drop_schema(&url, &schema);

    for fault in [
        aep_conformance::Fault::ReplayApplies,
        aep_conformance::Fault::IgnoreExpectedRevision,
        aep_conformance::Fault::DropRejectionAudit,
    ] {
        let schema = self::schema("faulty");
        let faulty = aep_conformance::FaultyBackend::new(open(&url, &schema), fault);
        let report = aep_conformance::run(&faulty, aep_conformance::Level::Full);
        assert!(
            !report.passed(),
            "the suites passed a backend injected with {fault:?}, so they are not evidence"
        );
        drop(faulty);
        drop_schema(&url, &schema);
    }
}

fn envelope(command: Command, n: u32, actor: &str) -> CommandEnvelope<Command> {
    let kind = command.kind().as_str();
    CommandEnvelope::new(
        format!("cmd-{n}-{actor}").parse().expect("a command id"),
        kind,
        command,
        CommandContext::new(
            format!("req-{n}-{actor}").parse().expect("a request id"),
            format!("key-{n}-{actor}")
                .parse()
                .expect("an idempotency key"),
            ActorRef::parse(&format!("human:{actor}")).expect("an actor"),
            "corr-race".parse().expect("a correlation id"),
            Timestamp::from_epoch_millis(1_700_000_000_000 + u64::from(n)),
        ),
    )
}

fn retitle(target: &EntityId, title: &str) -> Command {
    Command::UpdateEntity(UpdateEntity {
        target: EntityRef::new(target.clone()),
        changes: [("title".to_owned(), Node::from(title))]
            .into_iter()
            .collect(),
    })
}

#[test]
fn two_processes_writing_one_artifact_resolve_to_one_accepted_and_one_refusal_naming_the_revision()
{
    // Two backends over one database — two processes, in effect — each hydrated at revision 1 of
    // the same artifact, each writing from it. The provider serialises them: one lands, the other
    // is told the revision it lost to. Its detached candidate is never published, so both adapters
    // remain at a state they actually held. Never a silent last-writer-wins.
    let Some(url) = url() else { return };
    let schema = schema("race");
    let first = open(&url, &schema);
    let created = block_on(
        first.execute(envelope(
            Command::CreateEntity(CreateEntity {
                entity_type: EntityType::parse("aep.story/v1").expect("a type"),
                locator: EntityLocator::parse("ep://beyond10x/plan/story/contested")
                    .expect("a locator"),
                data: Node::Map(
                    [
                        ("status".to_owned(), Node::from("draft")),
                        ("title".to_owned(), Node::from("One")),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }),
            1,
            "a",
        )),
    )
    .expect("created");
    let id = created.affected[0].id.clone();

    let second = open(&url, &schema);
    assert_eq!(
        second.len(),
        1,
        "the second process hydrated the first's entity"
    );

    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| block_on(first.execute(envelope(retitle(&id, "A won"), 2, "a"))));
        let b = scope.spawn(|| block_on(second.execute(envelope(retitle(&id, "B won"), 2, "b"))));
        (a.join().expect("thread a"), b.join().expect("thread b"))
    });
    let outcomes = [a, b];
    assert_eq!(
        outcomes.iter().filter(|o| o.is_ok()).count(),
        1,
        "exactly one writer lands: {outcomes:?}"
    );
    let refused = outcomes
        .iter()
        .find_map(|o| o.as_ref().err())
        .expect("one was refused");
    let message = refused.to_string();
    assert!(
        message.contains("expected revision 1") && message.contains("found revision 2"),
        "the loser is told the revision it lost to: {message}"
    );
    assert!(
        first.latched().is_none() && second.latched().is_none(),
        "a refused batch publishes no candidate state and therefore creates no latch"
    );

    // A third process reads what the winner wrote.
    let third = open(&url, &schema);
    let held = block_on(aep_contract::query::QueryService::get(
        &third,
        &EntityRef::new(id),
        aep_contract::QueryConsistency::Current,
    ))
    .expect("held");
    assert_eq!(held.metadata.revision.get(), 2, "one write landed, not two");
    drop((first, second, third));
    drop_schema(&url, &schema);
}

#[test]
fn each_session_command_reads_only_its_dependencies_and_replay_intent() {
    let Some(url) = url() else { return };
    let schema = schema("sessions");
    let first = SessionPostgresBackend::connect_in_schema(&url, &schema).expect("first session");
    let created = block_on(
        first.execute(envelope(
            Command::CreateEntity(CreateEntity {
                entity_type: EntityType::parse("aep.story/v1").expect("a type"),
                locator: EntityLocator::parse("ep://beyond10x/plan/story/session")
                    .expect("a locator"),
                data: Node::Map(
                    [
                        ("status".to_owned(), Node::from("draft")),
                        ("title".to_owned(), Node::from("Before")),
                    ]
                    .into_iter()
                    .collect(),
                ),
            }),
            10,
            "session",
        )),
    )
    .expect("created");
    let id = created.affected[0].id.clone();

    let second = SessionPostgresBackend::connect_in_schema(&url, &schema).expect("second session");
    let update = envelope(retitle(&id, "After"), 11, "session");
    let accepted = block_on(second.execute(update.clone())).expect("updated");
    let replayed = block_on(first.execute(update)).expect("replayed from the applied record");
    assert_eq!(
        replayed.outcome,
        aep_contract::command::CommandOutcome::Replayed
    );
    assert_eq!(replayed.command_id, accepted.command_id);

    let reader = open(&url, &schema);
    let held = block_on(aep_contract::query::QueryService::get(
        &reader,
        &EntityRef::new(id),
        aep_contract::QueryConsistency::Current,
    ))
    .expect("freshly hydrated reader sees the command");
    assert_eq!(held.metadata.revision.get(), 2);
    assert_eq!(
        held.data,
        Node::Map(
            [
                ("status".to_owned(), Node::from("draft")),
                ("title".to_owned(), Node::from("After")),
            ]
            .into_iter()
            .collect()
        )
    );

    drop((first, second, reader));
    drop_schema(&url, &schema);
}
