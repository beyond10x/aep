//! A negative expectation is decided when the producer says the stream is whole.
//!
//! # The question this file is about
//!
//! `tool.absent` asks *"did this never happen?"*, and a transcript can answer it only if it is the
//! whole record. Nothing in the wire used to say so, so the checker fell back to the only signal it
//! had — an event it could not read — and answered `unk` on every run that carried one. Eight paid
//! runs on 2026-09-03 ended "undecided" on exactly those rows while carrying twenty-three
//! `opaque` vendor lines apiece, which is the shape this file pins.
//!
//! metaharness owns the stream and closes it. `stream.closed` is its statement that what was
//! written is what happened, and `hermetic.stream_complete` is the same statement made in the
//! attestation. Either one decides an absence; neither one is inferred from the stream's shape,
//! because a truncated capture looks exactly like a short run.
//!
//! # The three fixtures
//!
//! | fixture | what it stands for |
//! |---|---|
//! | `metaharness-closed-stream.jsonl` | the producer wrote its closing event as the last line |
//! | `metaharness-open-stream.jsonl` | the same run without it — every recording made before the marker existed |
//! | `metaharness-attested-complete-stream.jsonl` | no closing event, and an attestation that states completeness |
//!
//! All three are **synthesized**, on `tests/event_stream.rs`'s own terms: a number here is a number
//! this file chose, so a failure is a change in this repository and never a finding about a
//! harness.

use trace_domain::ir::{ClosureWitness, TraceIr};
use trace_domain::spec::TraceSpec;
use trace_spec::check::check;
use trace_spec::reader::read_any;
use trace_spec::report::{Outcome, UnknownReason, Verdict};

/// The run whose producer closed the stream.
const CLOSED: &[u8] = include_bytes!("fixtures/metaharness-closed-stream.jsonl");

/// The same run with the closing line removed.
const OPEN: &[u8] = include_bytes!("fixtures/metaharness-open-stream.jsonl");

/// The same run again, closed by the attestation rather than by an event.
const ATTESTED: &[u8] = include_bytes!("fixtures/metaharness-attested-complete-stream.jsonl");

/// A specification with one negative row in it, over a tool the run never called.
const ABSENCE: &str = "\
format: trace-spec/1
id: eval-case/absence
title: nothing was written
expectations:
  - id: nothing-was-written
    statement: no write verb touched the tree
    expect:
      tool.absent:
        tools: [Write, Edit]
";

/// A specification whose negative row the run **contradicts**: it read a file.
const CONTRADICTED: &str = "\
format: trace-spec/1
id: eval-case/absence-contradicted
title: nothing was read
expectations:
  - id: nothing-was-read
    statement: no read verb touched the tree
    expect:
      tool.absent:
        tools: [Read]
";

fn ir(bytes: &[u8]) -> TraceIr {
    read_any(bytes).expect("the committed fixture is a stream this build reads")
}

fn spec(text: &str) -> TraceSpec {
    trace_domain::raw::read_spec(text).unwrap_or_else(|errors| panic!("{errors}"))
}

fn only_row(bytes: &[u8], document: &str) -> trace_spec::report::ExpectationReport {
    let report = check(&spec(document), &ir(bytes), &[]);
    report
        .expectations
        .first()
        .cloned()
        .expect("the document declares one expectation")
}

#[test]
fn a_closing_event_decides_an_absence_that_nothing_contradicted() {
    let row = only_row(CLOSED, ABSENCE);
    assert_eq!(row.verdict, Verdict::Ok, "{:?}", row.outcome);
    let Outcome::Ok(citation) = &row.outcome else {
        panic!("a closed stream decides an absence: {:?}", row.outcome);
    };
    assert!(
        citation.note.contains("stream.closed"),
        "the verdict names the witness that decided it: {}",
        citation.note
    );
}

#[test]
fn an_unread_event_no_longer_poisons_an_absence_on_a_closed_stream() {
    // The fixture carries one `opaque` vendor line, which is what made all eight paid runs
    // undecidable. The row is decided and the unread line stays on the record rather than
    // disappearing into a green verdict.
    let ir = ir(CLOSED);
    assert_eq!(ir.opaque_events().len(), 1, "the fixture carries one");
    let row = only_row(CLOSED, ABSENCE);
    let Outcome::Ok(citation) = &row.outcome else {
        panic!("{:?}", row.outcome);
    };
    assert!(
        citation.note.contains("1 unread event"),
        "the unread line is still named: {}",
        citation.note
    );
}

