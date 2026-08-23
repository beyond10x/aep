//! The contract runner's record, at the guard it decides.
//!
//! `story:contract-result-ingestion` built the road a `contract_result` travels — an outside
//! runner's bytes become an evidence document `protocol evaluate --evidence` reads — and said in
//! its own *Out of Scope* that **nothing gated on it**: no workflow, profile or step map asked the
//! record for anything. This file is the other half. `principles/development/contract-testing.yaml`
//! now owes `contracts.breaking_changes == 0` **before the review phase**, so the number an outside
//! runner measured decides a transition in `adp/default`, and this asserts it against the shipped
//! documents rather than against a fixture written to agree.
//!
//! # Why `breaking_changes` and not `failed`, and why that is testable here
//!
//! The two counts are not the same claim. `failed` says *the contract run is red*;
//! `breaking_changes` says *and somebody who already calls this interface was told something that
//! is no longer true*. A review is exactly the right place for the first — a person can read a
//! failure, decide it is the runner's own machinery and say so — and no place at all for the
//! second, because the people affected are not in the room.
//!
//! So the three rows below differ in **one number** and the middle one is the control: rows two and
//! three agree that the contract run went red, and disagree only about whose fault it is. Without
//! that row the obligation could have been written over `contracts.failed` and every other
//! assertion here would still pass.
//!
//! # Where the record arrives
//!
//! In `adversarial_verify`, not in `verify`. That is not a convenience: a `contract_result` about a
//! metaharness adapter is produced in another repository, on another day, and reaches this one as
//! bytes somebody pipes in — `protocol contract evidence --record -`. Evidence is submitted where
//! it arrives, and the fact store takes the latest record for a path, so a run already past its own
//! contract suite is precisely the case this gate exists for.

use std::fs;
use std::path::{Path, PathBuf};

use aep_domain::artifact::ArtifactGraph;
use aep_domain::evidence::{ContractResult, Evidence, EvidenceKind, Producer};
use aep_domain::task::Task;
use aep_domain::time::{ObservedAt, Timestamp};
use aep_domain::verification::Verifier;
use aep_engine::engine::{EvidenceSubmission, ProtocolEngine, TransitionResult};
use aep_engine::evaluate::TransitionEvaluation;
use aep_engine::{load_tree, Engine, Execution, FixedClock, Registry};

/// The repository root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root exists")
}

/// A file of the worked example this walk is made of.
fn example(file: &str) -> PathBuf {
    root().join("examples/development-passkeys").join(file)
}

/// The repository's own document tree, because the claim is about the shipped principle.
fn registry() -> Registry {
    load_tree(&root()).expect("the document tree is valid")
}

/// The example task: a feature under `development.standard`, which carries `contract-testing`.
fn task() -> Task {
    let text = fs::read_to_string(example("task.yaml")).expect("the example task is readable");
    aep_schema::parse::task(&text, Some("examples/development-passkeys/task.yaml"))
        .expect("the example task is valid")
}

/// Its artifact graph.
fn artifacts() -> ArtifactGraph {
    let text = fs::read_to_string(example("artifacts.yaml")).expect("the manifest is readable");
    aep_schema::parse::artifact_manifest(
        &text,
        Some("examples/development-passkeys/artifacts.yaml"),
    )
    .expect("the example manifest is valid")
}

/// An engine on a fixed clock, so the walk is the same one every time.
fn engine() -> Engine<FixedClock> {
    Engine::with_clock(registry(), FixedClock::new(1_700_000_000_000))
}

/// When every submission here says somebody looked: a minute before the fixed clock, so no record
/// is refused for claiming a future observation.
const OBSERVED: ObservedAt = ObservedAt::new(Timestamp::from_epoch_millis(1_699_999_940_000));

/// One contract run, as a contract runner reports it.
fn contract_run(checked: usize, failed: usize, breaking_changes: usize) -> Evidence {
    Evidence::ContractResult(ContractResult {
        checked,
        failed,
        breaking_changes,
        consumer: Some("metaharness.event/1".to_owned()),
        provider: Some("claude 2.1.239".to_owned()),
    })
}

/// A record from the one producer class this evidence kind names.
fn by_contract_runner(evidence: Evidence) -> EvidenceSubmission {
    EvidenceSubmission::new(
        evidence,
        Producer::Verifier {
            verifier: Verifier::ContractRunner,
        },
        OBSERVED,
    )
}

