//! A write is a write whichever verb made it — and an unread event does not unmake one.
//!
//! # What this file exists for, and the run that bought it
//!
//! The first live pilot of the eval programme, 2026-08-23, asked Claude Code to write a failing
//! test before the code. It did: `git status` in the run's working clone shows
//! `crates/protocol-cli/tests/planning_cli.rs` and `crates/protocol-cli/src/planning.rs` both
//! modified. The corpus reported `the-test-came-before-the-code` as **undecidable**, reason
//! `never_occurred`, selector `Write(file_path glob "*/tests/*")` — because the run used **`Edit`**
//! on files that already existed, and no `Write` event ever occurred.
//!
//! That is the worst kind of wrong verdict: not a gap somebody would investigate, but a shrug about
//! work that visibly happened. The fix is a selector that scopes to the **set** of verbs that put
//! bytes in a file, and the tests below are what stop it from being a one-line change nobody
//! checked.
//!
//! # The claim did not get weaker, and this file is where that is shown
//!
//! Widening the witness set is not weakening the bound. `the_order_is_contradicted_when_the_code_came_first`
//! is the mutation: the same document, the same tool, the same paths — with the two edits swapped —
//! must come back a **gap**. A selector that reported `ok` for both orderings would have widened
//! the claim into nothing, and that is the failure this file refuses.
//!
//! # The other half: an unread event and an existence claim
//!
//! The same pilot reported `the-editor-was-used-at-all` (`tool.called`) as undecidable with reason
//! `opaque_events`, over two `system` events, in a stream carrying 56 readable tool events. That
//! reads like a checker bug and is not one: the selector was `Write`, **nothing matched**, and with
//! zero observed calls an unread event genuinely could have been the missing `Write`. `unk` was the
//! correct answer to the question actually asked.
//!
//! `decide_count` (`crates/trace-spec/src/check.rs`) already does the three-valued reasoning the
//! module documentation there argues for: an unread event can only *add* calls, so `at_least: 1`
//! with one call already seen holds whatever the unread event was. The two tests at the bottom pin
//! both polarities against a real opaque event, so that the semantics is held by a test rather than
//! by having been read once.

use trace_domain::spec::TraceSpec;
use trace_spec::check::check;
use trace_spec::event_stream::read_event_stream_str;
use trace_spec::report::Verdict;

/// The verbs Claude Code puts bytes in a file with.
///
/// Not invented here: `crates/protocol-cli/src/drive.rs` renders exactly `Edit`, `Write` and
/// `NotebookEdit` to the `repository.write` capability, and this repository's own driver is the
/// authority on what a write is. Keeping one list means a fourth verb is added in one place.
const WRITE_TOOLS: [&str; 3] = ["Edit", "NotebookEdit", "Write"];

/// One `tool.requested` line, as the seam writes one.
fn call(seq: usize, tool: &str, file: &str) -> String {
    format!(
        r#"{{"format":"metaharness.event/1","seq":{seq},"run":"T/1","event":"tool.requested","call_id":"c-{seq}","name":"{tool}","input":{{"file_path":"{file}"}},"decision_required":false,"seam":"none"}}"#
    )
}

/// A stream of the given calls, terminated the way a finished session is.
fn stream(calls: &[String]) -> trace_domain::ir::TraceIr {
    let mut lines = vec![
        r#"{"format":"metaharness.event/1","seq":0,"run":"T/1","event":"session.started","adapter":"claude","offered_tools":["Bash","Edit","Write"],"mcp_servers":[]}"#.to_owned(),
    ];
    lines.extend(calls.iter().cloned());
    lines.push(
        r#"{"format":"metaharness.event/1","seq":99,"run":"T/1","event":"session.ended","is_error":false,"terminal_reason":"completed","permission_denials":[]}"#
            .to_owned(),
    );
    read_event_stream_str(&lines.join("\n")).expect("a readable event stream")
}

