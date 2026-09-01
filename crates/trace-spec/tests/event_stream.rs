//! The event-stream reader against three whole driven steps, and the three shipped documents.
//!
//! # These three fixtures are **synthesized**, and that is the difference from `adapter.rs`
//!
//! The `stream-json` fixtures beside them are real eval runs, committed verbatim, and the tests
//! over them assert numbers a paid run actually produced. These three are not: they are written by
//! hand against metaharness's own emitted stream — its
//! `crates/metaharness-claude/fixtures/c2/session.expected.jsonl`, read on 2026-08-22, and its
//! amendment a9 wire tests, read on 2026-08-23 — and they are structurally faithful rather than
//! observed. Every payload field is present, an absent one
//! is an explicit `null`, `at` is omitted where a vendor recorded no timestamp, and the seam's
//! `tool.decided` events carry the reasons this repository's own `decide_tool` policy writes.
//!
//! What that means for a reader of a failing assertion here: a number in this file is a number
//! **this file chose**, so a mismatch is a change in the reader and never a change in a harness.
//! The moment a real driven run is committed, it belongs beside these and these become what they
//! are — a shape test.
//!
//! # What each fixture is for
//!
//! | fixture | the step it stands for |
//! |---|---|
//! | `metaharness-driven-honest-step.jsonl` | the session asked to do the ordinary thing: loads the skill, creates through the CLI, validates, and is refused once for chaining a command line |
//! | `metaharness-driven-denial-step.jsonl` | the session induced to hand-edit frontmatter and to reach outside the driven surface: three calls, three denials, no result the guardrails did not intend |
//! | `metaharness-driven-null-a9-step.jsonl` | a step whose **vendor answered none of amendment a9's questions**: every one of its five keys is present and explicitly `null`, which is the wire's way of saying *nobody reported this* |
//!
//! # The third fixture is a polarity, not a step
//!
//! The first two are about what an agent did. The third is about what a *record* does not say, and
//! it exists because amendment a9 made four expectation kinds decidable **where a value arrives**
//! and changed nothing about where none does. A reader that answered `ok` on a `null` would pass
//! every Codex-driven run in silence, so the fixture that must keep failing to decide is committed
//! beside the ones that decide.

use trace_domain::ir::TraceIr;
use trace_domain::spec::TraceSpec;
use trace_spec::check::check;
use trace_spec::event_stream::{read_event_stream, read_event_stream_str};
use trace_spec::reader::{detect, read_any, TranscriptFormat};
use trace_spec::report::{Outcome, UnknownReason, Verdict};

/// The honest driven step.
const HONEST: &[u8] = include_bytes!("fixtures/metaharness-driven-honest-step.jsonl");

/// The deliberate-denial driven step.
const DENIAL: &[u8] = include_bytes!("fixtures/metaharness-driven-denial-step.jsonl");

/// The step whose vendor reported none of amendment a9's five fields.
const NULL_A9: &[u8] = include_bytes!("fixtures/metaharness-driven-null-a9-step.jsonl");

/// A recorded Claude Code run, which must keep reading through the other adapter untouched.
const RECORDED: &[u8] = include_bytes!("fixtures/plugin-eval-7hTYjT.jsonl");

fn ir(bytes: &[u8]) -> TraceIr {
    read_event_stream(bytes).expect("the committed fixture is an event stream this build reads")
}

/// One of the three shipped expectation documents, as the repository ships it.
fn document(name: &str) -> TraceSpec {
    let path = format!(
        "{}/../../conformance/trace/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{path} is committed"));
    trace_domain::raw::read_spec(&text)
        .unwrap_or_else(|errors| panic!("{name} must validate:\n{errors}"))
}

/// The ids of every row with this verdict, in report order.
fn rows_with(report: &trace_spec::report::CheckReport, verdict: Verdict) -> Vec<&str> {
    report
        .expectations
        .iter()
        .filter(|row| row.verdict == verdict)
        .map(|row| row.id.as_str())
        .collect()
}

#[test]
fn the_driven_step_document_holds_against_a_driven_event_stream() {
    // The story's first acceptance, end to end: the specification the migration left behind is
    // checkable again, without a word of it changing.
    let report = check(
        &document("expectations.driven-step.trace.yaml"),
        &ir(HONEST),
        &[],
    );
    assert_eq!(
        report.exit_code(),
        0,
        "{}",
        trace_spec::render::report_to_text(&report)
    );
    assert_eq!(report.summary.gap, 0, "nothing contradicted");
    assert_eq!(
        rows_with(&report, Verdict::Unknown),
        Vec::<&str>::new(),
        "every row decides. `the-skill-ran-to-completion` was the one that could not until \
         metaharness's amendment a9 put the vendor's own result record on the wire"
    );
    assert_eq!(
        report.adapter.name, "metaharness/event-stream",
        "the report says which reader judged the run"
    );
}

