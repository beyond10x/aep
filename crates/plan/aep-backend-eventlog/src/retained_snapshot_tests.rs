//! What one hydrate costs the provider, and what a write through the handle retires.
//!
//! Every capture of a file Eventlog authority re-reads and re-hashes every bound object — about
//! 11 MB for the real planning store — so the number that matters is not how long one read takes
//! but how many captures one open makes. These cases count them, through a
//! [`RecordedPlanningProvider`] that records every call the store makes, because a count is the
//! only observation that distinguishes "asked once" from "asked once per record".

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use aep_backend_entity::{
    EntityBackend, PlanningCommit, PlanningStore as _, METADATA_KEY, STORED_AS,
};
use aep_domain::entity::{ActorRef, EntityId, EntityLocator, EntityMetadata, EntityType};
use aep_domain::time::Timestamp;
use entity_core::{Decision, DecisionCommand, DecisionRecord, EntityInstance};
use entity_eventlog::sync::{SyncExecutionError, SyncReadError};
use entity_eventlog::{Authority, EventlogOperationContext};
use entity_executor::{BatchAction, CreateRequest, ExecuteRequest};
use entity_store::asynchronous::{
    AppendOutcome, BatchKey, CompleteStoreSnapshot, HistoryOrigin, LegacyAnchor,
    LegacyCompleteness, LegacyEvidence, LegacyOrderDeclaration, StoreCoverage, StoredBatch,
    Subject, SubjectHistory, SubjectSnapshot,
};
use entity_store::{
    AtomicCommit, Expect, HistoryProvider as _, RecordedCommit, RecordedObservation, Recording,
    StateProvider as _, Store as _,
};
use serde_json::{json, Map, Value};

use crate::{EventlogPlanningStore, RecordedPlanningProvider, LEGACY_COORDINATE_AS};

const SCOPE: &str = "aep.test/1";
const AT: u64 = 1_700_000_000_000;

/// Every provider call one store made, by kind. A capture is `snapshots` plus `loads` plus
/// `histories` plus `lookups`: each one is a separate capture on the authority, and there is a
/// field here for every read [`RecordedPlanningProvider`] declares, because the trait's doc
/// comment says a test counts all of them and a read this struct cannot see measures as free.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Calls {
    snapshots: usize,
    loads: usize,
    histories: usize,
    lookups: usize,
    batches: usize,
    observations: usize,
}

impl Calls {
    /// What the provider was asked to do in total — the figure the story is about.
    const fn reads(self) -> usize {
        self.snapshots + self.loads + self.histories + self.lookups
    }
}

/// An in-memory recorded authority that counts what the store asks it for.
///
/// It is not a second Eventlog: it answers the six calls [`RecordedPlanningProvider`] declares,
/// with the same shapes the bridge answers them with, and applies a batch to its own terminals so
/// that a read after a write can be compared with a read before it.
#[derive(Debug, Default)]
struct Authoritative {
    terminals: BTreeMap<Subject, EntityInstance>,
    /// Review addition: a captured history that is not the empty genesis every existing
    /// case sees. Empty by default, so nothing above changes behaviour.
    histories: BTreeMap<Subject, SubjectHistory>,
}

#[derive(Debug, Clone, Default)]
struct CountingProvider {
    held: Arc<Mutex<Authoritative>>,
    calls: Arc<Mutex<Calls>>,
}

impl CountingProvider {
    fn calls(&self) -> Calls {
        *self.calls.lock().expect("counting provider lock")
    }

    /// Review addition: gives one subject a history that is not the empty genesis.
    fn put_history(&self, subject: Subject, history: SubjectHistory) {
        self.held
            .lock()
            .expect("counting provider lock")
            .histories
            .insert(subject, history);
    }

    /// Writes behind the handle's back, as a second process would.
    fn put(&self, subject: Subject, instance: EntityInstance) {
        self.held
            .lock()
            .expect("counting provider lock")
            .terminals
            .insert(subject, instance);
    }
}

impl RecordedPlanningProvider for CountingProvider {
    fn complete_snapshot(&self, scope: &str) -> Result<CompleteStoreSnapshot, SyncReadError> {
        self.calls.lock().expect("counting provider lock").snapshots += 1;
        let held = self.held.lock().expect("counting provider lock");
        let histories = held.histories.clone();
        Ok(CompleteStoreSnapshot {
            scope: scope.to_owned(),
            coverage: StoreCoverage::CompleteSnapshot,
            histories: held
                .terminals
                .iter()
                .map(|(subject, terminal)| SubjectSnapshot {
                    history: histories
                        .get(subject)
                        .cloned()
                        .unwrap_or_else(|| genesis(subject)),
                    terminal: terminal.clone(),
                })
                .collect(),
        })
    }

    fn load(&self, subject: &Subject) -> Result<Option<EntityInstance>, SyncReadError> {
        self.calls.lock().expect("counting provider lock").loads += 1;
        Ok(self
            .held
            .lock()
            .expect("counting provider lock")
            .terminals
            .get(subject)
            .cloned())
    }

    fn history(&self, subject: &Subject) -> Result<SubjectHistory, SyncReadError> {
        self.calls.lock().expect("counting provider lock").histories += 1;
        let held = self.held.lock().expect("counting provider lock");
        Ok(held
            .histories
            .get(subject)
            .cloned()
            .unwrap_or_else(|| genesis(subject)))
    }

