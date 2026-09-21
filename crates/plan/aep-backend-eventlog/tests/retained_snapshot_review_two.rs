//! What an open handle answers after somebody else writes to the same authority.
//!
//! Written for the second independent verification pass over
//! `story:eventlog-store-hydrates-from-one-snapshot`. Everything here goes through the crate's
//! **public** surface — `prepare_file`, `provision_file`, `write_file_control`, `open` — so it
//! compiles and runs unchanged at the change's base commit `1a5ceda4`, which is how origin is
//! settled for the staleness question rather than by reading the diff.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use entity_eventlog::{Authority, EventlogOperationContext};
use entity_store::StateProvider as _;
use serde_json::json;
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

const SCOPE: &str = "planning-review-two";
const TENANT: &str = "tenant-review-two";

fn context(request: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "retained-snapshot-review-two".to_owned(),
        actor: "retained-snapshot-review-two".to_owned(),
        request_id: request.to_owned(),
        trace_id: "retained-snapshot-review-two".to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// One provisioned file authority nothing else in this binary touches.
fn authority() -> (PathBuf, Authority) {
    let root = std::env::temp_dir().join(format!(
        "aep-retained-snapshot-review-two-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("an authority root");
    let stream_identity =
        aep_backend_eventlog::prepare_file(&root, TENANT).expect("the provider prepares one");
    let authority = Authority {
        logical_scope: SCOPE.to_owned(),
        tenant: TENANT.to_owned(),
        stream_identity: stream_identity.clone(),
    };
    aep_backend_eventlog::provision_file(
        &root,
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        context("review-two-binding"),
    )
    .expect("the binding is provisioned");
    (root, authority)
}

/// One control document, committed through a handle of its own — a second writer.
fn write_control(root: &Path, authority: &Authority, identity: &str, marker: &str) {
    aep_backend_eventlog::write_file_control(
        root.to_path_buf(),
        authority.clone(),
        aep_backend_eventlog::INVOCATION_AS,
        identity.to_owned(),
        identity.to_owned(),
        json!({
            "lifecycle_state": "recorded",
            "fields": { "marker": marker },
            "events": [],
        }),
        None,
        context(identity),
    )
    .expect("the second writer commits");
}

/// A handle that only reads answers about the authority as it is, not as it was when it opened.
///
/// `write_file_control` opens its own bridge (`lib.rs:364`), so the write below is a second
/// writer on the same authority, exactly as a second process would be. At `1a5ceda4` every
/// `load` through the opened backend was one fresh capture, so the second read saw the write.
#[test]
fn a_backend_that_only_reads_answers_about_the_authority_as_it_is_now() {
    let (root, authority) = authority();
    write_control(
        &root,
        &authority,
        "before-open",
        "written before the handle opened",
    );

    let backend = aep_backend_eventlog::open(
        root.clone(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
    )
    .expect("the authority opens");

    let first = backend
        .with_store(|store| store.load(aep_backend_eventlog::INVOCATION_AS, "after-open"))
        .expect("the first read succeeds");
    assert_eq!(first, None, "nothing has written `after-open` yet");

    write_control(
        &root,
        &authority,
        "after-open",
        "written while the handle was open",
    );

    // The write landed: a handle opened now sees it. This is the control for the assertion below,
    // so a red there is staleness and not a write that never happened.
    let witness = aep_backend_eventlog::open(
        root.clone(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
    )
    .expect("the authority reopens");
    assert!(
        witness
            .with_store(|store| store.load(aep_backend_eventlog::INVOCATION_AS, "after-open"))
            .expect("the witness read succeeds")
            .is_some(),
        "the second writer's commit is durable"
    );

    let second = backend
        .with_store(|store| store.load(aep_backend_eventlog::INVOCATION_AS, "after-open"))
        .expect("the second read succeeds");
    assert!(
        second.is_some(),
        "the handle answered from the capture it took when it opened; another writer has \
         committed to this authority since and the handle has made no write of its own, so it \
         will answer this way for as long as it lives"
    );

    let _ = std::fs::remove_dir_all(&root);
}
