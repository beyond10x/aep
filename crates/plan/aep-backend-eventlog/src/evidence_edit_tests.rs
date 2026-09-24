//! Evidence a tree store recorded as an `edit` of the artifact, before evidence became an
//! observation, still reads as the evidence it was.
//!
//! The stores cut over to a tree before that change hold their evidence in the `document` of an
//! `edit` decision. The case writes that exact form through the provider — the action the typed
//! writer used to build for an evidence record — and then reads and writes the store as it is now.

use std::collections::BTreeMap;
use std::path::Path;

use aep_backend_entity::STORED_AS;
use aep_contract::command::CommandService as _;
use aep_contract::testing::block_on;
use aep_domain::artifact::LifecycleRegistry;
use aep_domain::entity::{EntityId, EntityLocator, EntityRef};
use entity_core::DomainEvent;
use entity_executor::{BatchAction, ExecuteRequest};
use entity_store::asynchronous::BatchKey;
use entity_store::{HistoryProvider as _, Recording, StateProvider as _};
use serde_json::{json, Map, Value};

use crate::{
    context_from_recording, open_tree, prepare_tree, provision_tree, provisioning_context,
    RecordedPlanningProvider as _, TreeBackend,
};

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

fn identity(root: &Path) -> String {
    let text = std::fs::read_to_string(root.join("tenants/planning/identity.json"))
        .expect("the tenant identity");
    let value: Value = serde_json::from_str(&text).expect("it is JSON");
    value["stream_identity"]
        .as_str()
        .expect("a stream identity")
        .to_owned()
}

fn open(root: &Path) -> TreeBackend {
    open_tree(
        root.to_owned(),
        "scope-planning".into(),
        "planning".into(),
        identity(root),
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
        provisioning_context("provision"),
    )
    .expect("its binding is provisioned");
    open(root)
}

fn run(backend: &TreeBackend, name: &str, payload: aep_domain::command::Command) {
    use aep_contract::command::{CommandContext, CommandEnvelope};
    let context = CommandContext::new(
        format!("req-{name}").parse().unwrap(),
        name.parse().unwrap(),
        "human:tester".parse().unwrap(),
        "correlation-1".parse().unwrap(),
        aep_domain::time::Timestamp::from_epoch_millis(1_790_000_000_000),
    );
    let envelope = CommandEnvelope::new(
        format!("cmd-{name}").parse().unwrap(),
        "test",
        payload,
        context,
    );
    block_on(backend.execute(envelope))
        .unwrap_or_else(|error| panic!("`{name}` was refused: {error}"));
}

fn move_to(backend: &TreeBackend, name: &str, id: &EntityId, to: &str) {
    run(
        backend,
        name,
        aep_domain::command::Command::MoveStatus(aep_domain::command::MoveStatus {
            target: EntityRef::new(id.clone()),
            to: to.to_owned(),
            expected_revision: None,
            decided_on: None,
        }),
    );
}

fn test_results(backend: &TreeBackend, id: &EntityId) -> usize {
    backend
        .with_store(|store| store.events_in_store_order(STORED_AS, &id.to_string()))
        .expect("the events read")
        .into_iter()
        .filter(|(_, event)| event.args.get("kind") == Some(&json!("test_result")))
        .count()
}

fn decisions(backend: &TreeBackend, id: &EntityId) -> usize {
    backend
        .with_store(|store| store.records(STORED_AS, &id.to_string()))
        .expect("the decisions read")
        .len()
}

