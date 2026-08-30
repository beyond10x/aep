//! Adversarial cases against `usage.trend` and `usage.share`.
//!
//! Written against commit `impl/usage-series-assertions` @ HEAD, base
//! `wave/reading-what-is-recorded`. Every case here is phrased against a claim the change makes
//! about itself — a doc comment, the CHANGELOG entry, or the story's acceptance — and not against
//! a preference of the reader's.
//!
//! Two of the four are **red on purpose**. They are the finding; the two green ones are the
//! evidence that the red pair is not a misreading.

use trace_domain::ir::{AdapterRef, AssistantRequest, TraceIr};
use trace_domain::spec::TraceSpec;
use trace_spec::adapter::read_transcript;
use trace_spec::check::check;
use trace_spec::codex::read_rollout;
use trace_spec::event_stream::read_event_stream;
use trace_spec::report::{Outcome, UnknownReason, Verdict};

/// The committed transcript of eval run `7hTYjT` — eight API requests, 2026-08-21.
const SEVEN_H: &[u8] = include_bytes!("fixtures/plugin-eval-7hTYjT.jsonl");

/// A committed Codex rollout: a reader that records no per-request usage at all.
const CODEX: &[u8] = include_bytes!("fixtures/codex-rollout.jsonl");

/// The three committed driven steps, read through the event-stream reader.
const DRIVEN_HONEST: &[u8] = include_bytes!("fixtures/metaharness-driven-honest-step.jsonl");
const DRIVEN_DENIAL: &[u8] = include_bytes!("fixtures/metaharness-driven-denial-step.jsonl");
const DRIVEN_NULL_A9: &[u8] = include_bytes!("fixtures/metaharness-driven-null-a9-step.jsonl");

/// The one-expectation document a case is about.
fn spec(expect: &str) -> TraceSpec {
    let text =
        format!("format: trace-spec/1\nid: adversarial\nexpectations:\n  - id: only\n    expect: {expect}\n");
    trace_domain::raw::read_spec(&text)
        .unwrap_or_else(|errors| panic!("this document must validate:\n{errors}"))
}

/// Whether `read_spec` accepts a one-expectation document at all.
fn accepted(expect: &str) -> bool {
    let text =
        format!("format: trace-spec/1\nid: adversarial\nexpectations:\n  - id: only\n    expect: {expect}\n");
    trace_domain::raw::read_spec(&text).is_ok()
}

fn adapter() -> AdapterRef {
    AdapterRef {
        name: "adversarial/synthetic",
        written_against: &[],
    }
}

/// An IR whose only content is a per-request `cache_read_input_tokens` series, one request each.
fn series_ir(values: &[u64]) -> TraceIr {
    let requests = values
        .iter()
        .enumerate()
        .map(|(index, value)| AssistantRequest {
            source_line: index + 1,
            request_id: Some(format!("req_{index}")),
            cache_read_input_tokens: Some(*value),
            ..AssistantRequest::default()
        })
        .collect();
    TraceIr::new("adversarial".to_owned(), adapter(), Vec::new(), requests)
}

/// The single row's outcome for one document against one run.
fn outcome(expect: &str, ir: &TraceIr) -> Outcome {
    check(&spec(expect), ir, &[]).expectations[0]
        .outcome
        .clone()
}

const RAMPS: &str = "{usage.trend: {field: cache_read_input_tokens, trend: non_decreasing}}";
const FALLS: &str = "{usage.trend: {field: cache_read_input_tokens, trend: non_increasing}}";

// --- RED ---------------------------------------------------------------------------------

