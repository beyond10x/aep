//! Gap-register `:39`, the provenance half: evidence that names what it is about.
//!
//! The mechanism half closed when a rung could declare `requires:` and a move began refusing
//! without evidence. That left the trust root untouched — `--evidence test_result=1` is a number
//! somebody typed, naming no test, no run and no artifact — and these tests are about the part that
//! moves it: a recorded observation with a subject, a source and an instant, which the move *finds*
//! rather than being told.
//!
//! The load-bearing one is [`evidence_counts_only_for_the_artifact_it_names`]. Everything else here
//! is arrangement; that one is the property.

use std::collections::BTreeMap;
use std::path::PathBuf;

use aep_backend_markdown::journal::{self, Change, Entry, Provenance};
use aep_domain::artifact::{ArtifactId, ArtifactKind, ArtifactStatus, RelationKind};
use aep_domain::evidence::EvidenceKind;

/// Under `target/`, never the system temp directory: this machine's tmpfs drops writes under
/// pressure, and a journal test that loses an append would fail for a reason that is not the code's.
fn scratch(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a writable scratch directory");
    root
}

fn id(name: &str) -> ArtifactId {
    ArtifactId::new(format!("story:{name}")).expect("a well-formed id")
}

fn evidence_about(artifact: &ArtifactId, source: &str) -> Entry {
    Entry {
        at: "2026-08-25T12:00:00Z".to_owned(),
        actor: "tester".to_owned(),
        artifact: artifact.clone(),
        kind: ArtifactKind::Story,
        revision: 1,
        change: Change::Evidence {
            kind: EvidenceKind::TestResult,
            source: source.to_owned(),
            reference: None,
            review: None,
            outcome: None,
        },
    }
}

/// The property the whole provenance half exists for.
///
/// A test result recorded against `story:one` must be worth nothing to `story:two`. Without this,
/// "evidence on hand" is a per-store tally and any artifact can spend another's — which is a
/// stronger-looking version of the bare count it replaced, and a worse one, because it looks
/// attributed.
#[test]
fn evidence_counts_only_for_the_artifact_it_names() {
    let root = scratch("subject");
    let (one, two) = (id("one"), id("two"));

    journal::append(&root, &evidence_about(&one, "task check")).expect("appendable");
    journal::append(&root, &evidence_about(&one, "ci run 8814")).expect("appendable");

    assert_eq!(
        journal::evidence_on_hand(&root, &one),
        BTreeMap::from([(EvidenceKind::TestResult, 2)]),
        "two observations about `one` are two observations about `one`"
    );
    assert_eq!(
        journal::evidence_on_hand(&root, &two),
        BTreeMap::new(),
        "and none of them is about `two`"
    );
}

/// Both origins survive into the total, and the account of which is which survives beside it.
#[test]
fn provenance_totals_without_losing_where_each_part_came_from() {
    let provenance = Provenance {
        recorded: BTreeMap::from([(EvidenceKind::TestResult, 2)]),
        asserted: BTreeMap::from([(EvidenceKind::TestResult, 1), (EvidenceKind::Approval, 1)]),
    };
    assert_eq!(
        provenance.total(),
        BTreeMap::from([(EvidenceKind::TestResult, 3), (EvidenceKind::Approval, 1)]),
        "three test results are three test results for the purpose of `did anybody look`"
    );
    assert!(
        provenance.leans_on_an_assertion(),
        "and the reader must still be able to tell that one of them is a number nobody checked"
    );
    assert!(
        !Provenance {
            recorded: BTreeMap::from([(EvidenceKind::TestResult, 1)]),
            asserted: BTreeMap::new(),
        }
        .leans_on_an_assertion(),
        "a move decided entirely on the record leans on nothing"
    );
}