    fn lookup_batch(&self, _key: &BatchKey) -> Result<Option<StoredBatch>, SyncReadError> {
        self.calls.lock().expect("counting provider lock").lookups += 1;
        Ok(None)
    }

    fn batch(
        &self,
        _context: EventlogOperationContext,
        _key: BatchKey,
        actions: Vec<BatchAction>,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        self.calls.lock().expect("counting provider lock").batches += 1;
        let mut held = self.held.lock().expect("counting provider lock");
        for action in actions {
            match action {
                BatchAction::Create(CreateRequest {
                    subject, fields, ..
                }) => {
                    let instance = row(&subject, 1, &fields);
                    held.terminals.insert(subject, instance);
                }
                BatchAction::Execute(ExecuteRequest {
                    subject,
                    expected_revision,
                    arguments,
                    ..
                }) => {
                    let instance = row(&subject, expected_revision + 1, &arguments);
                    held.terminals.insert(subject, instance);
                }
                other @ (BatchAction::Observe(_) | BatchAction::Merge(_)) => {
                    panic!("unexpected batch action {other:?}")
                }
            }
        }
        Ok(AppendOutcome::Empty)
    }

    fn observe(
        &self,
        _context: EventlogOperationContext,
        _observation: RecordedObservation,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        self.calls
            .lock()
            .expect("counting provider lock")
            .observations += 1;
        Ok(AppendOutcome::Empty)
    }
}

fn genesis(subject: &Subject) -> SubjectHistory {
    SubjectHistory {
        subject: subject.clone(),
        origin: HistoryOrigin::Genesis,
        records: Vec::new(),
    }
}

fn row(subject: &Subject, revision: u64, fields: &Value) -> EntityInstance {
    EntityInstance {
        entity: subject.entity.clone(),
        version: 1,
        id: subject.id.clone(),
        lifecycle_state: "recorded".to_owned(),
        revision,
        fields: fields
            .as_object()
            .expect("provider row fields are an object")
            .clone(),
    }
}

fn subject(entity: &str, id: &str) -> Subject {
    Subject::new(entity, id).expect("a subject")
}

fn authority() -> Authority {
    Authority {
        logical_scope: SCOPE.to_owned(),
        tenant: "tenant-test".to_owned(),
        stream_identity: "stream-test".to_owned(),
    }
}

fn store(provider: CountingProvider) -> EventlogPlanningStore<CountingProvider> {
    EventlogPlanningStore::new(provider, authority())
}

/// One migrated planning artifact, exactly as `commit_planning_batch` writes one.
fn artifact(n: u32) -> (Subject, EntityInstance) {
    let id = EntityId::new(format!("01JCOUNT{n:018}")).expect("an entity id");
    let metadata = EntityMetadata::new(
        id.clone(),
        EntityLocator::parse(&format!("ep://beyond10x/plan/story/counted-{n}")).expect("a locator"),
        EntityType::parse("aep.story/v1").expect("a type"),
        Timestamp::from_epoch_millis(AT + u64::from(n)),
        ActorRef::parse("human:operator").expect("an actor"),
    );
    let document = json!({
        "lifecycle_state": "draft",
        "fields": {
            "status": "draft",
            "title": format!("counted {n}"),
            METADATA_KEY: { "metadata": metadata, "archived": false },
        },
        "events": [],
    });
    let subject = subject(STORED_AS, id.as_str());
    let instance = row(&subject, 1, &json!({ "document": document }));
    (subject, instance)
}

/// A planning row for a subject nothing has written yet, ready to create.
fn creation(id: &str) -> Decision {
    let instance = EntityInstance {
        entity: STORED_AS.to_owned(),
        version: 1,
        id: id.to_owned(),
        lifecycle_state: "draft".to_owned(),
        revision: 1,
        fields: Map::new(),
    };
    Decision {
        record: DecisionRecord {
            definition: None,
            command: DecisionCommand::Create {
                fields: Map::new(),
                arguments: Map::new(),
            },
            entity: instance.entity.clone(),
            id: instance.id.clone(),
            revision: 1,
            from_state: None,
            to_state: instance.lifecycle_state.clone(),
            result: instance.clone(),
            changed: Map::new(),
            removed: std::collections::BTreeSet::new(),
            events: Vec::new(),
            outcome: None,
            effect: None,
            response: None,
        },
        instance,
        events: Vec::new(),
    }
}

fn recording(record_id: &str) -> Recording {
    Recording {
        record_id: record_id.to_owned(),
        recorded_at: "1970-01-01T00:00:00Z".to_owned(),
        correlation: None,
        causation: None,
        actor: Some("aep".to_owned()),
    }
}

/// The acceptance case: opening a populated authority asks the provider once, not once per record.
#[test]
fn opening_a_populated_authority_costs_one_capture_not_one_per_record() {
    let provider = CountingProvider::default();
    for n in 1..=3 {
        let (subject, instance) = artifact(n);
        provider.put(subject, instance);
    }
    let handle = store(provider.clone());

    // Exactly what `open` does: validate the legacy boundaries, then hydrate the contract.
    handle
        .validate_legacy_boundaries()
        .expect("no legacy boundaries to disagree with");
    let backend = EntityBackend::over(handle).expect("the store hydrates");
    drop(backend);

    let calls = provider.calls();
    assert_eq!(
        calls.reads(),
        1,
        "opening a three-artifact authority must ask the provider once, and asked {calls:?}"
    );
    assert_eq!(calls.snapshots, 1, "one complete capture: {calls:?}");
    assert_eq!(
        calls.loads, 0,
        "no per-record load reaches the provider: {calls:?}"
    );
    assert_eq!(
        calls.histories, 0,
        "no per-record history reaches the provider: {calls:?}"
    );
}