/// The kind's own doc says it separates a healthy ramp from a shape that has gone flat.
///
/// `ExpectationKind::UsageTrend`, `crates/trace-domain/src/spec.rs`: *"`cache.read_tokens` says
/// how much a run read from the cache; it cannot say that the reads **ramped**, and a context
/// strategy that has quietly stopped working keeps a healthy total while the shape goes flat."*
/// `CHANGELOG.md`: *"This is what catches a context strategy that has quietly stopped working:
/// `cache.read_tokens` stays healthy while the ramp goes flat."*
///
/// Three runs with the **same** `cache_read_input_tokens` total, 313 513: the observed ramp, a
/// dead-flat line, and a run that read nothing for seven requests and everything on the eighth.
///
/// The flat line is caught — it moves nowhere, so it is consistent with both directions and
/// evidence for neither. **The spike is not, and that is a decided limit rather than a defect.**
/// `[0 x7, total]` and `[3, 3, 35]` have the same pair shapes — no wrong-way pair, one that moved,
/// the rest still — and differ only in *how many* stood still. Separating them needs a count, which
/// this story's `## Out of Scope` refuses. So `non_decreasing` asserts what it says, that the series
/// never falls, and a shape claim is a different kind that does not exist yet. Operator decision,
/// 2026-08-30; the doc and CHANGELOG claims about catching a spike were withdrawn rather than
/// left standing false.
#[test]
fn a_trend_separates_the_ramp_from_the_flat_line_it_says_it_catches() {
    let observed = [
        26_168, 36_616, 39_475, 40_299, 40_591, 40_965, 43_257, 46_142,
    ];
    let total: u64 = observed.iter().sum();
    let ramp = series_ir(&observed);
    let flat = series_ir(&[total / 8; 8]);
    let spike = series_ir(&[0, 0, 0, 0, 0, 0, 0, total]);

    let ramp_verdict = outcome(RAMPS, &ramp).verdict();
    assert_eq!(
        ramp_verdict,
        Verdict::Ok,
        "the observed ramp is the positive case"
    );

    let flat_outcome = outcome(RAMPS, &flat);
    assert_ne!(
        flat_outcome.verdict(),
        ramp_verdict,
        "a run whose cache reads never move once is the failure this kind names, and it gets \
         the ramp's own verdict: {}",
        flat_outcome.detail()
    );

    // The limit, pinned rather than papered over: a spike never falls, so it passes a
    // monotonicity claim. If this ever needs to fail, the kind that fails it is a shape
    // assertion with a count in it, and this line is where to come looking.
    let spike_outcome = outcome(RAMPS, &spike);
    assert_eq!(
        spike_outcome.verdict(),
        ramp_verdict,
        "a spike never falls, so a monotonicity claim holds over it — this kind is not a shape \
         assertion and does not pretend to be: {}",
        spike_outcome.detail()
    );
}

/// A run cannot both ramp and front-load the same field, and the committed run says it does.
///
/// `input_tokens` on `7hTYjT` is 2 on every one of the eight requests. Weak monotonicity makes
/// `non_decreasing` and `non_increasing` hold **at once**, so the pair of verdicts carries no
/// information about the run: whichever direction an author writes, the answer is `ok`.
#[test]
fn the_committed_run_does_not_satisfy_both_directions_of_one_trend_at_once() {
    let ir =
        read_transcript(SEVEN_H).expect("the committed fixture is a transcript this build reads");
    let up = outcome(
        "{usage.trend: {field: input_tokens, trend: non_decreasing}}",
        &ir,
    );
    let down = outcome(
        "{usage.trend: {field: input_tokens, trend: non_increasing}}",
        &ir,
    );
    assert!(
        !(up.verdict() == Verdict::Ok && down.verdict() == Verdict::Ok),
        "a constant series satisfies every direction, so neither verdict is evidence:\n  \
         non_decreasing -> {}\n  non_increasing -> {}",
        up.detail(),
        down.detail()
    );
}

