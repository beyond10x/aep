//! What a seeded open costs the authority, counted at the seam the seeded open actually uses.
//!
//! `CountingBackend` counts the provider calls this crate makes through `with_async_store_through`
//! — the path `complete_file_snapshot` and the import take. It cannot see the other one:
//! [`aep_backend_eventlog::open`] and [`aep_backend_eventlog::open_with_snapshot`] reach the
//! authority through `RecordedEventlogBridge::start`, which builds its own provider inside Entity
//! Runtime, so no backend decorator is between them and the disk. Those are exactly the opens the
//! held-capture path exists to make free, so the saving this unit is for was counted by nothing.
//!
//! These cases count it, through `RecordedPlanningProvider`, against the **real bridge** rather
//! than a stub — the difference between the two opens is one provider call and nothing a caller
//! can see in the values they return, so counting is the only observation that separates
//! "answered from the seed" from "re-captured and got the same answer".

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use aep_backend_eventlog::counting::ProviderCalls;
use entity_eventlog::{Authority, EventlogOperationContext};
use entity_store::asynchronous::{CompleteStoreSnapshot, StoreCoverage};
use serde_json::json;
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aep-held-seam-{name}-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

fn context(request: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "held-seam".to_owned(),
        actor: "held-seam".to_owned(),
        request_id: request.to_owned(),
        trace_id: request.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

/// One provisioned authority carrying one control row, so a capture has content to describe.
fn provisioned(name: &str) -> (PathBuf, Authority) {
    let root = scratch(name).join("state");
    let identity = aep_backend_eventlog::prepare_file(&root, "tenant-held-seam")
        .expect("the provider prepares a disposable file authority");
    let authority = Authority {
        logical_scope: "planning-held-seam".to_owned(),
        tenant: "tenant-held-seam".to_owned(),
        stream_identity: identity,
    };
    aep_backend_eventlog::provision_file(
        &root,
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        context("held-seam-binding"),
    )
    .expect("the disposable authority binds");
    aep_backend_eventlog::write_file_invocation(
        root.clone(),
        authority.clone(),
        "seeded-row".to_owned(),
        "seeded-row".to_owned(),
        json!({ "written": "before the capture" }),
        None,
        context("held-seam-row"),
    )
    .expect("the authority carries one control row");
    (root, authority)
}

fn snapshot(root: &Path, authority: &Authority) -> CompleteStoreSnapshot {
    aep_backend_eventlog::complete_file_snapshot(root, authority.clone())
        .expect("the caller captures the authority")
}

/// Opening on a seed costs the authority no capture; opening without one costs exactly one.
///
/// Both numbers are here on purpose. `0` alone would pass against a handle that never reads the
/// authority at all, and `1` is what the same open costs on the same authority in the same case,
/// so the pair says the saving is real and says how big it is.
#[test]
fn a_seeded_open_asks_the_bridge_for_no_capture_and_an_unseeded_one_asks_for_exactly_one() {
    let (root, authority) = provisioned("counted");
    let held = snapshot(&root, &authority);

    let unseeded = Arc::new(ProviderCalls::default());
    let backend = aep_backend_eventlog::open_counted(
        root.clone(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        None,
        Arc::clone(&unseeded),
    )
    .expect("the unseeded open succeeds");
    drop(backend);

    let seeded = Arc::new(ProviderCalls::default());
    let backend = aep_backend_eventlog::open_counted(
        root.clone(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        Some(held),
        Arc::clone(&seeded),
    )
    .expect("the seeded open succeeds");
    drop(backend);

    assert_eq!(
        unseeded.read().snapshots,
        1,
        "opening without a seed captures the authority once: {:?}",
        unseeded.read()
    );
    assert_eq!(
        seeded.read().snapshots,
        0,
        "opening on a capture the caller holds captures the authority not at all; it captured \
         {:?}",
        seeded.read()
    );
    assert_eq!(
        seeded.reads(),
        0,
        "and reaches the provider for nothing else either: {:?}",
        seeded.read()
    );
}

/// A seed that is not a complete capture is refused, rather than retained and read through.
///
/// A retained capture answers a subject it names and falls through to the provider for one it does
/// not. For a handle that took its own capture the fall-through is unreachable, because the
/// capture is complete. A partial seed makes it reachable, and then a set of reads the caller is
/// entitled to read as one instant silently spans two.
#[test]
fn a_seed_that_is_not_a_complete_capture_is_refused() {
    let (root, authority) = provisioned("partial");
    let mut partial = snapshot(&root, &authority);
    partial.coverage = StoreCoverage::ExplicitSet;

    let refusal = aep_backend_eventlog::open_with_snapshot(
        root.clone(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        partial,
    )
    .expect_err("a partial capture is not a seed this handle may retain");
    assert!(
        refusal.contains("not a complete snapshot"),
        "the refusal names what is wrong with the seed: {refusal}"
    );
}

/// A seed of another authority's logical scope is refused.
///
/// Completeness is not enough: a complete capture of a different scope names every subject of the
/// wrong store, so every read would be answered out of it and none would fall through.
#[test]
fn a_seed_of_another_logical_scope_is_refused() {
    let (root, authority) = provisioned("scope");
    let mut foreign = snapshot(&root, &authority);
    foreign.scope = "planning-somewhere-else".to_owned();

    let refusal = aep_backend_eventlog::open_with_snapshot(
        root.clone(),
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        foreign,
    )
    .expect_err("a capture of another scope is not this authority's");
    assert!(
        refusal.contains("logical scope"),
        "the refusal names the scope disagreement: {refusal}"
    );
}
