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
revision: 3
---
## Outcome

An illegal lifecycle move on an Eventlog-backed plan is refused with the same information the
Markdown arm gives: the artifact's current status and the statuses it may move to. Parity is
pinned against the Markdown arm's answer, not against one arm's wording.

## Why

Measured 2026-09-21 in rehearsal run 3 (wave-validate-v2-20260920, final binary acb67b88…, copy
`home-path:sha256:33a89503aef363fe2509aefb7752c7e6ad48489f1b2a1c123b1c93dbad486964`): the same command before the migration (Markdown arm) and after it
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

## Cause, located

Located 2026-09-22 09:40 CEST, read at AEP `def6ba720c0969313e52702a9ba046976d8c80f3`.

One `Err(_)` arm loses every refusal the decision made:

| site | what it does |
| --- | --- |
| `crates/edge/aep-cli/src/planning.rs:1098-1105` | `eventlog_invocation`'s `Err(_) => ChildExecutionFailure::Refused(vec![mutation_refusal(&authority, SemanticMismatch)])` discards the error value and emits a refusal located at the authority with `subject: missing` and `record_id: missing` |
| `crates/edge/aep-cli/src/planning.rs:3292-3320` | `move_status`'s closure reads `outcome.made.get(index)` with `.context("the requested status hop was refused before execution")`; when the decision refused, `made` is empty, the closure errors, and the error is the one the arm above throws away |
| `crates/edge/aep-cli/src/planning.rs:3324-3326` | the Eventlog arm `return`s on that envelope, before `report_moves` and the `match stopped` block that prints `outcome.refusal` — so the Markdown arm's text is not merely reworded on this path, it is never reached |

The story as written covers one refusal class, `MoveStopped::Refused` (an illegal rung). The arm
loses all three: `Refused`, `GuardedRungOnAWalk` and `WouldNotValidate`. `WouldNotValidate` is the
one that cost this wave, because it fires on moves that are legal.

## The reproduction that is not an illegal move

Every wave-filed draft story refuses `draft -> proposed` on a migrated store, and
`artifact explain` says the rung is unguarded (`next: proposed needs no record`), so nothing in the
store's own answer accounts for the refusal.

| store | command | answer |
| --- | --- | --- |
| entity-runtime `01a0c44b-286b-70c8-b04f-86fd14613830` | `move story:seeded-open-under-one-second --to proposed` | refused, `semantic_mismatch` at authority, naming nothing |
| entity-runtime, same store | `move story:one-git-url-one-rev-across-the-workspace --to proposed` | the same document |
| eventlog `01a0c3a5-80f2-718e-bfd8-b1537ffe273c` | `move story:incremental-history-digest-for-a-resumed-handle --to proposed` | the same document |

The Markdown arm, run against a copy of the entity-runtime store's own projection
(`cp -r .engineering/planning`, then `--store <copy>`), answers the same move in one line:

```
story:seeded-open-under-one-second would move draft -> proposed, and the store would not validate:
  - [empty_declaration] artifacts.story:seeded-open-under-one-second.relations:
    story:seeded-open-under-one-second is proposed and serves no objective: this store declares
    objectives (`vision` artifacts), and agreed work says which it moves
    (hint: `protocol artifact relate <id> serves vision:<objective>`)
```

So no migrated store is refusing writes. The stores are sound — entity-runtime validates 107
artifacts clean in 6.3 s — and the moves are refused for a reason the caller may act on, which the
Eventlog arm replaces with a code that names the authority and nothing else.

## What this adds to the acceptance above

A second red case, beside the illegal-move one: a **legal** rung whose post-move store would not
validate, asserted to carry the finding text the Markdown arm prints, on both output formats. A fix
that only threads the lifecycle's legal-move list through would leave this case red.
