//! The Git-native layout (`aep.project/5`): the documents are the authority and there is no
//! journal. A move is a transition appended to the document it moved; an observation is one
//! evidence file; `journal.jsonl` never appears.

use std::path::{Path, PathBuf};

use aep_backend_markdown::backend::MarkdownBackend;
use aep_backend_markdown::journal::{self, Change, Entry};
use aep_backend_markdown::provider::MarkdownProvider;
use aep_backend_markdown::{PlanningDocument, PlanningFormat};
use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::query::QueryService;
use aep_contract::testing::block_on;
use aep_domain::artifact::ArtifactId;
use aep_domain::command::{Command, RecordEvidence, UpdateEntity};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef};
use aep_domain::evidence::EvidenceKind;
use aep_domain::time::Timestamp;
use entity_store::EventProvider;

const DOCUMENT: &str = "---\nformat: aep.planning-md/3\nid: story:one\nkind: story\nstatus: draft\ntitle: One\nrevision: 1\n---\n\n# One\n";

struct Store {
    root: PathBuf,
    planning: PathBuf,
    evidence: PathBuf,
}

fn scratch(name: &str) -> Store {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("git-layout")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    let planning = root.join("planning");
    let evidence = root.join("evidence");
    std::fs::create_dir_all(planning.join("story")).expect("a scratch store");
    std::fs::write(planning.join("story/one.md"), DOCUMENT).expect("a document");
    Store {
        root,
        planning,
        evidence,
    }
}

fn open(store: &Store) -> MarkdownBackend {
    MarkdownBackend::open_git(
        &store.planning,
        &store.evidence,
        aep_domain::workspace::Membership::default(),
        Timestamp::from_epoch_millis(1_700_000_000_000),
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
            "corr-git".parse().expect("a correlation id"),
            Timestamp::from_epoch_millis(at),
        ),
    )
}

fn one(backend: &MarkdownBackend) -> EntityId {
    block_on(
        backend.resolve(&EntityLocator::parse("ep://planning/store/story/one").expect("a locator")),
    )
    .expect("story:one is seeded")
}

fn artifact() -> ArtifactId {
    ArtifactId::new("story:one").expect("an id")
}

fn move_to(backend: &MarkdownBackend, status: &str, name: &str, at: u64) {
    let id = one(backend);
    block_on(
        backend.execute(envelope(
            Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(id),
                changes: [("status".to_owned(), aep_domain::node::Node::from(status))]
                    .into_iter()
                    .collect(),
            }),
            name,
            at,
        )),
    )
    .expect("permitted");
}

fn record_evidence(backend: &MarkdownBackend, name: &str, at: u64) {
    let id = one(backend);
    block_on(backend.execute(envelope(
        Command::RecordEvidence(RecordEvidence {
            target: EntityRef::new(id),
            kind: "test_result".to_owned(),
            source: "task check".to_owned(),
            reference: Some("run-1".to_owned()),
            review: None,
            outcome: None,
        }),
        name,
        at,
    )))
    .expect("permitted");
}

