//! `aep plan store` — explicit planning-authority inspection and migration.

#![allow(missing_docs)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use aep_contract::migration::{
    ApplyCompleteV1, ApplyFormatV1, ApplyOutcomeV1, ApplyRefusedV1, ApplyResultV1, AuthorityCoordinateV1,
    AuthoritySnapshotIdV1, AuthorityValueV1, BackendKindV1, CommandRefusalCodeV1,
    CommandRefusalV1, DiagnosticCoordinateV1, DigestV1,
    DryRunAdmittedV2, DryRunFormatV2, DryRunOutcomeV2, DryRunRefusedV1, DryRunResultV2,
    HistorySummaryV1, InspectionFormatV1, InspectionObservedV1, InspectionOutcomeV1,
    InspectionReadinessV1, InspectionResultV1, InventoryCountsV1, MappingFormatV1,
    MappingIdentityV1, MigrationIdV1, PresenceV1, ProjectVersionV1,
    MigrationReceiptV1,
    RebuildFormatV1, RebuildOutcomeV1, RebuildRefusedV1, RebuildResultV1, RebuildUncertainV1,
    RebuiltV1, RefusedV1,
    SelectorDiagnosticV1, VerificationFormatV1, VerificationOutcomeV1, VerificationRefusedV1,
    VerificationResultV1, VerifiedV1, VerificationMismatchV1, ProjectionObservationV1,
    ProjectionDriftV1, ProjectionWatermarkV1, ProjectionInventoryDigestV1,
    SelectionV1, SourceCoordinateV1, SourceSnapshotIdV1, MarkdownSourceCoordinateV1,
    SqliteSourceCoordinateV1, HybridSourceCoordinateV1, SqlReplicaCoordinateV1,
    SqliteReplicaCoordinateV1, PostgresReplicaCoordinateV1, EventlogSourceCoordinateV1,
    HybridPolicyWordsV1, MigrationIntentFormatV2, MigrationIntentV2, IntentDigestV1,
    DestinationRequestV2, DestinationRequirementsV2,
    ObservationOutcomeV1, LegacyRawCaptureV1, AuthorityObservationV1,
};
use aep_backend_markdown::{MarkdownStore, StoreReport};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum StoreOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CommonArgs {
    /// Exact project selector. Ordinary discovery is used when omitted.
    #[arg(long)]
    project: Option<PathBuf>,
    /// Deterministic result encoding.
    #[arg(long, value_enum, default_value_t = StoreOutputFormat::Text)]
    format: StoreOutputFormat,
}

#[derive(Debug, Clone, Args)]
// The prefix is the stable public flag vocabulary (`--authority-*`), including clap's conflict
// references; dropping it from Rust fields would make the parser definition harder to audit.
#[allow(clippy::struct_field_names)]
pub(crate) struct AuthorityArgs {
    #[arg(long)]
    authority_scope: String,
    #[arg(long)]
    authority_tenant: String,
    /// Let the fresh destination provider assign its physical stream identity.
    #[arg(long, conflicts_with = "authority_identity", required_unless_present = "authority_identity")]
    authority_new: bool,
    /// Recover an exact identity only from an already owned migration stage.
    #[arg(long, conflicts_with = "authority_new", required_unless_present = "authority_new")]
    authority_identity: Option<String>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum StoreCommand {
    Inspect(CommonArgs),
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
    Verify(CommonArgs),
    Rebuild {
        #[command(flatten)]
        common: CommonArgs,
        #[arg(long)]
        authority_snapshot: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum MigrateCommand {
    DryRun {
        #[command(flatten)]
        common: CommonArgs,
        #[command(flatten)]
        authority: AuthorityArgs,
    },
    Apply {
        #[command(flatten)]
        common: CommonArgs,
        #[command(flatten)]
        authority: AuthorityArgs,
        #[arg(long)]
        snapshot: String,
        #[arg(long)]
        migration: String,
    },
}

pub(crate) fn run(command: StoreCommand) -> Result<ExitCode> {
    run_with_control(command, &UnavailableWriterControl)
}

pub(crate) fn run_with_control<C>(
    command: StoreCommand,
    control: &C,
) -> Result<ExitCode>
where
    C: aep_planning_migration::WriterControl + aep_planning_migration::AuthorityWriterControl,
{
    match command {
        StoreCommand::Inspect(common) => {
            let result = inspect(&common);
            emit(&result, common.format)?;
            Ok(crate::exit_code(result.success()))
        }
        StoreCommand::Migrate {
            command: MigrateCommand::DryRun { common, authority },
        } => {
            let destination = destination_request(&authority)?;
            let result = dry_run(&common, destination);
            emit(&result, common.format)?;
            Ok(crate::exit_code(result.success()))
        }
        StoreCommand::Migrate {
            command:
                MigrateCommand::Apply {
                    common,
                    authority,
                    snapshot,
                    migration,
                },
        } => {
            let destination = destination_request(&authority)?;
            let snapshot = DigestV1::parse(&snapshot).context("invalid --snapshot")?;
            let migration = MigrationIdV1::new(migration).context("invalid --migration")?;
            let result = match resolve(&common).and_then(|resolved| {
                crate::planning_writer_fence::PlanningWriterFence::acquire(&resolved.engineering)
            }) {
                Ok(_cooperative_fence) => {
                    apply_with_control(&common, destination, snapshot, migration, control)
                }
                Err(_) => apply_refusal(
                    &common,
                    migration,
                    CommandRefusalCodeV1::WriterExclusionUnavailable,
                ),
            };
            emit(&result, common.format)?;
            Ok(crate::exit_code(result.success()))
        }
        StoreCommand::Verify(common) => {
            let result = verify(&common);
            emit(&result, common.format)?;
            Ok(crate::exit_code(result.success()))
        }
        StoreCommand::Rebuild {
            common,
            authority_snapshot,
        } => {
            let requested_snapshot = AuthoritySnapshotIdV1(
                DigestV1::parse(&authority_snapshot).context("invalid --authority-snapshot")?,
            );
            let result = match resolve(&common).and_then(|resolved| {
                crate::planning_writer_fence::PlanningWriterFence::acquire(&resolved.engineering)
            }) {
                Ok(_cooperative_fence) => {
                    rebuild_with_control(&common, requested_snapshot, control)
                }
                Err(_) => rebuild_refusal(
                    &common,
                    requested_snapshot,
                    PresenceV1::Missing,
                    CommandRefusalCodeV1::WriterExclusionUnavailable,
                ),
            };
            emit(&result, common.format)?;
            Ok(crate::exit_code(result.success()))
        }
    }
}

struct UnavailableWriterControl;

impl aep_planning_migration::WriterControl for UnavailableWriterControl {
    type Guard = ();

    fn acquire(
        &self,
        _intent: &MigrationIntentV2,
    ) -> std::result::Result<Self::Guard, aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }

    fn recheck(
        &self,
        _guard: &mut Self::Guard,
        _source_snapshot: SourceSnapshotIdV1,
        _selector_digest: DigestV1,
    ) -> std::result::Result<(), aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }

    fn retire_source(
        &self,
        _guard: &mut Self::Guard,
    ) -> std::result::Result<(), aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }
}

impl aep_planning_migration::AuthorityWriterControl for UnavailableWriterControl {
    type Guard = ();

    fn acquire_authority(
        &self,
        _authority: &AuthorityCoordinateV1,
        _requested: AuthoritySnapshotIdV1,
    ) -> std::result::Result<Self::Guard, aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }

    fn recheck_authority(
        &self,
        _guard: &mut Self::Guard,
        _authority: &AuthorityCoordinateV1,
        _requested: AuthoritySnapshotIdV1,
    ) -> std::result::Result<(), aep_planning_migration::WriterControlError> {
        Err(aep_planning_migration::WriterControlError::Unavailable)
    }
}

// These explicit matches map each failed phase to the command's closed refusal vocabulary. The
// state machine stays linear so an early return cannot accidentally permit a later effect.
#[allow(clippy::manual_let_else, clippy::too_many_lines)]
pub(crate) fn apply_with_control<C: aep_planning_migration::WriterControl>(
    common: &CommonArgs,
    destination_request: DestinationRequestV2,
    requested_snapshot: DigestV1,
    migration_id: MigrationIdV1,
    control: &C,
) -> ApplyResultV1 {
    let resolved = match resolve(common) {
        Ok(value) => value,
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnreadable),
    };
    let migration_root = match migration_root(&resolved.engineering, &migration_id) {
        Ok(value) => value,
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::IntentConflict),
    };
    if migration_root.join("intent.json").exists() {
        return resume_apply_with_control(
            common,
            &resolved,
            &destination_request,
            requested_snapshot,
            migration_id,
            migration_root,
            control,
        );
    }
    let staging_path = migration_root.join("stage/authority");
    let destination_path = resolved.engineering.join("state");
    let projection_path = resolved.engineering.join("planning");
    let mapping = mapping_identity();
    let mut intent = MigrationIntentV2 {
        format: MigrationIntentFormatV2,
        migration_id: migration_id.clone(),
        source_snapshot: SourceSnapshotIdV1(requested_snapshot),
        selector_digest: resolved.selection.selector_digest,
        config_digest: resolved.selection.config_digest,
        source_coordinate: resolved.selection.source.clone(),
        migration_root: host_path(&migration_root),
        staging_path: host_path(&staging_path),
        destination_path: host_path(&destination_path),
        projection_path: host_path(&projection_path),
        destination_request,
        mapping,
        selector_version: ProjectVersionV1::V2,
        intent_digest: IntentDigestV1(DigestV1::from_bytes([0; 32])),
    };
    intent.intent_digest = match aep_planning_migration::intent_digest_v2(&intent) {
        Ok(value) => value,
        Err(_) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::IntentConflict,
            )
        }
    };
    let mut guard = match control.acquire(&intent) {
        Ok(value) => value,
        Err(_) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::WriterExclusionUnavailable,
            )
        }
    };
    if control
        .recheck(&mut guard, intent.source_snapshot, intent.selector_digest)
        .is_err()
    {
        return apply_refusal(
            common,
            migration_id,
            CommandRefusalCodeV1::WriterExclusionUnavailable,
        );
    }
    let captured = match resolved.capture() {
        Ok(value) => value,
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnreadable),
    };
    let ObservationOutcomeV1::Complete(complete) = &captured.observation else {
        return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnstable);
    };
    if complete.raw_snapshot_id != requested_snapshot {
        return apply_refusal(common, migration_id, CommandRefusalCodeV1::SnapshotChanged);
    }
    let histories = match mapped_histories(&resolved, &complete.capture, complete.raw_snapshot_id) {
        Ok(value) => value,
        Err(code) => return apply_refusal(common, migration_id, code),
    };
    if captured.source != PresenceV1::Present(intent.source_coordinate.clone()) {
        return apply_refusal(
            common,
            migration_id,
            CommandRefusalCodeV1::SourceUnreadable,
        );
    }
    let inputs = aep_planning_migration::ApplyInputs {
        intent,
        selector_path: resolved.selector_path.clone(),
        phase_root: migration_root,
        staging_path,
        destination_path: destination_path.clone(),
        projection_path,
        recovery_capture_bytes: match compact_json_line(&captured) {
            Ok(value) => value,
            Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::IntentConflict),
        },
        source_capture_digest: complete.transcript_digest,
        histories,
    };
    execute_apply(common, &resolved, migration_id, &inputs, control, guard)
}

#[allow(clippy::manual_let_else)]
fn execute_apply<C: aep_planning_migration::WriterControl>(
    common: &CommonArgs,
    resolved: &Resolved,
    migration_id: MigrationIdV1,
    inputs: &aep_planning_migration::ApplyInputs,
    control: &C,
    guard: C::Guard,
) -> ApplyResultV1 {
    let destination_path = inputs.destination_path.clone();
    let receipt = match aep_planning_migration::apply_held(control, guard, inputs, |authority| {
        intended_v2_selector(resolved, authority)
            .map_err(|_| aep_planning_migration::ApplyError::EvidenceConflict)
    }) {
        Ok(value) => value,
        Err(aep_planning_migration::ApplyError::Writer(_)) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::WriterExclusionUnavailable),
        Err(aep_planning_migration::ApplyError::IntentConflict) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::IntentConflict),
        Err(aep_planning_migration::ApplyError::ForeignStage) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::ForeignStage),
        Err(aep_planning_migration::ApplyError::DestinationConflict) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::DestinationConflict),
        Err(aep_planning_migration::ApplyError::AuthorityIdentityMismatch) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::AuthorityIdentityMismatch),
        Err(aep_planning_migration::ApplyError::SelectorChanged) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::SelectorChanged),
        Err(aep_planning_migration::ApplyError::ReceiptConflict | aep_planning_migration::ApplyError::EvidenceConflict) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::ReceiptConflict),
        Err(aep_planning_migration::ApplyError::VerificationMismatch) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::VerificationMismatch),
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::PublishUncertain),
    };
    let snapshot = match aep_backend_eventlog::complete_file_snapshot(
        &destination_path,
        entity_eventlog::Authority {
            logical_scope: receipt.authority.logical_scope.as_str().to_owned(),
            tenant: receipt.authority.tenant.as_str().to_owned(),
            stream_identity: receipt.authority.stream_identity.as_str().to_owned(),
        },
    ) {
        Ok(value) => value,
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::AuthorityIdentityMismatch),
    };
    let (snapshot_id, capture_digest) = match aep_planning_migration::authority_snapshot_identity(&receipt.authority, &snapshot) {
        Ok(value) => value,
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::VerificationMismatch),
    };
    let facts = inventory_from_histories(&snapshot.histories);
    let current_authority = receipt.authority.clone();
    ApplyResultV1 {
        format: ApplyFormatV1,
        outcome: ApplyOutcomeV1::Complete(ApplyCompleteV1 {
            receipt,
            current: AuthorityObservationV1 {
                authority: current_authority,
                snapshot_id,
                inventory: facts.inventory,
                history: facts.history,
                capture_digest,
            },
        }),
    }
}