/// A journal written before provenance existed still reads, and does not acquire a claim.
///
/// Append-only means old lines are never rewritten, so `Moved` must deserialise without
/// `decided_on` — and the value it takes must be *empty*, which reads as "nothing was recorded about
/// how this was decided". Defaulting the other way would have every historical move assert it was
/// evidence-backed.
#[test]
fn a_move_written_before_provenance_existed_claims_nothing() {
    let root = scratch("older");
    // The exact shape 0.19.0 wrote, `decided_on` and all its absence.
    let older = r#"{"at":"2026-08-01T09:00:00Z","actor":"operator","artifact":"story:legacy","kind":"story","revision":2,"change":{"change":"moved","from":"draft","to":"proposed"}}"#;
    std::fs::write(root.join(journal::JOURNAL), format!("{older}\n")).expect("writable");

    let (entries, unreadable) = journal::read(&root);
    assert_eq!(unreadable, 0, "an older entry is not a corrupt one");
    let [entry] = entries.as_slice() else {
        panic!("expected exactly one entry, got {entries:?}");
    };
    let Change::Moved { decided_on, .. } = &entry.change else {
        panic!("expected a move, got {:?}", entry.change);
    };
    assert_eq!(*decided_on, Provenance::default());
    assert!(
        !decided_on.leans_on_an_assertion(),
        "an absent account is not an assertion; it is an absent account"
    );
}

/// A corrupt line is skipped and *counted*, and the entries around it survive.
#[test]
fn one_unreadable_line_does_not_cost_the_history() {
    let root = scratch("corrupt");
    let artifact = id("survivor");
    journal::append(&root, &evidence_about(&artifact, "before")).expect("appendable");
    std::fs::write(
        root.join(journal::JOURNAL),
        std::fs::read_to_string(root.join(journal::JOURNAL)).expect("readable")
            + "half a line, written by a process that died\n",
    )
    .expect("writable");
    journal::append(&root, &evidence_about(&artifact, "after")).expect("appendable");

    let (entries, unreadable) = journal::history(&root, &artifact);
    assert_eq!(unreadable, 1, "the skipped line is counted, not swallowed");
    assert_eq!(
        entries.len(),
        2,
        "and the readable ones are all still there"
    );
    assert_eq!(
        journal::evidence_on_hand(&root, &artifact),
        BTreeMap::from([(EvidenceKind::TestResult, 2)])
    );
}

/// Every kind of change survives a write and a read unchanged.
///
/// **The list is held to the enum by the compiler.** It used to be four of the five variants there
/// were, hand-maintained, and `Change::Related` — the one variant added because an edge had been
/// journalled as `body_replaced` — was never in it. A variant that no test round-trips is a line
/// this crate can write and cannot read back, which is the one thing a closed vocabulary is closed
/// to prevent. The `match` below has no wildcard, so a sixth variant stops this file compiling
/// until it is represented above.
#[test]
fn every_change_round_trips() {
    let root = scratch("roundtrip");
    let artifact = id("round");
    let changes = [
        Change::Created {
            status: ArtifactStatus::Draft,
        },
        Change::Moved {
            from: ArtifactStatus::Draft,
            to: ArtifactStatus::Proposed,
            decided_on: Provenance {
                recorded: BTreeMap::from([(EvidenceKind::TestResult, 1)]),
                asserted: BTreeMap::from([(EvidenceKind::Approval, 2)]),
            },
        },
        Change::Related {
            relation: RelationKind::DependsOn,
            target: "story:the-other-one".to_owned(),
        },
        Change::Unrelated {
            relation: RelationKind::DependsOn,
            target: "story:the-other-one".to_owned(),
        },
        Change::BodyReplaced,
        Change::Evidence {
            kind: EvidenceKind::Approval,
            source: "a person".to_owned(),
            reference: Some("https://example.invalid/8814".to_owned()),
            review: None,
            outcome: None,
        },
    ];
    // One of every variant, and one **only**: the compiler catches a variant the list has never
    // heard of, and the tag count catches one written down twice in place of the one that is
    // missing.
    let mut tags: Vec<&'static str> = changes
        .iter()
        .map(|change| match change {
            Change::Created { .. } => "created",
            Change::Moved { .. } => "moved",
            Change::Related { .. } => "related",
            Change::Unrelated { .. } => "unrelated",
            Change::BodyReplaced => "body_replaced",
            Change::Evidence { .. } => "evidence",
        })
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(
        tags.len(),
        changes.len(),
        "every variant is written down once: {tags:?}"
    );
    for change in &changes {
        journal::append(
            &root,
            &Entry {
                at: "2026-08-25T12:00:00Z".to_owned(),
                actor: "tester".to_owned(),
                artifact: artifact.clone(),
                kind: ArtifactKind::Story,
                revision: 1,
                change: change.clone(),
            },
        )
        .expect("appendable");
    }

    let (entries, unreadable) = journal::history(&root, &artifact);
    assert_eq!(unreadable, 0);
    let read: Vec<Change> = entries.into_iter().map(|entry| entry.change).collect();
    assert_eq!(read, changes.to_vec());
}

