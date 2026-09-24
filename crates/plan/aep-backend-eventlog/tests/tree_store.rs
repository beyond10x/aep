//! The contract over a planning store kept as a tree of files: the sixteen suites pass over it, and
//! an artifact of a declared kind is recorded as a typed Entity Runtime entity.

use aep_backend_eventlog::{open_tree, prepare_tree, provision_tree, TreeBackend};
use aep_conformance::Level;
use aep_domain::artifact::LifecycleRegistry;
use entity_eventlog::EventlogOperationContext;
use std::path::Path;
use time::OffsetDateTime;

fn context() -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "aep".into(),
        actor: "aep".into(),
        request_id: "provision".into(),
        trace_id: "provision".into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn provisioned(root: &Path, lifecycles: LifecycleRegistry) -> TreeBackend {
    let identity = prepare_tree(root, "planning").expect("a tree store is prepared");
    provision_tree(
        root,
        "scope-planning".into(),
        "planning".into(),
        identity.clone(),
        context(),
    )
    .expect("its binding is provisioned");
    open_tree(
        root.to_owned(),
        "scope-planning".into(),
        "planning".into(),
        identity,
        lifecycles,
    )
    .expect("it opens")
}

#[test]
fn the_sixteen_suites_pass_over_a_tree_store() {
    let directory = tempfile::tempdir().expect("directory");
    let backend = provisioned(directory.path(), LifecycleRegistry::new());
    let report = aep_conformance::run(&backend, Level::Full);
    let failing: Vec<String> = report
        .failing_suites()
        .flat_map(|suite| {
            suite
                .checks
                .iter()
                .filter(|check| !check.passed)
                .map(|check| {
                    format!(
                        "  {}: {}",
                        check.name,
                        check.detail.as_deref().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        report.passed(),
        "the tree store failed {} of {} checks:\n{}",
        report.failures(),
        report.checks(),
        failing.join("\n")
    );
}

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

#[test]
fn a_story_is_recorded_as_a_typed_entity_and_its_moves_as_the_ladders_operations() {
    use aep_contract::command::CommandService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity, MoveStatus};
    use aep_domain::entity::{EntityLocator, EntityRef, EntityType};
    use aep_domain::node::Node;
    use entity_store::StateProvider;

    let directory = tempfile::tempdir().expect("directory");
    let backend = provisioned(directory.path(), story_lifecycles());
    let mut data = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from("draft"));
    data.insert("title".to_owned(), Node::from("One"));
    data.insert("body".to_owned(), Node::from("The first story."));
    let created = block_on(backend.execute(envelope(
        "create-one",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").unwrap(),
            locator: EntityLocator::new("planning", "store", "story", "one").unwrap(),
            data: Node::Map(data),
        }),
    )))
    .expect("a story is created");
    let id = created.affected[0].id.clone();
    let typed = backend
        .with_store(|store| store.ids("story"))
        .expect("typed ids");
    assert_eq!(
        typed,
        vec![id.to_string()],
        "the story is not a typed story entity"
    );
    let instance = backend
        .with_store(|store| store.load("story", &id.to_string()))
        .expect("read")
        .expect("the typed entity");
    assert_eq!(instance.lifecycle_state, "draft");
    assert_eq!(instance.fields["title"], serde_json::json!("One"));

    block_on(backend.execute(envelope(
        "move-one",
        Command::MoveStatus(MoveStatus {
            target: EntityRef::new(id.clone()),
            to: "proposed".to_owned(),
            expected_revision: None,
            decided_on: None,
        }),
    )))
    .expect("draft → proposed is on the ladder");
    let moved = backend
        .with_store(|store| store.load("story", &id.to_string()))
        .expect("read")
        .expect("the typed entity");
    assert_eq!(
        moved.lifecycle_state, "proposed",
        "the move is not the entity's state"
    );

    drop(backend);
    let identity = std::fs::read_to_string(directory.path().join("tenants/planning/identity.json"))
        .expect("the tenant identity");
    let identity: serde_json::Value = serde_json::from_str(&identity).unwrap();
    let reopened = open_tree(
        directory.path().to_owned(),
        "scope-planning".into(),
        "planning".into(),
        identity["stream_identity"].as_str().unwrap().to_owned(),
        story_lifecycles(),
    )
    .expect("the store reopens");
    let entity = block_on(aep_contract::query::QueryService::get(
        &reopened,
        &EntityRef::new(id),
        aep_contract::consistency::QueryConsistency::default(),
    ))
    .expect("the story reads through the contract");
    let _ = entity;
}

