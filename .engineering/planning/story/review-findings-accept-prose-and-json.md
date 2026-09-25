---
format: aep.planning-md/2
id: story:review-findings-accept-prose-and-json
kind: story
status: active
title: A review-result records findings whose messages are ordinary prose
relations:
- decomposes: epic:review-facts
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/planning_cli.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/findings.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 7
---
## Outcome

A person or an agent recording a `review-result` whose finding messages are ordinary prose — colons,
quotes, apostrophes — gets it recorded on the first attempt, or gets a refusal that shows the
offending line and says how to write it.

## Why

GitHub issue beyond10x/aep#38 (aep 0.59.3). The fenced `findings` block is parsed as YAML
(`crates/plan/aep-backend-markdown/src/findings.rs`). A message such as
`the judge reports "(platform): X" for an unmeasured step` fails with `mapping values are not
allowed in this context`; single-quoting then fails on any apostrophe. The refusal reports
`at line 10 of the body: … at line 7 column 41` — two coordinate systems, and the offending text
is not shown (`FindingsError`, `findings.rs:338-358`). JSON inside the fence already parses,
because JSON is YAML, but nothing documents it or points a failing writer at it.

## Acceptance

- Red first: a case records a block whose message contains `": "` and one containing an
  apostrophe, written as JSON inside the fence, and asserts the findings round-trip through
  `show --format json`.
- A malformed block is refused with one coordinate (the body line), the offending source line
  quoted, and a hint naming JSON as the machine-written form. A case asserts all three.
- `aep plan artifact new review-result … --findings <file>` (and `-` for stdin) accepts the findings
  as a JSON array validated against the same entry schema, so a tool need not embed them in
  markdown. The body's fenced block and `--findings` together are refused as ambiguous.
- `website/docs/reference/cli.md` documents JSON as the machine-written form of the block and the
  `--findings` input.

## Out of Scope

- The reviewer charters in `agentplugins` emitting JSON; that repository owns them.
- Retro-fitting existing immutable `review-result` records.

## Scope

- `crates/plan/aep-backend-markdown/src/findings.rs` — cited (error type and parse)
- `crates/edge/aep-cli/src/planning.rs` — cited (the `new review-result --from` path)
- `crates/edge/aep-cli/tests/planning_cli.rs` — cited (existing findings cases)
- `website/docs/reference/cli.md` — cited (documents the block today)
- Confidence: medium — paths read by the coordinator with `rg`; no scoper agent ran.
