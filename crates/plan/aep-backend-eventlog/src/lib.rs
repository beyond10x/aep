//! AEP's interaction contract over the recorded file Eventlog adapter.
//!
//! The existing planning contract represents its provider rows as legacy-import decisions. Those
//! decisions were valid for the old current-state providers, but a recorded Eventlog authority
//! quite correctly refuses to append them as new history. This adapter translates each atomic AEP
//! provider batch into real kernel create/replace commands over a closed JSON document field. The
//! AEP state and original AEP events remain the document being decided on; reopening unwraps that
//! exact document before the existing backend hydrates.

#![allow(missing_docs)]

#[cfg(test)]
mod retained_snapshot_tests;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU16;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use aep_backend_entity::{EntityBackend, Identity, PlanningCommit, PlanningStore};
use entity_core::{Decision, DomainEvent, EntityDefinition, EntityInstance, Registry};
use entity_eventlog::sync::{
    BridgeConfig, CallWait, EventlogRecordedStoreOwner, RecordedEventlogBridge, SyncExecutionError,
    SyncReadError,
};
use entity_eventlog::{
    AsyncBindingProvisioner, AsyncImportedAnchorWriter, ErRecordedProjector,
    EventlogBindingProvisioner, EventlogRecordedStore,
};
use entity_eventlog::{Authority, EventlogOperationContext};
use entity_executor::{BatchAction, CreateRequest, ExecuteRequest};
use entity_store::asynchronous::{
    AppendOutcome, AsyncRecordedReader, CompleteStoreSnapshot, StoredBatch, SubjectHistory,
};
use entity_store::asynchronous::{BatchKey, HistoryOrigin, LegacyEvidence, RecordedEntry, Subject};
use entity_store::{
    AtomicCommit, EventProvider, Expect, HistoryProvider, RecordedCommit, RecordedObservation,
    Recording, StateProvider, Store, StoreError,
};
use eventlog_core::{EventStore, InlineProjectionAdmin};
use serde_json::{json, Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Internal ordinary-authority subject type for invocation reservations and receipt prefixes.
/// It is deliberately absent from the Markdown planning projection inventory.
pub const INVOCATION_AS: &str = "aep.planning-invocation";
/// Provider-owned projection join metadata. A watermark names the verified authority prefix that
/// was projected; later complete captures retain this subject and its receipt like every other.
pub const PROJECTION_METADATA_AS: &str = "aep.planning-projection-metadata";
const LEGACY_COORDINATE_AS: &str = "aep.migration.LegacyRecordCoordinate";
const LEGACY_EVIDENCE_AS: &str = "aep.migration.LegacyEvidenceBlob";
const LEGACY_ROSTER_AS: &str = "aep.migration.LegacyIdReservationRoster";
const LEGACY_IMPORT_BOUNDARY_AS: &str = "aep.planning-import-boundary";

/// The order actually established by a retained legacy source, never a presentation index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyBoundaryOrder {
    Store,
    Subject,
    PerKind,
    Unavailable,
}

/// A complete imported envelope's kind, separate from newly recorded suffix entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyBoundaryKind {
    Decision,
    Observation,
}

/// Exact imported envelope bytes and their validated provider-complete provenance.
///
/// Collection order is by coordinate subject ID for stable presentation; `order` and `ordinal`
/// alone describe historical order. This value carries no newly recorded receipt or revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedLegacyEvidence {
    pub source_snapshot: aep_contract::migration::DigestV1,
    pub source_locator: String,
    pub boundary_id: String,
    pub coordinate_subject_id: String,
    pub evidence_blob_subject_id: String,
    pub reservation_roster_id: String,
    pub destination_entity: String,
    pub destination_id: String,
    pub original_record_id: String,
    pub kind: LegacyBoundaryKind,
    pub order: LegacyBoundaryOrder,
    pub ordinal: aep_contract::migration::PresenceV1<u64>,
    pub envelope_digest: aep_contract::migration::DigestV1,
    pub exact_bytes: Vec<u8>,
}

const CAPTURE_LIMITS: eventlog_core::CaptureLimits = eventlog_core::CaptureLimits {
    max_events: 1_000_000,
    max_blobs: 1_000_000,
    max_projection_rows: 1_000_000,
    max_payload_bytes: 4 * 1024 * 1024 * 1024,
};

/// Exact provider coordinates returned after binding provisioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedBinding {
    pub event_id: String,
    pub global_seq: u64,
    pub stream_id: String,
    pub stream_version: u64,
    pub replayed: bool,
}

/// Prepares an owned file provider and returns its provider-minted stream identity.
///
/// The identity cannot be selected by the caller. A migration intent that binds a newly created
/// file authority must retain this exact value before it can publish the immutable ER binding.
pub fn prepare_file(path: &Path, tenant: &str) -> Result<String, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing Eventlog preparation runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = Arc::new(
            eventlog_file::FileEventStore::open(path)
                .await
                .map_err(|error| format!("opening Eventlog destination: {error}"))?,
        );
        concrete
            .create_projections(Arc::new(ErRecordedProjector::new()))
            .await
            .map_err(|error| format!("admitting Eventlog projection: {error}"))?;
        let tenant = eventlog_core::TenantId::new(tenant.to_owned())
            .map_err(|error| format!("invalid Eventlog tenant: {error}"))?;
        concrete
            .stream_identity(&tenant)
            .await
            .map_err(|error| format!("reading Eventlog stream identity: {error}"))
    })
}

/// Provisions or recovers one immutable file Eventlog binding.
///
/// The caller must hold the migration's explicit destination writer-control guard. This function
/// does not infer exclusion from the path or from process state.
pub fn provision_file(
    path: &Path,
    logical_scope: String,
    tenant: String,
    stream_identity: String,
    context: EventlogOperationContext,
) -> Result<ProvisionedBinding, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing Eventlog provisioning runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = Arc::new(
            eventlog_file::FileEventStore::open(path)
                .await
                .map_err(|error| format!("opening Eventlog destination: {error}"))?,
        );
        let projector = Arc::new(ErRecordedProjector::new());
        concrete
            .create_projections(projector.clone())
            .await
            .map_err(|error| format!("admitting Eventlog projection: {error}"))?;
        concrete
            .attach_inline_existing(projector)
            .await
            .map_err(|error| format!("attaching Eventlog projection: {error}"))?;
        let tenant_id = eventlog_core::TenantId::new(tenant.clone())
            .map_err(|error| format!("invalid Eventlog tenant: {error}"))?;
        let actual_stream_identity = concrete
            .stream_identity(&tenant_id)
            .await
            .map_err(|error| format!("reading Eventlog stream identity: {error}"))?;
        if actual_stream_identity != stream_identity {
            return Err(format!(
                "Eventlog destination stream identity is {actual_stream_identity}, not requested {stream_identity}"
            ));
        }
        let backend: Arc<dyn entity_eventlog::EventlogBackend> = concrete;
        let provisioner = EventlogBindingProvisioner::new(backend, CAPTURE_LIMITS);
        let authority = Authority {
            logical_scope,
            tenant,
            stream_identity,
        };
        let outcome = provisioner
            .provision_binding(authority, context)
            .await
            .map_err(|error| format!("provisioning Eventlog binding: {error:?}"))?;
        Ok(ProvisionedBinding {
            event_id: outcome.physical.event_id,
            global_seq: outcome.physical.global_seq,
            stream_id: outcome.physical.stream_id,
            stream_version: outcome.physical.stream_version,
            replayed: outcome.replayed,
        })
    })
}