fn migration_root(engineering: &Path, migration_id: &MigrationIdV1) -> Result<PathBuf> {
    let migration_key = aep_contract::migration::digest_parts_v1(
        "aep.migration.directory/1",
        &[migration_id.as_str().as_bytes().to_vec()],
    )?
    .as_wire()
    .trim_start_matches("sha256:")
    .to_owned();
    Ok(engineering.join("migrations").join(migration_key))
}

#[allow(clippy::manual_let_else)]
fn resume_apply_with_control<C: aep_planning_migration::WriterControl>(
    common: &CommonArgs,
    resolved: &Resolved,
    destination_request: &DestinationRequestV2,
    requested_snapshot: DigestV1,
    migration_id: MigrationIdV1,
    migration_root: PathBuf,
    control: &C,
) -> ApplyResultV1 {
    let refuse = |code| apply_refusal(common, migration_id.clone(), code);
    let intent = match fs::read(migration_root.join("intent.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<MigrationIntentV2>(&bytes).ok())
    {
        Some(value) => value,
        None => return refuse(CommandRefusalCodeV1::IntentConflict),
    };
    if intent.migration_id != migration_id
        || intent.source_snapshot != SourceSnapshotIdV1(requested_snapshot)
        || &intent.destination_request != destination_request
        || intent.mapping != mapping_identity()
        || intent.selector_version != ProjectVersionV1::V2
        || aep_planning_migration::intent_digest_v2(&intent).ok() != Some(intent.intent_digest)
    {
        return refuse(CommandRefusalCodeV1::IntentConflict);
    }

    let staging_path = migration_root.join("stage/authority");
    let destination_path = resolved.engineering.join("state");
    let projection_path = resolved.engineering.join("planning");
    if intent.migration_root != host_path(&migration_root)
        || intent.staging_path != host_path(&staging_path)
        || intent.destination_path != host_path(&destination_path)
        || intent.projection_path != host_path(&projection_path)
    {
        return refuse(CommandRefusalCodeV1::IntentConflict);
    }

    match &resolved.plan {
        crate::planning::Plan::Eventlog {
            authority_root,
            projection_root,
            ..
        } if authority_root == &destination_path && projection_root == &projection_path => {}
        crate::planning::Plan::Eventlog { .. } => {
            return refuse(CommandRefusalCodeV1::AuthorityIdentityMismatch);
        }
        _ if resolved.selection.selector_digest == intent.selector_digest
            && resolved.selection.config_digest == intent.config_digest
            && resolved.selection.source == intent.source_coordinate => {}
        _ => return refuse(CommandRefusalCodeV1::SelectorChanged),
    }

    let mut guard = match control.acquire(&intent) {
        Ok(value) => value,
        Err(_) => return refuse(CommandRefusalCodeV1::WriterExclusionUnavailable),
    };
    if control
        .recheck(&mut guard, intent.source_snapshot, intent.selector_digest)
        .is_err()
    {
        return refuse(CommandRefusalCodeV1::WriterExclusionUnavailable);
    }

    let recovery_capture_bytes = match fs::read(migration_root.join("recovery/raw-capture.json")) {
        Ok(value) => value,
        Err(_) => return refuse(CommandRefusalCodeV1::ReceiptConflict),
    };
    let captured = match aep_contract::migration::RawCaptureObservationV1::from_json(
        &recovery_capture_bytes,
    ) {
        Ok(value) => value,
        Err(_) => return refuse(CommandRefusalCodeV1::ReceiptConflict),
    };
    if captured.source != PresenceV1::Present(intent.source_coordinate.clone()) {
        return refuse(CommandRefusalCodeV1::ReceiptConflict);
    }
    let ObservationOutcomeV1::Complete(complete) = &captured.observation else {
        return refuse(CommandRefusalCodeV1::ReceiptConflict);
    };
    if complete.raw_snapshot_id != requested_snapshot {
        return refuse(CommandRefusalCodeV1::SnapshotChanged);
    }
    let histories = match mapped_histories_for_source(
        &intent.source_coordinate,
        &complete.capture,
        complete.raw_snapshot_id,
    ) {
        Ok(value) => value,
        Err(code) => return refuse(code),
    };
    let inputs = aep_planning_migration::ApplyInputs {
        intent,
        selector_path: resolved.selector_path.clone(),
        phase_root: migration_root,
        staging_path,
        destination_path,
        projection_path,
        recovery_capture_bytes,
        source_capture_digest: complete.transcript_digest,
        histories,
    };
    execute_apply(common, resolved, migration_id, &inputs, control, guard)
}

fn compact_json_line(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn apply_refusal(common: &CommonArgs, migration_id: MigrationIdV1, code: CommandRefusalCodeV1) -> ApplyResultV1 {
    let last_proved_phase = resolve(common)
        .ok()
        .and_then(|resolved| current_phase_for_migration(&resolved.engineering, &migration_id))
        .map_or(PresenceV1::Missing, PresenceV1::Present);
    ApplyResultV1 { format: ApplyFormatV1, outcome: ApplyOutcomeV1::Refused(ApplyRefusedV1 {
        migration_id, last_proved_phase, refusals: vec![selector_refusal(common, code)],
    }) }
}

// The ordered refusal mapping mirrors the rebuild protocol: fence, capture, stage, recheck,
// publish, then verify the exact watermark boundary.
#[allow(clippy::manual_let_else, clippy::too_many_lines)]
fn rebuild_with_control<C: aep_planning_migration::AuthorityWriterControl>(
    common: &CommonArgs,
    requested: AuthoritySnapshotIdV1,
    control: &C,
) -> RebuildResultV1 {
    let resolved = match resolve(common) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::SourceUnreadable),
    };
    let crate::planning::Plan::Eventlog {
        authority_root,
        projection_root,
        authority: selected,
    } = &resolved.plan else {
        return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::IncompletePublication);
    };
    let authority = match (
        AuthorityValueV1::new(&selected.logical_scope),
        AuthorityValueV1::new(&selected.tenant),
        AuthorityValueV1::new(&selected.stream_identity),
    ) {
        (Ok(logical_scope), Ok(tenant), Ok(stream_identity)) => AuthorityCoordinateV1 {
            logical_scope,
            tenant,
            stream_identity,
        },
        _ => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::AuthorityIdentityMismatch),
    };
    let mut guard = match control.acquire_authority(&authority, requested) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::WriterExclusionUnavailable),
    };
    let adapter = || entity_eventlog::Authority {
        logical_scope: authority.logical_scope.as_str().to_owned(),
        tenant: authority.tenant.as_str().to_owned(),
        stream_identity: authority.stream_identity.as_str().to_owned(),
    };
    let first = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::SourceUnreadable),
    };
    let (first_id, _) = match aep_planning_migration::authority_snapshot_identity(&authority, &first) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::VerificationMismatch),
    };
    if first_id != requested {
        return rebuild_refusal(common, requested, PresenceV1::Present(first_id), CommandRefusalCodeV1::AuthoritySnapshotChanged);
    }
    let publisher = aep_planning_migration::FileProjectionPublisher::new(
        authority_root.clone(),
        authority.clone(),
        projection_root.clone(),
    );
    let staged = match publisher.stage(requested) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Present(first_id), CommandRefusalCodeV1::ProjectionConflict),
    };
    let staged_inventory = staged.inventory_digest();
    if control.recheck_authority(&mut guard, &authority, requested).is_err() {
        return rebuild_refusal(common, requested, PresenceV1::Present(first_id), CommandRefusalCodeV1::WriterExclusionUnavailable);
    }
    let second = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Present(first_id), CommandRefusalCodeV1::SourceUnreadable),
    };
    let (second_id, _) = match aep_planning_migration::authority_snapshot_identity(&authority, &second) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Present(first_id), CommandRefusalCodeV1::VerificationMismatch),
    };
    if second_id != requested {
        return rebuild_refusal(common, requested, PresenceV1::Present(second_id), CommandRefusalCodeV1::AuthoritySnapshotChanged);
    }
    let publication = match publisher.commit(staged) {
        Ok(value) => value,
        Err(_) => return RebuildResultV1 {
            format: RebuildFormatV1,
            outcome: RebuildOutcomeV1::Uncertain(RebuildUncertainV1 {
                requested_snapshot: requested,
                staged_inventory_digest: staged_inventory,
                refusals: vec![selector_refusal(common, CommandRefusalCodeV1::PublishUncertain)],
            }),
        },
    };
    let current = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::SourceUnreadable),
    };
    let (current_id, capture_digest) = match aep_planning_migration::authority_snapshot_identity(&authority, &current) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Missing, CommandRefusalCodeV1::VerificationMismatch),
    };
    let prior = match aep_planning_migration::before_projection_watermark(&current, requested) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Present(current_id), CommandRefusalCodeV1::ProjectionDrift),
    };
    let (prior_id, _) = match aep_planning_migration::authority_snapshot_identity(&authority, &prior) {
        Ok(value) => value,
        Err(_) => return rebuild_refusal(common, requested, PresenceV1::Present(current_id), CommandRefusalCodeV1::ProjectionDrift),
    };
    if prior_id != requested {
        return rebuild_refusal(common, requested, PresenceV1::Present(current_id), CommandRefusalCodeV1::AuthoritySnapshotChanged);
    }
    let facts = inventory_from_histories(&current.histories);
    RebuildResultV1 {
        format: RebuildFormatV1,
        outcome: RebuildOutcomeV1::Rebuilt(RebuiltV1 {
            authority: AuthorityObservationV1 {
                authority,
                snapshot_id: current_id,
                inventory: facts.inventory,
                history: facts.history,
                capture_digest,
            },
            projection: ProjectionObservationV1 {
                root: host_path(projection_root),
                authority_snapshot: requested,
                inventory_digest: publication.inventory_digest,
                watermark_digest: aep_planning_migration::projection_watermark_digest(
                    requested,
                    publication.inventory_digest,
                ),
                drift: ProjectionDriftV1::Current,
            },
            replaced_owned_paths: publication.replaced_owned_paths,
            preserved_foreign_paths: publication.preserved_foreign_paths,
        }),
    }
}

fn rebuild_refusal(
    common: &CommonArgs,
    requested_snapshot: AuthoritySnapshotIdV1,
    current_snapshot: PresenceV1<AuthoritySnapshotIdV1>,
    code: CommandRefusalCodeV1,
) -> RebuildResultV1 {
    RebuildResultV1 {
        format: RebuildFormatV1,
        outcome: RebuildOutcomeV1::Refused(RebuildRefusedV1 {
            requested_snapshot,
            current_snapshot,
            refusals: vec![selector_refusal(common, code)],
        }),
    }
}

#[allow(clippy::manual_let_else)]
fn inspect(common: &CommonArgs) -> InspectionResultV1 {
    let resolved = match resolve(common) {
        Ok(value) => value,
        Err(_) => return InspectionResultV1 {
            format: InspectionFormatV1,
            outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                refusals: vec![selector_refusal(common, CommandRefusalCodeV1::SourceUnreadable)],
            }),
        },
    };
    if matches!(resolved.plan, crate::planning::Plan::Eventlog { .. }) {
        return inspect_eventlog(common, resolved);
    }
    match resolved.inventory().map(|facts| (resolved, facts)) {
        Ok((resolved, facts)) => InspectionResultV1 {
            format: InspectionFormatV1,
            outcome: InspectionOutcomeV1::Observed(InspectionObservedV1 {
                selection: resolved.selection,
                raw_complete: facts.clean,
                inventory: facts.inventory,
                history: facts.history,
                migration_phase: PresenceV1::Missing,
                authority: PresenceV1::Missing,
                projection: PresenceV1::Missing,
                readiness: if facts.clean {
                    InspectionReadinessV1::Ready
                } else {
                    InspectionReadinessV1::SourceUnready
                },
            }),
        },
        Err(_) => InspectionResultV1 {
            format: InspectionFormatV1,
            outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                refusals: vec![selector_refusal(common, CommandRefusalCodeV1::SourceUnreadable)],
            }),
        },
    }
}

