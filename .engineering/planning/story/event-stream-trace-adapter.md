---
format: aep.planning-md/1
id: story:event-stream-trace-adapter
kind: story
status: implemented
title: A trace adapter for the metaharness event stream
relations:
- decomposes: epic:metaharness-migration
revision: 6
---
# Story: A trace adapter for the metaharness event stream

## Outcome

A driven run's transcript — now a `metaharness.event/1` stream — can be checked against a trace
expectation document, so the trace-spec join the migrated eval suspended
(`evals/engineering-protocols/run-driven.sh` § 3.4 in the metaharness repository) comes back.

## Context

Left open by `epic:metaharness-migration` wave 2: `protocol trace check` reads Claude stream-json,
and the driver's transcripts stopped being that format when every `llm` step moved onto the
metaharness seam. The event stream is richer, not poorer — `session.ended` carries the vendor's
`permission_denials` *and* the seam's own decision census in one record — so this is one adapter
for every harness metaharness ever drives, and the Claude stream-json adapter becomes the
recorded-fixture reader.

## Acceptance

- **Met.** A `metaharness.event/1` file lifts into `trace-ir/1` with session start, tool calls,
  decisions, denials and usage populated; an absent field stays absent, never zero.
  `crates/trace-spec/src/event_stream.rs`, with `a_field_the_stream_records_as_null_stays_absent_rather_than_becoming_zero`
  as the invariant-5 guard — this wire writes every absent field as an explicit `null`, so a reader
  that took a present key for an answer would report fifteen confident zeros.
- **Met for the two documents that describe a driven step**, and reported for the third.
  `expectations.driven-step.trace.yaml` is 12 ok / 0 gap / 0 unk and
  `expectations.denial-step.trace.yaml` is 10 ok / 0 gap / 0 unk against the committed fixtures,
  with no word of either changing. `expectations.trace.yaml` is the **interactive** plugin eval's
  document and a driven step is not its subject: 39 ok, 3 gap, 0 unk, every row named and pinned in
  `crates/trace-spec/tests/event_stream.rs`. Its three gaps are facts about the run — a
  metaharness session stays in the vendor's `default` permission posture because decisions arrive
  over the seam, and the driven surface refused one chained command line, which the driven-step
  document bounds at two rather than forbidding. **The four `unk` rows this story shipped with are
  gone**, and it is the seam that closed them rather than a rule here — below.
- **Not this repository's to close.** The migrated eval's § 3.4 lives at metaharness
  `evals/engineering-protocols/run-driven.sh`; the reader it was waiting for exists, and switching
  the join back on is a change there.

### What a driven event stream cannot answer, and why

Named rather than worked around. Each reads `unk` in a verdict, never a pass:

| expectation | reason |
|---|---|
| `tool.failed` / `tool.error_rate` over a result with no `is_error` | absence is the `unk` verdict on this wire, and collapsing it to *succeeded* is exactly what acceptance 1 forbids. The other adapter's opposite rule is a fact about Claude Code's transcript, not about a seam that may be carrying any vendor. |

Fixing it is a change at the seam — in metaharness — and not a second rule here.

### The four rows that left this table, 2026-08-23

This table had five rows. Four of them said the same thing in four ways — *the wire has no key for
what this kind reads* — and the gap register carried them as one row against metaharness rather
than against this repository: **"not this repository's to close: it is four fields at the seam …
this reader gains them the day they arrive, with no rule changing here."**

They arrived. metaharness protocol **amendment a9** (2026-08-23) gives `tool.result` a
`tool_use_result` carrying the vendor's own per-tool record verbatim, and gives every `usage`
payload `thinking_tokens`, `iterations`, `speed` and `cost_usd`. The row's promise held to the
letter: `crates/trace-spec/src/event_stream.rs` lifts the five fields and **no rule changed**.

| expectation | what it now reads |
|---|---|
| `skill.completed` | the `commandName`/`success` pair out of the `tool_use_result` sibling, through the same `crate::json::result_fields` the `stream-json` adapter reads it with — one definition, so two adapters cannot disagree about one run |
| `tokens.thinking`, `iterations`, `speed` | the terminal record's aggregate `usage`, which is the only record Claude fills them on |
| `cost.total` scoped to one model | `model_usage.<model>.cost_usd`, which is where the vendor prices. The aggregate carries no cost by the amendment's own rule, and the run's figure stays `session.ended.total_cost_usd` |

**What did not change is the point of the entry.** A key present and `null` still reads as absence,
so every one of those kinds is `unk` against a vendor that reported nothing — which is every
Codex-driven run for four of the five. The polarity is committed as a fixture rather than left as a
claim: `crates/trace-spec/tests/fixtures/metaharness-driven-null-a9-step.jsonl` carries all five
keys explicitly `null`, and `a_field_the_vendor_reported_as_null_is_unknown_and_never_ok` requires
`unk` for each. Two substitutions the reader could have made are refused by name and tested —
`thinking_tokens` is never taken from a `thinking.estimate`, and `iterations` is never a tally of
`usage` events.

## Out of Scope

- Removing the Claude stream-json adapter: the recorded fixtures under the migrated eval and
  `crates/trace-spec/tests/fixtures/` still read through it.
- `Engine::authorize` at decision time — tracked on the epic, lands with the first case where a
  decision would change engine state.

## Open Questions

- ~~Whether the adapter lives beside the stream-json reader in `trace-domain` or behind
  `metaharness project` (its Q9).~~ **Decided in implementation: beside the other adapter, in
  `trace-spec`** — `crates/trace-spec/src/event_stream.rs`, with detection in
  `crates/trace-spec/src/reader.rs` and the JSON shapes both readers meet in a private `json`
  module. `trace-domain` holds the *models* and `trace-spec` the *mechanisms*, and an adapter is a
  mechanism; a projection living behind `metaharness project` would put a `trace-ir/1` producer in
  a repository that deliberately depends on nothing here (its D1), so metaharness publishes the
  projection *contract* — which event lands in which family, and which land in none — and this
  repository reads it.