/// A `usage.share` ceiling at or above 1 is a bound no run can fail.
///
/// The share is `peak / total` over non-negative terms with `total > 0` checked first, so it is
/// bounded by 1 by construction — the property below states that over 512 pseudo-random series
/// from a fixed seed, and it holds. `range_of` (`crates/trace-domain/src/raw.rs:2193`) already
/// refuses the other spelling of the same defect — *"a bound with no side accepts every value,
/// which is not a bound"* — and `ExpectationKind::UsageShare`'s own field doc says *"The
/// acceptable share, from 0 to 1"*. Validation accepts `at_most: 1.5`, `at_most: -0.5` and
/// `at_least: 2.0` all the same, and each is an expectation whose verdict is decided before the
/// run is read.
///
/// **Premise re-founded by the implementor, and only the premise.** It read the bound `(0, 1]`
/// off a live `at_most: 1.0` document, which is the document the case at the end of this file
/// argues must be refused — the premise would have been asserting the negation of a sibling
/// case's finding. It now reads `peak` and `total` out of the citation and compares them
/// directly, which is the same claim taken from the arithmetic rather than from a verdict. What
/// the case asserts is untouched.
// Lint-only, added by the implementor: the gate runs `clippy --all-targets -- -D warnings`
// and this file did not compile under it. Neither attribute changes a value or an assertion —
// the seed's digits and the modulo are exactly as the adversary wrote them.
#[allow(clippy::unusual_byte_groupings, clippy::cast_possible_truncation)]
#[test]
fn a_share_bound_outside_zero_to_one_is_refused_rather_than_decided_in_advance() {
    // The premise: the share never leaves (0, 1], so a ceiling of 1.5 can only ever say `ok`.
    let mut seed = 0x2026_08_30_u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % 100_000
    };
    for _ in 0..512 {
        let length = 1 + (next() as usize % 12);
        let values: Vec<u64> = (0..length).map(|_| next()).collect();
        if values.iter().sum::<u64>() == 0 {
            continue;
        }
        let ir = series_ir(&values);
        let detail = outcome(
            "{usage.share: {field: cache_read_input_tokens, at_most: 0.5}}",
            &ir,
        )
        .detail()
        .clone();
        let fraction = detail
            .split(" = ")
            .nth(1)
            .and_then(|rest| rest.split(" at request").next())
            .expect("the citation prints `peak / total`");
        let (peak, total) = fraction
            .split_once(" / ")
            .expect("the citation prints `peak / total`");
        let peak: u64 = peak.trim().parse().expect("a token count");
        let total: u64 = total.trim().parse().expect("a token count");
        assert!(
            peak > 0 && peak <= total,
            "the share is bounded by 1 by construction: {values:?} gave {detail}"
        );
    }

    // The finding: a ceiling that no run can exceed, and a floor no run can reach, are both
    // accepted as bounds.
    for written in [
        "{usage.share: {field: cache_creation_input_tokens, at_most: 1.5}}",
        "{usage.share: {field: cache_creation_input_tokens, at_most: -0.5}}",
        "{usage.share: {field: cache_creation_input_tokens, at_least: 2.0}}",
    ] {
        assert!(
            !accepted(written),
            "`{written}` is an expectation whose verdict does not depend on the run, and it \
             validates"
        );
    }
}

// --- GREEN: the two claims that hold, checked rather than taken -----------------------------

/// A reader that records no per-request usage leaves every series assertion `unk`, never `ok`.
///
/// The claim that keeps a blind reader honest: `codex.rs` builds no `AssistantRequest` at all, so
/// a vacuous pass here would make `usage.trend` green on every Codex run.
///
/// **Green.** It also shows that the story's `## Scope` is wrong about which readers are blind —
/// `event_stream.rs` is not one of them, and the case below is what that costs.
#[test]
fn a_reader_that_records_no_request_leaves_a_series_assertion_undecided() {
    let ir = read_rollout(CODEX).expect("the committed rollout reads");
    assert!(
        ir.requests.is_empty(),
        "the Codex reader records no per-request usage"
    );
    for written in [
        RAMPS,
        FALLS,
        "{usage.share: {field: cache_read_input_tokens, at_most: 0.5}}",
    ] {
        assert_eq!(
            outcome(written, &ir),
            Outcome::Undecidable(UnknownReason::NoRequests),
            "{written} must not be able to hold by selecting nothing"
        );
    }
}

