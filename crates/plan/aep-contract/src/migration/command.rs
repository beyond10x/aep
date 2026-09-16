//! Closed command, durable-phase, and result values for planning authority migration.

#![allow(missing_docs)]

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    DigestV1, HexBytesV1, HostPathV1, PhysicalCoordinateV1, PresenceV1, SourceCoordinateV1,
};

macro_rules! literal {
    ($name:ident, $value:literal) => {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name;

        impl $name {
            /// The sole admitted wire spelling.
            pub const VALUE: &'static str = $value;
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(Self::VALUE)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                if value == Self::VALUE {
                    Ok(Self)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected {}, found {value:?}",
                        Self::VALUE
                    )))
                }
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_owned()
            }

            fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                let mut schema = schemars::schema::SchemaObject {
                    instance_type: Some(schemars::schema::InstanceType::String.into()),
                    enum_values: Some(vec![serde_json::Value::String(Self::VALUE.to_owned())]),
                    ..schemars::schema::SchemaObject::default()
                };
                schema.metadata().description =
                    Some(format!("The scalar literal {}.", Self::VALUE));
                schema.into()
            }
        }
    };
}

literal!(InspectionFormatV1, "aep.planning-inspection/1");
literal!(DryRunFormatV1, "aep.planning-migration-dry-run/1");
literal!(ApplyFormatV1, "aep.planning-migration-apply/1");
literal!(VerificationFormatV1, "aep.planning-verification/1");
literal!(RebuildFormatV1, "aep.planning-projection-rebuild/1");
literal!(MigrationReceiptFormatV1, "aep.planning-migration-receipt/1");
literal!(MigrationIntentFormatV1, "aep.planning-migration-intent/1");
literal!(OwnershipMarkerFormatV1, "aep.planning-store-ownership/1");
literal!(PhaseRecordFormatV1, "aep.planning-migration-phase/1");
literal!(CurrentPhaseFormatV1, "aep.planning-migration-current/1");
literal!(MappingFormatV1, "aep.planning-import/1");
literal!(
    ProjectionWatermarkFormatV1,
    "aep.planning-projection-watermark/1"
);
literal!(DryRunFormatV2, "aep.planning-migration-dry-run/2");
literal!(MigrationIntentFormatV2, "aep.planning-migration-intent/2");
literal!(OwnershipMarkerFormatV2, "aep.planning-store-ownership/2");
literal!(PhaseRecordFormatV2, "aep.planning-migration-phase/2");
literal!(CurrentPhaseFormatV2, "aep.planning-migration-current/2");

/// Stable migration identity supplied by the operator.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(transparent)]
pub struct MigrationIdV1(String);

impl MigrationIdV1 {
    /// Validates and preserves a migration identity byte-for-byte.
    pub fn new(value: impl Into<String>) -> Result<Self, MigrationScalarError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 128
            || !bytes[0].is_ascii_alphanumeric()
            || !bytes.iter().skip(1).all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
        {
            return Err(MigrationScalarError::MigrationId);
        }
        Ok(Self(value))
    }

    /// Borrows the exact caller-supplied identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for MigrationIdV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MigrationIdV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// One opaque public adapter authority member.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(transparent)]
pub struct AuthorityValueV1(String);

impl AuthorityValueV1 {
    /// Validates without trimming or normalising the caller's bytes.
    pub fn new(value: impl Into<String>) -> Result<Self, MigrationScalarError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 255
            || value.chars().all(char::is_whitespace)
            || bytes
                .iter()
                .any(|byte| *byte == 0 || byte.is_ascii_control())
        {
            return Err(MigrationScalarError::AuthorityValue);
        }
        Ok(Self(value))
    }

    /// Borrows the exact opaque authority value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for AuthorityValueV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AuthorityValueV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Invalid command-contract scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MigrationScalarError {
    /// Migration identity grammar mismatch.
    #[error("migration identity must match [A-Za-z0-9][A-Za-z0-9._:-]{{0,127}}")]
    MigrationId,
    /// Authority value grammar mismatch.
    #[error("authority value must contain 1..=255 UTF-8 bytes, a non-whitespace byte, and no control bytes")]
    AuthorityValue,
}