/// A handle opened on a capture its caller already holds asks the provider for nothing at all.
///
/// One capture per open is the floor only for a caller that has none. The migration has one: it
/// captures the destination to verify what it imported, and the projection that follows opens the
/// same authority with nothing written to it in between. That open re-read and re-hashed every
/// bound blob — on the 448-artifact store, 7,815 of them and 174 MB — to be told what the
/// verification capture had already said.
///
/// The count here is **zero**, not one, and that is the whole property: `1` is what this costs
/// without a seeded capture, and the case above measures that same open at `1`.
#[test]
fn opening_on_a_capture_the_caller_holds_costs_the_provider_nothing() {
    let provider = CountingProvider::default();
    for n in 1..=3 {
        let (subject, instance) = artifact(n);
        provider.put(subject, instance);
    }

    // The caller's capture — a migration's verification snapshot, taken once.
    let taken = store(provider.clone())
        .snapshot()
        .expect("the caller captures the authority once");
    let after_taking = provider.calls();
    assert_eq!(
        after_taking.reads(),
        1,
        "the caller's own capture is one read: {after_taking:?}"
    );

    // Exactly what `open_with_snapshot` does with it.
    let handle = store(provider.clone());
    handle.seed(CompleteStoreSnapshot::clone(&taken));
    handle
        .validate_legacy_boundaries()
        .expect("no legacy boundaries to disagree with");
    let backend = EntityBackend::over(handle).expect("the seeded store hydrates");
    drop(backend);

    assert_eq!(
        provider.calls(),
        after_taking,
        "opening and hydrating on a capture the caller already holds must reach the provider \
         zero further times; it reached it {:?} against {after_taking:?} before",
        provider.calls()
    );
}

/// A seeded handle answers what an unseeded one answers, subject for subject.
///
/// The saving is only worth taking if the answers are the same ones. A capture handed in is the
/// same shape as a capture taken, so a read served from it cannot differ — this is the case that
/// says so rather than assuming it.
#[test]
fn a_seeded_handle_answers_exactly_what_a_capturing_handle_answers() {
    let provider = CountingProvider::default();
    for n in 1..=3 {
        let (subject, instance) = artifact(n);
        provider.put(subject, instance);
    }
    let taken = store(provider.clone())
        .snapshot()
        .expect("the caller captures the authority once");

    let capturing = store(provider.clone());
    let seeded = store(provider.clone());
    seeded.seed(CompleteStoreSnapshot::clone(&taken));

    let ids = capturing.ids(STORED_AS).expect("ids");
    assert_eq!(ids, seeded.ids(STORED_AS).expect("seeded ids"));
    assert!(!ids.is_empty(), "the fixture has subjects to compare");
    for id in &ids {
        assert_eq!(
            capturing.load(STORED_AS, id).expect("load"),
            seeded.load(STORED_AS, id).expect("seeded load"),
            "terminal state of {id} differs between a taken and a handed-in capture"
        );
        assert_eq!(
            capturing.records(STORED_AS, id).expect("records"),
            seeded.records(STORED_AS, id).expect("seeded records"),
            "history of {id} differs between a taken and a handed-in capture"
        );
    }
}

/// The retained capture is per handle, not per call: a second read of the same thing is free.
#[test]
fn a_second_read_after_a_capture_asks_the_provider_nothing() {
    let provider = CountingProvider::default();
    let (subject, instance) = artifact(1);
    provider.put(subject.clone(), instance);
    let handle = store(provider.clone());

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![subject.id.clone()]
    );
    let before = provider.calls();
    assert!(handle.load(STORED_AS, &subject.id).expect("load").is_some());
    assert!(handle
        .records(STORED_AS, &subject.id)
        .expect("records")
        .is_empty());
    assert_eq!(
        provider.calls(),
        before,
        "a read after a capture must not reach the provider again"
    );
}

/// A handle that has never captured still answers a single read, and does it with a single read.
#[test]
fn a_read_before_any_capture_falls_through_to_the_provider() {
    let provider = CountingProvider::default();
    let (subject, instance) = artifact(1);
    provider.put(subject.clone(), instance);
    let handle = store(provider.clone());

    assert!(handle.load(STORED_AS, &subject.id).expect("load").is_some());
    assert!(handle
        .records(STORED_AS, &subject.id)
        .expect("records")
        .is_empty());

    let calls = provider.calls();
    assert_eq!(calls.loads, 1, "the single-subject load is made: {calls:?}");
    assert_eq!(
        calls.histories, 1,
        "the single-subject history is made: {calls:?}"
    );
    assert_eq!(
        calls.snapshots, 0,
        "and neither materializes the whole authority: {calls:?}"
    );
}

/// `commit` retires the retained capture, so the next read sees what was committed.
#[test]
fn a_commit_retires_the_retained_capture() {
    let provider = CountingProvider::default();
    let mut handle = store(provider.clone());
    let id = "01JCOMMIT0000000000000001";

    assert!(handle.ids(STORED_AS).expect("ids").is_empty());
    handle
        .commit(&creation(id), Expect::Absent)
        .expect("the commit is accepted");

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![id.to_owned()],
        "the read after a commit must see the commit"
    );
}