/// Every series verdict on a **driven** run cites no event at all.
///
/// `event_stream.rs:356` builds an `AssistantRequest` from a `usage` line and pushes **no IR
/// event** for it, so the `source_line` -> event-index join in `check.rs`'s `events_on_line`
/// matches nothing: the request lines (11, 16, 24 on the honest step) are not among the event
/// lines. Every `usage.trend` and `usage.share` verdict on that reader — `ok` and `gap` alike —
/// arrives with an empty citation.
///
/// That contradicts two statements the code makes about itself:
///
/// * `report.rs:95` — a citation's events "may be empty for a fact that is a property of the
///   transcript as a whole ... **It is never empty for a fact read off an event.**"
/// * `check.rs:8` — "Every verdict cites what produced it ... `Outcome` has no shape for a
///   verdict with nothing to cite."
///
/// It is not an edge case: the event stream is the reader for every driven run, and all three
/// committed driven fixtures behave this way on all three checked expectations.
#[test]
fn a_series_verdict_on_a_driven_run_cites_the_events_it_was_read_from() {
    for (label, bytes) in [
        ("honest", DRIVEN_HONEST),
        ("denial", DRIVEN_DENIAL),
        ("null-a9", DRIVEN_NULL_A9),
    ] {
        let ir = read_event_stream(bytes).expect("the committed driven step reads");
        assert!(
            !ir.requests.is_empty(),
            "{label}: the event-stream reader does record per-request usage"
        );
        for written in [
            "{usage.trend: {field: input_tokens, trend: non_decreasing}}",
            "{usage.trend: {field: output_tokens, trend: non_decreasing}}",
            "{usage.share: {field: input_tokens, at_most: 0.6}}",
        ] {
            let found = outcome(written, &ir);
            assert_ne!(
                found.verdict(),
                Verdict::Unknown,
                "{label}: {written} is decidable on this reader"
            );
            assert!(
                !found.events().is_empty(),
                "{label}: {written} decided the run and cites nothing a reader can open: {}",
                found.detail()
            );
        }
    }
}

/// The `source_line` -> event-index join a series verdict cites resolves to the right events.
///
/// `events_on_line` is a derived join and nothing in the change asserts its output, so a checker
/// that cited no event — or the wrong one — would ship green. On `7hTYjT` the gap must cite the
/// events of the request that broke the trend and the pass those of the last request.
#[test]
fn a_series_verdict_cites_the_events_of_the_request_it_named() {
    let ir =
        read_transcript(SEVEN_H).expect("the committed fixture is a transcript this build reads");
    let series = ir.request_series();
    let events_of = |line: usize| -> Vec<usize> {
        ir.events
            .iter()
            .filter(|event| event.source_line == line)
            .map(|event| event.index)
            .collect()
    };

    let gap = outcome(FALLS, &ir);
    assert_eq!(gap.verdict(), Verdict::Gap);
    assert!(
        !gap.events().is_empty(),
        "a gap read off an event cites one"
    );
    assert_eq!(
        gap.events(),
        events_of(series[1].source_line).as_slice(),
        "the gap cites the request it named, request 1: {}",
        gap.detail()
    );

    let pass = outcome(RAMPS, &ir);
    assert_eq!(pass.verdict(), Verdict::Ok);
    assert_eq!(
        pass.events(),
        events_of(series[series.len() - 1].source_line).as_slice(),
        "the pass cites the last request in the series: {}",
        pass.detail()
    );
}

// ===========================================================================================
// SECOND PASS — the three decisions taken during the correction.
//
// Written against the corrected tree (`Trend::moves`, `UnknownReason::SeriesDidNotMove`,
// `TraceIr::events_of_request`, `share_of_a_total`). Nothing here re-attacks the six cases
// above.
// ===========================================================================================

/// The one shape the story's Outcome names by hand: *"cache creation is front-loaded"*.
const FRONT_LOADED: &str =
    "{usage.trend: {field: cache_creation_input_tokens, trend: non_increasing}}";

/// A seeded stream of small integers, so a property here is reproducible on any machine.
fn pseudo_random(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % 40
    }
}

/// One line of a `metaharness.event/1` stream.
fn driven(lines: &[String]) -> TraceIr {
    let text = lines.join("\n");
    read_event_stream(text.as_bytes()).expect("this synthetic driven step reads")
}

// --- decision 1: a flat pair takes the series to `unk` ---------------------------------------

