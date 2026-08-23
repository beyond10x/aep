//! Which principles bind a workflow, and where each of their obligations lands.
//!
//! # The join this module exists to make
//!
//! A principle does not name a state. It times its obligations against **phases** — `before
//! implementation`, `during observation` — which is what lets one principle hold for a development
//! workflow, a release workflow and an incident workflow without being rewritten. A workflow's
//! states declare the phases they belong to. Neither document mentions the other, and the sentence
//! a person actually needs — *you may not enter `implement` until a failing test exists* — is the
//! join of the two.
//!
//! [`Obligations::of`] makes that join and nothing else. It resolves each timing to the states of
//! *this* workflow it reaches, and where a timing reaches none of them, the obligation is reported
//! as owed elsewhere rather than quietly dropped or silently applied.
//!
//! # It decides nothing
//!
//! Like the rest of this crate, this module never evaluates. It does not ask whether a principle
//! applies to a task, whether an obligation is met or whether a capability is available — that is
//! the engine's work, over facts this crate cannot see. A principle's `applies_when` is carried
//! through as **text**, so a reader is told the condition rather than an answer to it.
//!
//! # What counts as binding
//!
//! Three clauses, because a principle can reach a workflow in three ways, and dropping any of them
//! loses an instruction a reader is entitled to:
//!
//! | clause | why it binds |
//! |---|---|
//! | an obligation or a verification requirement lands on a state | the workflow declares the phase it is timed against |
//! | it denies a capability, or puts one behind an approval | a withdrawal holds in every state of every workflow — `least-privilege` has no obligations at all and *you may not read a secret* is the whole of it |
//! | it requires evidence | evidence is owed by completion, and a workflow that can complete can owe it |

use std::collections::BTreeSet;

use aep_domain::ids::StateId;
use aep_domain::predicate::Predicate;
use aep_domain::principle::{ObligationTiming, PhaseRef, Principle};
use aep_domain::workflow::Workflow;

use crate::scene::requirement_lines;

/// When something is owed, and which of one workflow's states that turns out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    /// The timing as the principle states it, such as `before phase implementation`.
    pub timing: String,
    /// The phase it is timed against, when it is timed against a phase rather than a state.
    pub phase: Option<String>,
    /// The states of this workflow the timing reaches, in id order.
    pub states: Vec<StateId>,
    /// Whether it is owed at every transition, which is what [`ObligationTiming::Always`] means.
    pub everywhere: bool,
}

impl Landing {
    /// Where `timing` lands in `workflow`.
    pub fn of(workflow: &Workflow, timing: &ObligationTiming) -> Self {
        let (phase, states) = match timing.target() {
            None => (None, Vec::new()),
            Some(PhaseRef::Phase(phase)) => (
                Some(phase.to_string()),
                workflow
                    .states_with_phase(phase)
                    .iter()
                    .map(|state| state.id.clone())
                    .collect(),
            ),
            Some(PhaseRef::State(state)) => (
                None,
                workflow
                    .state(state)
                    .map(|declared| declared.id.clone())
                    .into_iter()
                    .collect(),
            ),
        };
        Self {
            timing: timing.to_string(),
            phase,
            states,
            everywhere: matches!(timing, ObligationTiming::Always),
        }
    }

    /// `true` when this workflow has somewhere for it to land.
    ///
    /// An obligation timed against a phase no state declares reaches nothing here: it is a rule for
    /// a different workflow, and writing it into these instructions would be telling a reader to
    /// wait for a state that does not exist.
    pub fn reaches(&self) -> bool {
        self.everywhere || !self.states.is_empty()
    }
}

/// One obligation of a principle, placed in one workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundObligation {
    /// Its identifier, which the principle document determines.
    pub id: String,
    /// What it is for, when the document says.
    pub description: Option<String>,
    /// When it is owed, and where that is here.
    pub landing: Landing,
    /// What must hold, one line per requirement.
    pub requires: Vec<String>,
}

/// One verifier a principle demands, placed in one workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundVerification {
    /// What the principle asks for, such as `test-runner must run`.
    pub statement: String,
    /// By when, and where that is here.
    pub landing: Landing,
}