#[allow(clippy::manual_let_else, clippy::too_many_lines)]
fn inspect_eventlog(common: &CommonArgs, resolved: Resolved) -> InspectionResultV1 {
    let crate::planning::Plan::Eventlog {
        authority_root,
        projection_root,
        authority: selected,
    } = &resolved.plan else {
        unreachable!("caller selected Eventlog")
    };
    let authority = match (
        AuthorityValueV1::new(&selected.logical_scope),
        AuthorityValueV1::new(&selected.tenant),
        AuthorityValueV1::new(&selected.stream_identity),
    ) {
        (Ok(logical_scope), Ok(tenant), Ok(stream_identity)) => AuthorityCoordinateV1 {
            logical_scope,
            tenant,
            stream_identity,
        },
        _ => return InspectionResultV1 {
            format: InspectionFormatV1,
            outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                refusals: vec![selector_refusal(common, CommandRefusalCodeV1::AuthorityIdentityMismatch)],
            }),
        },
    };
    let snapshot = match aep_backend_eventlog::complete_file_snapshot(
        authority_root,
        entity_eventlog::Authority {
            logical_scope: authority.logical_scope.as_str().to_owned(),
            tenant: authority.tenant.as_str().to_owned(),
            stream_identity: authority.stream_identity.as_str().to_owned(),
        },
    ) {
        Ok(value) => value,
        Err(_) => return InspectionResultV1 {
            format: InspectionFormatV1,
            outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                refusals: vec![selector_refusal(common, CommandRefusalCodeV1::SourceUnreadable)],
            }),
        },
    };
    let (snapshot_id, capture_digest) = match aep_planning_migration::authority_snapshot_identity(
        &authority,
        &snapshot,
    ) {
        Ok(value) => value,
        Err(_) => return InspectionResultV1 {
            format: InspectionFormatV1,
            outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                refusals: vec![selector_refusal(common, CommandRefusalCodeV1::VerificationMismatch)],
            }),
        },
    };
    let facts = inventory_from_histories(&snapshot.histories);
    let projection = projection_inventory(projection_root, &snapshot)
        .ok()
        .and_then(|(inventory_digest, watermark)| {
            let prior = aep_planning_migration::before_projection_watermark(
                &snapshot,
                watermark.authority_snapshot,
            ).ok()?;
            let (prior_id, _) = aep_planning_migration::authority_snapshot_identity(
                &authority,
                &prior,
            ).ok()?;
            (prior_id == watermark.authority_snapshot).then_some(ProjectionObservationV1 {
                root: host_path(projection_root),
                authority_snapshot: watermark.authority_snapshot,
                inventory_digest,
                watermark_digest: watermark.watermark_digest,
                drift: ProjectionDriftV1::Current,
            })
        });
    let migration_phase = selected_migration_phase(
        &resolved.engineering,
        &authority,
        resolved.selection.selector_digest,
    );
    let readiness = if projection.is_some() {
        InspectionReadinessV1::Ready
    } else if migration_phase.is_some() {
        InspectionReadinessV1::MigrationIncomplete
    } else {
        InspectionReadinessV1::ProjectionDrifted
    };
    InspectionResultV1 {
        format: InspectionFormatV1,
        outcome: InspectionOutcomeV1::Observed(InspectionObservedV1 {
            selection: resolved.selection,
            raw_complete: true,
            inventory: facts.inventory.clone(),
            history: facts.history.clone(),
            migration_phase: migration_phase.map_or(PresenceV1::Missing, PresenceV1::Present),
            authority: PresenceV1::Present(AuthorityObservationV1 {
                authority,
                snapshot_id,
                inventory: facts.inventory,
                history: facts.history,
                capture_digest,
            }),
            projection: projection.map_or(PresenceV1::Missing, PresenceV1::Present),
            readiness,
        }),
    }
}

fn current_phase_for_migration(
    engineering: &Path,
    migration_id: &MigrationIdV1,
) -> Option<aep_contract::migration::MigrationPhaseV1> {
    let root = migration_root(engineering, migration_id).ok()?;
    let bytes = fs::read(root.join("current.json")).ok()?;
    if let Ok(current) = serde_json::from_slice::<aep_contract::migration::CurrentPhaseV2>(&bytes) {
        return (current.migration_id == *migration_id).then_some(current.phase);
    }
    let current = serde_json::from_slice::<aep_contract::migration::CurrentPhaseV1>(&bytes).ok()?;
    (current.migration_id == *migration_id).then_some(current.phase)
}

fn selected_migration_phase(
    engineering: &Path,
    authority: &AuthorityCoordinateV1,
    selector_digest: DigestV1,
) -> Option<aep_contract::migration::MigrationPhaseV1> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(engineering.join("migrations")).ok()? {
        let Ok(entry) = entry else { continue };
        let root = entry.path();
        let Ok(current_bytes) = fs::read(root.join("current.json")) else { continue };
        if let Ok(current) = serde_json::from_slice::<aep_contract::migration::CurrentPhaseV2>(
            &current_bytes,
        ) {
            let Ok(binding_bytes) = fs::read(root.join("phases/02-destination-provisioned.json")) else {
                continue;
            };
            let Ok(binding_record) = serde_json::from_slice::<aep_contract::migration::PhaseRecordV2>(
                &binding_bytes,
            ) else {
                continue;
            };
            let Some(binding) = binding_record.observations
                .into_iter()
                .find_map(|value| match value {
                    aep_contract::migration::MigrationPhaseObservationV2::Binding(value) => Some(value),
                    _ => None,
            }) else {
                continue;
            };
            if binding.authority == *authority
                && binding.intended_selector_digest == selector_digest
            {
                matches.push(current.phase);
            }
            continue;
        }
        let Ok(current) = serde_json::from_slice::<aep_contract::migration::CurrentPhaseV1>(
            &current_bytes,
        ) else {
            continue;
        };
        let Ok(intent_bytes) = fs::read(root.join("intent.json")) else { continue };
        let Ok(intent) = serde_json::from_slice::<aep_contract::migration::MigrationIntentV1>(
            &intent_bytes,
        ) else {
            continue;
        };
        if intent.authority == *authority && intent.intended_selector_digest == selector_digest {
            matches.push(current.phase);
        }
    }
    match matches.as_slice() {
        [phase] => Some(*phase),
        _ => None,
    }
}

fn selected_migration_receipt(
    engineering: &Path,
    authority: &AuthorityCoordinateV1,
    selector_digest: DigestV1,
) -> Result<MigrationReceiptV1> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(engineering.join("migrations"))? {
        let path = entry?.path().join("receipt.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let receipt: MigrationReceiptV1 = serde_json::from_slice(&bytes)
            .with_context(|| format!("{} is not a migration receipt", path.display()))?;
        if receipt.authority == *authority && receipt.selected_selector_digest == selector_digest {
            if aep_planning_migration::migration_receipt_digest(&receipt)?
                != receipt.receipt_digest
            {
                anyhow::bail!("{} has a mismatched receipt digest", path.display());
            }
            matches.push(receipt);
        }
    }
    match matches.as_slice() {
        [receipt] => Ok(receipt.clone()),
        [] => anyhow::bail!("the selected Eventlog authority has no completed migration receipt"),
        _ => anyhow::bail!("more than one completed migration receipt claims the selected authority"),
    }
}

#[allow(clippy::manual_let_else)]
fn dry_run(common: &CommonArgs, destination_request: DestinationRequestV2) -> DryRunResultV2 {
    let resolved = match resolve(common) {
        Ok(resolved) => resolved,
        Err(_) => return dry_refusal(common, PresenceV1::Missing, CommandRefusalCodeV1::SourceUnreadable),
    };
    let selection = resolved.selection.clone();
    let facts = match resolved.inventory() {
        Ok(facts) if facts.clean => facts,
        Ok(_) => return dry_refusal(common, PresenceV1::Present(selection), CommandRefusalCodeV1::IncompleteInventory),
        Err(_) => return dry_refusal(common, PresenceV1::Present(selection), CommandRefusalCodeV1::SourceUnreadable),
    };
    let complete = match resolved.capture() {
        Ok(aep_contract::migration::RawCaptureObservationV1 {
            observation: aep_contract::migration::ObservationOutcomeV1::Complete(complete),
            ..
        }) => complete,
        Ok(_) => return dry_refusal(common, PresenceV1::Present(selection), CommandRefusalCodeV1::SourceUnstable),
        Err(_) => return dry_refusal(common, PresenceV1::Present(selection), CommandRefusalCodeV1::SourceUnreadable),
    };
    if let Err(code) = mapped_histories(&resolved, &complete.capture, complete.raw_snapshot_id) {
        return dry_refusal(common, PresenceV1::Present(selection), code);
    }
    let source_snapshot = SourceSnapshotIdV1(complete.raw_snapshot_id);
    let mut comparison = Vec::new();
    comparison.extend_from_slice(source_snapshot.0.as_bytes());
    comparison.extend_from_slice(resolved.config_digest.as_bytes());
    comparison.extend_from_slice(
        &serde_json::to_vec(&destination_request).expect("destination request serialises"),
    );
    let migration_root = resolved.engineering.join("migrations");
    let stage_name = source_snapshot.0.as_wire().replace(':', "-");
    let destination = resolved.engineering.join("state");
    let projection = resolved.engineering.join("planning");
    DryRunResultV2 {
        format: DryRunFormatV2,
        outcome: DryRunOutcomeV2::Admitted(DryRunAdmittedV2 {
            selection,
            source_snapshot,
            inventory: facts.inventory,
            history: facts.history,
            destination_request: destination_request.clone(),
            mapping: mapping_identity(),
            comparison_digest: digest(&comparison),
            destination_requirements: DestinationRequirementsV2 {
                migration_root: host_path(&migration_root),
                staging_path: host_path(&migration_root.join(stage_name).join("stage/authority")),
                destination_path: host_path(&destination),
                projection_path: host_path(&projection),
                destination_request,
                foreign_content: Vec::new(),
            },
        }),
    }
}

fn dry_refusal(
    common: &CommonArgs,
    selection: PresenceV1<SelectionV1>,
    code: CommandRefusalCodeV1,
) -> DryRunResultV2 {
    DryRunResultV2 {
        format: DryRunFormatV2,
        outcome: DryRunOutcomeV2::Refused(DryRunRefusedV1 {
            selection,
            refusals: vec![selector_refusal(common, code)],
        }),
    }
}