/// `commit_recorded` retires it too.
#[test]
fn a_recorded_commit_retires_the_retained_capture() {
    let provider = CountingProvider::default();
    let mut handle = store(provider.clone());
    let id = "01JRECORDED00000000000001";
    let commit = RecordedCommit::new(creation(id), &recording("rec-1")).expect("a recorded commit");

    assert!(handle.ids(STORED_AS).expect("ids").is_empty());
    handle
        .commit_recorded(&commit, Expect::Absent)
        .expect("the recorded commit is accepted");

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![id.to_owned()],
        "the read after a recorded commit must see the commit"
    );
}

/// And so does a planning batch.
#[test]
fn a_planning_batch_retires_the_retained_capture() {
    let provider = CountingProvider::default();
    let mut handle = store(provider.clone());
    let id = "01JBATCH00000000000000001";

    assert!(handle.ids(STORED_AS).expect("ids").is_empty());
    handle
        .commit_planning_batch(&[PlanningCommit {
            commit: AtomicCommit::new(creation(id), Expect::Absent),
            recording: recording("batch-1"),
        }])
        .expect("the batch is accepted");

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![id.to_owned()],
        "the read after a batch must see the batch"
    );
}

/// An observation is a write through the handle as much as a decision is.
#[test]
fn an_observation_retires_the_retained_capture() {
    let provider = CountingProvider::default();
    let mut handle = store(provider.clone());
    let (subject, instance) = artifact(1);

    assert!(handle.ids(STORED_AS).expect("ids").is_empty());
    let observation: RecordedObservation = serde_json::from_value(json!({
        "entity": STORED_AS,
        "id": subject.id,
        "revision": 1,
        "envelope": {
            "record_id": "obs-1",
            "recorded_at": "1970-01-01T00:00:00Z",
            "correlation": null,
            "causation": null,
            "actor": "aep",
            "record": { "kind": "note" },
        },
    }))
    .expect("an observation");
    handle
        .observe(&observation)
        .expect("the observation is recorded");
    provider.put(subject.clone(), instance);

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![subject.id],
        "the read after an observation must not answer from before it"
    );
}

/// A write validates against the authority as it is now, not as the handle last saw it.
///
/// The retained capture is retired *before* the write's own reads, so a legacy boundary that
/// became inconsistent after the handle's last read still refuses the write.
#[test]
fn a_write_revalidates_the_authority_it_is_writing_to() {
    let provider = CountingProvider::default();
    let mut handle = store(provider.clone());

    assert!(handle.ids(STORED_AS).expect("ids").is_empty());

    // A coordinate whose evidence blob is absent: the join the store refuses to write over.
    let coordinate = subject(LEGACY_COORDINATE_AS, "coordinate-1");
    let instance = row(
        &coordinate,
        1,
        &json!({
            "format": "aep.legacy-record-coordinate/1",
            "source_snapshot": format!("sha256:{}", "00".repeat(32)),
            "source_locator": "journal.jsonl/0",
            "destination_entity": STORED_AS,
            "destination_id": "01JMISSING000000000000001",
            "evidence_kind": "change",
            "original_record_id": { "kind": "missing" },
            "order": "store",
            "ordinal": { "kind": "present", "value": 0 },
            "evidence_blob_id": "blob-that-is-not-there",
            "envelope_digest": format!("sha256:{}", "11".repeat(32)),
            "reservation_roster_id": { "kind": "missing" },
        }),
    );
    provider.put(coordinate, instance);

    let refusal = handle
        .commit_planning_batch(&[PlanningCommit {
            commit: AtomicCommit::new(creation("01JREVALIDATE00000000001"), Expect::Absent),
            recording: recording("revalidate-1"),
        }])
        .expect_err("a write over an inconsistent legacy boundary is refused");
    assert!(
        format!("{refusal:?}").contains("blob-that-is-not-there"),
        "the refusal names the absent evidence, and said {refusal:?}"
    );
}

// ─── Added by independent verification (review-1, 2026-09-21). Nothing above is changed. ───

/// Every read the trait declares costs one capture, so the counter must see every one.
///
/// [`RecordedPlanningProvider`]'s own doc comment says each of its six calls is one
/// `capture_tenant` and that "a test counts them through this". `lookup_batch` is one of the six
/// — on the real bridge it is `capture_model()`, a whole capture of the authority — and [`Calls`]
/// has no field for it, so [`Calls::reads`], the figure the `5 + 2N → 1` result is stated in,
/// cannot see it. A read path that grew one `recover_planning_receipt` would still count as one.
#[test]
fn every_provider_read_the_trait_declares_is_counted() {
    let provider = CountingProvider::default();
    let handle = store(provider.clone());

    let before = provider.calls().reads();
    handle
        .recover_planning_receipt("batch-1")
        .expect("recovering an unknown receipt answers absent");
    let after = provider.calls();
    assert_eq!(
        after.reads(),
        before + 1,
        "`lookup_batch` is one capture on the real bridge and the counter did not see it: {after:?}"
    );
}