/// A run that front-loads its cache creation *harder* must not lose the pass it had.
///
/// The story's `## Outcome` names this shape in as many words — *"cache creation is
/// front-loaded"* — and two committed driven steps are it:
///
/// | fixture | `cache_creation_input_tokens` per request |
/// |---|---|
/// | `metaharness-driven-denial-step` | 18 809, 0 |
/// | `metaharness-driven-honest-step` | 20 168, 0, 0 |
///
/// The second is the first with one more request that created no cache at all: strictly more
/// front-loaded, on a real transcript this repository committed. `non_increasing` passes the
/// two-request run and goes `unk` on the three-request one, because its last pair stood still.
///
/// The `unk`'s stated reason does not reach this series. `UnknownReason::SeriesDidNotMove`
/// (`crates/trace-spec/src/report.rs:254`) justifies itself with *"a pair of equal values is
/// consistent with `non_decreasing` and with `non_increasing` at once, so an `ok` would be a
/// verdict that holds whichever direction the author wrote"* — but `non_decreasing` over this
/// same series is a `gap` (20 168 -> 0 falls), so the two directions do **not** both hold and
/// the `ok` would have carried information. The rule is stated over a pair and applied to a
/// series.
#[test]
fn a_run_that_front_loads_harder_does_not_lose_the_pass_it_had() {
    let denial = read_event_stream(DRIVEN_DENIAL).expect("the committed driven step reads");
    let honest = read_event_stream(DRIVEN_HONEST).expect("the committed driven step reads");

    let two = outcome(FRONT_LOADED, &denial);
    assert_eq!(
        two.verdict(),
        Verdict::Ok,
        "the two-request run front-loads and the checker says so: {}",
        two.detail()
    );

    let opposite = outcome(
        "{usage.trend: {field: cache_creation_input_tokens, trend: non_decreasing}}",
        &honest,
    );
    assert_eq!(
        opposite.verdict(),
        Verdict::Gap,
        "the three-request run is not consistent with both directions — the other one gaps: {}",
        opposite.detail()
    );

    let three = outcome(FRONT_LOADED, &honest);
    assert_eq!(
        three.verdict(),
        Verdict::Ok,
        "one more request that created no cache is more front-loading, not less, and it turned \
         a pass into an undecidable: {}",
        three.detail()
    );
}

/// `unk` is reserved for a series that is consistent with **both** directions.
///
/// That is the whole argument `Trend::moves` (`crates/trace-domain/src/spec.rs:245`) and
/// `UnknownReason::SeriesDidNotMove` make for the third value: *"a repeated value is consistent
/// with both directions, so it is evidence for neither"*. `spec.rs:1330` states the biconditional
/// at the pair level and it holds there. This is the same claim at the level the **verdict** is
/// published at: a series the checker calls `gap` in one direction is not consistent with that
/// direction, so an `unk` in the other has nothing left to rest on.
///
/// Two named series, then 256 seeded ones. The first named series is a cache that fills and is
/// then fully reused — the healthiest shape a `cache_read_input_tokens` series has, and the one
/// `UsageField::CacheReadInputTokens` calls *"the ramp that says a context strategy is working"*.
#[test]
fn a_series_the_checker_gaps_in_one_direction_is_not_undecidable_in_the_other() {
    let mut series: Vec<Vec<u64>> = vec![vec![10, 20, 30, 40, 40, 40], vec![20_168, 0, 0]];
    let mut next = pseudo_random(0x2026_0830);
    for _ in 0..256 {
        let length = 2 + (usize::try_from(next()).expect("fits") % 6);
        series.push((0..length).map(|_| next()).collect());
    }

    for values in series {
        let ir = series_ir(&values);
        let up = outcome(RAMPS, &ir);
        let down = outcome(FALLS, &ir);
        assert!(
            !(up.verdict() == Verdict::Unknown && down.verdict() == Verdict::Gap),
            "{values:?}: non_increasing is a gap, so this series is not consistent with both \
             directions and non_decreasing has nothing to be undecided about:\n  up   -> {}\n  \
             down -> {}",
            up.detail(),
            down.detail()
        );
        assert!(
            !(down.verdict() == Verdict::Unknown && up.verdict() == Verdict::Gap),
            "{values:?}: the same, the other way round:\n  up   -> {}\n  down -> {}",
            up.detail(),
            down.detail()
        );
    }
}

// --- decision 2: `events_of_request`, the record's own line then its events by id -------------

