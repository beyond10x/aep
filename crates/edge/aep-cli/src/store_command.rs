//! `aep plan store` — explicit planning-authority inspection and migration.

#![allow(missing_docs)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use aep_backend_markdown::{MarkdownStore, StoreReport};
use aep_contract::migration::{
    ApplyCompleteV1, ApplyFormatV1, ApplyOutcomeV1, ApplyRefusedV1, ApplyResultV1,
    AuthorityCoordinateV1, AuthorityObservationV1, AuthoritySnapshotIdV1, AuthorityValueV1,
    BackendKindV1, CommandRefusalCodeV1, CommandRefusalV1, DestinationRequestV2,
    DestinationRequirementsV2, DiagnosticCoordinateV1, DigestV1, DryRunAdmittedV2, DryRunFormatV2,
    DryRunOutcomeV2, DryRunRefusedV1, DryRunResultV2, EventlogSourceCoordinateV1, HistorySummaryV1,
    HybridPolicyWordsV1, HybridSideRootCoordinateV1, HybridSideV1, HybridSourceCoordinateV1,
    InspectionFormatV1, InspectionObservedV1, InspectionOutcomeV1, InspectionReadinessV1,
    InspectionResultV1, IntentDigestV1, InventoryCountsV1, LegacyRawCaptureV1,
    MarkdownPathCoordinateV1, MarkdownRawV1, MarkdownRootCoordinateV1, MappingFormatV1,
    MappingIdentityV1, MarkdownSourceCoordinateV1, MigrationIdV1, MigrationIntentFormatV2,
    MigrationIntentV2, MigrationReceiptV1, ObservationOutcomeV1, PhysicalCoordinateV1,
    PostgresEndpointRootCoordinateV1, PostgresReplicaCoordinateV1, PresenceV1, ProjectVersionV1,
    ProjectionDriftV1, ProjectionInventoryDigestV1, ProjectionObservationV1, ProjectionWatermarkV1,
    RebuildFormatV1, RebuildOutcomeV1, RebuildRefusedV1, RebuildResultV1, RebuildUncertainV1,
    RebuiltV1, RefusedV1, RootCoordinateV1, SelectionV1, SelectorDiagnosticV1, SourceCoordinateV1,
    SourceDiagnosticV1, SourceSnapshotIdV1, SqlReplicaCoordinateV1, SqliteDatabaseRootCoordinateV1,
    SqliteReplicaCoordinateV1, SqliteSourceCoordinateV1, VerificationFormatV1,
    VerificationMismatchV1, VerificationOutcomeV1, VerificationRefusedV1, VerificationResultV1,
    VerifiedV1,
};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

#[cfg(target_os = "linux")]
mod writer_control;
#[cfg(not(target_os = "linux"))]
#[path = "store_command/writer_control_unsupported.rs"]
mod writer_control;

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
    #[arg(
        long,
        conflicts_with = "authority_identity",
        required_unless_present = "authority_identity"
    )]
    authority_new: bool,
    /// Recover an exact identity only from an already owned migration stage.
    #[arg(
        long,
        conflicts_with = "authority_new",
        required_unless_present = "authority_new"
    )]
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
    /// Hold observed writer stop and operator no-restart custody in this foreground process.
    WriterControl {
        #[command(subcommand)]
        command: writer_control::WriterControlCommand,
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
    if let StoreCommand::WriterControl { command } = command {
        return writer_control::hold(command);
    }
    let common = match &command {
        StoreCommand::Migrate {
            command: MigrateCommand::Apply { common, .. },
        }
        | StoreCommand::Rebuild { common, .. } => Some(common.clone()),
        _ => None,
    };
    run_with_control(command, &writer_control::PublicWriterControl::new(common))
}

pub(crate) fn run_with_control<C>(command: StoreCommand, control: &C) -> Result<ExitCode>
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
        StoreCommand::WriterControl { .. } => anyhow::bail!("writer control is a foreground command"),
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
        Err(_) => {
            return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnreadable)
        }
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
        Err(_) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::IntentConflict),
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
        Err(aep_planning_migration::AcquisitionError::RefusedObservation(observation)) => {
            return apply_capture_refusal(common, migration_id, &observation);
        }
        Err(_) => {
            return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnreadable)
        }
    };
    let ObservationOutcomeV1::Complete(complete) = &captured.observation else {
        if matches!(captured.observation, ObservationOutcomeV1::Refused(_)) {
            return apply_capture_refusal(common, migration_id, &captured);
        }
        return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnstable);
    };
    if complete.raw_snapshot_id != requested_snapshot {
        return apply_refusal(common, migration_id, CommandRefusalCodeV1::SnapshotChanged);
    }
    // Read once, here, and handed to both the mapper and the receipt: the crossing count the
    // receipt publishes has to come from the same declaration the mapper mapped under, or the two
    // halves of one cutover answer from different files.
    let membership = match resolved.membership() {
        Ok(value) => value,
        Err(_) => return apply_refusal_at(common, migration_id, *workspace_refusal()),
    };
    let histories = match mapped_histories_for_source(
        &resolved.selection.source,
        &membership,
        &complete.capture,
        complete.raw_snapshot_id,
    ) {
        Ok(value) => value,
        Err(refusal) => return apply_refusal_at(common, migration_id, *refusal),
    };
    if captured.source != PresenceV1::Present(intent.source_coordinate.clone()) {
        return apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnreadable);
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
            Err(_) => {
                return apply_refusal(common, migration_id, CommandRefusalCodeV1::IntentConflict)
            }
        },
        source_capture_digest: complete.transcript_digest,
        histories,
    };
    execute_apply(
        common,
        &resolved,
        &membership,
        migration_id,
        &inputs,
        control,
        guard,
    )
}

#[allow(clippy::manual_let_else)]
fn execute_apply<C: aep_planning_migration::WriterControl>(
    common: &CommonArgs,
    resolved: &Resolved,
    membership: &aep_domain::workspace::Membership,
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
        Err(aep_planning_migration::ApplyError::Writer(_)) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::WriterExclusionUnavailable,
            )
        }
        Err(aep_planning_migration::ApplyError::IntentConflict) => {
            return apply_refusal(common, migration_id, CommandRefusalCodeV1::IntentConflict)
        }
        Err(aep_planning_migration::ApplyError::ForeignStage) => {
            return apply_refusal(common, migration_id, CommandRefusalCodeV1::ForeignStage)
        }
        Err(aep_planning_migration::ApplyError::DestinationConflict) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::DestinationConflict,
            )
        }
        Err(aep_planning_migration::ApplyError::AuthorityIdentityMismatch) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::AuthorityIdentityMismatch,
            )
        }
        Err(aep_planning_migration::ApplyError::SelectorChanged) => {
            return apply_refusal(common, migration_id, CommandRefusalCodeV1::SelectorChanged)
        }
        Err(
            aep_planning_migration::ApplyError::ReceiptConflict
            | aep_planning_migration::ApplyError::EvidenceConflict,
        ) => return apply_refusal(common, migration_id, CommandRefusalCodeV1::ReceiptConflict),
        Err(aep_planning_migration::ApplyError::VerificationMismatch) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::VerificationMismatch,
            )
        }
        Err(_) => {
            return apply_refusal(common, migration_id, CommandRefusalCodeV1::PublishUncertain)
        }
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
        Err(_) => {
            return apply_refusal(
                common,
                migration_id,
                CommandRefusalCodeV1::AuthorityIdentityMismatch,
            )
        }
    };
    let (snapshot_id, capture_digest) =
        match aep_planning_migration::authority_snapshot_identity(&receipt.authority, &snapshot) {
            Ok(value) => value,
            Err(_) => {
                return apply_refusal(
                    common,
                    migration_id,
                    CommandRefusalCodeV1::VerificationMismatch,
                )
            }
        };
    let facts = inventory_from_histories(&snapshot.histories, membership);
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

/// The three paths a resumed migration writes to, derived once and checked against the intent.
struct ResumePaths {
    staging: PathBuf,
    destination: PathBuf,
    projection: PathBuf,
}

/// The recorded intent, when it is the one *this* invocation is resuming.
///
/// Every check here asks the same question — does the intent on disk still describe this
/// invocation — and none of them is about the store's contents. Read together so the resume body
/// stays about what it does rather than about what it refuses.
fn resumable_intent(
    resolved: &Resolved,
    destination_request: &DestinationRequestV2,
    requested_snapshot: DigestV1,
    migration_id: &MigrationIdV1,
    migration_root: &Path,
) -> std::result::Result<(MigrationIntentV2, ResumePaths), CommandRefusalCodeV1> {
    let Some(intent) = fs::read(migration_root.join("intent.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<MigrationIntentV2>(&bytes).ok())
    else {
        return Err(CommandRefusalCodeV1::IntentConflict);
    };
    if &intent.migration_id != migration_id
        || intent.source_snapshot != SourceSnapshotIdV1(requested_snapshot)
        || &intent.destination_request != destination_request
        || intent.mapping != mapping_identity()
        || intent.selector_version != ProjectVersionV1::V2
        || aep_planning_migration::intent_digest_v2(&intent).ok() != Some(intent.intent_digest)
    {
        return Err(CommandRefusalCodeV1::IntentConflict);
    }

    let paths = ResumePaths {
        staging: migration_root.join("stage/authority"),
        destination: resolved.engineering.join("state"),
        projection: resolved.engineering.join("planning"),
    };
    if intent.migration_root != host_path(migration_root)
        || intent.staging_path != host_path(&paths.staging)
        || intent.destination_path != host_path(&paths.destination)
        || intent.projection_path != host_path(&paths.projection)
    {
        return Err(CommandRefusalCodeV1::IntentConflict);
    }

    match &resolved.plan {
        crate::planning::Plan::Eventlog {
            authority_root,
            projection_root,
            ..
        } if authority_root == &paths.destination && projection_root == &paths.projection => {}
        crate::planning::Plan::Eventlog { .. } => {
            return Err(CommandRefusalCodeV1::AuthorityIdentityMismatch);
        }
        _ if resolved.selection.selector_digest == intent.selector_digest
            && resolved.selection.config_digest == intent.config_digest
            && resolved.selection.source == intent.source_coordinate => {}
        _ => return Err(CommandRefusalCodeV1::SelectorChanged),
    }
    Ok((intent, paths))
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
    let (intent, paths) = match resumable_intent(
        resolved,
        destination_request,
        requested_snapshot,
        &migration_id,
        &migration_root,
    ) {
        Ok(value) => value,
        Err(code) => return refuse(code),
    };
    let ResumePaths {
        staging: staging_path,
        destination: destination_path,
        projection: projection_path,
    } = paths;

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
    let Ok(membership) = resolved.membership() else {
        return apply_refusal_at(common, migration_id.clone(), *workspace_refusal());
    };
    let histories = match mapped_histories_for_source(
        &intent.source_coordinate,
        &membership,
        &complete.capture,
        complete.raw_snapshot_id,
    ) {
        Ok(value) => value,
        Err(refusal) => return apply_refusal_at(common, migration_id.clone(), *refusal),
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
    execute_apply(
        common,
        resolved,
        &membership,
        migration_id,
        &inputs,
        control,
        guard,
    )
}

fn compact_json_line(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn apply_refusal(
    common: &CommonArgs,
    migration_id: MigrationIdV1,
    code: CommandRefusalCodeV1,
) -> ApplyResultV1 {
    let refusal = selector_refusal(common, code);
    apply_refusal_at(common, migration_id, refusal)
}

/// The same refusal, at a coordinate the caller already knows.
///
/// Every refusal used to be attached to the selector, whatever produced it. A mapping refusal
/// reported at `--project` names the file that chose the store rather than the record that did not
/// map, and the receipt then says neither what failed nor where.
fn apply_refusal_at(
    common: &CommonArgs,
    migration_id: MigrationIdV1,
    refusal: CommandRefusalV1,
) -> ApplyResultV1 {
    let last_proved_phase = resolve(common)
        .ok()
        .and_then(|resolved| current_phase_for_migration(&resolved.engineering, &migration_id))
        .map_or(PresenceV1::Missing, PresenceV1::Present);
    ApplyResultV1 {
        format: ApplyFormatV1,
        outcome: ApplyOutcomeV1::Refused(ApplyRefusedV1 {
            migration_id,
            last_proved_phase,
            refusals: vec![refusal],
        }),
    }
}

fn apply_capture_refusal(
    common: &CommonArgs,
    migration_id: MigrationIdV1,
    observation: &aep_contract::migration::RawCaptureObservationV1,
) -> ApplyResultV1 {
    let mut result = apply_refusal(common, migration_id, CommandRefusalCodeV1::SourceUnreadable);
    if let ApplyOutcomeV1::Refused(ref mut refused) = result.outcome {
        refused.refusals = capture_refusals(observation);
    }
    result
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
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Missing,
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    // Read once, here: the rebuilt authority observation publishes the same two counts the cutover
    // did, and they are only the same two when they are computed under the same declaration.
    let membership = match resolved.membership() {
        Ok(value) => value,
        Err(_) => {
            return RebuildResultV1 {
                format: RebuildFormatV1,
                outcome: RebuildOutcomeV1::Refused(RebuildRefusedV1 {
                    requested_snapshot: requested,
                    current_snapshot: PresenceV1::Missing,
                    refusals: vec![*workspace_refusal()],
                }),
            }
        }
    };
    let crate::planning::Plan::Eventlog {
        authority_root,
        projection_root,
        authority: selected,
    } = &resolved.plan
    else {
        return rebuild_refusal(
            common,
            requested,
            PresenceV1::Missing,
            CommandRefusalCodeV1::IncompletePublication,
        );
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
        _ => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Missing,
                CommandRefusalCodeV1::AuthorityIdentityMismatch,
            )
        }
    };
    let mut guard = match control.acquire_authority(&authority, requested) {
        Ok(value) => value,
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Missing,
                CommandRefusalCodeV1::WriterExclusionUnavailable,
            )
        }
    };
    let adapter = || entity_eventlog::Authority {
        logical_scope: authority.logical_scope.as_str().to_owned(),
        tenant: authority.tenant.as_str().to_owned(),
        stream_identity: authority.stream_identity.as_str().to_owned(),
    };
    let first = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Missing,
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let (first_id, _) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &first) {
            Ok(value) => value,
            Err(_) => {
                return rebuild_refusal(
                    common,
                    requested,
                    PresenceV1::Missing,
                    CommandRefusalCodeV1::VerificationMismatch,
                )
            }
        };
    if first_id != requested {
        return rebuild_refusal(
            common,
            requested,
            PresenceV1::Present(first_id),
            CommandRefusalCodeV1::AuthoritySnapshotChanged,
        );
    }
    let publisher = aep_planning_migration::FileProjectionPublisher::new(
        authority_root.clone(),
        authority.clone(),
        projection_root.clone(),
    );
    let staged = match publisher.stage(requested) {
        Ok(value) => value,
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Present(first_id),
                CommandRefusalCodeV1::ProjectionConflict,
            )
        }
    };
    let staged_inventory = staged.inventory_digest();
    if control
        .recheck_authority(&mut guard, &authority, requested)
        .is_err()
    {
        return rebuild_refusal(
            common,
            requested,
            PresenceV1::Present(first_id),
            CommandRefusalCodeV1::WriterExclusionUnavailable,
        );
    }
    let second = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Present(first_id),
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let (second_id, _) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &second) {
            Ok(value) => value,
            Err(_) => {
                return rebuild_refusal(
                    common,
                    requested,
                    PresenceV1::Present(first_id),
                    CommandRefusalCodeV1::VerificationMismatch,
                )
            }
        };
    if second_id != requested {
        return rebuild_refusal(
            common,
            requested,
            PresenceV1::Present(second_id),
            CommandRefusalCodeV1::AuthoritySnapshotChanged,
        );
    }
    let publication = match publisher.commit(staged) {
        Ok(value) => value,
        Err(_) => {
            return RebuildResultV1 {
                format: RebuildFormatV1,
                outcome: RebuildOutcomeV1::Uncertain(RebuildUncertainV1 {
                    requested_snapshot: requested,
                    staged_inventory_digest: staged_inventory,
                    refusals: vec![selector_refusal(
                        common,
                        CommandRefusalCodeV1::PublishUncertain,
                    )],
                }),
            }
        }
    };
    let current = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Missing,
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let (current_id, capture_digest) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &current) {
            Ok(value) => value,
            Err(_) => {
                return rebuild_refusal(
                    common,
                    requested,
                    PresenceV1::Missing,
                    CommandRefusalCodeV1::VerificationMismatch,
                )
            }
        };
    let prior = match aep_planning_migration::before_projection_watermark(&current, requested) {
        Ok(value) => value,
        Err(_) => {
            return rebuild_refusal(
                common,
                requested,
                PresenceV1::Present(current_id),
                CommandRefusalCodeV1::ProjectionDrift,
            )
        }
    };
    let (prior_id, _) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &prior) {
            Ok(value) => value,
            Err(_) => {
                return rebuild_refusal(
                    common,
                    requested,
                    PresenceV1::Present(current_id),
                    CommandRefusalCodeV1::ProjectionDrift,
                )
            }
        };
    if prior_id != requested {
        return rebuild_refusal(
            common,
            requested,
            PresenceV1::Present(current_id),
            CommandRefusalCodeV1::AuthoritySnapshotChanged,
        );
    }
    let facts = inventory_from_histories(&current.histories, &membership);
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
        Err(_) => {
            return InspectionResultV1 {
                format: InspectionFormatV1,
                outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                    refusals: vec![selector_refusal(
                        common,
                        CommandRefusalCodeV1::SourceUnreadable,
                    )],
                }),
            }
        }
    };
    if matches!(resolved.plan, crate::planning::Plan::Eventlog { .. }) {
        return inspect_eventlog(common, resolved);
    }
    let membership = match resolved.membership() {
        Ok(value) => value,
        Err(_) => {
            return InspectionResultV1 {
                format: InspectionFormatV1,
                outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                    refusals: vec![*workspace_refusal()],
                }),
            }
        }
    };
    match resolved
        .inventory(&membership)
        .map(|facts| (resolved, facts))
    {
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
                refusals: vec![selector_refusal(
                    common,
                    CommandRefusalCodeV1::SourceUnreadable,
                )],
            }),
        },
    }
}