/// A read served from the retained capture answers with *that subject's* row.
///
/// The counting cases above assert call counts and nothing else: the acceptance case drops the
/// backend without looking at it, and the second-read case holds one artifact, so neither can
/// tell a capture that is indexed correctly from one that answers every subject with the first
/// row it holds. This compares the value.
#[test]
fn a_capture_answers_each_subject_with_its_own_row() {
    let provider = CountingProvider::default();
    let subjects: Vec<_> = (1..=3)
        .map(|n| {
            let (subject, instance) = artifact(n);
            provider.put(subject.clone(), instance);
            (n, subject)
        })
        .collect();
    let handle = store(provider.clone());

    // Warm the capture exactly as `hydrate` does, then read every subject back through it.
    assert_eq!(handle.ids(STORED_AS).expect("ids").len(), 3);
    for (n, subject) in &subjects {
        let loaded = handle
            .load(STORED_AS, &subject.id)
            .expect("the capture answers a listed subject")
            .expect("a listed subject is present");
        assert_eq!(
            loaded.id, subject.id,
            "the capture answered subject {} with row {}",
            subject.id, loaded.id
        );
        assert_eq!(
            loaded.fields.get("title").and_then(Value::as_str),
            Some(format!("counted {n}").as_str()),
            "the capture answered {} with another subject's document",
            subject.id
        );
    }
    assert_eq!(
        provider.calls().reads(),
        1,
        "and all of that came from the one capture"
    );

    let backend = EntityBackend::over(store(provider.clone())).expect("the store hydrates");
    assert_eq!(
        backend.len(),
        3,
        "hydrating a three-artifact authority holds three artifacts"
    );
}

/// A history served from the retained capture is the capture's history, not a fresh genesis.
///
/// Every `SubjectSnapshot` the fixture above builds carried an empty genesis history, so no case
/// in this module compared a history *value* that came out of the capture against the one the
/// provider holds — only the number of calls. A `subject_history` that threw the captured history
/// away and answered every subject with an empty genesis left all eight of them green (review
/// probe R1), while `records`, `observations`, `events_in_store_order` and `revisions_before` all
/// read through it.
#[test]
fn a_capture_answers_a_subject_with_the_history_it_captured() {
    let provider = CountingProvider::default();
    let (subject, instance) = artifact(1);
    provider.put(subject.clone(), instance.clone());
    provider.put_history(
        subject.clone(),
        SubjectHistory {
            subject: subject.clone(),
            origin: HistoryOrigin::Imported(LegacyAnchor {
                instance,
                completeness: LegacyCompleteness::AvailableEvidenceOnly,
                order: LegacyOrderDeclaration::PerKindOnly,
                evidence: vec![LegacyEvidence::Event(migrated_event(&subject.id))],
            }),
            records: Vec::new(),
        },
    );
    let handle = store(provider.clone());

    assert_eq!(handle.ids(STORED_AS).expect("ids").len(), 1);
    let events = handle
        .events_in_store_order(STORED_AS, &subject.id)
        .expect("the capture answers the subject's history");
    assert_eq!(
        events.len(),
        1,
        "the imported anchor the capture holds carries one event, and the handle answered {events:?}"
    );
    assert_eq!(
        provider.calls().reads(),
        1,
        "and it came from the one capture"
    );
}

/// One legacy event preserved in an imported anchor, as migration writes one.
fn migrated_event(id: &str) -> entity_core::DomainEvent {
    serde_json::from_value(json!({
        "entity": STORED_AS,
        "version": 1,
        "id": id,
        "revision": 1,
        "type": "aep.entity.create/v1",
        "from_state": null,
        "to_state": "draft",
        "changed": {},
        "args": {},
        "payload": {},
    }))
    .expect("a migrated domain event")
}
// ---------------------------------------------------------------------------
// Independent verification, second pass (2026-09-21). Everything below this
// marker was appended by the reviewer; no line above it was changed, deleted
// or renamed. These cases ask two questions the cases above do not: what the
// retained capture *answers* (not only when it is retired), and whether the
// fall-through the story's acceptance names is reachable at all.
// ---------------------------------------------------------------------------

/// A provider whose history is not always empty, and whose complete capture is built from the
/// same values its single-subject reads answer with.
///
/// [`CountingProvider`] answers every `history` with genesis and builds every capture entry's
/// history with genesis too, so a read served from the capture and the same read served by the
/// provider are equal for both being empty. This one models
/// `entity-eventlog/src/adapter.rs:613-679` instead: `load` is `terminals.get(subject)`,
/// `history` is `histories.get(subject)` or the genesis history, and `complete_snapshot` is the
/// pairs of both — so the two paths can be compared for *equality* rather than for emptiness.
#[derive(Debug, Clone, Default)]
struct FaithfulProvider {
    held: Arc<Mutex<BTreeMap<Subject, SubjectSnapshot>>>,
}

impl FaithfulProvider {
    fn put(&self, history: SubjectHistory, terminal: EntityInstance) {
        self.held.lock().expect("faithful provider lock").insert(
            history.subject.clone(),
            SubjectSnapshot { history, terminal },
        );
    }
}

impl RecordedPlanningProvider for FaithfulProvider {
    fn complete_snapshot(&self, scope: &str) -> Result<CompleteStoreSnapshot, SyncReadError> {
        Ok(CompleteStoreSnapshot {
            scope: scope.to_owned(),
            coverage: StoreCoverage::CompleteSnapshot,
            histories: self
                .held
                .lock()
                .expect("faithful provider lock")
                .values()
                .cloned()
                .collect(),
        })
    }

    fn load(&self, subject: &Subject) -> Result<Option<EntityInstance>, SyncReadError> {
        Ok(self
            .held
            .lock()
            .expect("faithful provider lock")
            .get(subject)
            .map(|value| value.terminal.clone()))
    }

