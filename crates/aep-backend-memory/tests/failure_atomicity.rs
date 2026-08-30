//! Regression tests for the command candidate-state boundary.

use aep_backend_memory::MemoryBackend;
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::consistency::QueryConsistency;
use aep_contract::query::QueryService;
use aep_contract::testing::block_on;
use aep_domain::audit::AuditKind;
use aep_domain::command::{AcceptAdr, Command, CreateEntity};
use aep_domain::entity::{EntityId, EntityRef, EntityRevision, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;

fn envelope(id: &str, key: &str, at: u64, payload: Command) -> CommandEnvelope<Command> {
    let command_type = payload.kind().as_str().to_owned();
    let target = payload.target();
    let expected_revision = payload.expected_revision();
    let mut envelope = CommandEnvelope::new(
        id.parse().expect("command id"),
        command_type,
        payload,
        CommandContext::new(
            format!("request-{id}").parse().expect("request id"),
            key.parse().expect("idempotency key"),
            "human:alice".parse().expect("actor"),
            "correlation-atomicity".parse().expect("correlation"),
            Timestamp::from_epoch_millis(at),
        ),
    );
    envelope.target = target;
    envelope.expected_revision = expected_revision;
    envelope
}

#[test]
fn accepting_an_adr_with_a_missing_superseded_target_changes_only_the_rejection_audit() {
    let backend = MemoryBackend::new();
    let created = block_on(backend.execute(envelope(
        "create-adr",
        "key-create-adr",
        1_000,
        Command::CreateEntity(CreateEntity {
            entity_type: "aep.adr/v1".parse::<EntityType>().expect("entity type"),
            locator: "ep://acme/architecture/adr/0007".parse().expect("locator"),
            data: Node::Map([("status".to_owned(), Node::from("proposed"))].into()),
        }),
    )))
    .expect("ADR is created");
    let adr = created.affected[0].clone();
    let missing = EntityRef::new(EntityId::new("01MISSING00000000001").expect("opaque entity id"));

    let before = backend.with_store(|store| {
        (
            store.entity(&adr.id).cloned(),
            store.relations().cloned().collect::<Vec<_>>(),
            store.history(&adr.id).to_vec(),
            store.events().to_vec(),
            store.audit().len(),
            store.len(),
        )
    });

    let error = block_on(backend.execute(envelope(
        "accept-adr",
        "key-accept-adr",
        2_000,
        Command::AcceptAdr(AcceptAdr {
            adr: adr.clone(),
            supersedes: Some(missing),
        }),
    )))
    .expect_err("the superseded ADR does not exist");
    assert_eq!(error.code(), "not_found");

    let after = backend.with_store(|store| {
        (
            store.entity(&adr.id).cloned(),
            store.relations().cloned().collect::<Vec<_>>(),
            store.history(&adr.id).to_vec(),
            store.events().to_vec(),
            store.audit().len(),
            store.len(),
        )
    });
    assert_eq!(after.0, before.0, "the candidate entity was not published");
    assert_eq!(after.1, before.1, "no relation escaped the candidate");
    assert_eq!(after.2, before.2, "history did not advance");
    assert_eq!(after.3, before.3, "no domain event escaped the candidate");
    assert_eq!(
        after.4,
        before.4 + 1,
        "exactly one refusal audit was appended"
    );
    assert_eq!(after.5, before.5, "the entity set did not change");

    let current = block_on(backend.get(&adr.unversioned(), QueryConsistency::Current))
        .expect("ADR remains readable");
    assert_eq!(current.metadata.revision, EntityRevision::INITIAL);
    let Node::Map(data) = current.data else {
        panic!("ADR body remains a mapping");
    };
    assert_eq!(data.get("status"), Some(&Node::from("proposed")));
    assert_eq!(
        backend.with_store(|store| store.audit().last().map(|record| record.kind)),
        Some(AuditKind::CommandRejected)
    );
}
