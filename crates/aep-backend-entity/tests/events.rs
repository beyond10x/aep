//! Wave F, story 3: a command's record reaches the store as events, so the file holds the history.
//!
//! Every assertion about what landed is made through a **second** handle on the same file, or
//! through the provider's own trait — never by asking the backend under test whether its own write
//! happened.

use std::collections::BTreeMap;

use aep_backend_entity::{EntityBackend, METADATA_KEY, STORED_AS};
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity, MoveStatus, UpdateEntity};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_sqlite::SqliteStore;
use entity_store::{EventProvider as _, StateProvider as _, Store};

const AT: u64 = 1_700_000_000_000;

/// One envelope for `command`, with every identifier derived from `n` so a replay is recognisable.
fn envelope(command: Command, n: u32) -> CommandEnvelope<Command> {
    let context = CommandContext::new(
        format!("req-{n}").parse().expect("a request id"),
        format!("key-{n}").parse().expect("an idempotency key"),
        ActorRef::parse("human:operator").expect("an actor"),
        "corr-1".parse().expect("a correlation id"),
        Timestamp::from_epoch_millis(AT + u64::from(n)),
    );
    let kind = command.kind().as_str();
    CommandEnvelope::new(
        format!("cmd-{n}").parse().expect("a command id"),
        kind,
        command,
        context,
    )
}

fn text(pairs: &[(&str, &str)]) -> Node {
    Node::Map(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), Node::from(*value)))
            .collect(),
    )
}

fn create(name: &str) -> Command {
    Command::CreateEntity(CreateEntity {
        entity_type: EntityType::parse("aep.story/v1").expect("a type"),
        locator: EntityLocator::parse(&format!("ep://beyond10x/plan/story/{name}"))
            .expect("a locator"),
        data: text(&[("status", "draft"), ("title", "One")]),
    })
}

fn rename(target: &EntityId, title: &str) -> Command {
    Command::UpdateEntity(UpdateEntity {
        target: EntityRef::new(target.clone()),
        changes: BTreeMap::from([("title".to_owned(), Node::from(title))]),
    })
}

fn propose(target: &EntityId) -> Command {
    Command::MoveStatus(MoveStatus {
        target: EntityRef::new(target.clone()),
        to: "proposed".to_owned(),
        expected_revision: None,
        decided_on: Some(text(&[("recorded", "test_result=1")])),
    })
}

/// Create, rename, propose — three accepted commands against one entity — and its identity.
fn a_story_moved_three_times<S: Store>(backend: &EntityBackend<S>) -> EntityId {
    let created = block_on(backend.execute(envelope(create("one"), 1))).expect("created");
    let id = created.affected[0].id.clone();
    block_on(backend.execute(envelope(rename(&id, "One, renamed"), 2))).expect("renamed");
    block_on(backend.execute(envelope(propose(&id), 3))).expect("proposed");
    id
}