/// A driven request that produced only reasoning and tool calls still cites the events it was
/// read from.
///
/// `TraceIr::events_of_request` (`crates/trace-domain/src/ir.rs:927`) has two paths and both
/// miss this request: its `usage` line builds no event, and its id is carried by
/// `EventKind::AssistantText` alone — `event_stream.rs:322` drops the `request_id` a `thinking`
/// event carries, and `EventKind::ToolCall` has no field for one. A turn that thought, called a
/// tool and said nothing to the operator is ordinary; `plugin-eval-7hTYjT.jsonl:16` is one on the
/// other reader.
///
/// Two statements this repository makes about itself:
///
/// * `report.rs:95` — a citation's events "may be empty for a fact that is a property of the
///   transcript as a whole ... **It is never empty for a fact read off an event.**"
/// * `check.rs:8` — "Every verdict cites what produced it."
#[test]
fn a_series_verdict_on_a_request_that_said_nothing_still_cites_its_events() {
    let stream = driven(&[
        r#"{"format":"metaharness.event/1","seq":1,"event":"session.started","adapter":"claude"}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":2,"event":"text","text":"Reading the file.","request_id":"req_a"}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":3,"event":"usage","request_id":"req_a","usage":{"output_tokens":100}}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":4,"event":"thinking","text":"No need to narrate this one.","request_id":"req_b"}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":5,"event":"tool.requested","call_id":"c1","name":"Bash","input":{"command":"ls"}}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":6,"event":"tool.result","call_id":"c1","is_error":false,"content":"a\n"}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":7,"event":"usage","request_id":"req_b","usage":{"output_tokens":300}}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":8,"event":"session.ended","is_error":false,"num_turns":1}"#.to_owned(),
    ]);
    assert_eq!(
        stream.request_series().len(),
        2,
        "two requests, one of them silent"
    );

    let pass = outcome(
        "{usage.trend: {field: output_tokens, trend: non_decreasing}}",
        &stream,
    );
    assert_eq!(pass.verdict(), Verdict::Ok, "100 then 300 moves up");
    assert!(
        !pass.events().is_empty(),
        "the pass names the second request and cites nothing a reader can open: {}",
        pass.detail()
    );

    let gap = outcome(
        "{usage.share: {field: output_tokens, at_most: 0.6}}",
        &stream,
    );
    assert_eq!(gap.verdict(), Verdict::Gap, "300 of 400 is 0.75");
    assert!(
        !gap.events().is_empty(),
        "a gap read off an event cites one, and this one accuses a request while pointing at \
         nothing: {}",
        gap.detail()
    );
}

/// A driven request whose `usage` line carries no `request_id` still cites its events.
///
/// `event_stream.rs:362` reads `request_id` as an `Option`, so a stream that omits it is this
/// wire — and `events_of_request` returns early with an empty vector for exactly that record.
/// The request's text is on the wire two lines up; nothing about the transcript is missing.
#[test]
fn a_series_verdict_on_an_unlabelled_request_still_cites_its_events() {
    let stream = driven(&[
        r#"{"format":"metaharness.event/1","seq":1,"event":"session.started","adapter":"claude"}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":2,"event":"text","text":"First.","request_id":"req_a"}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":3,"event":"usage","request_id":"req_a","usage":{"output_tokens":100}}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":4,"event":"text","text":"Second."}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":5,"event":"usage","usage":{"output_tokens":300}}"#.to_owned(),
        r#"{"format":"metaharness.event/1","seq":6,"event":"session.ended","is_error":false,"num_turns":1}"#.to_owned(),
    ]);
    assert_eq!(stream.request_series().len(), 2, "one labelled, one not");

    let pass = outcome(
        "{usage.trend: {field: output_tokens, trend: non_decreasing}}",
        &stream,
    );
    assert_eq!(pass.verdict(), Verdict::Ok);
    assert!(
        !pass.events().is_empty(),
        "the verdict names the unlabelled request and cites nothing: {}",
        pass.detail()
    );
}

// --- decision 3: the share bound is `[0, 1]` inclusive ---------------------------------------

