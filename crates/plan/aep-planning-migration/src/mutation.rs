//! Durable reservation and retry ledger for every ordinary multi-command invocation.

#![allow(missing_docs)]

use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

#[allow(clippy::wildcard_imports)]
// This coordinator implements the complete closed mutation vocabulary.
use aep_contract::migration::*;
use entity_eventlog::{Authority, EventlogOperationContext};
use entity_store::asynchronous::{BatchKey, CommitReceipt, RecordKind, RecordReceipt};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest as _, Sha256};
use time::OffsetDateTime;

#[derive(Debug, thiserror::Error)]
pub enum MutationLedgerError {
    #[error("the command identity is already reserved for different request bytes")]
    IdentityConflict,
    #[error("a committed child does not match its immutable planned step")]
    StepConflict,
    #[error("mutation ledger IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("mutation ledger encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("the authority mutation contract is invalid: {0}")]
    Contract(#[from] MutationContractError),
    #[error("Eventlog invocation authority failed: {0}")]
    Eventlog(String),
}

/// The closed ordinary-authority value stored for one invocation. A filesystem copy may aid
/// recovery, but only this subject admits or advances an invocation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InvocationAuthorityDocument {
    reservation: InvocationReservationV1,
    committed: Vec<CommittedMutationStepV1>,
    result: PresenceV1<InvocationBusinessResult>,
}

/// Durable business outcome written before a projection watermark. Projection evidence is joined
/// only in the returned public envelope, after this value and its complete authority snapshot are
/// stable; otherwise the result would need to contain a digest of itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum InvocationBusinessResult {
    Complete {
        invocation_receipt: DurableInvocationReceiptV1,
        committed: Vec<CommittedMutationStepV1>,
    },
    Partial {
        invocation_receipt: DurableInvocationReceiptV1,
        committed: Vec<CommittedMutationStepV1>,
        stopped: MutationStopV1,
    },
    Refused(MutationRefusedV1),
}

/// Eventlog-backed authoritative reservation and ordered receipt prefix.
pub struct EventlogMutationLedger {
    path: PathBuf,
    authority: AuthorityCoordinateV1,
    document: InvocationAuthorityDocument,
    revision: u64,
    reservation_receipt: CommitReceipt,
}

impl EventlogMutationLedger {
    #[allow(clippy::needless_pass_by_value)] // The owned reservation becomes durable ledger state.
    pub fn reserve(
        path: PathBuf,
        reservation: InvocationReservationV1,
    ) -> Result<Self, MutationLedgerError> {
        reservation.validate()?;
        let identity = reservation.command_identity.as_str();
        let authority = adapter_authority(&reservation.authority);
        let existing =
            aep_backend_eventlog::read_file_invocation(path.clone(), authority.clone(), identity)
                .map_err(MutationLedgerError::Eventlog)?;
        let (revision, document, receipt) = if let Some((revision, value, receipt)) = existing {
            let document: InvocationAuthorityDocument = serde_json::from_value(value)?;
            if document.reservation != reservation {
                return Err(MutationLedgerError::IdentityConflict);
            }
            (revision, document, receipt)
        } else {
            let document = InvocationAuthorityDocument {
                reservation: reservation.clone(),
                committed: Vec::new(),
                result: PresenceV1::Missing,
            };
            let receipt = aep_backend_eventlog::write_file_invocation(
                path.clone(),
                authority,
                identity.to_owned(),
                identity.to_owned(),
                serde_json::to_value(&document)?,
                None,
                operation_context(identity, "reserve"),
            )
            .map_err(MutationLedgerError::Eventlog)?;
            (1, document, receipt)
        };
        Ok(Self {
            path,
            authority: reservation.authority.clone(),
            document,
            revision,
            reservation_receipt: receipt,
        })
    }

    pub fn committed_prefix(&self) -> &[CommittedMutationStepV1] {
        &self.document.committed
    }

    pub fn reservation_receipt(&self) -> CommitReceiptV1 {
        contract_receipt(&self.reservation_receipt)
    }

    fn prior_result(&self) -> Option<&InvocationBusinessResult> {
        match &self.document.result {
            PresenceV1::Missing => None,
            PresenceV1::Present(value) => Some(value),
        }
    }