/// Imports exact legacy subject boundaries into an already provisioned file authority.
pub fn import_file_anchors(
    path: &Path,
    authority: Authority,
    context: EventlogOperationContext,
    histories: Vec<SubjectHistory>,
) -> Result<Vec<bool>, String> {
    with_async_store(path, authority, |store| async move {
        let operation = store.operation(context);
        let mut replayed = Vec::with_capacity(histories.len());
        for history in histories {
            let outcome = operation
                .import_anchor(history)
                .await
                .map_err(|error| format!("importing Eventlog boundary: {error:?}"))?;
            replayed.push(outcome.replayed);
        }
        Ok(replayed)
    })
}

/// Obtains the adapter's provider-complete logical snapshot.
pub fn complete_file_snapshot(
    path: &Path,
    authority: Authority,
) -> Result<CompleteStoreSnapshot, String> {
    let scope = authority.logical_scope.clone();
    with_async_store(path, authority, |store| async move {
        store
            .complete_snapshot(&scope)
            .await
            .map_err(|error| format!("capturing complete Eventlog authority: {error}"))
    })
}

/// Rebuilds the fixed provider indexes and then returns a fresh complete snapshot.
pub fn rebuild_file_indexes(
    path: &Path,
    authority: Authority,
) -> Result<CompleteStoreSnapshot, String> {
    let scope = authority.logical_scope.clone();
    with_async_store(path, authority, |store| async move {
        store
            .rebuild_indexes()
            .await
            .map_err(|error| format!("rebuilding Eventlog indexes: {error}"))?;
        store
            .complete_snapshot(&scope)
            .await
            .map_err(|error| format!("capturing rebuilt Eventlog authority: {error}"))
    })
}

/// Reads one closed invocation control document from the selected authority.
pub fn read_file_invocation(
    path: PathBuf,
    authority: Authority,
    identity: &str,
) -> Result<Option<(u64, Value, entity_store::asynchronous::CommitReceipt)>, String> {
    read_file_control(path, authority, INVOCATION_AS, identity)
}

pub fn read_file_control(
    path: PathBuf,
    authority: Authority,
    entity: &str,
    identity: &str,
) -> Result<Option<(u64, Value, entity_store::asynchronous::CommitReceipt)>, String> {
    let bridge = control_bridge(path, authority)?;
    let subject = Subject::new(entity, identity)
        .map_err(|error| format!("invalid invocation identity: {error}"))?;
    let instance = bridge
        .load(&subject, CallWait::Forever)
        .map_err(|error| format!("reading invocation authority subject: {error:?}"))?
        .map(|instance| {
            let document =
                instance.fields.get("document").cloned().ok_or_else(|| {
                    "invocation authority subject has no document field".to_owned()
                })?;
            Ok::<_, String>((instance.revision, document))
        })
        .transpose()?;
    let Some((revision, document)) = instance else {
        return Ok(None);
    };
    let batch = bridge
        .lookup_batch(&BatchKey::Named(identity.to_owned()), CallWait::Forever)
        .map_err(|error| format!("recovering invocation reservation receipt: {error:?}"))?
        .ok_or_else(|| {
            "invocation subject exists without its named reservation batch".to_owned()
        })?;
    Ok(Some((revision, document, batch.receipt)))
}

/// Creates or advances one invocation's ordinary authority subject with an exact named batch key.
pub fn write_file_invocation(
    path: PathBuf,
    authority: Authority,
    identity: String,
    batch_key: String,
    document: Value,
    expected_revision: Option<u64>,
    context: EventlogOperationContext,
) -> Result<entity_store::asynchronous::CommitReceipt, String> {
    write_file_control(
        path,
        authority,
        INVOCATION_AS,
        identity,
        batch_key,
        document,
        expected_revision,
        context,
    )
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)] // The public control write names every authority, identity, document and causal coordinate.
pub fn write_file_control(
    path: PathBuf,
    authority: Authority,
    entity: &str,
    identity: String,
    batch_key: String,
    document: Value,
    expected_revision: Option<u64>,
    context: EventlogOperationContext,
) -> Result<entity_store::asynchronous::CommitReceipt, String> {
    let bridge = control_bridge(path, authority)?;
    let subject = Subject::new(entity, &identity)
        .map_err(|error| format!("invalid invocation identity: {error}"))?;
    let recording = Recording {
        record_id: format!(
            "{identity}@{}",
            expected_revision.map_or(1, |value| value + 1)
        ),
        recorded_at: context
            .occurred_at
            .format(&Rfc3339)
            .map_err(|error| format!("formatting invocation timestamp: {error}"))?,
        correlation: Some(context.trace_id.clone()),
        causation: Some(identity.clone()),
        actor: Some(context.actor.clone()),
    };
    let action = match expected_revision {
        None => BatchAction::Create(CreateRequest {
            subject,
            definition_version: 1,
            fields: json!({ "document": document }),
            recording,
        }),
        Some(expected_revision) => BatchAction::Execute(ExecuteRequest {
            subject,
            expected_revision,
            operation: "replace".to_owned(),
            arguments: json!({ "document": document }),
            fulfillments: BTreeMap::new(),
            recording,
        }),
    };
    let outcome = bridge
        .operation(context)
        .batch(BatchKey::Named(batch_key), vec![action], CallWait::Forever)
        .map_err(|error| format!("writing invocation authority subject: {error:?}"))?;
    let receipt = outcome
        .receipt()
        .cloned()
        .ok_or_else(|| "invocation authority write returned no immutable receipt".to_owned())?;
    Ok(receipt)
}

fn control_bridge(path: PathBuf, authority: Authority) -> Result<RecordedEventlogBridge, String> {
    RecordedEventlogBridge::start(
        registry()?,
        EventlogRecordedStoreOwner::File {
            path,
            authority,
            limits: CAPTURE_LIMITS,
        },
        BridgeConfig {
            queue_capacity: NonZeroU16::new(32).expect("nonzero queue"),
        },
    )
    .map_err(|error| format!("opening invocation authority bridge: {error:?}"))
}

fn with_async_store<T, F, Fut>(path: &Path, authority: Authority, operation: F) -> Result<T, String>
where
    F: FnOnce(EventlogRecordedStore) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing Eventlog operation runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = Arc::new(
            eventlog_file::FileEventStore::open(path)
                .await
                .map_err(|error| format!("opening Eventlog authority: {error}"))?,
        );
        concrete
            .attach_inline_existing(Arc::new(ErRecordedProjector::new()))
            .await
            .map_err(|error| format!("attaching Eventlog projection: {error}"))?;
        let backend: Arc<dyn entity_eventlog::EventlogBackend> = concrete;
        let store = EventlogRecordedStore::open(backend, authority, CAPTURE_LIMITS)
            .await
            .map_err(|error| format!("opening recorded Eventlog authority: {error}"))?;
        operation(store).await
    })
}

