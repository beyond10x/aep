---
format: aep.planning-md/1
id: story:eventlog-lifecycle-refusal-names-the-legal-moves
kind: story
status: draft
title: An Eventlog plan's illegal-move refusal names the current status and the legal moves
relations:
- informed_by: story:strict-advisory-classes-on-an-eventlog-plan
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/planning.rs
revision: 2
---
## Outcome

An illegal lifecycle move on an Eventlog-backed plan is refused with the same information the
Markdown arm gives: the artifact's current status and the statuses it may move to. Parity is
pinned against the Markdown arm's answer, not against one arm's wording.

## Why

Measured 2026-09-21 in rehearsal run 3 (wave-validate-v2-20260920, final binary acb67b88…, copy
`~/.cache/ess-c-reh-3`): the same command before the migration (Markdown arm) and after it
(Eventlog arm):

```
aep plan artifact move story:inline-projection-administration --to implemented
```

The artifact is `implemented` and `story` declares `implemented → archived` only, so both arms
refuse correctly. What they tell the caller differs.

Markdown arm (`receipts/r06-illegal-before-json`, exit 1, 85 bytes, on stdout):

```
story:inline-projection-administration is implemented; a story may move to: archived
```

Eventlog arm (`receipts/r36-illegal-after-json`, exit 1, 721 bytes, on stdout): an
`aep.planning-mutation/1` outcome, `kind: refused`, one refusal `code: semantic_mismatch` at the
authority, with `subject: {kind: missing}` and `record_id: {kind: missing}`. The string `archived`
appears nowhere in the document; no current status, no legal-move list. Plain output
(`r37-illegal-after-text`) is the same document.

Both arms print their human output on stdout; all four `.stderr` receipts are empty, which any
case written for this must account for.

Related: story:strict-advisory-classes-on-an-eventlog-plan (the other place where the Eventlog
arm's answer carries less than the Markdown arm's).

## Acceptance

- Red first: one case that issues the same illegal move on a Markdown store and on a migrated copy
  of it, and asserts that the Eventlog refusal names the current status and the legal moves, read
  from the lifecycle document, in both `--format json` (a field per item) and plain output (the
  Markdown arm's sentence).
- `semantic_mismatch` with a missing subject is no longer the answer to a lifecycle refusal; the
  refusal code names the lifecycle.
- The Markdown arm's output is unchanged.
