//! Whether `open_with_snapshot`'s completeness check makes a seed *this authority's*.
//!
//! The unit added `validate_seed` in answer to a finding that read, in full: *"for a seeded handle
//! nothing checks the seed is complete **for this authority**"*. What it built checks two fields
//! of `CompleteStoreSnapshot` — `coverage` and `scope`.
//!
//! Neither field identifies a store. `coverage` is the constant `StoreCoverage::CompleteSnapshot`
//! that `EventlogRecordedStore::complete_snapshot` writes into every capture it returns, and
//! `scope` is the *string the caller passed in*, echoed back (`adapter.rs:694-697`). An
//! `Authority` is three values — `logical_scope`, `tenant`, `stream_identity` — and `validate_seed`
//! compares one of them against a label the capture copied from its own caller.
//!
//! So a genuine, complete capture of a **different authority** passes, whenever that authority
//! shares this one's `logical_scope`. That is not an exotic shape here: every cutover in this wave
//! ran `--authority-scope <repository> --authority-tenant planning --authority-new`, so the scope
//! is per-repository, the tenant is the constant `planning`, and **`--authority-new` mints a fresh
//! `stream_identity` on every apply** — the one component of the three that distinguishes two
//! authorities of the same store is the one nothing here compares.
//!
//! These cases build exactly that: two authorities, same `logical_scope`, same `tenant`, different
//! `stream_identity`, different content. They are written against the documents this unit wrote
//! about itself, both of which claim the check delivers more than it does:
//!
//! * `open_with_snapshot`: *"That the capture is complete, and of this authority's logical scope,
//!   is not taken on the caller's word: it is checked here, **so that a seeded handle names every
//!   subject that existed at the capture's instant exactly as a handle that captured for itself
//!   does**."*
//! * `EventlogPlanningStore::retained`: *"**A held capture names every subject that existed at its
//!   instant**, whichever way the capture got here, so the only subject the fall-through can reach
//!   while one is held is a subject that did not exist then ... one handed in by
//!   `open_with_snapshot` is checked to be the same thing before it is seeded."*
//!
//! A handle seeded with another authority's capture names every subject of the *wrong* store, and
//! answers `ids` and `load` out of it while a subject of the right store — which did exist at that
//! instant — falls through to the provider. That is the two-instant read the field's documentation
//! says cannot happen while a capture is held.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
use aep_contract::query::{EntityQuery, QueryService};
use aep_contract::testing::block_on;
use aep_domain::command::{Command, CreateEntity};
use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
use aep_domain::node::Node;
use aep_domain::time::Timestamp;
use entity_eventlog::{Authority, EventlogOperationContext};
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

/// The one `logical_scope` both authorities are provisioned under, as a real cutover's is: the
/// repository name, with `planning` as the tenant, for every store this wave migrated.
const SHARED_SCOPE: &str = "seed-authority-scope";
const SHARED_TENANT: &str = "planning";

struct Fixture {
    root: PathBuf,
    authority: Authority,
}