/// The selected AEP backend over an owned recorded Eventlog bridge.
pub type EventlogBackend = EntityBackend<EventlogPlanningStore, Identity>;

/// Opens an already provisioned Eventlog planning authority.
///
/// This operation is read-only at provider-open time. Provisioning is an explicit migration phase.
pub fn open(
    path: PathBuf,
    logical_scope: String,
    tenant: String,
    stream_identity: String,
) -> Result<EventlogBackend, String> {
    let authority = Authority {
        logical_scope,
        tenant,
        stream_identity,
    };
    let registry = registry()?;
    let bridge = RecordedEventlogBridge::start(
        registry,
        EventlogRecordedStoreOwner::File {
            path,
            authority: authority.clone(),
            limits: CAPTURE_LIMITS,
        },
        BridgeConfig {
            queue_capacity: NonZeroU16::new(32).expect("nonzero queue"),
        },
    )
    .map_err(|error| format!("opening recorded Eventlog planning authority: {error:?}"))?;
    let store = EventlogPlanningStore::new(bridge, authority);
    store.validate_legacy_boundaries()?;
    EntityBackend::over(store).map_err(|error| error.to_string())
}

/// Every provider call an [`EventlogPlanningStore`] makes, named once.
///
/// The production implementation forwards to [`RecordedEventlogBridge`]. It is a trait because
/// the cost of this store is measured in *calls*, not in latency: each read below is one
/// `capture_tenant` on the authority, and a capture re-reads and re-hashes every bound object —
/// about 11 MB for the migrated planning store. How many a read path makes is observable only by
/// counting them, and a test counts them through this.
pub trait RecordedPlanningProvider {
    /// One consistent capture of the whole logical store.
    fn complete_snapshot(&self, scope: &str) -> Result<CompleteStoreSnapshot, SyncReadError>;
    /// One subject's terminal state.
    fn load(&self, subject: &Subject) -> Result<Option<EntityInstance>, SyncReadError>;
    /// One subject's mixed history.
    fn history(&self, subject: &Subject) -> Result<SubjectHistory, SyncReadError>;
    /// One previously committed named batch.
    fn lookup_batch(&self, key: &BatchKey) -> Result<Option<StoredBatch>, SyncReadError>;
    /// One ordered atomic recorded batch.
    fn batch(
        &self,
        context: EventlogOperationContext,
        key: BatchKey,
        actions: Vec<BatchAction>,
    ) -> Result<AppendOutcome, SyncExecutionError>;
    /// One observation at a subject's current revision.
    fn observe(
        &self,
        context: EventlogOperationContext,
        observation: RecordedObservation,
    ) -> Result<AppendOutcome, SyncExecutionError>;
}

impl RecordedPlanningProvider for RecordedEventlogBridge {
    fn complete_snapshot(&self, scope: &str) -> Result<CompleteStoreSnapshot, SyncReadError> {
        Self::complete_snapshot(self, scope, CallWait::Forever)
    }

    fn load(&self, subject: &Subject) -> Result<Option<EntityInstance>, SyncReadError> {
        Self::load(self, subject, CallWait::Forever)
    }

    fn history(&self, subject: &Subject) -> Result<SubjectHistory, SyncReadError> {
        Self::history(self, subject, CallWait::Forever)
    }

    fn lookup_batch(&self, key: &BatchKey) -> Result<Option<StoredBatch>, SyncReadError> {
        Self::lookup_batch(self, key, CallWait::Forever)
    }

    fn batch(
        &self,
        context: EventlogOperationContext,
        key: BatchKey,
        actions: Vec<BatchAction>,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        self.operation(context)
            .batch(key, actions, CallWait::Forever)
    }

    fn observe(
        &self,
        context: EventlogOperationContext,
        observation: RecordedObservation,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        self.operation(context)
            .observe(observation, CallWait::Forever)
    }
}

/// Synchronous compatibility facade over the public recorded Eventlog bridge.
pub struct EventlogPlanningStore<P = RecordedEventlogBridge> {
    provider: P,
    authority: Authority,
    /// The last complete capture this handle took, held until a write goes through this handle.
    ///
    /// A complete capture already carries, per subject, both the terminal state `load` answers
    /// with and the history `history` answers with (`entity-store`'s `SubjectSnapshot`), and it
    /// costs the provider exactly what a single-subject read costs: one `capture_tenant`. So a
    /// hydration that asks for `ids` and then reads every subject it named can be one capture
    /// instead of one per record, and it is also *one* instant rather than one per record.
    ///
    /// It is never held across a write through this handle: every write calls [`Self::retire`]
    /// after appending, so no read answers from a view the write invalidated, and a write that
    /// reads before it appends — a planning batch validates the legacy boundaries and resolves
    /// its revision expectations — retires it first as well, so those checks see the authority as
    /// it is rather than as this handle last saw it.
    retained: Mutex<Option<Arc<CompleteStoreSnapshot>>>,
}

impl<P: RecordedPlanningProvider> EventlogPlanningStore<P> {
    fn new(provider: P, authority: Authority) -> Self {
        Self {
            provider,
            authority,
            retained: Mutex::new(None),
        }
    }

    /// The complete capture this handle answers from, taking one if it holds none.
    fn snapshot(&self) -> Result<Arc<CompleteStoreSnapshot>, SyncReadError> {
        if let Some(held) = self.retained.lock().expect("retained capture").as_ref() {
            return Ok(Arc::clone(held));
        }
        let captured = Arc::new(
            self.provider
                .complete_snapshot(&self.authority.logical_scope)?,
        );
        let mut slot = self.retained.lock().expect("retained capture");
        Ok(Arc::clone(slot.get_or_insert(captured)))
    }

    /// Drops the retained capture. A write through this handle does this on both sides of its
    /// append.
    fn retire(&self) {
        self.retained.lock().expect("retained capture").take();
    }

    /// One subject's terminal state, from the retained capture when this handle holds one.
    ///
    /// A handle that has captured nothing makes the single-subject read instead: one `load` and
    /// one complete capture cost the provider the same one `capture_tenant`, and the complete one
    /// additionally materializes every subject, which a caller reading one row never needs.
    fn terminal(&self, subject: &Subject) -> Result<Option<EntityInstance>, SyncReadError> {
        let held = self.retained.lock().expect("retained capture").clone();
        match held {
            Some(snapshot) => Ok(snapshot
                .histories
                .iter()
                .find(|value| value.history.subject == *subject)
                .map(|value| value.terminal.clone())),
            None => self.provider.load(subject),
        }
    }

    /// One subject's history, from the retained capture when this handle holds one.
    ///
    /// A subject the capture does not name gets the empty genesis history the provider answers
    /// an unknown subject with, so the two paths cannot disagree about an absent subject.
    fn subject_history(&self, subject: &Subject) -> Result<SubjectHistory, SyncReadError> {
        let held = self.retained.lock().expect("retained capture").clone();
        match held {
            Some(snapshot) => Ok(snapshot
                .histories
                .iter()
                .find(|value| value.history.subject == *subject)
                .map_or_else(
                    || SubjectHistory {
                        subject: subject.clone(),
                        origin: HistoryOrigin::Genesis,
                        records: Vec::new(),
                    },
                    |value| value.history.clone(),
                )),
            None => self.provider.history(subject),
        }
    }
}