#[test]
fn the_denial_step_document_holds_against_a_denied_driven_event_stream() {
    // The other half, and the one the seam changed: three refusals taken by this repository's own
    // policy, none of them in the vendor's array, all three counted.
    let report = check(
        &document("expectations.denial-step.trace.yaml"),
        &ir(DENIAL),
        &[],
    );
    assert_eq!(
        report.exit_code(),
        0,
        "{}",
        trace_spec::render::report_to_text(&report)
    );
    assert_eq!(report.summary.gap, 0);
    assert_eq!(report.summary.unknown, 0, "nothing was left undecidable");
}

#[test]
fn the_denials_the_seam_took_are_what_permission_denied_counts() {
    // The load-bearing mapping. `session.ended.permission_denials` is empty in this run — the
    // seam refused all three calls before the vendor's own permission pipeline saw them — so a
    // reader that only read the vendor's array would report the run where enforcement worked as
    // the run where nothing was refused.
    let denial = ir(DENIAL);
    let outcome = denial.run_outcome().expect("a terminal record");
    assert_eq!(outcome.permission_denials, Some(3));

    // And the honest step, where the vendor listed the same refusal the seam took: one denial,
    // not two.
    let honest = ir(HONEST);
    assert_eq!(
        honest
            .run_outcome()
            .expect("a terminal record")
            .permission_denials,
        Some(1),
        "one refused call is one denial however many layers wrote it down"
    );
}

#[test]
fn the_census_of_a_driven_step_counts_the_control_plane_out_and_nothing_as_unread() {
    let census = ir(HONEST).census();
    assert_eq!(
        census.events, 18,
        "twenty-nine lines, eleven of them control plane: step and turn boundaries, four \
         decisions, and the usage records that fold into the request series"
    );
    assert_eq!(
        census.opaque_events, 0,
        "an event with no IR family is not an event nobody could read — routing the control plane \
         through the opaque path would make every count in every driven run `unk`"
    );
    assert_eq!(
        census.api_requests, 3,
        "three `usage` events, three requests"
    );
    assert_eq!(census.tool_traffic["Bash"].calls, 3);
    assert_eq!(
        census.tool_traffic["Bash"].errors, 1,
        "the refused command came back to the model as an error result"
    );
    assert_eq!(census.repeated_call_groups, 0);
}

#[test]
fn the_plugin_eval_document_reports_a_driven_step_as_a_different_run_rather_than_as_a_defect() {
    // The third shipped document is the *interactive* plugin eval's, and a driven step is not its
    // subject. It is checked here anyway, because the useful thing is the list: which of its rows
    // a driven event stream cannot satisfy, and why. Pinned so that a change in either the
    // document or the reader has to face the list rather than discover it in a paid run.
    let report = check(&document("expectations.trace.yaml"), &ir(HONEST), &[]);

    assert_eq!(
        rows_with(&report, Verdict::Gap),
        vec![
            // Three facts about the *run*, not about the reader:
            // a metaharness session stays in the vendor's default posture, because decisions
            // arrive over the seam rather than from a permission mode;
            "the-run-did-not-ask",
            // and the driven surface refused one chained command line, which the driven-step
            // document bounds at two rather than forbidding.
            "the-cli-never-refused-a-shell-call",
            "no-permission-denials",
        ],
        "{}",
        trace_spec::render::report_to_text(&report)
    );

    assert_eq!(
        rows_with(&report, Verdict::Unknown),
        Vec::<&str>::new(),
        "the four rows that used to be here — `skill-completed`, `one-pass-per-request`, \
         `thinking-tokens-within-reason` and `served-at-standard-speed` — were the reader's limit \
         and not the run's, and metaharness's amendment a9 removed it: {}",
        trace_spec::render::report_to_text(&report)
    );
}

/// A one-row specification, written the way a document writes one.
///
/// Inline rather than committed under `conformance/`, because these rows exist to pin the
/// **reader's** polarity and not to say what a driven step owes: a shipped document asserting
/// `speed: fast` would be a claim about somebody's account, which is exactly what the `speed`
/// kind's own doc comment warns against.
fn one_row(id: &str, expect: &str) -> TraceSpec {
    let text = format!(
        "format: trace-spec/1\n\
         id: a9-polarity/{id}\n\
         title: One field metaharness amendment a9 added\n\
         expectations:\n  \
         - id: {id}\n    \
             statement: the reader decides this row from what the vendor reported, or does not\n    \
             expect:\n      {expect}\n"
    );
    trace_domain::raw::read_spec(&text).unwrap_or_else(|errors| panic!("{text}\n{errors}"))
}

