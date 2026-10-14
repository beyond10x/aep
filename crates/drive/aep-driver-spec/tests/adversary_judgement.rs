//! Adversarial cases for story:llm-step-declares-judgement at the loader.
//!
//! The unit's own tests refuse a missing judgement on an otherwise valid `llm` step. They do not
//! put a second defect on the same step, so a check that runs only when the rest of the step is
//! sound passes them.

use aep_domain::error::{ValidationCode, ValidationErrors};
use aep_driver_spec::map::{RawStepMap, StepMap};

fn load(states: &str) -> Result<StepMap, ValidationErrors> {
    let raw: RawStepMap = serde_json::from_str(&format!(
        r#"{{"format":"aep.driver-steps/1","id":"development/default",
            "workflow":"adp/default/1","states":{states}}}"#
    ))
    .expect("the fixture deserializes");
    StepMap::try_from(raw)
}

/// Invariant *Validation accumulates*: an `llm` step with a blank prompt and no judgement has two
/// independent defects, and both are reported together.
///
/// `validate_step` returns at the blank prompt before `validated_judgement` runs
/// (`crates/drive/aep-driver-spec/src/map.rs`, the `RawStep::Llm` arm), so the author fixes the
/// prompt, reloads, and only then learns the judgement is missing.
#[test]
fn an_llm_step_with_a_blank_prompt_and_no_judgement_reports_both_defects() {
    let errors = load(r#"{"implement":{"steps":[{"kind":"llm","prompt":"  "}]}}"#)
        .expect_err("two defects on one step");
    let found: Vec<(ValidationCode, String)> = errors
        .as_slice()
        .iter()
        .map(|error| (error.code, error.location.clone()))
        .collect();
    let step = "driver-steps[development/default].states.implement.steps[0]";
    assert!(
        found.contains(&(ValidationCode::EmptyDeclaration, format!("{step}.prompt"))),
        "the blank prompt is refused: {errors}"
    );
    assert!(
        found.contains(&(
            ValidationCode::MissingDeclaration,
            format!("{step}.judgement")
        )),
        "the missing judgement is refused beside the blank prompt, not after it is fixed: {errors}"
    );
}