    pub fn record_committed(
        &mut self,
        step: CommittedMutationStepV1,
    ) -> Result<(), MutationLedgerError> {
        let index = self.document.committed.len();
        let planned = self
            .document
            .reservation
            .planned
            .get(index)
            .ok_or(MutationLedgerError::StepConflict)?;
        if step.step_index != index as u64
            || step.child_identity != planned.child_identity
            || step.child_request_digest != planned.child_request_digest
        {
            return Err(MutationLedgerError::StepConflict);
        }
        step.receipt.commit_receipt.validate()?;
        self.document.committed.push(step);
        self.advance(&format!("prefix-{index}"))
    }

    fn finish(&mut self, result: InvocationBusinessResult) -> Result<(), MutationLedgerError> {
        if let PresenceV1::Present(existing) = &self.document.result {
            return if existing == &result {
                Ok(())
            } else {
                Err(MutationLedgerError::IdentityConflict)
            };
        }
        self.document.result = PresenceV1::Present(result);
        self.advance("result")
    }

    fn advance(&mut self, suffix: &str) -> Result<(), MutationLedgerError> {
        let identity = self.document.reservation.command_identity.as_str();
        aep_backend_eventlog::write_file_invocation(
            self.path.clone(),
            adapter_authority(&self.authority),
            identity.to_owned(),
            format!("{identity}:{suffix}"),
            serde_json::to_value(&self.document)?,
            Some(self.revision),
            operation_context(identity, suffix),
        )
        .map_err(MutationLedgerError::Eventlog)?;
        self.revision += 1;
        Ok(())
    }
}

/// One invocation's immutable reservation and ordered committed prefix.
pub struct MutationLedger {
    root: PathBuf,
    reservation: InvocationReservationV1,
}

impl MutationLedger {
    /// Reserves the complete request before the first child command is admitted.
    pub fn reserve(
        authority_root: &Path,
        reservation: InvocationReservationV1,
    ) -> Result<Self, MutationLedgerError> {
        reservation.validate()?;
        let identity = digest(reservation.command_identity.as_str().as_bytes()).as_wire();
        let root = authority_root
            .join("invocations")
            .join(identity.replace(':', "-"));
        fs::create_dir_all(root.join("steps"))?;
        create_or_compare(
            &root.join("reservation.json"),
            &compact_line(&reservation)?,
            MutationLedgerError::IdentityConflict,
        )?;
        Ok(Self { root, reservation })
    }

    /// Returns the exact committed prefix. A missing index before a present later index refuses.
    pub fn committed_prefix(&self) -> Result<Vec<CommittedMutationStepV1>, MutationLedgerError> {
        let mut committed = Vec::new();
        for planned in &self.reservation.planned {
            let path = self.step_path(planned.step_index);
            if let Some(step) = read_json(&path)? {
                committed.push(step);
            } else {
                if self.reservation.planned.iter().any(|later| {
                    later.step_index > planned.step_index
                        && self.step_path(later.step_index).exists()
                }) {
                    return Err(MutationLedgerError::StepConflict);
                }
                break;
            }
        }
        Ok(committed)
    }

    /// Durably records one child only after the provider returned its immutable receipt.
    #[allow(clippy::needless_pass_by_value)] // The serialized step is the method's owned input.
    pub fn record_committed(
        &self,
        step: CommittedMutationStepV1,
    ) -> Result<(), MutationLedgerError> {
        step.receipt.commit_receipt.validate()?;
        let planned = self
            .reservation
            .planned
            .iter()
            .find(|planned| planned.step_index == step.step_index)
            .ok_or(MutationLedgerError::StepConflict)?;
        if planned.child_identity != step.child_identity {
            return Err(MutationLedgerError::StepConflict);
        }
        let prefix = self.committed_prefix()?;
        if step.step_index != prefix.len() as u64 {
            let step_index =
                usize::try_from(step.step_index).map_err(|_| MutationLedgerError::StepConflict)?;
            if prefix.get(step_index) == Some(&step) {
                return Ok(());
            }
            return Err(MutationLedgerError::StepConflict);
        }
        create_or_compare(
            &self.step_path(step.step_index),
            &compact_line(&step)?,
            MutationLedgerError::StepConflict,
        )
    }

    /// Persists the terminal envelope without erasing the committed prefix on failure.
    pub fn finish(&self, result: &PlanningMutationEnvelopeV1) -> Result<(), MutationLedgerError> {
        create_or_compare(
            &self.root.join("result.json"),
            &compact_line(result)?,
            MutationLedgerError::IdentityConflict,
        )
    }