// Verification reports a distinct refusal coordinate for every failed read or comparison; the
// explicit matches preserve that closed result mapping.
#[allow(clippy::manual_let_else)]
fn verify(common: &CommonArgs) -> VerificationResultV1 {
    let resolved = match resolve(common) {
        Ok(value) => value,
        Err(_) => return verification_refusal(common, PresenceV1::Missing, CommandRefusalCodeV1::SourceUnreadable),
    };
    let crate::planning::Plan::Eventlog { authority_root, projection_root, authority: selected } = &resolved.plan else {
        return verification_refusal(common, PresenceV1::Present(resolved.selection), CommandRefusalCodeV1::IncompletePublication);
    };
    let authority = match (AuthorityValueV1::new(&selected.logical_scope),
        AuthorityValueV1::new(&selected.tenant), AuthorityValueV1::new(&selected.stream_identity)) {
        (Ok(logical_scope), Ok(tenant), Ok(stream_identity)) => AuthorityCoordinateV1 { logical_scope, tenant, stream_identity },
        _ => return verification_refusal(common, PresenceV1::Present(resolved.selection), CommandRefusalCodeV1::AuthorityIdentityMismatch),
    };
    let adapter = || entity_eventlog::Authority {
        logical_scope: authority.logical_scope.as_str().to_owned(),
        tenant: authority.tenant.as_str().to_owned(),
        stream_identity: authority.stream_identity.as_str().to_owned(),
    };
    let first = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => return verification_refusal(common, PresenceV1::Present(resolved.selection), CommandRefusalCodeV1::SourceUnreadable),
    };
    let (current_id, capture_digest) = match aep_planning_migration::authority_snapshot_identity(&authority, &first) {
        Ok(value) => value,
        Err(_) => return verification_refusal(common, PresenceV1::Present(resolved.selection), CommandRefusalCodeV1::VerificationMismatch),
    };
    let (inventory_digest, watermark) = match projection_inventory(projection_root, &first) {
        Ok(value) => value,
        Err(_) => return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift),
    };
    if watermark.authority != authority
        || watermark.projection_inventory_digest != inventory_digest
        || watermark.watermark_digest != aep_planning_migration::projection_watermark_digest(
            watermark.authority_snapshot, watermark.projection_inventory_digest)
    {
        return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift);
    }
    let prior = match aep_planning_migration::before_projection_watermark(&first, watermark.authority_snapshot) {
        Ok(value) => value,
        Err(_) => return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift),
    };
    let (prior_id, _) = match aep_planning_migration::authority_snapshot_identity(&authority, &prior) {
        Ok(value) => value,
        Err(_) => return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift),
    };
    let second = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => return verification_refusal(common, PresenceV1::Present(resolved.selection), CommandRefusalCodeV1::SourceUnreadable),
    };
    let (second_id, _) = match aep_planning_migration::authority_snapshot_identity(&authority, &second) {
        Ok(value) => value,
        Err(_) => return verification_mismatch(resolved.selection, CommandRefusalCodeV1::AuthoritySnapshotChanged),
    };
    if current_id != second_id || prior_id != watermark.authority_snapshot {
        return verification_mismatch(resolved.selection, CommandRefusalCodeV1::AuthoritySnapshotChanged);
    }
    let receipt = match selected_migration_receipt(
        &resolved.engineering,
        &authority,
        resolved.selection.selector_digest,
    ) {
        Ok(receipt) => receipt,
        Err(_) => return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ReceiptConflict),
    };
    let facts = inventory_from_histories(&first.histories);
    let authority_observation = AuthorityObservationV1 {
        authority: authority.clone(), snapshot_id: current_id, inventory: facts.inventory,
        history: facts.history, capture_digest,
    };
    let projection = ProjectionObservationV1 {
        root: host_path(projection_root), authority_snapshot: watermark.authority_snapshot,
        inventory_digest, watermark_digest: watermark.watermark_digest, drift: ProjectionDriftV1::Current,
    };
    VerificationResultV1 {
        format: VerificationFormatV1,
        outcome: VerificationOutcomeV1::Verified(VerifiedV1 {
            selection: resolved.selection,
            authority: authority_observation,
            projection,
            import_comparison_digest: receipt.import_comparison_digest,
        }),
    }
}

fn verification_refusal(
    common: &CommonArgs,
    selection: PresenceV1<SelectionV1>,
    code: CommandRefusalCodeV1,
) -> VerificationResultV1 {
    VerificationResultV1 { format: VerificationFormatV1,
        outcome: VerificationOutcomeV1::Refused(VerificationRefusedV1 {
            selection, refusals: vec![selector_refusal(common, code)],
        }) }
}

fn verification_mismatch(selection: SelectionV1, code: CommandRefusalCodeV1) -> VerificationResultV1 {
    VerificationResultV1 { format: VerificationFormatV1,
        outcome: VerificationOutcomeV1::Mismatch(VerificationMismatchV1 {
            selection, authority: PresenceV1::Missing, projection: PresenceV1::Missing,
            refusals: vec![CommandRefusalV1 { code, at: DiagnosticCoordinateV1::Output(
                aep_contract::migration::OutputDiagnosticV1 { stream: aep_contract::migration::OutputStreamV1::Stdout }) }],
        }) }
}

fn projection_inventory(
    root: &Path,
    snapshot: &entity_store::asynchronous::CompleteStoreSnapshot,
) -> Result<(ProjectionInventoryDigestV1, ProjectionWatermarkV1)> {
    let report = MarkdownStore::open(root).load();
    if !report.failures.is_empty() { anyhow::bail!("projection is unreadable"); }
    let mut owned = report.documents.values().map(|stored| {
        (stored.relative_path.clone(), stored.document.render().into_bytes())
    }).collect::<Vec<_>>();
    owned.sort_by(|left, right| left.0.cmp(&right.0));
    let parts = owned.iter().flat_map(|(path, bytes)| [path.as_bytes().to_vec(), bytes.clone()]).collect::<Vec<_>>();
    let inventory = ProjectionInventoryDigestV1(aep_contract::migration::digest_parts_v1(
        "aep.planning-projection-inventory/1", &parts,
    )?);
    let mut candidates = snapshot.histories.iter().filter_map(|subject| {
        if subject.history.subject.entity != aep_backend_eventlog::PROJECTION_METADATA_AS
            || subject.history.records.len() != 1 { return None; }
        let value = subject.terminal.fields.get("document")?.clone();
        let watermark = serde_json::from_value::<ProjectionWatermarkV1>(value).ok()?;
        Some((subject.history.records[0].receipt.position.store, watermark))
    }).collect::<Vec<_>>();
    candidates.sort_by_key(|(position, _)| *position);
    let watermark = candidates.into_iter().rev().find_map(|(_, watermark)| {
        (watermark.projection_inventory_digest == inventory).then_some(watermark)
    }).context("no authority watermark covers the projection")?;
    Ok((inventory, watermark))
}

fn destination_request(args: &AuthorityArgs) -> Result<DestinationRequestV2> {
    let logical_scope = AuthorityValueV1::new(&args.authority_scope)
        .context("invalid --authority-scope")?;
    let tenant = AuthorityValueV1::new(&args.authority_tenant)
        .context("invalid --authority-tenant")?;
    match (&args.authority_identity, args.authority_new) {
        (None, true) => Ok(DestinationRequestV2::ProviderAssigned {
            logical_scope,
            tenant,
        }),
        (Some(identity), false) => Ok(DestinationRequestV2::ExactExisting {
            authority: AuthorityCoordinateV1 {
                logical_scope,
                tenant,
                stream_identity: AuthorityValueV1::new(identity)
                    .context("invalid --authority-identity")?,
            },
        }),
        _ => anyhow::bail!(
            "choose exactly one of --authority-new or --authority-identity"
        ),
    }
}

struct Resolved {
    plan: crate::planning::Plan,
    engineering: PathBuf,
    selector_path: PathBuf,
    config_digest: DigestV1,
    store_field: aep_contract::migration::StoreFieldV1,
    selection: SelectionV1,
}

struct InventoryFacts {
    inventory: InventoryCountsV1,
    history: HistorySummaryV1,
    clean: bool,
}

impl Resolved {
    fn selector_binding(&self) -> aep_planning_migration::SelectorBinding {
        aep_planning_migration::SelectorBinding {
            project_root: host_path(self.engineering.parent().unwrap_or(&self.engineering)),
            project_file: self.selection.selector.clone(),
            selector_digest: self.selection.selector_digest,
            store_field: self.store_field.clone(),
            config_digest: self.config_digest,
        }
    }

    fn capture(&self) -> std::result::Result<aep_contract::migration::RawCaptureObservationV1, aep_planning_migration::AcquisitionError> {
        match &self.plan {
            crate::planning::Plan::Markdown { root } => aep_planning_migration::capture_markdown(
                root, host_path(root), self.selector_binding(),
            ),
            crate::planning::Plan::Sqlite { path } => {
                aep_planning_migration::capture_sqlite(path, self.selector_binding())
            }
            crate::planning::Plan::Postgres { url } => {
                aep_planning_migration::capture_postgres(url, self.selector_binding())
            }
            crate::planning::Plan::Hybrid { root, replica, policy } => {
                let divergence = root.join(aep_backend_hybrid::DIVERGENCES);
                let policy = HybridPolicyWordsV1 {
                    authority: policy.authority.clone(),
                    read: policy.read.clone(),
                    on_unreachable: policy.on_unreachable.clone(),
                    on_divergence: policy.on_divergence.clone(),
                };
                match replica {
                    crate::planning::Replica::Sqlite(path) => aep_planning_migration::capture_hybrid_sqlite(
                        root, path, &divergence, policy, self.selector_binding(),
                    ),
                    crate::planning::Replica::Postgres(url) => aep_planning_migration::capture_hybrid_postgres(
                        root, url, &divergence, policy, self.selector_binding(),
                    ),
                }
            }
            crate::planning::Plan::Eventlog { .. } => Err(aep_planning_migration::AcquisitionError::InvalidCapture),
        }
    }

    fn inventory(&self) -> Result<InventoryFacts> {
        let report = match &self.plan {
            crate::planning::Plan::Markdown { root }
            | crate::planning::Plan::Hybrid { root, .. } => {
                MarkdownStore::open(root.clone()).load()
            }
            durable => {
                let backend = durable
                    .open_backend()?
                    .context("selected durable backend did not open")?;
                crate::planning::report_from_backend(&backend)?
            }
        };
        Ok(inventory(&report, matches!(self.plan, crate::planning::Plan::Markdown { .. } | crate::planning::Plan::Hybrid { .. })))
    }
}

fn resolve(common: &CommonArgs) -> Result<Resolved> {
    let here = std::env::current_dir().context("reading current directory")?;
    resolve_from(common, &here)
}

#[allow(clippy::too_many_lines)]
fn resolve_from(common: &CommonArgs, here: &Path) -> Result<Resolved> {
    let selector = if let Some(path) = &common.project { path.clone() } else {
        let root = aep_project::project::discover(here).context("no project found")?;
        root.join(aep_project::project::project_directory())
            .join(aep_domain::project::PROJECT_FILE)
    };
    let engineering = selector
        .parent()
        .context("project selector has no parent")?
        .to_path_buf();
    let selector_bytes = fs::read(&selector).with_context(|| format!("reading {}", selector.display()))?;
    let config = aep_schema::parse::project(
        std::str::from_utf8(&selector_bytes).context("project selector is not UTF-8")?,
        Some(&selector.display().to_string()),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    let store_field = serde_yaml::from_slice::<serde_yaml::Value>(&selector_bytes)
        .ok()
        .and_then(|value| value.as_mapping().map(|map| map.contains_key("store")))
        .map_or(
            aep_contract::migration::StoreFieldV1::MissingDefault,
            |present| {
                if present {
                    aep_contract::migration::StoreFieldV1::Present
                } else {
                    aep_contract::migration::StoreFieldV1::MissingDefault
                }
            },
        );
    let plan = crate::planning::Plan::for_project(&engineering)?;
    let selector_digest = aep_contract::migration::selector_digest_v1(&selector_bytes);
    let config_digest = aep_contract::migration::config_digest_v1(&selector_bytes);
    let (backend, source, authority, projection) = match &plan {
        crate::planning::Plan::Markdown { root } => (
            BackendKindV1::Markdown,
            SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 { root: host_path(root) }),
            PresenceV1::Missing,
            PresenceV1::Missing,
        ),
        crate::planning::Plan::Sqlite { path } => (
            BackendKindV1::Sqlite,
            SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 { database: host_path(path) }),
            PresenceV1::Missing,
            PresenceV1::Missing,
        ),
        crate::planning::Plan::Postgres { url } => {
            let coordinate = aep_planning_migration::postgres_source_coordinate(url)
                .map_err(|error| anyhow::anyhow!(error))?;
            (
                BackendKindV1::Postgres,
                SourceCoordinateV1::Postgres(coordinate),
                PresenceV1::Missing,
                PresenceV1::Missing,
            )
        }
        crate::planning::Plan::Hybrid { root, replica, policy } => {
            let replica = match replica {
                crate::planning::Replica::Sqlite(path) => SqlReplicaCoordinateV1::Sqlite(
                    SqliteReplicaCoordinateV1 { database: host_path(path) },
                ),
                crate::planning::Replica::Postgres(url) => {
                    let value = aep_planning_migration::postgres_source_coordinate(url)
                        .map_err(|error| anyhow::anyhow!(error))?;
                    SqlReplicaCoordinateV1::Postgres(PostgresReplicaCoordinateV1 {
                        endpoint: value.endpoint,
                        endpoint_id: value.endpoint_id,
                    })
                }
            };
            (
                BackendKindV1::Hybrid,
                SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                    local_root: host_path(root),
                    replica,
                    divergence_file: host_path(&root.join(aep_backend_hybrid::DIVERGENCES)),
                    policy: PresenceV1::Present(HybridPolicyWordsV1 {
                        authority: policy.authority.clone(),
                        read: policy.read.clone(),
                        on_unreachable: policy.on_unreachable.clone(),
                        on_divergence: policy.on_divergence.clone(),
                    }),
                }),
                PresenceV1::Missing,
                PresenceV1::Missing,
            )
        }
        crate::planning::Plan::Eventlog { authority_root, projection_root, authority } => (
            BackendKindV1::Eventlog,
            SourceCoordinateV1::Eventlog(EventlogSourceCoordinateV1 { authority_root: host_path(authority_root) }),
            PresenceV1::Present(AuthorityCoordinateV1 {
                logical_scope: AuthorityValueV1::new(&authority.logical_scope)?,
                tenant: AuthorityValueV1::new(&authority.tenant)?,
                stream_identity: AuthorityValueV1::new(&authority.stream_identity)?,
            }),
            PresenceV1::Present(host_path(projection_root)),
        ),
    };
    let project_version = match config.version {
        aep_domain::project::ProjectVersion::V1 => ProjectVersionV1::V1,
        aep_domain::project::ProjectVersion::V2 => ProjectVersionV1::V2,
    };
    Ok(Resolved {
        plan,
        engineering,
        selector_path: selector.clone(),
        config_digest,
        store_field,
        selection: SelectionV1 {
            project_version,
            backend,
            selector: host_path(&selector),
            selector_digest,
            config_digest,
            source,
            authority,
            projection,
        },
    })
}

