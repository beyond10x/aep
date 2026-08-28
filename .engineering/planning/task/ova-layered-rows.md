---
format: aep.planning-md/1
id: task:ova-layered-rows
kind: task
status: implemented
title: 'One row per layer: the artifact-status case gets two rows, never an averaged verdict'
summary: 'R5: a declaration open at the document layer and closed at the value layer appears twice with its own verdict each time, shown by the artifact-status worked example.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-table-shape
revision: 6
---
# Task: one row per layer, never an averaged verdict

## What

**R5.** A declaration that is open at one layer and closed at another gets **two rows**, one per
layer, each with its own verdict — never one row with a qualified verdict.

The worked example is artifact status, and it is real: `artifacts/lifecycles/*.yaml` lets an adopter
declare a kind's ladder in a document, but every rung it names must be a variant of
`pub enum ArtifactStatus` in `crates/aep-domain/src/artifact.rs`. The store layer is open; the value
layer is not. A single verdict would be wrong in one of the two directions.

## Why

A single averaged verdict is exactly the sentence an adopter would have believed. "Artifact statuses:
open" sends someone off to write a lifecycle document with a rung the engine will refuse; "artifact
statuses: closed" stops someone writing the lifecycle document they were entitled to write. Both
failures are the review item B this whole story answers.

## Done When

| # | Acceptance |
|---|---|
| L1 | The table carries two rows for the artifact-status vocabulary, distinguished in the `Declaration` cell by the layer each describes. |
| L2 | One has `Verdict` `open` with a `Decided by` under `artifacts/lifecycles/`; the other has `closed` with a `Decided by` of the form `crates/aep-domain/src/artifact.rs:<line>`. |
| L3 | Both `Decided by` paths exist, and the closed one's cited line contains `pub enum ArtifactStatus`. |
| L4 | No `Declaration` value appears twice with the same `Verdict` — a duplicate row must differ by verdict, and its cell must say which layer it is about. |
| L5 | The audit carries a subsection stating the one-row-per-layer rule, and that subsection names both artifact-status rows. |
| L6 | Merging the two rows into one on a scratch copy turns L1 red. |
| L7 | The check derives the closed row's line number from the cell and resolves it against the file at check time, so moving the enum in `artifact.rs` turns L3 red rather than leaving a stale citation. |

## What the adversarial pass found

No new row, but the **join was too loose**. L1 picked the artifact-status rows by the `Decided by`
path alone, and `crates/aep-domain/src/artifact.rs` holds `RelationKind` as well as `ArtifactStatus`.
Two consequences, one latent and one that arrived immediately:

- a table where the artifact-status row had been deleted and any other row cited the same file would
  have answered L1 with the wrong row — the exact averaging R5 exists to refuse;
- repointing the relation row's verdict to `artifact.rs:987` (its declaration, per `task:ova-citations`
  I12) would have reddened L6, which removes both status rows and asks the predicate again.

The selector now requires the `Declaration` to be about a status as well, so the pair it resolves is
the pair the rule is about.

## Notes

- L3 is the reason this unit is separate from `task:ova-citations`: the generic citation check
  resolves that a path and line exist, and this one resolves that the line is the *right* line. The
  enum can move without the file shrinking.
- Nothing here modifies `crates/aep-domain/src/artifact.rs`. The write surface is `docs/` and
  `.engineering/` only; this task reads that file and cites it.
- If the reading pass finds a second layered declaration, it gets two rows under the same rule. L1
  names artifact status because it is the one instance the specification has already established;
  L4 is the general form.

## Verifier

`.engineering/checks/check-layered-rows.sh`. L1–L7 are its rows.
