//! Adversarial cases for phase two — `StepMap::check_run` against the protocol in force.
//!
//! The unit test in `map.rs` runs one map with **one** state holding **one** step whose kind is
//! `test_result`, against a protocol that declares `["approval"]` and then one that declares
//! `["test_result"]`. Every assertion it makes is satisfied by an implementation that ignores
//! `mapping.kind` entirely and asks the protocol about a fixed `EvidenceKind::TestResult`, and by
//! one that stops at the first state, the first step, or the first refusal. These cases say the
//! things that shape cannot say.

use aep_domain::error::{ValidationCode, ValidationErrors};
use aep_domain::protocol::Protocol;
use aep_domain::workflow::Workflow;
use aep_driver_spec::map::{RawStepMap, StepMap};

/// A protocol declaring exactly the evidence kinds named, and every verifier this file uses.
fn protocol_declaring(evidence_kinds: &str) -> Protocol {
    let raw: aep_domain::raw::RawProtocol = serde_json::from_str(&format!(
        r#"{{"id":"aep","version":1,"title":"t","evidence_kinds":{evidence_kinds},
            "verifiers":["test-runner","contract-runner","human-approval","static-analyzer"],
            "observables":["tests.**"]}}"#
    ))
    .expect("the fixture deserializes");
    Protocol::try_from(raw).expect("the fixture validates")
}

/// The workflow the maps below pin, with the two states they name.
fn workflow() -> Workflow {
    let raw: aep_domain::raw::RawWorkflow = serde_json::from_str(
        r#"{"id":"adp/default","version":1,"title":"t","initial":"implement",
            "states":{"implement":{"title":"Implement"},
                      "verify":{"title":"Verify","terminal":true}},
            "transitions":[{"from":"implement","to":"verify"}]}"#,
    )
    .expect("the fixture deserializes");
    Workflow::try_from(raw).expect("the fixture validates")
}

/// A map pinned to `adp/default/1` with the states given.
fn map(states: &str) -> StepMap {
    let raw: RawStepMap = serde_json::from_str(&format!(
        r#"{{"format":"aep.driver-steps/1","id":"development/default",
            "workflow":"adp/default/1","states":{states}}}"#
    ))
    .expect("the fixture deserializes");
    StepMap::try_from(raw).expect("the fixture validates at load")
}

/// Every location the errors name, for asserting which step was refused.
fn locations(errors: &ValidationErrors) -> Vec<String> {
    errors
        .as_slice()
        .iter()
        .map(|error| error.location.clone())
        .collect()
}