#[test]
fn a_status_the_journal_does_not_account_for_is_drift() {
    // The defect, exactly as it happened on 2026-08-26: two epics shipped at
    // `implemented, revision 4` with a journal holding one `created` entry apiece, because the
    // binary that moved them predated the journal and printed every move it failed to record.
    let directory = scratch("reconcile-disagrees");
    let artifact = ArtifactId::new("epic:the-shell").expect("an id");
    journal::append(
        &directory,
        &Entry {
            at: "2026-08-26T00:00:00Z".to_owned(),
            actor: "operator".to_owned(),
            artifact: artifact.clone(),
            kind: ArtifactKind::parse("epic").expect("a kind"),
            revision: 1,
            change: Change::Created {
                status: ArtifactStatus::parse("draft").expect("a status"),
            },
        },
    )
    .expect("the entry lands");

    let held = [(
        artifact,
        (ArtifactStatus::parse("implemented").expect("a status"), 4),
    )]
    .into_iter()
    .collect();

    let drift = journal::reconcile(&directory, &held);
    assert_eq!(drift.len(), 1, "{drift:?}");
    assert!(drift[0].to_string().contains("implemented"), "{}", drift[0]);
    assert!(drift[0].to_string().contains("draft"), "{}", drift[0]);
}

#[test]
fn an_entry_naming_an_artifact_this_store_does_not_hold_is_drift() {
    // The other half, also from that day: six entries in one repository recorded the creation of
    // another repository's artifacts. That store held none of them, and every file was valid.
    let directory = scratch("reconcile-orphan");
    journal::append(
        &directory,
        &Entry {
            at: "2026-08-26T01:37:16Z".to_owned(),
            actor: "operator".to_owned(),
            artifact: ArtifactId::new("story:workspace-manifest").expect("an id"),
            kind: ArtifactKind::parse("story").expect("a kind"),
            revision: 1,
            change: Change::Created {
                status: ArtifactStatus::parse("draft").expect("a status"),
            },
        },
    )
    .expect("the entry lands");

    let drift = journal::reconcile(&directory, &BTreeMap::new());
    assert_eq!(drift.len(), 1, "{drift:?}");
    assert!(drift[0].to_string().contains("never here"), "{}", drift[0]);
}

#[test]
fn an_artifact_with_no_journal_entries_at_all_is_not_drift() {
    // A store predating the journal is a known state, not a defect. Reporting every artifact in one
    // as drifted would make the check useless on the only stores that have the problem.
    let directory = scratch("reconcile-silent");
    let held = [(
        ArtifactId::new("story:old").expect("an id"),
        (ArtifactStatus::parse("implemented").expect("a status"), 9),
    )]
    .into_iter()
    .collect();
    assert!(journal::reconcile(&directory, &held).is_empty());
}

#[test]
fn a_status_is_checked_even_when_the_newest_entry_says_nothing_about_it() {
    // The blind spot: only the **last** entry was examined, and an `evidence` entry carries no
    // status — so four of this repository's own artifacts had their status checked by nothing at
    // all, because recording evidence was the last thing that happened to each of them.
    let directory = scratch("reconcile-silent-newest");
    let artifact = ArtifactId::new("story:quiet").expect("an id");
    let kind = ArtifactKind::parse("story").expect("a kind");

    journal::append(
        &directory,
        &Entry {
            at: "2026-08-26T00:00:00Z".to_owned(),
            actor: "operator".to_owned(),
            artifact: artifact.clone(),
            kind: kind.clone(),
            revision: 1,
            change: Change::Created {
                status: ArtifactStatus::parse("draft").expect("a status"),
            },
        },
    )
    .expect("the entry lands");
    journal::append(
        &directory,
        &Entry {
            at: "2026-08-26T00:00:01Z".to_owned(),
            actor: "operator".to_owned(),
            artifact: artifact.clone(),
            kind,
            revision: 1,
            change: Change::Evidence {
                kind: EvidenceKind::TestResult,
                source: "task check".to_owned(),
                reference: None,
                review: None,
                outcome: None,
            },
        },
    )
    .expect("the entry lands");

    // The file claims a status the journal never recorded a move to.
    let held = [(
        artifact,
        (ArtifactStatus::parse("implemented").expect("a status"), 1),
    )]
    .into_iter()
    .collect();

    let drift = journal::reconcile(&directory, &held);
    assert_eq!(drift.len(), 1, "{drift:?}");
    assert!(drift[0].to_string().contains("implemented"), "{}", drift[0]);
    assert!(drift[0].to_string().contains("draft"), "{}", drift[0]);
}