macro_rules! digest_newtype {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
            #[serde(transparent)]
            pub struct $name(pub DigestV1);
        )+
    };
}

digest_newtype!(
    AuthorityIdV1,
    AuthoritySnapshotIdV1,
    SourceSnapshotIdV1,
    IntentDigestV1,
    PhaseDigestV1,
    ProjectionInventoryDigestV1,
    ReceiptDigestV1,
);

/// Exact public Eventlog adapter authority tuple.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorityCoordinateV1 {
    pub logical_scope: AuthorityValueV1,
    pub tenant: AuthorityValueV1,
    pub stream_identity: AuthorityValueV1,
}

/// Whether migration binds an existing provider identity or lets a fresh provider assign it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DestinationRequestV2 {
    ProviderAssigned {
        logical_scope: AuthorityValueV1,
        tenant: AuthorityValueV1,
    },
    ExactExisting {
        authority: AuthorityCoordinateV1,
    },
}

/// Selected planning backend kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackendKindV1 {
    Markdown,
    Sqlite,
    Postgres,
    Hybrid,
    Eventlog,
}

/// Accepted project selector version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ProjectVersionV1 {
    #[serde(rename = "aep.project/1")]
    V1,
    #[serde(rename = "aep.project/2")]
    V2,
}

/// Available legacy history boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistoryAvailabilityV1 {
    CompleteRecorded,
    Partial,
    Unrecorded,
}

/// Last durably established migration phase.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MigrationPhaseV1 {
    Prepared,
    DestinationProvisioned,
    Imported,
    Verified,
    Published,
    Selected,
    ProjectionPublished,
    Complete,
}

/// Project configuration field coordinate.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFieldV1 {
    Version,
    Store,
    PlanningScope,
    PlanningTenant,
    PlanningIdentity,
    Protocol,
    Profile,
    Protocols,
    Artifacts,
    Task,
    State,
    Principles,
    Profiles,
    Schemas,
    Providers,
}

