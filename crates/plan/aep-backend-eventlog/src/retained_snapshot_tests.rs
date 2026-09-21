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
    AppendOutcome, BatchKey, CompleteStoreSnapshot, HistoryOrigin, StoreCoverage, StoredBatch,
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
/// `histories`: each one is a separate `capture_tenant` on the authority.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Calls {
    snapshots: usize,
    loads: usize,
    histories: usize,
    batches: usize,
    observations: usize,
}

impl Calls {
    /// What the provider was asked to do in total — the figure the story is about.
    const fn reads(self) -> usize {
        self.snapshots + self.loads + self.histories
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
        Ok(CompleteStoreSnapshot {
            scope: scope.to_owned(),
            coverage: StoreCoverage::CompleteSnapshot,
            histories: held
                .terminals
                .iter()
                .map(|(subject, terminal)| SubjectSnapshot {
                    history: genesis(subject),
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
        Ok(genesis(subject))
    }

    fn lookup_batch(&self, _key: &BatchKey) -> Result<Option<StoredBatch>, SyncReadError> {
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
                other @ BatchAction::Observe(_) => panic!("unexpected batch action {other:?}"),
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
