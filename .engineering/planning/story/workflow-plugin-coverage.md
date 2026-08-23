---
format: aep.planning-md/1
id: story:workflow-plugin-coverage
kind: story
status: implemented
title: Every shipped workflow is mapped to the plugin surface that teaches it, or carries a named gap
summary: The join between workflows/ and integrations/ becomes a checked document, so a workflow nothing teaches is a row a reader can find rather than a silence.
owner: eval
tags:
- eval
- plugin
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: Every shipped workflow is mapped to the plugin surface that teaches it

## Outcome

Somebody about to run a three-arm evaluation can read, in one document, which of this repository's
workflows either plugin actually teaches — and, for every state neither teaches, the reason nobody
wrote a surface for it. Before this, the second half was not written down anywhere, and the first
half was a sentence in a README that nothing checked.

## Context

`workflows/` declares four state machines: `adp/default`, `release/progressive`,
`incident/standard` and `migration/forward-only`. `integrations/` ships two plugins, both of them
**planning** plugins. Nothing joined the two trees.

The interesting half was the missing one. That the planning skill teaches the planning store is
obvious from reading it; that **four of the nine states of this repository's own development
workflow are taught by nothing either plugin carries**, and that three whole workflows are
uncovered end to end, is a fact nobody could get at without reading six files and holding them in
their head. An evaluation that measures how well a harness follows these workflows under (a) raw
instructions, (b) the shipped plugin and (c) enforcement needs that fact *before* it spends a model
budget: for a workflow with no plugin surface, arms (a) and (b) are handed the same instructions,
and any difference measured between them is noise.

## What shipped

- **`integrations/workflow-coverage.yaml`** — `format: plugin-coverage/1`. One entry per workflow,
  keyed by the id the document **declares** rather than by its filename (invariant 10), holding
  `covered_by:` rows — a plugin surface, the harness it is for, the states it teaches and what it
  teaches about them — and `gaps:` rows, each naming states and a reason.
- **`crates/protocol-cli/tests/workflow_coverage.rs`** — seven tests, every claim refused by name.

## Acceptance

- A workflow file added without a map entry turns `task check` red, **naming the workflow**. **Met**:
  `every_shipped_workflow_has_a_row_in_the_coverage_map` enumerates `workflows/**/*.yaml`, parses
  each, and lists every declared id the map does not carry with the file it came from.
- A map entry naming a plugin path that does not exist is refused **by name**. **Met**:
  `every_plugin_surface_the_map_names_is_a_file_that_ships`, which also refuses a surface outside
  `integrations/` — this document is a claim about the plugin surface and a row pointing at a design
  document would be a claim about something else.
- A map entry for a workflow no document declares, or whose `document:` no longer holds that id, is
  refused. **Met**: `every_map_entry_names_a_workflow_that_exists_at_the_path_it_gives`, which
  reports the two as different defects because they are.
- Coverage is **total**. **Met**: `every_state_of_every_workflow_is_either_covered_or_named_in_a_gap`
  refuses a state that is neither, and a state claimed as both.
- A state name no workflow declares is refused. **Met**:
  `every_state_the_map_names_is_a_state_its_workflow_declares`.
- The guard is verified by breaking it. **Met**:
  `the_map_is_refused_when_a_workflow_it_never_heard_of_appears` runs the mutation this story exists
  to catch — a fifth workflow lands and nobody updates the map — against the real map, without
  touching the tree.

## Why totality is the load-bearing rule

A map that lists only what is covered reads identically whether the uncovered states were considered
and named or never looked at, and it goes green on the day a workflow grows a state — which is the
day the claim stopped being true. Refusing a state that is neither covered nor gapped is what turns
this from a document somebody has to remember to update into one the gate updates them about.

The rule also produced the finding. Writing every state out forced the two gaps in `adp/default` to
be stated rather than implied: `no-implementor-or-verifier-agent` over `establish_verifiers`,
`implement`, `verify` and `adversarial_verify`, and `no-review-or-completion-surface` over `review`
and `complete`. Both reasons were already true and already argued in
`integrations/claude-code/README.md`; neither was anywhere a check could reach.

## Out of Scope

- **The converse coverage** — a surface under `integrations/` that no map entry claims. It would
  catch a skill nobody mapped, and it needs a rule for what counts as a surface (`README.md`, a
  manifest and an eval script are not), which is a judgement this story does not want to encode.
  Named rather than assumed.
- **Making any gap smaller.** Whether an implementor agent should ship is a decision, and the gap
  register already holds the argument that a verifier agent cannot honestly ship until D-3 closes.
  This story writes the gaps down; it does not close them.
- **A schema under `schemas/generated/`.** The map is not a protocol document kind — `protocol
  validate` does not read it and no engine decision turns on it. Publishing a schema would say
  otherwise, and invariant 1 would then oblige a Rust type for a file the engine has no opinion
  about. The test is the parser and the rules; the document says so in its own header.

## Open Questions

**Should the map carry a per-harness verdict rather than per-surface rows?**
Decides: eval owner. Default if nobody answers: **no** — a surface is the thing that exists on disk
and can be refused when it stops existing, and a per-harness roll-up would be derived from the rows
that are already here.
