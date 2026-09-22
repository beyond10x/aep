//! What one AEP call costs the Eventlog backend, counted.
//!
//! The migration's import used to ask the provider once per subject, and each ask captured the
//! whole destination twice. Nothing in a receipt, a digest or a wall clock says how many times
//! the destination was captured, so nothing stopped that growing with the store: a count is the
//! only observation that distinguishes "captured once for the batch" from "captured once per
//! subject". This is the write-path twin of the read-path seam in `retained_snapshot_tests`.
//!
//! [`CountingBackend`] wraps any [`EventlogBackend`] and forwards every call unchanged, adding
//! one counter increment. It is a seam, not a policy: it answers exactly what the wrapped backend
//! answers.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use entity_core::EntityInstance;
use entity_eventlog::sync::{SyncExecutionError, SyncReadError};
use entity_eventlog::EventlogBackend;
use entity_eventlog::EventlogOperationContext;
use entity_executor::BatchAction;
use entity_store::asynchronous::{
    AppendOutcome, BatchKey, CompleteStoreSnapshot, StoredBatch, Subject, SubjectHistory,
};
use entity_store::RecordedObservation;

use crate::RecordedPlanningProvider;
use eventlog_core::{
    AppendGroup, AppendGroupResult, AppendResult, AtomicEventStore, BoxFuture, CaptureError,
    CaptureLimits, CatchUpProgress, Claim, ClaimedCommand, CommandMeta, ConsistentTenantCapture,
    EventLogError, EventStore, Expected, FeedPage, Guard, InlineProjectionAdmin,
    InlineRebuildResult, NewEvent, ProjectionSpec, Projector, RecordedEvent, Snapshot, StreamId,
    StreamSlice, TenantCapture, TenantId,
};
use serde_json::Value;

/// What a run asked the backend for. Every field is a call the cost of a migration is made of.
#[derive(Debug, Default)]
pub struct BackendCalls {
    captures: AtomicUsize,
    groups: AtomicUsize,
    appends: AtomicUsize,
    blob_writes: AtomicUsize,
    blob_reads: AtomicUsize,
}

/// One reading of [`BackendCalls`], comparable and printable.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BackendCallCounts {
    /// Full captures of the destination authority — the cost that used to grow with the store.
    pub captures: usize,
    /// Atomic append groups, guarded or not, with or without bound blobs.
    pub groups: usize,
    /// Single-stream appends outside a group.
    pub appends: usize,
    /// Blobs written on their own path, outside a group's barrier.
    pub blob_writes: usize,
    /// Blobs read back.
    pub blob_reads: usize,
}

impl BackendCalls {
    /// Reads every counter at once.
    #[must_use]
    pub fn read(&self) -> BackendCallCounts {
        BackendCallCounts {
            captures: self.captures.load(Ordering::Relaxed),
            groups: self.groups.load(Ordering::Relaxed),
            appends: self.appends.load(Ordering::Relaxed),
            blob_writes: self.blob_writes.load(Ordering::Relaxed),
            blob_reads: self.blob_reads.load(Ordering::Relaxed),
        }
    }
}

/// An [`EventlogBackend`] that counts what it is asked for and otherwise changes nothing.
pub struct CountingBackend {
    inner: Arc<dyn EventlogBackend>,
    calls: Arc<BackendCalls>,
}

impl CountingBackend {
    /// Wraps `inner`, recording every call into `calls`.
    #[must_use]
    pub fn new(inner: Arc<dyn EventlogBackend>, calls: Arc<BackendCalls>) -> Self {
        Self { inner, calls }
    }
}

impl std::fmt::Debug for CountingBackend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CountingBackend")
            .field("calls", &self.calls.read())
            .finish_non_exhaustive()
    }
}

impl ConsistentTenantCapture for CountingBackend {
    fn capture_tenant<'a>(
        &'a self,
        tenant: &'a TenantId,
        projections: &'a [ProjectionSpec],
        limits: CaptureLimits,
    ) -> BoxFuture<'a, Result<TenantCapture, CaptureError>> {
        self.calls.captures.fetch_add(1, Ordering::Relaxed);
        self.inner.capture_tenant(tenant, projections, limits)
    }
}