    fn history(&self, subject: &Subject) -> Result<SubjectHistory, SyncReadError> {
        Ok(self
            .held
            .lock()
            .expect("faithful provider lock")
            .get(subject)
            .map_or_else(|| genesis(subject), |value| value.history.clone()))
    }

    fn lookup_batch(&self, _key: &BatchKey) -> Result<Option<StoredBatch>, SyncReadError> {
        Ok(None)
    }

    fn batch(
        &self,
        _context: EventlogOperationContext,
        _key: BatchKey,
        _actions: Vec<BatchAction>,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        panic!("the differential cases make no write");
    }

    fn observe(
        &self,
        _context: EventlogOperationContext,
        _observation: RecordedObservation,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        panic!("the differential cases make no write");
    }
}

/// A store over the faithful provider, as `store` is one over the counting provider.
fn faithful_store(provider: FaithfulProvider) -> EventlogPlanningStore<FaithfulProvider> {
    EventlogPlanningStore::new(provider, authority())
}

/// One stored observation about `subject`, so a history is something other than empty.
fn observed(
    subject: &Subject,
    record_id: &str,
    store_position: u64,
) -> entity_store::asynchronous::StoredRecord {
    let observation: RecordedObservation = serde_json::from_value(json!({
        "entity": subject.entity.clone(),
        "id": subject.id.clone(),
        "revision": 1,
        "envelope": {
            "record_id": record_id,
            "recorded_at": "1970-01-01T00:00:00Z",
            "correlation": null,
            "causation": null,
            "actor": "aep",
            "record": { "kind": "note", "about": record_id },
        },
    }))
    .expect("an observation");
    let position = entity_store::asynchronous::RecordPosition {
        subject: 1,
        store: store_position,
    };
    entity_store::asynchronous::StoredRecord {
        entry: entity_store::asynchronous::RecordedEntry::Observation(observation),
        position,
        receipt: entity_store::asynchronous::RecordReceipt {
            record_id: record_id.to_owned(),
            subject: subject.clone(),
            kind: entity_store::asynchronous::RecordKind::Observation,
            revision: 1,
            position,
            batch_key: BatchKey::Named(record_id.to_owned()),
            member_index: 0,
        },
        expect: Expect::Revision(1),
        request_bytes: Vec::new(),
        lineage: None,
        record_bytes: Vec::new(),
    }
}

/// A history carrying one observation, so `records` and `observations` answer differently.
fn history_with_one_observation(subject: &Subject, record_id: &str, at: u64) -> SubjectHistory {
    SubjectHistory {
        subject: subject.clone(),
        origin: HistoryOrigin::Genesis,
        records: vec![observed(subject, record_id, at)],
    }
}

/// A row of any kind the store can unpack: everything but `aep.entity` keeps ER's own revision.
fn control_row(subject: &Subject, revision: u64, marker: &str) -> EntityInstance {
    row(
        subject,
        revision,
        &json!({
            "document": {
                "lifecycle_state": "recorded",
                "fields": { "marker": marker },
                "events": [],
            }
        }),
    )
}

/// Every (kind, id) the differential below asks both paths about.
fn differential_subjects(shared_id: &str) -> Vec<Subject> {
    vec![
        subject(STORED_AS, shared_id),
        subject(aep_backend_entity::RELATIONS_AS, shared_id),
        subject(aep_backend_entity::AUDIT_AS, "000000000000000001"),
        subject(aep_backend_entity::APPLIED_AS, "idempotency-key-1"),
        subject(STORED_AS, "01JABSENT0000000000000001"),
        subject(aep_backend_entity::RELATIONS_AS, "a-relation-nothing-wrote"),
        subject(aep_backend_entity::AUDIT_AS, "000000000000000099"),
    ]
}

/// What the retained capture answers must be what the provider would have answered.
///
/// The acceptance turns on a capture standing in for per-subject reads, which is only sound if
/// the two answer the same thing. Asked for every kind `hydrate` reads — `aep.entity`,
/// `aep.relation`, `aep.audit`, `aep.applied` — for a subject whose id exists under **two**
/// kinds, and for three ids nothing wrote.
#[test]
fn what_the_retained_capture_answers_is_what_the_provider_would_have_answered() {
    let shared_id = "01JSHARED000000000000001";
    let provider = FaithfulProvider::default();

    let stored = subject(STORED_AS, shared_id);
    let (_, artifact_row) = artifact(1);
    provider.put(
        history_with_one_observation(&stored, "obs-stored", 11),
        row(
            &stored,
            1,
            &json!({ "document": artifact_row.fields["document"].clone() }),
        ),
    );

    let relation = subject(aep_backend_entity::RELATIONS_AS, shared_id);
    provider.put(
        history_with_one_observation(&relation, "obs-relation", 12),
        control_row(&relation, 4, "the relation row, not the entity row"),
    );

    let audit = subject(aep_backend_entity::AUDIT_AS, "000000000000000001");
    provider.put(genesis(&audit), control_row(&audit, 1, "audit"));

    let applied = subject(aep_backend_entity::APPLIED_AS, "idempotency-key-1");
    provider.put(
        history_with_one_observation(&applied, "obs-applied", 13),
        control_row(&applied, 2, "applied"),
    );

    // One handle takes the capture the acceptance is about; the other never captures, so every
    // read on it is the single-subject provider read this change replaced.
    let captured = faithful_store(provider.clone());
    assert_eq!(
        captured.ids(STORED_AS).expect("ids"),
        vec![shared_id.to_owned()],
        "the capture is taken through the path the acceptance names"
    );
    let bridged = faithful_store(provider.clone());

    for subject in differential_subjects(shared_id) {
        let (entity, id) = (subject.entity.as_str(), subject.id.as_str());
        assert_eq!(
            captured.load(entity, id).expect("load from the capture"),
            bridged.load(entity, id).expect("load from the provider"),
            "`load({entity}, {id})` answered from the retained capture differs from the \
             provider's answer"
        );
        assert_eq!(
            captured
                .records(entity, id)
                .expect("records from the capture"),
            bridged
                .records(entity, id)
                .expect("records from the provider"),
            "`records({entity}, {id})` answered from the retained capture differs from the \
             provider's answer"
        );
        assert_eq!(
            captured
                .observations(entity, id)
                .expect("observations from the capture"),
            bridged
                .observations(entity, id)
                .expect("observations from the provider"),
            "`observations({entity}, {id})` answered from the retained capture differs from \
             the provider's answer"
        );
        assert_eq!(
            entity_store::EventProvider::events(&captured, entity, id)
                .expect("events from the capture"),
            entity_store::EventProvider::events(&bridged, entity, id)
                .expect("events from the provider"),
            "`events({entity}, {id})` answered from the retained capture differs from the \
             provider's answer"
        );
    }
}