#[test]
fn entity_runtime_refuses_a_story_whose_title_is_not_text() {
    use aep_contract::command::CommandService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{EntityLocator, EntityType};
    use aep_domain::node::Node;
    use entity_store::StateProvider;

    let directory = tempfile::tempdir().expect("directory");
    let backend = provisioned(directory.path(), story_lifecycles());
    let mut data = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from("draft"));
    data.insert("title".to_owned(), Node::from(true));
    let refused = block_on(backend.execute(envelope(
        "create-bad",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").unwrap(),
            locator: EntityLocator::new("planning", "store", "story", "bad").unwrap(),
            data: Node::Map(data),
        }),
    )));
    assert!(
        refused.is_err(),
        "a story whose title is a boolean was recorded: {refused:?}"
    );
    let held = backend
        .with_store(|store| store.ids("story"))
        .expect("typed ids");
    assert!(held.is_empty(), "a refused story left an entity: {held:?}");
}

/// Copy every file under `from` that `into` does not hold, as merging two branches does; a file
/// both hold must be identical.
fn merge_into(from: &Path, into: &Path) {
    for entry in std::fs::read_dir(from).expect("readable") {
        let path = entry.expect("entry").path();
        let target = into.join(path.file_name().expect("name"));
        if path.is_dir() {
            std::fs::create_dir_all(&target).expect("directory");
            merge_into(&path, &target);
        } else if path.file_name().is_some_and(|name| name == ".lock") {
        } else if let Ok(existing) = std::fs::read(&target) {
            assert_eq!(
                existing,
                std::fs::read(&path).unwrap(),
                "{} conflicts",
                target.display()
            );
        } else {
            std::fs::copy(&path, &target).expect("copy");
        }
    }
}

fn create_story(backend: &TreeBackend, name: &str) {
    use aep_contract::command::CommandService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{EntityLocator, EntityType};
    use aep_domain::node::Node;
    let mut data = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from("draft"));
    data.insert("title".to_owned(), Node::from(name));
    block_on(backend.execute(envelope(
        &format!("create-{name}"),
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").unwrap(),
            locator: EntityLocator::new("planning", "store", "story", name).unwrap(),
            data: Node::Map(data),
        }),
    )))
    .expect("a story is created");
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

#[test]
fn two_branches_that_each_created_a_story_merge_into_a_store_with_both() {
    use entity_store::StateProvider;
    let base = tempfile::tempdir().expect("directory");
    drop(provisioned(base.path(), story_lifecycles()));
    let ours = tempfile::tempdir().expect("directory");
    let theirs = tempfile::tempdir().expect("directory");
    merge_into(base.path(), ours.path());
    merge_into(base.path(), theirs.path());
    create_story(&reopen(ours.path()), "ours");
    create_story(&reopen(theirs.path()), "theirs");
    merge_into(theirs.path(), ours.path());
    let merged = reopen(ours.path());
    let stories = merged
        .with_store(|store| store.ids("story"))
        .expect("typed ids");
    assert_eq!(
        stories.len(),
        2,
        "the merged store lost a branch's story: {stories:?}"
    );
    assert!(
        stories.iter().all(|id| id.starts_with("ent-")),
        "an identity was counted rather than derived: {stories:?}"
    );
}

fn run(backend: &TreeBackend, name: &str, payload: aep_domain::command::Command) {
    use aep_contract::command::CommandService;
    block_on(backend.execute(envelope(name, payload)))
        .unwrap_or_else(|error| panic!("`{name}` was refused: {error}"));
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    aep_contract::testing::block_on(future)
}

/// The id of the one story `name` names.
fn story_id(backend: &TreeBackend, name: &str) -> aep_domain::entity::EntityId {
    use aep_domain::entity::EntityLocator;
    block_on(aep_contract::query::QueryService::resolve(
        backend,
        &EntityLocator::new("planning", "store", "story", name).unwrap(),
    ))
    .expect("the story resolves")
}

fn record_test_result(backend: &TreeBackend, name: &str, id: &aep_domain::entity::EntityId) {
    use aep_domain::command::{Command, RecordEvidence};
    use aep_domain::entity::EntityRef;
    run(
        backend,
        name,
        Command::RecordEvidence(RecordEvidence {
            target: EntityRef::new(id.clone()),
            kind: "test_result".to_owned(),
            source: "task check".to_owned(),
            reference: None,
            review: None,
            outcome: None,
        }),
    );
}

fn move_to(backend: &TreeBackend, name: &str, id: &aep_domain::entity::EntityId, to: &str) {
    use aep_domain::command::{Command, MoveStatus};
    use aep_domain::entity::EntityRef;
    run(
        backend,
        name,
        Command::MoveStatus(MoveStatus {
            target: EntityRef::new(id.clone()),
            to: to.to_owned(),
            expected_revision: None,
            decided_on: None,
        }),
    );
}

/// What the typed entity's own history holds: its decisions' revisions, and its observations'.
fn typed_history(backend: &TreeBackend, id: &aep_domain::entity::EntityId) -> (Vec<u64>, Vec<u64>) {
    use entity_store::HistoryProvider;
    backend.with_store(|store| {
        let decisions = store
            .records("story", &id.to_string())
            .expect("the decisions read")
            .into_iter()
            .map(|record| record.record.result.revision)
            .collect();
        let observations = store
            .observations("story", &id.to_string())
            .expect("the observations read")
            .into_iter()
            .map(|observation| observation.revision)
            .collect();
        (decisions, observations)
    })
}

