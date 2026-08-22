---
format: aep.planning-md/1
id: task:w4-2-open-vocabulary-audit
kind: task
status: draft
title: 'W4-2: open-vocabulary-audit, driven as a governed run'
summary: 'The request recorded at intake: task W4-2 (kind feature, objective open-vocabulary-audit) asks for story:open-vocabulary-audit under protocol adp/1 and profile development.driven.'
relations:
- derived_from: story:open-vocabulary-audit
revision: 1
---
# Task: W4-2 — open-vocabulary-audit

Intake record. This is the request as `.engineering/task-w4-2.yaml` states it, not an interpretation
of it. Everything below is either a field copied from that task document or a quotation from it or
from the one artifact it names; nothing here is specified, designed or decomposed, because none of
that has been asked for yet.

## What

What the task document declares, field by field:

| Field | Value |
|---|---|
| task id | `W4-2` |
| task kind | `feature` |
| objective | `open-vocabulary-audit` |
| protocol | `adp/1` |
| profile | `development.driven` |
| derived from | `story:open-vocabulary-audit` |

Its own header calls it "The second governed run — W4.2's proof that the checks map drives a real
story to completion."

The requester's stated reason for choosing this story, verbatim: "`story:open-vocabulary-audit` is
chosen because its product is a document and its acceptance is checkable without a compiler: exactly
the work `development/checks` exists for, and exactly the shape `development/default` (the Rust map)
could never drive."

## Why

The one artifact the request names is `story:open-vocabulary-audit`, carried here by the task
document's own `derived_from` edge. The requester's note: "The story is `story:open-vocabulary-audit`
in `.engineering/planning/story/`; its Acceptance section is the specification this run is measured
against."

That story sits in `draft` under `epic:adopter-feedback-round-1`, and states its own outcome, quoted
unedited:

> For each thing the documentation invites an adopter to declare, the answer to *"can I put my own
> value here?"* is written down — and where the answer is no, the closure is deliberate, stated, and
> has a reason a reader can argue with.

Its own Context names where the request came from: "An early adopter's review, round 1 — **item B**,
the meta-defect: **things the docs invite an adopter to declare keep turning out to be fixed in the
engine.**" It also states what the audit's output is not: "So the audit's output is not 'open
everything'. It is a table with three columns: the declaration, whether it is open, and — where
closed — the guaranteed semantics that closure buys."

## Done When

The requester defers this to the story. Its Acceptance section, quoted unedited:

> - Every adopter-facing declaration surface in the published guides appears in one table with an
>   open/closed verdict and, for the closed ones, the guarantee the closure buys.
> - Each closed entry names where its reason is written down for adopters, not only in this table.
> - Every closed entry found to have **no** guarantee behind it gets a story or a recorded decision
>   that it stays closed; a closed vocabulary with no stated reason does not survive the audit
>   unremarked.
> - The audit is repeatable: it says how it was produced, so the next round is a diff and not a
>   rewrite.

The story also draws a boundary — its Out of Scope, quoted unedited: "Opening any specific
vocabulary. Each one that should open is its own story with its own migration question; this story
produces the verdict and the list."

## Notes

Declared facts — `constraints.facts` in the task document, asserted by the requester and observed by
nothing:

| Fact | Value |
|---|---|
| `change.public_contract` | `false` |
| `change.architectural` | `false` |

`constraints.notes`, the two that are not the story pointer already quoted above:

- "The product is a document (the audit table) plus the story's checks. Write the checks as
  `.engineering/checks/run.sh` plus whatever files it needs beside it — the map's verifier runs
  exactly that file, red before implementation, green at the end."
- "Do not modify anything under `crates/`, `website/`, `integrations/`, `drivers/`, or the workspace
  `Cargo.toml`. The audit document belongs under `docs/`."

The story carries an Open Question the requester says nothing about, quoted unedited: "Whether the
table lives in the guide or in `docs/reviews/`. Decides: protocol owner. Default if nobody answers:
**the guide**, because the audience is an adopter deciding what they may declare, and a review page
is where this repository talks to itself."

Its stated default is unanswered as of intake — recorded here, not decided here.

Sources: `.engineering/task-w4-2.yaml`; `.engineering/planning/story/open-vocabulary-audit.md`.
