//! Integrity conformance checks for the capture an opened planning handle retains.
//!
//! These run against the **real** file Eventlog provider through the crate's public surface, not
//! against the unit's own counting stub, because the question here is not how many reads a handle
//! makes but whether what it answers still describes the authority. The retained capture is
//! retired only by a write through the same handle; an append made through any other handle —
//! another process, or the crate's own `write_file_*` control writers, which open their own
//! bridge — leaves the retaining handle answering from before it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use entity_eventlog::{Authority, EventlogOperationContext};
use entity_store::{HistoryProvider as _, StateProvider as _};
use serde_json::json;
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aep-retained-capture-{name}-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

fn context(request: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "retained-capture-review".to_owned(),
        actor: "retained-capture-review".to_owned(),
        request_id: request.to_owned(),
        trace_id: request.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// One provisioned, empty file authority.
fn provisioned(name: &str) -> (PathBuf, Authority) {
    let root = scratch(name).join("state");
    let identity = aep_backend_eventlog::prepare_file(&root, "tenant-retained-capture")
        .expect("the provider prepares a disposable file authority");
    let authority = Authority {
        logical_scope: "planning-retained-capture".to_owned(),
        tenant: "tenant-retained-capture".to_owned(),
        stream_identity: identity,
    };
    aep_backend_eventlog::provision_file(
        &root,
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        context("retained-capture-binding"),
    )
    .expect("the disposable authority binds");
    (root, authority)
}

/// Appends one control row through a handle that is not the one under test, as a second process
/// writing to the same authority would.
fn append_through_another_handle(root: &Path, authority: &Authority, identity: &str) {
    aep_backend_eventlog::write_file_invocation(
        root.to_path_buf(),
        authority.clone(),
        identity.to_owned(),
        identity.to_owned(),
        json!({ "written-by": "another handle" }),
        None,
        context(identity),
    )
    .expect("the second handle appends");
}

fn opened(root: &Path, authority: &Authority) -> aep_backend_eventlog::EventlogBackend {
    aep_backend_eventlog::open(
        root.to_path_buf(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
    )
    .expect("the provisioned authority opens")
}

/// A handle lists the authority as of the instant it opened, and reopening is how a caller moves on.
///
/// `ids` is the read `hydrate` makes and the read every listing verb makes, and it is answered
/// from the capture the handle took while opening. This case was written by the first review
/// pass asserting the other thing — *"a read on a handle that has written nothing must describe
/// the authority as it is"*, expecting `["inv-after-open", "inv-before-open"]` — and the
/// coordinator settled that as a `no-op`: one handle is one command, its reads are deliberately
/// mutually consistent at one instant, and no holder in this repository outlives a command. The
/// assertion therefore moved to what was decided, and the second half of the case is the remedy
/// a future long-lived holder has: reopen. Both `ids` calls the reviewer made are still made and
/// still compared.
#[test]
fn a_read_only_handle_lists_the_authority_as_of_the_instant_it_opened() {
    let (root, authority) = provisioned("ids");
    append_through_another_handle(&root, &authority, "inv-before-open");

    let backend = opened(&root, &authority);
    let first = backend.with_store(|store| {
        store
            .ids(aep_backend_eventlog::INVOCATION_AS)
            .expect("the opened handle lists control rows")
    });
    assert_eq!(first, vec!["inv-before-open".to_owned()]);

    append_through_another_handle(&root, &authority, "inv-after-open");

    let second = backend.with_store(|store| {
        store
            .ids(aep_backend_eventlog::INVOCATION_AS)
            .expect("the opened handle lists control rows")
    });
    assert_eq!(
        second,
        vec!["inv-before-open".to_owned()],
        "every read on one handle answers from one instant, so a listing does not change under \
         a caller mid-command, and answered {second:?}"
    );

    let reopened = opened(&root, &authority);
    let third = reopened.with_store(|store| {
        store
            .ids(aep_backend_eventlog::INVOCATION_AS)
            .expect("the reopened handle lists control rows")
    });
    assert_eq!(
        third,
        vec!["inv-after-open".to_owned(), "inv-before-open".to_owned()],
        "and a handle opened after the other writer's append sees it: reopening is what a holder \
         that outlives one command does, and answered {third:?}"
    );
}

/// The same staleness, on the history read `artifact history` answers from.
///
/// `records` goes through the retained capture too, and a subject the capture does not name is
/// answered with a synthesized empty genesis history rather than with the provider's answer — so
/// a subject that exists reads as one that was never written.
#[test]
fn a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened() {
    let (root, authority) = provisioned("records");
    append_through_another_handle(&root, &authority, "inv-before-open");

    let backend = opened(&root, &authority);
    let warm = backend.with_store(|store| {
        store
            .ids(aep_backend_eventlog::INVOCATION_AS)
            .expect("the opened handle lists control rows")
    });
    assert_eq!(warm, vec!["inv-before-open".to_owned()]);

    append_through_another_handle(&root, &authority, "inv-after-open");

    let records = backend.with_store(|store| {
        store
            .records(aep_backend_eventlog::INVOCATION_AS, "inv-after-open")
            .expect("the opened handle reads a subject history")
    });
    assert_eq!(
        records.len(),
        1,
        "the subject was created by one record and the handle answered {} records",
        records.len()
    );
}
