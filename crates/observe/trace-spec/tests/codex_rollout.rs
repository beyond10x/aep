//! Gap register `:38`: the harness-neutrality claim meets a second harness.
//!
//! Every behavioural document here is published as harness-neutral and, until this adapter, exactly
//! one existed. A vocabulary tested against one harness is a vocabulary shaped like that harness,
//! and nobody can tell which from the inside.
//!
//! # Which acceptance tier this is
//!
//! `docs/plan/harness-wave-4-governed-dogfood.md` § W4.4 names three. This is **partial**: the
//! reader exists and is tested, and no live Codex run was made — that costs money and needs a person
//! at the keyboard, and the gate reaches no network.
//!
//! # About the fixture
//!
//! Written by hand in the format verified against a local **codex-cli 0.145.0** install, not copied
//! from a session. Real rollouts are somebody's actual working transcripts and do not belong in a
//! public repository. The development runs against 400 real rollouts are reported in the changelog
//! with their numbers; what is *committed* is this synthetic file, and saying so is the point —
//! a fixture whose provenance is unstated is a fixture nobody can weigh.

use trace_spec::codex::{read_rollout_str, CODEX_ROLLOUT};

const ROLLOUT: &str = include_str!("fixtures/codex-rollout.jsonl");
const TORN: &str = include_str!("fixtures/codex-rollout-torn.jsonl");

/// The whole point: a second harness produces the same IR the first one does.
#[test]
fn a_codex_rollout_reads_into_the_same_ir_the_other_adapter_produces() {
    let ir = read_rollout_str(ROLLOUT).expect("a well-formed rollout reads");
    assert_eq!(ir.adapter.name, CODEX_ROLLOUT.name);

    let families: Vec<&str> = ir.events.iter().map(|e| e.kind.family()).collect();
    assert_eq!(
        families.iter().filter(|f| **f == "tool_call").count(),
        2,
        "a `function_call` and a `custom_tool_call` are both tool calls: {families:?}"
    );
    assert_eq!(
        families.iter().filter(|f| **f == "tool_result").count(),
        2,
        "{families:?}"
    );
    assert_eq!(
        families.iter().filter(|f| **f == "session_start").count(),
        1,
        "{families:?}"
    );
    assert!(
        families.contains(&"assistant_thinking"),
        "a `reasoning` item is thinking, not an opaque record — an opaque one would make every \
         tool expectation in the run `unk`: {families:?}"
    );
}

/// Correlation is `TraceIr::new`'s job, not the adapter's, and it works across both spellings.
#[test]
fn a_result_is_correlated_to_its_call_by_the_id_codex_writes() {
    let ir = read_rollout_str(ROLLOUT).expect("reads");
    let calls: Vec<_> = ir.events.iter().filter_map(|e| e.tool_call()).collect();
    assert_eq!(calls.len(), 2);
    for call in &calls {
        assert!(
            call.result_event.is_some(),
            "`{}` has a recorded output and must be correlated to it",
            call.name
        );
    }
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[1].name, "apply_patch");
}

/// The absences that must stay absences.
///
/// Three fields where an empty value would be a **claim** rather than a reading, and where reading
/// them as empty is how a specification silently decides something nobody observed.
#[test]
fn what_the_rollout_does_not_say_reads_as_unknown_and_not_as_empty() {
    let ir = read_rollout_str(ROLLOUT).expect("reads");

    let session = ir
        .events
        .iter()
        .find_map(|e| match &e.kind {
            trace_domain::ir::EventKind::SessionStart(start) => Some(start),
            _ => None,
        })
        .expect("a session_meta line");
    assert!(
        session.tools.is_none(),
        "a rollout does not publish the tool inventory; an empty list would answer `no tool was \
         available`, which is a claim nobody made"
    );
    assert_eq!(session.model.as_deref(), Some("gpt-5-codex"));

    for call in ir.events.iter().filter_map(|e| e.tool_call()) {
        assert!(
            call.operations.is_empty(),
            "the rollout does not say what a call *was*, so an operations-scoped row must read \
             `unk` rather than false. Mapping `apply_patch` to `file.write` here would be this \
             adapter answering a rendering's question, invisibly, in Rust"
        );
    }

    for result in ir.events.iter().filter_map(|e| match &e.kind {
        trace_domain::ir::EventKind::ToolResult(result) => Some(result),
        _ => None,
    }) {
        assert!(
            result.is_error.is_none(),
            "codex does not flag a result as an error on the record; `false` would be this \
             adapter asserting the call succeeded"
        );
    }
}

/// An event family this build has never seen is kept, never dropped.
///
/// `turn_context`, `exec_command_end` and `token_count` are all real families the adapter does not
/// interpret. Dropping them would let a checker report *the tool was never called* when what
/// happened is that it stopped being able to see tool calls.
#[test]
fn an_unrecognised_family_is_opaque_rather_than_discarded() {
    let ir = read_rollout_str(ROLLOUT).expect("reads");
    let opaque: Vec<_> = ir
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            trace_domain::ir::EventKind::Opaque(record) => Some(record),
            _ => None,
        })
        .collect();
    assert!(
        !opaque.is_empty(),
        "the fixture carries families this build does not read"
    );
    let kept: Vec<&str> = opaque
        .iter()
        .filter_map(|record| record.subtype.as_deref())
        .collect();
    assert!(kept.contains(&"exec_command_end"), "{kept:?}");
    assert!(kept.contains(&"token_count"), "{kept:?}");

    assert_eq!(
        ir.events.len(),
        ROLLOUT.lines().filter(|l| !l.trim().is_empty()).count(),
        "every line of the file is represented by exactly one event; a line that yielded nothing \
         recognisable is still a record"
    );
}

/// A torn line is refused by its line number rather than recovered from.
///
/// **This is a verified finding about the format, not a hypothesis.** Reading 400 real rollouts from
/// a local codex-cli 0.145.0 install, 26 were refused, and in 23 of them a record began *mid-line* —
/// two JSON objects concatenated with no newline between them, which is a torn append rather than a
/// truncation (only 3 were at end-of-file).
///
/// Refusing is the right answer and the same one the Claude adapter gives. Recovering would mean
/// guessing where a record starts, and a reader that guesses produces records nobody can trust —
/// which is worse here than elsewhere, because the whole purpose of the reader is to be the thing a
/// verdict rests on.
#[test]
fn a_record_that_begins_mid_line_is_refused_and_says_which_line() {
    let errors = read_rollout_str(TORN).expect_err("a torn line is not a readable record");
    let said = errors.to_string();
    assert!(
        said.contains("line 4"),
        "the refusal names the line: {said}"
    );
    assert!(said.contains("TRACE-ADAPT-001"), "{said}");
}

/// The two refusals are the same two the other adapter has, which is itself evidence about the seam.
#[test]
fn an_empty_rollout_is_refused_rather_than_judged_as_a_run_that_did_nothing() {
    let errors = read_rollout_str("\n\n").expect_err("nothing to judge");
    assert!(errors.to_string().contains("TRACE-ADAPT-002"), "{errors}");
}
