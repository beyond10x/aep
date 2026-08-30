---
format: aep.planning-md/1
id: story:recurrence-key
kind: story
status: draft
title: Two incidents with one root-cause shape, countable
summary: A cross-incident recurrence key on incident/standard, so a rollup over root-cause shapes exists without a hand-written index.
owner: protocol
tags:
- adoption
- workflow
relations:
- decomposes: epic:adopter-feedback-round-1
revision: 2
---
# Story: Two incidents with one root-cause shape, countable

## Outcome

The second time the same shape of failure happens, the store says so. A rollup over root-cause shapes
exists without anybody maintaining an index by hand.

## Context

An early adopter's review, round 1 — **item G1**, filed by them under *smaller* and marked **cheap,
high value**. `workflows/incidents/standard.yaml` ends at `learn` on purpose — its own header says an
incident workflow that ends at `recover` produces the same incident again — and then gives `learn`
nothing to write the recurrence into. Each incident's lessons land in its own document, and nothing
joins two incidents that failed the same way.

The adopter runs the missing piece by hand: root-cause shape tags, **181 lines across 21 incidents**,
with a working rollup on top. That is the evidence for *cheap*: the thing being asked for is a key and
a rollup over it, not an analysis engine.

G2 — the untyped failure policy, the report's other *smaller* item — is **not** here: it rides
`story:adopter-bugs`, because it is a one-line validation fix with no design question, and splitting
it out would have made a story that is finished before it is read.

The value is the count. One incident with a shape is an anecdote; the same shape three times is a
decision to make, and it is invisible today unless the same person happens to remember both.

## Acceptance

- `incident/standard` carries a cross-incident recurrence key, declared where the workflow's other
  outputs are declared, and `learn` is the state that owes it.
- Two incidents sharing a key are reported together, with a count, by the store rather than by a
  hand-written page.
- The key vocabulary is open — an adopter names their own root-cause shapes without a code change.
- An incident that reaches `learn` without a key is visible as owing one, on the same principle that a
  refusal is printed beside the scenarios that exist.

## Out of Scope

Inferring the shape. Nothing here clusters incidents, suggests a key or reads a postmortem; a person
names the shape and the protocol counts.

## Open Questions

Whether the key is a single value or a set of tags per incident. Decides: protocol owner. Default if
nobody answers: **a set** — the adopter's working version is tags, and an incident with two causes is
the normal case rather than the exception.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** or **inferred**.

**This is not document-only, and that is the finding.** `RawWorkflow` and `RawState` are both
`#[serde(deny_unknown_fields)]` (`crates/aep-domain/src/workflow.rs:246`, `:280`), so a recurrence
key *declared in the workflow YAML* is impossible without a Rust field and a regenerated schema.

- **Primary surface:** `crates/aop-domain` — inferred, the only typed incident in the tree (`Incident`, `IncidentStatus`) and so the only thing a cross-incident key can hang on
- **Files:** `workflows/incidents/standard.yaml:63` — cited, the `learn` state the story names
- **Files:** `crates/aop-domain/src/body.rs:406` — inferred, `Incident` has no recurrence field and `to_node`/`from_node` (`:434`, `:459`) enumerate every field, so carrying one is a field plus two arms
- **Files:** `crates/aep-domain/src/workflow.rs:279-280` — inferred, the `deny_unknown_fields` line that decides this story is not document-only
- **Files:** `schemas/generated/workflow.schema.json` — inferred, generated from those types, gated by `cargo xtask schema --check`
- **Files:** `crates/protocol-cli/src/planning.rs:2183` — inferred, `blocked` is the tree's only group-and-count read verb and the nearest precedent for a rollup
- **Symbols:** `Incident`, `IncidentStatus`, `RawState`, `RawWorkflow` — cited from the tree; `incident/standard`, `learn` — cited from the story
- **Also likely:** `protocols/aop/1.yaml:25` — inferred; `incident.**` already declares the observable, so if the key is modelled as a **fact** rather than a YAML key, `aep-domain/src/workflow.rs` drops out and only `learn.requires.predicates` moves. That is the cheaper shape and nobody has chosen between them.
- **Confidence:** **medium** — the story cites the workflow file and the `learn` state, but the declaration site it names does not exist in the tree, and the rollup surface does not exist either. Both placements are reasoned, not read.
- **Would collide with:** any unit touching `workflows/incidents/standard.yaml`, the workflow document model (`RawWorkflow`/`RawState`) or the generated `workflow.schema.json`; and any unit adding a read verb to `crates/protocol-cli/src/planning.rs`. It would **not** collide through `crates/aop-domain`, which no other workspace crate depends on.

**Not established, and enough of it that this story is not ready to dispatch.** *"Where the
workflow's other outputs are declared"* has no referent — `git grep outputs` over `crates/`,
`workflows/`, `protocols/` and `artifacts/` returns one unrelated hit, and no workflow, profile or
protocol document declares outputs. Which store does the counting is undecided between the planning
store and the entity backends, and the two need different work. There are **no incident instances
in this tree** to roll up, so the acceptance has no local corpus. And *"visible as owing one"* has
no mechanism: state `requires` blocks **entry** (`workflow.rs:88`), which is the opposite of what a
terminal `learn` state needs — the non-blocking form is `story:advisory-enforcement-tier`, and this
story declares no edge to it.