/// The two endpoints of the accepted share range are not bounds.
///
/// `share_of_a_total` (`crates/trace-domain/src/raw.rs:2226`) refuses a share bound outside
/// `[0, 1]` because *"a bound no run can reach decides the verdict before the transcript is
/// opened"*. It accepts both endpoints, and all three of these are the same defect it names:
///
/// | written | what it can ever say |
/// |---|---|
/// | `at_most: 1.0` | `ok` or `unk` — never `gap`, since `peak <= total` |
/// | `at_least: 0.0` | `ok` or `unk` — never `gap`, since `peak >= 1` when `total > 0` |
/// | `at_most: 0.0` | `gap` or `unk` — never `ok`, for the same reason |
///
/// `at_least: 1.0` is **not** in the table and must stay accepted: it is the assertion *one
/// request took the whole run's total*, and the committed driven steps satisfy it for
/// `cache_creation_input_tokens`.
///
/// The premise is established from the citation's own arithmetic rather than from a bound this
/// case says should be refused, so the case does not depend on the thing it is arguing about.
#[test]
fn the_endpoints_of_the_share_range_are_no_ops_rather_than_bounds() {
    let mut next = pseudo_random(0x2026_0831);
    for _ in 0..256 {
        let length = 1 + (usize::try_from(next()).expect("fits") % 8);
        let values: Vec<u64> = (0..length).map(|_| next()).collect();
        if values.iter().sum::<u64>() == 0 {
            continue;
        }
        let detail = outcome(
            "{usage.share: {field: cache_read_input_tokens, at_most: 0.5}}",
            &series_ir(&values),
        )
        .detail()
        .clone();
        let fraction = detail
            .split(" = ")
            .nth(1)
            .and_then(|rest| rest.split(" at request").next())
            .expect("the citation prints `peak / total`");
        let (peak, total) = fraction
            .split_once(" / ")
            .expect("the citation prints `peak / total`");
        let peak: u64 = peak.trim().parse().expect("a token count");
        let total: u64 = total.trim().parse().expect("a token count");
        assert!(
            peak > 0 && peak <= total,
            "{values:?}: the share is `peak / total` over non-negative terms with a zero total \
             refused first, so it never leaves (0, 1]: {detail}"
        );
    }

    for written in [
        "{usage.share: {field: cache_creation_input_tokens, at_most: 1.0}}",
        "{usage.share: {field: cache_creation_input_tokens, at_least: 0.0}}",
        "{usage.share: {field: cache_creation_input_tokens, at_most: 0.0}}",
    ] {
        assert!(
            !accepted(written),
            "`{written}` is an expectation whose verdict does not depend on the run, and it \
             validates"
        );
    }

    assert!(
        accepted("{usage.share: {field: cache_creation_input_tokens, at_least: 1.0}}"),
        "a floor of 1 is a real assertion — one request took the whole total — and must survive"
    );
}

// --- the fourth probe: the two `unk` sentences a reader has to tell apart ---------------------

/// The series `unk` names `requests[].<field>` and the run-aggregate one names something else.
///
/// `series` (`crates/trace-spec/src/check.rs:1516`) argues for the spelling on the ground that
/// *"the run-aggregate kinds already use"* `usage.<field>`. This checks the consequence rather
/// than the reason: the two sentences a reader sees are different sentences.
#[test]
fn the_series_and_the_aggregate_name_different_missing_things() {
    let ir = read_rollout(CODEX).expect("the committed rollout reads");
    let blind = TraceIr::new(
        "adversarial".to_owned(),
        adapter(),
        ir.events.clone(),
        vec![AssistantRequest {
            source_line: 1,
            request_id: Some("req_0".to_owned()),
            ..AssistantRequest::default()
        }],
    );
    let series = outcome(RAMPS, &blind);
    assert_eq!(
        series,
        Outcome::Undecidable(UnknownReason::FieldAbsent {
            field: "requests[].cache_read_input_tokens".to_owned(),
        })
    );
    let aggregate = outcome("{cache.read_tokens: {count: {at_least: 1}}}", &blind);
    assert_ne!(
        format!("{series:?}"),
        format!("{aggregate:?}"),
        "two expectations reporting the same sentence for two different missing things is a \
         reader having to guess which one fired"
    );
}