/// The example's whole evidence sequence, with its one `contract_result` replaced by `contract`,
/// walked until the run is in `adversarial_verify`.
///
/// The substitution is what makes the fixture reach the state where the rule is load-bearing: the
/// record the example ships is green, and a walk that never varied it could not tell this gate from
/// no gate at all.
fn at_adversarial_verify(engine: &Engine<FixedClock>, contract: &Evidence) -> Execution {
    let mut execution = engine
        .initialize_with_artifacts(task(), artifacts())
        .expect("the example initialises");

    let mut files: Vec<PathBuf> = fs::read_dir(example("evidence"))
        .expect("the evidence directory is readable")
        .map(|entry| entry.expect("a readable entry").path())
        .collect();
    files.sort();

    for file in files {
        let text = fs::read_to_string(&file).expect("the evidence file is readable");
        let inputs = aep_schema::parse::evidence_list(&text, Some(&file.display().to_string()))
            .unwrap_or_else(|error| panic!("{} is not valid evidence: {error}", file.display()));
        for input in inputs {
            let evidence = if input.evidence.kind() == EvidenceKind::ContractResult {
                contract.clone()
            } else {
                input.evidence
            };
            let mut submission = EvidenceSubmission::new(evidence, input.producer, OBSERVED);
            submission.subject = input.about;
            engine
                .submit_evidence(&mut execution, submission)
                .unwrap_or_else(|error| panic!("{} was refused: {error}", file.display()));
        }
    }

    // Step, never leap: the walk stops the moment it arrives, so the assertions below are made in
    // `adversarial_verify` and not in whatever state a greedy advance would have ended in.
    for _ in 0..12 {
        if execution.state_id().as_str() == "adversarial_verify" {
            break;
        }
        match engine.transition(&mut execution) {
            Ok(TransitionResult::Moved { .. }) => {}
            Ok(_) | Err(_) => break,
        }
    }
    execution
}

/// How `adversarial_verify -> review` reads right now.
fn into_review(engine: &Engine<FixedClock>, execution: &Execution) -> TransitionEvaluation {
    engine
        .evaluate(execution)
        .transitions
        .into_iter()
        .find(|transition| transition.to.as_str() == "review")
        .expect("`adp/default` offers review from adversarial verification")
}

/// The line this gate is named after, as the evaluation spells it.
const GATE: &str = "contracts.breaking_changes == 0";

#[test]
fn a_breaking_record_does_not_reach_review_and_a_red_one_does() {
    let engine = engine();
    let mut execution = at_adversarial_verify(&engine, &contract_run(20, 0, 0));

    // The fixture reached the state where the rule decides anything. Asserted before the outcome,
    // because a walk that stopped early would make every claim below vacuous.
    assert_eq!(
        execution.state_id().as_str(),
        "adversarial_verify",
        "the example's evidence must walk this far, or this file tests nothing"
    );
    assert!(
        into_review(&engine, &execution)
            .requirements
            .iter()
            .any(|requirement| requirement.outcome.requirement.contains(GATE)),
        "the obligation must be read at this transition, or the rows below prove nothing"
    );

    // Row one: the captured shape of a green adapter contract. Twenty vectors, nothing red.
    assert!(
        into_review(&engine, &execution).permitted,
        "a green contract record does not hold a change out of review: {:?}",
        into_review(&engine, &execution).unmet()
    );

    // Row two, the control: the run went red and it is the runner's own machinery. A review is what
    // that finding is for, so it moves.
    engine
        .submit_evidence(&mut execution, by_contract_runner(contract_run(20, 1, 0)))
        .expect("a failing contract run is still a record");
    assert!(
        into_review(&engine, &execution).permitted,
        "a contract run that went red without breaking a consumer is a review's business: {:?}",
        into_review(&engine, &execution).unmet()
    );

    // Row three: the same red run, and this time the vendor moved. One number differs from row two.
    engine
        .submit_evidence(&mut execution, by_contract_runner(contract_run(20, 1, 1)))
        .expect("a breaking contract run is still a record");
    let refused = into_review(&engine, &execution);
    assert!(
        !refused.permitted,
        "one breaking change must hold the change out of review"
    );
    assert!(
        refused.unmet().iter().any(|reason| reason.contains(GATE)),
        "and the refusal must name the line that refused, not merely block: {:?}",
        refused.unmet()
    );
    assert!(
        refused
            .unmet()
            .iter()
            .all(|reason| !reason.contains("contracts.failed")),
        "rows two and three differ on exactly one line, so nothing about `failed` may appear \
         here — otherwise this gate is a copy of the one the workflow already has: {:?}",
        refused.unmet()
    );
}

#[test]
fn a_run_that_never_heard_from_a_contract_runner_does_not_enter_review_by_saying_nothing() {
    // The fail-closed direction, and invariant 5 in its narrowest form: the gate reads a count, an
    // unobserved count is `Unknown`, and `Unknown` is not `True`. The substitution puts a
    // `test_result` named `contract` in the record's place — which is what
    // `drivers/development/checks.yaml` actually submits, from `protocol validate` — so
    // `tests.contract.failed == 0` is satisfied and the run walks to `adversarial_verify` exactly
    // as before. What no test runner can produce is `contracts.breaking_changes`, and that is the
    // whole difference between the two fixtures.
    let engine = engine();
    let execution = at_adversarial_verify(
        &engine,
        &Evidence::TestResult(aep_domain::evidence::TestResult::passing(
            aep_domain::evidence::TestSuite::Contract,
            20,
        )),
    );
    assert_eq!(
        execution.state_id().as_str(),
        "adversarial_verify",
        "a contract suite still carries the run this far, which is what makes the gap visible here"
    );

    let refused = into_review(&engine, &execution);
    assert!(
        !refused.permitted,
        "silence about breaking changes is not a report of none"
    );
    let named = refused
        .requirements
        .iter()
        .find(|requirement| requirement.outcome.requirement.contains(GATE))
        .expect("the obligation is listed at this transition");
    assert!(
        !named.is_satisfied(),
        "an unobserved count must not satisfy the gate that reads it"
    );
    assert!(
        named
            .outcome
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("unobserved"),
        "and it must read as unobserved rather than as a measured zero: {named:?}"
    );
}