impl<P: RecordedPlanningProvider> StateProvider for EventlogPlanningStore<P> {
    fn load(&self, entity: &str, id: &str) -> Result<Option<EntityInstance>, StoreError> {
        let subject = Subject::new(entity, id).map_err(async_error)?;
        self.terminal(&subject)
            .map_err(read_error)?
            .map(unpack)
            .transpose()
    }

    fn ids(&self, entity: &str) -> Result<Vec<String>, StoreError> {
        let snapshot = self.snapshot().map_err(read_error)?;
        let mut ids: Vec<_> = snapshot
            .histories
            .iter()
            .filter(|snapshot| snapshot.history.subject.entity == entity)
            .map(|snapshot| snapshot.history.subject.id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }
}

impl<P: RecordedPlanningProvider> EventProvider for EventlogPlanningStore<P> {
    fn events(&self, entity: &str, id: &str) -> Result<Vec<DomainEvent>, StoreError> {
        Ok(self
            .events_in_store_order(entity, id)?
            .into_iter()
            .map(|(_, event)| event)
            .collect())
    }
}

impl<P: RecordedPlanningProvider> HistoryProvider for EventlogPlanningStore<P> {
    fn records(
        &self,
        entity: &str,
        id: &str,
    ) -> Result<Vec<entity_store::Envelope<entity_core::DecisionRecord>>, StoreError> {
        let subject = Subject::new(entity, id).map_err(async_error)?;
        let history = self.subject_history(&subject).map_err(read_error)?;
        Ok(history
            .records
            .into_iter()
            .filter_map(|record| match record.entry {
                RecordedEntry::Decision(commit) => Some(commit.envelope),
                RecordedEntry::Observation(_) => None,
            })
            .collect())
    }

    fn observations(&self, entity: &str, id: &str) -> Result<Vec<RecordedObservation>, StoreError> {
        let subject = Subject::new(entity, id).map_err(async_error)?;
        let history = self.subject_history(&subject).map_err(read_error)?;
        Ok(history
            .records
            .into_iter()
            .filter_map(|record| match record.entry {
                RecordedEntry::Observation(observation) => Some(observation),
                RecordedEntry::Decision(_) => None,
            })
            .collect())
    }
}

impl<P: RecordedPlanningProvider> Store for EventlogPlanningStore<P> {
    fn history(&self) -> Option<&dyn HistoryProvider> {
        Some(self)
    }

    fn commit(&mut self, decision: &Decision, expect: Expect) -> Result<(), StoreError> {
        let record_id = format!(
            "{}:{}@{}",
            decision.instance.entity, decision.instance.id, decision.instance.revision
        );
        self.commit_planning_batch(&[PlanningCommit {
            commit: AtomicCommit::new(decision.clone(), expect),
            recording: Recording {
                record_id,
                recorded_at: "1970-01-01T00:00:00Z".to_owned(),
                correlation: None,
                causation: None,
                actor: Some("aep".to_owned()),
            },
        }])
        .map(|_| ())
    }

    fn commit_recorded(
        &mut self,
        commit: &RecordedCommit,
        expect: Expect,
    ) -> Result<(), StoreError> {
        self.commit_planning_batch(&[PlanningCommit {
            commit: AtomicCommit::new(commit.decision(), expect),
            recording: Recording {
                record_id: commit.envelope.record_id.clone(),
                recorded_at: commit.envelope.recorded_at.clone(),
                correlation: commit.envelope.correlation.clone(),
                causation: commit.envelope.causation.clone(),
                actor: commit.envelope.actor.clone(),
            },
        }])
        .map(|_| ())
    }

    fn observe(&mut self, observation: &RecordedObservation) -> Result<(), StoreError> {
        let subject = Subject::new(&observation.entity, &observation.id).map_err(async_error)?;
        let context = context_from_parts(
            observation.envelope.actor.as_ref(),
            &observation.envelope.record_id,
            observation.envelope.correlation.as_ref(),
            observation.envelope.causation.as_ref(),
            &observation.envelope.recorded_at,
        );
        // An observation reads nothing before it appends, so only the retained capture behind it
        // has to go: a read after this must not answer from before it.
        let outcome = self.provider.observe(context, observation.clone());
        self.retire();
        outcome.map_err(execution_error)?;
        let _ = subject;
        Ok(())
    }
}

impl<P: RecordedPlanningProvider> PlanningStore for EventlogPlanningStore<P> {
    /// Commits every member in order or commits none, against the authority as it is now.
    ///
    /// The retained capture is retired on both sides of the append: before, so the boundary
    /// validation and the revision expectations below read the current authority rather than
    /// whatever this handle last saw; after, so no later read answers from before the write.
    fn commit_planning_batch(
        &mut self,
        commits: &[PlanningCommit],
    ) -> Result<Option<entity_store::asynchronous::CommitReceipt>, StoreError> {
        if commits.is_empty() {
            return Ok(None);
        }
        self.retire();
        let outcome = self.append_planning_batch(commits);
        self.retire();
        outcome
    }