/// An id that exists under two kinds is two subjects, and the capture must not confuse them.
///
/// Held on its own because it is the one comparison above whose two sides hold different bytes:
/// a lookup into the capture that matched on the id alone would answer a `aep.relation` read
/// with the `aep.entity` row and stay green on every case shipped with this change.
#[test]
fn a_capture_lookup_answers_the_kind_that_was_asked_for() {
    let shared_id = "01JSHARED000000000000002";
    let provider = FaithfulProvider::default();
    let stored = subject(STORED_AS, shared_id);
    let (_, artifact_row) = artifact(2);
    provider.put(
        genesis(&stored),
        row(
            &stored,
            1,
            &json!({ "document": artifact_row.fields["document"].clone() }),
        ),
    );
    let relation = subject(aep_backend_entity::RELATIONS_AS, shared_id);
    provider.put(
        genesis(&relation),
        control_row(&relation, 7, "the relation row"),
    );

    let handle = faithful_store(provider);
    assert_eq!(handle.ids(STORED_AS).expect("ids").len(), 1);

    let answered = handle
        .load(aep_backend_entity::RELATIONS_AS, shared_id)
        .expect("load the relation")
        .expect("the relation row is there");
    assert_eq!(
        answered.entity,
        aep_backend_entity::RELATIONS_AS,
        "a read for one kind was answered with another kind's row"
    );
    assert_eq!(
        answered.fields.get("marker").and_then(Value::as_str),
        Some("the relation row"),
        "a read for one kind was answered with another kind's document"
    );
}

/// A subject the capture does not name answers exactly as an unknown subject answers.
///
/// The capture lookup's absent arm synthesizes the empty genesis history the provider gives an
/// unknown subject. Nothing in the shipped cases reaches it: every read they make while a
/// capture is held is for a subject the capture holds.
#[test]
fn a_subject_the_capture_does_not_name_answers_as_an_unknown_subject() {
    let provider = FaithfulProvider::default();
    let (present, instance) = artifact(3);
    provider.put(
        history_with_one_observation(&present, "obs-present", 21),
        instance,
    );

    let captured = faithful_store(provider.clone());
    assert_eq!(captured.ids(STORED_AS).expect("ids").len(), 1);
    let bridged = faithful_store(provider);

    let absent = "01JNOTHINGWROTETHIS00001";
    assert_eq!(
        captured.load(STORED_AS, absent).expect("load"),
        None,
        "an id nothing wrote must be absent, not the first row in the capture"
    );
    assert_eq!(
        captured.records(STORED_AS, absent).expect("records"),
        bridged.records(STORED_AS, absent).expect("records"),
        "the capture's answer for an unknown subject differs from the provider's"
    );
    assert_eq!(
        captured
            .observations(STORED_AS, absent)
            .expect("observations"),
        bridged
            .observations(STORED_AS, absent)
            .expect("observations"),
        "the capture's answer for an unknown subject differs from the provider's"
    );
    assert!(
        captured
            .observations(STORED_AS, absent)
            .expect("observations")
            .is_empty(),
        "an unknown subject has no observations"
    );
}

