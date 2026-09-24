//! Independent verification of `FileProjectionPublisher::with_snapshot`'s stated contract.
//!
//! The unit this checks publishes from a capture its caller already holds, on the caller's
//! assertion that nothing has been written to the authority since that capture was taken. Three
//! things are checked here, against the documents the unit wrote about itself:
//!
//! 1. **Identity, on one authority rather than two.** The unit's own identity case builds two
//!    separate fixtures and compares the `*.md` files each publishes. Two fixtures is one
//!    confound — a difference that is the same on both authorities is invisible — and `*.md` is
//!    not the whole stage: `.aep-projection-ownership.json` is written into it too. This compares
//!    **every** staged byte of **one** authority, staged twice at one instant, once through each
//!    code path.
//! 2. **That the seeded path really is reading the capture.** Identity alone cannot tell a
//!    publisher that read the seed from one that quietly re-captured and got the same answer, so
//!    nothing in the suite would go red if `with_snapshot` regressed to taking its own captures.
//!    Changing the authority after the capture separates them.
//! 3. **`publish_current` on a seeded publisher**, which is a combination the types permit, the
//!    documentation does not warn about, and whose `AuthoritySnapshotChanged` refusal compares
//!    two captures that the seeded path never reads.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use aep_contract::query::QueryService;
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity, UpdateEntity};
use aep_domain::entity::{ActorRef, EntityLocator, EntityRef, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use aep_planning_migration::{authority_snapshot_identity, FileProjectionPublisher};
use entity_eventlog::{Authority, EventlogOperationContext};
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    authority_path: PathBuf,
    authority: AuthorityCoordinateV1,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "aep-held-capture-review-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let authority_path = root.join("authority");
        let tenant = "tenant-held-capture";
        let stream_identity =
            aep_backend_eventlog::prepare_file(&authority_path, tenant).expect("prepare authority");
        let authority = AuthorityCoordinateV1 {
            logical_scope: AuthorityValueV1::new("planning-held-capture").expect("scope"),
            tenant: AuthorityValueV1::new(tenant).expect("tenant"),
            stream_identity: AuthorityValueV1::new(stream_identity).expect("stream"),
        };
        aep_backend_eventlog::provision_file(
            &authority_path,
            authority.logical_scope.as_str().to_owned(),
            authority.tenant.as_str().to_owned(),
            authority.stream_identity.as_str().to_owned(),
            context("held-capture:binding", "held-capture"),
        )
        .expect("provision authority binding");
        Self {
            root,
            authority_path,
            authority,
        }
    }

    fn adapter_authority(&self) -> Authority {
        Authority {
            logical_scope: self.authority.logical_scope.as_str().to_owned(),
            tenant: self.authority.tenant.as_str().to_owned(),
            stream_identity: self.authority.stream_identity.as_str().to_owned(),
        }
    }

    fn open(&self) -> aep_backend_eventlog::EventlogBackend {
        aep_backend_eventlog::open(
            self.authority_path.clone(),
            self.authority.logical_scope.as_str().to_owned(),
            self.authority.tenant.as_str().to_owned(),
            self.authority.stream_identity.as_str().to_owned(),
        )
        .expect("open authority")
    }

    fn create_story(&self) {
        let backend = self.open();
        let command = Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").expect("entity type"),
            locator: EntityLocator::parse("ep://planning/store/story/projected").expect("locator"),
            data: Node::Map(BTreeMap::from([
                ("status".to_owned(), Node::from("draft")),
                ("title".to_owned(), Node::from("Projected")),
                ("body".to_owned(), Node::from("# Projected\n")),
            ])),
        });
        let envelope = CommandEnvelope::new(
            "cmd-projected".parse().expect("command"),
            command.kind().as_str(),
            command,
            CommandContext::new(
                "req-projected".parse().expect("request"),
                "key-projected".parse().expect("idempotency key"),
                ActorRef::parse("human:held-capture").expect("actor"),
                "corr-projected".parse().expect("correlation"),
                Timestamp::from_epoch_millis(1_700_000_000_000),
            ),
        );
        block_on(backend.execute(envelope)).expect("create projected story");
    }

    fn update_story_title(&self, title: &str) {
        let suffix = title.to_ascii_lowercase().replace(' ', "-");
        let backend = self.open();
        let target = block_on(backend.resolve(
            &EntityLocator::parse("ep://planning/store/story/projected").expect("locator"),
        ))
        .expect("resolve projected story");
        let command = Command::UpdateEntity(UpdateEntity {
            target: EntityRef::new(target),
            changes: BTreeMap::from([("title".to_owned(), Node::from(title))]),
        });
        let envelope = CommandEnvelope::new(
            format!("cmd-update-{suffix}").parse().expect("command"),
            command.kind().as_str(),
            command,
            CommandContext::new(
                format!("req-update-{suffix}").parse().expect("request"),
                format!("key-update-{suffix}")
                    .parse()
                    .expect("idempotency key"),
                ActorRef::parse("human:held-capture").expect("actor"),
                "corr-update".parse().expect("correlation"),
                Timestamp::from_epoch_millis(1_700_000_000_001),
            ),
        );
        block_on(backend.execute(envelope)).expect("update projected story");
    }

    // ADAPTED for the correction-2 fix, and only here: `with_snapshot` now takes the capture
    // together with the authority it was taken from. Every assertion below is this case's own.
    fn snapshot(&self) -> aep_backend_eventlog::HeldCapture {
        aep_backend_eventlog::capture_held(&self.authority_path, self.adapter_authority())
            .expect("capture complete authority")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn context(request: &str, actor: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: actor.to_owned(),
        actor: actor.to_owned(),
        request_id: request.to_owned(),
        trace_id: request.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// Every file under `root`, keyed by its path relative to `root`. Not only the `*.md` ones.
fn every_file(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("under the root")
                    .to_string_lossy()
                    .into_owned();
                found.insert(relative, fs::read(&path).expect("file"));
            }
        }
    }
    found
}

