//! Adversarial check of the stage sweep's lock handshake, through the public staging API.
//!
//! The unit's `Claim::Lost` documents that a stage "removed" between its creation and its lock is
//! lost and the creator tries another name. A sweep running in another pid namespace (or behind
//! `hidepid`) cannot see the creator's pid, so it may lock and remove a just-created, still
//! unlocked stage — exactly what the lock handshake is there to arbitrate. If the removal lands
//! before the creator's `File::open`, the creator must not carry on with an unlocked stage: every
//! stage `stage()` hands back has to be held by its owner, or any later sweep can remove it while
//! it is being written.
//!
//! The foreign sweep is emulated faithfully: it removes a stage only after it has taken the
//! stage's lock and checked the lock is on the directory at the name, which is what
//! `remove_abandoned_stage` does once `/proc` says the pid is gone.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
#[allow(clippy::wildcard_imports)]
use aep_contract::migration::*;
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity};
use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use aep_planning_migration::{authority_snapshot_identity, FileProjectionPublisher};
use entity_eventlog::{Authority, EventlogOperationContext};
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    authority_path: PathBuf,
    projection_root: PathBuf,
    authority: AuthorityCoordinateV1,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "aep-stage-sweep-adversary-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let authority_path = root.join("authority");
        let tenant = "tenant-stage-sweep";
        let stream_identity =
            aep_backend_eventlog::prepare_file(&authority_path, tenant).expect("prepare authority");
        let authority = AuthorityCoordinateV1 {
            logical_scope: AuthorityValueV1::new("planning-stage-sweep").expect("scope"),
            tenant: AuthorityValueV1::new(tenant).expect("tenant"),
            stream_identity: AuthorityValueV1::new(stream_identity).expect("stream"),
        };
        aep_backend_eventlog::provision_file(
            &authority_path,
            authority.logical_scope.as_str().to_owned(),
            authority.tenant.as_str().to_owned(),
            authority.stream_identity.as_str().to_owned(),
            EventlogOperationContext {
                subject: "stage-sweep".to_owned(),
                actor: "stage-sweep".to_owned(),
                request_id: "stage-sweep:binding".to_owned(),
                trace_id: "stage-sweep".to_owned(),
                causation_id: None,
                causation_depth: 0,
                occurred_at: OffsetDateTime::UNIX_EPOCH,
            },
        )
        .expect("provision authority binding");
        Self {
            projection_root: root.join("planning"),
            root,
            authority_path,
            authority,
        }
    }

    /// One story, so every stage has a document to write into it.
    fn create_story(&self) {
        let backend = aep_backend_eventlog::open(
            self.authority_path.clone(),
            self.authority.logical_scope.as_str().to_owned(),
            self.authority.tenant.as_str().to_owned(),
            self.authority.stream_identity.as_str().to_owned(),
        )
        .expect("open authority");
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
                ActorRef::parse("human:stage-sweep").expect("actor"),
                "corr-projected".parse().expect("correlation"),
                Timestamp::from_epoch_millis(1_700_000_000_000),
            ),
        );
        block_on(backend.execute(envelope)).expect("create projected story");
    }

    fn adapter_authority(&self) -> Authority {
        Authority {
            logical_scope: self.authority.logical_scope.as_str().to_owned(),
            tenant: self.authority.tenant.as_str().to_owned(),
            stream_identity: self.authority.stream_identity.as_str().to_owned(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Whether some open file description holds the flock on the directory at `path`.
fn is_locked(path: &Path) -> bool {
    let probe = File::open(path).expect("open the stage to probe its lock");
    matches!(probe.try_lock(), Err(fs::TryLockError::WouldBlock))
}

/// A sweep from another pid namespace: it cannot see the owner's pid, so it removes the stage at
/// `path` whenever it can take the stage's lock on the directory at that name.
fn foreign_sweep(path: &Path) -> bool {
    let Ok(lock) = File::open(path) else {
        return false;
    };
    if lock.try_lock().is_err() {
        return true;
    }
    let (Ok(held), Ok(named)) = (lock.metadata(), fs::symlink_metadata(path)) else {
        return true;
    };
    if named.is_dir() && held.dev() == named.dev() && held.ino() == named.ino() {
        let _ = fs::remove_dir_all(path);
    }
    true
}

#[test]
#[ignore = "stress: about ten minutes; the deterministic case is projection::tests::a_stage_removed_before_its_creator_opens_it_is_lost"]
fn every_stage_handed_back_is_held_by_its_owner_even_when_a_foreign_sweep_races_its_creation() {
    let fixture = Fixture::new();
    fixture.create_story();
    let held =
        aep_backend_eventlog::capture_held(&fixture.authority_path, fixture.adapter_authority())
            .expect("capture the authority");
    let (identity, _) =
        authority_snapshot_identity(&fixture.authority, held.snapshot()).expect("identity");
    let publisher = FileProjectionPublisher::new(
        fixture.authority_path.clone(),
        fixture.authority.clone(),
        fixture.projection_root.clone(),
    );

    // One stage to learn the name's spelling; the counter is this binary's own and only goes up.
    let first = publisher.stage(identity).expect("the first stage");
    let name = first
        .directory()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a stage name")
        .to_owned();
    assert!(is_locked(first.directory()), "an unraced stage is not held");
    drop(first);
    let (stem, counter) = name.rsplit_once('-').expect("<stem>-<counter>");
    let start: u64 = counter.parse().expect("a counter");
    let parent = fixture
        .projection_root
        .parent()
        .expect("parent")
        .to_path_buf();
    let stem = stem.to_owned();

    let stop = Arc::new(AtomicBool::new(false));
    let sweeper = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut next = start + 1;
            while !stop.load(Ordering::Relaxed) {
                let path = parent.join(format!("{stem}-{next}"));
                if foreign_sweep(&path) {
                    next += 1;
                }
            }
        })
    };

    let mut unheld = Vec::new();
    let mut refused = 0_u32;
    for _ in 0..3000 {
        match publisher.stage(identity) {
            Ok(staged) => {
                let directory = staged.directory().to_path_buf();
                if !directory.is_dir() || !is_locked(&directory) {
                    unheld.push(directory);
                }
            }
            Err(_) => refused += 1,
        }
        if !unheld.is_empty() {
            break;
        }
    }
    stop.store(true, Ordering::Relaxed);
    sweeper.join().expect("the sweeper");

    assert!(
        unheld.is_empty(),
        "stage() handed back a stage its owner does not hold, open to any later sweep: {unheld:?} \
         ({refused} stagings refused)"
    );
}
