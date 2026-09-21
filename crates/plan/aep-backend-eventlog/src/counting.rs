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

use entity_eventlog::EventlogBackend;
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