impl AtomicEventStore for CountingBackend {
    fn append_group_guarded<'a>(
        &'a self,
        group: &'a AppendGroup,
        admission: Arc<dyn Guard>,
    ) -> BoxFuture<'a, Result<AppendGroupResult, EventLogError>> {
        self.calls.groups.fetch_add(1, Ordering::Relaxed);
        self.inner.append_group_guarded(group, admission)
    }

    fn append_group_guarded_with_blobs<'a>(
        &'a self,
        group: &'a AppendGroup,
        admission: Arc<dyn Guard>,
        blobs: &'a [(String, Vec<u8>)],
    ) -> BoxFuture<'a, Result<AppendGroupResult, EventLogError>> {
        self.calls.groups.fetch_add(1, Ordering::Relaxed);
        self.inner
            .append_group_guarded_with_blobs(group, admission, blobs)
    }
}

impl InlineProjectionAdmin for CountingBackend {
    fn attach_inline_existing(
        &self,
        projector: Arc<dyn Projector>,
    ) -> BoxFuture<'_, Result<(), EventLogError>> {
        self.inner.attach_inline_existing(projector)
    }

    fn rebuild_inline_projection<'a>(
        &'a self,
        projector_name: &'a str,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<InlineRebuildResult, EventLogError>> {
        self.inner.rebuild_inline_projection(projector_name, tenant)
    }
}

impl EventStore for CountingBackend {
    fn append<'a>(
        &'a self,
        stream: &'a StreamId,
        expected: Expected,
        events: &'a [NewEvent],
        meta: &'a CommandMeta,
    ) -> BoxFuture<'a, Result<AppendResult, EventLogError>> {
        self.calls.appends.fetch_add(1, Ordering::Relaxed);
        self.inner.append(stream, expected, events, meta)
    }

    fn append_guarded<'a>(
        &'a self,
        stream: &'a StreamId,
        expected: Expected,
        events: &'a [NewEvent],
        meta: &'a CommandMeta,
        guard: Arc<dyn Guard>,
    ) -> BoxFuture<'a, Result<AppendResult, EventLogError>> {
        self.calls.appends.fetch_add(1, Ordering::Relaxed);
        self.inner
            .append_guarded(stream, expected, events, meta, guard)
    }

    fn put_blob<'a>(
        &'a self,
        tenant: &'a TenantId,
        digest: &'a str,
        bytes: &'a [u8],
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.calls.blob_writes.fetch_add(1, Ordering::Relaxed);
        self.inner.put_blob(tenant, digest, bytes)
    }

    fn get_blob<'a>(
        &'a self,
        tenant: &'a TenantId,
        digest: &'a str,
    ) -> BoxFuture<'a, Result<Option<Vec<u8>>, EventLogError>> {
        self.calls.blob_reads.fetch_add(1, Ordering::Relaxed);
        self.inner.get_blob(tenant, digest)
    }

    fn delete_blob<'a>(
        &'a self,
        tenant: &'a TenantId,
        digest: &'a str,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.inner.delete_blob(tenant, digest)
    }

    fn recorded_claim<'a>(
        &'a self,
        tenant: &'a TenantId,
        claim: &'a Claim,
    ) -> BoxFuture<'a, Result<Option<ClaimedCommand>, EventLogError>> {
        self.inner.recorded_claim(tenant, claim)
    }

    fn recorded_command<'a>(
        &'a self,
        stream: &'a StreamId,
        idempotency_key: &'a str,
        request_hash: &'a str,
    ) -> BoxFuture<'a, Result<Option<AppendResult>, EventLogError>> {
        self.inner
            .recorded_command(stream, idempotency_key, request_hash)
    }

    fn read_stream<'a>(
        &'a self,
        stream: &'a StreamId,
        after_version: u64,
        limit: usize,
    ) -> BoxFuture<'a, Result<StreamSlice, EventLogError>> {
        self.inner.read_stream(stream, after_version, limit)
    }

    fn stream_version<'a>(
        &'a self,
        stream: &'a StreamId,
    ) -> BoxFuture<'a, Result<Option<u64>, EventLogError>> {
        self.inner.stream_version(stream)
    }

    fn read_feed<'a>(
        &'a self,
        tenant: &'a TenantId,
        after_position: u64,
        limit: usize,
    ) -> BoxFuture<'a, Result<FeedPage, EventLogError>> {
        self.inner.read_feed(tenant, after_position, limit)
    }

    fn redact<'a>(
        &'a self,
        stream: &'a StreamId,
        version: u64,
        reason: &'a str,
    ) -> BoxFuture<'a, Result<RecordedEvent, EventLogError>> {
        self.inner.redact(stream, version, reason)
    }

    fn save_snapshot<'a>(
        &'a self,
        stream: &'a StreamId,
        snapshot: &'a Snapshot,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.inner.save_snapshot(stream, snapshot)
    }

    fn load_snapshot<'a>(
        &'a self,
        stream: &'a StreamId,
    ) -> BoxFuture<'a, Result<Option<Snapshot>, EventLogError>> {
        self.inner.load_snapshot(stream)
    }

    fn forget_tenant<'a>(
        &'a self,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<(), EventLogError>> {
        self.inner.forget_tenant(tenant)
    }

    fn create_projections(
        &self,
        projector: Arc<dyn Projector>,
    ) -> BoxFuture<'_, Result<(), EventLogError>> {
        self.inner.create_projections(projector)
    }

    fn register_inline(
        &self,
        projector: Arc<dyn Projector>,
    ) -> BoxFuture<'_, Result<(), EventLogError>> {
        self.inner.register_inline(projector)
    }

    fn is_inline<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        self.inner.is_inline(name)
    }

    fn run_catch_up<'a>(
        &'a self,
        projector: Arc<dyn Projector>,
        tenant: &'a TenantId,
        batch: usize,
    ) -> BoxFuture<'a, Result<CatchUpProgress, EventLogError>> {
        self.inner.run_catch_up(projector, tenant, batch)
    }

    fn rebuild_projection<'a>(
        &'a self,
        projector: Arc<dyn Projector>,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<u64, EventLogError>> {
        self.inner.rebuild_projection(projector, tenant)
    }

    fn projection_get<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Value>, EventLogError>> {
        self.inner.projection_get(projection, tenant, key)
    }

    fn projection_find<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        field: &'a str,
        value: &'a str,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Value>, EventLogError>> {
        self.inner
            .projection_find(projection, tenant, field, value, limit)
    }

    fn projection_list<'a>(
        &'a self,
        projection: &'a ProjectionSpec,
        tenant: &'a TenantId,
        after_key: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<(String, Value)>, EventLogError>> {
        self.inner
            .projection_list(projection, tenant, after_key, limit)
    }

    fn stream_identity<'a>(
        &'a self,
        tenant: &'a TenantId,
    ) -> BoxFuture<'a, Result<String, EventLogError>> {
        self.inner.stream_identity(tenant)
    }
}

