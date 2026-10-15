//! Adversarial cases for story:command-step-expects-failure at the loader and at
//! `StepMap::check_inputs`.
//!
//! The unit's own tests check an `output:` against the exact spellings in
//! `CommandStep::RESERVED_OUTPUTS`, and an input value only for whether it names a placeholder.
//! They do not ask what the file system the name lands on does with it, or whether a value can
//! make a context entry something the loader refuses when a map writes it.

use std::collections::BTreeMap;

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

fn with_output(output: &str) -> Result<StepMap, ValidationErrors> {
    load(&format!(
        r#"{{"implement":{{"steps":[{{"kind":"command","run":["git","log"],"output":{}}}]}}}}"#,
        serde_json::to_string(output).expect("a string serializes")
    ))
}

/// `aarch64-apple-darwin` and `x86_64-apple-darwin` are release targets, and APFS is
/// case-insensitive by default: an `output: State.json` is the run's own `state.json` there, and
/// the step's stdout replaces the generation pointer the run resumes from. The reserved check
/// compares the exact spelling only (`map.rs` `validated_output`).
#[test]
fn an_output_naming_a_run_record_in_another_case_is_refused() {
    for output in ["State.json", "Cursor.json", "LAUNCH.JSON", "Transcripts"] {
        let outcome = with_output(output);
        let errors = outcome.expect_err(&format!(
            "`{output}` is a run record on a case-insensitive file system"
        ));
        assert!(
            errors.contains(ValidationCode::UnsupportedConstruct),
            "`{output}`: {errors}"
        );
    }
}

/// A name the loader admits must be one the driver can write: a NUL byte cannot be in a file name
/// on any target, so `fs::write` refuses it after the command has already run, and the step is a
/// `NoVerdict` retried to exhaustion instead of a refusal at load.
#[test]
fn an_output_holding_a_nul_byte_is_refused_at_load() {
    assert!(
        with_output("a\u{0}b").is_err(),
        "a NUL byte is admitted as an output file name"
    );
}

/// Every run directory holds `<state>-<index>-<attempt>.log`, the step's own log, written before
/// the output is kept: an `output: implement-0-1.log` on step 0 of `implement` overwrites the log
/// of the attempt that produced it. `RESERVED_OUTPUTS` lists the run's records but not this one.
#[test]
fn an_output_named_like_a_step_log_is_refused() {
    let errors = with_output("implement-0-1.log")
        .expect_err("the name is the first attempt's log of this very step");
    assert!(
        errors.contains(ValidationCode::UnsupportedConstruct),
        "{errors}"
    );
}

/// The loader refuses a context entry that is blank or a glob when a map writes it
/// (`validated_context`). The same entry spelt through an input — `context: ['{input.notes}']`
/// with `--input notes='docs/*.md'` or `--input notes=` — is admitted by `check_inputs`, so the
/// session is handed exactly the entry the loader exists to refuse.
#[test]
fn an_input_value_that_makes_a_context_entry_a_glob_or_blank_is_refused() {
    let map = load(
        r#"{"implement":{"steps":[
            {"kind":"llm","prompt":"p","judgement":"j","context":["{input.notes}"]}]}}"#,
    )
    .expect("the map loads");
    for value in ["docs/*.md", "notes?.md", "", "   "] {
        let given: BTreeMap<String, String> = [("notes".to_owned(), value.to_owned())].into();
        let errors = map.check_inputs(&given);
        assert!(
            !errors.is_empty(),
            "`--input notes={value}` turns the context entry into one the loader refuses, and \
             check_inputs admits it"
        );
    }
}
