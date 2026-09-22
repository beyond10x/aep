//! Durable ordinary-mutation identities and receipts for Eventlog planning authority.

#![allow(missing_docs)]

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    AuthorityCoordinateV1, AuthoritySnapshotIdV1, CommandRefusalCodeV1, CommandRefusalV1,
    DiagnosticCoordinateV1, DigestV1, MigrationIdV1, ProjectionInventoryDigestV1, ReceiptDigestV1,
    SubjectCoordinateV1,
};

macro_rules! literal {
    ($name:ident, $value:literal) => {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name;
        impl $name {
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

literal!(
    InvocationReservationFormatV1,
    "aep.planning-invocation-reservation/1"
);
literal!(CommandReceiptFormatV1, "aep.planning-command-receipt/1");
literal!(
    InvocationReceiptFormatV1,
    "aep.planning-invocation-receipt/1"
);
literal!(MutationFormatV1, "aep.planning-mutation/1");

/// The identity of one complete CLI, served, or driven invocation.
pub type CommandIdentityV1 = MigrationIdV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedCommandStepV1 {
    pub step_index: u64,
    pub child_identity: String,
    pub child_request_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InvocationReservationV1 {
    pub format: InvocationReservationFormatV1,
    pub command_identity: CommandIdentityV1,
    pub authority: AuthorityCoordinateV1,
    pub request_digest: DigestV1,
    pub planned: Vec<PlannedCommandStepV1>,
    pub plan_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum BatchKeyV1 {
    SingleRecord(String),
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordKindV1 {
    Decision,
    Observation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordPositionV1 {
    pub subject: u64,
    pub store: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordReceiptV1 {
    pub record_id: String,
    pub subject: SubjectCoordinateV1,
    pub kind: RecordKindV1,
    pub revision: u64,
    pub position: RecordPositionV1,
    pub batch_key: BatchKeyV1,
    pub member_index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchReceiptV1 {
    pub key: BatchKeyV1,
    pub members: Vec<RecordReceiptV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CommitReceiptV1 {
    Single(RecordReceiptV1),
    Batch(BatchReceiptV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatedResultV1 {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub revision: u64,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MovedResultV1 {
    pub id: String,
    pub from: String,
    pub to: String,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelationResultV1 {
    pub id: String,
    pub relation: String,
    pub target: String,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BodyUpdatedResultV1 {
    pub id: String,
    pub revision: u64,
    pub body_digest: DigestV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldsSetResultV1 {
    pub id: String,
    pub revision: u64,
    pub fields: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScopeUpdatedResultV1 {
    pub id: String,
    pub revision: u64,
    pub scope_digest: DigestV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecordedResultV1 {
    pub id: String,
    pub evidence_id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PlanningMutationResultV1 {
    Created(CreatedResultV1),
    Moved(MovedResultV1),
    Related(RelationResultV1),
    Unrelated(RelationResultV1),
    BodyUpdated(BodyUpdatedResultV1),
    FieldsSet(FieldsSetResultV1),
    ScopeUpdated(ScopeUpdatedResultV1),
    EvidenceRecorded(EvidenceRecordedResultV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DurableCommandReceiptV1 {
    pub format: CommandReceiptFormatV1,
    pub command_identity: String,
    pub authority: AuthorityCoordinateV1,
    pub commit_receipt: CommitReceiptV1,
    pub result_digest: DigestV1,
    pub committed_authority_snapshot: AuthoritySnapshotIdV1,
    pub receipt_digest: ReceiptDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommittedMutationStepV1 {
    pub step_index: u64,
    pub child_identity: String,
    pub child_request_digest: DigestV1,
    pub receipt: DurableCommandReceiptV1,
    pub result: PlanningMutationResultV1,
    pub authority_snapshot: AuthoritySnapshotIdV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DurableInvocationReceiptV1 {
    pub format: InvocationReceiptFormatV1,
    pub command_identity: CommandIdentityV1,
    pub authority: AuthorityCoordinateV1,
    pub request_digest: DigestV1,
    pub reservation_receipt_digest: ReceiptDigestV1,
    pub planned: Vec<PlannedCommandStepV1>,
    pub committed_count: u64,
    pub invocation_receipt_digest: ReceiptDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationStoppedStepV1 {
    pub step_index: u64,
    pub child_identity: String,
    pub child_request_digest: DigestV1,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MutationStopV1 {
    Refused(MutationStoppedStepV1),
    Uncertain(MutationStoppedStepV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionPublishedV1 {
    pub authority_snapshot: AuthoritySnapshotIdV1,
    pub inventory_digest: ProjectionInventoryDigestV1,
    pub watermark_digest: DigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ProjectionCompletionV1 {
    Published(ProjectionPublishedV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionFailureReasonV1 {
    Io,
    OwnershipConflict,
    ForeignPath,
    InventoryMismatch,
    WatermarkFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectionFailureV1 {
    pub code: CommandRefusalCodeV1,
    pub at: DiagnosticCoordinateV1,
    pub reason: ProjectionFailureReasonV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationCompleteV1 {
    pub invocation_receipt: DurableInvocationReceiptV1,
    pub committed: Vec<CommittedMutationStepV1>,
    pub authority_snapshot: AuthoritySnapshotIdV1,
    pub projection: ProjectionCompletionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationPartialV1 {
    pub invocation_receipt: DurableInvocationReceiptV1,
    pub committed: Vec<CommittedMutationStepV1>,
    pub stopped: MutationStopV1,
    pub current_authority_snapshot: AuthoritySnapshotIdV1,
    pub projection: ProjectionCompletionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationProjectionFailureV1 {
    pub invocation_receipt: DurableInvocationReceiptV1,
    pub committed: Vec<CommittedMutationStepV1>,
    pub failed_step_index: u64,
    pub projection: ProjectionFailureV1,
    pub current_authority_snapshot: AuthoritySnapshotIdV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationRefusedV1 {
    pub command_identity: CommandIdentityV1,
    pub request_digest: DigestV1,
    pub step_index: u64,
    pub child_identity: String,
    pub child_request_digest: DigestV1,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationUncertainV1 {
    pub command_identity: CommandIdentityV1,
    pub request_digest: DigestV1,
    pub authority: AuthorityCoordinateV1,
    pub step_index: u64,
    pub child_identity: String,
    pub child_request_digest: DigestV1,
    pub committed: Vec<CommittedMutationStepV1>,
    pub refusals: Vec<CommandRefusalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PlanningMutationOutcomeV1 {
    Complete(MutationCompleteV1),
    Partial(MutationPartialV1),
    CommittedProjectionFailure(MutationProjectionFailureV1),
    Refused(MutationRefusedV1),
    Uncertain(MutationUncertainV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanningMutationEnvelopeV1 {
    pub format: MutationFormatV1,
    pub outcome: PlanningMutationOutcomeV1,
}

impl PlanningMutationEnvelopeV1 {
    /// Only a complete invocation is an exit-zero mutation.
    #[must_use]
    pub const fn success(&self) -> bool {
        matches!(self.outcome, PlanningMutationOutcomeV1::Complete(_))
    }

    /// Checks the cross-field invariants that serde cannot express.
    pub fn validate(&self) -> Result<(), MutationContractError> {
        match &self.outcome {
            PlanningMutationOutcomeV1::Complete(value) => {
                value.invocation_receipt.validate()?;
                validate_prefix(&value.invocation_receipt.planned, &value.committed)?;
                if value.committed.len() != value.invocation_receipt.planned.len() {
                    return Err(MutationContractError::IncompleteComplete);
                }
            }
            PlanningMutationOutcomeV1::Partial(value) => {
                value.invocation_receipt.validate()?;
                validate_prefix(&value.invocation_receipt.planned, &value.committed)?;
                if value.committed.is_empty() {
                    return Err(MutationContractError::EmptyCommittedPrefix);
                }
                let stopped = match &value.stopped {
                    MutationStopV1::Refused(value) | MutationStopV1::Uncertain(value) => value,
                };
                validate_stop(
                    stopped,
                    &value.invocation_receipt.planned,
                    value.committed.len(),
                )?;
            }
            PlanningMutationOutcomeV1::CommittedProjectionFailure(value) => {
                value.invocation_receipt.validate()?;
                validate_prefix(&value.invocation_receipt.planned, &value.committed)?;
                let failed_step = usize::try_from(value.failed_step_index)
                    .map_err(|_| MutationContractError::InvalidProjectionFailure)?;
                if value.committed.is_empty()
                    || failed_step >= value.committed.len()
                    || value.projection.code != CommandRefusalCodeV1::CommittedProjectionFailure
                    || !matches!(value.projection.at, DiagnosticCoordinateV1::Projection(_))
                {
                    return Err(MutationContractError::InvalidProjectionFailure);
                }
            }
            PlanningMutationOutcomeV1::Refused(value) => {
                if value.step_index != 0 || value.refusals.is_empty() {
                    return Err(MutationContractError::InvalidFirstRefusal);
                }
            }
            PlanningMutationOutcomeV1::Uncertain(value) => {
                validate_prefix_for_request(&value.committed, value.request_digest)?;
                if value.refusals.is_empty()
                    || usize::try_from(value.step_index) != Ok(value.committed.len())
                {
                    return Err(MutationContractError::InvalidStop);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MutationContractError {
    #[error("planned command steps must be a nonempty contiguous zero-based vector")]
    InvalidPlan,
    #[error("a child command identity is malformed or repeated")]
    InvalidChildIdentity,
    #[error("a provider receipt violates its immutable batch contract")]
    InvalidCommitReceipt,
    #[error("the committed steps are not the reservation's exact contiguous prefix")]
    InvalidCommittedPrefix,
    #[error("a complete outcome omitted planned commands")]
    IncompleteComplete,
    #[error("a partial or projection-failure outcome requires a committed prefix")]
    EmptyCommittedPrefix,
    #[error("a stopped step is not the first uncommitted reservation step")]
    InvalidStop,
    #[error("a refused outcome is not a nonempty step-zero refusal")]
    InvalidFirstRefusal,
    #[error("a committed projection failure has an invalid step or diagnostic")]
    InvalidProjectionFailure,
}

impl InvocationReservationV1 {
    pub fn validate(&self) -> Result<(), MutationContractError> {
        validate_plan(&self.planned)
    }
}

impl DurableInvocationReceiptV1 {
    pub fn validate(&self) -> Result<(), MutationContractError> {
        validate_plan(&self.planned)?;
        let committed_count = usize::try_from(self.committed_count)
            .map_err(|_| MutationContractError::InvalidCommittedPrefix)?;
        if committed_count > self.planned.len() {
            return Err(MutationContractError::InvalidCommittedPrefix);
        }
        Ok(())
    }
}

impl CommitReceiptV1 {
    pub fn validate(&self) -> Result<(), MutationContractError> {
        let (key, members): (&BatchKeyV1, &[RecordReceiptV1]) = match self {
            Self::Single(member) => (&member.batch_key, std::slice::from_ref(member)),
            Self::Batch(batch) => (&batch.key, &batch.members),
        };
        if members.is_empty() {
            return Err(MutationContractError::InvalidCommitReceipt);
        }
        let mut record_ids = std::collections::BTreeSet::new();
        for (index, member) in members.iter().enumerate() {
            if member.member_index != index as u64
                || &member.batch_key != key
                || !record_ids.insert(&member.record_id)
            {
                return Err(MutationContractError::InvalidCommitReceipt);
            }
        }
        if let BatchKeyV1::SingleRecord(record_id) = key {
            if members.len() != 1 || members[0].record_id != *record_id {
                return Err(MutationContractError::InvalidCommitReceipt);
            }
        }
        Ok(())
    }
}

fn validate_plan(planned: &[PlannedCommandStepV1]) -> Result<(), MutationContractError> {
    if planned.is_empty()
        || planned
            .iter()
            .enumerate()
            .any(|(index, step)| step.step_index != index as u64)
    {
        return Err(MutationContractError::InvalidPlan);
    }
    let mut identities = std::collections::BTreeSet::new();
    if planned.iter().any(|step| {
        !valid_child_identity(&step.child_identity) || !identities.insert(&step.child_identity)
    }) {
        return Err(MutationContractError::InvalidChildIdentity);
    }
    Ok(())
}

fn valid_child_identity(value: &str) -> bool {
    value.len() == 70
        && value.starts_with("child-")
        && value.as_bytes()[6..].iter().all(u8::is_ascii_hexdigit)
        && !value.as_bytes()[6..].iter().any(u8::is_ascii_uppercase)
}

fn validate_prefix(
    planned: &[PlannedCommandStepV1],
    committed: &[CommittedMutationStepV1],
) -> Result<(), MutationContractError> {
    if committed.len() > planned.len() {
        return Err(MutationContractError::InvalidCommittedPrefix);
    }
    for (index, step) in committed.iter().enumerate() {
        let plan = &planned[index];
        if step.step_index != index as u64
            || step.child_identity != plan.child_identity
            || step.child_request_digest != plan.child_request_digest
            || step.receipt.command_identity != step.child_identity
        {
            return Err(MutationContractError::InvalidCommittedPrefix);
        }
        step.receipt.commit_receipt.validate()?;
    }
    Ok(())
}

fn validate_prefix_for_request(
    committed: &[CommittedMutationStepV1],
    _request: DigestV1,
) -> Result<(), MutationContractError> {
    for (index, step) in committed.iter().enumerate() {
        if step.step_index != index as u64 || step.receipt.command_identity != step.child_identity {
            return Err(MutationContractError::InvalidCommittedPrefix);
        }
        step.receipt.commit_receipt.validate()?;
    }
    Ok(())
}

fn validate_stop(
    stopped: &MutationStoppedStepV1,
    planned: &[PlannedCommandStepV1],
    committed: usize,
) -> Result<(), MutationContractError> {
    let Some(plan) = planned.get(committed) else {
        return Err(MutationContractError::InvalidStop);
    };
    if stopped.refusals.is_empty()
        || stopped.step_index != committed as u64
        || stopped.child_identity != plan.child_identity
        || stopped.child_request_digest != plan.child_request_digest
    {
        return Err(MutationContractError::InvalidStop);
    }
    Ok(())
}