/// When the fall-through runs on a store built the way `open` builds one: after a write.
///
/// The second review pass wrote this case to ask whether the fall-through the story's acceptance
/// named — *a read on a handle that has never called `ids()`* — is reachable, and measured that
/// it is not: `open` (`lib.rs:439-441`) constructs the store and then calls
/// `validate_legacy_boundaries()`, which retains a capture before the caller is given anything,
/// so no handle a caller holds has ever captured nothing. The code path is right and the records
/// describing it were wrong; the arm's real job is the state *after* a write through the handle
/// retires the capture, and that is what this pins. The assertion moved with it: the reviewer's
/// was `after.loads == before.loads + 1` on the first read of a freshly opened handle, which
/// asserts a state that cannot exist; this asserts `+ 0` there and `+ 1` once a write has
/// retired the capture.
#[test]
fn a_store_built_the_way_open_builds_one_falls_through_once_a_write_retires_the_capture() {
    let provider = CountingProvider::default();
    let (subject, instance) = artifact(1);
    provider.put(subject.clone(), instance);
    let mut handle = store(provider.clone());

    // Exactly `open`'s next statement, and the only way a caller obtains one of these.
    handle
        .validate_legacy_boundaries()
        .expect("no legacy boundaries to disagree with");

    let opened = provider.calls();
    assert!(handle.load(STORED_AS, &subject.id).expect("load").is_some());
    let read = provider.calls();
    assert_eq!(
        read.loads, opened.loads,
        "a handle straight out of `open` holds the boundary validation's capture, so the read \
         it answers costs the provider nothing: opened {opened:?} read {read:?}"
    );

    handle
        .commit(&creation("01JRETIRED000000000000001"), Expect::Absent)
        .expect("the commit is accepted");

    let written = provider.calls();
    assert!(handle.load(STORED_AS, &subject.id).expect("load").is_some());
    let after = provider.calls();
    assert_eq!(
        after.loads,
        written.loads + 1,
        "a write through the handle retires the capture, and the single-subject read after it \
         is the fall-through: written {written:?} after {after:?}"
    );
    assert_eq!(
        after.snapshots, written.snapshots,
        "and the fall-through is the single-subject read, not another whole capture: \
         written {written:?} after {after:?}"
    );
}

/// A handle answers every read from one instant, and a write through it is what moves that on.
///
/// The second review pass wrote this case asserting that the handle's second `ids` sees a row a
/// second writer added — *"a handle that has made no write of its own answered from the capture
/// it took when it opened, and the authority has moved since"*. The coordinator settled that as
/// a `no-op`: a handle is one command, and within one command the reads are deliberately
/// mutually consistent at the capture's instant, which is what makes `ids` and the `load` of
/// every id it named one capture instead of `5 + 2N`. No holder in this repository outlives a
/// command (the enumeration is in the review reports and the rule is in the `retained` field's
/// doc comment), so the assertion moved to the behaviour that was decided: `ids` holds its
/// instant, and the write through the handle is what re-captures. Nothing was dropped — the
/// reviewer's second `ids` call is still made and still compared, against the other value.
#[test]
fn a_read_only_handle_answers_ids_from_one_instant_until_a_write_through_it_retires_the_capture() {
    let provider = CountingProvider::default();
    let (first, first_row) = artifact(1);
    provider.put(first.clone(), first_row);
    let mut handle = store(provider.clone());

    assert_eq!(handle.ids(STORED_AS).expect("ids"), vec![first.id.clone()]);

    // A second writer on the same authority — another process, or another handle in this one.
    let (second, second_row) = artifact(2);
    provider.put(second.clone(), second_row);

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![first.id.clone()],
        "every read on one handle answers from one instant: this is the constraint that makes a \
         handle one command, and a holder that wants to see a foreign write reopens"
    );

    // A write through this handle retires the capture, so the next read takes a fresh one.
    handle
        .commit(&creation("01JREOPENED00000000000001"), Expect::Absent)
        .expect("the commit is accepted");

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![first.id, second.id, "01JREOPENED00000000000001".to_owned()],
        "and the capture a write retires is taken again on the next read, which therefore sees \
         the other writer's row as well as this handle's own"
    );
}

/// A subject the capture does not name is asked of the provider, not answered out of the capture.
///
/// The reviewers' differential cases compare an *unknown* subject's two answers, and those agree
/// whatever the absent arm does: the provider answers an unknown subject with the same empty
/// genesis a synthesized one would carry. This is the case that can tell them apart — the
/// provider knows the subject and the capture does not, which is every subject written after
/// this handle captured. A capture-miss arm that answered out of the capture, by synthesizing a
/// genesis or by taking the first entry, answers here with something the authority does not
/// hold.
#[test]
fn a_subject_the_capture_does_not_name_is_read_from_the_provider() {
    let provider = CountingProvider::default();
    let (captured, captured_row) = artifact(1);
    provider.put(captured.clone(), captured_row);
    let handle = store(provider.clone());

    assert_eq!(
        handle.ids(STORED_AS).expect("ids"),
        vec![captured.id.clone()]
    );

    // Written after this handle captured, so the capture cannot name it.
    let (later, later_row) = artifact(2);
    provider.put(later.clone(), later_row);
    provider.put_history(
        later.clone(),
        history_with_one_observation(&later, "obs-later", 31),
    );

    let before = provider.calls();
    let loaded = handle
        .load(STORED_AS, &later.id)
        .expect("load")
        .expect("the authority holds the subject the capture does not name");
    assert_eq!(
        loaded.id, later.id,
        "the read answered with some other subject's row"
    );
    assert_eq!(
        loaded.fields.get("title").and_then(Value::as_str),
        Some("counted 2"),
        "the read answered with some other subject's document"
    );
    assert_eq!(
        handle
            .observations(STORED_AS, &later.id)
            .expect("observations")
            .len(),
        1,
        "the subject's history is the provider's, not an empty genesis this handle invented"
    );

    let after = provider.calls();
    assert_eq!(
        (after.loads, after.histories, after.snapshots),
        (before.loads + 1, before.histories + 1, before.snapshots),
        "and a capture miss costs exactly the single-subject read it falls through to, not \
         another whole capture: before {before:?} after {after:?}"
    );
}