/// Durable migration component coordinate.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MigrationComponentV1 {
    Intent,
    Ownership,
    PhasePrepared,
    PhaseDestinationProvisioned,
    PhaseImported,
    PhaseVerified,
    PhasePublished,
    PhaseSelected,
    PhaseProjectionPublished,
    PhaseComplete,
    Current,
    Receipt,
    RecoveryCapture,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubjectCoordinateV1 {
    pub entity: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectorDiagnosticV1 {
    pub path: HostPathV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigDiagnosticV1 {
    pub field: ConfigFieldV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceDiagnosticV1 {
    pub coordinate: PhysicalCoordinateV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrationDiagnosticV1 {
    pub migration_id: MigrationIdV1,
    pub component: MigrationComponentV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathDiagnosticV1 {
    pub path: HostPathV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorityDiagnosticV1 {
    pub authority: AuthorityCoordinateV1,
    pub subject: PresenceV1<SubjectCoordinateV1>,
    pub record_id: PresenceV1<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputStreamV1 {
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutputDiagnosticV1 {
    pub stream: OutputStreamV1,
}

/// Closed diagnostic coordinate; public results carry no provider prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DiagnosticCoordinateV1 {
    Selector(SelectorDiagnosticV1),
    Config(ConfigDiagnosticV1),
    Source(SourceDiagnosticV1),
    Migration(MigrationDiagnosticV1),
    Destination(PathDiagnosticV1),
    Projection(PathDiagnosticV1),
    Authority(AuthorityDiagnosticV1),
    Output(OutputDiagnosticV1),
}

/// Stable command refusal vocabulary.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CommandRefusalCodeV1 {
    UnknownProjectVersion,
    InvalidProject,
    UnknownField,
    PathOutsideProject,
    PathAlias,
    NestedStorePaths,
    ForeignContent,
    ProjectionIsNotAuthority,
    LegacySourceRetired,
    OwnershipUncertain,
    SourceUnreadable,
    SourceUnstable,
    PendingLegacyIntent,
    UnsupportedSchema,
    IncompleteInventory,
    MissingDefinitions,
    SemanticMismatch,
    DivergentHybrid,
    SnapshotChanged,
    ConfigChanged,
    SelectorChanged,
    WriterExclusionUnavailable,
    IntentConflict,
    ForeignStage,
    DestinationConflict,
    ProvisionUncertain,
    ImportConflict,
    ImportUncertain,
    VerificationMismatch,
    PublishUncertain,
    SelectorUncertain,
    ProjectionConflict,
    ProjectionDrift,
    AuthorityIdentityMismatch,
    AuthoritySnapshotChanged,
    IncompletePublication,
    ReceiptConflict,
    CommandIdentityConflict,
    CommittedProjectionFailure,
    OutputFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandRefusalV1 {
    pub code: CommandRefusalCodeV1,
    pub at: DiagnosticCoordinateV1,
}

/// Exact selected source and destination coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectionV1 {
    pub project_version: ProjectVersionV1,
    pub backend: BackendKindV1,
    pub selector: HostPathV1,
    pub selector_digest: DigestV1,
    pub config_digest: DigestV1,
    pub source: SourceCoordinateV1,
    pub authority: PresenceV1<AuthorityCoordinateV1>,
    pub projection: PresenceV1<HostPathV1>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InventoryCountsV1 {
    pub subjects: u64,
    pub entities: u64,
    pub relations: u64,
    pub audit_records: u64,
    pub applied_commands: u64,
    pub complete_envelopes: u64,
    pub bare_decisions: u64,
    pub bare_events: u64,
    pub raw_evidence_items: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HistorySummaryV1 {
    pub complete_recorded: u64,
    pub partial: u64,
    pub unrecorded: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MappingIdentityV1 {
    pub mapping_version: MappingFormatV1,
    pub definition_digests: Vec<DigestV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorityObservationV1 {
    pub authority: AuthorityCoordinateV1,
    pub snapshot_id: AuthoritySnapshotIdV1,
    pub inventory: InventoryCountsV1,
    pub history: HistorySummaryV1,
    pub capture_digest: DigestV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionDriftV1 {
    Current,
    Missing,
    Extra,
    Stale,
    Corrupt,
    ForeignConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionObservationV1 {
    pub root: HostPathV1,
    pub authority_snapshot: AuthoritySnapshotIdV1,
    pub inventory_digest: ProjectionInventoryDigestV1,
    pub watermark_digest: DigestV1,
    pub drift: ProjectionDriftV1,
}

/// Authority-owned join proving which complete snapshot a Markdown projection represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionWatermarkV1 {
    pub format: ProjectionWatermarkFormatV1,
    pub authority: AuthorityCoordinateV1,
    pub authority_snapshot: AuthoritySnapshotIdV1,
    pub projection_inventory_digest: ProjectionInventoryDigestV1,
    pub watermark_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrationReceiptV1 {
    pub format: MigrationReceiptFormatV1,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub source_snapshot: SourceSnapshotIdV1,
    pub source_config_digest: DigestV1,
    pub authority: AuthorityCoordinateV1,
    pub mapping: MappingIdentityV1,
    pub import_comparison_digest: DigestV1,
    pub selected_selector_digest: DigestV1,
    pub covered_authority_snapshot: AuthoritySnapshotIdV1,
    pub projection_inventory_digest: ProjectionInventoryDigestV1,
    pub receipt_digest: ReceiptDigestV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InspectionReadinessV1 {
    Ready,
    SourceUnready,
    MigrationIncomplete,
    ProjectionDrifted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DestinationRequirementsV1 {
    pub migration_root: HostPathV1,
    pub staging_path: HostPathV1,
    pub destination_path: HostPathV1,
    pub projection_path: HostPathV1,
    pub authority: AuthorityCoordinateV1,
    pub foreign_content: Vec<HostPathV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DestinationRequirementsV2 {
    pub migration_root: HostPathV1,
    pub staging_path: HostPathV1,
    pub destination_path: HostPathV1,
    pub projection_path: HostPathV1,
    pub destination_request: DestinationRequestV2,
    pub foreign_content: Vec<HostPathV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectionObservedV1 {
    pub selection: SelectionV1,
    pub raw_complete: bool,
    pub inventory: InventoryCountsV1,
    pub history: HistorySummaryV1,
    pub migration_phase: PresenceV1<MigrationPhaseV1>,
    pub authority: PresenceV1<AuthorityObservationV1>,
    pub projection: PresenceV1<ProjectionObservationV1>,
    pub readiness: InspectionReadinessV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RefusedV1 {
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Closed persisted variants stay direct to preserve the Rust API.
pub enum InspectionOutcomeV1 {
    Observed(InspectionObservedV1),
    Refused(RefusedV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectionResultV1 {
    pub format: InspectionFormatV1,
    pub outcome: InspectionOutcomeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DryRunAdmittedV1 {
    pub selection: SelectionV1,
    pub source_snapshot: SourceSnapshotIdV1,
    pub inventory: InventoryCountsV1,
    pub history: HistorySummaryV1,
    pub target_authority: AuthorityCoordinateV1,
    pub mapping: MappingIdentityV1,
    pub comparison_digest: DigestV1,
    pub destination_requirements: DestinationRequirementsV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DryRunRefusedV1 {
    pub selection: PresenceV1<SelectionV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Closed persisted variants stay direct to preserve the Rust API.
pub enum DryRunOutcomeV1 {
    Admitted(DryRunAdmittedV1),
    Refused(DryRunRefusedV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DryRunResultV1 {
    pub format: DryRunFormatV1,
    pub outcome: DryRunOutcomeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DryRunAdmittedV2 {
    pub selection: SelectionV1,
    pub source_snapshot: SourceSnapshotIdV1,
    pub inventory: InventoryCountsV1,
    pub history: HistorySummaryV1,
    pub destination_request: DestinationRequestV2,
    pub mapping: MappingIdentityV1,
    pub comparison_digest: DigestV1,
    pub destination_requirements: DestinationRequirementsV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Closed persisted variants stay direct to preserve the Rust API.
pub enum DryRunOutcomeV2 {
    Admitted(DryRunAdmittedV2),
    Refused(DryRunRefusedV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DryRunResultV2 {
    pub format: DryRunFormatV2,
    pub outcome: DryRunOutcomeV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyCompleteV1 {
    pub receipt: MigrationReceiptV1,
    pub current: AuthorityObservationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyRecoverableV1 {
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub last_proved_phase: MigrationPhaseV1,
    pub authority: PresenceV1<AuthorityCoordinateV1>,
    pub original_receipt: PresenceV1<MigrationReceiptV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyUncertainV1 {
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub last_proved_phase: PresenceV1<MigrationPhaseV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyRefusedV1 {
    pub migration_id: MigrationIdV1,
    pub last_proved_phase: PresenceV1<MigrationPhaseV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ApplyOutcomeV1 {
    Complete(ApplyCompleteV1),
    Recoverable(ApplyRecoverableV1),
    Uncertain(ApplyUncertainV1),
    Refused(ApplyRefusedV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyResultV1 {
    pub format: ApplyFormatV1,
    pub outcome: ApplyOutcomeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerifiedV1 {
    pub selection: SelectionV1,
    pub authority: AuthorityObservationV1,
    pub projection: ProjectionObservationV1,
    pub import_comparison_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationMismatchV1 {
    pub selection: SelectionV1,
    pub authority: PresenceV1<AuthorityObservationV1>,
    pub projection: PresenceV1<ProjectionObservationV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationRefusedV1 {
    pub selection: PresenceV1<SelectionV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum VerificationOutcomeV1 {
    Verified(VerifiedV1),
    Mismatch(VerificationMismatchV1),
    Refused(VerificationRefusedV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationResultV1 {
    pub format: VerificationFormatV1,
    pub outcome: VerificationOutcomeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RebuiltV1 {
    pub authority: AuthorityObservationV1,
    pub projection: ProjectionObservationV1,
    pub replaced_owned_paths: u64,
    pub preserved_foreign_paths: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RebuildRefusedV1 {
    pub requested_snapshot: AuthoritySnapshotIdV1,
    pub current_snapshot: PresenceV1<AuthoritySnapshotIdV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RebuildUncertainV1 {
    pub requested_snapshot: AuthoritySnapshotIdV1,
    pub staged_inventory_digest: ProjectionInventoryDigestV1,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Closed persisted variants stay direct to preserve the Rust API.
pub enum RebuildOutcomeV1 {
    Rebuilt(RebuiltV1),
    Refused(RebuildRefusedV1),
    Uncertain(RebuildUncertainV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RebuildResultV1 {
    pub format: RebuildFormatV1,
    pub outcome: RebuildOutcomeV1,
}

/// Immutable migration intent persisted before provider effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrationIntentV1 {
    pub format: MigrationIntentFormatV1,
    pub migration_id: MigrationIdV1,
    pub source_snapshot: SourceSnapshotIdV1,
    pub selector_digest: DigestV1,
    pub config_digest: DigestV1,
    pub source_coordinate: SourceCoordinateV1,
    pub migration_root: HostPathV1,
    pub staging_path: HostPathV1,
    pub destination_path: HostPathV1,
    pub projection_path: HostPathV1,
    pub authority: AuthorityCoordinateV1,
    pub mapping: MappingIdentityV1,
    pub intended_selector_bytes: HexBytesV1,
    pub intended_selector_digest: DigestV1,
    pub intent_digest: IntentDigestV1,
}

/// Immutable provider-owned-identity migration request persisted before provider effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrationIntentV2 {
    pub format: MigrationIntentFormatV2,
    pub migration_id: MigrationIdV1,
    pub source_snapshot: SourceSnapshotIdV1,
    pub selector_digest: DigestV1,
    pub config_digest: DigestV1,
    pub source_coordinate: SourceCoordinateV1,
    pub migration_root: HostPathV1,
    pub staging_path: HostPathV1,
    pub destination_path: HostPathV1,
    pub projection_path: HostPathV1,
    pub destination_request: DestinationRequestV2,
    pub mapping: MappingIdentityV1,
    pub selector_version: ProjectVersionV1,
    pub intent_digest: IntentDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnershipMarkerV1 {
    pub format: OwnershipMarkerFormatV1,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub legacy_source: SourceCoordinateV1,
    pub projection_path: HostPathV1,
    pub intended_selector_digest: DigestV1,
    pub marker_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnershipMarkerV2 {
    pub format: OwnershipMarkerFormatV2,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub legacy_source: SourceCoordinateV1,
    pub staging_path: HostPathV1,
    pub destination_path: HostPathV1,
    pub projection_path: HostPathV1,
    pub destination_request: DestinationRequestV2,
    pub marker_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhysicalRefV1 {
    pub event_id: String,
    pub global_seq: u64,
    pub stream_id: String,
    pub stream_version: u64,
}

macro_rules! phase_observation_struct {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            $(pub $field: $ty,)*
        }
    };
}

phase_observation_struct!(SelectorPhaseObservationV1 { digest: DigestV1 });
phase_observation_struct!(ConfigPhaseObservationV1 { digest: DigestV1 });
phase_observation_struct!(SourceCapturePhaseObservationV1 {
    snapshot_id: SourceSnapshotIdV1,
    capture_digest: DigestV1,
});
phase_observation_struct!(WriterControlPhaseObservationV1 {
    source_digest: DigestV1,
    selector_digest: DigestV1,
});
phase_observation_struct!(BindingPhaseObservationV1 {
    authority: AuthorityCoordinateV1,
    physical: PhysicalRefV1,
    replayed: bool,
});
phase_observation_struct!(DestinationPreconditionV2 {
    staging_path: HostPathV1,
    destination_path: HostPathV1,
    projection_path: HostPathV1,
    staging_absent: bool,
    destination_absent: bool,
    projection_safe: bool,
});
phase_observation_struct!(BindingPhaseObservationV2 {
    intent_digest: IntentDigestV1,
    authority: AuthorityCoordinateV1,
    physical: PhysicalRefV1,
    replayed: bool,
    intended_selector_bytes: HexBytesV1,
    intended_selector_digest: DigestV1,
});
phase_observation_struct!(ImportedSubjectPhaseObservationV1 {
    subject: SubjectCoordinateV1,
    boundary_digest: DigestV1,
    replayed: bool,
});
phase_observation_struct!(CompleteSnapshotPhaseObservationV1 {
    snapshot_id: AuthoritySnapshotIdV1,
    capture_digest: DigestV1,
});
phase_observation_struct!(ProjectionPhaseObservationV1 {
    authority_snapshot: AuthoritySnapshotIdV1,
    inventory_digest: ProjectionInventoryDigestV1,
});
phase_observation_struct!(RenamePhaseObservationV1 {
    from: HostPathV1,
    to: HostPathV1,
    source_absent: bool,
    destination_present: bool,
    parents_synced: bool,
});
phase_observation_struct!(SelectorDurabilityPhaseObservationV1 {
    old_digest: DigestV1,
    new_digest: DigestV1,
    file_synced: bool,
    parent_synced: bool,
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MigrationPhaseObservationV1 {
    Selector(SelectorPhaseObservationV1),
    Config(ConfigPhaseObservationV1),
    SourceCapture(SourceCapturePhaseObservationV1),
    WriterControl(WriterControlPhaseObservationV1),
    Binding(BindingPhaseObservationV1),
    ImportedSubject(ImportedSubjectPhaseObservationV1),
    CompleteSnapshot(CompleteSnapshotPhaseObservationV1),
    Projection(ProjectionPhaseObservationV1),
    Rename(RenamePhaseObservationV1),
    SelectorDurability(SelectorDurabilityPhaseObservationV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MigrationPhaseObservationV2 {
    Selector(SelectorPhaseObservationV1),
    Config(ConfigPhaseObservationV1),
    SourceCapture(SourceCapturePhaseObservationV1),
    WriterControl(WriterControlPhaseObservationV1),
    DestinationPrecondition(DestinationPreconditionV2),
    Binding(BindingPhaseObservationV2),
    ImportedSubject(ImportedSubjectPhaseObservationV1),
    CompleteSnapshot(CompleteSnapshotPhaseObservationV1),
    Projection(ProjectionPhaseObservationV1),
    Rename(RenamePhaseObservationV1),
    SelectorDurability(SelectorDurabilityPhaseObservationV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhaseRecordV1 {
    pub format: PhaseRecordFormatV1,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub phase: MigrationPhaseV1,
    pub predecessor: PresenceV1<PhaseDigestV1>,
    pub observations: Vec<MigrationPhaseObservationV1>,
    pub phase_digest: PhaseDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CurrentPhaseV1 {
    pub format: CurrentPhaseFormatV1,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub phase: MigrationPhaseV1,
    pub phase_digest: PhaseDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhaseRecordV2 {
    pub format: PhaseRecordFormatV2,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub phase: MigrationPhaseV1,
    pub predecessor: PresenceV1<PhaseDigestV1>,
    pub observations: Vec<MigrationPhaseObservationV2>,
    pub phase_digest: PhaseDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CurrentPhaseV2 {
    pub format: CurrentPhaseFormatV2,
    pub migration_id: MigrationIdV1,
    pub intent_digest: IntentDigestV1,
    pub phase: MigrationPhaseV1,
    pub phase_digest: PhaseDigestV1,
}

impl InspectionResultV1 {
    /// Exit status fixed by the closed outcome.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, InspectionOutcomeV1::Observed(_))
    }
}

impl DryRunResultV1 {
    /// Exit status fixed by the closed outcome.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, DryRunOutcomeV1::Admitted(_))
    }
}

impl DryRunResultV2 {
    /// Exit status fixed by the closed outcome.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, DryRunOutcomeV2::Admitted(_))
    }
}

impl ApplyResultV1 {
    /// Exit status fixed by the closed outcome.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, ApplyOutcomeV1::Complete(_))
    }
}

impl VerificationResultV1 {
    /// Exit status fixed by the closed outcome.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, VerificationOutcomeV1::Verified(_))
    }
}

impl RebuildResultV1 {
    /// Exit status fixed by the closed outcome.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, RebuildOutcomeV1::Rebuilt(_))
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn version_one_readers_reject_every_version_two_envelope_literal() {
        let pairs = [
            (DryRunFormatV1::VALUE, DryRunFormatV2::VALUE),
            (
                MigrationIntentFormatV1::VALUE,
                MigrationIntentFormatV2::VALUE,
            ),
            (
                OwnershipMarkerFormatV1::VALUE,
                OwnershipMarkerFormatV2::VALUE,
            ),
            (PhaseRecordFormatV1::VALUE, PhaseRecordFormatV2::VALUE),
            (CurrentPhaseFormatV1::VALUE, CurrentPhaseFormatV2::VALUE),
        ];
        for (version_one, version_two) in pairs {
            assert_ne!(version_one, version_two);
        }
        assert!(serde_json::from_str::<DryRunFormatV1>(
            &serde_json::to_string(&DryRunFormatV2).expect("v2 dry-run literal")
        )
        .is_err());
        assert!(serde_json::from_str::<MigrationIntentFormatV1>(
            &serde_json::to_string(&MigrationIntentFormatV2).expect("v2 intent literal")
        )
        .is_err());
        assert!(serde_json::from_str::<OwnershipMarkerFormatV1>(
            &serde_json::to_string(&OwnershipMarkerFormatV2).expect("v2 ownership literal")
        )
        .is_err());
        assert!(serde_json::from_str::<PhaseRecordFormatV1>(
            &serde_json::to_string(&PhaseRecordFormatV2).expect("v2 phase literal")
        )
        .is_err());
        assert!(serde_json::from_str::<CurrentPhaseFormatV1>(
            &serde_json::to_string(&CurrentPhaseFormatV2).expect("v2 current literal")
        )
        .is_err());
    }

    #[test]
    fn version_one_format_bytes_remain_exact() {
        let fixtures = [
            (
                DryRunFormatV1::VALUE,
                "\"aep.planning-migration-dry-run/1\"",
            ),
            (
                MigrationIntentFormatV1::VALUE,
                "\"aep.planning-migration-intent/1\"",
            ),
            (
                OwnershipMarkerFormatV1::VALUE,
                "\"aep.planning-store-ownership/1\"",
            ),
            (
                PhaseRecordFormatV1::VALUE,
                "\"aep.planning-migration-phase/1\"",
            ),
            (
                CurrentPhaseFormatV1::VALUE,
                "\"aep.planning-migration-current/1\"",
            ),
        ];
        for (value, expected) in fixtures {
            assert_eq!(
                serde_json::to_string(value).expect("literal serialises"),
                expected
            );
        }
    }
}