fn inventory(report: &StoreReport, unrecorded: bool) -> InventoryFacts {
    let subjects = report.documents.len() as u64;
    let relations = report
        .documents
        .values()
        .map(|stored| stored.document.frontmatter.relations.len() as u64)
        .sum();
    InventoryFacts {
        inventory: InventoryCountsV1 {
            subjects,
            entities: subjects,
            relations,
            raw_evidence_items: report.files_read as u64,
            ..InventoryCountsV1::default()
        },
        history: HistorySummaryV1 {
            unrecorded: if unrecorded { subjects } else { 0 },
            partial: if unrecorded { 0 } else { subjects },
            complete_recorded: 0,
        },
        clean: report.is_clean(),
    }
}

fn mapped_histories(
    resolved: &Resolved,
    capture: &LegacyRawCaptureV1,
    snapshot: DigestV1,
) -> std::result::Result<
    Vec<entity_store::asynchronous::SubjectHistory>,
    CommandRefusalCodeV1,
> {
    mapped_histories_for_source(&resolved.selection.source, capture, snapshot)
}

fn mapped_histories_for_source(
    source: &SourceCoordinateV1,
    capture: &LegacyRawCaptureV1,
    snapshot: DigestV1,
) -> std::result::Result<
    Vec<entity_store::asynchronous::SubjectHistory>,
    CommandRefusalCodeV1,
> {
    let histories = match capture {
        LegacyRawCaptureV1::Markdown(raw) => {
            let SourceCoordinateV1::Markdown(_) = source else {
                return Err(CommandRefusalCodeV1::SemanticMismatch);
            };
            aep_planning_migration::markdown_boundaries_raw(raw)
                .map_err(|_| CommandRefusalCodeV1::SemanticMismatch)?
        }
        LegacyRawCaptureV1::Sqlite(raw) | LegacyRawCaptureV1::Postgres(raw) => {
            if !matches!(
                (source, capture),
                (SourceCoordinateV1::Sqlite(_), LegacyRawCaptureV1::Sqlite(_))
                    | (SourceCoordinateV1::Postgres(_), LegacyRawCaptureV1::Postgres(_))
            ) {
                return Err(CommandRefusalCodeV1::SemanticMismatch);
            }
            aep_planning_migration::sql_boundaries(raw, &snapshot.as_wire())
                .map_err(|_| CommandRefusalCodeV1::SemanticMismatch)?
        }
        LegacyRawCaptureV1::Hybrid(raw) => {
            if matches!(&raw.divergences,
                aep_contract::migration::FileImageV1::Present(value) if !value.bytes.as_bytes().is_empty()) {
                return Err(CommandRefusalCodeV1::DivergentHybrid);
            }
            if !matches!(source, SourceCoordinateV1::Hybrid(_)) {
                return Err(CommandRefusalCodeV1::SemanticMismatch);
            }
            let local = aep_planning_migration::markdown_boundaries_raw(&raw.local)
                .map_err(|_| CommandRefusalCodeV1::SemanticMismatch)?;
            let replica = aep_planning_migration::sql_boundaries(&raw.replica, &snapshot.as_wire())
                .map_err(|_| CommandRefusalCodeV1::SemanticMismatch)?;
            let local_terminal = terminal_instances(&local);
            let replica_terminal = terminal_instances(&replica);
            if local_terminal != replica_terminal {
                return Err(CommandRefusalCodeV1::DivergentHybrid);
            }
            match raw.policy.authority.as_str() {
                "local" => local,
                "replica" => replica,
                _ => return Err(CommandRefusalCodeV1::SemanticMismatch),
            }
        }
    };
    aep_planning_migration::boundaries_with_authoritative_evidence(histories, capture, snapshot)
        .map_err(|_| CommandRefusalCodeV1::SemanticMismatch)
}

fn terminal_instances(
    histories: &[entity_store::asynchronous::SubjectHistory],
) -> Vec<entity_core::EntityInstance> {
    let mut instances = histories
        .iter()
        .filter_map(|history| match &history.origin {
            entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                comparable_planning_instance(&anchor.instance)
            }
            entity_store::asynchronous::HistoryOrigin::Genesis => None,
        })
        .collect::<Vec<_>>();
    instances.sort_by(|left, right| {
        left.entity.cmp(&right.entity).then_with(|| left.id.cmp(&right.id))
    });
    instances
}

fn comparable_planning_instance(
    instance: &entity_core::EntityInstance,
) -> Option<entity_core::EntityInstance> {
    if instance.entity != aep_backend_entity::STORED_AS {
        return (!instance.entity.starts_with("aep.")).then(|| {
            let mut projected = instance.clone();
            if projected.fields.get("version").and_then(serde_json::Value::as_str) == Some("1") {
                projected.fields.remove("version");
            }
            projected
        });
    }
    let locator = instance
        .fields
        .get("$aep")?
        .get("metadata")?
        .get("locator")?
        .as_str()?
        .strip_prefix("ep://planning/store/")?;
    let (kind, name) = locator.split_once('/')?;
    if kind.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    let mut projected = instance.clone();
    kind.clone_into(&mut projected.entity);
    name.clone_into(&mut projected.id);
    projected.fields.remove("$aep");
    projected.fields.remove("status");
    if projected.fields.get("version").and_then(serde_json::Value::as_str) == Some("1") {
        projected.fields.remove("version");
    }
    Some(projected)
}

fn mapping_identity() -> MappingIdentityV1 {
    MappingIdentityV1 {
        mapping_version: MappingFormatV1,
        definition_digests: [
            aep_backend_entity::STORED_AS,
            aep_backend_entity::RELATIONS_AS,
            aep_backend_entity::AUDIT_AS,
            aep_backend_entity::APPLIED_AS,
            aep_backend_eventlog::INVOCATION_AS,
            "aep.planning-import-boundary",
        ].map(|value| digest(value.as_bytes())).to_vec(),
    }
}

fn intended_v2_selector(resolved: &Resolved, authority: &AuthorityCoordinateV1) -> Result<Vec<u8>> {
    let bytes = fs::read(&resolved.selector_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_slice(&bytes)?;
    let map = yaml.as_mapping().context("project selector is not an object")?;
    let ordered = [
        "version", "protocol", "profile", "summary", "protocols", "artifacts", "task",
        "state", "principles", "profiles", "schemas", "store", "planning_scope",
        "planning_tenant", "planning_identity", "providers",
    ];
    let mut output = String::from("{");
    let mut first = true;
    for key in ordered {
        let value = match key {
            "version" => Some(serde_json::Value::String("aep.project/2".to_owned())),
            "store" => Some(serde_json::json!({"eventlog":{"path":"state","projection":"planning"}})),
            "planning_scope" => Some(serde_json::Value::String(authority.logical_scope.as_str().to_owned())),
            "planning_tenant" => Some(serde_json::Value::String(authority.tenant.as_str().to_owned())),
            "planning_identity" => Some(serde_json::Value::String(authority.stream_identity.as_str().to_owned())),
            other => map.get(serde_yaml::Value::String(other.to_owned()))
                .map(serde_json::to_value).transpose()?,
        };
        let Some(value) = value else { continue };
        if key == "providers" && value.as_object().is_some_and(serde_json::Map::is_empty) { continue; }
        if !first { output.push(','); }
        first = false;
        output.push_str(&serde_json::to_string(key)?);
        output.push(':');
        output.push_str(&serde_json::to_string(&value)?);
    }
    output.push_str("}\n");
    Ok(output.into_bytes())
}

fn inventory_from_histories(histories: &[entity_store::asynchronous::SubjectSnapshot]) -> InventoryFacts {
    let mut inventory = InventoryCountsV1::default();
    let mut history = HistorySummaryV1::default();
    inventory.subjects = histories.len() as u64;
    for subject in histories {
        match subject.history.subject.entity.as_str() {
            aep_backend_entity::RELATIONS_AS => inventory.relations += 1,
            aep_backend_entity::AUDIT_AS => inventory.audit_records += 1,
            aep_backend_entity::APPLIED_AS => inventory.applied_commands += 1,
            aep_backend_eventlog::INVOCATION_AS | "aep.planning-import-boundary" => {},
            _ => inventory.entities += 1,
        }
        match &subject.history.origin {
            entity_store::asynchronous::HistoryOrigin::Genesis => history.complete_recorded += 1,
            entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                history.partial += 1;
                inventory.raw_evidence_items += anchor.evidence.len() as u64;
                inventory.complete_envelopes += anchor.evidence.iter().filter(|value| matches!(value, entity_store::asynchronous::LegacyEvidence::Envelope(_))).count() as u64;
                inventory.bare_decisions += anchor.evidence.iter().filter(|value| matches!(value, entity_store::asynchronous::LegacyEvidence::Decision(_))).count() as u64;
                inventory.bare_events += anchor.evidence.iter().filter(|value| matches!(value, entity_store::asynchronous::LegacyEvidence::Event(_))).count() as u64;
            }
        }
    }
    InventoryFacts { inventory, history, clean: true }
}