/// A one-row document asserting that a write under `first` precedes a write under `before`.
///
/// Written as a **document**, not built from the types, because the document is the thing the
/// corpus ships and `tools:` is a new spelling in it: a helper that constructed the selector
/// directly would leave the parser — the half that can refuse — untested.
fn ordering(first: &str, before: &str) -> TraceSpec {
    let set = WRITE_TOOLS.join(", ");
    document(&format!(
        "  - id: the-test-came-before-the-code\n    \
             statement: the failing test was written before the source it judges\n    \
             expect:\n      \
             order:\n        \
             first: {{tools: [{set}], args: {{file_path: {{glob: \"{first}\"}}}}}}\n        \
             before: {{tools: [{set}], args: {{file_path: {{glob: \"{before}\"}}}}}}\n"
    ))
}

/// A specification with the given expectation rows.
fn document(rows: &str) -> TraceSpec {
    let text = format!("format: trace-spec/1\nid: write-selectors/case\nexpectations:\n{rows}");
    trace_domain::raw::read_spec(&text).unwrap_or_else(|errors| panic!("{text}\n{errors}"))
}

/// The verdict of the single row in a document, against a stream.
fn verdict(spec: &TraceSpec, ir: &trace_domain::ir::TraceIr) -> Verdict {
    let report = check(spec, ir, &[]);
    assert_eq!(report.expectations.len(), 1);
    report.expectations[0].verdict
}

#[test]
fn an_edit_witnesses_a_write_the_way_a_write_does() {
    // The pilot's exact shape, in miniature: two `Edit` calls, the test file first. Under the old
    // one-name selector this stream produced `never_occurred`; it must now decide, and decide `ok`.
    let ir = stream(&[
        call(1, "Edit", "/w/crates/protocol-cli/tests/planning_cli.rs"),
        call(2, "Edit", "/w/crates/protocol-cli/src/planning.rs"),
    ]);
    assert_eq!(
        verdict(&ordering("*/tests/*", "*/src/*"), &ir),
        Verdict::Ok,
        "an `Edit` under tests/ before an `Edit` under src/ is the ordering the row asserts"
    );
}

#[test]
fn the_order_is_contradicted_when_the_code_came_first() {
    // **The mutation that proves the widening did not weaken the claim.** Same document, same
    // tool, same two paths — swapped. A selector that answered `ok` here would have turned an
    // ordering assertion into a statement that two files exist.
    let ir = stream(&[
        call(1, "Edit", "/w/crates/protocol-cli/src/planning.rs"),
        call(2, "Edit", "/w/crates/protocol-cli/tests/planning_cli.rs"),
    ]);
    assert_eq!(
        verdict(&ordering("*/tests/*", "*/src/*"), &ir),
        Verdict::Gap,
        "the code was written first, and the row must say so"
    );
}

#[test]
fn every_verb_in_the_set_witnesses_the_claim_and_a_read_does_not() {
    // The set is the set, one member at a time — and the negative control is the reason the tool
    // scope was kept at all. Dropping it and matching on `file_path` alone would have let a `Read`
    // of the test file satisfy "the test was written first", which is a different sentence.
    for verb in WRITE_TOOLS {
        let ir = stream(&[
            call(1, verb, "/w/crates/protocol-cli/tests/planning_cli.rs"),
            call(2, verb, "/w/crates/protocol-cli/src/planning.rs"),
        ]);
        assert_eq!(
            verdict(&ordering("*/tests/*", "*/src/*"), &ir),
            Verdict::Ok,
            "`{verb}` puts bytes in a file and must witness the ordering"
        );
    }

    let read_then_edit = stream(&[
        call(1, "Read", "/w/crates/protocol-cli/tests/planning_cli.rs"),
        call(2, "Edit", "/w/crates/protocol-cli/src/planning.rs"),
    ]);
    assert_eq!(
        verdict(&ordering("*/tests/*", "*/src/*"), &read_then_edit),
        Verdict::Unknown,
        "reading the test first is not writing it first: nothing witnesses the first side, and \
         `unk` — not `ok` — is what that means"
    );
}

// --- the opaque-event polarity, pinned rather than believed ------------------------------------

