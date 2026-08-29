//! Who may answer an `operator` step, decided by the driver from what the record says.
//!
//! # The question, and why it is the driver's
//!
//! An `operator` step stops the run because the driver cannot *attest* an approver, not because
//! approval needs a person (`story:attested-approver`). The record already has the vocabulary:
//! an `Evidence::Approval` carries a `Producer`, and `Producer::Human` is one producer among
//! several. What was missing is the rule that reads it. The engine deliberately does not judge a
//! producer's independence from what it reports on — that refusal is recorded on
//! `story-completion-evidence-design-v0.1.md` § 10 and held under gap **D-S4** — so the rule lives
//! here, one layer up, as a per-run policy the operator states on the command line, exactly as
//! `decide_tool` is a per-call policy the driver states.
//!
//! # What is decided, and what is not
//!
//! * A **person** is admissible without being named. That route is unchanged: a person at a
//!   keyboard is an attestation of a kind, and the store records their approval as theirs.
//! * A **named agent** is admissible when the operator named it — `--approver agent:<name>` — and
//!   it is not one of this run's own actors. Two sessions of one model are two actors: the check
//!   compares identities, never roles (the story's second open question, default taken).
//! * **This run's own actors** are refused whoever named them: the execution, the task, and the
//!   harnesses its `llm` steps run under. A run that approved its own specification would satisfy
//!   a principle by writing to the document the principle is about.
//! * A **harness, tool or verifier** is never an approver: the first is this run's own actor and
//!   the other two do not approve things.
//!
//! **Nothing here verifies an identity.** The approver is whatever `producer` the record carries,
//! which is exactly as strong as the rest of the evidence model and no stronger; attestation by
//! signature is gap register **D-3** and stays proposed. What this module adds is that the record
//! is *read*, and that a run continuing past an approval says in its own cursor who granted it.
//!
//! # Why the driver never constructs an approval
//!
//! `crates/aep-driver/tests/evidence_scan.rs` refuses any construction of an `Evidence::Approval`
//! or a `Producer::Human` in this crate's shipped code. Everything below **reads** a record that
//! somebody else wrote and submitted while the run was stopped — `protocol evaluate --evidence`
//! against the run's snapshot — and decides whether the step the run stopped at may count it as
//! its answer. There is no flag that answers an operator step; there is a flag that says whose
//! answer counts.

use aep_domain::entity::ActorRef;
use aep_domain::evidence::Producer;
use aep_domain::ids::ExecutionId;

/// The actor a driven session acts as, and writes to the planning store as.
///
/// **One spelling, in one place, because two readers need the same answer.** [`crate::run`] puts
/// it in the set of actors an approval may not come from — a run cannot approve what it produced —
/// and `protocol-cli` hands the same string to every `llm` step's session in `AEP_ACTOR`, so a
/// `protocol artifact move` made from inside the run is journalled as the run's own act rather
/// than as the operator's. If the two spellings could drift, a run would be able to approve its
/// own work under a name its own refusal did not recognise, which is the one failure this module
/// exists to prevent.
///
/// `None` when the execution id cannot be spelled as an actor name: [`ExecutionId`] admits `/` and
/// [`ActorRef`] does not. A run with no actor to declare declares none, and the caller falls back
/// to whatever it did before — never to a name that is somebody else's.
pub fn session_actor(execution: &ExecutionId) -> Option<ActorRef> {
    ActorRef::parse(&format!("agent:{execution}")).ok()
}

/// Whether one approval's producer may answer an `operator` step of this run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    /// The producer may answer.
    Admitted,
    /// The producer may not, and this is why — one sentence, naming the producer.
    Refused {
        /// Why not.
        reason: String,
    },
}

impl Admission {
    /// `true` when the producer may answer.
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted)
    }
}

/// The actor a producer would be, in the vocabulary `--approver` is written in.
///
/// `None` for a producer that is not an actor at all — a tool, a verifier — and for the harness,
/// which is an actor but one this run can never accept an approval from.
pub fn actor_of(producer: &Producer) -> Option<ActorRef> {
    match producer {
        Producer::Human { id } => ActorRef::parse(&format!("human:{id}")).ok(),
        Producer::Agent { id } => ActorRef::parse(&format!("agent:{id}")).ok(),
        Producer::Harness { .. } | Producer::Tool { .. } | Producer::Verifier { .. } => None,
    }
}