/// The kind asked about is the one the *step* declares, not one the check picked for itself.
///
/// Two steps, one kind each; the protocol declares the first kind and not the second. A check
/// hard-wired to `test_result` — the only kind the existing test's map contains — sees a protocol
/// that declares it and refuses nothing, and the existing test cannot tell the difference.
#[test]
fn the_kind_compared_is_the_one_the_step_declares_and_the_step_refused_is_the_one_that_named_it() {
    let states = r#"{"implement":{"steps":[
        {"kind":"command","run":["cargo","test"],
         "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}},
        {"kind":"command","run":["pact","verify"],
         "evidence":{"kind":"contract_result","verifier":"contract-runner"}}]}}"#;
    let map = map(states);
    let workflow = workflow();
    assert!(
        map.cross_validate(&workflow).is_empty(),
        "phase one has nothing to say about this map, so phase two is what is under test"
    );

    let errors = map.check_run(&protocol_declaring(r#"["test_result"]"#), &workflow);
    assert_eq!(
        errors.len(),
        1,
        "the protocol declares `test_result` and not `contract_result`: {errors}"
    );
    assert!(
        errors.contains(ValidationCode::UndeclaredEvidenceKind),
        "{errors}"
    );
    let said = errors.to_string();
    assert!(
        said.contains("contract_result"),
        "the refusal names the kind the second step declares: {said}"
    );
    assert_eq!(
        locations(&errors),
        vec!["driver-steps[development/default].states.implement.steps[1].evidence.kind"],
        "and it names the step that declared it, not the first step in the state"
    );
}

/// Every undeclared kind is reported, not the first one.
///
/// Invariant 3, the same rule `four_broken_steps_report_four_errors` asserts for load: a validator
/// that returns at the first problem passes a `contains` test and fails a counted one. Two states,
/// two steps each, four kinds the protocol does not declare.
#[test]
fn four_steps_naming_kinds_the_protocol_does_not_declare_report_four_refusals() {
    let states = r#"{"implement":{"steps":[
        {"kind":"command","run":["cargo","test"],
         "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}},
        {"kind":"command","run":["pact","verify"],
         "evidence":{"kind":"contract_result","verifier":"contract-runner"}}]},
        "verify":{"steps":[
        {"kind":"command","run":["clippy"],
         "evidence":{"kind":"static_analysis","verifier":"static-analyzer"}},
        {"kind":"command","run":["git","diff"],
         "evidence":{"kind":"diff","verifier":"static-analyzer"}}]}}"#;
    let map = map(states);
    let workflow = workflow();
    assert!(
        map.cross_validate(&workflow).is_empty(),
        "phase one is quiet"
    );

    let errors = map.check_run(&protocol_declaring(r#"["approval"]"#), &workflow);
    assert_eq!(
        errors.len(),
        4,
        "one refusal per step, not one for the document: {errors}"
    );
    assert_eq!(
        locations(&errors),
        vec![
            "driver-steps[development/default].states.implement.steps[0].evidence.kind",
            "driver-steps[development/default].states.implement.steps[1].evidence.kind",
            "driver-steps[development/default].states.verify.steps[0].evidence.kind",
            "driver-steps[development/default].states.verify.steps[1].evidence.kind",
        ],
        "every state and every step of it is reached: {errors}"
    );
}

/// Review finding F5, the half `map.rs`'s own doc comment promises and nothing asserts.
///
/// An external tool is exempt from the *verifier* check at load — `cross_validate` skips it — and
/// the module doc says "its kind is still checked at run start against the protocol". Folding the
/// `Verifier::NAMED` guard into `check_run` alongside it would make that sentence false and leave
/// every existing test green.
#[test]
fn an_external_tool_is_exempt_at_load_and_its_kind_is_still_checked_at_run_start() {
    let states = r#"{"implement":{"steps":[
        {"kind":"command","run":["ruff","check"],
         "evidence":{"kind":"static_analysis","verifier":"ruff"}}]}}"#;
    let map = map(states);
    let workflow = workflow();
    assert!(
        map.cross_validate(&workflow).is_empty(),
        "an external tool has no row in the defaults table, so load has nothing to refuse it with"
    );

    let errors = map.check_run(&protocol_declaring(r#"["test_result"]"#), &workflow);
    assert_eq!(errors.len(), 1, "{errors}");
    assert!(
        errors.contains(ValidationCode::UndeclaredEvidenceKind),
        "an external tool's exemption is from the verifier table, not from the protocol: {errors}"
    );
    assert!(errors.to_string().contains("static_analysis"), "{errors}");
}

/// Phase two checks the pin as well as the kinds, which is the other half of what it is for.
///
/// `map.rs` asserts the pin through `cross_validate` only. The claim the module doc makes is about
/// `check_run` — "the resolved workflow's id and version equal the pin" — and a `check_run` that
/// stopped calling `cross_validate` would satisfy every assertion the new test makes.
#[test]
fn a_resolved_workflow_at_a_major_the_map_does_not_pin_is_refused_at_run_start_too() {
    let map = map(r#"{"implement":{"steps":[
            {"kind":"command","run":["cargo","test"],
             "evidence":{"kind":"test_result","verifier":"test-runner","suite":"unit"}}]}}"#);
    let raw: aep_domain::raw::RawWorkflow = serde_json::from_str(
        r#"{"id":"adp/default","version":2,"title":"t","initial":"implement",
            "states":{"implement":{"title":"Implement","terminal":true}},
            "transitions":[]}"#,
    )
    .expect("the fixture deserializes");
    let moved = Workflow::try_from(raw).expect("the fixture validates");

    let errors = map.check_run(&protocol_declaring(r#"["test_result"]"#), &moved);
    assert_eq!(
        errors.len(),
        1,
        "the kind is declared; the pin is not satisfied: {errors}"
    );
    assert!(errors.contains(ValidationCode::VersionMismatch), "{errors}");
}