/// A stream carrying one event this build cannot read, beside the given calls.
fn stream_with_an_unread_event(calls: &[String]) -> trace_domain::ir::TraceIr {
    let mut lines = vec![
        r#"{"format":"metaharness.event/1","seq":0,"run":"T/1","event":"session.started","adapter":"claude","offered_tools":["Edit"],"mcp_servers":[]}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":1,"run":"T/1","event":"system"}"#.to_owned(),
    ];
    lines.extend(calls.iter().cloned());
    lines.push(
        r#"{"format":"metaharness.event/1","seq":99,"run":"T/1","event":"session.ended","is_error":false,"terminal_reason":"completed","permission_denials":[]}"#
            .to_owned(),
    );
    read_event_stream_str(&lines.join("\n")).expect("a readable event stream")
}

/// A one-row `tool.called` document over the write set.
fn existence() -> TraceSpec {
    let set = WRITE_TOOLS.join(", ");
    document(&format!(
        "  - id: the-editor-was-used-at-all\n    \
             statement: the run wrote files\n    \
             expect:\n      \
             tool.called: {{tools: [{set}], count: {{at_least: 1}}}}\n"
    ))
}

#[test]
fn an_unread_event_does_not_unmake_a_write_that_was_observed() {
    // The pilot's second surprise, answered. One readable `Edit` proves a write happened, and an
    // event the adapter could not read can only *add* calls — so `at_least: 1` holds whatever the
    // unread event was. Reporting `unk` here would be timid rather than careful, and it would make
    // every existence claim in the corpus hostage to a harness emitting one housekeeping record.
    let ir = stream_with_an_unread_event(&[call(2, "Edit", "/w/src/planning.rs")]);
    assert_eq!(
        ir.opaque_events().len(),
        1,
        "the fixture must actually carry an unread event, or this test proves nothing"
    );
    assert_eq!(verdict(&existence(), &ir), Verdict::Ok);
}

#[test]
fn an_unread_event_leaves_an_existence_claim_undecided_when_nothing_matched() {
    // The other polarity, and the one the pilot actually hit: with **zero** observed calls the
    // unread event could have been the write, so `unk` is the honest answer rather than a
    // contradiction. This is what makes the test above a refinement instead of a hole — and it is
    // why the pilot's `unk` was a wrong *selector*, not a wrong checker.
    let ir = stream_with_an_unread_event(&[]);
    assert_eq!(verdict(&existence(), &ir), Verdict::Unknown);

    // And an absence claim stays undecidable whatever else is in the stream: the forbidden call
    // could be hiding in the event nobody could read. That asymmetry is deliberate and is the
    // reason `the-store-was-never-hand-edited` still reports `unk` on the pilot's transcript.
    let with_a_write = stream_with_an_unread_event(&[call(2, "Edit", "/w/src/planning.rs")]);
    let set = WRITE_TOOLS.join(", ");
    let absent = document(&format!(
        "  - id: nothing-was-written-under-the-store\n    \
             statement: no artifact file was written directly\n    \
             expect:\n      \
             tool.absent: {{tools: [{set}], args: {{file_path: {{glob: \"*/.engineering/planning/*\"}}}}}}\n"
    ));
    assert_eq!(
        verdict(&absent, &with_a_write),
        Verdict::Unknown,
        "an absence cannot be proved past an event the adapter could not read"
    );
}

// --- the operation, which is the claim the verbs were standing in for -----------------------

/// One `tool.requested` from a three-verb surface: one tool name, the entry inside the call, and
/// the operation the harness resolved it to.
fn invocation(seq: usize, entry: &str, operation: &str, file: &str) -> String {
    format!(
        r#"{{"format":"metaharness.event/1","seq":{seq},"run":"T/1","event":"tool.requested","call_id":"c-{seq}","name":"tool_invoke","operations":["{operation}"],"input":{{"name":"{entry}","file_path":"{file}"}},"decision_required":false,"seam":"none"}}"#
    )
}