/// Decides whether `producer` may answer an `operator` step.
///
/// `named` is the one non-human actor the operator admitted on the command line, if any; `own` is
/// every actor this run itself is. The order of the checks is the safety argument: being this
/// run's own actor refuses before being named admits, so naming the run itself buys nothing.
pub fn admit(producer: &Producer, named: Option<&ActorRef>, own: &[ActorRef]) -> Admission {
    let refused = |reason: String| Admission::Refused { reason };
    match producer {
        Producer::Human { .. } => Admission::Admitted,
        Producer::Harness { id } => refused(format!(
            "harness `{id}` is this run's own actor, and a run cannot attest its own work"
        )),
        Producer::Tool { tool } => refused(format!("tool `{tool}` is not an approver")),
        Producer::Verifier { verifier } => {
            refused(format!("verifier `{verifier}` is not an approver"))
        }
        Producer::Agent { id } => {
            let Some(actor) = actor_of(producer) else {
                return refused(format!(
                    "agent `{id}` is not spelled as an actor this driver can name"
                ));
            };
            if own.contains(&actor) {
                return refused(format!(
                    "`{actor}` is this run's own actor, and a run cannot approve what it produced"
                ));
            }
            match named {
                Some(named) if *named == actor => Admission::Admitted,
                Some(named) => refused(format!(
                    "`{actor}` was not named: this run admits `{named}` (`--approver`) and a person"
                )),
                None => refused(format!(
                    "`{actor}` was not named: this run admits a person only — pass `--approver \
                     {actor}` to admit it"
                )),
            }
        }
    }
}

/// One line saying who would be admissible, for a refusal that has to answer the question it
/// creates.
pub fn admissible(named: Option<&ActorRef>) -> String {
    match named {
        Some(named) => format!(
            "a person (`producer: human`), or `{named}` (`producer: agent`, `id: {}`)",
            named.name()
        ),
        None => "a person (`producer: human`); an agent is admissible only when named with \
                 `--approver agent:<name>`"
            .to_owned(),
    }
}