    pub fn prior_result(&self) -> Result<Option<PlanningMutationEnvelopeV1>, MutationLedgerError> {
        read_json(&self.root.join("result.json"))
    }

    fn step_path(&self, index: u64) -> PathBuf {
        self.root.join("steps").join(format!("{index:020}.json"))
    }
}

/// Converts the provider receipt without dropping batch identity or physical positions.
pub fn contract_receipt(receipt: &CommitReceipt) -> CommitReceiptV1 {
    match receipt {
        CommitReceipt::Single(record) => CommitReceiptV1::Single(contract_record(record)),
        CommitReceipt::Batch(batch) => CommitReceiptV1::Batch(BatchReceiptV1 {
            key: contract_batch_key(&batch.key),
            members: batch.members.iter().map(contract_record).collect(),
        }),
    }
}

/// Builds the immutable reservation from exact invocation and child request bytes.
pub fn reservation(
    command_identity: CommandIdentityV1,
    authority: AuthorityCoordinateV1,
    request_bytes: &[u8],
    child_requests: &[Vec<u8>],
) -> Result<InvocationReservationV1, MutationLedgerError> {
    let request_digest = digest_parts_v1(
        "aep.planning-invocation-request/1",
        &[request_bytes.to_vec()],
    )
    .map_err(|_| MutationLedgerError::StepConflict)?;
    let mut planned = Vec::with_capacity(child_requests.len());
    for (index, bytes) in child_requests.iter().enumerate() {
        let child_request_digest = digest_parts_v1(
            "aep.planning-command-request/1",
            std::slice::from_ref(bytes),
        )
        .map_err(|_| MutationLedgerError::StepConflict)?;
        let child = digest_parts_v1(
            "aep.planning-command-child/1",
            &[
                command_identity.as_str().as_bytes().to_vec(),
                request_digest.as_bytes().to_vec(),
                (index as u64).to_be_bytes().to_vec(),
                child_request_digest.as_bytes().to_vec(),
            ],
        )
        .map_err(|_| MutationLedgerError::StepConflict)?;
        planned.push(PlannedCommandStepV1 {
            step_index: index as u64,
            child_identity: format!("child-{}", child.as_wire().trim_start_matches("sha256:")),
            child_request_digest,
        });
    }
    let plan_digest = digest_parts_v1(
        "aep.planning-invocation-plan/1",
        &[serde_json::to_vec(&planned)?],
    )
    .map_err(|_| MutationLedgerError::StepConflict)?;
    let value = InvocationReservationV1 {
        format: InvocationReservationFormatV1,
        command_identity,
        authority,
        request_digest,
        planned,
        plan_digest,
    };
    value.validate()?;
    Ok(value)
}

pub struct ExecutedChild {
    pub commit_receipt: CommitReceipt,
    pub result: PlanningMutationResultV1,
    pub authority_snapshot: AuthoritySnapshotIdV1,
}

pub enum ChildExecutionFailure {
    Refused(Vec<CommandRefusalV1>),
    Uncertain(Vec<CommandRefusalV1>),
}

