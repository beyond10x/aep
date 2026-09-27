//! The one-time migration of an `aep.project/3` tree store into an `aep.project/4` one.
//!
//! Nothing is decided again by AEP and nothing is invented. The migration replays the source's own
//! recorded history into a new tree store with the source's store and stream identities:
//!
//! 1. every imported anchor, in one import, as the source was given them;
//! 2. every recorded batch, in the order the source committed it, with the same batch key, the
//!    same caller-supplied recording, the same command and the same expected revision.
//!
//! Every write goes through [`crate::stored_action`], so the target records each large value once
//! in the content-blob directory and names it by digest everywhere else. Entity Runtime decides
//! every replayed command again over the same definitions, which is what makes the target's
//! history a recorded history rather than a copy of one.
//!
//! [`verify_equivalent`] is the proof. With every reference resolved, the target must hold the
//! same subjects, the same anchors, the same terminal instances and — record by record — the same
//! entries, receipts, positions and expectations as the source. What the planning contract reads
//! (artifacts, statuses, relations, evidence, reviews, history and audit) is read from exactly
//! those values, so equal values read equal. Only the stored bytes differ, which is the point.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aep_domain::artifact::LifecycleRegistry;
use entity_core::DecisionCommand;
use entity_eventlog::sync::{
    BridgeConfig, CallWait, EventlogRecordedStoreOwner, RecordedEventlogBridge,
};
use entity_eventlog::{
    AsyncImportedAnchorWriter, Authority, ErRecordedProjector, EventlogRecordedStore,
};
use entity_executor::{BatchAction, CreateRequest, ExecuteRequest};
use entity_store::asynchronous::{
    CompleteStoreSnapshot, HistoryOrigin, RecordedEntry, StoredRecord, Subject, SubjectHistory,
};
use entity_store::{Expect, Recording};
use eventlog_core::InlineProjectionAdmin;
use serde_json::Value;

use crate::content::ContentStore;
use crate::typed::{typed_registry, TypedKinds};
use crate::{
    context_from_recording, stored_action, stored_anchor, AuthoritySession, CAPTURE_LIMITS,
};

/// What a content migration wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentMigrationReport {
    /// Subjects the source held.
    pub subjects: usize,
    /// Imported anchors carried over.
    pub anchors: usize,
    /// Recorded batches replayed.
    pub batches: usize,
    /// Decisions replayed.
    pub decisions: usize,
    /// Observations replayed.
    pub observations: usize,
    /// The stream identity both stores carry.
    pub stream_identity: String,
}

/// Whether two captures hold the same history, and where they do not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Equivalence {
    /// Subjects compared.
    pub subjects: usize,
    /// Records compared.
    pub records: usize,
    /// Every difference, naming the subject and what differs. Empty is equivalence.
    pub differences: Vec<String>,
}

