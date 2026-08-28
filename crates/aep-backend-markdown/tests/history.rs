//! Wave G, story 3: history and audit are answered from the event log, not from a per-process memory.
//!
//! Two processes over one store. The first moves a story and records evidence about it; the second
//! opens the same directory and must answer the same history — from the log, because the second
//! process's memory holds only what seeding put there.

use std::path::{Path, PathBuf};

use aep_backend_markdown::backend::MarkdownBackend;
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::query::{AuditQuery, QueryService};
use aep_contract::testing::block_on;
use aep_domain::command::{Command, RecordEvidence, UpdateEntity};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef};
use aep_domain::time::Timestamp;

fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("history")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("story")).expect("a scratch store");
    std::fs::write(
        root.join("story/one.md"),
        "---\nformat: aep.planning-md/1\nid: story:one\nkind: story\nstatus: draft\ntitle: One\nrevision: 1\n---\n\n# One\n",
    )
    .expect("a document");
    root
}

fn open(root: &Path, at: u64) -> MarkdownBackend {
    MarkdownBackend::open(
        root,
        std::iter::empty(),
        Timestamp::from_epoch_millis(at),
        ActorRef::parse("human:operator").expect("an actor"),
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .expect("the store opens")
}

fn envelope(command: Command, name: &str, at: u64) -> CommandEnvelope<Command> {
    let kind = command.kind().as_str();
    CommandEnvelope::new(
        format!("cmd-{name}").parse().expect("a command id"),
        kind,
        command,
        CommandContext::new(
            format!("req-{name}").parse().expect("a request id"),
            format!("key-{name}").parse().expect("an idempotency key"),
            ActorRef::parse("human:operator").expect("an actor"),
            "corr-history".parse().expect("a correlation id"),
            Timestamp::from_epoch_millis(at),
        ),
    )
}

fn one(store: &MarkdownBackend) -> EntityId {
    block_on(
        store.resolve(&EntityLocator::parse("ep://planning/store/story/one").expect("a locator")),
    )
    .expect("story:one is seeded")
}

#[test]
fn a_second_process_answers_the_history_the_first_wrote() {
    let root = scratch("two-processes");
    let first = open(&root, 1_700_000_000_000);
    let id = one(&first);
    block_on(
        first.execute(envelope(
            Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(id.clone()),
                changes: [("status".to_owned(), aep_domain::node::Node::from("active"))]
                    .into_iter()
                    .collect(),
            }),
            "move",
            1_700_000_001_000,
        )),
    )
    .expect("permitted");
    block_on(first.execute(envelope(
        Command::RecordEvidence(RecordEvidence {
            target: EntityRef::new(id.clone()),
            kind: "test_result".to_owned(),
            source: "task check".to_owned(),
            reference: None,
        }),
        "evidence",
        1_700_000_002_000,
    )))
    .expect("permitted");
    let history_before = block_on(first.history(&EntityRef::new(id.clone()))).expect("answers");
    assert_eq!(
        history_before.len(),
        1,
        "one revision the log knows: the move"
    );
    assert_eq!(history_before[0].revision.get(), 2);
    assert_eq!(
        history_before[0].at,
        Timestamp::from_epoch_millis(1_700_000_001_000),
        "the instant the command carried, not the seed's"
    );
    assert_eq!(
        history_before[0]
            .command_id
            .as_ref()
            .map(ToString::to_string),
        Some("cmd-move".to_owned())
    );
    drop(first);

    // Process 2: a different seed instant, so anything answered from memory would differ.
    let second = open(&root, 1_800_000_000_000);
    let id = one(&second);
    let history_after = block_on(second.history(&EntityRef::new(id.clone()))).expect("answers");
    assert_eq!(
        history_after, history_before,
        "the history is the log's, not this process's"
    );

    // The audit trail, from the log too: the accepted move and the recorded observation are there
    // with the first process's command ids, beside what this process seeded.
    let audit = block_on(second.audit(&AuditQuery {
        entity: Some(EntityRef::new(id)),
        ..AuditQuery::default()
    }))
    .expect("answers")
    .items;
    let commands: Vec<String> = audit
        .iter()
        .filter_map(|record| record.command_id.as_ref().map(ToString::to_string))
        .collect();
    assert!(commands.contains(&"cmd-move".to_owned()), "{commands:?}");
    assert!(
        commands.contains(&"cmd-evidence".to_owned()),
        "{commands:?}"
    );
    let moved = audit
        .iter()
        .find(|record| {
            record.command_id.as_ref().map(ToString::to_string) == Some("cmd-move".to_owned())
        })
        .expect("the move's record");
    assert_eq!(
        moved
            .change
            .as_ref()
            .and_then(|change| change.after)
            .map(aep_domain::entity::EntityRevision::get),
        Some(2),
        "and it says which revision it produced"
    );
    let observed = audit
        .iter()
        .find(|record| {
            record.command_id.as_ref().map(ToString::to_string) == Some("cmd-evidence".to_owned())
        })
        .expect("the observation's record");
    assert!(observed.change.is_none(), "an observation changed nothing");
}

#[test]
fn a_document_that_predates_the_provider_still_has_the_history_it_had() {
    // No events at all: the answer is what it was before wave G — the seeded record — rather than
    // an empty history that would read as "nothing ever happened to this".
    let root = scratch("pre-provider");
    let store = open(&root, 1_700_000_000_000);
    let id = one(&store);
    let history = block_on(store.history(&EntityRef::new(id))).expect("answers");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].revision.get(), 1);
}