/// Two stagings of one authority snapshot at once, as two concurrent `validate` runs make, each
/// keep their own directory: neither removes or overwrites what the other is reading.
#[test]
fn two_stagings_of_one_snapshot_at_once_do_not_share_a_directory() {
    let fixture = Fixture::new();
    fixture.create_story();
    let held = fixture.snapshot();
    let (identity, _) = authority_snapshot_identity(&fixture.authority, held.snapshot())
        .expect("authority snapshot identity");
    let root = fixture.root.join("planning-shared");
    let publisher = || {
        FileProjectionPublisher::new(
            fixture.authority_path.clone(),
            fixture.authority.clone(),
            root.clone(),
        )
    };
    let first = publisher().stage(identity).expect("first stage");
    let before = every_file(first.directory());
    let second = publisher().stage(identity).expect("second stage");
    assert_ne!(first.directory(), second.directory());
    assert!(!before.is_empty(), "the first stage rendered nothing");
    assert_eq!(
        every_file(first.directory()),
        before,
        "the second staging changed the first one's files"
    );
}

/// Identity, on **one** authority at **one** instant, over **every** staged byte.
///
/// The unit's own identity case stages two separate fixtures and compares the `*.md` files. This
/// removes both approximations: one authority, one capture, two publishers, and the comparison
/// includes `.aep-projection-ownership.json`, which the unit's case does not read.
#[test]
fn a_held_capture_stages_every_byte_a_fresh_capture_stages() {
    let fixture = Fixture::new();
    fixture.create_story();
    let held = fixture.snapshot();
    let (identity, _) = authority_snapshot_identity(&fixture.authority, held.snapshot())
        .expect("authority snapshot identity");

    let fresh_root = fixture.root.join("planning-fresh");
    let seeded_root = fixture.root.join("planning-seeded");

    let fresh = FileProjectionPublisher::new(
        fixture.authority_path.clone(),
        fixture.authority.clone(),
        fresh_root.clone(),
    );
    let seeded = FileProjectionPublisher::with_snapshot(
        fixture.authority_path.clone(),
        fixture.authority.clone(),
        seeded_root.clone(),
        held,
    );

    let from_fresh = fresh.stage(identity).expect("fresh stage");
    let from_seeded = seeded.stage(identity).expect("seeded stage");

    assert_eq!(
        from_fresh.inventory_digest(),
        from_seeded.inventory_digest(),
        "the inventory digest must not depend on where the capture came from"
    );

    let fresh_files = every_file(from_fresh.directory());
    let seeded_files = every_file(from_seeded.directory());
    assert!(
        fresh_files.keys().any(|name| std::path::Path::new(name)
            .extension()
            .is_some_and(|kind| kind == "md")),
        "the fixture stages at least one document to compare"
    );
    assert!(
        fresh_files.contains_key(".aep-projection-ownership.json"),
        "the stage carries its ownership record, and this comparison reads it: {:?}",
        fresh_files.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        fresh_files.keys().collect::<Vec<_>>(),
        seeded_files.keys().collect::<Vec<_>>(),
        "the same set of staged paths either way"
    );
    assert_eq!(
        fresh_files, seeded_files,
        "every staged byte is identical whichever capture it was rendered from"
    );
}