/// Writes one evidence record about `id` as the typed writer did before it was an observation:
/// an `edit` whose `document` carries the evidence event, at AEP's unchanged revision.
fn record_evidence_as_an_edit(backend: &TreeBackend, id: &EntityId) {
    backend.with_store(|store| {
        let subject = store
            .subject_of(STORED_AS, &id.to_string())
            .expect("the typed subject");
        let held = store
            .load(STORED_AS, &id.to_string())
            .expect("the story reads")
            .expect("the story is held");
        let (logical, physical) = store
            .revisions_before(&subject, "legacy-evidence")
            .expect("the revisions read");
        assert_eq!(logical, held.revision);
        let event = DomainEvent {
            entity: STORED_AS.to_owned(),
            version: 1,
            id: id.to_string(),
            revision: logical,
            event_type: "aep.evidence.record/v1".to_owned(),
            from_state: Some(held.lifecycle_state.clone()),
            to_state: held.lifecycle_state.clone(),
            changed: Map::new(),
            removed: std::collections::BTreeSet::new(),
            args: json!({ "kind": "test_result", "source": "task check" })
                .as_object()
                .cloned()
                .unwrap(),
            // The seal the contract writes into every event it records.
            payload: json!({
                "event_id": "legacy-evidence",
                "recorded_at": "2026-09-24T00:00:00Z",
                "at": 1_790_000_000_000_u64,
                "correlation": "correlation-1",
                "causation": null,
                "actor": "human:tester",
                "executor": null,
            }),
        };
        let document = json!({
            "lifecycle_state": held.lifecycle_state,
            "fields": held.fields,
            "events": [event],
        });
        let recording = Recording {
            record_id: "legacy-evidence".to_owned(),
            recorded_at: "2026-09-24T00:00:00Z".to_owned(),
            correlation: None,
            causation: None,
            actor: Some("aep".to_owned()),
        };
        store
            .provider
            .batch(
                context_from_recording(&recording),
                BatchKey::Named("legacy-evidence".to_owned()),
                vec![BatchAction::Execute(ExecuteRequest {
                    subject,
                    expected_revision: physical,
                    operation: crate::typed::EDIT_OPERATION.to_owned(),
                    arguments: Value::Object(crate::typed::typed_fields(&held.fields, document)),
                    fulfillments: BTreeMap::new(),
                    recording,
                })],
            )
            .expect("the edit is recorded");
    });
}

#[test]
fn evidence_a_tree_store_recorded_as_an_edit_still_reads_and_new_evidence_is_counted_beside_it() {
    let directory = tempfile::tempdir().expect("directory");
    let id = {
        let backend = provisioned(directory.path());
        let mut data = BTreeMap::new();
        data.insert("status".to_owned(), aep_domain::node::Node::from("draft"));
        data.insert("title".to_owned(), aep_domain::node::Node::from("Legacy"));
        run(
            &backend,
            "create-legacy",
            aep_domain::command::Command::CreateEntity(aep_domain::command::CreateEntity {
                entity_type: aep_domain::entity::EntityType::parse("aep.story/v1").unwrap(),
                locator: EntityLocator::new("planning", "store", "story", "legacy").unwrap(),
                data: aep_domain::node::Node::Map(data),
            }),
        );
        let id = block_on(aep_contract::query::QueryService::resolve(
            &backend,
            &EntityLocator::new("planning", "store", "story", "legacy").unwrap(),
        ))
        .expect("the story resolves");
        move_to(&backend, "move-legacy", &id, "proposed");
        record_evidence_as_an_edit(&backend, &id);
        id
    };

    let reopened = open(directory.path());
    assert_eq!(
        test_results(&reopened, &id),
        1,
        "evidence recorded as an edit is no longer read"
    );
    let before = decisions(&reopened, &id);
    run(
        &reopened,
        "evidence-now",
        aep_domain::command::Command::RecordEvidence(aep_domain::command::RecordEvidence {
            target: EntityRef::new(id.clone()),
            kind: "test_result".to_owned(),
            source: "task check".to_owned(),
            reference: None,
            review: None,
            outcome: None,
        }),
    );
    assert_eq!(
        decisions(&reopened, &id),
        before,
        "evidence recorded now is a decision, not an observation"
    );
    assert_eq!(
        test_results(&reopened, &id),
        2,
        "the edit's evidence and the observation's are not both read"
    );
    move_to(&reopened, "move-legacy-on", &id, "active");
    drop(reopened);
    let again = open(directory.path());
    assert_eq!(
        test_results(&again, &id),
        2,
        "a second process does not read both records"
    );
    let held = again
        .with_store(|store| store.load(STORED_AS, &id.to_string()))
        .expect("the story reads")
        .expect("the story is held");
    assert_eq!(
        (held.lifecycle_state.as_str(), held.revision),
        ("active", 3),
        "a move after an edit-borne and an observed evidence record did not land at AEP's next revision"
    );
}
