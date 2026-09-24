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