/// A move made by a driven session reads as the agent's, not as the operator's.
///
/// **This is the end of the chain, and the only place a reader ever sees it.** `protocol drive`
/// declares `AEP_ACTOR=agent:<execution id>` on the session it launches, `command_actor()` parses
/// that into the actor the store is opened with, and the journal is where the answer lands —
/// `protocol artifact history` and `protocol artifact explain` print this field. Before it, every
/// entry in this store said `human:<$USER>` whoever made the move, so an agent's
/// `protocol artifact move <spec> approved` was indistinguishable from the operator's own: the
/// *accepts any caller* gap `story:attested-approver` names from the store's side.
///
/// It goes through `MarkdownBackend` rather than appending an entry directly, because appending
/// one would assert only that a struct round-trips. What has to hold is that the actor the store
/// was **opened with** is the actor the record carries.
#[test]
fn a_move_made_by_a_driven_session_is_journalled_as_the_agents_and_not_the_operators() {
    use aep_backend_markdown::backend::MarkdownBackend;
    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::query::QueryService;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, UpdateEntity};
    use aep_domain::entity::{ActorRef, EntityLocator, EntityRef};
    use aep_domain::time::Timestamp;

    let root = scratch("driven-actor");
    std::fs::create_dir_all(root.join("story")).expect("a scratch store");
    std::fs::write(
        root.join("story/one.md"),
        "---\nformat: aep.planning-md/1\nid: story:one\nkind: story\nstatus: draft\ntitle: One\nrevision: 1\n---\n\n# One\n",
    )
    .expect("a document");

    let session = ActorRef::parse("agent:W4-3.1").expect("an actor");
    let store = MarkdownBackend::open(
        &root,
        std::iter::empty(),
        Timestamp::from_epoch_millis(1_700_000_000_000),
        session.clone(),
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .expect("the store opens");
    let target = block_on(
        store.resolve(&EntityLocator::parse("ep://planning/store/story/one").expect("a locator")),
    )
    .expect("story:one is seeded");
    block_on(
        store.execute(CommandEnvelope::new(
            "cmd-move".parse().expect("a command id"),
            "aep.entity.update/v1",
            Command::UpdateEntity(UpdateEntity {
                target: EntityRef::new(target),
                changes: [("status".to_owned(), aep_domain::node::Node::from("active"))]
                    .into_iter()
                    .collect(),
            }),
            CommandContext::new(
                "req-move".parse().expect("a request id"),
                "key-move".parse().expect("an idempotency key"),
                session.clone(),
                "corr-driven".parse().expect("a correlation id"),
                Timestamp::from_epoch_millis(1_700_000_001_000),
            ),
        )),
    )
    .expect("the move is accepted");

    let (entries, unreadable) = journal::history(&root, &id("one"));
    assert_eq!(unreadable, 0, "every line the move wrote is readable");
    let moved = entries
        .iter()
        .find(|entry| matches!(entry.change, Change::Moved { .. }))
        .expect("the move is in the journal, or there is nothing to be attributed");
    assert_eq!(
        moved.actor, "agent:W4-3.1",
        "the journal says who moved it, and a driven move is the run's"
    );
    assert_eq!(
        ActorRef::parse(&moved.actor).expect("what the journal holds is an actor"),
        session,
        "and it reads back as the same actor the store was opened with"
    );
    assert!(
        !moved.actor.starts_with("human:"),
        "the defect this closes: every write said `human:<$USER>` whoever made it"
    );
}
