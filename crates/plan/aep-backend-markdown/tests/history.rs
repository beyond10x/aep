//! A document with no recorded history still answers the history it has.
//!
//! The Git-native layout keeps no event log beside its documents, so the contract's `history` of a
//! document is what seeding put there. What a move did is read from the document's `transitions`
//! instead (`tests/git_layout.rs`).

use std::path::{Path, PathBuf};

use aep_backend_markdown::backend::MarkdownBackend;
use aep_contract::query::QueryService;
use aep_contract::testing::block_on;
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityRef};
use aep_domain::time::Timestamp;

fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("history")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("planning/story")).expect("a scratch store");
    std::fs::write(
        root.join("planning/story/one.md"),
        "---\nformat: aep.planning-md/1\nid: story:one\nkind: story\nstatus: draft\ntitle: One\nrevision: 1\n---\n\n# One\n",
    )
    .expect("a document");
    root
}

fn open(root: &Path, at: u64) -> MarkdownBackend {
    MarkdownBackend::open_git(
        root.join("planning"),
        root.join("evidence"),
        aep_domain::workspace::Membership::default(),
        Timestamp::from_epoch_millis(at),
        ActorRef::parse("human:operator").expect("an actor"),
        aep_domain::artifact::LifecycleRegistry::default(),
    )
    .expect("the store opens")
}

fn one(store: &MarkdownBackend) -> EntityId {
    block_on(
        store.resolve(&EntityLocator::parse("ep://planning/store/story/one").expect("a locator")),
    )
    .expect("story:one is seeded")
}

#[test]
fn a_document_that_predates_the_provider_still_has_the_history_it_had() {
    // No events at all: the answer is the seeded record, rather than an empty history that would
    // read as "nothing ever happened to this".
    let root = scratch("pre-provider");
    let store = open(&root, 1_700_000_000_000);
    let id = one(&store);
    let history = block_on(store.history(&EntityRef::new(id))).expect("answers");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].revision.get(), 1);
}
