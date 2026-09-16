//! Durable eight-phase cutover with explicit writer-control and projection ports.

#![allow(missing_docs)]

use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

#[allow(clippy::wildcard_imports)]
// This coordinator implements the complete closed migration vocabulary.
use aep_contract::migration::*;
use entity_eventlog::{Authority, EventlogOperationContext};
use entity_store::{
    asynchronous::{
        BatchKey, CompleteStoreSnapshot, HistoryOrigin, RecordKind, StoreCoverage, SubjectHistory,
    },
    Expect,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use time::OffsetDateTime;

/// Exclusive operational facts supplied by the host. Implementations may not infer them from a
/// lock file, process scan, or store path.
pub trait WriterControl {
    type Guard;
    fn acquire(&self, intent: &MigrationIntentV2) -> Result<Self::Guard, WriterControlError>;
    fn recheck(
        &self,
        guard: &mut Self::Guard,
        source_snapshot: SourceSnapshotIdV1,
        selector_digest: DigestV1,
    ) -> Result<(), WriterControlError>;
    fn retire_source(&self, guard: &mut Self::Guard) -> Result<(), WriterControlError>;
}

/// Writer exclusion over the already selected Eventlog authority during a projection rebuild.
pub trait AuthorityWriterControl {
    type Guard;
    fn acquire_authority(
        &self,
        authority: &AuthorityCoordinateV1,
        requested: AuthoritySnapshotIdV1,
    ) -> Result<Self::Guard, WriterControlError>;
    fn recheck_authority(
        &self,
        guard: &mut Self::Guard,
        authority: &AuthorityCoordinateV1,
        requested: AuthoritySnapshotIdV1,
    ) -> Result<(), WriterControlError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriterControlError {
    #[error("writer exclusion is unavailable")]
    Unavailable,
    #[error("writer exclusion was lost")]
    Lost,
    #[error("legacy source retirement was not established")]
    RetirementUnproved,
}

/// Projection publication remains separate from Eventlog authority and returns exact owned-path
/// inventory evidence.
pub trait ProjectionPublisher {
    fn publish(
        &self,
        authority_snapshot: AuthoritySnapshotIdV1,
    ) -> Result<ProjectionPublication, ProjectionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionPublication {
    pub inventory_digest: ProjectionInventoryDigestV1,
    pub replaced_owned_paths: u64,
    pub preserved_foreign_paths: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProjectionError {
    #[error("projection conflicts with foreign content")]
    ForeignConflict,
    #[error("projection publication failed before replacement")]
    NotPublished,
    #[error("projection publication outcome is uncertain")]
    Uncertain,
}

pub struct ApplyInputs {
    pub intent: MigrationIntentV2,
    pub selector_path: PathBuf,
    pub phase_root: PathBuf,
    pub staging_path: PathBuf,
    pub destination_path: PathBuf,
    pub projection_path: PathBuf,
    /// Canonical JSON-plus-LF bytes of the semantically validated source observation. The caller
    /// owns typed validation; the coordinator preserves and compares the exact recovery value.
    pub recovery_capture_bytes: Vec<u8>,
    pub source_capture_digest: DigestV1,
    pub histories: Vec<SubjectHistory>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("durable migration evidence conflicts")]
    EvidenceConflict,
    #[error("the immutable migration request conflicts with retained evidence")]
    IntentConflict,
    #[error("the migration stage or destination existed before ownership was established")]
    ForeignStage,
    #[error("the owned destination paths have a conflicting physical state")]
    DestinationConflict,
    #[error("the provider authority identity differs from the durable binding")]
    AuthorityIdentityMismatch,
    #[error("the project selector differs from the durable binding")]
    SelectorChanged,
    #[error("the retained receipt or its evidence chain conflicts")]
    ReceiptConflict,
    #[error("migration IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("migration encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("migration canonical framing failed: {0}")]
    Framing(#[from] FrameError),
    #[error(transparent)]
    Writer(#[from] WriterControlError),
    #[error(transparent)]
    Projection(#[from] ProjectionError),
    #[error("Eventlog operation failed: {0}")]
    Eventlog(String),
    #[error("imported authority differs from the complete source")]
    VerificationMismatch,
}

/// Executes or resumes one immutable migration. Existing phase bytes are compared, never replaced.
pub fn apply<C, F>(
    control: &C,
    input: &ApplyInputs,
    finalize_selector: F,
) -> Result<MigrationReceiptV1, ApplyError>
where
    C: WriterControl,
    F: Fn(&AuthorityCoordinateV1) -> Result<Vec<u8>, ApplyError>,
{
    apply_with_hook(control, input, finalize_selector, |_| Ok(()))
}

/// Executes or resumes an immutable migration with a guard acquired before source capture.
///
/// The CLI uses this entry after it has acquired writer control, captured and mapped the source
/// under that same non-clone guard. This prevents the former gap in which capture happened before
/// the implementation even asked whether exclusion was available.
pub fn apply_held<C, F>(
    control: &C,
    guard: C::Guard,
    input: &ApplyInputs,
    finalize_selector: F,
) -> Result<MigrationReceiptV1, ApplyError>
where
    C: WriterControl,
    F: Fn(&AuthorityCoordinateV1) -> Result<Vec<u8>, ApplyError>,
{
    apply_held_with_hook(control, guard, input, finalize_selector, |_| Ok(()))
}

fn apply_with_hook<C, F, H>(
    control: &C,
    input: &ApplyInputs,
    finalize_selector: F,
    after_effect: H,
) -> Result<MigrationReceiptV1, ApplyError>
where
    C: WriterControl,
    F: Fn(&AuthorityCoordinateV1) -> Result<Vec<u8>, ApplyError>,
    H: FnMut(MigrationPhaseV1) -> Result<(), ApplyError>,
{
    let guard = control.acquire(&input.intent)?;
    apply_held_with_hook(control, guard, input, finalize_selector, after_effect)
}

#[allow(clippy::too_many_lines)] // The eight durable phases stay visibly ordered in one coordinator.
fn apply_held_with_hook<C, F, H>(
    control: &C,
    mut guard: C::Guard,
    input: &ApplyInputs,
    finalize_selector: F,
    mut after_effect: H,
) -> Result<MigrationReceiptV1, ApplyError>
where
    C: WriterControl,
    F: Fn(&AuthorityCoordinateV1) -> Result<Vec<u8>, ApplyError>,
    H: FnMut(MigrationPhaseV1) -> Result<(), ApplyError>,
{
    let expected_intent = intent_digest_v2(&input.intent)?;
    if input.intent.intent_digest != expected_intent {
        return Err(ApplyError::IntentConflict);
    }
    let mut marker = OwnershipMarkerV2 {
        format: OwnershipMarkerFormatV2,
        migration_id: input.intent.migration_id.clone(),
        intent_digest: input.intent.intent_digest,
        legacy_source: input.intent.source_coordinate.clone(),
        staging_path: input.intent.staging_path.clone(),
        destination_path: input.intent.destination_path.clone(),
        projection_path: input.intent.projection_path.clone(),
        destination_request: input.intent.destination_request.clone(),
        marker_digest: DigestV1::from_bytes([0; 32]),
    };
    marker.marker_digest =
        digest_omitting("aep.migration.ownership-marker/2", &marker, "marker_digest")?;
    let intent_path = input.phase_root.join("intent.json");
    let marker_path = input.phase_root.join("ownership.json");
    let recovery_path = input.phase_root.join("recovery/raw-capture.json");
    let receipt_path = input.phase_root.join("receipt.json");
    if receipt_path.exists() {
        if fs::read(&intent_path)? != compact_line(&input.intent)?
            || fs::read(&marker_path)? != compact_line(&marker)?
            || fs::read(&recovery_path)? != input.recovery_capture_bytes
        {
            return Err(ApplyError::ReceiptConflict);
        }
        let receipt =
            read_json::<MigrationReceiptV1>(&receipt_path)?.ok_or(ApplyError::ReceiptConflict)?;
        validate_completed_receipt(input, &receipt, &finalize_selector)
            .map_err(|_| ApplyError::ReceiptConflict)?;
        return Ok(receipt);
    }

    control.recheck(
        &mut guard,
        input.intent.source_snapshot,
        input.intent.selector_digest,
    )?;
    fs::create_dir_all(&input.phase_root)?;
    create_or_compare(&intent_path, &compact_line(&input.intent)?)?;
    create_or_compare(&marker_path, &compact_line(&marker)?)?;
    fs::create_dir_all(recovery_path.parent().ok_or(ApplyError::EvidenceConflict)?)?;
    create_or_compare(&recovery_path, &input.recovery_capture_bytes)?;

    let mut journal = PhaseJournal::open(
        input.phase_root.clone(),
        input.intent.migration_id.clone(),
        input.intent.intent_digest,
    )?;
    let first_prepared = !journal.is_at_least(MigrationPhaseV1::Prepared);
    if first_prepared && (input.staging_path.exists() || input.destination_path.exists()) {
        return Err(ApplyError::ForeignStage);
    }
    if first_prepared {
        after_effect(MigrationPhaseV1::Prepared)?;
    }
    journal.establish(
        MigrationPhaseV1::Prepared,
        vec![
            MigrationPhaseObservationV2::Selector(SelectorPhaseObservationV1 {
                digest: input.intent.selector_digest,
            }),
            MigrationPhaseObservationV2::Config(ConfigPhaseObservationV1 {
                digest: input.intent.config_digest,
            }),
            MigrationPhaseObservationV2::SourceCapture(SourceCapturePhaseObservationV1 {
                snapshot_id: input.intent.source_snapshot,
                capture_digest: input.source_capture_digest,
            }),
            MigrationPhaseObservationV2::WriterControl(WriterControlPhaseObservationV1 {
                source_digest: input.source_capture_digest,
                selector_digest: input.intent.selector_digest,
            }),
            MigrationPhaseObservationV2::DestinationPrecondition(DestinationPreconditionV2 {
                staging_path: input.intent.staging_path.clone(),
                destination_path: input.intent.destination_path.clone(),
                projection_path: input.intent.projection_path.clone(),
                staging_absent: true,
                destination_absent: true,
                projection_safe: true,
            }),
        ],
    )?;

    let (bound_authority, intended_selector_bytes, intended_selector_digest) = if journal
        .is_at_least(MigrationPhaseV1::DestinationProvisioned)
    {
        let record = journal.establish(MigrationPhaseV1::DestinationProvisioned, Vec::new())?;
        let binding = record
            .observations
            .iter()
            .find_map(|observation| match observation {
                MigrationPhaseObservationV2::Binding(value) => Some(value),
                _ => None,
            })
            .ok_or(ApplyError::ReceiptConflict)?;
        if binding.intent_digest != input.intent.intent_digest
            || selector_digest_v1(binding.intended_selector_bytes.as_bytes())
                != binding.intended_selector_digest
            || finalize_selector(&binding.authority)? != binding.intended_selector_bytes.as_bytes()
        {
            return Err(ApplyError::ReceiptConflict);
        }
        (
            binding.authority.clone(),
            binding.intended_selector_bytes.clone(),
            binding.intended_selector_digest,
        )
    } else {
        let (logical_scope, tenant, expected_identity) = match &input.intent.destination_request {
            DestinationRequestV2::ProviderAssigned {
                logical_scope,
                tenant,
            } => (logical_scope.clone(), tenant.clone(), None),
            DestinationRequestV2::ExactExisting { authority } => (
                authority.logical_scope.clone(),
                authority.tenant.clone(),
                Some(authority.stream_identity.clone()),
            ),
        };
        if matches!(
            input.intent.destination_request,
            DestinationRequestV2::ExactExisting { .. }
        ) && !input.staging_path.exists()
        {
            return Err(ApplyError::ForeignStage);
        }
        fs::create_dir_all(
            input
                .staging_path
                .parent()
                .ok_or(ApplyError::EvidenceConflict)?,
        )?;
        let actual_identity =
            aep_backend_eventlog::prepare_file(&input.staging_path, tenant.as_str())
                .map_err(ApplyError::Eventlog)?;
        after_effect(MigrationPhaseV1::DestinationProvisioned)?;
        if expected_identity
            .as_ref()
            .is_some_and(|expected| expected.as_str() != actual_identity)
        {
            return Err(ApplyError::AuthorityIdentityMismatch);
        }
        let bound = AuthorityCoordinateV1 {
            logical_scope,
            tenant,
            stream_identity: AuthorityValueV1::new(actual_identity)
                .map_err(|_| ApplyError::EvidenceConflict)?,
        };
        let physical = aep_backend_eventlog::provision_file(
            &input.staging_path,
            bound.logical_scope.as_str().to_owned(),
            bound.tenant.as_str().to_owned(),
            bound.stream_identity.as_str().to_owned(),
            operation_context(&input.intent.migration_id, "binding"),
        )
        .map_err(ApplyError::Eventlog)?;
        let selector = finalize_selector(&bound)?;
        let selector_digest = selector_digest_v1(&selector);
        after_effect(MigrationPhaseV1::DestinationProvisioned)?;
        journal.establish(
            MigrationPhaseV1::DestinationProvisioned,
            vec![MigrationPhaseObservationV2::Binding(
                BindingPhaseObservationV2 {
                    intent_digest: input.intent.intent_digest,
                    authority: bound.clone(),
                    physical: PhysicalRefV1 {
                        event_id: physical.event_id,
                        global_seq: physical.global_seq,
                        stream_id: physical.stream_id,
                        stream_version: physical.stream_version,
                    },
                    replayed: physical.replayed,
                    intended_selector_bytes: HexBytesV1::new(selector.clone()),
                    intended_selector_digest: selector_digest,
                },
            )],
        )?;
        (bound, HexBytesV1::new(selector), selector_digest)
    };
    let authority = adapter_authority(&bound_authority);
    let authority_path = match (input.staging_path.exists(), input.destination_path.exists()) {
        (true, false) => input.staging_path.as_path(),
        (false, true) if journal.is_at_least(MigrationPhaseV1::Verified) => {
            // A crash may happen after the atomic rename but before Published is recorded.
            // The durable phase then trails the physical effect, so recovery follows the one
            // bound authority at its destination and later establishes the missing phase.
            input.destination_path.as_path()
        }
        _ => return Err(ApplyError::DestinationConflict),
    };
    let actual_identity =
        aep_backend_eventlog::prepare_file(authority_path, bound_authority.tenant.as_str())
            .map_err(ApplyError::Eventlog)?;
    if actual_identity != bound_authority.stream_identity.as_str() {
        return Err(ApplyError::AuthorityIdentityMismatch);
    }
    let import_histories =
        crate::mapping::bind_authoritative_evidence(input.histories.clone(), &bound_authority)
            .map_err(|_| ApplyError::EvidenceConflict)?;
    if !journal.is_at_least(MigrationPhaseV1::Imported) {
        let replayed = aep_backend_eventlog::import_file_anchors(
            authority_path,
            authority.clone(),
            operation_context(&input.intent.migration_id, "import"),
            import_histories.clone(),
        )
        .map_err(ApplyError::Eventlog)?;
        let imported = import_histories
            .iter()
            .zip(replayed)
            .map(|(history, replayed)| {
                let boundary = match &history.origin {
                    entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                        serde_json::to_vec(anchor).expect("legacy anchor serialises")
                    }
                    entity_store::asynchronous::HistoryOrigin::Genesis => Vec::new(),
                };
                MigrationPhaseObservationV2::ImportedSubject(ImportedSubjectPhaseObservationV1 {
                    subject: SubjectCoordinateV1 {
                        entity: history.subject.entity.clone(),
                        id: history.subject.id.clone(),
                    },
                    boundary_digest: digest(&boundary),
                    replayed,
                })
            })
            .collect();
        after_effect(MigrationPhaseV1::Imported)?;
        journal.establish(MigrationPhaseV1::Imported, imported)?;
    }

    let snapshot = aep_backend_eventlog::complete_file_snapshot(authority_path, authority.clone())
        .map_err(ApplyError::Eventlog)?;
    let (authority_snapshot, authority_capture) = if journal.is_at_least(MigrationPhaseV1::Verified)
    {
        let record = journal.establish(MigrationPhaseV1::Verified, Vec::new())?;
        let saved = record
            .observations
            .iter()
            .find_map(|observation| match observation {
                MigrationPhaseObservationV2::CompleteSnapshot(value) => Some(value),
                _ => None,
            })
            .ok_or(ApplyError::EvidenceConflict)?;
        let (current_id, _) = authority_snapshot_identity(&bound_authority, &snapshot)?;
        if current_id != saved.snapshot_id {
            let prior = before_projection_watermark(&snapshot, saved.snapshot_id)?;
            let (prior_id, _) = authority_snapshot_identity(&bound_authority, &prior)?;
            if prior_id != saved.snapshot_id {
                return Err(ApplyError::VerificationMismatch);
            }
        }
        (saved.snapshot_id, saved.capture_digest)
    } else {
        if snapshot.histories.len() != import_histories.len() {
            return Err(ApplyError::VerificationMismatch);
        }
        let (snapshot_id, capture_digest) =
            authority_snapshot_identity(&bound_authority, &snapshot)?;
        after_effect(MigrationPhaseV1::Verified)?;
        journal.establish(
            MigrationPhaseV1::Verified,
            vec![MigrationPhaseObservationV2::CompleteSnapshot(
                CompleteSnapshotPhaseObservationV1 {
                    snapshot_id,
                    capture_digest,
                },
            )],
        )?;
        (snapshot_id, capture_digest)
    };

    control.recheck(
        &mut guard,
        input.intent.source_snapshot,
        input.intent.selector_digest,
    )?;
    if !journal.is_at_least(MigrationPhaseV1::Published) {
        match (input.staging_path.exists(), input.destination_path.exists()) {
            (true, false) => publish_directory(&input.staging_path, &input.destination_path)?,
            (false, true) => {
                aep_backend_eventlog::complete_file_snapshot(
                    &input.destination_path,
                    authority.clone(),
                )
                .map_err(ApplyError::Eventlog)?;
            }
            _ => return Err(ApplyError::DestinationConflict),
        }
        after_effect(MigrationPhaseV1::Published)?;
        journal.establish(
            MigrationPhaseV1::Published,
            vec![MigrationPhaseObservationV2::Rename(
                RenamePhaseObservationV1 {
                    from: input.intent.staging_path.clone(),
                    to: input.intent.destination_path.clone(),
                    source_absent: !input.staging_path.exists(),
                    destination_present: input.destination_path.is_dir(),
                    parents_synced: true,
                },
            )],
        )?;
    }

    if !journal.is_at_least(MigrationPhaseV1::Selected) {
        control.retire_source(&mut guard)?;
        replace_file(&input.selector_path, intended_selector_bytes.as_bytes())?;
        after_effect(MigrationPhaseV1::Selected)?;
        journal.establish(
            MigrationPhaseV1::Selected,
            vec![MigrationPhaseObservationV2::SelectorDurability(
                SelectorDurabilityPhaseObservationV1 {
                    old_digest: input.intent.selector_digest,
                    new_digest: intended_selector_digest,
                    file_synced: true,
                    parent_synced: true,
                },
            )],
        )?;
    } else if fs::read(&input.selector_path)? != intended_selector_bytes.as_bytes() {
        return Err(ApplyError::SelectorChanged);
    }

    let publication = if journal.is_at_least(MigrationPhaseV1::ProjectionPublished) {
        let record = journal.establish(MigrationPhaseV1::ProjectionPublished, Vec::new())?;
        let inventory_digest = record
            .observations
            .iter()
            .find_map(|observation| match observation {
                MigrationPhaseObservationV2::Projection(value)
                    if value.authority_snapshot == authority_snapshot =>
                {
                    Some(value.inventory_digest)
                }
                _ => None,
            })
            .ok_or(ApplyError::EvidenceConflict)?;
        ProjectionPublication {
            inventory_digest,
            replaced_owned_paths: 0,
            preserved_foreign_paths: 0,
        }
    } else {
        let projection = crate::projection::FileProjectionPublisher::new(
            input.destination_path.clone(),
            bound_authority.clone(),
            input.projection_path.clone(),
        );
        let publication = projection.publish(authority_snapshot)?;
        after_effect(MigrationPhaseV1::ProjectionPublished)?;
        journal.establish(
            MigrationPhaseV1::ProjectionPublished,
            vec![MigrationPhaseObservationV2::Projection(
                ProjectionPhaseObservationV1 {
                    authority_snapshot,
                    inventory_digest: publication.inventory_digest,
                },
            )],
        )?;
        publication
    };

    let current = aep_backend_eventlog::complete_file_snapshot(&input.destination_path, authority)
        .map_err(ApplyError::Eventlog)?;
    let (current_snapshot, current_capture) =
        authority_snapshot_identity(&bound_authority, &current)?;
    let before_watermark = before_projection_watermark(&current, authority_snapshot)?;
    let (projected_prefix, _) = authority_snapshot_identity(&bound_authority, &before_watermark)?;
    if projected_prefix != authority_snapshot {
        return Err(ApplyError::VerificationMismatch);
    }
    if !journal.is_at_least(MigrationPhaseV1::Complete) {
        after_effect(MigrationPhaseV1::Complete)?;
    }
    journal.establish(
        MigrationPhaseV1::Complete,
        vec![MigrationPhaseObservationV2::CompleteSnapshot(
            CompleteSnapshotPhaseObservationV1 {
                snapshot_id: current_snapshot,
                capture_digest: current_capture,
            },
        )],
    )?;

    let mut receipt = MigrationReceiptV1 {
        format: MigrationReceiptFormatV1,
        migration_id: input.intent.migration_id.clone(),
        intent_digest: input.intent.intent_digest,
        source_snapshot: input.intent.source_snapshot,
        source_config_digest: input.intent.config_digest,
        authority: bound_authority,
        mapping: input.intent.mapping.clone(),
        import_comparison_digest: authority_capture,
        selected_selector_digest: intended_selector_digest,
        covered_authority_snapshot: authority_snapshot,
        projection_inventory_digest: publication.inventory_digest,
        receipt_digest: ReceiptDigestV1(DigestV1::from_bytes([0; 32])),
    };
    receipt.receipt_digest = ReceiptDigestV1(digest_omitting(
        "aep.migration.receipt/1",
        &receipt,
        "receipt_digest",
    )?);
    create_or_compare(
        &input.phase_root.join("receipt.json"),
        &compact_line(&receipt)?,
    )?;
    Ok(receipt)
}

/// Reconstructs the exact provider-complete prefix immediately before the latest projection
/// watermark command. The full current snapshot is never filtered: this derived historical prefix
/// is used only to prove what the watermark projected.
pub fn before_projection_watermark(
    current: &CompleteStoreSnapshot,
    projected: AuthoritySnapshotIdV1,
) -> Result<CompleteStoreSnapshot, ApplyError> {
    let mut prior = current.clone();
    let identity = format!(
        "projection-watermark-{}",
        projected.0.as_wire().trim_start_matches("sha256:")
    );
    let index = prior
        .histories
        .iter()
        .position(|subject| {
            subject.history.subject.entity == aep_backend_eventlog::PROJECTION_METADATA_AS
                && subject.history.subject.id == identity
        })
        .ok_or(ApplyError::VerificationMismatch)?;
    let history = &prior.histories[index];
    if !matches!(history.history.origin, HistoryOrigin::Genesis)
        || history.history.records.len() != 1
        || history.terminal.revision != 1
    {
        return Err(ApplyError::VerificationMismatch);
    }
    let record = &history.history.records[0];
    let entity_store::asynchronous::RecordedEntry::Decision(commit) = &record.entry else {
        return Err(ApplyError::VerificationMismatch);
    };
    if record.entry.subject() != history.history.subject
        || record.receipt.revision != 1
        || record.receipt.record_id != format!("{identity}@1")
        || record.receipt.member_index != 0
        || record.receipt.subject != history.history.subject
        || record.receipt.kind != RecordKind::Decision
        || record.receipt.batch_key != BatchKey::Named(identity.clone())
        || record.position != record.receipt.position
        || record.entry.record_id() != record.receipt.record_id
        || record.entry.revision() != record.receipt.revision
    {
        return Err(ApplyError::VerificationMismatch);
    }
    if !matches!(
        commit.envelope.record.command,
        entity_core::DecisionCommand::Create { .. }
    ) {
        return Err(ApplyError::VerificationMismatch);
    }
    let document = history
        .terminal
        .fields
        .get("document")
        .ok_or(ApplyError::VerificationMismatch)?;
    let watermark: ProjectionWatermarkV1 = serde_json::from_value(document.clone())?;
    if watermark.authority_snapshot != projected
        || watermark.watermark_digest
            != crate::projection::projection_watermark_digest(
                watermark.authority_snapshot,
                watermark.projection_inventory_digest,
            )
    {
        return Err(ApplyError::VerificationMismatch);
    }
    prior.histories.remove(index);
    Ok(prior)
}

pub struct PhaseJournal {
    root: PathBuf,
    migration_id: MigrationIdV1,
    intent_digest: IntentDigestV1,
    predecessor: PresenceV1<PhaseDigestV1>,
    current: Option<MigrationPhaseV1>,
}

impl PhaseJournal {
    pub fn open(
        root: PathBuf,
        migration_id: MigrationIdV1,
        intent_digest: IntentDigestV1,
    ) -> Result<Self, ApplyError> {
        fs::create_dir_all(root.join("phases"))?;
        let current_record = read_json::<CurrentPhaseV2>(&root.join("current.json"))?;
        if let Some(current) = &current_record {
            if current.migration_id != migration_id || current.intent_digest != intent_digest {
                return Err(ApplyError::EvidenceConflict);
            }
            let phases = [
                MigrationPhaseV1::Prepared,
                MigrationPhaseV1::DestinationProvisioned,
                MigrationPhaseV1::Imported,
                MigrationPhaseV1::Verified,
                MigrationPhaseV1::Published,
                MigrationPhaseV1::Selected,
                MigrationPhaseV1::ProjectionPublished,
                MigrationPhaseV1::Complete,
            ];
            let mut predecessor = PresenceV1::Missing;
            for phase in phases
                .into_iter()
                .take(usize::from(phase_ordinal(current.phase)))
            {
                let path = root.join("phases").join(format!(
                    "{:02}-{}.json",
                    phase_ordinal(phase),
                    phase_name(phase)
                ));
                let record =
                    read_json::<PhaseRecordV2>(&path)?.ok_or(ApplyError::EvidenceConflict)?;
                if record.migration_id != migration_id
                    || record.intent_digest != intent_digest
                    || record.phase != phase
                    || record.predecessor != predecessor
                    || record.phase_digest
                        != PhaseDigestV1(digest_omitting(
                            "aep.migration.phase/2",
                            &record,
                            "phase_digest",
                        )?)
                {
                    return Err(ApplyError::EvidenceConflict);
                }
                predecessor = PresenceV1::Present(record.phase_digest);
            }
            if current.phase_digest
                != match predecessor {
                    PresenceV1::Present(value) => value,
                    PresenceV1::Missing => return Err(ApplyError::EvidenceConflict),
                }
            {
                return Err(ApplyError::EvidenceConflict);
            }
        }
        let predecessor = current_record
            .as_ref()
            .map_or(PresenceV1::Missing, |current| {
                PresenceV1::Present(current.phase_digest)
            });
        let current = current_record.map(|record| record.phase);
        Ok(Self {
            root,
            migration_id,
            intent_digest,
            predecessor,
            current,
        })
    }

    pub fn is_at_least(&self, phase: MigrationPhaseV1) -> bool {
        self.current
            .is_some_and(|current| phase_ordinal(current) >= phase_ordinal(phase))
    }

    pub fn establish(
        &mut self,
        phase: MigrationPhaseV1,
        observations: Vec<MigrationPhaseObservationV2>,
    ) -> Result<PhaseRecordV2, ApplyError> {
        if self.is_at_least(phase) {
            let path = self.root.join("phases").join(format!(
                "{:02}-{}.json",
                phase_ordinal(phase),
                phase_name(phase)
            ));
            return read_json(&path)?.ok_or(ApplyError::EvidenceConflict);
        }
        let mut record = PhaseRecordV2 {
            format: PhaseRecordFormatV2,
            migration_id: self.migration_id.clone(),
            intent_digest: self.intent_digest,
            phase,
            predecessor: self.predecessor.clone(),
            observations,
            phase_digest: PhaseDigestV1(DigestV1::from_bytes([0; 32])),
        };
        record.phase_digest = PhaseDigestV1(digest_omitting(
            "aep.migration.phase/2",
            &record,
            "phase_digest",
        )?);
        let ordinal = phase_ordinal(phase);
        let phase_path = self
            .root
            .join("phases")
            .join(format!("{ordinal:02}-{}.json", phase_name(phase)));
        create_or_compare(&phase_path, &compact_line(&record)?)?;
        let current = CurrentPhaseV2 {
            format: CurrentPhaseFormatV2,
            migration_id: self.migration_id.clone(),
            intent_digest: self.intent_digest,
            phase,
            phase_digest: record.phase_digest,
        };
        replace_file(&self.root.join("current.json"), &compact_line(&current)?)?;
        self.predecessor = PresenceV1::Present(record.phase_digest);
        self.current = Some(phase);
        Ok(record)
    }
}

fn validate_completed_receipt<F>(
    input: &ApplyInputs,
    receipt: &MigrationReceiptV1,
    finalize_selector: &F,
) -> Result<(), ApplyError>
where
    F: Fn(&AuthorityCoordinateV1) -> Result<Vec<u8>, ApplyError>,
{
    let journal = PhaseJournal::open(
        input.phase_root.clone(),
        input.intent.migration_id.clone(),
        input.intent.intent_digest,
    )?;
    if !journal.is_at_least(MigrationPhaseV1::Complete) {
        return Err(ApplyError::EvidenceConflict);
    }
    let binding_path = input
        .phase_root
        .join("phases/02-destination-provisioned.json");
    let binding_record =
        read_json::<PhaseRecordV2>(&binding_path)?.ok_or(ApplyError::EvidenceConflict)?;
    let binding = binding_record
        .observations
        .iter()
        .find_map(|observation| match observation {
            MigrationPhaseObservationV2::Binding(value) => Some(value),
            _ => None,
        })
        .ok_or(ApplyError::EvidenceConflict)?;
    if receipt.migration_id != input.intent.migration_id
        || receipt.intent_digest != input.intent.intent_digest
        || receipt.source_snapshot != input.intent.source_snapshot
        || receipt.source_config_digest != input.intent.config_digest
        || receipt.authority != binding.authority
        || receipt.mapping != input.intent.mapping
        || receipt.selected_selector_digest != binding.intended_selector_digest
        || receipt.receipt_digest != migration_receipt_digest(receipt)?
        || fs::read(&input.selector_path)? != binding.intended_selector_bytes.as_bytes()
        || finalize_selector(&binding.authority)? != binding.intended_selector_bytes.as_bytes()
    {
        return Err(ApplyError::EvidenceConflict);
    }
    Ok(())
}

fn adapter_authority(authority: &AuthorityCoordinateV1) -> Authority {
    Authority {
        logical_scope: authority.logical_scope.as_str().to_owned(),
        tenant: authority.tenant.as_str().to_owned(),
        stream_identity: authority.stream_identity.as_str().to_owned(),
    }
}

fn operation_context(id: &MigrationIdV1, phase: &str) -> EventlogOperationContext {
    let request = format!("{}:{phase}", id.as_str());
    EventlogOperationContext {
        subject: "aep-planning-migration".to_owned(),
        actor: "aep-planning-migration".to_owned(),
        request_id: request.clone(),
        trace_id: id.as_str().to_owned(),
        causation_id: Some(id.as_str().to_owned()),
        causation_depth: 1,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

pub fn authority_snapshot_identity(
    authority: &AuthorityCoordinateV1,
    snapshot: &CompleteStoreSnapshot,
) -> Result<(AuthoritySnapshotIdV1, DigestV1), ApplyError> {
    if snapshot.coverage != StoreCoverage::CompleteSnapshot {
        return Err(ApplyError::VerificationMismatch);
    }
    let mut subjects = snapshot.histories.iter().collect::<Vec<_>>();
    subjects.sort_by(|left, right| left.history.subject.cmp(&right.history.subject));
    let mut parts = vec![
        snapshot.scope.as_bytes().to_vec(),
        b"complete_snapshot".to_vec(),
        (subjects.len() as u64).to_be_bytes().to_vec(),
    ];
    for subject in subjects {
        parts.push(subject.history.subject.entity.as_bytes().to_vec());
        parts.push(subject.history.subject.id.as_bytes().to_vec());
        parts.push(serde_json::to_vec(&subject.terminal)?);
        match &subject.history.origin {
            HistoryOrigin::Genesis => parts.push(b"genesis".to_vec()),
            HistoryOrigin::Imported(anchor) => {
                parts.push(b"imported".to_vec());
                parts.push(serde_json::to_vec(anchor)?);
            }
        }
        parts.push(
            (subject.history.records.len() as u64)
                .to_be_bytes()
                .to_vec(),
        );
        for record in &subject.history.records {
            parts.push(serde_json::to_vec(&record.entry)?);
            parts.push(serde_json::to_vec(&record.position)?);
            parts.push(serde_json::to_vec(&record.receipt)?);
            match record.expect {
                Expect::Absent => parts.push(b"absent".to_vec()),
                Expect::Revision(revision) => {
                    parts.push(b"revision".to_vec());
                    parts.push(revision.to_be_bytes().to_vec());
                }
            }
            parts.push(record.request_bytes.clone());
            parts.push(record.record_bytes.clone());
        }
    }
    let transcript = frame_v1("aep.migration.complete-transcript/1", &parts)?;
    let value = digest_parts_v1(
        "aep.migration.authority-snapshot/1",
        &[
            authority.logical_scope.as_str().as_bytes().to_vec(),
            authority.tenant.as_str().as_bytes().to_vec(),
            authority.stream_identity.as_str().as_bytes().to_vec(),
            transcript,
        ],
    )?;
    Ok((AuthoritySnapshotIdV1(value), value))
}

pub fn intent_digest(intent: &MigrationIntentV1) -> Result<IntentDigestV1, ApplyError> {
    Ok(IntentDigestV1(digest_omitting(
        "aep.migration.intent/1",
        intent,
        "intent_digest",
    )?))
}

pub fn intent_digest_v2(intent: &MigrationIntentV2) -> Result<IntentDigestV1, ApplyError> {
    Ok(IntentDigestV1(digest_omitting(
        "aep.migration.intent/2",
        intent,
        "intent_digest",
    )?))
}

/// Recomputes the immutable migration receipt identity without its self-authenticating field.
pub fn migration_receipt_digest(
    receipt: &MigrationReceiptV1,
) -> Result<ReceiptDigestV1, ApplyError> {
    Ok(ReceiptDigestV1(digest_omitting(
        "aep.migration.receipt/1",
        receipt,
        "receipt_digest",
    )?))
}

fn digest_omitting(
    domain: &str,
    value: &impl Serialize,
    field: &str,
) -> Result<DigestV1, ApplyError> {
    let mut value = serde_json::to_value(value)?;
    let object = value.as_object_mut().ok_or(ApplyError::EvidenceConflict)?;
    if object.remove(field).is_none() {
        return Err(ApplyError::EvidenceConflict);
    }
    Ok(digest_parts_v1(domain, &[serde_json::to_vec(&value)?])?)
}

fn create_or_compare(path: &Path, bytes: &[u8]) -> Result<(), ApplyError> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
            sync_parent(path)?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(path)? == bytes {
                Ok(())
            } else {
                Err(ApplyError::EvidenceConflict)
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), ApplyError> {
    let temporary = path.with_extension("aep-migration-tmp");
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
    {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(&temporary)? != bytes {
                return Err(ApplyError::EvidenceConflict);
            }
        }
        Err(error) => return Err(error.into()),
    }
    fs::rename(&temporary, path)?;
    sync_parent(path)?;
    Ok(())
}

fn publish_directory(stage: &Path, destination: &Path) -> Result<(), ApplyError> {
    if destination.exists() {
        return Err(ApplyError::EvidenceConflict);
    }
    fs::rename(stage, destination)?;
    sync_parent(destination)
}

fn sync_parent(path: &Path) -> Result<(), ApplyError> {
    let parent = path.parent().ok_or(ApplyError::EvidenceConflict)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, ApplyError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn compact_line(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> DigestV1 {
    DigestV1::from_bytes(Sha256::digest(bytes).into())
}

const fn phase_ordinal(phase: MigrationPhaseV1) -> u8 {
    match phase {
        MigrationPhaseV1::Prepared => 1,
        MigrationPhaseV1::DestinationProvisioned => 2,
        MigrationPhaseV1::Imported => 3,
        MigrationPhaseV1::Verified => 4,
        MigrationPhaseV1::Published => 5,
        MigrationPhaseV1::Selected => 6,
        MigrationPhaseV1::ProjectionPublished => 7,
        MigrationPhaseV1::Complete => 8,
    }
}

const fn phase_name(phase: MigrationPhaseV1) -> &'static str {
    match phase {
        MigrationPhaseV1::Prepared => "prepared",
        MigrationPhaseV1::DestinationProvisioned => "destination-provisioned",
        MigrationPhaseV1::Imported => "imported",
        MigrationPhaseV1::Verified => "verified",
        MigrationPhaseV1::Published => "published",
        MigrationPhaseV1::Selected => "selected",
        MigrationPhaseV1::ProjectionPublished => "projection-published",
        MigrationPhaseV1::Complete => "complete",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct AllowWriter;

    impl WriterControl for AllowWriter {
        type Guard = ();

        fn acquire(&self, _intent: &MigrationIntentV2) -> Result<Self::Guard, WriterControlError> {
            Ok(())
        }

        fn recheck(
            &self,
            _guard: &mut Self::Guard,
            _source_snapshot: SourceSnapshotIdV1,
            _selector_digest: DigestV1,
        ) -> Result<(), WriterControlError> {
            Ok(())
        }

        fn retire_source(&self, _guard: &mut Self::Guard) -> Result<(), WriterControlError> {
            Ok(())
        }
    }

    #[cfg(unix)]
    fn path(value: &Path) -> HostPathV1 {
        use std::os::unix::ffi::OsStrExt as _;
        HostPathV1::Unix(HexBytesV1::new(value.as_os_str().as_bytes().to_vec()))
    }

    #[test]
    fn provider_assigned_apply_binds_one_identity_and_exact_retry_returns_one_receipt() {
        let root = std::env::temp_dir().join(format!(
            "aep-provider-assigned-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let phase_root = root.join("migrations/one");
        let staging_path = phase_root.join("stage/authority");
        let destination_path = root.join("state");
        let projection_path = root.join("planning");
        let selector_path = root.join("project.yaml");
        fs::create_dir_all(&root).expect("fixture root");
        fs::write(&selector_path, b"version: aep.project/1\n").expect("legacy selector");
        let source_snapshot = SourceSnapshotIdV1(DigestV1::from_bytes([1; 32]));
        let selector_digest = selector_digest_v1(b"version: aep.project/1\n");
        let mut intent = MigrationIntentV2 {
            format: MigrationIntentFormatV2,
            migration_id: MigrationIdV1::new("provider-assigned-test").expect("migration"),
            source_snapshot,
            selector_digest,
            config_digest: DigestV1::from_bytes([2; 32]),
            source_coordinate: SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                root: path(&root.join("legacy")),
            }),
            migration_root: path(&phase_root),
            staging_path: path(&staging_path),
            destination_path: path(&destination_path),
            projection_path: path(&projection_path),
            destination_request: DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-test").expect("scope"),
                tenant: AuthorityValueV1::new("tenant-test").expect("tenant"),
            },
            mapping: MappingIdentityV1 {
                mapping_version: MappingFormatV1,
                definition_digests: Vec::new(),
            },
            selector_version: ProjectVersionV1::V2,
            intent_digest: IntentDigestV1(DigestV1::from_bytes([0; 32])),
        };
        intent.intent_digest = intent_digest_v2(&intent).expect("intent digest");
        let input = ApplyInputs {
            intent,
            selector_path: selector_path.clone(),
            phase_root: phase_root.clone(),
            staging_path: staging_path.clone(),
            destination_path: destination_path.clone(),
            projection_path,
            recovery_capture_bytes: b"{}\n".to_vec(),
            source_capture_digest: DigestV1::from_bytes([3; 32]),
            histories: Vec::new(),
        };
        let selector = |authority: &AuthorityCoordinateV1| {
            let mut bytes = serde_json::to_vec(authority)?;
            bytes.push(b'\n');
            Ok(bytes)
        };

        let first = apply(&AllowWriter, &input, selector).expect("fresh apply completes");
        assert!(!first.authority.stream_identity.as_str().is_empty());
        assert!(destination_path.exists());
        assert!(!staging_path.exists());
        assert_eq!(
            fs::read(&selector_path).expect("selected bytes"),
            selector(&first.authority).expect("selector")
        );
        let second = apply(&AllowWriter, &input, selector).expect("same request resumes");
        assert_eq!(
            second, first,
            "retry returns the immutable original receipt"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn provider_assigned_apply_recovers_after_every_phase_effect_without_replacing_authority() {
        let interruption_points = [
            (MigrationPhaseV1::Prepared, 1_u8),
            // Fresh provider creation and ER binding are separate effects inside the second
            // logical phase. Both gaps must reopen the same physical authority.
            (MigrationPhaseV1::DestinationProvisioned, 1),
            (MigrationPhaseV1::DestinationProvisioned, 2),
            (MigrationPhaseV1::Imported, 1),
            (MigrationPhaseV1::Verified, 1),
            (MigrationPhaseV1::Published, 1),
            (MigrationPhaseV1::Selected, 1),
            (MigrationPhaseV1::ProjectionPublished, 1),
            (MigrationPhaseV1::Complete, 1),
        ];

        for (interrupted, target_occurrence) in interruption_points {
            let root = std::env::temp_dir().join(format!(
                "aep-provider-assigned-interruption-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            let phase_root = root.join("migrations/one");
            let staging_path = phase_root.join("stage/authority");
            let destination_path = root.join("state");
            let projection_path = root.join("planning");
            let selector_path = root.join("project.yaml");
            fs::create_dir_all(&root).expect("fixture root");
            fs::write(&selector_path, b"version: aep.project/1\n").expect("legacy selector");
            let source_snapshot = SourceSnapshotIdV1(DigestV1::from_bytes([1; 32]));
            let selector_digest = selector_digest_v1(b"version: aep.project/1\n");
            let mut intent = MigrationIntentV2 {
                format: MigrationIntentFormatV2,
                migration_id: MigrationIdV1::new(format!(
                    "provider-assigned-interruption-{}",
                    phase_name(interrupted)
                ))
                .expect("migration"),
                source_snapshot,
                selector_digest,
                config_digest: DigestV1::from_bytes([2; 32]),
                source_coordinate: SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                    root: path(&root.join("legacy")),
                }),
                migration_root: path(&phase_root),
                staging_path: path(&staging_path),
                destination_path: path(&destination_path),
                projection_path: path(&projection_path),
                destination_request: DestinationRequestV2::ProviderAssigned {
                    logical_scope: AuthorityValueV1::new("planning-test").expect("scope"),
                    tenant: AuthorityValueV1::new("tenant-test").expect("tenant"),
                },
                mapping: MappingIdentityV1 {
                    mapping_version: MappingFormatV1,
                    definition_digests: Vec::new(),
                },
                selector_version: ProjectVersionV1::V2,
                intent_digest: IntentDigestV1(DigestV1::from_bytes([0; 32])),
            };
            intent.intent_digest = intent_digest_v2(&intent).expect("intent digest");
            let input = ApplyInputs {
                intent,
                selector_path: selector_path.clone(),
                phase_root: phase_root.clone(),
                staging_path: staging_path.clone(),
                destination_path: destination_path.clone(),
                projection_path,
                recovery_capture_bytes: b"{}\n".to_vec(),
                source_capture_digest: DigestV1::from_bytes([3; 32]),
                histories: Vec::new(),
            };
            let selector = |authority: &AuthorityCoordinateV1| {
                let mut bytes = serde_json::to_vec(authority)?;
                bytes.push(b'\n');
                Ok(bytes)
            };
            let mut occurrence = 0_u8;
            let failure = apply_with_hook(&AllowWriter, &input, selector, |completed| {
                if completed == interrupted {
                    occurrence += 1;
                }
                if completed == interrupted && occurrence == target_occurrence {
                    Err(ApplyError::EvidenceConflict)
                } else {
                    Ok(())
                }
            })
            .expect_err("injected interruption must stop before phase evidence");
            assert!(matches!(failure, ApplyError::EvidenceConflict));

            let minted_before_retry = if phase_ordinal(interrupted)
                >= phase_ordinal(MigrationPhaseV1::DestinationProvisioned)
            {
                let authority_path = if destination_path.exists() {
                    &destination_path
                } else {
                    &staging_path
                };
                Some(
                    aep_backend_eventlog::prepare_file(authority_path, "tenant-test")
                        .expect("interrupted destination retains provider identity"),
                )
            } else {
                None
            };

            if interrupted == MigrationPhaseV1::Imported {
                let retained = phase_root.join("retained-bound-authority");
                fs::rename(&staging_path, &retained).expect("retain the bound stage");
                let replacement = aep_backend_eventlog::prepare_file(&staging_path, "tenant-test")
                    .expect("create a foreign replacement stage");
                assert_ne!(
                    Some(replacement),
                    minted_before_retry,
                    "the substitution fixture must have a different provider identity"
                );
                let substituted = apply(&AllowWriter, &input, selector)
                    .expect_err("a replacement at the same owned path must be refused");
                assert!(matches!(substituted, ApplyError::AuthorityIdentityMismatch));
                fs::remove_dir_all(&staging_path).expect("remove replacement stage");
                fs::rename(retained, &staging_path).expect("restore bound stage");
            }

            let receipt = apply(&AllowWriter, &input, selector)
                .unwrap_or_else(|error| panic!("{interrupted:?} retry must complete: {error}"));
            if let Some(minted) = minted_before_retry {
                assert_eq!(
                    receipt.authority.stream_identity.as_str(),
                    minted,
                    "{interrupted:?} retry replaced the provider identity"
                );
            }
            let repeated = apply(&AllowWriter, &input, selector)
                .unwrap_or_else(|error| panic!("{interrupted:?} completed retry failed: {error}"));
            assert_eq!(
                receipt, repeated,
                "{interrupted:?} receipt changed on retry"
            );
            assert_eq!(
                PhaseJournal::open(
                    phase_root,
                    input.intent.migration_id.clone(),
                    input.intent.intent_digest,
                )
                .expect("phase chain opens")
                .current,
                Some(MigrationPhaseV1::Complete)
            );

            let _ = fs::remove_dir_all(root);
        }
    }
}