/// One principle, as it bears on one workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundPrinciple {
    /// Its identifier.
    pub id: String,
    /// Its reference, as `<id>/<major>`.
    pub reference: String,
    /// Its human title.
    pub title: String,
    /// What it is for, when the document says.
    pub summary: Option<String>,
    /// The condition under which it applies, or `None` when it applies to everything.
    ///
    /// Text, never a verdict: this crate cannot see a task's facts, and a rendering that printed
    /// *applies* or *does not apply* would be answering a question the engine answers.
    pub applies_when: Option<String>,
    /// Its obligations that land in this workflow, in the order the principle declares them.
    pub obligations: Vec<BoundObligation>,
    /// The timings of its obligations that land nowhere here, deduplicated and sorted.
    ///
    /// Reported rather than dropped. A reader who knows a principle also governs `before phase
    /// mitigation` knows that this document is a view of it and not the whole of it.
    pub elsewhere: Vec<String>,
    /// Evidence it requires, one line each.
    pub evidence: Vec<String>,
    /// Verifiers that must have spoken.
    pub verification: Vec<BoundVerification>,
    /// Capabilities it withdraws.
    pub denied: Vec<String>,
    /// Capabilities it puts behind a recorded approval.
    pub approval_required: Vec<String>,
    /// Capabilities it grants.
    pub allowed: Vec<String>,
    /// What happens when one of its requirements is not met.
    pub on_failure: String,
}

impl BoundPrinciple {
    /// `true` when this principle reaches the workflow at all.
    fn binds(&self) -> bool {
        !self.obligations.is_empty()
            || self
                .verification
                .iter()
                .any(|check| check.landing.reaches())
            || !self.denied.is_empty()
            || !self.approval_required.is_empty()
            || !self.evidence.is_empty()
    }
}

/// The principles that bind one workflow, in id order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligations {
    /// Each binding principle, sorted by reference so two renderings agree.
    pub principles: Vec<BoundPrinciple>,
}

impl Obligations {
    /// Works out which of `principles` bind `workflow`, and where each obligation lands.
    ///
    /// The result is sorted by principle reference and every collection inside it is ordered, so
    /// the same workflow and the same principles produce the same bytes however the caller's
    /// registry happened to iterate.
    pub fn of<'a>(
        workflow: &Workflow,
        principles: impl IntoIterator<Item = &'a Principle>,
    ) -> Self {
        let mut bound: Vec<BoundPrinciple> = principles
            .into_iter()
            .map(|principle| bind(workflow, principle))
            .filter(BoundPrinciple::binds)
            .collect();
        bound.sort_by(|one, other| one.reference.cmp(&other.reference));
        Self { principles: bound }
    }

    /// `true` when no principle in the tree bears on this workflow.
    pub fn is_empty(&self) -> bool {
        self.principles.is_empty()
    }
}

/// One principle, resolved against one workflow.
fn bind(workflow: &Workflow, principle: &Principle) -> BoundPrinciple {
    let mut obligations = Vec::new();
    let mut elsewhere: BTreeSet<String> = BTreeSet::new();
    for obligation in &principle.obligations {
        let landing = Landing::of(workflow, &obligation.timing);
        if !landing.reaches() {
            elsewhere.insert(landing.timing);
            continue;
        }
        obligations.push(BoundObligation {
            id: obligation.id.to_string(),
            description: obligation.description.clone(),
            landing,
            requires: requirement_lines(&obligation.requires),
        });
    }

    let verification = principle
        .verification
        .iter()
        .map(|requirement| BoundVerification {
            statement: requirement.to_string(),
            landing: Landing::of(workflow, &requirement.timing),
        })
        .collect();

    BoundPrinciple {
        id: principle.id.to_string(),
        reference: format!("{}/{}", principle.id, principle.version.get()),
        title: principle.title.clone(),
        summary: principle.summary.clone(),
        applies_when: match &principle.applicability {
            // `always` is what a principle with no `applies_when` parses to, and an instruction
            // reading *applies when: always* is a line that costs a reader time and tells them
            // nothing they did not already have by the rule being in the document.
            Predicate::Always => None,
            other => Some(other.to_string()),
        },
        obligations,
        elsewhere: elsewhere.into_iter().collect(),
        evidence: principle.evidence.iter().map(ToString::to_string).collect(),
        verification,
        denied: capabilities(&principle.capabilities.deny),
        approval_required: capabilities(&principle.capabilities.approval_required),
        allowed: capabilities(&principle.capabilities.allow),
        on_failure: principle.failure_policy.to_string(),
    }
}