/// Every file under `directory`, relative, sorted — what a `git status` would list.
fn files(directory: &Path) -> Vec<String> {
    fn walk(base: &Path, directory: &Path, found: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                found.push(
                    path.strip_prefix(base)
                        .expect("under the base")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    let mut found = Vec::new();
    walk(directory, directory, &mut found);
    found.sort();
    found
}

fn modified(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .expect("a file")
}

fn document(store: &Store) -> PlanningDocument {
    let text = std::fs::read_to_string(store.planning.join("story/one.md")).expect("the document");
    PlanningDocument::parse(&text, None).expect("the document parses")
}

#[test]
fn a_move_writes_only_its_document_and_appends_one_transition() {
    let store = scratch("one-move");
    let backend = open(&store);
    let before = files(&store.root);

    move_to(&backend, "active", "move", 1_700_000_001_000);

    assert_eq!(
        files(&store.root),
        before,
        "a move adds no file: no journal, no evidence, no sidecar — only the document changes"
    );
    let moved = document(&store);
    assert_eq!(moved.frontmatter.format, PlanningFormat::V3);
    assert_eq!(moved.frontmatter.status.as_str(), "active");
    assert_eq!(moved.frontmatter.revision, 2);
    assert_eq!(
        moved.frontmatter.transitions.len(),
        1,
        "the move is recorded in the document it moved, once"
    );
    let transition = &moved.frontmatter.transitions[0];
    assert_eq!(
        (transition.from.as_str(), transition.to.as_str()),
        ("draft", "active")
    );
    assert_eq!(transition.revision, 2, "the revision the move produced");
    assert_eq!(transition.actor, "human:operator");
    assert!(
        transition.at.starts_with("2023-11-14T22:13:21"),
        "the instant the command carried, not the seed's: {}",
        transition.at
    );
    let text = std::fs::read_to_string(store.planning.join("story/one.md")).expect("the document");
    assert!(
        text.contains("\ntransitions:\n- {from: \"draft\", to: \"active\", "),
        "one flow mapping per transition line:\n{text}"
    );
    assert!(!store.planning.join(journal::LEGACY_JOURNAL).exists());
}

#[test]
fn three_moves_give_three_transitions_and_three_moved_entries_in_history() {
    let store = scratch("three-moves");
    let backend = open(&store);
    move_to(&backend, "active", "one", 1_700_000_001_000);
    move_to(&backend, "implemented", "two", 1_700_000_002_000);
    // A second process: the transitions come from the file, so it appends rather than replaces.
    drop(backend);
    let backend = open(&store);
    move_to(&backend, "active", "three", 1_700_000_003_000);

    let moved = document(&store);
    assert_eq!(
        moved
            .frontmatter
            .transitions
            .iter()
            .map(|transition| (
                transition.from.as_str().to_owned(),
                transition.to.as_str().to_owned(),
                transition.revision
            ))
            .collect::<Vec<_>>(),
        vec![
            ("draft".to_owned(), "active".to_owned(), 2),
            ("active".to_owned(), "implemented".to_owned(), 3),
            ("implemented".to_owned(), "active".to_owned(), 4),
        ],
        "each move appends; none replaces an earlier one"
    );
    assert_eq!(moved.frontmatter.revision, 4);

    let (history, unreadable) = journal::history_git(&store.planning, &store.evidence, &artifact());
    assert_eq!(unreadable, 0);
    assert_eq!(history.len(), 3, "{history:?}");
    assert!(history
        .iter()
        .all(|entry| matches!(entry.change, Change::Moved { .. })));
    assert_eq!(
        history
            .iter()
            .map(|entry| entry.revision)
            .collect::<Vec<_>>(),
        vec![2, 3, 4],
        "oldest first"
    );
    let (all, _) = journal::read_git(&store.planning, &store.evidence);
    assert_eq!(
        all, history,
        "one artifact's history is the whole store's here"
    );
    assert!(!store.planning.join(journal::LEGACY_JOURNAL).exists());
}

#[test]
fn recording_evidence_writes_one_evidence_file_and_it_is_counted() {
    let store = scratch("evidence");
    let backend = open(&store);
    let document_path = store.planning.join("story/one.md");
    let before = std::fs::read(&document_path).expect("the document");
    let stamp = modified(&document_path);

    record_evidence(&backend, "evidence", 1_700_000_004_000);

    let written = files(&store.evidence);
    assert_eq!(written.len(), 1, "one observation, one file: {written:?}");
    assert!(
        written[0].starts_with("story/one/20231114T221324"),
        "filed under the artifact, named by its instant: {written:?}"
    );
    assert_eq!(
        std::fs::read(&document_path).expect("the document"),
        before,
        "an observation changes nothing in the document"
    );
    assert_eq!(modified(&document_path), stamp, "and does not rewrite it");

    let counted = journal::evidence_on_hand_git(&store.planning, &store.evidence, &artifact());
    assert_eq!(counted.get(&EvidenceKind::TestResult), Some(&1));
    let (history, _) = journal::history_git(&store.planning, &store.evidence, &artifact());
    assert!(
        matches!(&history[..], [Entry { change: Change::Evidence { reference: Some(reference), .. }, .. }] if reference == "run-1"),
        "{history:?}"
    );
    assert!(!store.planning.join(journal::LEGACY_JOURNAL).exists());
    assert!(!store.root.join(journal::LEGACY_JOURNAL).exists());
}

#[test]
fn the_git_layout_keeps_no_event_log() {
    let store = scratch("no-log");
    let backend = open(&store);
    move_to(&backend, "active", "move", 1_700_000_001_000);
    record_evidence(&backend, "evidence", 1_700_000_002_000);
    // A journal somebody left behind is not read either.
    std::fs::write(store.planning.join(journal::LEGACY_JOURNAL), "not a log\n")
        .expect("a stray file");
    let provider = MarkdownProvider::open_git(&store.planning, &store.evidence);
    assert_eq!(
        provider.events("story", "one").expect("answers"),
        Vec::new(),
        "no events are answered in the Git layout"
    );
}

#[test]
fn writing_the_same_evidence_twice_is_a_no_op_and_a_different_file_is_refused() {
    let store = scratch("evidence-twice");
    let entry = Entry {
        at: "2026-09-28T10:04:11Z".to_owned(),
        actor: "human:operator".to_owned(),
        artifact: artifact(),
        kind: "story".parse().expect("a kind"),
        revision: 1,
        change: Change::Evidence {
            kind: EvidenceKind::TestResult,
            source: "task check".to_owned(),
            reference: None,
            review: None,
            outcome: None,
        },
    };
    let first = journal::write_evidence(&store.evidence, &entry).expect("written");
    let again = journal::write_evidence(&store.evidence, &entry).expect("identical is a no-op");
    assert_eq!(first, again);
    assert_eq!(files(&store.evidence).len(), 1);
    let text = std::fs::read_to_string(&first).expect("the record");
    assert!(text.ends_with("}\n"), "pretty JSON with a trailing newline");
    assert_eq!(
        serde_json::from_str::<Entry>(&text).expect("the record reads back"),
        entry
    );

    std::fs::write(&first, "{}\n").expect("tampered");
    let refused =
        journal::write_evidence(&store.evidence, &entry).expect_err("a record is never rewritten");
    assert_eq!(refused.kind(), std::io::ErrorKind::AlreadyExists);
}

/// A pending batch is completed before a read, and completing it a second time — the process
/// stopped after the document landed and before the intent was removed — does not append the
/// transition again.
#[test]
fn an_interrupted_batch_is_completed_once_and_its_transition_is_not_doubled() {
    use entity_store::StateProvider;

    let store = scratch("pending");
    let provider = MarkdownProvider::open_git(&store.planning, &store.evidence);
    let mut instance = provider
        .load("story", "one")
        .expect("reads")
        .expect("the document is there");
    instance.lifecycle_state = "active".to_owned();
    instance.revision = 2;
    let event: entity_core::DomainEvent = serde_json::from_value(serde_json::json!({
        "entity": "story",
        "version": 1,
        "id": "one",
        "revision": 2,
        "type": "update_entity",
        "from_state": "draft",
        "to_state": "active",
        "changed": {},
        "args": {},
        "payload": {
            "recorded_at": "2026-09-28T10:04:11Z",
            "actor": "human:operator",
            "change": { "change": "moved", "from": "draft", "to": "active" }
        }
    }))
    .expect("an event");
    let decision = entity_core::Decision::legacy_import(instance, vec![event]);
    let pending = serde_json::json!({
        "commits": [{ "decision": decision, "expect": { "revision": 1 } }]
    });
    let intent = store
        .planning
        .join(aep_backend_markdown::provider::PENDING_BATCH);

    for attempt in ["before anything landed", "after the document landed"] {
        std::fs::write(&intent, serde_json::to_vec(&pending).expect("serialises")).expect("intent");
        provider
            .load("story", "one")
            .expect("the read completes the batch");
        assert!(
            !intent.exists(),
            "{attempt}: the intent is removed once it landed"
        );
        let moved = document(&store);
        assert_eq!(moved.frontmatter.status.as_str(), "active", "{attempt}");
        assert_eq!(moved.frontmatter.revision, 2, "{attempt}");
        assert_eq!(
            moved.frontmatter.transitions.len(),
            1,
            "{attempt}: one move, one transition"
        );
    }
    assert!(!store.planning.join(journal::LEGACY_JOURNAL).exists());
}

/// Two records made in the same second come back in the order they were made, whatever their
/// digests: every other backend answers in recording order, and a history that ordered by hash
/// disagreed with them.
#[test]
fn evidence_recorded_in_one_second_is_read_back_in_recording_order() {
    let store = scratch("same-second");
    let record = |source: &str| Entry {
        at: "2026-09-28T10:04:11Z".to_owned(),
        actor: "human:operator".to_owned(),
        artifact: artifact(),
        kind: "story".parse().expect("a kind"),
        revision: 1,
        change: Change::Evidence {
            kind: EvidenceKind::TestResult,
            source: source.to_owned(),
            reference: None,
            review: None,
            outcome: None,
        },
    };
    // Enough records that some later one must hash below an earlier one.
    let sources: Vec<String> = (0..12).map(|n| format!("run {n}")).collect();
    for source in &sources {
        journal::write_evidence(&store.evidence, &record(source)).expect("written");
    }
    let (history, unreadable) = journal::history_git(&store.planning, &store.evidence, &artifact());
    assert_eq!(unreadable, 0);
    let read: Vec<String> = history
        .iter()
        .filter_map(|entry| match &entry.change {
            Change::Evidence { source, .. } => Some(source.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        read, sources,
        "same-second records keep their recording order"
    );
}