/// Every provider call one planning handle made, by kind.
///
/// [`CountingBackend`] counts the provider calls this crate makes through
/// [`crate::with_async_store_through`], which is the path `complete_file_snapshot` and the import
/// take. It is blind to the other one: [`crate::open`] and [`crate::open_with_snapshot`] reach the
/// authority through `RecordedEventlogBridge::start`, which builds its own provider inside Entity
/// Runtime, so a backend decorator never sees it — and those are exactly the opens whose captures
/// the held-capture path exists to remove. This counts that path, at the seam
/// [`crate::RecordedPlanningProvider`] already defines.
#[derive(Debug, Default)]
pub struct ProviderCalls {
    snapshots: AtomicUsize,
    loads: AtomicUsize,
    histories: AtomicUsize,
    lookups: AtomicUsize,
    batches: AtomicUsize,
    observations: AtomicUsize,
}

/// One reading of [`ProviderCalls`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCallCounts {
    /// Complete captures of the authority. The number this unit is about.
    pub snapshots: usize,
    /// Single-subject terminal reads.
    pub loads: usize,
    /// Single-subject history reads.
    pub histories: usize,
    /// Named batch lookups.
    pub lookups: usize,
    /// Recorded atomic batches.
    pub batches: usize,
    /// Recorded observations.
    pub observations: usize,
}

impl ProviderCalls {
    /// Reads every counter at once.
    #[must_use]
    pub fn read(&self) -> ProviderCallCounts {
        ProviderCallCounts {
            snapshots: self.snapshots.load(Ordering::Relaxed),
            loads: self.loads.load(Ordering::Relaxed),
            histories: self.histories.load(Ordering::Relaxed),
            lookups: self.lookups.load(Ordering::Relaxed),
            batches: self.batches.load(Ordering::Relaxed),
            observations: self.observations.load(Ordering::Relaxed),
        }
    }