#[allow(clippy::manual_let_else, clippy::too_many_lines)]
fn inspect_eventlog(common: &CommonArgs, resolved: Resolved) -> InspectionResultV1 {
    // The declaration is beside the store whichever backend is selected, so an Eventlog `inspect`
    // reads it exactly as the Markdown one does — and refuses the same way. Answering from an
    // empty member list here is what made `workspace_crossings` a hard zero on this receipt.
    let membership = match resolved.membership() {
        Ok(value) => value,
        Err(_) => {
            return InspectionResultV1 {
                format: InspectionFormatV1,
                outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                    refusals: vec![*workspace_refusal()],
                }),
            }
        }
    };
    let crate::planning::Plan::Eventlog {
        authority_root,
        projection_root,
        authority: selected,
    } = &resolved.plan
    else {
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
        _ => {
            return InspectionResultV1 {
                format: InspectionFormatV1,
                outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                    refusals: vec![selector_refusal(
                        common,
                        CommandRefusalCodeV1::AuthorityIdentityMismatch,
                    )],
                }),
            }
        }
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
        Err(_) => {
            return InspectionResultV1 {
                format: InspectionFormatV1,
                outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                    refusals: vec![selector_refusal(
                        common,
                        CommandRefusalCodeV1::SourceUnreadable,
                    )],
                }),
            }
        }
    };
    let (snapshot_id, capture_digest) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &snapshot) {
            Ok(value) => value,
            Err(_) => {
                return InspectionResultV1 {
                    format: InspectionFormatV1,
                    outcome: InspectionOutcomeV1::Refused(RefusedV1 {
                        refusals: vec![selector_refusal(
                            common,
                            CommandRefusalCodeV1::VerificationMismatch,
                        )],
                    }),
                }
            }
        };
    let facts = inventory_from_histories(&snapshot.histories, &membership);
    let projection = projection_inventory(projection_root, &snapshot)
        .ok()
        .and_then(|(inventory_digest, watermark)| {
            let prior = aep_planning_migration::before_projection_watermark(
                &snapshot,
                watermark.authority_snapshot,
            )
            .ok()?;
            let (prior_id, _) =
                aep_planning_migration::authority_snapshot_identity(&authority, &prior).ok()?;
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
        let Ok(current_bytes) = fs::read(root.join("current.json")) else {
            continue;
        };
        if let Ok(current) =
            serde_json::from_slice::<aep_contract::migration::CurrentPhaseV2>(&current_bytes)
        {
            let Ok(binding_bytes) = fs::read(root.join("phases/02-destination-provisioned.json"))
            else {
                continue;
            };
            let Ok(binding_record) =
                serde_json::from_slice::<aep_contract::migration::PhaseRecordV2>(&binding_bytes)
            else {
                continue;
            };
            let Some(binding) =
                binding_record
                    .observations
                    .into_iter()
                    .find_map(|value| match value {
                        aep_contract::migration::MigrationPhaseObservationV2::Binding(value) => {
                            Some(value)
                        }
                        _ => None,
                    })
            else {
                continue;
            };
            if binding.authority == *authority
                && binding.intended_selector_digest == selector_digest
            {
                matches.push(current.phase);
            }
            continue;
        }
        let Ok(current) =
            serde_json::from_slice::<aep_contract::migration::CurrentPhaseV1>(&current_bytes)
        else {
            continue;
        };
        let Ok(intent_bytes) = fs::read(root.join("intent.json")) else {
            continue;
        };
        let Ok(intent) =
            serde_json::from_slice::<aep_contract::migration::MigrationIntentV1>(&intent_bytes)
        else {
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
            if aep_planning_migration::migration_receipt_digest(&receipt)? != receipt.receipt_digest
            {
                anyhow::bail!("{} has a mismatched receipt digest", path.display());
            }
            matches.push(receipt);
        }
    }
    match matches.as_slice() {
        [receipt] => Ok(receipt.clone()),
        [] => anyhow::bail!("the selected Eventlog authority has no completed migration receipt"),
        _ => {
            anyhow::bail!("more than one completed migration receipt claims the selected authority")
        }
    }
}