/// Executes the first missing independent child and durably retains the exact prefix. Every
/// ledger write, including the terminal business result, precedes one final complete-snapshot
/// publication. Reopening joins that durable result to the existing/recovered watermark without
/// reexecuting a completed child.
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)] // One coordinator keeps child execution, durable prefix and publication order explicit.
pub fn execute_file_invocation<E, P>(
    path: PathBuf,
    reservation: InvocationReservationV1,
    mut execute: E,
    mut publish: P,
) -> Result<PlanningMutationEnvelopeV1, MutationLedgerError>
where
    E: FnMut(&PlannedCommandStepV1) -> Result<ExecutedChild, ChildExecutionFailure>,
    P: FnMut() -> Result<ProjectionPublishedV1, (AuthoritySnapshotIdV1, ProjectionFailureV1)>,
{
    let mut ledger = EventlogMutationLedger::reserve(path, reservation.clone())?;
    let reservation_receipt_digest = receipt_digest(&ledger.reservation_receipt())?;
    if let Some(result) = ledger.prior_result().cloned() {
        return publish_business_result(
            &reservation,
            reservation_receipt_digest,
            result,
            &mut publish,
        );
    }
    // A recovered committed prefix must have a current projection before a later child is
    // admitted. Repairing it uses the already durable child receipt and never reexecutes it.
    if let Some(last) = ledger.committed_prefix().last() {
        if let Err((snapshot, failure)) = publish() {
            return projection_failure_envelope(
                &reservation,
                reservation_receipt_digest,
                ledger.committed_prefix(),
                last.step_index,
                failure,
                snapshot,
            );
        }
    }

    while ledger.committed_prefix().len() < reservation.planned.len() {
        let plan = &reservation.planned[ledger.committed_prefix().len()];
        let executed = match execute(plan) {
            Ok(value) => value,
            Err(ChildExecutionFailure::Refused(refusals))
                if ledger.committed_prefix().is_empty() =>
            {
                let result = InvocationBusinessResult::Refused(MutationRefusedV1 {
                    command_identity: reservation.command_identity.clone(),
                    request_digest: reservation.request_digest,
                    step_index: plan.step_index,
                    child_identity: plan.child_identity.clone(),
                    child_request_digest: plan.child_request_digest,
                    refusals,
                });
                ledger.finish(result.clone())?;
                return publish_business_result(
                    &reservation,
                    reservation_receipt_digest,
                    result,
                    &mut publish,
                );
            }
            Err(ChildExecutionFailure::Refused(refusals)) => {
                let receipt = invocation_receipt(
                    &reservation,
                    reservation_receipt_digest,
                    ledger.committed_prefix().len(),
                )?;
                let stopped = MutationStoppedStepV1 {
                    step_index: plan.step_index,
                    child_identity: plan.child_identity.clone(),
                    child_request_digest: plan.child_request_digest,
                    refusals,
                };
                let result = InvocationBusinessResult::Partial {
                    invocation_receipt: receipt,
                    committed: ledger.committed_prefix().to_vec(),
                    stopped: MutationStopV1::Refused(stopped),
                };
                ledger.finish(result.clone())?;
                return publish_business_result(
                    &reservation,
                    reservation_receipt_digest,
                    result,
                    &mut publish,
                );
            }
            Err(ChildExecutionFailure::Uncertain(refusals)) => {
                // An uncertain child is deliberately not terminal in the invocation ledger.
                // Retrying calls the same child identity again so the provider either recovers
                // its immutable receipt or confirms refusal before any later child can run.
                let result = PlanningMutationEnvelopeV1 {
                    format: MutationFormatV1,
                    outcome: PlanningMutationOutcomeV1::Uncertain(MutationUncertainV1 {
                        command_identity: reservation.command_identity.clone(),
                        request_digest: reservation.request_digest,
                        authority: reservation.authority.clone(),
                        step_index: plan.step_index,
                        child_identity: plan.child_identity.clone(),
                        child_request_digest: plan.child_request_digest,
                        committed: ledger.committed_prefix().to_vec(),
                        refusals,
                    }),
                };
                result.validate()?;
                return Ok(result);
            }
        };
        let result_digest = digest_parts_v1(
            "aep.planning-command-result/1",
            &[serde_json::to_vec(&executed.result)?],
        )
        .map_err(|_| MutationLedgerError::StepConflict)?;
        let mut durable = DurableCommandReceiptV1 {
            format: CommandReceiptFormatV1,
            command_identity: plan.child_identity.clone(),
            authority: reservation.authority.clone(),
            commit_receipt: contract_receipt(&executed.commit_receipt),
            result_digest,
            committed_authority_snapshot: executed.authority_snapshot,
            receipt_digest: ReceiptDigestV1(DigestV1::from_bytes([0; 32])),
        };
        durable.receipt_digest = receipt_digest(&durable)?;
        let step = CommittedMutationStepV1 {
            step_index: plan.step_index,
            child_identity: plan.child_identity.clone(),
            child_request_digest: plan.child_request_digest,
            receipt: durable,
            result: executed.result,
            authority_snapshot: executed.authority_snapshot,
        };
        ledger.record_committed(step)?;
        if let Err((snapshot, failure)) = publish() {
            return projection_failure_envelope(
                &reservation,
                reservation_receipt_digest,
                ledger.committed_prefix(),
                plan.step_index,
                failure,
                snapshot,
            );
        }
    }
    let result = InvocationBusinessResult::Complete {
        invocation_receipt: invocation_receipt(
            &reservation,
            reservation_receipt_digest,
            ledger.committed_prefix().len(),
        )?,
        committed: ledger.committed_prefix().to_vec(),
    };
    ledger.finish(result.clone())?;
    publish_business_result(
        &reservation,
        reservation_receipt_digest,
        result,
        &mut publish,
    )
}