    /// Every read, which is what a capture-shaped cost is counted in.
    #[must_use]
    pub fn reads(&self) -> usize {
        let counts = self.read();
        counts.snapshots + counts.loads + counts.histories + counts.lookups
    }
}

/// A [`RecordedPlanningProvider`] that counts what it is asked for and otherwise changes nothing.
pub struct CountingPlanningProvider<P> {
    inner: P,
    calls: Arc<ProviderCalls>,
}

impl<P> CountingPlanningProvider<P> {
    /// Wraps `inner`, recording every call into `calls`.
    pub const fn new(inner: P, calls: Arc<ProviderCalls>) -> Self {
        Self { inner, calls }
    }
}

impl<P> std::fmt::Debug for CountingPlanningProvider<P> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CountingPlanningProvider")
            .field("calls", &self.calls.read())
            .finish_non_exhaustive()
    }
}

impl<P: RecordedPlanningProvider> RecordedPlanningProvider for CountingPlanningProvider<P> {
    fn complete_snapshot(&self, scope: &str) -> Result<CompleteStoreSnapshot, SyncReadError> {
        self.calls.snapshots.fetch_add(1, Ordering::Relaxed);
        self.inner.complete_snapshot(scope)
    }

    fn load(&self, subject: &Subject) -> Result<Option<EntityInstance>, SyncReadError> {
        self.calls.loads.fetch_add(1, Ordering::Relaxed);
        self.inner.load(subject)
    }

    fn history(&self, subject: &Subject) -> Result<SubjectHistory, SyncReadError> {
        self.calls.histories.fetch_add(1, Ordering::Relaxed);
        self.inner.history(subject)
    }

    fn lookup_batch(&self, key: &BatchKey) -> Result<Option<StoredBatch>, SyncReadError> {
        self.calls.lookups.fetch_add(1, Ordering::Relaxed);
        self.inner.lookup_batch(key)
    }

    fn batch(
        &self,
        context: EventlogOperationContext,
        key: BatchKey,
        actions: Vec<BatchAction>,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        self.calls.batches.fetch_add(1, Ordering::Relaxed);
        self.inner.batch(context, key, actions)
    }

    fn observe(
        &self,
        context: EventlogOperationContext,
        observation: RecordedObservation,
    ) -> Result<AppendOutcome, SyncExecutionError> {
        self.calls.observations.fetch_add(1, Ordering::Relaxed);
        self.inner.observe(context, observation)
    }
}

/// What this crate asked the authority for on **this thread**, by entry point.
///
/// Two of the migration's capture sites are not store reads and no decorator can see them: the
/// projection's `complete` step calls [`crate::complete_file_snapshot`], and `commit`'s
/// `recovering` step calls [`crate::read_file_control`]. Each is a full capture of the authority
/// and each is skipped when a held capture answers instead — 12.17 s of one 27.3 s saving sits
/// behind the first of them — so what guards them has to observe the capture at its own site.
///
/// The counters are charged where those entry points open the authority, and they are
/// thread-local so that cases running in parallel in one test binary do not see each other's.
pub mod sites {
    use std::cell::Cell;

    thread_local! {
        static OPENS: Cell<AuthorityOpens> = const { Cell::new(AuthorityOpens::NONE) };
    }

    /// How many times this crate opened the authority for a fresh read, by entry point.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct AuthorityOpens {
        /// Whole-authority captures: `complete_file_snapshot`, `import_file_anchors`, and the
        /// other `with_async_store` entry points.
        pub captures: usize,
        /// Control-row bridges: `read_file_control` and `write_file_control`.
        pub control_bridges: usize,
    }

    impl AuthorityOpens {
        const NONE: Self = Self {
            captures: 0,
            control_bridges: 0,
        };
    }

    pub(crate) fn charge_capture() {
        OPENS.with(|opens| {
            let mut value = opens.get();
            value.captures += 1;
            opens.set(value);
        });
    }

    pub(crate) fn charge_control_bridge() {
        OPENS.with(|opens| {
            let mut value = opens.get();
            value.control_bridges += 1;
            opens.set(value);
        });
    }

    /// Zeroes this thread's counters and returns what they held.
    pub fn take_authority_opens() -> AuthorityOpens {
        OPENS.with(|opens| opens.replace(AuthorityOpens::NONE))
    }

    /// This thread's counters, left in place.
    #[must_use]
    pub fn authority_opens() -> AuthorityOpens {
        OPENS.with(Cell::get)
    }
}