#[allow(clippy::manual_let_else)]
#[allow(clippy::too_many_lines)] // The result keeps every read-only capture, mapping, and destination refusal visibly ordered.
fn dry_run(common: &CommonArgs, destination_request: DestinationRequestV2) -> DryRunResultV2 {
    let resolved = match resolve(common) {
        Ok(resolved) => resolved,
        Err(_) => {
            return dry_refusal(
                common,
                PresenceV1::Missing,
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let selection = resolved.selection.clone();
    // Read once, at the edge, before anything counts or maps. A declaration that does not parse is
    // refused here at the file that is wrong rather than at whichever artifact document would then
    // have read as a dangling edge.
    let membership = match resolved.membership() {
        Ok(value) => value,
        Err(_) => {
            return dry_refusal_at(PresenceV1::Present(selection), *workspace_refusal());
        }
    };
    let facts = match resolved.inventory(&membership) {
        Ok(facts) if facts.clean => facts,
        Ok(_) => {
            return dry_refusal(
                common,
                PresenceV1::Present(selection),
                CommandRefusalCodeV1::IncompleteInventory,
            )
        }
        Err(_) => {
            return dry_refusal(
                common,
                PresenceV1::Present(selection),
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let complete = match resolved.capture() {
        Ok(aep_contract::migration::RawCaptureObservationV1 {
            observation: aep_contract::migration::ObservationOutcomeV1::Complete(complete),
            ..
        }) => complete,
        Ok(observation) if matches!(observation.observation, ObservationOutcomeV1::Refused(_)) => {
            return DryRunResultV2 {
                format: DryRunFormatV2,
                outcome: DryRunOutcomeV2::Refused(DryRunRefusedV1 {
                    selection: PresenceV1::Present(selection),
                    refusals: capture_refusals(&observation),
                }),
            };
        }
        Ok(_) => {
            return dry_refusal(
                common,
                PresenceV1::Present(selection),
                CommandRefusalCodeV1::SourceUnstable,
            )
        }
        Err(aep_planning_migration::AcquisitionError::RefusedObservation(observation)) => {
            return DryRunResultV2 {
                format: DryRunFormatV2,
                outcome: DryRunOutcomeV2::Refused(DryRunRefusedV1 {
                    selection: PresenceV1::Present(selection),
                    refusals: capture_refusals(&observation),
                }),
            };
        }
        Err(_) => {
            return dry_refusal(
                common,
                PresenceV1::Present(selection),
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    // The membership read at the top of this command, not a second read of the same file: the
    // receipt's counts and the mapping it previews have to answer from one declaration.
    if let Err(refusal) = mapped_histories_for_source(
        &resolved.selection.source,
        &membership,
        &complete.capture,
        complete.raw_snapshot_id,
    ) {
        return dry_refusal_at(PresenceV1::Present(selection), *refusal);
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
    let staging = migration_root.join(&stage_name).join("stage/authority");
    let foreign_content = migration_foreign_content(
        &resolved.selection.source,
        &staging,
        &destination,
        &projection,
    );
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
                staging_path: host_path(&staging),
                destination_path: host_path(&destination),
                projection_path: host_path(&projection),
                destination_request,
                foreign_content,
            },
        }),
    }
}

fn migration_foreign_content(
    source: &SourceCoordinateV1,
    staging: &Path,
    destination: &Path,
    projection: &Path,
) -> Vec<aep_contract::migration::HostPathV1> {
    let projection_is_source = match source {
        SourceCoordinateV1::Markdown(source) => source.root == host_path(projection),
        SourceCoordinateV1::Hybrid(source) => source.local_root == host_path(projection),
        SourceCoordinateV1::Sqlite(_)
        | SourceCoordinateV1::Postgres(_)
        | SourceCoordinateV1::Eventlog(_) => false,
    };
    let mut paths = vec![staging, destination];
    if !projection_is_source {
        paths.push(projection);
    }
    aep_planning_migration::foreign_destination_paths(&paths)
        .iter()
        .map(|path| host_path(path))
        .collect()
}

fn capture_refusals(
    observation: &aep_contract::migration::RawCaptureObservationV1,
) -> Vec<CommandRefusalV1> {
    use aep_contract::migration::{CaptureRefusalCodeV1, PhaseResultV1};
    let fallback = || {
        vec![CommandRefusalV1 {
            code: CommandRefusalCodeV1::SourceUnreadable,
            at: DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
                coordinate: PhysicalCoordinateV1::Root(RootCoordinateV1::Observation),
            }),
        }]
    };
    if observation.validate().is_err() {
        return fallback();
    }
    let ObservationOutcomeV1::Refused(refused) = &observation.observation else {
        return fallback();
    };
    let failures = refused
        .preflight_refusals
        .iter()
        .chain(refused.phases.iter().flat_map(|phase| match &phase.result {
            PhaseResultV1::Refused(result) => result.refusals.as_slice(),
            _ => &[],
        }));
    failures
        .map(|failure| CommandRefusalV1 {
            code: match failure.code {
                CaptureRefusalCodeV1::UnsupportedSchema => CommandRefusalCodeV1::UnsupportedSchema,
                CaptureRefusalCodeV1::UnknownPhysicalObject
                | CaptureRefusalCodeV1::ForeignMarkdownNode => CommandRefusalCodeV1::ForeignContent,
                CaptureRefusalCodeV1::PendingBatchPresent => {
                    CommandRefusalCodeV1::PendingLegacyIntent
                }
                CaptureRefusalCodeV1::HybridContradiction => CommandRefusalCodeV1::DivergentHybrid,
                _ => CommandRefusalCodeV1::SourceUnreadable,
            },
            at: DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
                coordinate: failure.at.clone(),
            }),
        })
        .collect()
}

fn dry_refusal(
    common: &CommonArgs,
    selection: PresenceV1<SelectionV1>,
    code: CommandRefusalCodeV1,
) -> DryRunResultV2 {
    let refusal = selector_refusal(common, code);
    dry_refusal_at(selection, refusal)
}

/// The same refusal, at a coordinate the caller already knows. See [`apply_refusal_at`].
fn dry_refusal_at(
    selection: PresenceV1<SelectionV1>,
    refusal: CommandRefusalV1,
) -> DryRunResultV2 {
    DryRunResultV2 {
        format: DryRunFormatV2,
        outcome: DryRunOutcomeV2::Refused(DryRunRefusedV1 {
            selection,
            refusals: vec![refusal],
        }),
    }
}

// Verification reports a distinct refusal coordinate for every failed read or comparison; the
// explicit matches preserve that closed result mapping.
#[allow(clippy::manual_let_else)]
#[allow(clippy::too_many_lines)]
fn verify(common: &CommonArgs) -> VerificationResultV1 {
    let resolved = match resolve(common) {
        Ok(value) => value,
        Err(_) => {
            return verification_refusal(
                common,
                PresenceV1::Missing,
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    // Read once, here, for the same reason the other three receipts read it: the two counts this
    // verb publishes are the two the cutover published, and they are only the same two when they
    // are computed under the same declaration.
    let membership = match resolved.membership() {
        Ok(value) => value,
        Err(_) => {
            return verification_refusal_at(
                PresenceV1::Present(resolved.selection),
                *workspace_refusal(),
            )
        }
    };
    let crate::planning::Plan::Eventlog {
        authority_root,
        projection_root,
        authority: selected,
    } = &resolved.plan
    else {
        return verification_refusal(
            common,
            PresenceV1::Present(resolved.selection),
            CommandRefusalCodeV1::IncompletePublication,
        );
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
        _ => {
            return verification_refusal(
                common,
                PresenceV1::Present(resolved.selection),
                CommandRefusalCodeV1::AuthorityIdentityMismatch,
            )
        }
    };
    let adapter = || entity_eventlog::Authority {
        logical_scope: authority.logical_scope.as_str().to_owned(),
        tenant: authority.tenant.as_str().to_owned(),
        stream_identity: authority.stream_identity.as_str().to_owned(),
    };
    let first = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => {
            return verification_refusal(
                common,
                PresenceV1::Present(resolved.selection),
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let (current_id, capture_digest) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &first) {
            Ok(value) => value,
            Err(_) => {
                return verification_refusal(
                    common,
                    PresenceV1::Present(resolved.selection),
                    CommandRefusalCodeV1::VerificationMismatch,
                )
            }
        };
    let (inventory_digest, watermark) = match projection_inventory(projection_root, &first) {
        Ok(value) => value,
        Err(_) => {
            return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift)
        }
    };
    if watermark.authority != authority
        || watermark.projection_inventory_digest != inventory_digest
        || watermark.watermark_digest
            != aep_planning_migration::projection_watermark_digest(
                watermark.authority_snapshot,
                watermark.projection_inventory_digest,
            )
    {
        return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift);
    }
    let prior = match aep_planning_migration::before_projection_watermark(
        &first,
        watermark.authority_snapshot,
    ) {
        Ok(value) => value,
        Err(_) => {
            return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ProjectionDrift)
        }
    };
    let (prior_id, _) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &prior) {
            Ok(value) => value,
            Err(_) => {
                return verification_mismatch(
                    resolved.selection,
                    CommandRefusalCodeV1::ProjectionDrift,
                )
            }
        };
    let second = match aep_backend_eventlog::complete_file_snapshot(authority_root, adapter()) {
        Ok(value) => value,
        Err(_) => {
            return verification_refusal(
                common,
                PresenceV1::Present(resolved.selection),
                CommandRefusalCodeV1::SourceUnreadable,
            )
        }
    };
    let (second_id, _) =
        match aep_planning_migration::authority_snapshot_identity(&authority, &second) {
            Ok(value) => value,
            Err(_) => {
                return verification_mismatch(
                    resolved.selection,
                    CommandRefusalCodeV1::AuthoritySnapshotChanged,
                )
            }
        };
    if current_id != second_id || prior_id != watermark.authority_snapshot {
        return verification_mismatch(
            resolved.selection,
            CommandRefusalCodeV1::AuthoritySnapshotChanged,
        );
    }
    let receipt = match selected_migration_receipt(
        &resolved.engineering,
        &authority,
        resolved.selection.selector_digest,
    ) {
        Ok(receipt) => receipt,
        Err(_) => {
            return verification_mismatch(resolved.selection, CommandRefusalCodeV1::ReceiptConflict)
        }
    };
    let facts = inventory_from_histories(&first.histories, &membership);
    let authority_observation = AuthorityObservationV1 {
        authority: authority.clone(),
        snapshot_id: current_id,
        inventory: facts.inventory,
        history: facts.history,
        capture_digest,
    };
    let projection = ProjectionObservationV1 {
        root: host_path(projection_root),
        authority_snapshot: watermark.authority_snapshot,
        inventory_digest,
        watermark_digest: watermark.watermark_digest,
        drift: ProjectionDriftV1::Current,
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
    VerificationResultV1 {
        format: VerificationFormatV1,
        outcome: VerificationOutcomeV1::Refused(VerificationRefusedV1 {
            selection,
            refusals: vec![selector_refusal(common, code)],
        }),
    }
}

/// A verification refusal at an exact coordinate rather than at the selector.
///
/// The sibling of [`verification_refusal`] for the one refusal whose coordinate is a config file
/// and not the selector: an unreadable `workspace.yaml`. The selector is correct in that case, and
/// naming it is the reading this unit exists to remove.
fn verification_refusal_at(
    selection: PresenceV1<SelectionV1>,
    refusal: CommandRefusalV1,
) -> VerificationResultV1 {
    VerificationResultV1 {
        format: VerificationFormatV1,
        outcome: VerificationOutcomeV1::Refused(VerificationRefusedV1 {
            selection,
            refusals: vec![refusal],
        }),
    }
}

fn verification_mismatch(
    selection: SelectionV1,
    code: CommandRefusalCodeV1,
) -> VerificationResultV1 {
    VerificationResultV1 {
        format: VerificationFormatV1,
        outcome: VerificationOutcomeV1::Mismatch(VerificationMismatchV1 {
            selection,
            authority: PresenceV1::Missing,
            projection: PresenceV1::Missing,
            refusals: vec![CommandRefusalV1 {
                code,
                at: DiagnosticCoordinateV1::Output(aep_contract::migration::OutputDiagnosticV1 {
                    stream: aep_contract::migration::OutputStreamV1::Stdout,
                }),
            }],
        }),
    }
}

fn projection_inventory(
    root: &Path,
    snapshot: &entity_store::asynchronous::CompleteStoreSnapshot,
) -> Result<(ProjectionInventoryDigestV1, ProjectionWatermarkV1)> {
    let mut owned = aep_planning_migration::projection_owned_inventory(root, snapshot)
        .map_err(|_| anyhow::anyhow!("projection ownership is absent or disagrees"))?;
    owned.sort_by(|left, right| left.0.cmp(&right.0));
    let inventory = aep_planning_migration::projection_inventory_digest(&owned)?;
    let mut candidates = snapshot
        .histories
        .iter()
        .filter_map(|subject| {
            if subject.history.subject.entity != aep_backend_eventlog::PROJECTION_METADATA_AS
                || subject.history.records.len() != 1
            {
                return None;
            }
            let value = subject.terminal.fields.get("document")?.clone();
            let watermark = serde_json::from_value::<ProjectionWatermarkV1>(value).ok()?;
            Some((subject.history.records[0].receipt.position.store, watermark))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(position, _)| *position);
    let watermark = candidates
        .into_iter()
        .rev()
        .find_map(|(_, watermark)| {
            (watermark.projection_inventory_digest == inventory).then_some(watermark)
        })
        .context("no authority watermark covers the projection")?;
    Ok((inventory, watermark))
}

/// Whether the projection under `projection_root` is the one the authority under
/// `authority_root` last published.
///
/// Decided from the facts `verify` reads before it answers `projection_drift`: the owned inventory
/// under the projection's ownership marker digests to a watermark the authority recorded; that
/// watermark names this authority and covers its own inventory digest; and the snapshot it names
/// is one the authority's own history reaches. `Err` says which of those failed, as prose rather
/// than a [`CommandRefusalCodeV1`], because the caller is `validate` — it accumulates findings and
/// refuses nothing.
pub(crate) fn projection_current(
    authority_root: &Path,
    projection_root: &Path,
    selected: &aep_domain::project::PlanningAuthority,
) -> Result<()> {
    let authority = AuthorityCoordinateV1 {
        logical_scope: AuthorityValueV1::new(&selected.logical_scope)
            .map_err(|error| anyhow::anyhow!(error))?,
        tenant: AuthorityValueV1::new(&selected.tenant).map_err(|error| anyhow::anyhow!(error))?,
        stream_identity: AuthorityValueV1::new(&selected.stream_identity)
            .map_err(|error| anyhow::anyhow!(error))?,
    };
    let snapshot = aep_backend_eventlog::complete_file_snapshot(
        authority_root,
        entity_eventlog::Authority {
            logical_scope: selected.logical_scope.clone(),
            tenant: selected.tenant.clone(),
            stream_identity: selected.stream_identity.clone(),
        },
    )
    .map_err(|error| anyhow::anyhow!("the authority could not be read whole: {error}"))?;
    let (_, watermark) = projection_inventory(projection_root, &snapshot)?;
    if watermark.authority != authority {
        anyhow::bail!("the projection's watermark names another authority");
    }
    if watermark.watermark_digest
        != aep_planning_migration::projection_watermark_digest(
            watermark.authority_snapshot,
            watermark.projection_inventory_digest,
        )
    {
        anyhow::bail!("the projection's watermark does not cover its own inventory digest");
    }
    let prior =
        aep_planning_migration::before_projection_watermark(&snapshot, watermark.authority_snapshot)
            .map_err(|_| {
                anyhow::anyhow!("the authority reaches no snapshot the projection's watermark names")
            })?;
    let (prior_id, _) = aep_planning_migration::authority_snapshot_identity(&authority, &prior)
        .map_err(|_| {
            anyhow::anyhow!("the snapshot the projection's watermark names has no identity")
        })?;
    if prior_id != watermark.authority_snapshot {
        anyhow::bail!("the snapshot the projection's watermark names is not the authority's");
    }
    Ok(())
}

fn destination_request(args: &AuthorityArgs) -> Result<DestinationRequestV2> {
    let logical_scope =
        AuthorityValueV1::new(&args.authority_scope).context("invalid --authority-scope")?;
    let tenant =
        AuthorityValueV1::new(&args.authority_tenant).context("invalid --authority-tenant")?;
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
        _ => anyhow::bail!("choose exactly one of --authority-new or --authority-identity"),
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

    /// Where this store stands in the workspace declared beside it.
    ///
    /// The same value `planning.rs` hands `graph_in_workspace` for `artifact validate`, `graph`,
    /// `board` and every other ordinary read — from the same reader, not a second one. It is read
    /// here, at the edge, and handed to the mapper as data: the declaration lives beside the store
    /// rather than inside it, so it is not in the capture, and the mapper reopens nothing.
    ///
    /// # Errors
    ///
    /// A `workspace.yaml` that exists and does not parse. Refused here rather than read as *this
    /// store declares no members*: that reading turns every crossing into a dangling edge and
    /// produces a receipt naming an artifact document that is correct, which is byte-identical to
    /// the receipt a store with no declaration gets. Unknown differs from false.
    fn membership(&self) -> Result<aep_domain::workspace::Membership> {
        crate::planning::declared_membership(self.engineering.parent().unwrap_or(&self.engineering))
    }

    fn capture(
        &self,
    ) -> std::result::Result<
        aep_contract::migration::RawCaptureObservationV1,
        aep_planning_migration::AcquisitionError,
    > {
        match &self.plan {
            crate::planning::Plan::Markdown { root } => aep_planning_migration::capture_markdown(
                root,
                host_path(root),
                self.selector_binding(),
            ),
            crate::planning::Plan::Sqlite { path } => {
                aep_planning_migration::capture_sqlite(path, self.selector_binding())
            }
            crate::planning::Plan::Postgres { url } => {
                aep_planning_migration::capture_postgres(url, self.selector_binding())
            }
            crate::planning::Plan::Hybrid {
                root,
                replica,
                policy,
            } => {
                let divergence = root.join(aep_backend_hybrid::DIVERGENCES);
                let policy = HybridPolicyWordsV1 {
                    authority: policy.authority.clone(),
                    read: policy.read.clone(),
                    on_unreachable: policy.on_unreachable.clone(),
                    on_divergence: policy.on_divergence.clone(),
                };
                match replica {
                    crate::planning::Replica::Sqlite(path) => {
                        aep_planning_migration::capture_hybrid_sqlite(
                            root,
                            path,
                            &divergence,
                            policy,
                            self.selector_binding(),
                        )
                    }
                    crate::planning::Replica::Postgres(url) => {
                        aep_planning_migration::capture_hybrid_postgres(
                            root,
                            url,
                            &divergence,
                            policy,
                            self.selector_binding(),
                        )
                    }
                }
            }
            crate::planning::Plan::Eventlog { .. } => {
                Err(aep_planning_migration::AcquisitionError::InvalidCapture)
            }
        }
    }

    fn inventory(
        &self,
        membership: &aep_domain::workspace::Membership,
    ) -> Result<InventoryFacts> {
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
        Ok(inventory(
            &report,
            matches!(
                self.plan,
                crate::planning::Plan::Markdown { .. } | crate::planning::Plan::Hybrid { .. }
            ),
            membership,
        ))
    }
}

fn resolve(common: &CommonArgs) -> Result<Resolved> {
    let here = std::env::current_dir().context("reading current directory")?;
    resolve_from(common, &here)
}

#[allow(clippy::too_many_lines)]
fn resolve_from(common: &CommonArgs, here: &Path) -> Result<Resolved> {
    let selector = if let Some(path) = &common.project {
        path.clone()
    } else {
        let root = aep_project::project::discover(here).context("no project found")?;
        root.join(aep_project::project::project_directory())
            .join(aep_domain::project::PROJECT_FILE)
    };
    let engineering = selector
        .parent()
        .context("project selector has no parent")?
        .to_path_buf();
    let selector_bytes =
        fs::read(&selector).with_context(|| format!("reading {}", selector.display()))?;
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
            SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 {
                root: host_path(root),
            }),
            PresenceV1::Missing,
            PresenceV1::Missing,
        ),
        crate::planning::Plan::Sqlite { path } => (
            BackendKindV1::Sqlite,
            SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 {
                database: host_path(path),
            }),
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
        crate::planning::Plan::Hybrid {
            root,
            replica,
            policy,
        } => {
            let replica = match replica {
                crate::planning::Replica::Sqlite(path) => {
                    SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
                        database: host_path(path),
                    })
                }
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
        crate::planning::Plan::Eventlog {
            authority_root,
            projection_root,
            authority,
        } => (
            BackendKindV1::Eventlog,
            SourceCoordinateV1::Eventlog(EventlogSourceCoordinateV1 {
                authority_root: host_path(authority_root),
            }),
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

/// What the migration will import, counted as two kinds rather than one.
///
/// `relations` used to be every authored relation in the source frontmatter, and the mapper
/// imports only those whose target is itself a captured document: on a store that declares members
/// and carries a crossing, the receipt promised one relation record per crossing more than the
/// authority ever received, and no count taken afterwards revealed it, because the post-migration
/// read agrees with the larger number through the retained body copy. The crossings are counted
/// here in their own right — not dropped — so the two figures still add up to what the source
/// declares and an operator can check either one.
fn inventory(
    report: &StoreReport,
    unrecorded: bool,
    membership: &aep_domain::workspace::Membership,
) -> InventoryFacts {
    let subjects = report.documents.len() as u64;
    let mut relations = 0_u64;
    let mut workspace_crossings = 0_u64;
    for stored in report.documents.values() {
        for relation in &stored.document.frontmatter.relations {
            if relation.crosses_to_a_declared_member(membership) {
                workspace_crossings += 1;
            } else {
                relations += 1;
            }
        }
    }
    InventoryFacts {
        inventory: InventoryCountsV1 {
            subjects,
            entities: subjects,
            relations,
            workspace_crossings,
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

/// The legacy source itself, as a diagnostic coordinate.
///
/// For a refusal that is about the source as a whole rather than one retained record. Still the
/// source, and still not the selector: `--project` names the file that chose this store, and a
/// store that does not map is not a selector defect.
fn source_root_coordinate(source: &SourceCoordinateV1) -> DiagnosticCoordinateV1 {
    let root = match source {
        SourceCoordinateV1::Markdown(markdown) => RootCoordinateV1::MarkdownRoot(
            MarkdownRootCoordinateV1 {
                root: markdown.root.clone(),
            },
        ),
        SourceCoordinateV1::Sqlite(sqlite) => {
            RootCoordinateV1::SqliteDatabase(SqliteDatabaseRootCoordinateV1 {
                database: sqlite.database.clone(),
            })
        }
        SourceCoordinateV1::Postgres(postgres) => {
            RootCoordinateV1::PostgresEndpoint(PostgresEndpointRootCoordinateV1 {
                endpoint: PresenceV1::Present(postgres.endpoint.clone()),
                endpoint_id: PresenceV1::Present(postgres.endpoint_id),
            })
        }
        SourceCoordinateV1::Hybrid(_) => hybrid_root(HybridSideV1::Local),
        // An Eventlog source is already the destination shape; acquisition refuses it before any
        // mapping, so this arm names the observation rather than inventing a root.
        SourceCoordinateV1::Eventlog(_) => RootCoordinateV1::Observation,
    };
    DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
        coordinate: PhysicalCoordinateV1::Root(root),
    })
}

fn hybrid_root(side: HybridSideV1) -> RootCoordinateV1 {
    RootCoordinateV1::HybridSide(HybridSideRootCoordinateV1 { side })
}

fn hybrid_side_coordinate(side: HybridSideV1) -> DiagnosticCoordinateV1 {
    DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
        coordinate: PhysicalCoordinateV1::Root(hybrid_root(side)),
    })
}

fn source_refusal(
    source: &SourceCoordinateV1,
    code: CommandRefusalCodeV1,
) -> Box<CommandRefusalV1> {
    Box::new(CommandRefusalV1 {
        code,
        at: source_root_coordinate(source),
    })
}

/// A Markdown mapping refusal, at the coordinate the mapper named.
///
/// The mapper's coordinates are store-relative paths of the nodes it read — `story/one.md`,
/// `journal.jsonl` — plus a few that name the whole read, such as `markdown/graph`. A coordinate
/// matching a captured node becomes that node; anything else becomes the source root. Nothing is
/// invented: the node list decides, so a synthetic coordinate is never published as a file path.
fn markdown_mapping_refusal(
    source: &SourceCoordinateV1,
    raw: &MarkdownRawV1,
    error: &aep_planning_migration::MappingError,
) -> Box<CommandRefusalV1> {
    let relative = host_path(Path::new(error.coordinate()));
    let at = if raw.nodes.iter().any(|node| node.relative == relative) {
        DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
            coordinate: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 { relative }),
        })
    } else {
        source_root_coordinate(source)
    };
    Box::new(CommandRefusalV1 {
        code: CommandRefusalCodeV1::SemanticMismatch,
        at,
    })
}