fn publish_business_result<P>(
    reservation: &InvocationReservationV1,
    reservation_receipt_digest: ReceiptDigestV1,
    business: InvocationBusinessResult,
    publish: &mut P,
) -> Result<PlanningMutationEnvelopeV1, MutationLedgerError>
where
    P: FnMut() -> Result<ProjectionPublishedV1, (AuthoritySnapshotIdV1, ProjectionFailureV1)>,
{
    let projection = match publish() {
        Ok(value) => value,
        Err((snapshot, failure)) => {
            let committed = match &business {
                InvocationBusinessResult::Complete { committed, .. }
                | InvocationBusinessResult::Partial { committed, .. } => committed.as_slice(),
                InvocationBusinessResult::Refused(_) => &[],
            };
            if let Some(last) = committed.last() {
                return projection_failure_envelope(
                    reservation,
                    reservation_receipt_digest,
                    committed,
                    last.step_index,
                    failure,
                    snapshot,
                );
            }
            let refusals = vec![CommandRefusalV1 {
                code: failure.code,
                at: failure.at,
            }];
            let result = PlanningMutationEnvelopeV1 {
                format: MutationFormatV1,
                outcome: PlanningMutationOutcomeV1::Uncertain(MutationUncertainV1 {
                    command_identity: reservation.command_identity.clone(),
                    request_digest: reservation.request_digest,
                    authority: reservation.authority.clone(),
                    step_index: 0,
                    child_identity: reservation.planned[0].child_identity.clone(),
                    child_request_digest: reservation.planned[0].child_request_digest,
                    committed: Vec::new(),
                    refusals,
                }),
            };
            result.validate()?;
            return Ok(result);
        }
    };
    let snapshot = projection.authority_snapshot;
    let projection = ProjectionCompletionV1::Published(projection);
    let outcome = match business {
        InvocationBusinessResult::Complete {
            invocation_receipt,
            committed,
        } => PlanningMutationOutcomeV1::Complete(MutationCompleteV1 {
            invocation_receipt,
            committed,
            authority_snapshot: snapshot,
            projection,
        }),
        InvocationBusinessResult::Partial {
            invocation_receipt,
            committed,
            stopped,
        } => PlanningMutationOutcomeV1::Partial(MutationPartialV1 {
            invocation_receipt,
            committed,
            stopped,
            current_authority_snapshot: snapshot,
            projection,
        }),
        InvocationBusinessResult::Refused(value) => PlanningMutationOutcomeV1::Refused(value),
    };
    let result = PlanningMutationEnvelopeV1 {
        format: MutationFormatV1,
        outcome,
    };
    result.validate()?;
    Ok(result)
}

fn invocation_receipt(
    reservation: &InvocationReservationV1,
    reservation_receipt_digest: ReceiptDigestV1,
    committed: usize,
) -> Result<DurableInvocationReceiptV1, MutationLedgerError> {
    let mut value = DurableInvocationReceiptV1 {
        format: InvocationReceiptFormatV1,
        command_identity: reservation.command_identity.clone(),
        authority: reservation.authority.clone(),
        request_digest: reservation.request_digest,
        reservation_receipt_digest,
        planned: reservation.planned.clone(),
        committed_count: committed as u64,
        invocation_receipt_digest: ReceiptDigestV1(DigestV1::from_bytes([0; 32])),
    };
    value.invocation_receipt_digest = receipt_digest(&value)?;
    Ok(value)
}

fn projection_failure_envelope(
    reservation: &InvocationReservationV1,
    reservation_receipt_digest: ReceiptDigestV1,
    committed: &[CommittedMutationStepV1],
    failed_step_index: u64,
    projection: ProjectionFailureV1,
    snapshot: AuthoritySnapshotIdV1,
) -> Result<PlanningMutationEnvelopeV1, MutationLedgerError> {
    let result = PlanningMutationEnvelopeV1 {
        format: MutationFormatV1,
        outcome: PlanningMutationOutcomeV1::CommittedProjectionFailure(
            MutationProjectionFailureV1 {
                invocation_receipt: invocation_receipt(
                    reservation,
                    reservation_receipt_digest,
                    committed.len(),
                )?,
                committed: committed.to_vec(),
                failed_step_index,
                projection,
                current_authority_snapshot: snapshot,
            },
        ),
    };
    result.validate()?;
    Ok(result)
}