/// How many `test_result` records the history AEP reads counts for the story.
fn test_results(backend: &TreeBackend, id: &aep_domain::entity::EntityId) -> usize {
    backend
        .with_store(|store| store.events_in_store_order("aep.entity", &id.to_string()))
        .expect("the events read")
        .into_iter()
        .filter(|(_, event)| {
            event.args.get("kind") == Some(&serde_json::json!("test_result"))
                && event.args.get("source") == Some(&serde_json::json!("task check"))
        })
        .count()
}

#[test]
fn evidence_on_a_tree_store_is_an_observation_that_leaves_the_revision_where_it_was() {
    let directory = tempfile::tempdir().expect("directory");
    let backend = provisioned(directory.path(), story_lifecycles());
    create_story(&backend, "observed");
    let id = story_id(&backend, "observed");
    let (before, _) = typed_history(&backend, &id);

    record_test_result(&backend, "evidence-observed", &id);

    let (decisions, observations) = typed_history(&backend, &id);
    assert_eq!(
        decisions, before,
        "recording evidence was a decision on the story, which advanced its revision"
    );
    assert_eq!(
        observations,
        vec![*before.last().expect("the creation")],
        "the evidence is not one observation at the story's current revision"
    );
    assert_eq!(
        test_results(&backend, &id),
        1,
        "the history AEP reads does not hold the evidence the observation recorded"
    );
    drop(backend);
    assert_eq!(
        test_results(&reopen(directory.path()), &id),
        1,
        "a second process does not read the evidence the first recorded"
    );
}

#[test]
fn evidence_on_one_branch_and_a_move_on_another_merge_without_forking_the_story() {
    let base = tempfile::tempdir().expect("directory");
    {
        let backend = provisioned(base.path(), story_lifecycles());
        create_story(&backend, "both");
    }
    let ours = tempfile::tempdir().expect("directory");
    let theirs = tempfile::tempdir().expect("directory");
    merge_into(base.path(), ours.path());
    merge_into(base.path(), theirs.path());
    let id = story_id(&reopen(ours.path()), "both");
    record_test_result(&reopen(ours.path()), "evidence-both", &id);
    move_to(&reopen(theirs.path()), "move-both", &id, "proposed");
    merge_into(theirs.path(), ours.path());

    let merged = reopen(ours.path());
    let forked = merged
        .with_store(aep_backend_eventlog::EventlogPlanningStore::forked)
        .expect("the heads read");
    assert!(
        forked.is_empty(),
        "evidence on one branch and a move on the other forked the story: {forked:?}"
    );
    assert_eq!(
        test_results(&merged, &id),
        1,
        "the merge lost the evidence one branch recorded"
    );
    move_to(&merged, "move-both-on", &id, "active");
    let entity = block_on(aep_contract::query::QueryService::get(
        &merged,
        &aep_domain::entity::EntityRef::new(id),
        aep_contract::consistency::QueryConsistency::default(),
    ))
    .expect("the story reads");
    assert_eq!(
        entity
            .data
            .as_map()
            .and_then(|data| data.get("status"))
            .and_then(aep_domain::node::Node::as_text),
        Some("active"),
        "a write after the merge did not land on the one head both branches joined"
    );
}

#[test]
fn evidence_about_an_artifact_of_a_kind_no_lifecycle_declares_is_an_observation_too() {
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{EntityLocator, EntityType};
    use aep_domain::node::Node;
    use entity_store::HistoryProvider;

    let directory = tempfile::tempdir().expect("directory");
    let backend = provisioned(directory.path(), story_lifecycles());
    let mut data = std::collections::BTreeMap::new();
    data.insert("status".to_owned(), Node::from("open"));
    data.insert("title".to_owned(), Node::from("Blocked"));
    run(
        &backend,
        "create-blocker",
        Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.fixture-blocker/v1").unwrap(),
            locator: EntityLocator::new("planning", "store", "fixture-blocker", "blocked").unwrap(),
            data: Node::Map(data),
        }),
    );
    let id = block_on(aep_contract::query::QueryService::resolve(
        &backend,
        &EntityLocator::new("planning", "store", "fixture-blocker", "blocked").unwrap(),
    ))
    .expect("the blocker resolves");
    let decisions = |backend: &TreeBackend| {
        backend
            .with_store(|store| store.records("aep.entity", &id.to_string()))
            .expect("the decisions read")
            .len()
    };
    let before = decisions(&backend);

    record_test_result(&backend, "evidence-blocker", &id);

    assert_eq!(
        decisions(&backend),
        before,
        "evidence about an artifact recorded under the generic type was a decision"
    );
    assert_eq!(
        test_results(&backend, &id),
        1,
        "the history AEP reads does not hold the evidence about the blocker"
    );
}