/// One row's verdict against one stream.
fn decide(ir: &TraceIr, id: &str, expect: &str) -> Verdict {
    let report = check(&one_row(id, expect), ir, &[]);
    report.expectations[0].verdict
}

/// The five rows amendment a9 unlocked, each as a document writes it, with a bound the honest step
/// satisfies.
const A9_SATISFIED: [(&str, &str); 5] = [
    (
        "the-skill-ran-to-completion",
        "skill.completed: {skill: aep-planning:planning, count: {at_least: 1}}",
    ),
    (
        "billed-thinking",
        "tokens.thinking: {count: {at_most: 500}}",
    ),
    ("one-pass-per-request", "iterations: {count: {at_most: 5}}"),
    ("served-at-standard-speed", "speed: {equals: standard}"),
    (
        "what-one-model-cost",
        "cost.total: {at_most_usd: 2.0, model: claude-sonnet-5}",
    ),
];

/// The same five rows, each written so that the honest step contradicts it.
const A9_CONTRADICTED: [(&str, &str); 4] = [
    ("billed-thinking", "tokens.thinking: {count: {at_most: 10}}"),
    ("one-pass-per-request", "iterations: {count: {at_most: 1}}"),
    ("served-at-standard-speed", "speed: {equals: fast}"),
    (
        "what-one-model-cost",
        "cost.total: {at_most_usd: 0.01, model: claude-sonnet-5}",
    ),
];

#[test]
fn every_kind_amendment_a9_unlocked_is_decided_where_the_vendor_reported_the_value() {
    // The flip, one row at a time. Each of these read `unk` against a driven stream until
    // metaharness carried the field, and each is now answered out of the run's own record.
    let ir = ir(HONEST);
    for (id, expect) in A9_SATISFIED {
        assert_eq!(
            decide(&ir, id, expect),
            Verdict::Ok,
            "{id}: {expect}\n{}",
            trace_spec::render::report_to_text(&check(&one_row(id, expect), &ir, &[]))
        );
    }
}

#[test]
fn a_bound_the_recorded_figures_break_is_a_gap_rather_than_a_pass_or_an_unknown() {
    // The middle polarity, and the one that says the reader is reading the number rather than its
    // presence: the same fields, the same run, bounds it does not meet.
    let ir = ir(HONEST);
    for (id, expect) in A9_CONTRADICTED {
        assert_eq!(
            decide(&ir, id, expect),
            Verdict::Gap,
            "{id}: {expect}\n{}",
            trace_spec::render::report_to_text(&check(&one_row(id, expect), &ir, &[]))
        );
    }
}

#[test]
fn a_skill_the_vendor_recorded_as_unsuccessful_is_a_gap_and_not_a_completion() {
    // `skill.completed`'s own wrong value, which is a boolean and not a bound. The call was made,
    // the result was correlated, the vendor named the skill — and said it did not succeed. The
    // whole reason the kind is the strongest claim in the eval is that this case is distinguishable
    // from the other two.
    let failed = read_event_stream_str(&[
        r#"{"format":"metaharness.event/1","seq":1,"run":"T-a9/1","event":"tool.requested","call_id":"c-1","name":"Skill","input":{"skill":"aep-planning:planning"},"decision_required":true,"seam":"control_request"}"#,
        r#"{"format":"metaharness.event/1","seq":2,"run":"T-a9/1","event":"tool.result","call_id":"c-1","is_error":false,"content":"the skill did not load","bytes":22,"tool_use_result":{"commandName":"aep-planning:planning","success":false}}"#,
        r#"{"format":"metaharness.event/1","seq":3,"run":"T-a9/1","event":"session.ended","is_error":false}"#,
    ]
    .join("\n"))
    .expect("a readable event stream");

    let row = "skill.completed: {skill: aep-planning:planning, count: {at_least: 1}}";
    assert_eq!(
        decide(&failed, "the-skill-ran-to-completion", row),
        Verdict::Gap,
        "a recorded `success: false` is the vendor answering the question, and the answer is no"
    );
}

#[test]
fn a_field_the_vendor_reported_as_null_is_unknown_and_never_ok() {
    // **The polarity that matters most**, and the one amendment a9 deliberately did not change: a
    // key that is present and `null` reads exactly as a key that was never there. Every row here
    // is one the honest step passes; the only difference is a vendor that reported nothing.
    //
    // Getting this wrong would be silent and total — every Codex-driven run reports `null` for
    // four of these five — so a checker that read absence as success would certify runs it had
    // stopped being able to judge.
    let ir = ir(NULL_A9);
    for (id, expect) in A9_SATISFIED {
        assert_eq!(
            decide(&ir, id, expect),
            Verdict::Unknown,
            "{id}: {expect}\n{}",
            trace_spec::render::report_to_text(&check(&one_row(id, expect), &ir, &[]))
        );
    }
}