fn fresh_file(name: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.sqlite3"));
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn each_accepted_command_is_one_event_in_the_file_read_through_a_second_handle() {
    let path = fresh_file("events-land");
    let backend =
        EntityBackend::over(SqliteStore::open(&path).expect("a database")).expect("opens");
    let id = a_story_moved_three_times(&backend);
    drop(backend);

    let second = SqliteStore::open(&path).expect("the file reopens");
    let events = second
        .events(STORED_AS, &id.to_string())
        .expect("the store answers");

    let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
    assert_eq!(
        types,
        [
            "aep.entity.create/v1",
            "aep.entity.update/v1",
            "aep.status.move/v1"
        ],
        "one event per accepted command, typed with the contract's own vocabulary"
    );
    let revisions: Vec<u64> = events.iter().map(|e| e.revision).collect();
    assert_eq!(revisions, [1, 2, 3], "in order, one revision each");

    let transitions: Vec<(Option<&str>, &str)> = events
        .iter()
        .map(|e| (e.from_state.as_deref(), e.to_state.as_str()))
        .collect();
    assert_eq!(
        transitions,
        [
            (None, "draft"),
            (Some("draft"), "draft"),
            (Some("draft"), "proposed")
        ],
        "from/to are the status before and after; a creation has no before"
    );

    // Every command also moves the contract's metadata — revision, `updated_at`, provenance — which
    // rides in the reserved `$aep` field so a second process can rebuild the envelope. Beside it,
    // `changed` is exactly the fields the command wrote.
    let mut renamed = events[1].changed.clone();
    assert!(
        renamed.remove(METADATA_KEY).is_some(),
        "the metadata moved too"
    );
    assert_eq!(
        renamed,
        serde_json::json!({ "title": "One, renamed" })
            .as_object()
            .cloned()
            .expect("an object"),
        "an update's `changed` is exactly the fields the command wrote"
    );
    let mut proposed = events[2].changed.clone();
    assert!(
        proposed.remove(METADATA_KEY).is_some(),
        "the metadata moved too"
    );
    assert_eq!(
        proposed,
        serde_json::json!({ "status": "proposed" })
            .as_object()
            .cloned()
            .expect("an object"),
        "a move's `changed` is the status it wrote"
    );

    // The seal — who, when, which flow, what caused it — is in the file, not in a process that has
    // since exited. And the move's account travels with the move.
    let seal = &events[2].payload;
    assert_eq!(seal["causation"], "cmd-3", "causation is the command");
    assert_eq!(seal["correlation"], "corr-1", "correlation is the flow");
    assert_eq!(seal["actor"], "human:operator");
    assert_eq!(seal["recorded_at"], "2023-11-14T22:13:20Z");
    assert_eq!(
        seal["event_id"],
        format!("{STORED_AS}:{id}@3#0"),
        "the id is the runtime's own derived one"
    );
    assert_eq!(
        seal["decided_on"],
        serde_json::json!({ "recorded": "test_result=1" })
    );
    assert_eq!(
        events[0].payload["decided_on"],
        serde_json::Value::Null,
        "a command with no account writes `null`, never an absent key"
    );
}

#[test]
fn a_refused_command_writes_no_event_and_the_file_says_so() {
    let path = fresh_file("events-refused");
    let backend =
        EntityBackend::over(SqliteStore::open(&path).expect("a database")).expect("opens");
    let id = a_story_moved_three_times(&backend);

    // Renaming an entity nobody created is refused by the contract; the refusal is audited in
    // memory and must leave the file exactly as it was (runtime R-84, across the seam).
    let nobody = EntityId::new("01MEM9999999999999999".to_owned()).expect("an id");
    block_on(backend.execute(envelope(rename(&nobody, "ghost"), 4)))
        .expect_err("an entity nobody created cannot be renamed");
    assert!(
        backend.latched().is_none(),
        "a refusal is the contract working, not a fault"
    );
    drop(backend);

    let second = SqliteStore::open(&path).expect("the file reopens");
    assert_eq!(
        second
            .events(STORED_AS, &id.to_string())
            .expect("answers")
            .len(),
        3,
        "the three accepted commands, and nothing for the refused one"
    );
    assert!(
        second
            .events(STORED_AS, &nobody.to_string())
            .expect("answers")
            .is_empty(),
        "and nothing under the identity the refused command named"
    );
}

#[test]
fn a_replayed_command_writes_no_event_as_it_writes_no_instance() {
    let path = fresh_file("events-replayed");
    let backend =
        EntityBackend::over(SqliteStore::open(&path).expect("a database")).expect("opens");
    let id = a_story_moved_three_times(&backend);

    let replay =
        block_on(backend.execute(envelope(propose(&id), 3))).expect("a replay is accepted");
    assert_eq!(
        replay.outcome,
        aep_contract::command::CommandOutcome::Replayed,
        "the fixture reached the state the rule is about"
    );
    drop(backend);

    let second = SqliteStore::open(&path).expect("the file reopens");
    let events = second.events(STORED_AS, &id.to_string()).expect("answers");
    assert_eq!(events.len(), 3, "a replay appends nothing");
    assert_eq!(
        second
            .load(STORED_AS, &id.to_string())
            .expect("answers")
            .expect("held")
            .revision,
        3,
        "and moves nothing"
    );
}

#[test]
fn the_stored_events_fold_back_to_the_stored_instance() {
    // Runtime R-97: a fold reaches no state `execute` would have refused, and reproduces the
    // instance. The definition is the test's: the adapter stores every contract entity as one
    // `aep.entity` type whose statuses are open, so the fold is handed a ladder that declares the
    // three steps this history took — a creation into `draft`, a self-transition (an update), and
    // `draft -> proposed`.
    let definition: entity_core::EntityDefinition = serde_json::from_value(serde_json::json!({
        "entity": STORED_AS,
        "version": 1,
        "schema": { "fields": {}, "additional_fields": true },
        "lifecycle": { "initial": "draft", "states": ["draft", "proposed"] },
        "operations": {
            "update": { "transitions": [{ "from": "draft", "to": "draft" }] },
            "proposed": { "transitions": [{ "from": "draft", "to": "proposed" }] }
        }
    }))
    .expect("the definition parses");

    let backend = EntityBackend::over(entity_store::MemoryStore::new()).expect("opens");
    let id = a_story_moved_three_times(&backend);

    let (events, stored) = backend.with_store(|store| {
        (
            store.events(STORED_AS, &id.to_string()).expect("answers"),
            store
                .load(STORED_AS, &id.to_string())
                .expect("answers")
                .expect("held"),
        )
    });
    assert_eq!(events.len(), 3, "the fixture holds the history it folds");

    let folded = entity_core::rehydrate(&definition, &events).expect("the history folds");
    assert_eq!(
        folded, stored,
        "what the events rebuild is what the store holds: state, revision and every field"
    );
    assert_eq!(folded.lifecycle_state, "proposed");
    assert_eq!(folded.revision, 3);
    assert_eq!(folded.fields["title"], "One, renamed");
}