/// One `tool.requested` that started a program, as the seam resolves one.
fn started_program(seq: usize, program: &str) -> String {
    format!(
        r#"{{"format":"metaharness.event/1","seq":{seq},"run":"T/1","event":"tool.requested","call_id":"c-{seq}","name":"tool_invoke","operations":["shell"],"subjects":["proc:{program}"],"input":{{"name":"run","arguments":{{"argv":["{program}"]}}}},"decision_required":false,"seam":"none"}}"#
    )
}

/// A one-row existence document over a scope written however the caller spells it.
fn existence_scoped(scope: &str) -> TraceSpec {
    document(&format!(
        "  - id: the-editor-was-used-at-all\n    \
             statement: the run wrote files\n    \
             expect:\n      \
             tool.called:\n        \
             {scope}\n        \
             count: {{at_least: 1}}\n"
    ))
}

#[test]
fn an_operation_witnesses_a_write_under_a_tool_name_no_document_lists() {
    // The failure this replaces, twice over: `tool: Write` shrugged at a run that used `Edit`, and
    // the widened `tools:` set then shrugged at a harness that spelled a write `workspace_write`.
    // Every fix was one more name. The operation is the claim, and it needs none of them.
    let ir = stream(&[invocation(1, "file_write", "file.write", "a/src/lib.rs")]);

    assert_eq!(
        verdict(
            &existence_scoped("operations: [file.write, file.edit]"),
            &ir
        ),
        Verdict::Ok,
        "the harness said what the call was"
    );
    let set = WRITE_TOOLS.join(", ");
    assert_eq!(
        verdict(&existence_scoped(&format!("tools: [{set}]")), &ir),
        Verdict::Gap,
        "and a document that knew only Claude Code's verbs reports that the run wrote nothing"
    );
}

#[test]
fn a_call_the_harness_did_not_resolve_never_witnesses_an_operation() {
    // Silence is not a match. Widening the vocabulary must not turn *this harness did not say* into
    // *this harness wrote a file*, or the row reports a verdict nobody earned.
    let ir = stream(&[call(1, "Write", "a/src/lib.rs")]);
    assert_eq!(
        verdict(
            &existence_scoped("operations: [file.write, file.edit]"),
            &ir
        ),
        Verdict::Gap,
        "nothing matched, and the stream has no unread event that could have been the write"
    );
}

#[test]
fn naming_both_vocabularies_decides_against_either_stream() {
    // The union, and the reason the corpus writes both keys: `stream-json` carries tool names and
    // no operations, `metaharness.event/1` carries both. Written as an intersection this row would
    // go `unk` against the entire Claude arm while reading like a widening.
    let set = WRITE_TOOLS.join(", ");
    let both = format!("operations: [file.write, file.edit]\n        tools: [{set}]");

    let named_only = stream(&[call(1, "Edit", "a/src/lib.rs")]);
    let resolved_only = stream(&[invocation(1, "file_write", "file.write", "a/src/lib.rs")]);
    for ir in [&named_only, &resolved_only] {
        assert_eq!(verdict(&existence_scoped(&both), ir), Verdict::Ok);
    }

    // And it did not become "any call at all": a read is neither verb nor operation.
    let read = stream(&[invocation(1, "file_read", "file.read", "a/src/lib.rs")]);
    assert_eq!(verdict(&existence_scoped(&both), &read), Verdict::Gap);
}

#[test]
fn an_operation_scoped_ordering_still_contradicts_the_wrong_order() {
    // The mutation this file exists for, carried onto the new spelling. A scope that reported `ok`
    // for both orderings would have widened the claim into nothing.
    let set = WRITE_TOOLS.join(", ");
    let scoped = |glob: &str| {
        format!("{{operations: [file.write, file.edit], tools: [{set}], args: {{file_path: {{glob: \"{glob}\"}}}}}}")
    };
    let spec = document(&format!(
        "  - id: the-test-came-before-the-code\n    \
             statement: the failing test was written before the source it judges\n    \
             expect:\n      \
             order:\n        \
             first: {}\n        \
             before: {}\n",
        scoped("*/tests/*"),
        scoped("*/src/*")
    ));

    let right_way = stream(&[
        invocation(1, "file_write", "file.write", "a/tests/t.rs"),
        invocation(2, "file_edit", "file.edit", "a/src/lib.rs"),
    ]);
    assert_eq!(verdict(&spec, &right_way), Verdict::Ok);

    let wrong_way = stream(&[
        invocation(1, "file_edit", "file.edit", "a/src/lib.rs"),
        invocation(2, "file_write", "file.write", "a/tests/t.rs"),
    ]);
    assert_eq!(
        verdict(&spec, &wrong_way),
        Verdict::Gap,
        "the code came first and the row says so"
    );
}