fn receipt_digest(value: &impl Serialize) -> Result<ReceiptDigestV1, MutationLedgerError> {
    let mut value = serde_json::to_value(value)?;
    let object = value
        .as_object_mut()
        .ok_or(MutationLedgerError::StepConflict)?;
    let field = if object.contains_key("receipt_digest") {
        "receipt_digest"
    } else if object.contains_key("invocation_receipt_digest") {
        "invocation_receipt_digest"
    } else {
        return Ok(ReceiptDigestV1(
            digest_parts_v1(
                "aep.planning-reservation-receipt/1",
                &[serde_json::to_vec(&value)?],
            )
            .map_err(|_| MutationLedgerError::StepConflict)?,
        ));
    };
    object.remove(field);
    Ok(ReceiptDigestV1(
        digest_parts_v1("aep.planning-receipt/1", &[serde_json::to_vec(&value)?])
            .map_err(|_| MutationLedgerError::StepConflict)?,
    ))
}

fn adapter_authority(authority: &AuthorityCoordinateV1) -> Authority {
    Authority {
        logical_scope: authority.logical_scope.as_str().to_owned(),
        tenant: authority.tenant.as_str().to_owned(),
        stream_identity: authority.stream_identity.as_str().to_owned(),
    }
}

fn operation_context(identity: &str, step: &str) -> EventlogOperationContext {
    EventlogOperationContext {
        subject: "aep-planning-command".to_owned(),
        actor: "aep-planning-command".to_owned(),
        request_id: format!("{identity}:{step}"),
        trace_id: identity.to_owned(),
        causation_id: Some(identity.to_owned()),
        causation_depth: 1,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn contract_record(receipt: &RecordReceipt) -> RecordReceiptV1 {
    RecordReceiptV1 {
        record_id: receipt.record_id.clone(),
        subject: SubjectCoordinateV1 {
            entity: receipt.subject.entity.clone(),
            id: receipt.subject.id.clone(),
        },
        kind: match receipt.kind {
            RecordKind::Decision => RecordKindV1::Decision,
            RecordKind::Observation => RecordKindV1::Observation,
        },
        revision: receipt.revision,
        position: RecordPositionV1 {
            subject: receipt.position.subject,
            store: receipt.position.store,
        },
        batch_key: contract_batch_key(&receipt.batch_key),
        member_index: receipt.member_index,
    }
}

fn contract_batch_key(key: &BatchKey) -> BatchKeyV1 {
    match key {
        BatchKey::SingleRecord(value) => BatchKeyV1::SingleRecord(value.clone()),
        BatchKey::Named(value) => BatchKeyV1::Named(value.clone()),
    }
}

fn create_or_compare(
    path: &Path,
    bytes: &[u8],
    conflict: MutationLedgerError,
) -> Result<(), MutationLedgerError> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
            File::open(path.parent().ok_or(MutationLedgerError::StepConflict)?)?.sync_all()?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(path)? == bytes {
                Ok(())
            } else {
                Err(conflict)
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, MutationLedgerError> {
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicU64, Ordering};

    use entity_store::asynchronous::{
        BatchKey, CommitReceipt, RecordKind, RecordPosition, RecordReceipt, Subject,
    };

    use super::*;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        path: PathBuf,
        authority: AuthorityCoordinateV1,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "aep-mutation-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            let tenant = "tenant-test";
            let stream_identity = aep_backend_eventlog::prepare_file(&path, tenant)
                .expect("prepare a disposable file provider");
            let authority = AuthorityCoordinateV1 {
                logical_scope: AuthorityValueV1::new("planning-test").expect("scope"),
                tenant: AuthorityValueV1::new(tenant).expect("tenant"),
                stream_identity: AuthorityValueV1::new(stream_identity).expect("stream"),
            };
            aep_backend_eventlog::provision_file(
                &path,
                authority.logical_scope.as_str().to_owned(),
                authority.tenant.as_str().to_owned(),
                authority.stream_identity.as_str().to_owned(),
                operation_context(name, "binding"),
            )
            .expect("provision a disposable recorded authority");
            Self { path, authority }
        }

        fn reservation(&self, identity: &str) -> InvocationReservationV1 {
            reservation(
                MigrationIdV1::new(identity).expect("identity"),
                self.authority.clone(),
                br#"{"verb":"via","to":"done"}"#,
                &[br#"{"to":"middle"}"#.to_vec(), br#"{"to":"done"}"#.to_vec()],
            )
            .expect("valid immutable reservation")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn executed(step: &PlannedCommandStepV1) -> ExecutedChild {
        let subject =
            Subject::new("aep.test-step", format!("step-{}", step.step_index)).expect("subject");
        let key = BatchKey::Named(step.child_identity.clone());
        ExecutedChild {
            commit_receipt: CommitReceipt::Single(RecordReceipt {
                record_id: format!("record-{}", step.step_index),
                subject,
                kind: RecordKind::Decision,
                revision: 1,
                position: RecordPosition {
                    subject: 1,
                    store: step.step_index + 1,
                },
                batch_key: key,
                member_index: 0,
            }),
            result: PlanningMutationResultV1::FieldsSet(FieldsSetResultV1 {
                id: "story:test".to_owned(),
                revision: step.step_index + 2,
                fields: vec![format!("step-{}", step.step_index)],
            }),
            authority_snapshot: AuthoritySnapshotIdV1(DigestV1::from_bytes(
                [u8::try_from(step.step_index).expect("test step index fits u8") + 1; 32],
            )),
        }
    }

    fn executed_with_receipt(
        step: &PlannedCommandStepV1,
        commit_receipt: CommitReceipt,
    ) -> ExecutedChild {
        let mut child = executed(step);
        child.commit_receipt = commit_receipt;
        child
    }

    fn published(marker: u8) -> ProjectionPublishedV1 {
        ProjectionPublishedV1 {
            authority_snapshot: AuthoritySnapshotIdV1(DigestV1::from_bytes([marker; 32])),
            inventory_digest: ProjectionInventoryDigestV1(DigestV1::from_bytes([marker + 1; 32])),
            watermark_digest: DigestV1::from_bytes([marker + 2; 32]),
        }
    }

    fn projection_failure(_authority: &AuthorityCoordinateV1) -> ProjectionFailureV1 {
        ProjectionFailureV1 {
            code: CommandRefusalCodeV1::CommittedProjectionFailure,
            at: DiagnosticCoordinateV1::Projection(PathDiagnosticV1 {
                path: HostPathV1::Unix(HexBytesV1::new(b"projection".to_vec())),
            }),
            reason: ProjectionFailureReasonV1::Io,
        }
    }

    #[test]
    #[allow(clippy::result_large_err)]
    fn projection_failure_after_child_zero_stops_child_one_and_retry_repairs_before_resuming() {
        let fixture = Fixture::new("projection-stop");
        let reservation = fixture.reservation("projection-stop");
        let mut executions = [0_u64; 2];
        let mut publications = 0_u64;
        let first = execute_file_invocation(
            fixture.path.clone(),
            reservation.clone(),
            |step| {
                executions
                    [usize::try_from(step.step_index).expect("test step index fits usize")] += 1;
                Ok(executed(step))
            },
            || {
                publications += 1;
                Err((
                    AuthoritySnapshotIdV1(DigestV1::from_bytes([9; 32])),
                    projection_failure(&fixture.authority),
                ))
            },
        )
        .expect("the committed failure is a typed outcome");
        assert!(matches!(
            first.outcome,
            PlanningMutationOutcomeV1::CommittedProjectionFailure(MutationProjectionFailureV1 {
                failed_step_index: 0,
                ..
            })
        ));
        assert_eq!(
            executions,
            [1, 0],
            "child one must not cross a failed publication"
        );
        assert_eq!(publications, 1);

        let mut repair_publications = 0_u8;
        let order = RefCell::new(Vec::new());
        let repaired = execute_file_invocation(
            fixture.path.clone(),
            reservation,
            |step| {
                executions
                    [usize::try_from(step.step_index).expect("test step index fits usize")] += 1;
                order
                    .borrow_mut()
                    .push(format!("execute-{}", step.step_index));
                Ok(executed(step))
            },
            || {
                repair_publications += 1;
                order
                    .borrow_mut()
                    .push(format!("publish-{repair_publications}"));
                Ok(published(20 + repair_publications))
            },
        )
        .expect("the retry repairs the prefix and completes");
        assert!(repaired.success());
        assert_eq!(
            executions,
            [1, 1],
            "repair must not repeat committed child zero"
        );
        assert_eq!(
            repair_publications, 3,
            "repair prefix, publish child one, then publish the durable terminal result"
        );
        assert_eq!(
            order.into_inner(),
            ["publish-1", "execute-1", "publish-2", "publish-3"],
            "the recovered prefix is published before child one is admitted, followed by that \
             child's publication and the final durable-result publication"
        );
    }

    #[test]
    #[allow(clippy::result_large_err, clippy::too_many_lines)]
    fn uncertain_child_retries_the_same_identity_before_admitting_the_next_child() {
        let fixture = Fixture::new("uncertain-retry");
        let reservation = fixture.reservation("uncertain-retry");
        let expected_identity = reservation.planned[0].child_identity.clone();
        let mut executions = [0_u64; 2];
        let mut identities = Vec::new();
        let mut business_receipts = Vec::new();
        let first = execute_file_invocation(
            fixture.path.clone(),
            reservation.clone(),
            |step| {
                executions
                    [usize::try_from(step.step_index).expect("test step index fits usize")] += 1;
                identities.push(step.child_identity.clone());
                let receipt = aep_backend_eventlog::write_file_control(
                    fixture.path.clone(),
                    adapter_authority(&fixture.authority),
                    aep_backend_eventlog::INVOCATION_AS,
                    step.child_identity.clone(),
                    step.child_identity.clone(),
                    serde_json::json!({"effect":"child-zero"}),
                    None,
                    operation_context(&step.child_identity, "business"),
                )
                .expect("the business child committed before its reply was lost");
                business_receipts.push(receipt);
                Err::<ExecutedChild, _>(ChildExecutionFailure::Uncertain(vec![CommandRefusalV1 {
                    code: CommandRefusalCodeV1::ReceiptConflict,
                    at: DiagnosticCoordinateV1::Authority(AuthorityDiagnosticV1 {
                        authority: fixture.authority.clone(),
                        subject: PresenceV1::Missing,
                        record_id: PresenceV1::Missing,
                    }),
                }]))
            },
            || panic!("an uncertain child has no admitted prefix to publish"),
        )
        .expect("uncertainty is returned without becoming terminal");
        assert!(matches!(
            first.outcome,
            PlanningMutationOutcomeV1::Uncertain(_)
        ));
        assert_eq!(executions, [1, 0]);

        let completed = execute_file_invocation(
            fixture.path.clone(),
            reservation,
            |step| {
                executions
                    [usize::try_from(step.step_index).expect("test step index fits usize")] += 1;
                identities.push(step.child_identity.clone());
                if step.step_index == 0 {
                    let receipt = aep_backend_eventlog::write_file_control(
                        fixture.path.clone(),
                        adapter_authority(&fixture.authority),
                        aep_backend_eventlog::INVOCATION_AS,
                        step.child_identity.clone(),
                        step.child_identity.clone(),
                        serde_json::json!({"effect":"child-zero"}),
                        None,
                        operation_context(&step.child_identity, "business"),
                    )
                    .expect("same child key recovers the original business receipt");
                    business_receipts.push(receipt.clone());
                    Ok(executed_with_receipt(step, receipt))
                } else {
                    Ok(executed(step))
                }
            },
            || Ok(published(40)),
        )
        .expect("same-identity replay resolves and advances");
        assert!(completed.success());
        assert_eq!(executions, [2, 1]);
        assert_eq!(identities[0], expected_identity);
        assert_eq!(
            identities[1], expected_identity,
            "retry must reuse the original child key"
        );
        assert_ne!(
            identities[2], expected_identity,
            "only then may child one execute"
        );
        assert_eq!(
            business_receipts.len(),
            2,
            "the provider was called twice to resolve uncertainty"
        );
        assert_eq!(
            business_receipts[0], business_receipts[1],
            "same child key must recover the immutable original receipt"
        );
        let (revision, _, recovered) = aep_backend_eventlog::read_file_control(
            fixture.path.clone(),
            adapter_authority(&fixture.authority),
            aep_backend_eventlog::INVOCATION_AS,
            &expected_identity,
        )
        .expect("read the committed business effect")
        .expect("business effect exists");
        assert_eq!(
            revision, 1,
            "uncertainty recovery made exactly one durable business commit"
        );
        assert_eq!(recovered, business_receipts[0]);
    }
}