/// Replays the `aep.project/3` tree authority `source` into a new `aep.project/4` authority at
/// `target`, keeping its large values in `blobs`.
///
/// `target` must not exist. The source is only read.
///
/// # Errors
/// The source holds a fork, a merge or a record this replay cannot restate; the target exists;
/// or Entity Runtime refuses a replayed write.
#[allow(clippy::too_many_lines)] // The migration reads, replays and reports in one visible order.
pub fn migrate_to_content(
    source: &AuthoritySession,
    target: &Path,
    blobs: &Path,
    lifecycles: &LifecycleRegistry,
) -> Result<ContentMigrationReport, String> {
    if source.content().is_some() {
        return Err("the source store already keeps content blobs".to_owned());
    }
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }
    let snapshot = source.complete_snapshot()?;
    refuse_branches(&snapshot)?;
    let authority = source.authority().clone();
    let mut report = ContentMigrationReport {
        subjects: snapshot.histories.len(),
        stream_identity: authority.stream_identity.clone(),
        ..ContentMigrationReport::default()
    };

    // The target keeps the source's store and stream identities, so every recorded coordinate
    // that names the authority — receipts inside invocation documents among them — still names it.
    seed_identity(source.path(), target, &authority.tenant)?;
    let identity = crate::typed::prepare_tree(target, &authority.tenant)?;
    if identity != authority.stream_identity {
        return Err(format!(
            "the target's stream identity is {identity}, not the source's {}",
            authority.stream_identity
        ));
    }
    crate::typed::provision_tree(
        target,
        authority.logical_scope.clone(),
        authority.tenant.clone(),
        identity,
        crate::typed::provisioning_context("aep-plan-store-migrate-content"),
    )?;
    let content = ContentStore::at(blobs.to_owned());

    let anchors = snapshot
        .histories
        .iter()
        .filter_map(|held| match &held.history.origin {
            HistoryOrigin::Imported(anchor) => Some((held.history.subject.clone(), anchor.clone())),
            HistoryOrigin::Genesis => None,
        })
        .map(|(subject, anchor)| {
            Ok(SubjectHistory {
                subject,
                origin: HistoryOrigin::Imported(stored_anchor(&content, anchor)?),
                records: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    report.anchors = anchors.len();
    if !anchors.is_empty() {
        import_anchors(target, &authority, anchors)?;
    }

    let kinds = TypedKinds::of(lifecycles);
    let bridge = RecordedEventlogBridge::start(
        typed_registry(&kinds)?,
        EventlogRecordedStoreOwner::Tree {
            path: target.to_owned(),
            authority: authority.clone(),
            limits: CAPTURE_LIMITS,
        },
        BridgeConfig {
            queue_capacity: std::num::NonZeroU16::new(32).expect("nonzero queue"),
        },
    )
    .map_err(|error| format!("opening the target tree store: {error:?}"))?;

    for batch in batches(&snapshot)? {
        let first = batch.first().expect("a batch has a member");
        let key = first.1.receipt.batch_key.clone();
        let context = context_from_recording(&recording_of(&first.1.entry));
        let mut actions = Vec::with_capacity(batch.len());
        for (subject, record) in &batch {
            match &record.entry {
                RecordedEntry::Decision(_) => report.decisions += 1,
                RecordedEntry::Observation(_) => report.observations += 1,
            }
            actions.push(stored_action(&content, action_of(subject, record)?)?);
        }
        bridge
            .operation(context)
            .batch(key.clone(), actions, CallWait::Forever)
            .map_err(|error| format!("replaying batch {key:?}: {error:?}"))?;
        report.batches += 1;
    }
    Ok(report)
}

/// Compares two complete captures with every reference already resolved.
#[must_use]
pub fn verify_equivalent(
    source: &CompleteStoreSnapshot,
    target: &CompleteStoreSnapshot,
) -> Equivalence {
    let mut equivalence = Equivalence::default();
    fn index(
        snapshot: &CompleteStoreSnapshot,
    ) -> BTreeMap<Subject, &entity_store::asynchronous::SubjectSnapshot> {
        snapshot
            .histories
            .iter()
            .map(|held| (held.history.subject.clone(), held))
            .collect()
    }
    let (before, after) = (index(source), index(target));
    let subjects: BTreeSet<&Subject> = before.keys().chain(after.keys()).collect();
    for subject in subjects {
        let name = format!("{}:{}", subject.entity, subject.id);
        let (Some(old), Some(new)) = (before.get(subject), after.get(subject)) else {
            equivalence.differences.push(format!(
                "{name}: held only by the {}",
                if before.contains_key(subject) {
                    "source"
                } else {
                    "target"
                }
            ));
            continue;
        };
        equivalence.subjects += 1;
        if old.history.origin != new.history.origin {
            equivalence
                .differences
                .push(format!("{name}: the anchor differs"));
        }
        if old.terminal != new.terminal {
            equivalence
                .differences
                .push(format!("{name}: the terminal instance differs"));
        }
        if old.history.records.len() != new.history.records.len() {
            equivalence.differences.push(format!(
                "{name}: {} records in the source, {} in the target",
                old.history.records.len(),
                new.history.records.len()
            ));
            continue;
        }
        for (left, right) in old.history.records.iter().zip(&new.history.records) {
            equivalence.records += 1;
            let record = &left.receipt.record_id;
            for (what, differs) in [
                ("entry", left.entry != right.entry),
                ("receipt", left.receipt != right.receipt),
                ("position", left.position != right.position),
                ("expectation", left.expect != right.expect),
                ("record bytes", left.record_bytes != right.record_bytes),
                ("request bytes", left.request_bytes != right.request_bytes),
            ] {
                if differs {
                    equivalence
                        .differences
                        .push(format!("{name}: record {record}: the {what} differs"));
                }
            }
        }
    }
    equivalence
}

/// A fork or a merge has no single order to replay in. The source must be resolved first.
fn refuse_branches(snapshot: &CompleteStoreSnapshot) -> Result<(), String> {
    for held in &snapshot.histories {
        let mut parents = BTreeSet::new();
        for record in &held.history.records {
            let Some(lineage) = &record.lineage else {
                continue;
            };
            if lineage.parents.len() > 1 {
                return Err(format!(
                    "{}:{} holds a merge decision ({}); a merged history is not replayed",
                    held.history.subject.entity, held.history.subject.id, record.receipt.record_id
                ));
            }
            for parent in &lineage.parents {
                if !parents.insert(parent.clone()) {
                    return Err(format!(
                        "{}:{} has forked; run `aep plan artifact resolve` on it first",
                        held.history.subject.entity, held.history.subject.id
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Every record the source holds, grouped into its batches, in the order the source committed them.
fn batches(snapshot: &CompleteStoreSnapshot) -> Result<Vec<Vec<(Subject, StoredRecord)>>, String> {
    let mut records: Vec<(Subject, StoredRecord)> = snapshot
        .histories
        .iter()
        .flat_map(|held| {
            held.history
                .records
                .iter()
                .map(|record| (held.history.subject.clone(), record.clone()))
        })
        .collect();
    records.sort_by_key(|(_, record)| record.position.store);
    let mut batches: Vec<Vec<(Subject, StoredRecord)>> = Vec::new();
    for (subject, record) in records {
        let continues = batches.last().is_some_and(|batch| {
            let last = &batch.last().expect("a batch has a member").1.receipt;
            last.batch_key == record.receipt.batch_key
                && last.member_index + 1 == record.receipt.member_index
        });
        if continues {
            batches.last_mut().expect("checked").push((subject, record));
        } else {
            if record.receipt.member_index != 0 {
                return Err(format!(
                    "record {} is member {} of a batch whose first member is not before it",
                    record.receipt.record_id, record.receipt.member_index
                ));
            }
            batches.push(vec![(subject, record)]);
        }
    }
    Ok(batches)
}

/// The recording a record was sealed with.
fn recording_of(entry: &RecordedEntry) -> Recording {
    let (record_id, recorded_at, correlation, causation, actor) = match entry {
        RecordedEntry::Decision(commit) => {
            let envelope = &commit.envelope;
            (
                &envelope.record_id,
                &envelope.recorded_at,
                &envelope.correlation,
                &envelope.causation,
                &envelope.actor,
            )
        }
        RecordedEntry::Observation(observation) => {
            let envelope = &observation.envelope;
            (
                &envelope.record_id,
                &envelope.recorded_at,
                &envelope.correlation,
                &envelope.causation,
                &envelope.actor,
            )
        }
    };
    Recording {
        record_id: record_id.clone(),
        recorded_at: recorded_at.clone(),
        correlation: correlation.clone(),
        causation: causation.clone(),
        actor: actor.clone(),
    }
}

/// The command a record was decided on, restated as the action that decides it again.
fn action_of(subject: &Subject, record: &StoredRecord) -> Result<BatchAction, String> {
    let recording = recording_of(&record.entry);
    let commit = match &record.entry {
        RecordedEntry::Observation(observation) => {
            return Ok(BatchAction::Observe(observation.clone()))
        }
        RecordedEntry::Decision(commit) => commit,
    };
    match (&commit.envelope.record.command, record.expect) {
        (DecisionCommand::Create { fields, arguments }, Expect::Absent) if arguments.is_empty() => {
            Ok(BatchAction::Create(CreateRequest {
                subject: subject.clone(),
                definition_version: u32::try_from(commit.instance.version).map_err(|_| {
                    format!("{}: definition version out of range", recording.record_id)
                })?,
                fields: Value::Object(fields.clone()),
                recording,
            }))
        }
        (
            DecisionCommand::Execute {
                operation,
                arguments,
                fulfillments,
            },
            Expect::Revision(expected_revision),
        ) => Ok(BatchAction::Execute(ExecuteRequest {
            subject: subject.clone(),
            expected_revision,
            operation: operation.clone(),
            arguments: Value::Object(arguments.clone()),
            fulfillments: fulfillments.clone(),
            recording,
        })),
        _ => Err(format!(
            "record {} is not a creation or an operation this migration can restate",
            recording.record_id
        )),
    }
}

/// Copies the store and tenant identity files, so the target is provisioned under the source's
/// identities rather than newly minted ones.
fn seed_identity(source: &Path, target: &Path, tenant: &str) -> Result<(), String> {
    let files: [PathBuf; 2] = [
        PathBuf::from("store.json"),
        Path::new("tenants").join(tenant).join("identity.json"),
    ];
    for file in files {
        let from = source.join(&file);
        let to = target.join(&file);
        std::fs::create_dir_all(to.parent().expect("a file has a directory"))
            .map_err(|error| format!("creating {}: {error}", to.display()))?;
        std::fs::copy(&from, &to)
            .map_err(|error| format!("copying {} to {}: {error}", from.display(), to.display()))?;
    }
    Ok(())
}

/// Imports every anchor in one call, as the export that wrote them did.
fn import_anchors(
    target: &Path,
    authority: &Authority,
    anchors: Vec<SubjectHistory>,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| format!("constructing the migration runtime: {error}"))?;
    runtime.block_on(async {
        let concrete = Arc::new(
            eventlog_tree::TreeEventStore::open(target)
                .await
                .map_err(|error| format!("opening the target tree store: {error}"))?,
        );
        concrete
            .attach_inline_existing(Arc::new(ErRecordedProjector::new()))
            .await
            .map_err(|error| format!("attaching the recorded projection: {error}"))?;
        let backend: Arc<dyn entity_eventlog::EventlogBackend> = concrete;
        let store = EventlogRecordedStore::open(backend, authority.clone(), CAPTURE_LIMITS)
            .await
            .map_err(|error| format!("opening the recorded target store: {error:?}"))?;
        store
            .operation(crate::typed::provisioning_context(
                "aep-plan-store-migrate-content",
            ))
            .import_anchors(anchors)
            .await
            .map_err(|error| format!("importing anchors into the target: {error:?}"))?;
        Ok(())
    })
}