fn digest(bytes: &[u8]) -> DigestV1 {
    DigestV1::from_bytes(Sha256::digest(bytes).into())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use aep_contract::command::{CommandContext, CommandEnvelope, CommandService};
    use aep_contract::migration::HexBytesV1;
    use aep_contract::testing::block_on;
    use aep_domain::command::{Command, CreateEntity};
    use aep_domain::entity::{ActorRef, EntityLocator, EntityType};
    use aep_domain::time::Timestamp;

    use super::*;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
    const LEGACY_COLLISION_RECORD: &str =
        "aep.entity:01MEM0000000000000002@1#0~19c4cfd489ddecfd";

    struct DisposableWriterControl;

    fn write_one_story(planning: &Path, title: &str) {
        fs::create_dir_all(planning.join("story")).expect("story source");
        fs::write(
            planning.join("story/one.md"),
            format!(
                "---\nformat: aep.planning-md/1\nid: story:one\nkind: story\nstatus: draft\ntitle: {title}\nrelations: []\nrevision: 1\n---\n"
            ),
        )
        .expect("nonempty planning source");
    }

    fn seed_sqlite_from_markdown(planning: &Path, database: &Path) {
        let report = MarkdownStore::open(planning).load();
        assert!(report.is_clean(), "seed source must be clean");
        let graph = report
            .graph_in_workspace(Vec::<aep_domain::workspace::MemberName>::new())
            .expect("seed source graph");
        let backend = aep_backend_sqlite::SqliteBackend::open(database)
            .expect("SQLite fixture opens");
        aep_backend_memory::seed::from_manifest(
            &backend,
            &graph,
            aep_backend_markdown::backend::ORGANISATION,
            aep_backend_markdown::backend::SPACE,
            aep_domain::time::Timestamp::from_epoch_millis(1_700_000_000_000),
            &aep_domain::entity::ActorRef::parse("human:migration-fixture")
                .expect("actor"),
        )
        .expect("SQLite fixture seeds");
    }

    fn append_sqlite_observation(database: &Path) -> Vec<u8> {
        let connection = rusqlite::Connection::open(database).expect("seed database reopens");
        let (entity, id, revision): (String, String, i64) = connection
            .query_row(
                "SELECT entity,id,revision FROM instances ORDER BY entity,id LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("seed instance exists");
        let record_id = LEGACY_COLLISION_RECORD;
        let observation = entity_store::RecordedObservation {
            entity: entity.clone(),
            id: id.clone(),
            revision: u64::try_from(revision).expect("positive seed revision"),
            envelope: entity_store::Envelope::new(
                serde_json::json!({"legacy": "exact-history-envelope"}),
                record_id,
                "2026-09-16T12:00:00Z",
                Some("legacy-flow-one".to_owned()),
                Some("legacy-command-one".to_owned()),
                Some("human:migration-fixture".to_owned()),
            )
            .expect("valid legacy observation envelope"),
        };
        let document = serde_json::to_vec(&observation).expect("observation serialises");
        connection
            .execute(
                "INSERT INTO history(entity,id,position,kind,record_id,document) \
                 VALUES (?1,?2,0,'observation',?3,?4)",
                rusqlite::params![entity, id, record_id, String::from_utf8(document.clone()).unwrap()],
            )
            .expect("legacy observation inserts");
        document
    }

    fn seed_postgres_from_markdown(planning: &Path, url: &str) {
        let report = MarkdownStore::open(planning).load();
        assert!(report.is_clean(), "seed source must be clean");
        let graph = report
            .graph_in_workspace(Vec::<aep_domain::workspace::MemberName>::new())
            .expect("seed source graph");
        let backend = aep_backend_postgres::PostgresBackend::connect(url)
            .expect("PostgreSQL fixture opens");
        aep_backend_memory::seed::from_manifest(
            &backend,
            &graph,
            aep_backend_markdown::backend::ORGANISATION,
            aep_backend_markdown::backend::SPACE,
            aep_domain::time::Timestamp::from_epoch_millis(1_700_000_000_000),
            &aep_domain::entity::ActorRef::parse("human:migration-fixture")
                .expect("actor"),
        )
        .expect("PostgreSQL fixture seeds");
    }

    fn append_postgres_observation(url: &str) -> Vec<u8> {
        let mut client = postgres::Client::connect(url, postgres::NoTls)
            .expect("owned PostgreSQL fixture connects");
        let row = client
            .query_one(
                "SELECT entity,id,revision FROM instances ORDER BY entity,id LIMIT 1",
                &[],
            )
            .expect("seed instance exists");
        let entity: String = row.get(0);
        let id: String = row.get(1);
        let revision: i64 = row.get(2);
        let observation = entity_store::RecordedObservation {
            entity: entity.clone(),
            id: id.clone(),
            revision: u64::try_from(revision).expect("positive seed revision"),
            envelope: entity_store::Envelope::new(
                serde_json::json!({"legacy": "exact-postgres-history-envelope"}),
                LEGACY_COLLISION_RECORD,
                "2026-09-16T12:00:00Z",
                Some("legacy-postgres-flow".to_owned()),
                Some("legacy-postgres-command".to_owned()),
                Some("human:migration-fixture".to_owned()),
            )
            .expect("valid legacy observation envelope"),
        };
        let document = serde_json::to_vec(&observation).expect("observation serialises");
        let document_text = String::from_utf8(document.clone()).expect("JSON is UTF-8");
        client
            .execute(
                "INSERT INTO history(entity,id,position,kind,record_id,document) \
                 VALUES ($1,$2,0,'observation',$3,$4)",
                &[&entity, &id, &LEGACY_COLLISION_RECORD, &document_text],
            )
            .expect("legacy PostgreSQL observation inserts");
        document
    }

    fn isolated_postgres_url(url: &str, schema: &str) -> String {
        assert!(
            schema
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
            "the generated schema is a closed SQL identifier"
        );
        let mut client = postgres::Client::connect(url, postgres::NoTls)
            .expect("owned PostgreSQL fixture connects for schema creation");
        client
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .expect("isolated PostgreSQL schema is created once");
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}options=-csearch_path%3D{schema}")
    }

    fn drop_postgres_schema(url: &str, schema: &str) {
        let mut client = postgres::Client::connect(url, postgres::NoTls)
            .expect("owned PostgreSQL fixture connects for schema removal");
        client
            .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
            .expect("isolated PostgreSQL schema is removed");
    }

    impl aep_planning_migration::WriterControl for DisposableWriterControl {
        type Guard = ();

        fn acquire(
            &self,
            _intent: &MigrationIntentV2,
        ) -> std::result::Result<Self::Guard, aep_planning_migration::WriterControlError> {
            Ok(())
        }

        fn recheck(
            &self,
            _guard: &mut Self::Guard,
            _source_snapshot: SourceSnapshotIdV1,
            _selector_digest: DigestV1,
        ) -> std::result::Result<(), aep_planning_migration::WriterControlError> {
            Ok(())
        }

        fn retire_source(
            &self,
            _guard: &mut Self::Guard,
        ) -> std::result::Result<(), aep_planning_migration::WriterControlError> {
            Ok(())
        }
    }

    impl aep_planning_migration::AuthorityWriterControl for DisposableWriterControl {
        type Guard = ();

        fn acquire_authority(
            &self,
            _authority: &AuthorityCoordinateV1,
            _requested: AuthoritySnapshotIdV1,
        ) -> std::result::Result<Self::Guard, aep_planning_migration::WriterControlError> {
            Ok(())
        }

        fn recheck_authority(
            &self,
            _guard: &mut Self::Guard,
            _authority: &AuthorityCoordinateV1,
            _requested: AuthoritySnapshotIdV1,
        ) -> std::result::Result<(), aep_planning_migration::WriterControlError> {
            Ok(())
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn cli_dry_run_is_read_only_and_apply_selects_the_provider_minted_authority() {
        let project = std::env::temp_dir().join(format!(
            "aep-cli-provider-assigned-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let engineering = project.join(".engineering");
        let selector = engineering.join("project.yaml");
        fs::create_dir_all(engineering.join("planning")).expect("planning source");
        fs::create_dir_all(project.join("protocols")).expect("protocol source");
        fs::write(
            &selector,
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
        )
        .expect("legacy selector");
        write_one_story(&engineering.join("planning"), "One");
        let old_entry = aep_backend_markdown::journal::Entry {
            at: "2026-01-01T00:00:00Z".to_owned(),
            actor: "human:legacy".to_owned(),
            artifact: "story:one".parse().expect("artifact"),
            kind: "story".parse().expect("kind"),
            revision: 1,
            change: aep_backend_markdown::journal::Change::Created {
                status: "draft".parse().expect("status"),
            },
        };
        let bare_event = entity_core::DomainEvent {
            entity: "story".to_owned(),
            version: 1,
            id: "one".to_owned(),
            revision: 1,
            event_type: "created".to_owned(),
            from_state: None,
            to_state: "draft".to_owned(),
            changed: serde_json::Map::new(),
            removed: std::collections::BTreeSet::new(),
            args: serde_json::Map::new(),
            payload: serde_json::json!({
                "recorded_at": "2026-01-01T00:00:00Z",
                "actor": "human:legacy",
                "change": {"change": "created", "status": "draft"}
            }),
        };
        let legacy_entry_bytes = serde_json::to_vec(&old_entry).expect("old entry serialises");
        let bare_event_bytes = serde_json::to_vec(&bare_event).expect("event serialises");
        fs::write(
            engineering.join("planning/journal.jsonl"),
            [
                legacy_entry_bytes.as_slice(),
                b"\n",
                bare_event_bytes.as_slice(),
                b"\n",
            ]
            .concat(),
        )
        .expect("mixed legacy journal");
        let common = CommonArgs {
            project: Some(selector.clone()),
            format: StoreOutputFormat::Json,
        };
        let destination = DestinationRequestV2::ProviderAssigned {
            logical_scope: AuthorityValueV1::new("planning-cli-test").expect("scope"),
            tenant: AuthorityValueV1::new("tenant-cli-test").expect("tenant"),
        };

        let preview = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(preview) = preview.outcome else {
            panic!("nonempty Markdown source is admitted")
        };
        assert_eq!(preview.inventory.subjects, 1);
        assert!(
            !engineering.join("migrations").exists() && !engineering.join("state").exists(),
            "dry-run must not create a stage merely to discover an identity"
        );
        let foreign_id = MigrationIdV1::new("cli-foreign-stage").expect("migration");
        let foreign_root = migration_root(&engineering, &foreign_id).expect("migration root");
        fs::create_dir_all(foreign_root.join("stage/authority"))
            .expect("foreign stage fixture");
        let foreign = apply_with_control(
            &common,
            destination.clone(),
            preview.source_snapshot.0,
            foreign_id,
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Refused(foreign) = foreign.outcome else {
            panic!("unowned pre-existing stage is refused")
        };
        assert_eq!(foreign.refusals[0].code, CommandRefusalCodeV1::ForeignStage);
        let migration_id = MigrationIdV1::new("cli-provider-assigned").expect("migration");
        let applied = apply_with_control(
            &common,
            destination.clone(),
            preview.source_snapshot.0,
            migration_id.clone(),
            &DisposableWriterControl,
        );
        let applied = match applied.outcome {
            ApplyOutcomeV1::Complete(applied) => applied,
            other => panic!("disposable controlled CLI apply completes: {other:?}"),
        };
        assert!(!applied.receipt.authority.stream_identity.as_str().is_empty());
        let selected = fs::read_to_string(&selector).expect("selected v2 selector");
        assert!(selected.contains("\"version\":\"aep.project/2\""));
        assert!(selected.contains(applied.receipt.authority.stream_identity.as_str()));
        let nested = project.join("nested/working/directory");
        fs::create_dir_all(&nested).expect("nested discovery path");
        let discovered = CommonArgs {
            project: None,
            format: StoreOutputFormat::Json,
        };
        let ordinary = resolve_from(&discovered, &nested).expect("ordinary discovery reopens v2");
        assert_eq!(ordinary.selection.authority, PresenceV1::Present(applied.receipt.authority.clone()));
        assert!(verify(&common).success(), "selected authority reopens and verifies");
        let retried = apply_with_control(
            &common,
            destination,
            preview.source_snapshot.0,
            migration_id,
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Complete(retried) = retried.outcome else {
            panic!("ordinary selected-store retry recovers the completed result")
        };
        assert_eq!(retried.receipt, applied.receipt);

        let adapter = || entity_eventlog::Authority {
            logical_scope: applied.receipt.authority.logical_scope.as_str().to_owned(),
            tenant: applied.receipt.authority.tenant.as_str().to_owned(),
            stream_identity: applied.receipt.authority.stream_identity.as_str().to_owned(),
        };
        let before_rebuild = aep_backend_eventlog::complete_file_snapshot(
            &engineering.join("state"),
            adapter(),
        )
        .expect("capture authority before rebuild");
        let coordinates = before_rebuild
            .histories
            .iter()
            .filter(|value| {
                value.history.subject.entity == "aep.migration.LegacyRecordCoordinate"
            })
            .collect::<Vec<_>>();
        assert_eq!(coordinates.len(), 2, "both journal line shapes are retained");
        let mut journal_kinds = std::collections::BTreeSet::new();
        for coordinate in &coordinates {
            let entity_store::asynchronous::HistoryOrigin::Imported(anchor) =
                &coordinate.history.origin
            else {
                panic!("legacy coordinate is an imported boundary")
            };
            assert_eq!(
                anchor.instance.fields.get("order").and_then(serde_json::Value::as_str),
                Some("store")
            );
            assert_eq!(
                anchor.instance.fields.get("original_record_id"),
                Some(&serde_json::json!({"kind":"missing"}))
            );
            assert_eq!(
                anchor.instance.fields.get("reservation_roster_id"),
                Some(&serde_json::json!({"kind":"missing"}))
            );
            journal_kinds.insert(
                anchor
                    .instance
                    .fields
                    .get("evidence_kind")
                    .and_then(serde_json::Value::as_str)
                    .expect("closed journal evidence kind"),
            );
        }
        assert_eq!(journal_kinds, std::collections::BTreeSet::from(["change", "event"]));
        let retained_lines = before_rebuild
            .histories
            .iter()
            .filter(|value| {
                value.history.subject.entity == "aep.migration.LegacyEvidenceBlob"
            })
            .map(|value| {
                let entity_store::asynchronous::HistoryOrigin::Imported(anchor) =
                    &value.history.origin
                else {
                    panic!("legacy evidence is an imported boundary")
                };
                serde_json::from_value::<HexBytesV1>(
                    anchor
                        .instance
                        .fields
                        .get("exact_bytes")
                        .expect("exact boundary bytes")
                        .clone(),
                )
                .expect("canonical exact boundary bytes")
                .as_bytes()
                .to_vec()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            retained_lines,
            std::collections::BTreeSet::from([legacy_entry_bytes, bare_event_bytes])
        );
        fs::remove_dir_all(engineering.join("planning"))
            .expect("remove the owned projection before rebuilding");
        let rebuilt = rebuild_with_control(
            &common,
            applied.current.snapshot_id,
            &DisposableWriterControl,
        );
        let RebuildOutcomeV1::Rebuilt(rebuilt) = rebuilt.outcome else {
            panic!("writer-controlled rebuild recovers the removed projection")
        };
        assert!(engineering.join("planning/story/one.md").is_file());
        assert_eq!(rebuilt.projection.authority_snapshot, applied.current.snapshot_id);
        let after_rebuild = aep_backend_eventlog::complete_file_snapshot(
            &engineering.join("state"),
            adapter(),
        )
        .expect("capture authority after rebuild");
        let business = |snapshot: entity_store::asynchronous::CompleteStoreSnapshot| {
            snapshot
                .histories
                .into_iter()
                .filter(|value| {
                    value.history.subject.entity
                        != aep_backend_eventlog::PROJECTION_METADATA_AS
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            business(after_rebuild),
            business(before_rebuild),
            "projection recovery must not repeat or alter a business command"
        );
        let stale = rebuild_with_control(
            &common,
            applied.receipt.covered_authority_snapshot,
            &DisposableWriterControl,
        );
        let RebuildOutcomeV1::Refused(stale) = stale.outcome else {
            panic!("a stale authority snapshot must not rebuild")
        };
        assert_eq!(
            stale.refusals[0].code,
            CommandRefusalCodeV1::AuthoritySnapshotChanged
        );

        let changed = apply_with_control(
            &common,
            DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-cli-test").expect("scope"),
                tenant: AuthorityValueV1::new("changed-tenant").expect("tenant"),
            },
            preview.source_snapshot.0,
            MigrationIdV1::new("cli-provider-assigned").expect("migration"),
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Refused(changed) = changed.outcome else {
            panic!("changed request must not reuse the migration identity")
        };
        assert_eq!(changed.refusals[0].code, CommandRefusalCodeV1::IntentConflict);

        let selected_bytes = fs::read(&selector).expect("selected selector bytes");
        let selected_text = std::str::from_utf8(&selected_bytes).expect("selector UTF-8");
        let changed_selector = selected_text.replacen(
            "\"profile\":\"development.standard\"",
            "\"profile\":\"development.standard\",\"summary\":\"changed after selection\"",
            1,
        );
        assert_ne!(changed_selector.as_bytes(), selected_bytes);
        fs::write(&selector, changed_selector).expect("tampered but valid selector");
        let selector_conflict = apply_with_control(
            &common,
            DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-cli-test").expect("scope"),
                tenant: AuthorityValueV1::new("tenant-cli-test").expect("tenant"),
            },
            preview.source_snapshot.0,
            MigrationIdV1::new("cli-provider-assigned").expect("migration"),
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Refused(selector_conflict) = selector_conflict.outcome else {
            panic!("changed selected bytes must conflict with the retained receipt")
        };
        assert_eq!(
            selector_conflict.refusals[0].code,
            CommandRefusalCodeV1::ReceiptConflict
        );
        assert_eq!(
            selector_conflict.last_proved_phase,
            PresenceV1::Present(aep_contract::migration::MigrationPhaseV1::Complete)
        );
        fs::write(&selector, selected_bytes).expect("restore selected selector");

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    #[allow(clippy::items_after_statements, clippy::too_many_lines)]
    fn sqlite_and_hybrid_physical_sources_are_captured_and_divergence_refuses() {
        let project = std::env::temp_dir().join(format!(
            "aep-cli-sql-sources-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let engineering = project.join(".engineering");
        let planning = engineering.join("planning");
        let selector = engineering.join("project.yaml");
        let database = engineering.join("plan.sqlite3");
        fs::create_dir_all(project.join("protocols")).expect("protocol source");
        write_one_story(&planning, "One");
        seed_sqlite_from_markdown(&planning, &database);
        let legacy_observation = append_sqlite_observation(&database);
        let common = CommonArgs {
            project: Some(selector.clone()),
            format: StoreOutputFormat::Json,
        };
        let destination = DestinationRequestV2::ProviderAssigned {
            logical_scope: AuthorityValueV1::new("planning-sql-test").expect("scope"),
            tenant: AuthorityValueV1::new("tenant-sql-test").expect("tenant"),
        };
        let base = "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n";

        fs::write(&selector, format!("{base}store:\n  sqlite: plan.sqlite3\n"))
            .expect("SQLite selector");
        let sqlite = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(sqlite) = sqlite.outcome else {
            panic!("actual SQLite source is admitted")
        };
        assert_eq!(sqlite.inventory.subjects, 1);

        fs::write(
            &selector,
            format!(
                "{base}store:\n  hybrid:\n    authority: local\n    read: local-first\n    on_unreachable: refuse\n    on_divergence: record\n    local: markdown\n    replica:\n      sqlite: plan.sqlite3\n"
            ),
        )
        .expect("hybrid selector");
        let hybrid = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(hybrid) = hybrid.outcome else {
            panic!("matching physical hybrid source is admitted")
        };
        assert_eq!(hybrid.inventory.subjects, 1);

        write_one_story(&planning, "Changed only locally");
        let divergent = dry_run(&common, destination);
        let DryRunOutcomeV2::Refused(divergent) = divergent.outcome else {
            panic!("divergent physical hybrid source is refused")
        };
        assert_eq!(
            divergent.refusals[0].code,
            CommandRefusalCodeV1::DivergentHybrid
        );

        fs::write(&selector, format!("{base}store:\n  sqlite: plan.sqlite3\n"))
            .expect("restore SQLite selector");
        let migration_id = MigrationIdV1::new("sqlite-provider-assigned").expect("migration");
        let applied = apply_with_control(
            &common,
            DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-sql-test").expect("scope"),
                tenant: AuthorityValueV1::new("tenant-sql-test").expect("tenant"),
            },
            sqlite.source_snapshot.0,
            migration_id.clone(),
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Complete(applied) = applied.outcome else {
            panic!("physical SQLite source applies under disposable writer control")
        };
        assert!(
            verify(&common).success(),
            "SQL provider rows reopen through the ordinary Eventlog backend"
        );
        let authority = entity_eventlog::Authority {
            logical_scope: applied.receipt.authority.logical_scope.as_str().to_owned(),
            tenant: applied.receipt.authority.tenant.as_str().to_owned(),
            stream_identity: applied
                .receipt
                .authority
                .stream_identity
                .as_str()
                .to_owned(),
        };
        let authority_snapshot =
            aep_backend_eventlog::complete_file_snapshot(
                &engineering.join("state"),
                authority.clone(),
            )
                .expect("complete imported SQLite authority");
        let kinds = authority_snapshot
            .histories
            .iter()
            .map(|subject| subject.history.subject.entity.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(kinds.contains("aep.migration.LegacyRecordCoordinate"));
        assert!(kinds.contains("aep.migration.LegacyEvidenceBlob"));
        assert!(kinds.contains("aep.migration.LegacyIdReservationRoster"));
        let reserved = aep_backend_eventlog::validate_legacy_boundary_snapshot(
            &authority_snapshot,
            &authority,
        )
        .expect("the provider-complete boundary graph validates");
        assert!(reserved.contains(LEGACY_COLLISION_RECORD));
        let mut missing_blob = authority_snapshot.clone();
        let blob_index = missing_blob
            .histories
            .iter()
            .position(|subject| {
                subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob"
            })
            .expect("one retained evidence blob");
        missing_blob.histories.remove(blob_index);
        assert!(
            aep_backend_eventlog::validate_legacy_boundary_snapshot(&missing_blob, &authority)
                .expect_err("a missing provider subject must refuse")
                .contains("is absent")
        );
        let mut altered_blob = authority_snapshot.clone();
        let altered = altered_blob
            .histories
            .iter_mut()
            .find(|subject| {
                subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob"
            })
            .expect("one retained evidence blob");
        altered.terminal.fields.insert(
            "exact_bytes".to_owned(),
            serde_json::to_value(HexBytesV1::new(b"altered".to_vec()))
                .expect("altered bytes serialise"),
        );
        assert!(
            aep_backend_eventlog::validate_legacy_boundary_snapshot(&altered_blob, &authority)
                .expect_err("altered provider bytes must refuse")
                .contains("disagrees")
        );

        // A complete envelope whose source proves no order cannot enter ImportedRecordEvidence.
        // Bind it through the ordinary coordinate/blob/roster subjects and prove that those
        // provider-complete values alone retain its bytes and reserve its global identity.
        let recovery = fs::read(
            migration_root(&engineering, &migration_id)
                .expect("migration root")
                .join("recovery/raw-capture.json"),
        )
        .expect("retained raw capture");
        let captured = aep_contract::migration::RawCaptureObservationV1::from_json(&recovery)
            .expect("retained raw capture validates");
        let ObservationOutcomeV1::Complete(complete) = captured.observation else {
            panic!("retained capture is complete")
        };
        let LegacyRawCaptureV1::Sqlite(mut raw_without_order) = complete.capture else {
            panic!("retained capture is SQLite")
        };
        raw_without_order.history.clear();
        let mut unavailable_histories = aep_planning_migration::sql_boundaries(
            &raw_without_order,
            &sqlite.source_snapshot.0.as_wire(),
        )
        .expect("current SQL state maps without inventing unavailable history order");
        unavailable_histories = aep_planning_migration::boundaries_with_authoritative_evidence(
            unavailable_histories,
            &LegacyRawCaptureV1::Sqlite(raw_without_order),
            sqlite.source_snapshot.0,
        )
        .expect("raw boundary");
        let unavailable_path = engineering.join("unavailable-order-state");
        let unavailable_stream =
            aep_backend_eventlog::prepare_file(&unavailable_path, "tenant-unavailable")
                .expect("owned unavailable-order provider");
        let unavailable_entry = entity_store::asynchronous::RecordedEntry::Observation(
            serde_json::from_slice(&legacy_observation).expect("legacy observation envelope"),
        );
        let unavailable_subject = unavailable_entry.subject().clone();
        let unavailable_authority = AuthorityCoordinateV1 {
            logical_scope: AuthorityValueV1::new("planning-unavailable").expect("scope"),
            tenant: AuthorityValueV1::new("tenant-unavailable").expect("tenant"),
            stream_identity: AuthorityValueV1::new(unavailable_stream.clone()).expect("stream"),
        };
        unavailable_histories =
            aep_planning_migration::bind_authoritative_evidence_with_unavailable(
                unavailable_histories,
                &unavailable_authority,
                &[aep_planning_migration::UnavailableLegacyEnvelope {
                    source_locator: "history/aep.entity/01MEM0000000000000002/unavailable"
                        .to_owned(),
                    destination_entity: unavailable_subject.entity,
                    destination_id: unavailable_subject.id,
                    evidence_kind: aep_contract::migration::HistoryKindV1::Observation,
                    original_record_id: LEGACY_COLLISION_RECORD.to_owned(),
                    exact_bytes: HexBytesV1::new(legacy_observation.clone()),
                }],
            )
            .expect("unavailable envelope becomes a closed provider boundary");
        let unavailable_context = entity_eventlog::EventlogOperationContext {
            subject: "aep-unavailable-order-test".to_owned(),
            actor: "aep-unavailable-order-test".to_owned(),
            request_id: "unavailable-order".to_owned(),
            trace_id: "unavailable-order".to_owned(),
            causation_id: None,
            causation_depth: 0,
            occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        };
        aep_backend_eventlog::provision_file(
            &unavailable_path,
            unavailable_authority.logical_scope.as_str().to_owned(),
            unavailable_authority.tenant.as_str().to_owned(),
            unavailable_stream.clone(),
            unavailable_context.clone(),
        )
        .expect("unavailable-order authority provisioned");
        let unavailable_adapter = entity_eventlog::Authority {
            logical_scope: unavailable_authority.logical_scope.as_str().to_owned(),
            tenant: unavailable_authority.tenant.as_str().to_owned(),
            stream_identity: unavailable_stream,
        };
        aep_backend_eventlog::import_file_anchors(
            &unavailable_path,
            unavailable_adapter.clone(),
            unavailable_context,
            unavailable_histories,
        )
        .expect("unavailable-order boundaries import");
        let unavailable_snapshot = aep_backend_eventlog::complete_file_snapshot(
            &unavailable_path,
            unavailable_adapter.clone(),
        )
        .expect("unavailable-order provider-complete snapshot");
        assert!(
            aep_backend_eventlog::validate_legacy_boundary_snapshot(
                &unavailable_snapshot,
                &unavailable_adapter,
            )
            .expect("unavailable-order join validates")
            .contains(LEGACY_COLLISION_RECORD)
        );
        let unavailable_coordinate = unavailable_snapshot
            .histories
            .iter()
            .find(|subject| {
                subject.history.subject.entity == "aep.migration.LegacyRecordCoordinate"
            })
            .expect("unavailable coordinate is provider-complete");
        assert_eq!(
            unavailable_coordinate.terminal.fields.get("order"),
            Some(&serde_json::Value::String("unavailable".to_owned()))
        );
        assert_eq!(
            unavailable_coordinate.terminal.fields.get("ordinal"),
            Some(&serde_json::json!({"kind":"missing"}))
        );
        fn collision_command() -> CommandEnvelope<Command> {
            CommandEnvelope::new(
                "legacy-observation-one"
                    .parse()
                    .expect("legacy record id is also a valid command id"),
                "aep.entity.create/v1",
                Command::CreateEntity(CreateEntity {
                    entity_type: EntityType::parse("aep.story/v1").expect("story type"),
                    locator: EntityLocator::parse("ep://planning/store/story/collision")
                        .expect("fresh locator"),
                    data: aep_domain::node::Node::Map(
                        [("status".to_owned(), aep_domain::node::Node::from("draft"))]
                            .into_iter()
                            .collect(),
                    ),
                }),
                CommandContext::new(
                    "request-legacy-collision".parse().expect("request id"),
                    "key-legacy-collision".parse().expect("idempotency key"),
                    ActorRef::parse("human:migration-fixture").expect("actor"),
                    "legacy-collision-flow".parse().expect("correlation"),
                    Timestamp::from_epoch_millis(1_700_000_003_000),
                ),
            )
        }
        let unavailable_store = aep_backend_eventlog::open(
            unavailable_path,
            unavailable_adapter.logical_scope,
            unavailable_adapter.tenant,
            unavailable_adapter.stream_identity,
        )
        .expect("ordinary backend opens the unavailable-order authority");
        let unavailable_collision = block_on(unavailable_store.execute(collision_command()))
            .expect_err("unavailable-order roster blocks original record-id reuse");
        assert!(
            unavailable_collision
                .to_string()
                .contains(LEGACY_COLLISION_RECORD),
            "{unavailable_collision}"
        );
        let evidence_blob = authority_snapshot
            .histories
            .iter()
            .find_map(|subject| {
                (subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob")
                    .then(|| match &subject.history.origin {
                        entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                            anchor.instance.fields.clone()
                        }
                        entity_store::asynchronous::HistoryOrigin::Genesis => unreachable!(),
                    })
            })
            .expect("exact legacy evidence blob remains in provider-complete history");
        assert_eq!(
            evidence_blob.get("original_record_id"),
            Some(&serde_json::json!({
                "kind": "present",
                "value": LEGACY_COLLISION_RECORD
            }))
        );
        assert_eq!(
            evidence_blob.get("exact_bytes"),
            Some(
                &serde_json::to_value(HexBytesV1::new(legacy_observation))
                    .expect("expected exact bytes serialise")
            )
        );
        let selected = aep_backend_eventlog::open(
            engineering.join("state"),
            authority.logical_scope.clone(),
            authority.tenant.clone(),
            authority.stream_identity.clone(),
        )
        .expect("ordinary Eventlog backend reopens");
        let collision = block_on(selected.execute(collision_command()))
            .expect_err("an ordinary command cannot reuse a reserved legacy record id");
        assert!(
            collision.to_string().contains(LEGACY_COLLISION_RECORD),
            "the refusal names the exact colliding legacy identity: {collision}"
        );
        fs::remove_file(
            migration_root(&engineering, &migration_id)
                .expect("migration root")
                .join("recovery/raw-capture.json"),
        )
        .expect("remove external recovery copy after Complete");
        assert!(
            verify(&common).success(),
            "history and original-id evidence remain provider-complete without recovery copy"
        );
        let projected = fs::read_to_string(planning.join("story/one.md"))
            .expect("SQLite migration publishes the ordinary Markdown projection");
        assert!(projected.contains("title: One"));
        assert_eq!(applied.receipt.source_snapshot, sqlite.source_snapshot);

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn postgres_and_hybrid_physical_sources_migrate_with_complete_history() {
        let Ok(url) = std::env::var("ENTITY_POSTGRES_URL") else {
            eprintln!("skipped: ENTITY_POSTGRES_URL unset, so no owned PostgreSQL fixture");
            return;
        };
        if url.trim().is_empty() {
            eprintln!("skipped: ENTITY_POSTGRES_URL empty, so no owned PostgreSQL fixture");
            return;
        }
        let fixture = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let project = std::env::temp_dir().join(format!(
            "aep-cli-postgres-source-{}-{}",
            std::process::id(),
            fixture
        ));
        let schema = format!("aep_migration_{}_{}", std::process::id(), fixture);
        let isolated_url = isolated_postgres_url(&url, &schema);
        let engineering = project.join(".engineering");
        let planning = engineering.join("planning");
        let selector = engineering.join("project.yaml");
        fs::create_dir_all(project.join("protocols")).expect("protocol source");
        write_one_story(&planning, "One");
        seed_postgres_from_markdown(&planning, &isolated_url);
        let legacy_observation = append_postgres_observation(&isolated_url);
        let common = CommonArgs {
            project: Some(selector.clone()),
            format: StoreOutputFormat::Json,
        };
        let destination = DestinationRequestV2::ProviderAssigned {
            logical_scope: AuthorityValueV1::new("planning-postgres-test").expect("scope"),
            tenant: AuthorityValueV1::new("tenant-postgres-test").expect("tenant"),
        };
        let base = "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n";
        let quoted_url = serde_json::to_string(&isolated_url).expect("PostgreSQL URL quotes");

        fs::write(
            &selector,
            format!("{base}store:\n  postgres: {quoted_url}\n"),
        )
        .expect("PostgreSQL selector");
        let postgres = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(postgres) = postgres.outcome else {
            panic!("actual PostgreSQL source is admitted")
        };
        assert_eq!(postgres.inventory.subjects, 1);

        fs::write(
            &selector,
            format!(
                "{base}store:\n  hybrid:\n    authority: local\n    read: local-first\n    on_unreachable: refuse\n    on_divergence: record\n    local: markdown\n    replica:\n      postgres: {quoted_url}\n"
            ),
        )
        .expect("hybrid PostgreSQL selector");
        let hybrid = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(hybrid) = hybrid.outcome else {
            panic!("matching physical PostgreSQL hybrid source is admitted")
        };
        assert_eq!(hybrid.inventory.subjects, 1);

        write_one_story(&planning, "Changed only locally");
        let divergent = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Refused(divergent) = divergent.outcome else {
            panic!("divergent physical PostgreSQL hybrid source is refused")
        };
        assert_eq!(
            divergent.refusals[0].code,
            CommandRefusalCodeV1::DivergentHybrid
        );
        write_one_story(&planning, "One");

        fs::write(
            &selector,
            format!("{base}store:\n  postgres: {quoted_url}\n"),
        )
        .expect("restore PostgreSQL selector");
        let migration_id = MigrationIdV1::new("postgres-provider-assigned").expect("migration");
        let applied = apply_with_control(
            &common,
            destination,
            postgres.source_snapshot.0,
            migration_id.clone(),
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Complete(applied) = applied.outcome else {
            panic!("physical PostgreSQL source applies under disposable writer control")
        };
        assert!(
            verify(&common).success(),
            "PostgreSQL rows reopen through the ordinary Eventlog backend"
        );
        let authority = entity_eventlog::Authority {
            logical_scope: applied.receipt.authority.logical_scope.as_str().to_owned(),
            tenant: applied.receipt.authority.tenant.as_str().to_owned(),
            stream_identity: applied
                .receipt
                .authority
                .stream_identity
                .as_str()
                .to_owned(),
        };
        let authority_snapshot = aep_backend_eventlog::complete_file_snapshot(
            &engineering.join("state"),
            authority.clone(),
        )
        .expect("complete imported PostgreSQL authority");
        let reserved = aep_backend_eventlog::validate_legacy_boundary_snapshot(
            &authority_snapshot,
            &authority,
        )
        .expect("the PostgreSQL provider-complete boundary graph validates");
        assert!(reserved.contains(LEGACY_COLLISION_RECORD));
        let retained = authority_snapshot
            .histories
            .iter()
            .filter(|subject| {
                subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob"
            })
            .any(|subject| {
                subject.terminal.fields.get("exact_bytes")
                    == Some(
                        &serde_json::to_value(HexBytesV1::new(legacy_observation.clone()))
                            .expect("exact PostgreSQL bytes serialise"),
                    )
            });
        assert!(retained, "exact PostgreSQL history bytes remain provider-complete");
        fs::remove_file(
            migration_root(&engineering, &migration_id)
                .expect("migration root")
                .join("recovery/raw-capture.json"),
        )
        .expect("remove external PostgreSQL recovery copy after Complete");
        assert!(
            verify(&common).success(),
            "PostgreSQL history and original identity survive without recovery copy"
        );
        let projected = fs::read_to_string(planning.join("story/one.md"))
            .expect("PostgreSQL migration publishes the ordinary Markdown projection");
        assert!(projected.contains("title: One"));
        assert_eq!(applied.receipt.source_snapshot, postgres.source_snapshot);

        drop_postgres_schema(&url, &schema);
        let _ = fs::remove_dir_all(project);
    }
}

fn selector_refusal(common: &CommonArgs, code: CommandRefusalCodeV1) -> CommandRefusalV1 {
    let path = common.project.clone().unwrap_or_else(|| PathBuf::from("."));
    CommandRefusalV1 {
        code,
        at: DiagnosticCoordinateV1::Selector(SelectorDiagnosticV1 {
            path: host_path(&path),
        }),
    }
}

#[cfg(unix)]
fn host_path(path: &std::path::Path) -> aep_contract::migration::HostPathV1 {
    use std::os::unix::ffi::OsStrExt as _;
    aep_contract::migration::HostPathV1::Unix(aep_contract::migration::HexBytesV1::new(
        path.as_os_str().as_bytes().to_vec(),
    ))
}

#[cfg(windows)]
fn host_path(path: &std::path::Path) -> aep_contract::migration::HostPathV1 {
    use std::os::windows::ffi::OsStrExt as _;
    aep_contract::migration::HostPathV1::Windows(path.as_os_str().encode_wide().collect())
}

fn emit(value: &impl Serialize, format: StoreOutputFormat) -> Result<()> {
    let bytes = serde_json::to_vec(value).context("rendering store result")?;
    match format {
        StoreOutputFormat::Json => {
            let text = String::from_utf8(bytes).expect("JSON is UTF-8");
            outln!("{text}");
        }
        StoreOutputFormat::Text => {
            let node = OrderedNodeSeed
                .deserialize(&mut serde_json::Deserializer::from_slice(&bytes))
                .context("flattening store result")?;
            let mut rendered = String::new();
            flatten("", &node, &mut rendered);
            out!("{rendered}");
        }
    }
    Ok(())
}

pub(crate) fn emit_mutation(
    value: &aep_contract::migration::PlanningMutationEnvelopeV1,
    format: crate::Format,
) -> Result<()> {
    match format {
        crate::Format::Text => emit(value, StoreOutputFormat::Text),
        crate::Format::Json => emit(value, StoreOutputFormat::Json),
        crate::Format::Yaml => {
            out!("{}", serde_yaml::to_string(value).context("rendering mutation result")?);
            Ok(())
        }
    }
}

#[derive(Debug)]
enum OrderedNode {
    Scalar(String),
    Sequence(Vec<Self>),
    Object(Vec<(String, Self)>),
}

struct OrderedNodeSeed;

impl<'de> DeserializeSeed<'de> for OrderedNodeSeed {
    type Value = OrderedNode;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(OrderedNodeVisitor)
    }
}

struct OrderedNodeVisitor;

impl<'de> Visitor<'de> for OrderedNodeVisitor {
    type Value = OrderedNode;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON result")
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(OrderedNode::Scalar(value.to_string()))
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OrderedNode::Scalar(value.to_string()))
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OrderedNode::Scalar(value.to_string()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        serde_json::to_string(&value)
            .map(OrderedNode::Scalar)
            .map_err(E::custom)
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        serde_json::to_string(value)
            .map(OrderedNode::Scalar)
            .map_err(E::custom)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(OrderedNode::Scalar("null".to_owned()))
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.visit_none()
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(OrderedNodeSeed)? {
            values.push(value);
        }
        Ok(OrderedNode::Sequence(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            values.push((key, map.next_value_seed(OrderedNodeSeed)?));
        }
        Ok(OrderedNode::Object(values))
    }
}

fn flatten(path: &str, node: &OrderedNode, output: &mut String) {
    match node {
        OrderedNode::Scalar(value) => {
            let _ = writeln!(output, "{path}\t{value}");
        }
        OrderedNode::Sequence(values) => {
            let count = if path.is_empty() { "count".to_owned() } else { format!("{path}.count") };
            let _ = writeln!(output, "{count}\t{}", values.len());
            for (index, value) in values.iter().enumerate() {
                flatten(&format!("{path}[{index}]"), value, output);
            }
        }
        OrderedNode::Object(values) => {
            for (name, value) in values {
                let child = if path.is_empty() { name.clone() } else { format!("{path}.{name}") };
                flatten(&child, value, output);
            }
        }
    }
}
