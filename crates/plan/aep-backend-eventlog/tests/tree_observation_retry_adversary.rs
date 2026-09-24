//! Adversarial case for N7: an evidence command retried under the same identity, in a second
//! process, is a replay and not a second observation.

use aep_backend_eventlog::{open_tree, prepare_tree, provision_tree, TreeBackend};
use aep_contract::command::CommandService as _;
use aep_contract::testing::block_on;
use aep_domain::artifact::LifecycleRegistry;
use entity_eventlog::EventlogOperationContext;
use std::path::Path;
use time::OffsetDateTime;

fn story_lifecycles() -> LifecycleRegistry {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../artifacts/lifecycles/story.yaml");
    let text = std::fs::read_to_string(path).expect("the story ladder");
    let lifecycle: aep_domain::artifact::ArtifactLifecycle =
        serde_yaml::from_str(&text).expect("it parses");
    let mut registry = LifecycleRegistry::new();
    registry.insert(
        lifecycle.kind.clone().expect("it names its kind"),
        lifecycle,
    );
    registry
}

fn reopen(root: &Path) -> TreeBackend {
    let identity = std::fs::read_to_string(root.join("tenants/planning/identity.json")).unwrap();
    let identity: serde_json::Value = serde_json::from_str(&identity).unwrap();
    open_tree(
        root.to_owned(),
        "scope-planning".into(),
        "planning".into(),
        identity["stream_identity"].as_str().unwrap().to_owned(),
        story_lifecycles(),
    )
    .expect("the store opens")
}

fn provisioned(root: &Path) -> TreeBackend {
    let identity = prepare_tree(root, "planning").expect("a tree store is prepared");
    provision_tree(
        root,
        "scope-planning".into(),
        "planning".into(),
        identity,
        EventlogOperationContext {
            subject: "aep".into(),
            actor: "aep".into(),
            request_id: "provision".into(),
            trace_id: "provision".into(),
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::UNIX_EPOCH,
        },
    )
    .expect("its binding is provisioned");
    reopen(root)
}

fn envelope(
    name: &str,
    payload: aep_domain::command::Command,
) -> aep_contract::command::CommandEnvelope<aep_domain::command::Command> {
    use aep_contract::command::{CommandContext, CommandEnvelope};
    let context = CommandContext::new(
        format!("req-{name}").parse().unwrap(),
        name.parse().unwrap(),
        "human:tester".parse().unwrap(),
        "correlation-1".parse().unwrap(),
        aep_domain::time::Timestamp::from_epoch_millis(1_790_000_000_000),
    );
    CommandEnvelope::new(
        format!("cmd-{name}").parse().unwrap(),
        "test",
        payload,
        context,
    )
}

fn test_results(backend: &TreeBackend, id: &aep_domain::entity::EntityId) -> usize {
    backend
        .with_store(|store| store.events_in_store_order("aep.entity", &id.to_string()))
        .expect("the events read")
        .into_iter()
        .filter(|(_, event)| event.args.get("kind") == Some(&serde_json::json!("test_result")))
        .count()
}

#[test]
fn an_evidence_command_retried_under_its_identity_in_a_second_process_is_one_observation() {
    use aep_domain::command::{Command, CreateEntity, RecordEvidence};
    use aep_domain::entity::{EntityLocator, EntityRef, EntityType};
    use aep_domain::node::Node;
    use entity_store::HistoryProvider as _;

    let directory = tempfile::tempdir().expect("directory");
    let backend = provisioned(directory.path());
    let mut data = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from("draft"));
    data.insert("title".to_owned(), Node::from("Retried"));
    let created = block_on(backend.execute(envelope(
        "create-retried",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").unwrap(),
            locator: EntityLocator::new("planning", "store", "story", "retried").unwrap(),
            data: Node::Map(data),
        }),
    )))
    .expect("a story is created");
    let id = created.affected[0].id.clone();
    let evidence = || {
        envelope(
            "evidence-retried",
            Command::RecordEvidence(RecordEvidence {
                target: EntityRef::new(id.clone()),
                kind: "test_result".to_owned(),
                source: "task check".to_owned(),
                reference: None,
                review: None,
                outcome: None,
            }),
        )
    };
    block_on(backend.execute(evidence())).expect("the evidence is recorded");
    block_on(backend.execute(evidence())).expect("the same command again is a replay");
    drop(backend);
    let second = reopen(directory.path());
    block_on(second.execute(evidence())).expect("a second process replays it too");

    assert_eq!(
        test_results(&second, &id),
        1,
        "an evidence command retried under its identity was recorded more than once"
    );
    let observations = second
        .with_store(|store| store.observations("story", &id.to_string()))
        .expect("the observations read")
        .len();
    assert_eq!(
        observations, 1,
        "a retried evidence command appended a second observation"
    );
}