/// A capability set as sorted text.
fn capabilities(set: &BTreeSet<aep_domain::capability::Capability>) -> Vec<String> {
    set.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{fixture_principles, fixture_workflow, principle_from, workflow_at};

    #[test]
    fn an_obligation_timed_against_a_phase_lands_on_every_state_declaring_it() {
        let workflow = fixture_workflow();
        let bound = Obligations::of(&workflow, fixture_principles().iter());
        let test_driven = bound
            .principles
            .iter()
            .find(|principle| principle.id == "test-driven")
            .expect(
                "the repository ships `test-driven`, and `adp/default` has an implementation phase",
            );

        let before_implementation = test_driven
            .obligations
            .iter()
            .find(|obligation| obligation.landing.phase.as_deref() == Some("implementation"))
            .expect("`test-driven` times an obligation before implementation");
        assert_eq!(
            before_implementation
                .landing
                .states
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["implement".to_owned()],
            "`implement` is the state of `adp/default` that declares the implementation phase"
        );
        assert!(
            before_implementation
                .requires
                .iter()
                .any(|line| line == "test.first_result == failed"),
            "the requirement travels verbatim: {:?}",
            before_implementation.requires
        );
    }

    #[test]
    fn a_workflow_without_the_phase_reports_the_obligation_as_owed_elsewhere() {
        // The rule is only load-bearing where the two workflows differ, so this asserts the
        // difference rather than one side of it: `adp/default` declares an implementation phase and
        // `release/progressive` does not.
        let release = workflow_at("workflows/releases/progressive.yaml");
        assert!(
            !release
                .phases()
                .iter()
                .any(|phase| phase.as_str() == "implementation"),
            "the fixture must not declare the phase, or this asserts nothing"
        );

        let bound = Obligations::of(&release, fixture_principles().iter());
        let test_driven = bound
            .principles
            .iter()
            .find(|principle| principle.id == "test-driven")
            .expect("it still binds, through its completion obligation");
        assert!(
            test_driven
                .obligations
                .iter()
                .all(|obligation| obligation.landing.phase.as_deref() != Some("implementation")),
            "an obligation timed against a phase this workflow has no state for must not be \
             written as though it were owed here"
        );
        assert!(
            test_driven
                .elsewhere
                .iter()
                .any(|timing| timing.contains("implementation")),
            "and it must be reported as owed elsewhere rather than dropped: {:?}",
            test_driven.elsewhere
        );
    }

    #[test]
    fn a_principle_that_only_withdraws_a_capability_still_binds() {
        // `least-privilege` has no `requires:` at all — no obligations, no verification, no
        // evidence — and *you may not read a secret* is the entire instruction. A binding rule that
        // only looked at obligations would drop it, which is the one omission that matters.
        let principle = principle_from(
            r"
id: least-privilege-fixture
title: Least privilege
capabilities:
  deny: [secret.read]
",
        );
        assert!(
            principle.obligations.is_empty() && principle.evidence.is_empty(),
            "the fixture must reach the state the rule is about"
        );

        let bound = Obligations::of(&fixture_workflow(), [&principle]);
        let only = bound
            .principles
            .first()
            .expect("a capability withdrawal binds every workflow");
        assert_eq!(only.denied, vec!["secret.read".to_owned()]);
        assert!(only.obligations.is_empty());
    }

    #[test]
    fn a_principle_that_reaches_nothing_is_left_out_entirely() {
        let principle = principle_from(
            r"
id: mitigation-only-fixture
title: Mitigation only
requires:
  before_mitigation:
    predicates:
      - hypothesis.stated
",
        );
        let workflow = fixture_workflow();
        assert!(
            !workflow
                .phases()
                .iter()
                .any(|phase| phase.as_str() == "mitigation"),
            "the fixture workflow must not declare the phase, or this asserts nothing"
        );
        assert!(
            Obligations::of(&workflow, [&principle]).is_empty(),
            "a principle whose every obligation is timed against a phase this workflow does not \
             declare is a rule for a different workflow"
        );
    }

    #[test]
    fn the_binding_principles_are_sorted_by_reference_whatever_order_they_arrive_in() {
        let workflow = fixture_workflow();
        let principles = fixture_principles();
        let forwards = Obligations::of(&workflow, principles.iter());
        let backwards = Obligations::of(&workflow, principles.iter().rev());
        assert_eq!(
            forwards, backwards,
            "the same principles in the opposite order must produce the same document"
        );
        let references: Vec<&String> = forwards
            .principles
            .iter()
            .map(|principle| &principle.reference)
            .collect();
        let mut sorted = references.clone();
        sorted.sort();
        assert_eq!(references, sorted);
    }

    #[test]
    fn an_unconditional_principle_states_no_applicability() {
        let bound = Obligations::of(&fixture_workflow(), fixture_principles().iter());
        let least_privilege = bound
            .principles
            .iter()
            .find(|principle| principle.id == "least-privilege")
            .expect("`least-privilege` binds every workflow");
        assert_eq!(
            least_privilege.applies_when, None,
            "`always` is not worth a line: a principle with no `applies_when` applies"
        );
        let clean_room = bound
            .principles
            .iter()
            .find(|principle| principle.id == "clean-room")
            .expect("`clean-room` binds through its obligations");
        assert_eq!(
            clean_room.applies_when.as_deref(),
            Some("change.clean_room == true"),
            "a conditional principle says its condition, as text and not as a verdict"
        );
    }
}
