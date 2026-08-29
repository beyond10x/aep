//! Engine errors.
//!
//! Resolution failures are [`ValidationErrors`] — a document set that cannot be executed. Everything
//! else here is a runtime refusal, and each variant exists so a harness can branch on *why* rather
//! than parse a message.

use aep_domain::error::ValidationErrors;
use aep_domain::evidence::EvidenceKind;
use aep_domain::ids::{StateId, SubjectRef, TaskId};

/// Why the engine refused.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// The documents cannot be resolved into an executable plan.
    #[error("the task cannot be resolved: {0}")]
    Resolution(#[from] ValidationErrors),

    /// The execution refers to a state its workflow does not declare.
    ///
    /// Only reachable by restoring a snapshot taken against a different workflow version.
    #[error("state `{state}` is not part of workflow `{workflow}`")]
    UnknownState {
        /// The state that is missing.
        state: StateId,
        /// The workflow it was expected in.
        workflow: String,
    },

    /// Evidence was submitted that the protocol does not declare.
    ///
    /// Accepting it would let a document set grow a private vocabulary that no other harness can
    /// interpret, so it is refused rather than stored.
    #[error(
        "the protocol does not declare evidence of kind `{kind}`; declared kinds are {declared}"
    )]
    EvidenceRejected {
        /// The kind that was refused.
        kind: EvidenceKind,
        /// What the protocol does declare.
        declared: String,
    },

    /// Evidence was submitted about something other than what the task is about.
    ///
    /// # The failure this is named after
    ///
    /// An end-to-end job held a legacy service while a deployment rolled its successor, and produced
    /// weeks of green about a component nobody was shipping. Every record was true. Every record was
    /// about the wrong thing, and nothing in the loop could say so — the approvals rule already
    /// refuses a record bound to the wrong *revision*, and there was no analogue for the wrong
    /// *subject*.
    ///
    /// Both names are printed, because the whole content of the refusal is the difference between
    /// them and a message naming one of them is a message the reader has to go and complete.
    #[error(
        "this evidence is about `{observed}`, and {task} is about `{declared}`; \
         a fact observed of one thing does not move another"
    )]
    EvidenceSubjectMismatch {
        /// What the evidence was observed of.
        observed: SubjectRef,
        /// What the task declared it is about.
        declared: SubjectRef,
        /// Which task.
        task: TaskId,
    },

    /// A task declared what it is about, and evidence arrived naming nothing.
    ///
    /// Refused rather than admitted, and that is the deliberate half. Admitting it would leave the
    /// exact hole this pair of errors exists to close: unsubjected evidence would remain the way to
    /// move a task whose subject you cannot match, so the guard would be optional in practice while
    /// looking mandatory. A task that declares no subject is unaffected — see
    /// [`aep_domain::task::Task::subject`].
    #[error(
        "{task} is about `{declared}` and this evidence names no subject; \
         say what it was observed of"
    )]
    EvidenceSubjectMissing {
        /// What the task declared it is about.
        declared: SubjectRef,
        /// Which task.
        task: TaskId,
    },

    /// Evidence was submitted claiming an observation that has not happened yet.
    ///
    /// One comparison, and it is the cheapest guard in the engine. A planned re-check is a
    /// different object from a decaying observation, and storing the first as the second makes it
    /// the *freshest* record in the log — a negative age inflates the remaining horizon, and the
    /// store can no longer answer whether anybody has ever looked. Refusing it is what makes the
    /// conflation unwritable rather than merely discouraged.
    ///
    /// The message names the observation **the way the caller wrote it** — a date for a date, an
    /// epoch value for an epoch value — and the clock as a readable instant beside its millisecond
    /// count. An adopter pasted `the observation time 1787961600000ms is in the future; it is
    /// 1787956053626ms` and could not tell which of 215 records it was about, nor that the two
    /// values are ninety minutes apart.
    #[error("the observation time {observed_at} is in the future; the clock reads {} ({now})", now.iso_8601())]
    ObservationInFuture {
        /// What the submission claimed.
        observed_at: aep_domain::time::ObservedAt,
        /// What the engine's clock says.
        now: aep_domain::time::Timestamp,
    },

    /// No transition out of the current state is permitted.
    #[error("no transition out of `{state}` is permitted: {}", reasons.join("; "))]
    NoTransitionPermitted {
        /// Where the execution is stuck.
        state: StateId,
        /// One line per unmet requirement.
        reasons: Vec<String>,
    },

    /// The execution is already in a terminal state.
    #[error("the execution is already complete in `{state}`")]
    AlreadyComplete {
        /// The terminal state.
        state: StateId,
    },
}

impl ProtocolError {
    /// A stable machine-readable code, for a harness that reports rather than branches.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Resolution(_) => "resolution_failed",
            Self::UnknownState { .. } => "unknown_state",
            Self::EvidenceRejected { .. } => "evidence_rejected",
            Self::EvidenceSubjectMismatch { .. } => "evidence_subject_mismatch",
            Self::EvidenceSubjectMissing { .. } => "evidence_subject_missing",
            Self::ObservationInFuture { .. } => "observation_in_future",
            Self::NoTransitionPermitted { .. } => "no_transition_permitted",
            Self::AlreadyComplete { .. } => "already_complete",
        }
    }
}