/// A `workspace.yaml` that exists and does not parse, as a refusal.
///
/// At the file that is wrong. Read as *this store declares no members* it produced a refusal
/// byte-identical to the one a store with no declaration gets — `semantic_mismatch` naming
/// whichever artifact document happened to carry the first crossing — so the operator was pointed
/// at a document that is correct and told nothing about the file that is not.
fn workspace_refusal() -> Box<CommandRefusalV1> {
    Box::new(CommandRefusalV1 {
        code: CommandRefusalCodeV1::InvalidProject,
        at: DiagnosticCoordinateV1::Config(aep_contract::migration::ConfigDiagnosticV1 {
            field: aep_contract::migration::ConfigFieldV1::Workspace,
        }),
    })
}

fn mapped_histories_for_source(
    source: &SourceCoordinateV1,
    membership: &aep_domain::workspace::Membership,
    capture: &LegacyRawCaptureV1,
    snapshot: DigestV1,
) -> std::result::Result<Vec<entity_store::asynchronous::SubjectHistory>, Box<CommandRefusalV1>> {
    let histories = match capture {
        LegacyRawCaptureV1::Markdown(raw) => {
            let SourceCoordinateV1::Markdown(_) = source else {
                return Err(source_refusal(source, CommandRefusalCodeV1::SemanticMismatch));
            };
            aep_planning_migration::markdown_boundaries_raw(raw, membership)
                .map_err(|error| markdown_mapping_refusal(source, raw, &error))?
        }
        LegacyRawCaptureV1::Sqlite(raw) | LegacyRawCaptureV1::Postgres(raw) => {
            if !matches!(
                (source, capture),
                (SourceCoordinateV1::Sqlite(_), LegacyRawCaptureV1::Sqlite(_))
                    | (
                        SourceCoordinateV1::Postgres(_),
                        LegacyRawCaptureV1::Postgres(_)
                    )
            ) {
                return Err(source_refusal(source, CommandRefusalCodeV1::SemanticMismatch));
            }
            aep_planning_migration::sql_boundaries(raw, &snapshot.as_wire())
                .map_err(|_| source_refusal(source, CommandRefusalCodeV1::SemanticMismatch))?
        }
        LegacyRawCaptureV1::Hybrid(raw) => {
            if matches!(&raw.divergences,
                aep_contract::migration::FileImageV1::Present(value) if !value.bytes.as_bytes().is_empty())
            {
                return Err(Box::new(CommandRefusalV1 {
                    code: CommandRefusalCodeV1::DivergentHybrid,
                    at: hybrid_side_coordinate(HybridSideV1::Divergences),
                }));
            }
            if !matches!(source, SourceCoordinateV1::Hybrid(_)) {
                return Err(source_refusal(source, CommandRefusalCodeV1::SemanticMismatch));
            }
            let local = aep_planning_migration::markdown_boundaries_raw(&raw.local, membership)
                .map_err(|error| markdown_mapping_refusal(source, &raw.local, &error))?;
            let replica = aep_planning_migration::sql_boundaries(&raw.replica, &snapshot.as_wire())
                .map_err(|_| {
                    Box::new(CommandRefusalV1 {
                        code: CommandRefusalCodeV1::SemanticMismatch,
                        at: hybrid_side_coordinate(HybridSideV1::Replica),
                    })
                })?;
            let local_terminal = terminal_instances(&local);
            let replica_terminal = terminal_instances(&replica);
            if local_terminal != replica_terminal {
                return Err(source_refusal(source, CommandRefusalCodeV1::DivergentHybrid));
            }
            match raw.policy.authority.as_str() {
                "local" => local,
                "replica" => replica,
                _ => return Err(source_refusal(source, CommandRefusalCodeV1::SemanticMismatch)),
            }
        }
    };
    aep_planning_migration::boundaries_with_authoritative_evidence(histories, capture, snapshot)
        .map_err(|_| source_refusal(source, CommandRefusalCodeV1::SemanticMismatch))
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
        left.entity
            .cmp(&right.entity)
            .then_with(|| left.id.cmp(&right.id))
    });
    instances
}