#[test]
fn an_absence_the_run_contradicted_is_a_gap_on_a_closed_stream() {
    let row = only_row(CLOSED, CONTRADICTED);
    assert_eq!(row.verdict, Verdict::Gap, "{:?}", row.outcome);
}

#[test]
fn without_the_marker_the_row_stays_unknown_and_the_report_names_what_is_missing() {
    let row = only_row(OPEN, ABSENCE);
    assert_eq!(row.verdict, Verdict::Unknown, "{:?}", row.outcome);
    let Outcome::Undecidable(UnknownReason::StreamNotClosed { markers }) = &row.outcome else {
        panic!("the reason names the missing marker: {:?}", row.outcome);
    };
    assert_eq!(
        markers,
        &vec![
            "stream.closed".to_owned(),
            "hermetic.stream_complete".to_owned()
        ],
        "both witnesses are named, so an operator knows which two things would decide it"
    );
    let Outcome::Undecidable(reason) = &row.outcome else {
        unreachable!("matched above")
    };
    let sentence = reason.to_string();
    assert!(
        sentence.contains("stream.closed") && sentence.contains("whole run"),
        "{sentence}"
    );
}

#[test]
fn a_gap_does_not_need_the_marker() {
    // An absence a call contradicts is contradicted whether or not the record is whole: reading
    // more of a truncated stream can only add calls, never remove the one already cited.
    let row = only_row(OPEN, CONTRADICTED);
    assert_eq!(row.verdict, Verdict::Gap, "{:?}", row.outcome);
}

#[test]
fn the_attestation_is_the_second_witness_and_decides_the_same_row() {
    let ir = ir(ATTESTED);
    let close = ir.stream_close.as_ref().expect("the attestation states it");
    assert_eq!(close.witness, ClosureWitness::Attestation);
    assert_eq!(close.events, Some(6));
    assert_eq!(close.reason.as_deref(), Some("completed"));
    assert_eq!(only_row(ATTESTED, ABSENCE).verdict, Verdict::Ok);
}

#[test]
fn the_closing_event_is_read_and_is_not_an_unread_event_itself() {
    let ir = ir(CLOSED);
    let close = ir.stream_close.as_ref().expect("the last line closes it");
    assert_eq!(close.witness, ClosureWitness::ClosingEvent);
    assert_eq!(close.events, Some(7));
    assert_eq!(close.reason.as_deref(), Some("completed"));
    assert_eq!(
        ir.opaque_events().len(),
        1,
        "the marker is understood, not unread: routing it through the opaque path would have \
         poisoned the very rows it exists to decide"
    );
}

#[test]
fn a_vendor_transcript_is_closed_by_its_terminal_record_and_a_truncated_one_is_not() {
    // The other wire has no marker of its own, and it does not need one: the vendor writes its
    // terminal record and stops, so a file that ends with it ends where the run did. A capture
    // cut off anywhere else does not, and says so with the marker it was looking for.
    let recorded = include_bytes!("fixtures/plugin-eval-7hTYjT.jsonl");
    let close = ir(recorded)
        .stream_close
        .expect("the committed run ends with its result record");
    assert_eq!(close.witness, ClosureWitness::TerminalRecord);

    let text = String::from_utf8(recorded.to_vec()).expect("the fixture is UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    let truncated = lines[..lines.len() - 1].join("\n");
    assert!(
        ir(truncated.as_bytes()).stream_close.is_none(),
        "a transcript with its last record cut off states nothing about its own wholeness"
    );
}

#[test]
fn a_session_boundary_is_not_a_stream_boundary_on_the_metaharness_wire() {
    // Why `session.ended` is not read as this wire's closing record even though it is the last
    // line of the open fixture: that wire carries one terminal record **per session**, and a
    // driven run is a concatenation of them, so a subagent's `session.ended` would close a stream
    // in the middle of the run that wrote it.
    assert!(
        ir(OPEN).run_outcome().is_some(),
        "the open fixture does end with a terminal record"
    );
    assert!(ir(OPEN).stream_close.is_none());
}