    fn recover_planning_receipt(
        &self,
        key: &str,
    ) -> Result<Option<entity_store::asynchronous::CommitReceipt>, StoreError> {
        Ok(self
            .provider
            .lookup_batch(&BatchKey::Named(key.to_owned()))
            .map_err(read_error)?
            .map(|batch| batch.receipt))
    }
}

impl<P: RecordedPlanningProvider> EventlogPlanningStore<P> {
    fn append_planning_batch(
        &self,
        commits: &[PlanningCommit],
    ) -> Result<Option<entity_store::asynchronous::CommitReceipt>, StoreError> {
        let reserved = self
            .validate_legacy_boundaries()
            .map_err(StoreError::Backend)?;
        if let Some(record_id) = commits
            .iter()
            .map(|member| member.recording.record_id.as_str())
            .find(|record_id| reserved.contains(*record_id))
        {
            return Err(StoreError::RecordConflict {
                record_id: record_id.to_owned(),
            });
        }
        let key = commits[0]
            .recording
            .causation
            .clone()
            .or_else(|| commits[0].recording.correlation.clone())
            .unwrap_or_else(|| commits[0].recording.record_id.clone());
        let context = context_from_recording(&commits[0].recording);
        let mut actions = Vec::with_capacity(commits.len());
        let mut pending_revisions = BTreeMap::new();
        for member in commits {
            let instance = &member.commit.decision.instance;
            let subject = Subject::new(&instance.entity, &instance.id).map_err(async_error)?;
            let document = json!({
                "lifecycle_state": instance.lifecycle_state,
                "fields": instance.fields,
                "events": member.commit.decision.record.events,
            });
            match member.commit.expect {
                Expect::Absent => {
                    pending_revisions.insert(subject.clone(), (instance.revision, 1));
                    actions.push(BatchAction::Create(CreateRequest {
                        subject,
                        definition_version: 1,
                        fields: json!({ "document": document }),
                        recording: member.recording.clone(),
                    }));
                }
                Expect::Revision(expected_revision) => {
                    let (logical, physical) = match pending_revisions.get(&subject) {
                        Some(revisions) => *revisions,
                        None => self.revisions_before(&subject, &member.recording.record_id)?,
                    };
                    if logical != expected_revision {
                        return Err(StoreError::RevisionConflict {
                            entity: instance.entity.clone(),
                            id: instance.id.clone(),
                            expected: member.commit.expect,
                            found: Some(logical),
                        });
                    }
                    let next = physical.checked_add(1).ok_or_else(|| {
                        StoreError::Backend("planning storage revision exhausted".to_owned())
                    })?;
                    pending_revisions.insert(subject.clone(), (instance.revision, next));
                    actions.push(BatchAction::Execute(ExecuteRequest {
                        subject,
                        expected_revision: physical,
                        operation: "replace".to_owned(),
                        arguments: json!({ "document": document }),
                        fulfillments: BTreeMap::new(),
                        recording: member.recording.clone(),
                    }));
                }
            }
        }
        let outcome = self
            .provider
            .batch(context, BatchKey::Named(key), actions)
            .map_err(execution_error)?;
        Ok(outcome.receipt().cloned())
    }
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCoordinateValue {
    format: String,
    source_snapshot: aep_contract::migration::DigestV1,
    source_locator: String,
    destination_entity: String,
    destination_id: String,
    evidence_kind: String,
    original_record_id: aep_contract::migration::PresenceV1<String>,
    order: String,
    ordinal: aep_contract::migration::PresenceV1<u64>,
    evidence_blob_id: String,
    envelope_digest: aep_contract::migration::DigestV1,
    reservation_roster_id: aep_contract::migration::PresenceV1<String>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEvidenceValue {
    format: String,
    source_snapshot: aep_contract::migration::DigestV1,
    coordinate_subject_id: String,
    original_record_id: aep_contract::migration::PresenceV1<String>,
    exact_bytes: aep_contract::migration::HexBytesV1,
    byte_length: u64,
    envelope_digest: aep_contract::migration::DigestV1,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyReservationEntryValue {
    record_id: String,
    coordinate_subject_id: String,
    evidence_blob_subject_id: String,
    envelope_digest: aep_contract::migration::DigestV1,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRosterValue {
    format: String,
    boundary_id: String,
    authority: aep_contract::migration::AuthorityCoordinateV1,
    entries: Vec<LegacyReservationEntryValue>,
    roster_digest: aep_contract::migration::DigestV1,
}

impl<P: RecordedPlanningProvider> EventlogPlanningStore<P> {
    /// AEP entity revisions do not advance for evidence, while the containing ER row does.
    /// A retry uses the actual predecessor of its original record, preserving ER's request
    /// comparison instead of rebuilding an expectation from the latest physical revision.
    fn revisions_before(
        &self,
        subject: &Subject,
        record_id: &str,
    ) -> Result<(u64, u64), StoreError> {
        let history = self.subject_history(subject).map_err(read_error)?;
        let mut prior = match history.origin {
            HistoryOrigin::Imported(anchor) => Some(anchor.instance),
            HistoryOrigin::Genesis => None,
        };
        for record in history.records {
            if record.entry.record_id() == record_id {
                break;
            }
            if let RecordedEntry::Decision(commit) = record.entry {
                prior = Some(commit.envelope.record.result);
            }
        }
        let prior = prior.ok_or_else(|| {
            StoreError::Backend("planning revision has no predecessor".to_owned())
        })?;
        let physical = prior.revision;
        Ok((unpack(prior)?.revision, physical))
    }

    /// Original Markdown journal lines for a migrated subject, in their declared source order.
    ///
    /// These are retained evidence bytes, not newly recorded events or decisions. SQL events
    /// already live in the imported anchor and are deliberately not returned here.
    pub fn legacy_journal_for_subject(
        &self,
        entity: &str,
        id: &str,
    ) -> Result<Vec<Vec<u8>>, StoreError> {
        Ok(self
            .legacy_journal_in_store_order(entity, id)?
            .into_iter()
            .map(|(_, line)| line)
            .collect())
    }

    /// The same retained lines, each with the ordinal the migration preserved for it.
    ///
    /// The ordinal is the line's index in the single `journal.jsonl` the store was migrated from,
    /// and the coordinate declares `order: store` for it — so it orders two lines about *different*
    /// subjects, which is the one thing the authority's per-entity history and its
    /// second-granularity instants cannot do. A reader that concatenated per-subject histories put
    /// a review's outcomes in subject-id order and called it oldest first; this is what it needs
    /// instead. The bytes and the join are validated exactly as before: only the key is kept.
    pub fn legacy_journal_in_store_order(
        &self,
        entity: &str,
        id: &str,
    ) -> Result<Vec<(u64, Vec<u8>)>, StoreError> {
        Subject::new(entity, id).map_err(async_error)?;
        let snapshot = self.snapshot().map_err(read_error)?;
        validate_legacy_boundary_snapshot(&snapshot, &self.authority)
            .map_err(StoreError::Backend)?;
        if !selected_history_uses_markdown(&snapshot).map_err(StoreError::Backend)? {
            return Ok(Vec::new());
        }
        let mut lines = BTreeMap::new();
        let mut source = None;
        for subject in &snapshot.histories {
            if subject.history.subject.entity != LEGACY_COORDINATE_AS {
                continue;
            }
            let coordinate: LegacyCoordinateValue = serde_json::from_value(Value::Object(
                subject.terminal.fields.clone(),
            ))
            .map_err(|error| StoreError::Backend(format!("invalid journal coordinate: {error}")))?;
            if coordinate.destination_entity != entity
                || coordinate.destination_id != id
                || !coordinate.source_locator.starts_with("journal.jsonl/")
            {
                continue;
            }
            let aep_contract::migration::PresenceV1::Present(ordinal) = coordinate.ordinal else {
                return Err(StoreError::Backend(
                    "legacy journal order is absent".to_owned(),
                ));
            };
            if coordinate.order != "store"
                || coordinate.source_locator != format!("journal.jsonl/{ordinal}")
                || !matches!(coordinate.evidence_kind.as_str(), "change" | "event")
                || source.is_some_and(|held| held != coordinate.source_snapshot)
                || !snapshot.histories.iter().any(|boundary| {
                    boundary.history.subject.entity == LEGACY_IMPORT_BOUNDARY_AS
                        && boundary.history.subject.id == coordinate.source_snapshot.as_wire()
                        && boundary.terminal.fields.get("source_snapshot")
                            == Some(&Value::String(coordinate.source_snapshot.as_wire()))
                })
            {
                return Err(StoreError::Backend(
                    "legacy journal source or order disagrees".to_owned(),
                ));
            }
            source = Some(coordinate.source_snapshot);
            let blob = snapshot
                .histories
                .iter()
                .find(|blob| {
                    blob.history.subject.entity == LEGACY_EVIDENCE_AS
                        && blob.history.subject.id == coordinate.evidence_blob_id
                })
                .expect("validated coordinate has an evidence blob");
            let blob: LegacyEvidenceValue = serde_json::from_value(Value::Object(
                blob.terminal.fields.clone(),
            ))
            .map_err(|error| StoreError::Backend(format!("invalid journal evidence: {error}")))?;
            if lines
                .insert(ordinal, blob.exact_bytes.as_bytes().to_vec())
                .is_some()
            {
                return Err(StoreError::Backend(
                    "legacy journal ordinal is duplicated".to_owned(),
                ));
            }
        }
        Ok(lines.into_iter().collect())
    }

    /// Every event one subject's history stands for, each with the store-wide position the
    /// provider keeps the record at, or `None` for evidence preserved in the imported anchor.
    ///
    /// The single reading path behind [`EventProvider::events`], which drops the positions: a
    /// second traversal of the same history would be a second answer waiting to drift from the
    /// first. `StoredRecord::position.store` is monotone across the whole logical store, so it
    /// orders records about different subjects — the authority's own instants are written to the
    /// second and two records made in one second are otherwise indistinguishable.
    pub fn events_in_store_order(
        &self,
        entity: &str,
        id: &str,
    ) -> Result<Vec<(Option<u64>, DomainEvent)>, StoreError> {
        let subject = Subject::new(entity, id).map_err(async_error)?;
        let history = self.subject_history(&subject).map_err(read_error)?;
        let mut events = Vec::new();
        if let HistoryOrigin::Imported(anchor) = history.origin {
            for evidence in anchor.evidence {
                match evidence {
                    LegacyEvidence::Envelope(envelope) => {
                        events.extend(envelope.entry.events().iter().cloned().map(|e| (None, e)));
                    }
                    LegacyEvidence::Decision(decision) => {
                        events.extend(decision.events.into_iter().map(|e| (None, e)));
                    }
                    LegacyEvidence::Event(event) => events.push((None, event)),
                }
            }
        }
        for record in history.records {
            let position = record.position.store;
            if let RecordedEntry::Decision(commit) = record.entry {
                if let Some(document) = decision_document(&commit) {
                    events.extend(
                        document
                            .events
                            .into_iter()
                            .map(|event| (Some(position), event)),
                    );
                }
            }
        }
        Ok(events)
    }

    /// Reads complete imported envelopes for one destination from a fresh validated authority.
    /// The vector is sorted by coordinate ID, not by inferred historical order.
    pub fn legacy_evidence_for_subject(
        &self,
        entity: &str,
        id: &str,
    ) -> Result<Vec<ImportedLegacyEvidence>, StoreError> {
        Subject::new(entity, id).map_err(async_error)?;
        Ok(self
            .read_legacy_evidence()?
            .into_iter()
            .filter(|value| value.destination_entity == entity && value.destination_id == id)
            .collect())
    }

    /// Looks up one complete imported envelope by its reserved original global record ID.
    pub fn legacy_evidence_by_original_id(
        &self,
        record_id: &str,
    ) -> Result<Option<ImportedLegacyEvidence>, StoreError> {
        Ok(self
            .read_legacy_evidence()?
            .into_iter()
            .find(|value| value.original_record_id == record_id))
    }

    fn read_legacy_evidence(&self) -> Result<Vec<ImportedLegacyEvidence>, StoreError> {
        let snapshot = self.snapshot().map_err(read_error)?;
        validated_legacy_boundary_snapshot(&snapshot, &self.authority)
            .map(|validated| validated.evidence)
            .map_err(StoreError::Backend)
    }

    /// Validates every closed coordinate/blob/roster join and returns the immutable original IDs.
    /// This read is part of backend open and every recorded command, so deleting or corrupting a
    /// boundary value cannot silently disable history lookup or collision protection.
    fn validate_legacy_boundaries(&self) -> Result<BTreeSet<String>, String> {
        let snapshot = self
            .snapshot()
            .map_err(|error| format!("reading legacy boundary evidence: {error:?}"))?;
        validate_legacy_boundary_snapshot(&snapshot, &self.authority)
    }
}

fn selected_history_uses_markdown(snapshot: &CompleteStoreSnapshot) -> Result<bool, String> {
    let mut selected = None;
    for boundary in &snapshot.histories {
        if boundary.history.subject.entity != LEGACY_IMPORT_BOUNDARY_AS {
            continue;
        }
        let raw = boundary
            .terminal
            .fields
            .get("raw_capture")
            .cloned()
            .ok_or_else(|| "legacy import boundary has no raw capture".to_owned())?;
        let raw: aep_contract::migration::LegacyRawCaptureV1 = serde_json::from_value(raw)
            .map_err(|error| format!("invalid legacy raw capture: {error}"))?;
        let uses_markdown = match raw {
            aep_contract::migration::LegacyRawCaptureV1::Markdown(_) => true,
            aep_contract::migration::LegacyRawCaptureV1::Sqlite(_)
            | aep_contract::migration::LegacyRawCaptureV1::Postgres(_) => false,
            aep_contract::migration::LegacyRawCaptureV1::Hybrid(raw) => {
                match raw.policy.authority.as_str() {
                    "local" => true,
                    "replica" => false,
                    other => {
                        return Err(format!(
                            "legacy hybrid boundary has unknown selected authority {other:?}"
                        ));
                    }
                }
            }
        };
        if selected.replace(uses_markdown).is_some() {
            return Err("multiple legacy import boundaries disagree with one source".to_owned());
        }
    }
    Ok(selected.unwrap_or(false))
}

/// Validates the immutable legacy coordinate/blob/roster graph in one provider-complete snapshot.
///
/// Exposed so migration acceptance can mutate a captured disposable snapshot and prove that a
/// missing or altered provider subject is causally refused without corrupting a real authority.
pub fn validate_legacy_boundary_snapshot(
    snapshot: &CompleteStoreSnapshot,
    authority: &Authority,
) -> Result<BTreeSet<String>, String> {
    validated_legacy_boundary_snapshot(snapshot, authority).map(|validated| validated.reserved)
}

struct ValidatedLegacyBoundary {
    reserved: BTreeSet<String>,
    evidence: Vec<ImportedLegacyEvidence>,
}

#[allow(clippy::too_many_lines)] // One pass joins every coordinate, blob, roster and source boundary.
fn validated_legacy_boundary_snapshot(
    snapshot: &CompleteStoreSnapshot,
    authority: &Authority,
) -> Result<ValidatedLegacyBoundary, String> {
    let mut coordinates = BTreeMap::new();
    let mut evidence = BTreeMap::new();
    for subject in &snapshot.histories {
        let id = subject.history.subject.id.clone();
        if subject.history.subject.entity == LEGACY_COORDINATE_AS {
            let value: LegacyCoordinateValue =
                serde_json::from_value(serde_json::Value::Object(subject.terminal.fields.clone()))
                    .map_err(|error| format!("invalid legacy coordinate: {error}"))?;
            coordinates.insert(id, value);
        } else if subject.history.subject.entity == LEGACY_EVIDENCE_AS {
            let value: LegacyEvidenceValue =
                serde_json::from_value(serde_json::Value::Object(subject.terminal.fields.clone()))
                    .map_err(|error| format!("invalid legacy evidence: {error}"))?;
            evidence.insert(id, value);
        }
    }
    let mut joined_blobs = BTreeSet::new();
    for (coordinate_id, coordinate) in &coordinates {
        let blob = evidence.get(&coordinate.evidence_blob_id).ok_or_else(|| {
            format!(
                "legacy evidence `{}` is absent",
                coordinate.evidence_blob_id
            )
        })?;
        let evidence_digest = aep_contract::migration::digest_parts_v1(
            "aep.migration.legacy-evidence/1",
            &[blob.exact_bytes.as_bytes().to_vec()],
        )
        .map_err(|error| format!("digesting legacy evidence: {error}"))?;
        if coordinate.format != "aep.legacy-record-coordinate/1"
            || blob.format != "aep.legacy-evidence-blob/1"
            || coordinate.source_snapshot != blob.source_snapshot
            || coordinate.source_locator.is_empty()
            || coordinate.destination_entity.is_empty()
            || coordinate.destination_id.is_empty()
            || !matches!(
                coordinate.evidence_kind.as_str(),
                "decision" | "observation" | "event" | "change"
            )
            || !matches!(
                coordinate.order.as_str(),
                "store" | "subject" | "per_kind" | "unavailable"
            )
            || (coordinate.order != "unavailable"
                && matches!(
                    coordinate.ordinal,
                    aep_contract::migration::PresenceV1::Missing
                ))
            || blob.coordinate_subject_id != *coordinate_id
            || blob.original_record_id != coordinate.original_record_id
            || blob.envelope_digest != coordinate.envelope_digest
            || blob.byte_length != blob.exact_bytes.as_bytes().len() as u64
            || evidence_digest != coordinate.envelope_digest
            || matches!(
                (
                    &coordinate.original_record_id,
                    &coordinate.reservation_roster_id
                ),
                (
                    aep_contract::migration::PresenceV1::Present(_),
                    aep_contract::migration::PresenceV1::Missing
                ) | (
                    aep_contract::migration::PresenceV1::Missing,
                    aep_contract::migration::PresenceV1::Present(_)
                )
            )
        {
            return Err(format!(
                "legacy boundary join for `{coordinate_id}` disagrees"
            ));
        }
        if !joined_blobs.insert(coordinate.evidence_blob_id.clone()) {
            return Err(format!(
                "legacy evidence `{}` is joined more than once",
                coordinate.evidence_blob_id
            ));
        }
    }
    if joined_blobs.len() != evidence.len() {
        return Err("legacy evidence contains an unjoined blob".to_owned());
    }
    let mut reserved = BTreeSet::new();
    let mut imported = Vec::new();
    let mut rostered_coordinates = BTreeSet::new();
    for subject in &snapshot.histories {
        if subject.history.subject.entity != LEGACY_ROSTER_AS {
            continue;
        }
        let roster: LegacyRosterValue =
            serde_json::from_value(serde_json::Value::Object(subject.terminal.fields.clone()))
                .map_err(|error| format!("invalid legacy reservation roster: {error}"))?;
        if roster.format != "aep.legacy-id-reservations/1"
            || roster.entries.is_empty()
            || roster.authority.logical_scope.as_str() != authority.logical_scope
            || roster.authority.tenant.as_str() != authority.tenant
            || roster.authority.stream_identity.as_str() != authority.stream_identity
        {
            return Err(
                "legacy reservation roster binding disagrees with this authority".to_owned(),
            );
        }
        let mut sorted = roster.entries.clone();
        sorted.sort_by(|left, right| {
            left.record_id
                .cmp(&right.record_id)
                .then_with(|| left.coordinate_subject_id.cmp(&right.coordinate_subject_id))
                .then_with(|| {
                    left.evidence_blob_subject_id
                        .cmp(&right.evidence_blob_subject_id)
                })
                .then_with(|| left.envelope_digest.cmp(&right.envelope_digest))
        });
        if sorted.iter().zip(&roster.entries).any(|(left, right)| {
            left.record_id != right.record_id
                || left.coordinate_subject_id != right.coordinate_subject_id
                || left.evidence_blob_subject_id != right.evidence_blob_subject_id
                || left.envelope_digest != right.envelope_digest
        }) {
            return Err("legacy reservation roster entries are not canonical".to_owned());
        }
        let parts = roster
            .entries
            .iter()
            .flat_map(|entry| {
                [
                    entry.record_id.as_bytes().to_vec(),
                    entry.coordinate_subject_id.as_bytes().to_vec(),
                    entry.evidence_blob_subject_id.as_bytes().to_vec(),
                    entry.envelope_digest.as_bytes().to_vec(),
                ]
            })
            .chain([
                authority.logical_scope.as_bytes().to_vec(),
                authority.tenant.as_bytes().to_vec(),
                authority.stream_identity.as_bytes().to_vec(),
                roster.boundary_id.as_bytes().to_vec(),
            ])
            .collect::<Vec<_>>();
        let digest = aep_contract::migration::digest_parts_v1(
            "aep.migration.legacy-id-reservations/1",
            &parts,
        )
        .map_err(|error| format!("digesting legacy reservation roster: {error}"))?;
        if digest != roster.roster_digest {
            return Err("legacy reservation roster digest disagrees".to_owned());
        }
        let boundary = snapshot
            .histories
            .iter()
            .find(|value| {
                value.history.subject.entity == LEGACY_IMPORT_BOUNDARY_AS
                    && value.history.subject.id == roster.boundary_id
            })
            .ok_or_else(|| format!("legacy import boundary `{}` is absent", roster.boundary_id))?;
        let boundary_source = boundary
            .terminal
            .fields
            .get("source_snapshot")
            .and_then(Value::as_str)
            .ok_or_else(|| "legacy import boundary has no source snapshot".to_owned())?;
        for entry in &roster.entries {
            if !rostered_coordinates.insert(entry.coordinate_subject_id.clone()) {
                return Err(format!(
                    "legacy coordinate `{}` is rostered more than once",
                    entry.coordinate_subject_id
                ));
            }
            if !reserved.insert(entry.record_id.clone()) {
                return Err(format!(
                    "legacy record id `{}` is reserved more than once",
                    entry.record_id
                ));
            }
            let coordinate = coordinates
                .get(&entry.coordinate_subject_id)
                .ok_or_else(|| {
                    format!(
                        "legacy coordinate `{}` is absent",
                        entry.coordinate_subject_id
                    )
                })?;
            let blob = evidence
                .get(&entry.evidence_blob_subject_id)
                .ok_or_else(|| {
                    format!(
                        "legacy evidence `{}` is absent",
                        entry.evidence_blob_subject_id
                    )
                })?;
            if coordinate.original_record_id
                != aep_contract::migration::PresenceV1::Present(entry.record_id.clone())
                || coordinate.evidence_blob_id != entry.evidence_blob_subject_id
                || coordinate.envelope_digest != entry.envelope_digest
                || coordinate.reservation_roster_id
                    != aep_contract::migration::PresenceV1::Present(
                        subject.history.subject.id.clone(),
                    )
                || blob.coordinate_subject_id != entry.coordinate_subject_id
                || blob.original_record_id
                    != aep_contract::migration::PresenceV1::Present(entry.record_id.clone())
                || blob.envelope_digest != entry.envelope_digest
                || coordinate.source_snapshot.as_wire() != boundary_source
            {
                return Err(format!(
                    "legacy boundary join for `{}` disagrees",
                    entry.record_id
                ));
            }
            let (kind, envelope) = match coordinate.evidence_kind.as_str() {
                "decision" => (
                    LegacyBoundaryKind::Decision,
                    serde_json::from_slice::<RecordedCommit>(blob.exact_bytes.as_bytes())
                        .map(RecordedEntry::Decision),
                ),
                "observation" => (
                    LegacyBoundaryKind::Observation,
                    serde_json::from_slice::<RecordedObservation>(blob.exact_bytes.as_bytes())
                        .map(RecordedEntry::Observation),
                ),
                _ => {
                    return Err(format!(
                        "legacy record `{}` is not an envelope",
                        entry.record_id
                    ))
                }
            };
            let envelope = envelope.map_err(|error| {
                format!(
                    "legacy record `{}` envelope is invalid: {error}",
                    entry.record_id
                )
            })?;
            if envelope.record_id() != entry.record_id
                || envelope.subject().entity != coordinate.destination_entity
                || envelope.subject().id != coordinate.destination_id
            {
                return Err(format!(
                    "legacy record `{}` envelope subject or identity disagrees",
                    entry.record_id
                ));
            }
            let order = match coordinate.order.as_str() {
                "store" => LegacyBoundaryOrder::Store,
                "subject" => LegacyBoundaryOrder::Subject,
                "per_kind" => LegacyBoundaryOrder::PerKind,
                "unavailable" => LegacyBoundaryOrder::Unavailable,
                _ => unreachable!("validated coordinate order"),
            };
            imported.push(ImportedLegacyEvidence {
                source_snapshot: coordinate.source_snapshot,
                source_locator: coordinate.source_locator.clone(),
                boundary_id: roster.boundary_id.clone(),
                coordinate_subject_id: entry.coordinate_subject_id.clone(),
                evidence_blob_subject_id: entry.evidence_blob_subject_id.clone(),
                reservation_roster_id: subject.history.subject.id.clone(),
                destination_entity: coordinate.destination_entity.clone(),
                destination_id: coordinate.destination_id.clone(),
                original_record_id: entry.record_id.clone(),
                kind,
                order,
                ordinal: coordinate.ordinal.clone(),
                envelope_digest: coordinate.envelope_digest,
                exact_bytes: blob.exact_bytes.as_bytes().to_vec(),
            });
        }
    }
    for (coordinate_id, coordinate) in &coordinates {
        if matches!(
            coordinate.original_record_id,
            aep_contract::migration::PresenceV1::Present(_)
        ) && !rostered_coordinates.contains(coordinate_id)
        {
            return Err(format!(
                "legacy coordinate `{coordinate_id}` has an original record id but no roster entry"
            ));
        }
    }
    imported.sort_by(|left, right| left.coordinate_subject_id.cmp(&right.coordinate_subject_id));
    Ok(ValidatedLegacyBoundary {
        reserved,
        evidence: imported,
    })
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningDocument {
    lifecycle_state: String,
    fields: Map<String, Value>,
    events: Vec<DomainEvent>,
}

fn unpack(instance: EntityInstance) -> Result<EntityInstance, StoreError> {
    let document = instance.fields.get("document").cloned().ok_or_else(|| {
        StoreError::Backend("Eventlog planning row has no document field".to_owned())
    })?;
    let document: PlanningDocument = serde_json::from_value(document).map_err(|error| {
        StoreError::Backend(format!("invalid Eventlog planning document: {error}"))
    })?;
    // The typed AEP metadata already persists its semantic revision. The ER wrapper has its
    // own revision, including evidence-only writes, and must not substitute that for AEP's.
    let revision = if instance.entity == aep_backend_entity::STORED_AS {
        document
            .fields
            .get("$aep")
            .and_then(|value| value.get("metadata"))
            .and_then(|value| value.get("revision"))
            .and_then(Value::as_u64)
            .filter(|revision| *revision > 0)
            .ok_or_else(|| StoreError::Backend("planning entity revision is absent".to_owned()))?
    } else {
        instance.revision
    };
    Ok(EntityInstance {
        entity: instance.entity,
        version: instance.version,
        id: instance.id,
        lifecycle_state: document.lifecycle_state,
        revision,
        fields: document.fields,
    })
}

fn decision_document(commit: &RecordedCommit) -> Option<PlanningDocument> {
    let document = match &commit.envelope.record.command {
        entity_core::DecisionCommand::Create { fields, .. } => fields.get("document")?.clone(),
        entity_core::DecisionCommand::Execute { arguments, .. } => {
            arguments.get("document")?.clone()
        }
        entity_core::DecisionCommand::LegacyImport => return None,
    };
    serde_json::from_value(document).ok()
}

fn context_from_recording(recording: &Recording) -> EventlogOperationContext {
    context_from_parts(
        recording.actor.as_ref(),
        &recording.record_id,
        recording.correlation.as_ref(),
        recording.causation.as_ref(),
        &recording.recorded_at,
    )
}

fn context_from_parts(
    actor: Option<&String>,
    record_id: &str,
    correlation: Option<&String>,
    causation: Option<&String>,
    recorded_at: &str,
) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: actor.cloned().unwrap_or_else(|| "aep".to_owned()),
        actor: actor.cloned().unwrap_or_else(|| "aep".to_owned()),
        request_id: correlation.cloned().unwrap_or_else(|| record_id.to_owned()),
        trace_id: correlation.cloned().unwrap_or_else(|| record_id.to_owned()),
        causation_id: causation.cloned(),
        causation_depth: 0,
        occurred_at: OffsetDateTime::parse(recorded_at, &Rfc3339)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    }
}

fn registry() -> Result<Registry, String> {
    let mut registry = Registry::new();
    for entity in [
        aep_backend_entity::STORED_AS,
        aep_backend_entity::RELATIONS_AS,
        aep_backend_entity::AUDIT_AS,
        aep_backend_entity::APPLIED_AS,
        INVOCATION_AS,
        PROJECTION_METADATA_AS,
    ] {
        let definition: EntityDefinition = serde_json::from_value(json!({
            "entity": entity,
            "version": 1,
            "schema": {
                "fields": {
                    "document": { "type": "json", "required": true }
                }
            },
            "lifecycle": { "initial": "recorded", "states": ["recorded"] },
            "operations": {
                "replace": {
                    "arguments": {
                        "fields": {
                            "document": { "type": "json", "required": true }
                        }
                    },
                    "transitions": [{ "from": "recorded", "to": "recorded" }],
                    "set": { "document": "$args.document" }
                }
            }
        }))
        .map_err(|error| format!("building {entity} definition: {error}"))?;
        registry
            .register(definition)
            .map_err(|error| format!("registering {entity} definition: {error}"))?;
    }
    Ok(registry)
}

#[allow(clippy::needless_pass_by_value)] // `map_err` supplies this owned provider error directly.
fn async_error(error: entity_store::asynchronous::AsyncStoreError) -> StoreError {
    StoreError::Backend(error.to_string())
}

#[allow(clippy::needless_pass_by_value)] // `map_err` supplies this owned provider error directly.
fn read_error(error: SyncReadError) -> StoreError {
    StoreError::Backend(format!("recorded Eventlog read failed: {error:?}"))
}

#[allow(clippy::needless_pass_by_value)] // `map_err` supplies this owned provider error directly.
fn execution_error(error: SyncExecutionError) -> StoreError {
    StoreError::Backend(format!("recorded Eventlog write failed: {error:?}"))
}
