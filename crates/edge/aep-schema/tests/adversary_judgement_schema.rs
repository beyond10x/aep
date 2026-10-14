//! Adversarial case for story:llm-step-declares-judgement: the published schema against the
//! loader.
//!
//! `schema.rs` says "the schema says what the loader refuses, so an editor flags an llm step that
//! does not say why it needs a model". The unit tested that for an absent key only.

use std::path::Path;

use serde_json::Value;

const WHITESPACE_JUDGEMENT: &str = "format: aep.driver-steps/1
id: test/whitespace
workflow: adp/default/1
states:
  implement:
    steps:
      - kind: llm
        prompt: write the change
        judgement: \"   \"
";

/// A map whose only `llm` step's judgement is whitespace is refused by the loader with
/// `empty_declaration`; the published `driver-steps` schema (`minLength: 1`) accepts it, so an
/// editor shows the document as valid and the load refuses it.
#[test]
fn the_published_schema_refuses_a_whitespace_judgement_the_loader_refuses() {
    let refusal = aep_schema::parse::step_map(WHITESPACE_JUDGEMENT, None)
        .expect_err("the loader refuses a whitespace-only judgement");
    assert!(
        refusal.to_string().contains("judgement"),
        "the loader names the judgement: {refusal}"
    );

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../schemas/generated/driver-steps.schema.json");
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("the published schema is readable"),
    )
    .expect("the published schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("a usable schema");
    let document: Value = serde_yaml::from_str(WHITESPACE_JUDGEMENT).expect("well formed");
    assert!(
        !validator.is_valid(&document),
        "the published schema accepts a whitespace-only judgement the loader refuses"
    );
}