#[test]
fn an_unknown_from_a_null_names_the_field_rather_than_the_conclusion() {
    // A verdict of `unk` is only actionable if it says what was missing. The reasons differ by
    // kind on purpose: a usage figure names the field, and `skill.completed` names the *result*
    // field it went looking for, because the run in front of the reader did invoke the skill.
    let ir = ir(NULL_A9);
    for (id, expect, field) in [
        (
            "billed-thinking",
            "tokens.thinking: {count: {at_most: 500}}",
            "usage.thinking_tokens",
        ),
        (
            "one-pass-per-request",
            "iterations: {count: {at_most: 5}}",
            "usage.iterations",
        ),
        (
            "served-at-standard-speed",
            "speed: {equals: standard}",
            "usage.speed",
        ),
        (
            "what-one-model-cost",
            "cost.total: {at_most_usd: 2.0, model: claude-sonnet-5}",
            "total_cost_usd",
        ),
    ] {
        let report = check(&one_row(id, expect), &ir, &[]);
        assert!(
            matches!(
                &report.expectations[0].outcome,
                Outcome::Undecidable(UnknownReason::FieldAbsent { field: named }) if named == field
            ),
            "{id} must name `{field}`: {:?}",
            report.expectations[0].outcome
        );
    }

    let report = check(
        &one_row(
            "the-skill-ran-to-completion",
            "skill.completed: {skill: aep-planning:planning, count: {at_least: 1}}",
        ),
        &ir,
        &[],
    );
    assert!(
        matches!(
            &report.expectations[0].outcome,
            Outcome::Undecidable(UnknownReason::ResultFieldAbsent { field, .. }) if field == "commandName"
        ),
        "the skill was invoked and the vendor recorded nothing about it: {:?}",
        report.expectations[0].outcome
    );
}

#[test]
fn a_cost_scoped_to_a_model_the_vendor_did_not_price_is_not_the_runs_own_figure() {
    // The scoping trap, and the reason `cost.total`'s model scope was worth carrying at all. This
    // run states `total_cost_usd`, so the *run-wide* row passes; the model's own entry prices
    // nothing, so the *scoped* row must not borrow the run's number to answer with.
    let ir = ir(NULL_A9);
    assert_eq!(
        decide(&ir, "what-the-run-cost", "cost.total: {at_most_usd: 2.0}"),
        Verdict::Ok,
        "the run's own figure is the vendor's and is unaffected by a9"
    );
    assert_eq!(
        decide(
            &ir,
            "what-one-model-cost",
            "cost.total: {at_most_usd: 2.0, model: claude-sonnet-5}"
        ),
        Verdict::Unknown,
        "a model that was used and priced nothing is `unk`, not a model that cost nothing"
    );
    assert_eq!(
        decide(
            &ir,
            "what-a-model-nobody-ran-cost",
            "cost.total: {at_most_usd: 2.0, model: claude-opus-5}"
        ),
        Verdict::Unknown,
        "and a scope that selects nothing stays undecidable rather than passing on an empty set"
    );
}

#[test]
fn a_recorded_transcript_and_a_driven_stream_take_the_same_arguments() {
    // Acceptance 2, at the seam a caller actually meets: one entry point, and the file says which
    // reader it needs. The recorded fixtures still read through the `stream-json` adapter, which
    // is what keeps two years of committed runs checkable.
    assert_eq!(detect(HONEST), TranscriptFormat::MetaharnessEventStream);
    assert_eq!(detect(RECORDED), TranscriptFormat::ClaudeStreamJson);

    let driven = read_any(HONEST).expect("a driven stream reads");
    let recorded = read_any(RECORDED).expect("a recorded transcript still reads");
    assert_eq!(driven.adapter.name, "metaharness/event-stream");
    assert_eq!(recorded.adapter.name, "claude-code/stream-json");
    assert_ne!(
        driven.transcript_digest, recorded.transcript_digest,
        "the digest names the bytes, so two runs are two runs"
    );
}

#[test]
fn the_same_stream_and_specification_produce_a_byte_identical_report() {
    // Invariant 9, on the new reader. Reading a file twice must produce the same IR, and checking
    // it twice must produce the same bytes — a report that moved between runs could not be
    // committed, diffed or used as evidence.
    let specification = document("expectations.driven-step.trace.yaml");
    let first = serde_json::to_string(&check(&specification, &ir(HONEST), &[]))
        .expect("a report renders as JSON");
    let second = serde_json::to_string(&check(&specification, &ir(HONEST), &[]))
        .expect("a report renders as JSON");
    assert_eq!(first, second);
    assert_eq!(ir(HONEST), ir(HONEST), "and the IR itself is stable");
}