/// The seeded path answers from the capture it was handed, not from the authority.
///
/// Identity alone cannot see this: a publisher that ignored its seed and re-captured would stage
/// the same bytes and pass every assertion above. This is the case that would go red if
/// `with_snapshot` regressed to `open` and `complete_file_snapshot`, and no case in the unit's
/// own suite does that — the property the unit exists for is otherwise unguarded.
///
/// It is also what the premise costs when it is false: the staged documents are the ones the
/// capture named, nothing refuses, and the caller is told nothing.
#[test]
fn a_seeded_stage_answers_from_the_capture_and_not_from_the_authority() {
    let fixture = Fixture::new();
    fixture.create_story();
    let held = fixture.snapshot();
    let (held_identity, _) =
        authority_snapshot_identity(&fixture.authority, held.snapshot()).expect("held identity");

    fixture.update_story_title("Written After The Capture");

    let seeded_root = fixture.root.join("planning-seeded");
    let seeded = FileProjectionPublisher::with_snapshot(
        fixture.authority_path.clone(),
        fixture.authority.clone(),
        seeded_root.clone(),
        held,
    );
    let staged = seeded.stage(held_identity).expect("seeded stage");

    let staged_bytes = every_file(staged.directory());
    let rendered = staged_bytes
        .iter()
        .filter(|(name, _)| {
            Path::new(name)
                .extension()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("md"))
        })
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .collect::<String>();
    assert!(
        !rendered.is_empty(),
        "the fixture stages at least one document"
    );
    assert!(
        !rendered.contains("Written After The Capture"),
        "a seeded stage that read the authority rather than its seed would carry the later title"
    );
    assert!(
        rendered.contains("Projected"),
        "the seeded stage carries the title the capture named: {rendered}"
    );

    // The control arm, and the reason this case would go red on a revert: the code path
    // `with_snapshot` replaced is `new`, and on this same authority at this same moment it
    // stages the later title. A publisher that took its own captures could not pass the
    // assertions above, so a regression from `with_snapshot` back to `new` is caught here.
    let fresh_root = fixture.root.join("planning-fresh");
    let fresh = FileProjectionPublisher::new(
        fixture.authority_path.clone(),
        fixture.authority.clone(),
        fresh_root.clone(),
    );
    let current = fixture.snapshot();
    let (current_identity, _) = authority_snapshot_identity(&fixture.authority, current.snapshot())
        .expect("current identity");
    let staged = fresh.stage(current_identity).expect("fresh stage");
    let fresh_rendered = every_file(staged.directory())
        .iter()
        .filter(|(name, _)| {
            Path::new(name)
                .extension()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("md"))
        })
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .collect::<String>();
    assert!(
        fresh_rendered.contains("Written After The Capture"),
        "the capture-taking path reads the authority as it now is: {fresh_rendered}"
    );
}

/// A seeded publisher's `publish_current` stamps a snapshot id its documents do not describe.
///
/// `publish_current` exists to publish *the exact current authority* and to refuse with
/// `AuthoritySnapshotChanged` when the authority moved under it. It establishes that by capturing
/// the authority before staging and again after, and comparing the two identities. Both of those
/// captures are fresh. The staging between them is not, once the publisher holds a seed — so the
/// two identities agree, the refusal does not fire, and the watermark is written for a snapshot
/// whose content is not what was published.
///
/// What is asserted is the publisher's own contract, not a preference: a publication reported for
/// authority snapshot S must contain what S contains. Either the call refuses, or the documents
/// it published are the current ones.
#[test]
fn publishing_the_current_authority_from_a_seed_does_not_publish_a_stale_one_silently() {
    let fixture = Fixture::new();
    fixture.create_story();
    let held = fixture.snapshot();

    fixture.update_story_title("Written After The Capture");
    let current = fixture.snapshot();
    let (current_identity, _) = authority_snapshot_identity(&fixture.authority, current.snapshot())
        .expect("current identity");

    let projection_root = fixture.root.join("planning");
    let seeded = FileProjectionPublisher::with_snapshot(
        fixture.authority_path.clone(),
        fixture.authority.clone(),
        projection_root.clone(),
        held,
    );

    let Ok(published) = seeded.publish_current() else {
        // Refusing is a correct answer: the publisher cannot serve "the current authority" out of
        // a capture that no longer describes it.
        return;
    };

    assert_eq!(
        published.authority_snapshot, current_identity,
        "publish_current reports the authority it captured, which is the post-write one"
    );
    let published_bytes = every_file(&projection_root);
    let rendered = published_bytes
        .iter()
        .filter(|(name, _)| {
            Path::new(name)
                .extension()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("md"))
        })
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .collect::<String>();
    assert!(
        !rendered.is_empty(),
        "the fixture publishes at least one document"
    );
    assert!(
        rendered.contains("Written After The Capture"),
        "a publication reported for authority snapshot {} must contain what that snapshot \
         contains; it published the pre-capture state instead, and neither publish_current's \
         AuthoritySnapshotChanged refusal nor anything else objected:\n{rendered}",
        current_identity.0.as_wire()
    );
}