/// Why an actor may not be named as this run's approver, or `None` when it may.
///
/// Checked at launch, before a run id, a lock and a model bill exist: a person needs no naming,
/// `system` is nobody, a `service` is a producer no evidence record can carry, and the run's own
/// actors are refused for the reason [`admit`] gives.
pub fn naming_refusal(named: &ActorRef, own: &[ActorRef]) -> Option<String> {
    match named {
        ActorRef::Human(_) => Some(format!(
            "`{named}` needs no naming: a person's approval is admissible on every run"
        )),
        ActorRef::System => {
            Some("`system` is nobody, and an approval has to come from somebody".to_owned())
        }
        ActorRef::Service(_) => Some(format!(
            "`{named}` can never answer: an evidence record's producer is a person, an agent, a \
             harness, a tool or a verifier, and a service is none of them — name an `agent:`"
        )),
        ActorRef::Agent(_) if own.contains(named) => Some(format!(
            "`{named}` is this run's own actor, and a run cannot approve what it produced"
        )),
        ActorRef::Agent(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aep_domain::ids::ToolRef;
    use aep_domain::verification::Verifier;

    fn agent(name: &str) -> Producer {
        Producer::Agent {
            id: name.to_owned(),
        }
    }

    fn actor(text: &str) -> ActorRef {
        ActorRef::parse(text).expect("a legal actor")
    }

    // A person's admission is asserted in `tests/attested.rs`: this file is shipped driver code,
    // and the evidence scan refuses a `Producer::Human` constructed anywhere in it — inline test
    // modules included, because the scan reads files and not cfgs.

    #[test]
    fn a_named_agent_is_admitted_and_an_unnamed_one_is_refused_naming_the_flag() {
        let named = actor("agent:orchestrator");
        assert!(admit(&agent("orchestrator"), Some(&named), &[]).is_admitted());

        let Admission::Refused { reason } = admit(&agent("orchestrator"), None, &[]) else {
            panic!("an agent nobody named is refused");
        };
        assert!(reason.contains("--approver agent:orchestrator"), "{reason}");

        let Admission::Refused { reason } = admit(&agent("someone-else"), Some(&named), &[]) else {
            panic!("an agent other than the named one is refused");
        };
        assert!(
            reason.contains("agent:orchestrator"),
            "names who is admitted: {reason}"
        );
    }

    #[test]
    fn the_runs_own_actor_is_refused_even_when_it_was_named() {
        // The load-bearing case: naming the run itself must buy nothing, so the fixture names it.
        let own = [actor("agent:T-1.1"), actor("agent:b10x")];
        let named = actor("agent:T-1.1");
        let Admission::Refused { reason } = admit(&agent("T-1.1"), Some(&named), &own) else {
            panic!("a run cannot approve what it produced");
        };
        assert!(reason.contains("own actor"), "{reason}");
        let Admission::Refused { reason } = admit(&agent("b10x"), Some(&named), &own) else {
            panic!("the harness the run's sessions ran under is the run");
        };
        assert!(reason.contains("agent:b10x"), "{reason}");
    }

    #[test]
    fn a_harness_a_tool_and_a_verifier_never_approve() {
        let named = actor("agent:b10x");
        for producer in [
            Producer::Harness {
                id: "b10x".to_owned(),
            },
            Producer::Tool {
                tool: ToolRef::new("cargo").expect("a tool"),
            },
            Producer::Verifier {
                verifier: Verifier::TestRunner,
            },
        ] {
            assert!(
                !admit(&producer, Some(&named), &[]).is_admitted(),
                "{producer} is not an approver"
            );
        }
    }

    #[test]
    fn naming_a_person_the_system_a_service_or_the_run_itself_is_refused_at_launch() {
        let own = [actor("agent:T-1.1")];
        for text in [
            "human:alice",
            "system",
            "service:release-controller",
            "agent:T-1.1",
        ] {
            let refusal = naming_refusal(&actor(text), &own).expect("refused");
            assert!(refusal.contains(text) || text == "system", "{refusal}");
        }
        assert_eq!(naming_refusal(&actor("agent:orchestrator"), &own), None);
    }

    #[test]
    fn the_actor_a_session_writes_under_is_the_actor_its_own_approval_is_refused_under() {
        // The agreement the two readers of `session_actor` depend on, asserted on one value: the
        // string handed to a session in `AEP_ACTOR` and the string `admit` refuses as this run's
        // own must be the same, or a run could approve its own work under the name it writes
        // under. The fixture reaches that state — the producer *is* the session's own actor, and
        // it is named as the approver besides — before asserting the refusal.
        let execution = ExecutionId::new("T-1.1").expect("an execution id");
        let session = session_actor(&execution).expect("`T-1.1` spells an actor name");
        assert_eq!(session.to_string(), "agent:T-1.1");

        let own = [session.clone()];
        let Admission::Refused { reason } = admit(&agent(session.name()), Some(&session), &own)
        else {
            panic!("the actor a session writes under cannot approve that session's work");
        };
        assert!(
            reason.contains("agent:T-1.1") && reason.contains("own actor"),
            "{reason}"
        );
    }

    #[test]
    fn an_execution_id_that_cannot_be_spelled_as_an_actor_declares_none() {
        // `ExecutionId` admits `/` and an actor name does not, so the answer is *no actor* rather
        // than a mangled one: a run that cannot say who it is must not say it is somebody else.
        let slashed = ExecutionId::new("T-1/1").expect("an execution id may carry a slash");
        assert_eq!(session_actor(&slashed), None);
    }

    #[test]
    fn the_admissible_line_names_the_flag_or_the_named_agent() {
        assert!(admissible(None).contains("--approver agent:<name>"));
        let line = admissible(Some(&actor("agent:orchestrator")));
        assert!(
            line.contains("agent:orchestrator") && line.contains("orchestrator"),
            "{line}"
        );
    }
}