impl Fixture {
    /// One provisioned file authority under the shared scope and tenant, carrying one story whose
    /// locator key is `name`.
    fn provisioned(name: &str) -> Self {
        let root = std::env::temp_dir()
            .join(format!(
                "aep-seed-authority-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ))
            .join("state");
        let stream_identity =
            aep_backend_eventlog::prepare_file(&root, SHARED_TENANT).expect("prepare authority");
        let authority = Authority {
            logical_scope: SHARED_SCOPE.to_owned(),
            tenant: SHARED_TENANT.to_owned(),
            stream_identity,
        };
        aep_backend_eventlog::provision_file(
            &root,
            authority.logical_scope.clone(),
            authority.tenant.clone(),
            authority.stream_identity.clone(),
            context(&format!("seed-authority-binding-{name}")),
        )
        .expect("provision authority binding");
        let fixture = Self { root, authority };
        fixture.create_story(name);
        fixture
    }

    fn create_story(&self, name: &str) {
        let backend = aep_backend_eventlog::open(
            self.root.clone(),
            self.authority.logical_scope.clone(),
            self.authority.tenant.clone(),
            self.authority.stream_identity.clone(),
        )
        .expect("open authority");
        let command = Command::CreateEntity(CreateEntity {
            entity_type: EntityType::parse("aep.story/v1").expect("entity type"),
            locator: EntityLocator::parse(&format!("ep://planning/store/story/{name}"))
                .expect("locator"),
            data: Node::Map(BTreeMap::from([
                ("status".to_owned(), Node::from("draft")),
                ("title".to_owned(), Node::from(name)),
                ("body".to_owned(), Node::from("# Story\n")),
            ])),
        });
        let envelope = CommandEnvelope::new(
            format!("cmd-{name}").parse().expect("command"),
            command.kind().as_str(),
            command,
            CommandContext::new(
                format!("req-{name}").parse().expect("request"),
                format!("key-{name}").parse().expect("idempotency key"),
                ActorRef::parse("human:seed-authority").expect("actor"),
                format!("corr-{name}").parse().expect("correlation"),
                Timestamp::from_epoch_millis(1_700_000_000_000),
            ),
        );
        block_on(backend.execute(envelope)).expect("create story");
    }

    // ADAPTED for the fix, and only here: `open_with_snapshot` now takes the capture together
    // with the authority it was taken from, because a `CompleteStoreSnapshot` names none of the
    // three values an `Authority` is. Every assertion below is this case's own, unchanged.
    fn snapshot(&self) -> aep_backend_eventlog::HeldCapture {
        aep_backend_eventlog::capture_held(&self.root, self.authority.clone())
            .expect("capture complete authority")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(parent) = self.root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

fn context(request: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "seed-authority".to_owned(),
        actor: "seed-authority".to_owned(),
        request_id: request.to_owned(),
        trace_id: request.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// Every story locator key the handle's own `query` reports.
fn story_keys<B: QueryService>(backend: &B) -> Vec<String> {
    let page = block_on(backend.query(&EntityQuery::default())).expect("query the opened handle");
    let mut keys: Vec<String> = page
        .items
        .iter()
        .map(|envelope| envelope.metadata.locator.key().to_owned())
        .collect();
    keys.sort();
    keys
}

/// Two authorities under one scope and tenant have different stream identities.
///
/// The precondition the two cases below rest on, asserted separately so that a failure in either
/// of them cannot be mistaken for this.
#[test]
fn two_authorities_in_one_scope_differ_only_in_stream_identity() {
    let first = Fixture::provisioned("first");
    let second = Fixture::provisioned("second");

    assert_eq!(
        first.authority.logical_scope, second.authority.logical_scope,
        "the fixture provisions both under one logical scope, as a wave cutover does"
    );
    assert_eq!(
        first.authority.tenant, second.authority.tenant,
        "and under one tenant, which every cutover in this wave spelled `planning`"
    );
    assert_ne!(
        first.authority.stream_identity, second.authority.stream_identity,
        "and the stream identity is what tells them apart"
    );
}

/// A complete capture of another authority in the same logical scope is not a seed for this one.
///
/// `validate_seed` refuses a seed whose `coverage` is not `CompleteSnapshot` and one whose `scope`
/// differs. This capture is a real `CompleteSnapshot` and carries this authority's own scope
/// string, because the two authorities share it — so both checks pass, and a handle on the second
/// authority retains a complete capture of the first.
#[test]
fn a_complete_capture_of_another_authority_is_not_a_seed_for_this_one() {
    let source = Fixture::provisioned("only-in-source");
    let destination = Fixture::provisioned("only-in-destination");
    let foreign = source.snapshot();

    let opened = aep_backend_eventlog::open_with_snapshot(
        destination.root.clone(),
        destination.authority.logical_scope.clone(),
        destination.authority.tenant.clone(),
        destination.authority.stream_identity.clone(),
        foreign,
    );

    let refusal = match opened {
        Ok(_) => String::new(),
        Err(refusal) => refusal,
    };
    assert!(
        !refusal.is_empty(),
        "a capture of stream {source} is not a complete capture of stream {destination}, and \
         `open_with_snapshot` documents its check as establishing that a seeded handle `names \
         every subject that existed at the capture's instant exactly as a handle that captured \
         for itself does`; it accepted the capture instead",
        source = source.authority.stream_identity,
        destination = destination.authority.stream_identity,
    );
}

/// A seeded handle answers reads about the authority it was opened on.
///
/// This asserts the contract rather than a fix: whether `open_with_snapshot` refuses the seed or
/// accepts it and reads the authority, what a caller must never get is the *other* store's
/// subjects. Returning early on a refusal is deliberate — a refusal is a correct answer and this
/// case does not prescribe which correct answer is taken.
#[test]
fn a_seeded_handle_reads_the_authority_it_was_opened_on_and_not_the_one_the_seed_came_from() {
    let source = Fixture::provisioned("only-in-source");
    let destination = Fixture::provisioned("only-in-destination");
    let foreign = source.snapshot();

    let Ok(backend) = aep_backend_eventlog::open_with_snapshot(
        destination.root.clone(),
        destination.authority.logical_scope.clone(),
        destination.authority.tenant.clone(),
        destination.authority.stream_identity.clone(),
        foreign,
    ) else {
        // Refusing is a correct answer: a capture of another authority is not this one's.
        return;
    };

    let keys = story_keys(&backend);
    assert!(
        keys.iter().any(|key| key == "only-in-destination"),
        "a handle opened on the destination authority reports the destination's own story; it \
         reported {keys:?}"
    );
    assert!(
        !keys.iter().any(|key| key == "only-in-source"),
        "and never the story of the authority the seed was taken from, which this handle was not \
         opened on and has no other way to reach; it reported {keys:?}"
    );
}