// --- which file, which is the other half of the claim -------------------------------------------

#[test]
fn a_subject_scope_decides_a_path_row_on_a_harness_that_names_every_call_tool_invoke() {
    // `operations` says a write happened; this says which file. Without both, an ordering row
    // reading `args.file_path` finds nothing on a three-verb surface — the path is nested a level
    // down inside the call — and every such row reported `unk` for the whole arm.
    let ir = stream(&[
        invocation(1, "file_write", "file.write", "a/tests/t.rs"),
        invocation(2, "file_edit", "file.edit", "a/src/lib.rs"),
    ]);
    let scoped = |glob: &str| {
        format!("{{operations: [file.write, file.edit], subject: {{glob: \"file:{glob}\"}}}}")
    };
    let spec = document(&format!(
        "  - id: the-test-came-before-the-code\n    \
             statement: the failing test was written before the source it judges\n    \
             expect:\n      \
             order:\n        \
             first: {}\n        \
             before: {}\n",
        scoped("*/tests/*"),
        scoped("*/src/*")
    ));
    assert_eq!(verdict(&spec, &ir), Verdict::Ok);
}

#[test]
fn a_subject_matcher_reads_the_scheme_so_a_file_row_cannot_select_a_program() {
    // The prefix is part of the value on purpose. Dropping it would let a row about a file select
    // a program with a similar name, which is a widening nobody wrote.
    let ir = stream(&[started_program(1, "/usr/bin/python3")]);
    let program = document(
        "  - id: it-started-a-program\n    statement: a program was started\n    \
             expect:\n      tool.called:\n        subject: {glob: \"proc:*\"}\n        \
             count: {at_least: 1}\n",
    );
    let file = document(
        "  - id: it-touched-a-file\n    statement: a file was touched\n    \
             expect:\n      tool.called:\n        subject: {glob: \"file:*\"}\n        \
             count: {at_least: 1}\n",
    );
    assert_eq!(verdict(&program, &ir), Verdict::Ok);
    assert_eq!(verdict(&file, &ir), Verdict::Gap, "a program is not a file");
}

#[test]
fn a_vendor_transcripts_own_path_argument_is_read_into_the_same_form() {
    // `subject:` has to decide on a record that states a path and never says so neutrally, or the
    // corpus would need two spellings of every path row for the rest of time. *Which key holds a
    // path* is the same three names everywhere; *which tool is a write* is not, and stays the
    // harness's to answer.
    let ir = stream(&[call(1, "Write", "a/src/lib.rs")]);
    let spec = document(
        "  - id: it-wrote-the-source\n    statement: the source was written\n    \
             expect:\n      tool.called:\n        tools: [Edit, NotebookEdit, Write]\n        \
             subject: {glob: \"file:*/src/*\"}\n        count: {at_least: 1}\n",
    );
    assert_eq!(verdict(&spec, &ir), Verdict::Ok);
}

#[test]
fn a_call_whose_record_names_no_subject_never_satisfies_a_subject_row() {
    // Silence is not a match, on the same rule an absent argument carries: reading it as one would
    // let a row about `*/tests/*` select a call about anything at all.
    let ir = stream(&[started_program(1, "/usr/bin/python3")]);
    let spec = document(
        "  - id: it-touched-the-tests\n    statement: a test file was touched\n    \
             expect:\n      tool.called:\n        subject: {glob: \"file:*/tests/*\"}\n        \
             count: {at_least: 1}\n",
    );
    assert_eq!(verdict(&spec, &ir), Verdict::Gap);
}
