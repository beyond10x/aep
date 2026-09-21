//! What importing a batch of legacy boundaries costs the destination, counted.
//!
//! The migration imported one subject at a time, and each import captured the whole destination
//! before writing and again after it. Nothing a cutover produces — receipt, digest, projection —
//! records how often the destination was captured, so the cost grew with the store unobserved
//! until a 448-artifact store took hours: 7,810 boundaries, 15,620 captures, each one over
//! everything imported so far.
//!
//! These cases count `capture_tenant` at the boundary between this crate and the provider, with
//! [`CountingBackend`], because a count is the only observation that tells "captured once for the
//! batch" from "captured once per subject". A wall clock cannot: it says the same thing about a
//! fast machine and a linear cost.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use aep_backend_eventlog::counting::{BackendCallCounts, BackendCalls, CountingBackend};
use entity_core::EntityInstance;
use entity_eventlog::{Authority, EventlogOperationContext};
use entity_store::asynchronous::{
    HistoryOrigin, LegacyAnchor, LegacyCompleteness, LegacyOrderDeclaration, Subject,
    SubjectHistory,
};
use serde_json::json;
use time::OffsetDateTime;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aep-import-batch-{name}-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

fn context(request: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "import-batch".to_owned(),
        actor: "import-batch".to_owned(),
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
    let identity = aep_backend_eventlog::prepare_file(&root, "tenant-import-batch")
        .expect("the provider prepares a disposable file authority");
    let authority = Authority {
        logical_scope: "planning-import-batch".to_owned(),
        tenant: "tenant-import-batch".to_owned(),
        stream_identity: identity,
    };
    aep_backend_eventlog::provision_file(
        &root,
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        authority.stream_identity.clone(),
        context("import-batch-binding"),
    )
    .expect("the disposable authority binds");
    (root, authority)
}

/// One imported boundary, shaped the way the migration mapper shapes one: a terminal instance,
/// no retained evidence, and an empty suffix.
fn boundary(index: usize) -> SubjectHistory {
    let instance = EntityInstance {
        entity: "aep.entity".to_owned(),
        id: format!("story-{index:04}"),
        version: 1,
        revision: 1,
        lifecycle_state: "draft".to_owned(),
        fields: match json!({ "document": { "title": format!("Subject {index}") } }) {
            serde_json::Value::Object(map) => map,
            _ => unreachable!("the literal above is an object"),
        },
    };
    SubjectHistory {
        subject: Subject::new(&instance.entity, &instance.id).expect("subject"),
        origin: HistoryOrigin::Imported(LegacyAnchor {
            instance,
            completeness: LegacyCompleteness::AvailableEvidenceOnly,
            order: LegacyOrderDeclaration::PerKindOnly,
            evidence: Vec::new(),
        }),
        records: Vec::new(),
    }
}

/// Imports `histories` into `root` through the counting backend and returns what it cost.
fn import_counting(
    root: &std::path::Path,
    authority: &Authority,
    request: &str,
    histories: Vec<SubjectHistory>,
) -> (Vec<bool>, BackendCallCounts) {
    let calls = Arc::new(BackendCalls::default());
    let recorded = Arc::clone(&calls);
    let replayed = aep_backend_eventlog::import_file_anchors_through(
        root,
        authority.clone(),
        context(request),
        histories,
        move |backend| Arc::new(CountingBackend::new(backend, recorded)),
    )
    .expect("the batch imports");
    (replayed, calls.read())
}

/// Importing N boundaries captures the destination a fixed number of times, not N times.
///
/// At `d10fcebc5` this asked the provider once per subject and each ask captured twice, so the
/// count was `2N + 1`: 33 captures for 16 subjects, 129 for 64. The number here does not mention
/// N at all, which is the property — a cutover of a store ten times the size pays the same.
#[test]
fn importing_a_batch_captures_the_destination_a_fixed_number_of_times() {
    for subjects in [1_usize, 16, 64] {
        let (root, authority) = provisioned(&format!("fixed-{subjects}"));
        let histories = (0..subjects).map(boundary).collect::<Vec<_>>();

        let (replayed, counts) = import_counting(&root, &authority, "fixed", histories);

        assert_eq!(
            replayed.len(),
            subjects,
            "one outcome per imported boundary, in the input's order"
        );
        assert_eq!(
            counts.captures,
            3,
            "importing {subjects} boundaries captures the destination 3 times — once to open the \
             recorded store, once to verify the batch against, once to verify the result — and \
             not {} as one capture per subject would cost",
            2 * subjects + 1
        );
        assert_eq!(
            counts.groups, 1,
            "the whole batch of {subjects} is one atomic append group"
        );
        assert_eq!(
            counts.blob_writes, 0,
            "every blob the batch binds is written inside the group's own barrier, not on its own \
             path"
        );
    }
}

/// What an import captures depends on neither the batch's size nor the destination's.
///
/// Three imports into one authority: 64 into an empty destination, 64 more into one already
/// holding 64, then a single boundary into one holding 128. All three cost the same captures.
///
/// The first pair alone is not a guard. Under the `d10fcebc5` import the count was `2N + 1` — a
/// function of the batch, not of the destination — so two batches of equal size matched there
/// too, and a case comparing only those two passes against the defect it was written for. What
/// grew with the destination was the *price* of each capture, which no count can see. The third
/// import is what makes the property assertable: `2N + 1` cannot hold a batch of 1 and a batch of
/// 64 to the same number, and a count that is genuinely the batch's fixed cost can.
#[test]
fn neither_the_batch_nor_the_destination_changes_what_an_import_captures() {
    let (root, authority) = provisioned("independent");

    let first = (0..64).map(boundary).collect::<Vec<_>>();
    let (_, empty_destination) = import_counting(&root, &authority, "first", first);

    let second = (64..128).map(boundary).collect::<Vec<_>>();
    let (_, full_destination) = import_counting(&root, &authority, "second", second);

    let (_, one_into_full) = import_counting(&root, &authority, "third", vec![boundary(128)]);

    assert_eq!(
        full_destination, empty_destination,
        "importing 64 boundaries into an authority holding 64 costs exactly what importing 64 \
         into an empty one costs"
    );
    assert_eq!(
        one_into_full, full_destination,
        "and importing one boundary into an authority holding 128 costs that same fixed amount — \
         the cost is the batch's, and the batch is one call however many boundaries it carries"
    );
}

/// A batch of one costs what a batch of many costs, so nothing is gained by splitting one.
#[test]
fn a_batch_of_one_and_a_batch_of_many_cost_the_same_captures() {
    let (one_root, one_authority) = provisioned("one");
    let (_, one) = import_counting(&one_root, &one_authority, "one", vec![boundary(0)]);

    let (many_root, many_authority) = provisioned("many");
    let (_, many) = import_counting(
        &many_root,
        &many_authority,
        "many",
        (0..64).map(boundary).collect(),
    );

    assert_eq!(
        one.captures, many.captures,
        "the capture cost is the batch's, not the subject's"
    );
    assert_eq!(one.groups, many.groups, "one batch is one group either way");
}