fn comparable_planning_instance(
    instance: &entity_core::EntityInstance,
) -> Option<entity_core::EntityInstance> {
    if instance.entity != aep_backend_entity::STORED_AS {
        return (!instance.entity.starts_with("aep.")).then(|| {
            let mut projected = instance.clone();
            if projected
                .fields
                .get("version")
                .and_then(serde_json::Value::as_str)
                == Some("1")
            {
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
    if projected
        .fields
        .get("version")
        .and_then(serde_json::Value::as_str)
        == Some("1")
    {
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
        ]
        .map(|value| digest(value.as_bytes()))
        .to_vec(),
    }
}

fn intended_v2_selector(resolved: &Resolved, authority: &AuthorityCoordinateV1) -> Result<Vec<u8>> {
    let bytes = fs::read(&resolved.selector_path)?;
    let yaml: serde_yaml::Value = serde_yaml::from_slice(&bytes)?;
    let map = yaml
        .as_mapping()
        .context("project selector is not an object")?;
    let ordered = [
        "version",
        "protocol",
        "profile",
        "summary",
        "protocols",
        "artifacts",
        "task",
        "state",
        "principles",
        "profiles",
        "schemas",
        "store",
        "planning_scope",
        "planning_tenant",
        "planning_identity",
        "providers",
    ];
    let mut output = String::from("{");
    let mut first = true;
    for key in ordered {
        let value = match key {
            "version" => Some(serde_json::Value::String("aep.project/2".to_owned())),
            "store" => {
                Some(serde_json::json!({"eventlog":{"path":"state","projection":"planning"}}))
            }
            "planning_scope" => Some(serde_json::Value::String(
                authority.logical_scope.as_str().to_owned(),
            )),
            "planning_tenant" => Some(serde_json::Value::String(
                authority.tenant.as_str().to_owned(),
            )),
            "planning_identity" => Some(serde_json::Value::String(
                authority.stream_identity.as_str().to_owned(),
            )),
            other => map
                .get(serde_yaml::Value::String(other.to_owned()))
                .map(serde_json::to_value)
                .transpose()?,
        };
        let Some(value) = value else { continue };
        if key == "providers" && value.as_object().is_some_and(serde_json::Map::is_empty) {
            continue;
        }
        if !first {
            output.push(',');
        }
        first = false;
        output.push_str(&serde_json::to_string(key)?);
        output.push(':');
        output.push_str(&serde_json::to_string(&value)?);
    }
    output.push_str("}\n");
    Ok(output.into_bytes())
}

/// The authored relations one imported subject carries that cross to another declared member.
///
/// The **same** predicate the dry-run receipt counts with, asked of the **same** authored relation
/// list: `instance_of` retains `relations:` verbatim in the subject's body and projection reads it
/// back, so the list in the authority is the list in the frontmatter. A relation that is not a
/// crossing became an `aep.relation` subject and is counted there; one that is a crossing has no
/// destination entity here and is counted only from the body. The two therefore still add to what
/// the source declares, which is what the pair's published description promises.
///
/// It is read from the authority rather than carried forward from the dry-run receipt because
/// `verify`, `projection rebuild` and an Eventlog `inspect` have no source to read: the selector
/// is v2 and the Markdown tree is gone. A receipt that can only be right on one of the four
/// commands is the shape this count already had.
fn crossings_in_body(
    terminal: &entity_core::EntityInstance,
    membership: &aep_domain::workspace::Membership,
) -> u64 {
    // `fields.document.fields.relations`, and each step of that path is somebody's decision:
    // the Eventlog authority wraps every subject's terminal value under `document`
    // (`aep-backend-eventlog/src/lib.rs:346`, and how `projection.rs:281` reads a watermark back),
    // and inside it `instance_of` retains the frontmatter `relations:` list verbatim so a
    // projection can write the document out again unchanged. A subject that carries no relations
    // — a projection watermark, a legacy evidence blob, a story with none — has no such key and
    // contributes nothing.
    terminal
        .fields
        .get("document")
        .and_then(|document| document.get("fields"))
        .and_then(|fields| fields.get("relations"))
        .and_then(|value| {
            serde_json::from_value::<Vec<aep_domain::artifact::ArtifactRelation>>(value.clone())
                .ok()
        })
        .map(|relations| {
            relations
                .iter()
                .filter(|relation| relation.crosses_to_a_declared_member(membership))
                .count() as u64
        })
        .unwrap_or_default()
}

fn inventory_from_histories(
    histories: &[entity_store::asynchronous::SubjectSnapshot],
    membership: &aep_domain::workspace::Membership,
) -> InventoryFacts {
    let mut inventory = InventoryCountsV1::default();
    let mut history = HistorySummaryV1::default();
    inventory.subjects = histories.len() as u64;
    for subject in histories {
        match subject.history.subject.entity.as_str() {
            aep_backend_entity::RELATIONS_AS => inventory.relations += 1,
            aep_backend_entity::AUDIT_AS => inventory.audit_records += 1,
            aep_backend_entity::APPLIED_AS => inventory.applied_commands += 1,
            aep_backend_eventlog::INVOCATION_AS | "aep.planning-import-boundary" => {}
            _ => {
                inventory.entities += 1;
                inventory.workspace_crossings += crossings_in_body(&subject.terminal, membership);
            }
        }
        match &subject.history.origin {
            entity_store::asynchronous::HistoryOrigin::Genesis => history.complete_recorded += 1,
            entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                history.partial += 1;
                inventory.raw_evidence_items += anchor.evidence.len() as u64;
                inventory.complete_envelopes += anchor
                    .evidence
                    .iter()
                    .filter(|value| {
                        matches!(
                            value,
                            entity_store::asynchronous::LegacyEvidence::Envelope(_)
                        )
                    })
                    .count() as u64;
                inventory.bare_decisions += anchor
                    .evidence
                    .iter()
                    .filter(|value| {
                        matches!(
                            value,
                            entity_store::asynchronous::LegacyEvidence::Decision(_)
                        )
                    })
                    .count() as u64;
                inventory.bare_events += anchor
                    .evidence
                    .iter()
                    .filter(|value| {
                        matches!(value, entity_store::asynchronous::LegacyEvidence::Event(_))
                    })
                    .count() as u64;
            }
        }
    }
    InventoryFacts {
        inventory,
        history,
        clean: true,
    }
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
    const LEGACY_COLLISION_RECORD: &str = "aep.entity:01MEM0000000000000002@1#0~19c4cfd489ddecfd";

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
            .graph_in_workspace(aep_domain::workspace::Membership::default())
            .expect("seed source graph");
        let backend =
            aep_backend_sqlite::SqliteBackend::open(database).expect("SQLite fixture opens");
        aep_backend_memory::seed::from_manifest(
            &backend,
            &graph,
            aep_backend_markdown::backend::ORGANISATION,
            aep_backend_markdown::backend::SPACE,
            aep_domain::time::Timestamp::from_epoch_millis(1_700_000_000_000),
            &aep_domain::entity::ActorRef::parse("human:migration-fixture").expect("actor"),
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
                rusqlite::params![
                    entity,
                    id,
                    record_id,
                    String::from_utf8(document.clone()).unwrap()
                ],
            )
            .expect("legacy observation inserts");
        document
    }

    fn seed_postgres_from_markdown(planning: &Path, url: &str) {
        let report = MarkdownStore::open(planning).load();
        assert!(report.is_clean(), "seed source must be clean");
        let graph = report
            .graph_in_workspace(aep_domain::workspace::Membership::default())
            .expect("seed source graph");
        let backend =
            aep_backend_postgres::PostgresBackend::connect(url).expect("PostgreSQL fixture opens");
        aep_backend_memory::seed::from_manifest(
            &backend,
            &graph,
            aep_backend_markdown::backend::ORGANISATION,
            aep_backend_markdown::backend::SPACE,
            aep_domain::time::Timestamp::from_epoch_millis(1_700_000_000_000),
            &aep_domain::entity::ActorRef::parse("human:migration-fixture").expect("actor"),
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
        fs::create_dir_all(engineering.join("state")).expect("foreign destination fixture");
        let foreign_preview = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(foreign_preview) = foreign_preview.outcome else {
            panic!("read-only preflight reports foreign destination content")
        };
        assert_eq!(
            foreign_preview.destination_requirements.foreign_content,
            vec![host_path(&engineering.join("state"))],
            "dry-run must report the existing unowned destination without writing"
        );
        assert!(
            !engineering.join("migrations").exists(),
            "foreign discovery must remain read-only"
        );
        fs::remove_dir_all(engineering.join("state")).expect("remove foreign destination fixture");
        let foreign_id = MigrationIdV1::new("cli-foreign-stage").expect("migration");
        let foreign_root = migration_root(&engineering, &foreign_id).expect("migration root");
        fs::create_dir_all(foreign_root.join("stage/authority")).expect("foreign stage fixture");
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
        assert!(
            !foreign_root.join("intent.json").exists()
                && !foreign_root.join("ownership.json").exists()
                && !foreign_root.join("recovery/raw-capture.json").exists(),
            "foreign refusal must not leave migration ownership or recovery evidence"
        );
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
        assert!(!applied
            .receipt
            .authority
            .stream_identity
            .as_str()
            .is_empty());
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
        assert_eq!(
            ordinary.selection.authority,
            PresenceV1::Present(applied.receipt.authority.clone())
        );
        assert!(
            verify(&common).success(),
            "selected authority reopens and verifies"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            let projected = engineering.join("planning/story/one.md");
            fs::set_permissions(&projected, fs::Permissions::from_mode(0o4644))
                .expect("introduce mode-only projection drift");
            assert!(
                !verify(&common).success(),
                "watermark verification must detect special-bit mode drift"
            );
            fs::set_permissions(&projected, fs::Permissions::from_mode(0o644))
                .expect("restore canonical projection mode");
            assert!(
                verify(&common).success(),
                "canonical mode restores watermark verification"
            );
        }
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
            stream_identity: applied
                .receipt
                .authority
                .stream_identity
                .as_str()
                .to_owned(),
        };
        let before_rebuild =
            aep_backend_eventlog::complete_file_snapshot(&engineering.join("state"), adapter())
                .expect("capture authority before rebuild");
        let coordinates = before_rebuild
            .histories
            .iter()
            .filter(|value| value.history.subject.entity == "aep.migration.LegacyRecordCoordinate")
            .collect::<Vec<_>>();
        assert_eq!(
            coordinates.len(),
            2,
            "both journal line shapes are retained"
        );
        let mut journal_kinds = std::collections::BTreeSet::new();
        for coordinate in &coordinates {
            let entity_store::asynchronous::HistoryOrigin::Imported(anchor) =
                &coordinate.history.origin
            else {
                panic!("legacy coordinate is an imported boundary")
            };
            assert_eq!(
                anchor
                    .instance
                    .fields
                    .get("order")
                    .and_then(serde_json::Value::as_str),
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
        assert_eq!(
            journal_kinds,
            std::collections::BTreeSet::from(["change", "event"])
        );
        let retained_lines = before_rebuild
            .histories
            .iter()
            .filter(|value| value.history.subject.entity == "aep.migration.LegacyEvidenceBlob")
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
        assert_eq!(
            rebuilt.projection.authority_snapshot,
            applied.current.snapshot_id
        );
        let after_rebuild =
            aep_backend_eventlog::complete_file_snapshot(&engineering.join("state"), adapter())
                .expect("capture authority after rebuild");
        let business = |snapshot: entity_store::asynchronous::CompleteStoreSnapshot| {
            snapshot
                .histories
                .into_iter()
                .filter(|value| {
                    value.history.subject.entity != aep_backend_eventlog::PROJECTION_METADATA_AS
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
        assert_eq!(
            changed.refusals[0].code,
            CommandRefusalCodeV1::IntentConflict
        );

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


    /// The workspace members declared beside the fixture store, read exactly as every ordinary
    /// planning read command reads them.
    fn fixture_membership(project: &Path) -> aep_domain::workspace::Membership {
        // Through the command's own reader, not a copy of it: a fixture that reads the declaration
        // its own way cannot see the question the command asks, which is which member this is.
        crate::planning::declared_membership(project).expect("the fixture declaration parses")
    }

    fn write_crossing_story(planning: &Path, target: &str) {
        fs::create_dir_all(planning.join("story")).expect("story source");
        fs::write(
            planning.join("story/crossing.md"),
            format!(
                "---\nformat: aep.planning-md/1\nid: story:crossing\nkind: story\nstatus: draft\n\
                 title: A story that names another repository\nrelations:\n\
                 - informed_by: {target}\nrevision: 1\n---\n# Story\n\nBody.\n"
            ),
        )
        .expect("crossing planning source");
    }

    fn flattened(value: &impl Serialize) -> String {
        let bytes = serde_json::to_vec(value).expect("result serialises");
        let node = OrderedNodeSeed
            .deserialize(&mut serde_json::Deserializer::from_slice(&bytes))
            .expect("result flattens");
        let mut rendered = String::new();
        flatten("", &node, &mut rendered);
        rendered
    }

    /// `story:migration-mapper-reads-the-declared-workspace`.
    ///
    /// The AEP store is the only one of six real cutovers that declares workspace members, and it
    /// carries a relation into one of them. Every ordinary read command builds the graph with the
    /// declaration and the store is valid; the mapper built it without, so the crossing read as a
    /// dangling edge and the whole migration was refused.
    #[test]
    // One ordered cutover: admission, the receipt's two counts, the authority's relation surface,
    // the retained relation data and the read back. Splitting it would measure five stores.
    #[allow(clippy::too_many_lines)]
    fn a_relation_crossing_into_a_declared_member_migrates_and_reads_back_unchanged() {
        let project = std::env::temp_dir().join(format!(
            "aep-cli-workspace-crossing-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let engineering = project.join(".engineering");
        let planning = engineering.join("planning");
        let selector = engineering.join("project.yaml");
        fs::create_dir_all(&planning).expect("planning source");
        fs::create_dir_all(project.join("protocols")).expect("protocol source");
        fs::write(
            &selector,
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
        )
        .expect("legacy selector");
        // The declaration that makes the crossing a crossing rather than a dangling edge.
        fs::write(
            engineering.join("workspace.yaml"),
            "version: aep.workspace/1\nmembers:\n  - name: other\n    source: ../other\n",
        )
        .expect("workspace declaration");
        write_crossing_story(&planning, "other/story:theirs");

        let membership = fixture_membership(&project);
        assert_eq!(
            membership.declared().len(),
            1,
            "the fixture declares exactly one member"
        );
        assert_eq!(
            membership.own(),
            None,
            "and the fixture store is not that member, so its edge really does cross out"
        );
        let before = MarkdownStore::open(&planning).load();
        before
            .graph_in_workspace(membership.clone())
            .expect("the Markdown store is valid under its own declaration");
        let artifact: aep_domain::artifact::ArtifactId =
            "story:crossing".parse().expect("artifact id");
        let source_relations = before
            .documents
            .get(&artifact)
            .expect("the crossing document loaded")
            .document
            .frontmatter
            .relations
            .clone();

        let common = CommonArgs {
            project: Some(selector.clone()),
            format: StoreOutputFormat::Json,
        };
        let destination = DestinationRequestV2::ProviderAssigned {
            logical_scope: AuthorityValueV1::new("planning-workspace-test").expect("scope"),
            tenant: AuthorityValueV1::new("tenant-workspace-test").expect("tenant"),
        };

        let preview = dry_run(&common, destination.clone());
        let DryRunOutcomeV2::Admitted(preview) = preview.outcome else {
            panic!(
                "the mapper reads the declaration every ordinary read command reads: {:?}",
                preview.outcome
            )
        };
        assert_eq!(preview.inventory.subjects, 1);
        // The receipt promises what the migration delivers. The one authored relation is a
        // crossing, so it is counted as one — not as a relation record the authority will never
        // receive — and the two counts still sum to what the source declares.
        assert_eq!(preview.inventory.relations, 0);
        assert_eq!(preview.inventory.workspace_crossings, 1);
        assert_eq!(
            preview.inventory.relations + preview.inventory.workspace_crossings,
            source_relations.len() as u64,
            "every authored relation is counted exactly once"
        );

        let applied = apply_with_control(
            &common,
            destination,
            preview.source_snapshot.0,
            MigrationIdV1::new("cli-workspace-crossing").expect("migration"),
            &DisposableWriterControl,
        );
        let ApplyOutcomeV1::Complete(_) = applied.outcome else {
            panic!("the declared crossing applies: {:?}", applied.outcome)
        };

        // The crossing survives migration as the artifact's own relation ...
        let projected =
            fs::read_to_string(planning.join("story/crossing.md")).expect("published projection");
        assert!(
            projected.contains("- informed_by: other/story:theirs"),
            "the crossing survives migration: {projected}"
        );
        // ... the migrated store still validates under the declaration the Markdown store had ...
        let after = MarkdownStore::open(&planning).load();
        assert!(after.is_clean(), "{:?}", after.failures);
        after
            .graph_in_workspace(membership)
            .expect("the migrated store is a graph under the same declaration");
        // ... and the Eventlog arm reads the artifact back identically.
        let ordinary = resolve(&common).expect("the selected v2 selector reopens");
        let backend = ordinary
            .plan
            .open_backend()
            .expect("the selected Eventlog authority opens")
            .expect("a durable backend is selected");
        let eventlog = crate::planning::report_from_backend(&backend).expect("Eventlog report");
        assert_eq!(
            eventlog
                .documents
                .get(&artifact)
                .expect("the crossing artifact is in the Eventlog authority")
                .document
                .frontmatter
                .relations,
            source_relations,
            "the Eventlog arm reads the crossing back exactly as the Markdown store held it"
        );

        // ... the authority's own relation surface is asserted, so this case can no longer pass
        // by the projection's body merge alone. A crossing has no destination entity here, so the
        // authority holds no `aep.relation` row for it — that is the model boundary, stated — and
        // what carries it across is the authored relation data retained in the subject's body.
        {
            use aep_contract::query::{QueryService, RelationQuery};

            let locator = aep_domain::entity::EntityLocator::new(
                aep_backend_markdown::backend::ORGANISATION,
                aep_backend_markdown::backend::SPACE,
                artifact.namespace(),
                artifact.name(),
            )
            .expect("the crossing artifact has an address");
            let entity = block_on(QueryService::resolve(&backend, &locator))
                .expect("the crossing artifact is in the migrated authority");
            let edges = block_on(QueryService::relations(
                &backend,
                &RelationQuery {
                    source: Some(aep_domain::entity::EntityRef::new(entity.clone())),
                    ..RelationQuery::default()
                },
            ))
            .expect("the authority answers for the edges leaving the crossing artifact");
            assert!(
                edges.items.is_empty(),
                "a relation record naming a foreign workspace member is a model change this \
                 cutover does not make: {:?}",
                edges.items
            );
            let envelope = block_on(QueryService::get(
                &backend,
                &aep_domain::entity::EntityRef::new(entity),
                aep_contract::QueryConsistency::Current,
            ))
            .expect("the crossing subject reads back");
            let body = serde_json::to_value(&envelope.data).expect("the subject body serialises");
            assert_eq!(
                body["relations"],
                serde_json::to_value(&source_relations).expect("the authored relations serialise"),
                "the crossing survives migration as the authored relation data in the subject's \
                 body: {body}"
            );
        }


        let _ = fs::remove_dir_all(project);
    }

    /// `story:migration-mapper-reads-the-declared-workspace`, amended acceptance, item 3.
    ///
    /// The receipt names the file that is wrong. A `workspace.yaml` that exists and does not parse
    /// used to be read as *this store declares no members*, which turned the crossing into a
    /// dangling edge and produced `semantic_mismatch` at whichever artifact document carried it —
    /// a refusal byte-identical to the one a store with no declaration at all gets, pointing the
    /// operator at a document that is correct.
    #[test]
    fn an_unparseable_declaration_is_refused_at_the_workspace_file() {
        let project = std::env::temp_dir().join(format!(
            "aep-cli-unparseable-declaration-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let engineering = project.join(".engineering");
        let planning = engineering.join("planning");
        let selector = engineering.join("project.yaml");
        fs::create_dir_all(&planning).expect("planning source");
        fs::create_dir_all(project.join("protocols")).expect("protocol source");
        fs::write(
            &selector,
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
        )
        .expect("legacy selector");
        fs::write(
            engineering.join("workspace.yaml"),
            "version: aep.workspace/1\nmembers:\n  - name: other\n    sourcz: ../other\n",
        )
        .expect("a declaration with one key misspelled");
        write_crossing_story(&planning, "other/story:theirs");

        let common = CommonArgs {
            project: Some(selector),
            format: StoreOutputFormat::Json,
        };
        let result = dry_run(
            &common,
            DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-declaration-test").expect("scope"),
                tenant: AuthorityValueV1::new("tenant-declaration-test").expect("tenant"),
            },
        );
        let _ = fs::remove_dir_all(&project);

        let DryRunOutcomeV2::Refused(ref refused) = result.outcome else {
            panic!("a declaration that does not parse is refused, not read as an empty one")
        };
        assert_eq!(refused.refusals.len(), 1);
        assert_eq!(
            refused.refusals[0].code,
            CommandRefusalCodeV1::InvalidProject,
            "the store is not semantically wrong; its workspace declaration is"
        );
        assert_eq!(
            refused.refusals[0].at,
            DiagnosticCoordinateV1::Config(aep_contract::migration::ConfigDiagnosticV1 {
                field: aep_contract::migration::ConfigFieldV1::Workspace,
            }),
            "the refusal names `workspace.yaml`, not an artifact document and not the selector"
        );
    }


    /// Both arms of one cutover of a store that declares a member and carries a crossing into it.
    ///
    /// The migration runs in process through [`apply_with_control`] against a disposable writer,
    /// which is the same call `plan store migrate apply` makes. Each read-path case below opens
    /// the *same* fixture twice — once as the Markdown store, once as the Eventlog authority the
    /// migration produced — and compares the value that read path builds, not a rendering of it.
    struct CrossingArms {
        project: PathBuf,
        planning: PathBuf,
        common: CommonArgs,
        membership: aep_domain::workspace::Membership,
        artifact: aep_domain::artifact::ArtifactId,
    }

    impl CrossingArms {
        fn build(name: &str) -> Self {
            Self::build_with(
                name,
                "version: aep.workspace/1\nmembers:\n  - name: other\n    source: ../other\n",
                "other/story:theirs",
                None,
            )
        }

        /// The same fixture, declared the way this repository's own `workspace.yaml` is: one
        /// member, `source: ..`, and that member **is** this store. Its edge is written
        /// `mine/story:one`, which is this store's own `story:one`.
        fn build_declaring_itself(name: &str) -> Self {
            Self::build_with(
                name,
                "version: aep.workspace/1\nmembers:\n  - name: mine\n    source: ..\n",
                "mine/story:one",
                Some("mine"),
            )
        }

        fn build_with(
            name: &str,
            declaration: &str,
            target: &str,
            expected_own: Option<&str>,
        ) -> Self {
            let project = std::env::temp_dir().join(format!(
                "aep-cli-crossing-arms-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            let engineering = project.join(".engineering");
            let planning = engineering.join("planning");
            let selector = engineering.join("project.yaml");
            fs::create_dir_all(&planning).expect("planning source");
            fs::create_dir_all(project.join("protocols")).expect("protocol source");
            fs::write(
                &selector,
                "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
            )
            .expect("legacy selector");
            fs::write(engineering.join("workspace.yaml"), declaration)
                .expect("workspace declaration");
            write_one_story(&planning, "A local story");
            write_crossing_story(&planning, target);

            let membership = fixture_membership(&project);
            assert_eq!(
                membership.declared().len(),
                1,
                "the fixture declares exactly one member"
            );
            assert_eq!(
                membership.own().map(aep_domain::workspace::MemberName::as_str),
                expected_own,
                "which member this store is decides whether `{target}` leaves the repository"
            );
            Self {
                project,
                planning,
                common: CommonArgs {
                    project: Some(selector),
                    format: StoreOutputFormat::Json,
                },
                membership,
                artifact: "story:crossing".parse().expect("artifact id"),
            }
        }

        /// The location every ordinary read command would resolve for this store.
        fn location(&self) -> crate::planning::StoreLocation {
            crate::planning::StoreLocation::at(Some(self.planning.clone()), None)
        }

        /// The Markdown arm, opened the way the read commands open it.
        fn markdown(&self, with_backend: bool) -> crate::planning::Opened {
            crate::planning::open_plan(
                crate::planning::Plan::Markdown {
                    root: self.planning.clone(),
                },
                &self.location(),
                with_backend,
            )
            .expect("the Markdown arm opens")
        }

        /// Runs the cutover in process, exactly as `plan store migrate apply` runs it.
        ///
        /// Returns the two receipts an operator reads in order — the dry-run's and the apply's —
        /// so a case can compare what the cutover promised with what it reported afterwards.
        fn migrate(&self) -> (InventoryCountsV1, ApplyCompleteV1) {
            let destination = DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-arms-test").expect("scope"),
                tenant: AuthorityValueV1::new("tenant-arms-test").expect("tenant"),
            };
            let preview = dry_run(&self.common, destination.clone());
            let DryRunOutcomeV2::Admitted(preview) = preview.outcome else {
                panic!("the declared crossing is admitted: {:?}", preview.outcome)
            };
            let applied = apply_with_control(
                &self.common,
                destination,
                preview.source_snapshot.0,
                MigrationIdV1::new("cli-crossing-arms").expect("migration"),
                &DisposableWriterControl,
            );
            let ApplyOutcomeV1::Complete(complete) = applied.outcome else {
                panic!("the declared crossing applies: {:?}", applied.outcome)
            };
            (preview.inventory, complete)
        }

        /// The Eventlog arm, opened the way the read commands open it once the selector is v2.
        fn eventlog(&self, with_backend: bool) -> crate::planning::Opened {
            let resolved = resolve(&self.common).expect("the selected v2 selector reopens");
            crate::planning::open_plan(resolved.plan, &self.location(), with_backend)
                .expect("the Eventlog arm opens")
        }

        fn discard(self) {
            let _ = fs::remove_dir_all(&self.project);
        }
    }

    /// Every receipt that publishes the two counts publishes the **same** two.
    ///
    /// `InventoryCountsV1`'s own description of the pair is that "the two sum to what the source
    /// declares". `inventory_from_histories` never assigned the crossing half, so `apply`,
    /// `verify`, `projection rebuild` and an Eventlog `inspect` each published `0` for a store
    /// whose dry-run receipt — same store, same declaration, minutes earlier — published `1`. The
    /// documented procedure is dry-run, read the receipt, apply, read the receipt: the number
    /// changed between them with nothing said.
    ///
    /// Asserted **per side**, never as a sum. The dry-run split is one `if`/`else` over one
    /// iteration, so `relations + workspace_crossings` always equals the source there, and on the
    /// four receipts below the sum was `relations + 0` — which is also a sum, just the wrong one.
    /// Only the sides say which.
    #[test]
    fn every_receipt_that_publishes_the_two_counts_publishes_the_same_two() {
        let arms = CrossingArms::build("counts");
        let (preview, applied) = arms.migrate();

        // What the cutover promised, per side. One authored relation, and it crosses out.
        assert_eq!(
            (preview.relations, preview.workspace_crossings),
            (0, 1),
            "the dry-run receipt splits the store's one authored relation: {preview:?}"
        );

        let applied_counts = &applied.current.inventory;
        assert_eq!(
            (applied_counts.relations, applied_counts.workspace_crossings),
            (0, 1),
            "the apply receipt reports the same two counts the dry-run receipt did: \
             {applied_counts:?}"
        );

        let verified = verify(&arms.common);
        let VerificationOutcomeV1::Verified(verified) = verified.outcome else {
            panic!("the migrated store verifies: {:?}", verified.outcome)
        };
        let verified_counts = &verified.authority.inventory;
        assert_eq!(
            (verified_counts.relations, verified_counts.workspace_crossings),
            (0, 1),
            "`verify` reports the same two counts: {verified_counts:?}"
        );

        let inspected = inspect(&arms.common);
        let InspectionOutcomeV1::Observed(inspected) = inspected.outcome else {
            panic!("the migrated store inspects: {:?}", inspected.outcome)
        };
        assert_eq!(
            (
                inspected.inventory.relations,
                inspected.inventory.workspace_crossings
            ),
            (0, 1),
            "an Eventlog `inspect` reports the same two counts: {:?}",
            inspected.inventory
        );

        let rebuilt = rebuild_with_control(
            &arms.common,
            applied.current.snapshot_id,
            &DisposableWriterControl,
        );
        let RebuildOutcomeV1::Rebuilt(rebuilt) = rebuilt.outcome else {
            panic!("the projection rebuilds: {:?}", rebuilt.outcome)
        };
        let rebuilt_counts = &rebuilt.authority.inventory;
        assert_eq!(
            (rebuilt_counts.relations, rebuilt_counts.workspace_crossings),
            (0, 1),
            "`projection rebuild` reports the same two counts: {rebuilt_counts:?}"
        );

        arms.discard();
    }

    /// The crossing, as a read path's own value spells it, on whichever arm.
    fn crossing_targets(value: &serde_json::Value) -> Vec<String> {
        value
            .as_array()
            .expect("a read path writes relations as an array")
            .iter()
            .map(|relation| relation["target"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    /// `story:migration-mapper-reads-the-declared-workspace`, amended acceptance, read path 1 of 5.
    ///
    /// `show` builds what it prints with [`crate::planning::shown_from`]. The crossing has no
    /// destination entity in the migrated authority and therefore no `aep.relation` row, so the
    /// only thing that can carry it across is the authored relation data retained in the subject
    /// entity's body. This asserts that `show` answers identically on both arms.
    #[test]
    fn show_lists_the_crossing_identically_on_both_arms() {
        let arms = CrossingArms::build("show");
        let config = aep_project::project::load_config(&arms.project).ok();
        let before = arms.markdown(false);
        let markdown = serde_json::to_value(crate::planning::shown_from(
            before
                .report
                .documents
                .get(&arms.artifact)
                .expect("the crossing document loaded"),
            config.as_ref(),
        ))
        .expect("`show` serialises");
        drop(before);

        arms.migrate();

        let after = arms.eventlog(false);
        let eventlog = serde_json::to_value(crate::planning::shown_from(
            after
                .report
                .documents
                .get(&arms.artifact)
                .expect("the crossing artifact is in the Eventlog authority"),
            config.as_ref(),
        ))
        .expect("`show` serialises");
        drop(after);
        arms.discard();

        assert_eq!(
            crossing_targets(&markdown["relations"]),
            vec!["other/story:theirs".to_owned()],
            "the Markdown arm is the control and must hold the crossing: {markdown}"
        );
        assert_eq!(
            markdown["relations"], eventlog["relations"],
            "`show` lists the crossing identically on both arms"
        );
    }

    /// Read path 2 of 5: `list --format json`.
    ///
    /// `Listed::relations` is a documented `jq` shape, so a crossing missing from it is a consumer
    /// reading a shorter list after a cutover than before it.
    #[test]
    fn list_json_lists_the_crossing_identically_on_both_arms() {
        let arms = CrossingArms::build("list");
        let ladders = arms.location().lifecycles().unwrap_or_default();

        let before = arms.markdown(false);
        let blocked = crate::planning::blockers_by_target(&before.report, ladders.lifecycles());
        let markdown = serde_json::to_value(
            crate::planning::select(&before.report, &blocked, None, None, None)
                .expect("`list` selects"),
        )
        .expect("`list --format json` serialises");
        drop(before);

        arms.migrate();

        let after = arms.eventlog(false);
        let blocked = crate::planning::blockers_by_target(&after.report, ladders.lifecycles());
        let eventlog = serde_json::to_value(
            crate::planning::select(&after.report, &blocked, None, None, None)
                .expect("`list` selects"),
        )
        .expect("`list --format json` serialises");
        drop(after);
        arms.discard();

        let row_of = |listed: &serde_json::Value| -> serde_json::Value {
            listed
                .as_array()
                .expect("`list` writes an array")
                .iter()
                .find(|row| row["id"] == "story:crossing")
                .expect("the crossing artifact is listed")
                .clone()
        };
        assert_eq!(
            crossing_targets(&row_of(&markdown)["relations"]),
            vec!["other/story:theirs".to_owned()],
            "the Markdown arm is the control and must hold the crossing: {markdown}"
        );
        assert_eq!(
            row_of(&markdown)["relations"],
            row_of(&eventlog)["relations"],
            "`list --format json` lists the crossing identically on both arms"
        );
    }

    /// Read path 3 of 5: `graph`.
    ///
    /// The graph is the read that refused the whole migration before this unit, so it is the one
    /// that must not quietly answer with one edge fewer afterwards.
    #[test]
    fn graph_lists_the_crossing_identically_on_both_arms() {
        let arms = CrossingArms::build("graph");

        let before = arms.markdown(false);
        let markdown = serde_json::to_value(
            before
                .report
                .graph_in_workspace(arms.membership.clone())
                .expect("the Markdown store is a graph under its own declaration"),
        )
        .expect("`graph --format json` serialises");
        drop(before);

        arms.migrate();

        let after = arms.eventlog(false);
        let eventlog = serde_json::to_value(
            after
                .report
                .graph_in_workspace(arms.membership.clone())
                .expect("the migrated store is a graph under the same declaration"),
        )
        .expect("`graph --format json` serialises");
        drop(after);
        arms.discard();

        // A serialised graph is its artifacts, keyed by id (`ArtifactGraph` is `transparent`), and
        // an edge is its document form `{<relation>: <artifact-ref>}`.
        let edges_of = |graph: &serde_json::Value| -> serde_json::Value {
            graph["story:crossing"]["relations"].clone()
        };
        let targets_of = |edges: &serde_json::Value| -> Vec<String> {
            edges
                .as_array()
                .expect("`graph` writes relations as an array")
                .iter()
                .filter_map(|edge| {
                    edge.as_object()
                        .and_then(|entry| entry.values().next())
                        .and_then(|target| target.as_str())
                        .map(ToOwned::to_owned)
                })
                .collect()
        };
        assert_eq!(
            targets_of(&edges_of(&markdown)),
            vec!["other/story:theirs".to_owned()],
            "the Markdown arm is the control and must hold the crossing: {markdown}"
        );
        assert_eq!(
            edges_of(&markdown),
            edges_of(&eventlog),
            "`graph` carries the crossing identically on both arms"
        );
    }

    /// Read path 4 of 5: `validate`.
    ///
    /// `validate` reports what the graph refuses. A crossing lost in migration would not show up
    /// here as a missing edge — it would show up as nothing at all, which is why the assertion is
    /// on the relations the same accumulation read, not only on the problem list.
    #[test]
    fn validate_reads_the_crossing_identically_on_both_arms() {
        let arms = CrossingArms::build("validate");
        let registry = arms.location().lifecycles().unwrap_or_default();

        let before = arms.markdown(false);
        let markdown_problems =
            serde_json::to_value(crate::planning::findings(&before, &registry, &arms.project))
                .expect("`validate` serialises")["problems"]
                .clone();
        let markdown_relations = before
            .report
            .documents
            .get(&arms.artifact)
            .expect("the crossing document loaded")
            .document
            .frontmatter
            .relations
            .clone();
        drop(before);

        arms.migrate();

        let after = arms.eventlog(false);
        let eventlog_problems =
            serde_json::to_value(crate::planning::findings(&after, &registry, &arms.project))
                .expect("`validate` serialises")["problems"]
                .clone();
        let eventlog_relations = after
            .report
            .documents
            .get(&arms.artifact)
            .expect("the crossing artifact is in the Eventlog authority")
            .document
            .frontmatter
            .relations
            .clone();
        drop(after);
        arms.discard();

        assert_eq!(
            markdown_relations.len(),
            1,
            "the Markdown arm is the control and must hold the crossing"
        );
        assert_eq!(
            markdown_relations, eventlog_relations,
            "`validate` judges the same edges on both arms"
        );
        assert_eq!(
            markdown_problems, eventlog_problems,
            "a declared crossing is a problem on neither arm: {markdown_problems} \
             against {eventlog_problems}"
        );
    }

    /// Read path 5 of 5: `history`.
    ///
    /// `history` lists *changes*, and a relation appears in one only as a `related` entry. The
    /// question this case asks is therefore the one that can be answered: no relation the Markdown
    /// arm's history named is missing from the migrated arm's, and the store view `history` itself
    /// opens still carries the crossing.
    #[test]
    fn history_loses_no_relation_the_markdown_arm_recorded() {
        use aep_backend_markdown::journal::Change;

        let related = |entries: &[aep_backend_markdown::journal::Entry]| -> Vec<String> {
            let mut targets: Vec<String> = entries
                .iter()
                .filter_map(|entry| match &entry.change {
                    Change::Related { target, .. } => Some(target.clone()),
                    _ => None,
                })
                .collect();
            targets.sort();
            targets
        };

        let arms = CrossingArms::build("history");
        let (markdown_entries, _) =
            aep_backend_markdown::journal::history(&arms.planning, &arms.artifact);
        let markdown_related = related(&markdown_entries);

        arms.migrate();

        // `with_backend: true`, exactly as `history_from_the_contract` opens it.
        let after = arms.eventlog(true);
        let (eventlog_entries, unreadable) =
            crate::planning::entries_from_the_contract(&after, &arms.artifact)
                .expect("the migrated authority answers for the crossing artifact");
        let eventlog_related = related(&eventlog_entries);
        let eventlog_relations = after
            .report
            .documents
            .get(&arms.artifact)
            .expect("the crossing artifact is in the store `history` opened")
            .document
            .frontmatter
            .relations
            .clone();
        drop(after);
        arms.discard();

        assert_eq!(unreadable, 0, "every migrated event reads as one entry");
        for target in &markdown_related {
            assert!(
                eventlog_related.contains(target),
                "the migrated history dropped the `related` entry naming {target}: \
                 {eventlog_related:?}"
            );
        }
        assert_eq!(
            eventlog_relations
                .iter()
                .map(|relation| relation.target.to_string())
                .collect::<Vec<_>>(),
            vec!["other/story:theirs".to_owned()],
            "the store view `history` opens still carries the crossing"
        );
    }

    /// The authority's relation surface answers the same on both arms — it never held the crossing.
    ///
    /// The `continue` at `mapping.rs:324` that skips a relation with no destination entity is not
    /// the migration inventing a rule. `aep-backend-memory/src/seed.rs:94` skips exactly the same
    /// edges when the **Markdown** plan is hydrated, with the same reason written above it, so
    /// `aep entity relations` — the one read path that answers from relation records alone — held
    /// no record for a crossing before the cutover either. Nothing is lost at migration, and this
    /// is what makes the amended acceptance a statement about the model rather than a concession.
    #[test]
    fn the_relation_surface_holds_no_crossing_on_either_arm() {
        use aep_contract::query::{QueryService, RelationQuery};

        let arms = CrossingArms::build("surface");
        let edges = |opened: &crate::planning::Opened| -> usize {
            let backend = opened.backend().expect("the arm was opened with a backend");
            let locator = aep_domain::entity::EntityLocator::new(
                aep_backend_markdown::backend::ORGANISATION,
                aep_backend_markdown::backend::SPACE,
                arms.artifact.namespace(),
                arms.artifact.name(),
            )
            .expect("the crossing artifact has an address");
            let entity = block_on(QueryService::resolve(backend, &locator))
                .expect("the crossing artifact is in this arm");
            block_on(QueryService::relations(
                backend,
                &RelationQuery {
                    source: Some(aep_domain::entity::EntityRef::new(entity)),
                    ..RelationQuery::default()
                },
            ))
            .expect("the arm answers for the edges leaving the crossing artifact")
            .items
            .len()
        };

        let before = arms.markdown(true);
        let markdown = edges(&before);
        drop(before);

        arms.migrate();

        let after = arms.eventlog(true);
        let eventlog = edges(&after);
        drop(after);
        arms.discard();

        assert_eq!(
            markdown, 0,
            "the Markdown plan's relation surface never held the crossing either"
        );
        assert_eq!(
            eventlog, markdown,
            "the cutover changes nothing about which edges are relation records"
        );
    }

    /// The same question for an edge into the member this store **is**: a relation record, on both
    /// arms.
    ///
    /// The mirror of the case above, and the one that decides the defect. `mine/story:one` read in
    /// `mine` is this store's own `story:one` — `WorkspaceRef`'s long spelling of a local edge —
    /// so the relation surface holds it on the Markdown arm (`seed.rs` resolves the target before
    /// asking whether it names a member) and on the Eventlog arm (the mapper resolves it before
    /// looking the destination entity up). Counted **per arm**, because the defect was that one
    /// arm answered differently from the other and no total said so.
    #[test]
    fn a_self_member_edge_is_a_relation_record_on_both_arms() {
        use aep_contract::query::{QueryService, RelationQuery};

        let arms = CrossingArms::build_declaring_itself("self-surface");
        let edges = |opened: &crate::planning::Opened| -> usize {
            let backend = opened.backend().expect("the arm was opened with a backend");
            let locator = aep_domain::entity::EntityLocator::new(
                aep_backend_markdown::backend::ORGANISATION,
                aep_backend_markdown::backend::SPACE,
                arms.artifact.namespace(),
                arms.artifact.name(),
            )
            .expect("the artifact has an address");
            let entity = block_on(QueryService::resolve(backend, &locator))
                .expect("the artifact is in this arm");
            block_on(QueryService::relations(
                backend,
                &RelationQuery {
                    source: Some(aep_domain::entity::EntityRef::new(entity)),
                    ..RelationQuery::default()
                },
            ))
            .expect("the arm answers for the edges leaving the artifact")
            .items
            .len()
        };

        let before = arms.markdown(true);
        let markdown = edges(&before);
        drop(before);

        let (preview, applied) = arms.migrate();

        let after = arms.eventlog(true);
        let eventlog = edges(&after);
        drop(after);

        assert_eq!(
            markdown, 1,
            "`mine/story:one` names this store's own `story:one`, so the Markdown arm's relation \
             surface holds it exactly as it holds the unqualified spelling"
        );
        assert_eq!(
            eventlog, 1,
            "and the migrated authority holds it too; the cutover changes nothing about which \
             edges are relation records"
        );
        assert_eq!(
            (preview.relations, preview.workspace_crossings),
            (1, 0),
            "nothing crosses out of this store, and the dry-run receipt says so: {preview:?}"
        );
        let applied_counts = &applied.current.inventory;
        assert_eq!(
            (applied_counts.relations, applied_counts.workspace_crossings),
            (1, 0),
            "and the apply receipt says the same: {applied_counts:?}"
        );
        arms.discard();
    }



    /// `story:migration-mapper-reads-the-declared-workspace`, second defect.
    ///
    /// A mapper refusal was mapped to `semantic_mismatch` at the *selector* coordinate, so the
    /// receipt named neither the document nor the reason — which is why the first defect took a
    /// day to find. The selector coordinate belongs to selector failures.
    #[test]
    fn a_mapper_refusal_names_the_document_it_read_and_not_the_selector() {
        let project = std::env::temp_dir().join(format!(
            "aep-cli-mapper-coordinate-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let engineering = project.join(".engineering");
        let planning = engineering.join("planning");
        let selector = engineering.join("project.yaml");
        fs::create_dir_all(&planning).expect("planning source");
        fs::create_dir_all(project.join("protocols")).expect("protocol source");
        fs::write(
            &selector,
            "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\n",
        )
        .expect("legacy selector");
        // No workspace declaration, so `other/story:theirs` is a genuinely dangling edge and the
        // refusal is correct. What it must not do is refuse without saying where.
        write_crossing_story(&planning, "other/story:theirs");

        let common = CommonArgs {
            project: Some(selector),
            format: StoreOutputFormat::Json,
        };
        let result = dry_run(
            &common,
            DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("planning-coordinate-test").expect("scope"),
                tenant: AuthorityValueV1::new("tenant-coordinate-test").expect("tenant"),
            },
        );
        let DryRunOutcomeV2::Refused(ref refused) = result.outcome else {
            panic!("an undeclared member is a dangling edge and is refused")
        };
        assert_eq!(refused.refusals.len(), 1);
        assert_eq!(
            refused.refusals[0].code,
            CommandRefusalCodeV1::SemanticMismatch
        );
        assert_eq!(
            refused.refusals[0].at,
            DiagnosticCoordinateV1::Source(aep_contract::migration::SourceDiagnosticV1 {
                coordinate: aep_contract::migration::PhysicalCoordinateV1::MarkdownPath(
                    aep_contract::migration::MarkdownPathCoordinateV1 {
                        relative: host_path(Path::new("story/crossing.md")),
                    }
                ),
            }),
            "a mapper refusal carries the mapper's coordinate, not the selector's"
        );

        let wire = HexBytesV1::new(b"story/crossing.md".to_vec()).as_wire();
        let json = serde_json::to_string(&result).expect("result serialises");
        assert!(
            json.contains("\"kind\":\"markdown_path\"") && json.contains(&wire),
            "--format json carries the coordinate as a field: {json}"
        );
        let text = flattened(&result);
        assert!(
            text.lines().any(|line| line
                .starts_with("outcome.value.refusals[0].at.value.coordinate.value.relative")
                && line.contains(&wire)),
            "plain output carries the coordinate as text: {text}"
        );

        let _ = fs::remove_dir_all(project);
    }


    /// The mapper names a captured node by its store-relative path, and names a few things that
    /// are the whole read — `markdown/graph`, `markdown/path` — with words that are not paths.
    /// Publishing one of those as a file path would be a coordinate nobody can open.
    #[test]
    fn a_mapper_coordinate_is_a_captured_node_or_it_is_the_source_root() {
        let root = host_path(Path::new("/nowhere/.engineering/planning"));
        let source = SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 { root: root.clone() });
        let raw = MarkdownRawV1 {
            nodes: vec![aep_contract::migration::MarkdownNodeV1 {
                relative: host_path(Path::new("story/one.md")),
                node: aep_contract::migration::MarkdownNodeKindV1::Regular(
                    aep_contract::migration::RegularMarkdownNodeV1 {
                        bytes: HexBytesV1::new(Vec::new()),
                    },
                ),
            }],
        };
        let at = |coordinate: &str| {
            markdown_mapping_refusal(
                &source,
                &raw,
                &aep_planning_migration::MappingError::Invalid {
                    coordinate: coordinate.to_owned(),
                },
            )
        };

        assert_eq!(
            at("story/one.md").at,
            DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
                coordinate: PhysicalCoordinateV1::MarkdownPath(MarkdownPathCoordinateV1 {
                    relative: host_path(Path::new("story/one.md")),
                }),
            }),
            "a coordinate that is a captured node is published as that node"
        );
        for synthetic in ["markdown/graph", "markdown/path", "story/absent.md"] {
            assert_eq!(
                at(synthetic).at,
                DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
                    coordinate: PhysicalCoordinateV1::Root(RootCoordinateV1::MarkdownRoot(
                        MarkdownRootCoordinateV1 { root: root.clone() }
                    )),
                }),
                "`{synthetic}` names no captured node, so it is not published as a file path"
            );
        }
    }

    /// The class the selector-coordinate defect belonged to: a refusal about the *source* must
    /// never be reported at the selector. `source_root_coordinate` is exhaustive over
    /// `SourceCoordinateV1` by construction — a new source kind will not compile until it answers
    /// — and this asserts every existing answer is a source coordinate.
    #[test]
    fn every_legacy_source_kind_names_itself_and_never_the_selector() {
        let path = host_path(Path::new("/nowhere"));
        let endpoint = aep_contract::migration::PostgresEndpointV1 {
            hosts: Vec::new(),
            database: "planning".to_owned(),
            schema: "public".to_owned(),
        };
        let sources = [
            SourceCoordinateV1::Markdown(MarkdownSourceCoordinateV1 { root: path.clone() }),
            SourceCoordinateV1::Sqlite(SqliteSourceCoordinateV1 {
                database: path.clone(),
            }),
            SourceCoordinateV1::Postgres(aep_contract::migration::PostgresSourceCoordinateV1 {
                endpoint_id: endpoint.endpoint_id(),
                endpoint,
            }),
            SourceCoordinateV1::Hybrid(HybridSourceCoordinateV1 {
                local_root: path.clone(),
                replica: SqlReplicaCoordinateV1::Sqlite(SqliteReplicaCoordinateV1 {
                    database: path.clone(),
                }),
                divergence_file: path.clone(),
                policy: PresenceV1::Missing,
            }),
            SourceCoordinateV1::Eventlog(EventlogSourceCoordinateV1 {
                authority_root: path,
            }),
        ];
        for source in &sources {
            let refusal = source_refusal(source, CommandRefusalCodeV1::SemanticMismatch);
            assert!(
                matches!(refusal.at, DiagnosticCoordinateV1::Source(_)),
                "a source refusal names the source: {source:?} produced {:?}",
                refusal.at
            );
        }
        assert_eq!(
            source_root_coordinate(&sources[0]),
            DiagnosticCoordinateV1::Source(SourceDiagnosticV1 {
                coordinate: PhysicalCoordinateV1::Root(RootCoordinateV1::MarkdownRoot(
                    MarkdownRootCoordinateV1 {
                        root: host_path(Path::new("/nowhere")),
                    }
                )),
            })
        );
        assert_eq!(
            source_root_coordinate(&sources[3]),
            hybrid_side_coordinate(HybridSideV1::Local),
            "a hybrid mapping refusal that is not about one record is about the authoritative side"
        );
    }

    #[test]
    fn dry_run_retains_catalog_refusal_coordinate_without_creating_migration_state() {
        let project = std::env::temp_dir().join(format!(
            "aep-cli-catalog-refusal-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
        ));
        let engineering = project.join(".engineering");
        let planning = engineering.join("planning");
        let selector = engineering.join("project.yaml");
        let database = engineering.join("plan.sqlite3");
        fs::create_dir_all(project.join("protocols")).unwrap();
        write_one_story(&planning, "One");
        seed_sqlite_from_markdown(&planning, &database);
        let connection = rusqlite::Connection::open(&database).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE history ADD COLUMN foreign_value TEXT DEFAULT 'retained failure'",
            )
            .unwrap();
        drop(connection);
        fs::write(&selector, "version: aep.project/1\nprotocol: adp/1\nprofile: development.standard\nprotocols: ../protocols\nstore:\n  sqlite: plan.sqlite3\n").unwrap();
        let before = fs::read(&database).unwrap();
        let result = dry_run(
            &CommonArgs {
                project: Some(selector),
                format: StoreOutputFormat::Json,
            },
            DestinationRequestV2::ProviderAssigned {
                logical_scope: AuthorityValueV1::new("refusal-scope").unwrap(),
                tenant: AuthorityValueV1::new("refusal-tenant").unwrap(),
            },
        );
        let DryRunOutcomeV2::Refused(refused) = result.outcome else {
            panic!("unsupported schema admitted");
        };
        assert_eq!(refused.refusals.len(), 1);
        assert_eq!(
            refused.refusals[0].code,
            CommandRefusalCodeV1::UnsupportedSchema
        );
        assert_eq!(
            refused.refusals[0].at,
            DiagnosticCoordinateV1::Source(aep_contract::migration::SourceDiagnosticV1 {
                coordinate: aep_contract::migration::PhysicalCoordinateV1::SqlCatalog(
                    aep_contract::migration::SqlCatalogCoordinateV1 {
                        namespace: PresenceV1::Present("main".into()),
                        family: aep_contract::migration::CatalogFamilyV1::TableDefinition,
                        table: PresenceV1::Present("history".into()),
                        row: PresenceV1::Present(0),
                    }
                ),
            })
        );
        assert_eq!(fs::read(database).unwrap(), before);
        assert!(!engineering.join("migrations").exists());
        assert!(!engineering.join("state").exists());
        fs::remove_dir_all(project).unwrap();
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
        assert_eq!(
            sqlite.destination_requirements.foreign_content,
            vec![host_path(&planning)],
            "the Markdown seed is not owned by the selected SQL source"
        );

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
        fs::remove_dir_all(&planning).expect("positive SQL projection begins unoccupied");
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
        let authority_snapshot = aep_backend_eventlog::complete_file_snapshot(
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
        let ordered_subject = serde_json::from_slice::<entity_store::RecordedObservation>(
            &legacy_observation,
        )
        .expect("SQLite observation is a real encoded envelope");
        let ordered_store = aep_backend_eventlog::open(
            engineering.join("state"),
            authority.logical_scope.clone(),
            authority.tenant.clone(),
            authority.stream_identity.clone(),
        )
        .expect("ordinary backend opens ordered imported history");
        let ordered_evidence = ordered_store
            .with_store(|store| {
                store.legacy_evidence_for_subject(&ordered_subject.entity, &ordered_subject.id)
            })
            .expect("ordinary reader resolves ordered imported evidence");
        assert_eq!(ordered_evidence.len(), 1);
        assert_eq!(ordered_evidence[0].exact_bytes, legacy_observation);
        assert_eq!(
            ordered_evidence[0].order,
            aep_backend_eventlog::LegacyBoundaryOrder::Subject
        );
        assert_eq!(ordered_evidence[0].ordinal, PresenceV1::Present(0));
        assert!(
            ordered_store
                .with_store(|store| {
                    entity_store::HistoryProvider::observations(
                        store,
                        &ordered_subject.entity,
                        &ordered_subject.id,
                    )
                })
                .expect("recorded suffix remains queryable")
                .is_empty(),
            "imported ordered evidence remains separate from the newly recorded suffix"
        );
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
            .find(|subject| subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob")
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
        let decision_definition = serde_json::from_value(serde_json::json!({
            "entity": "aep.reader-decision",
            "version": 1,
            "schema": { "fields": { "title": { "type": "string", "required": true } } },
            "lifecycle": { "initial": "open", "states": ["open", "closed"] },
            "operations": { "close": { "transitions": [{ "from": "open", "to": "closed" }] } }
        }))
        .expect("decision fixture definition parses");
        let mut decision_registry = entity_core::Registry::new();
        decision_registry
            .register(decision_definition)
            .expect("decision fixture definition validates");
        let decision = entity_core::Runtime::new(&decision_registry)
            .create(
                "aep.reader-decision",
                1,
                "reader-decision",
                serde_json::json!({"title": "Retained decision"}),
            )
            .expect("real decision is created");
        let legacy_decision = entity_store::RecordedCommit::new(
            decision,
            &entity_store::Recording {
                record_id: "legacy-decision-one".to_owned(),
                recorded_at: "2026-09-16T12:00:00Z".to_owned(),
                correlation: None,
                causation: None,
                actor: None,
            },
        )
        .expect("real decision is recorded");
        let decision_subject = entity_store::asynchronous::RecordedEntry::Decision(
            legacy_decision.clone(),
        )
        .subject()
        .clone();
        let legacy_decision_bytes =
            serde_json::to_vec(&legacy_decision).expect("decision envelope serialises");
        let unavailable_authority = AuthorityCoordinateV1 {
            logical_scope: AuthorityValueV1::new("planning-unavailable").expect("scope"),
            tenant: AuthorityValueV1::new("tenant-unavailable").expect("tenant"),
            stream_identity: AuthorityValueV1::new(unavailable_stream.clone()).expect("stream"),
        };
        unavailable_histories =
            aep_planning_migration::bind_authoritative_evidence_with_unavailable(
                unavailable_histories,
                &unavailable_authority,
                &[
                    aep_planning_migration::UnavailableLegacyEnvelope {
                        source_locator: "history/aep.entity/01MEM0000000000000002/unavailable"
                            .to_owned(),
                        destination_entity: unavailable_subject.entity.clone(),
                        destination_id: unavailable_subject.id.clone(),
                        evidence_kind: aep_contract::migration::HistoryKindV1::Observation,
                        original_record_id: LEGACY_COLLISION_RECORD.to_owned(),
                        exact_bytes: HexBytesV1::new(legacy_observation.clone()),
                    },
                    aep_planning_migration::UnavailableLegacyEnvelope {
                        source_locator: "history/aep.reader-decision/reader-decision/unavailable"
                            .to_owned(),
                        destination_entity: decision_subject.entity.clone(),
                        destination_id: decision_subject.id.clone(),
                        evidence_kind: aep_contract::migration::HistoryKindV1::Decision,
                        original_record_id: "legacy-decision-one".to_owned(),
                        exact_bytes: HexBytesV1::new(legacy_decision_bytes.clone()),
                    },
                ],
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
        assert!(aep_backend_eventlog::validate_legacy_boundary_snapshot(
            &unavailable_snapshot,
            &unavailable_adapter,
        )
        .expect("unavailable-order join validates")
        .contains(LEGACY_COLLISION_RECORD));
        let unavailable_coordinate = unavailable_snapshot
            .histories
            .iter()
            .find(|subject| {
                subject.history.subject.entity == "aep.migration.LegacyRecordCoordinate"
                    && subject.terminal.fields.get("original_record_id")
                        == Some(&serde_json::json!({
                            "kind": "present",
                            "value": LEGACY_COLLISION_RECORD
                        }))
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
        let mut wrong_subject = unavailable_snapshot.clone();
        let altered_coordinate = wrong_subject
            .histories
            .iter_mut()
            .find(|subject| {
                subject.history.subject == unavailable_coordinate.history.subject
            })
            .expect("unavailable coordinate is present");
        altered_coordinate.terminal.fields.insert(
            "destination_id".to_owned(),
            serde_json::Value::String("wrong-subject".to_owned()),
        );
        assert!(
            aep_backend_eventlog::validate_legacy_boundary_snapshot(
                &wrong_subject,
                &unavailable_adapter,
            )
            .expect_err("an envelope cannot be joined to another destination subject")
            .contains("envelope subject or identity disagrees")
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
            unavailable_path.clone(),
            unavailable_adapter.logical_scope.clone(),
            unavailable_adapter.tenant.clone(),
            unavailable_adapter.stream_identity.clone(),
        )
        .expect("ordinary backend opens the unavailable-order authority");
        let subject_evidence = unavailable_store
            .with_store(|store| {
                store.legacy_evidence_for_subject(
                    &unavailable_subject.entity,
                    &unavailable_subject.id,
                )
            })
            .expect("ordinary subject lookup reads imported evidence");
        assert_eq!(subject_evidence.len(), 1);
        let original_id_evidence = unavailable_store
            .with_store(|store| store.legacy_evidence_by_original_id(LEGACY_COLLISION_RECORD))
            .expect("ordinary original-id lookup reads imported evidence")
            .expect("the reserved original id resolves");
        assert_eq!(subject_evidence[0], original_id_evidence);
        let decision_evidence = unavailable_store
            .with_store(|store| {
                store.legacy_evidence_for_subject(&decision_subject.entity, &decision_subject.id)
            })
            .expect("ordinary reader resolves a real encoded decision");
        assert_eq!(decision_evidence.len(), 1);
        assert_eq!(decision_evidence[0].exact_bytes, legacy_decision_bytes);
        assert_eq!(
            decision_evidence[0].kind,
            aep_backend_eventlog::LegacyBoundaryKind::Decision
        );
        assert_eq!(
            unavailable_store
                .with_store(|store| store.legacy_evidence_by_original_id("legacy-decision-one"))
                .expect("original ID resolves the real encoded decision"),
            Some(decision_evidence[0].clone())
        );
        assert_eq!(original_id_evidence.exact_bytes, legacy_observation);
        assert_eq!(original_id_evidence.source_snapshot, sqlite.source_snapshot.0);
        assert_eq!(original_id_evidence.destination_entity, unavailable_subject.entity);
        assert_eq!(original_id_evidence.destination_id, unavailable_subject.id);
        assert_eq!(original_id_evidence.original_record_id, LEGACY_COLLISION_RECORD);
        assert_eq!(
            original_id_evidence.kind,
            aep_backend_eventlog::LegacyBoundaryKind::Observation
        );
        assert_eq!(
            original_id_evidence.order,
            aep_backend_eventlog::LegacyBoundaryOrder::Unavailable
        );
        assert_eq!(original_id_evidence.ordinal, PresenceV1::Missing);
        assert_eq!(
            original_id_evidence.source_locator,
            "history/aep.entity/01MEM0000000000000002/unavailable"
        );
        assert_eq!(
            original_id_evidence.boundary_id,
            unavailable_snapshot
                .histories
                .iter()
                .find(|value| value.history.subject.entity == "aep.planning-import-boundary")
                .expect("import boundary remains provider-complete")
                .history
                .subject
                .id
        );
        assert_eq!(
            original_id_evidence.coordinate_subject_id,
            unavailable_coordinate.history.subject.id
        );
        assert!(unavailable_snapshot.histories.iter().any(|value| {
            value.history.subject.entity == "aep.migration.LegacyEvidenceBlob"
                && value.history.subject.id == original_id_evidence.evidence_blob_subject_id
        }));
        assert!(unavailable_snapshot.histories.iter().any(|value| {
            value.history.subject.entity == "aep.migration.LegacyIdReservationRoster"
                && value.history.subject.id == original_id_evidence.reservation_roster_id
        }));
        assert!(
            unavailable_store
                .with_store(|store| store.legacy_evidence_by_original_id("not-reserved"))
                .expect("missing original id is a complete lookup")
                .is_none()
        );
        assert!(
            unavailable_store
                .with_store(|store| {
                    entity_store::HistoryProvider::observations(
                        store,
                        &unavailable_subject.entity,
                        &unavailable_subject.id,
                    )
                })
                .expect("recorded suffix remains queryable")
                .is_empty(),
            "unavailable imported evidence is not newly recorded history"
        );
        let unavailable_collision = block_on(unavailable_store.execute(collision_command()))
            .expect_err("unavailable-order roster blocks original record-id reuse");
        assert!(
            unavailable_collision
                .to_string()
                .contains(LEGACY_COLLISION_RECORD),
            "{unavailable_collision}"
        );
        drop(unavailable_store);
        let evidence_blob = authority_snapshot
            .histories
            .iter()
            .find_map(|subject| {
                (subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob").then(|| {
                    match &subject.history.origin {
                        entity_store::asynchronous::HistoryOrigin::Imported(anchor) => {
                            anchor.instance.fields.clone()
                        }
                        entity_store::asynchronous::HistoryOrigin::Genesis => unreachable!(),
                    }
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
        let reopened_unavailable_store = aep_backend_eventlog::open(
            unavailable_path,
            unavailable_adapter.logical_scope,
            unavailable_adapter.tenant,
            unavailable_adapter.stream_identity,
        )
        .expect("reopen imported evidence after recovery-copy deletion");
        assert_eq!(
            reopened_unavailable_store
                .with_store(|store| {
                    store.legacy_evidence_for_subject(
                        &unavailable_subject.entity,
                        &unavailable_subject.id,
                    )
                })
                .expect("subject lookup uses only the selected authority"),
            subject_evidence
        );
        assert_eq!(
            reopened_unavailable_store
                .with_store(|store| store.legacy_evidence_by_original_id(LEGACY_COLLISION_RECORD))
                .expect("original-id lookup uses only the selected authority"),
            Some(original_id_evidence)
        );
        let after_copy_collision = block_on(reopened_unavailable_store.execute(collision_command()))
            .expect_err("original ID remains reserved without a recovery copy");
        assert!(after_copy_collision.to_string().contains(LEGACY_COLLISION_RECORD));
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
        assert_eq!(
            postgres.destination_requirements.foreign_content,
            vec![host_path(&planning)],
            "the Markdown seed is not owned by the selected PostgreSQL source"
        );

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
        fs::remove_dir_all(&planning).expect("positive PostgreSQL projection begins unoccupied");
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
            .filter(|subject| subject.history.subject.entity == "aep.migration.LegacyEvidenceBlob")
            .any(|subject| {
                subject.terminal.fields.get("exact_bytes")
                    == Some(
                        &serde_json::to_value(HexBytesV1::new(legacy_observation.clone()))
                            .expect("exact PostgreSQL bytes serialise"),
                    )
            });
        assert!(
            retained,
            "exact PostgreSQL history bytes remain provider-complete"
        );
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
            out!(
                "{}",
                serde_yaml::to_string(value).context("rendering mutation result")?
            );
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

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
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
            let count = if path.is_empty() {
                "count".to_owned()
            } else {
                format!("{path}.count")
            };
            let _ = writeln!(output, "{count}\t{}", values.len());
            for (index, value) in values.iter().enumerate() {
                flatten(&format!("{path}[{index}]"), value, output);
            }
        }
        OrderedNode::Object(values) => {
            for (name, value) in values {
                let child = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                flatten(&child, value, output);
            }
        }
    }
}
