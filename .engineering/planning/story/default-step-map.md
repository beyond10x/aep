---
format: aep.planning-md/1
id: story:default-step-map
kind: story
status: implemented
title: The first step map, and the tree row that loads it
summary: drivers/development/default.yaml over adp/default/1, its generated JSON Schema, and drivers/ as the last row of the document tree.
owner: driver
tags:
- driver
relations:
- decomposes: epic:reference-driver
- depends_on: story:driver-spec-crate
- serves: vision:O3
revision: 6
---
# Story: The first step map, and the tree row that loads it

## Outcome

A reader can open one YAML file and see what the default development workflow actually does at each
state — which step runs, what it is allowed to touch, and what would make it move on.

## Context

Until this exists the driver has decisions and no document to walk. The map goes under `drivers/`,
which has been a reserved directory name with nothing writing to it since wave 2, and `drivers/` is
added as the **last** row of the document tree loader so that no existing tree's load order moves.
The generated schema is what makes an author's editor agree with the loader.

## Acceptance

- `drivers/development/default.yaml` loads, pins `adp/default/1`, and covers every state that
  workflow declares — a state with no step is a refusal, not a silent skip.
- `schemas/generated/driver-steps.schema.json` is generated from the type and checked by the ordinary
  generate-check, so a drifted schema fails the gate.
- `drivers/` is the last entry of the loader's tree table, and a repository with no `drivers/`
  directory still loads exactly as before.
- The map's every named verifier resolves at load; its evidence kinds are checked at run start.

## Re-scoped on evidence — 2026-08-28

The map ships and loads (`the_committed_step_map_loads_and_is_refused_when_a_state_is_renamed`,
`crates/protocol-cli/tests/drive_cli.rs:399`; `cargo test -p protocol-cli --test drive_cli` → 11
passed) and the schema is generated and gated (`cargo xtask schema --check` → *schemas are up to
date*; gate step `schema-check`).

| line | state | what remains |
|---|---|---|
| pins `adp/default/1` | **stale text.** The map pins `adp/default/2` (`drivers/development/default.yaml:47`), with the repin explained at `:41-46` | the story's wording; nothing to build |
| covers every state the workflow declares — a state with no step is a refusal, not a silent skip | **contradicted by the code, deliberately.** The workflow declares `declined` (`workflows/development/default.yaml:105`); the map has nine states and not that one. `crates/aep-driver-spec/src/map.rs:754-762` states the opposite rule, and `a_state_the_map_says_nothing_about_transitions_immediately` (`crates/aep-driver/tests/routing.rs:83`) asserts it | a decision, not a build: either the map covers `declined` and the loader refuses a gap, or this line is replaced by the rule that shipped — *a state the map is silent about transitions immediately* — with the reason. Recommended: the second, because a terminal state has nothing for a step to do |
| `drivers/` is the last row of the tree loader, and a repository without it loads as before | **partial** — it is last (`crates/aep-engine/src/load.rs:22-34`) and a missing directory is skipped (`:133-135`), but **no test asserts the ordering** | one test over the loader's table |
| every named verifier resolves at load; evidence kinds at run start | **partial** — load side tested (`map.rs:1512`); run-start side green-path only | the red-path test also owed by `story:driver-spec-crate`; whichever story lands it, the other cites it |

### Re-verified — 2026-08-30

`cargo test -p protocol-cli --test drive_cli` → **47 passed**, exit 0 (11 on 2026-08-28). Nothing
in the four rows moved; two are prose and two are a decision plus one test.

- **The pin is still `adp/default/2`** (`drivers/development/default.yaml:47`). Story text only.
- **The map still declares nine states and the workflow ten.** Read off the documents today:
  map — `adversarial_verify, complete, decompose, establish_verifiers, implement, receive, review,
  specify, verify`; workflow — the same nine plus **`declined`**
  (`workflows/development/default.yaml:105`, with the edges into it at `:126` and `:218`). This is
  the one row that is a **decision and not a build**, and it has been open for two days.
- **The loader ordering is still unasserted.** `drivers` is the last row of `TREE`
  (`crates/aep-engine/src/load.rs:33`), `TREE` is walked at `:131`, and a search of
  `crates/aep-engine/tests/` finds no test that reads the table or its order. One test.

## Out of Scope

A second map, a second profile's map, and anything under `incidents/`, `migrations/` or `releases/`.
One workflow, walked properly, is what proves the shape.

## Open Questions

Whether a step map may extend another the way a profile extends a profile. Decides: driver owner.
Not blocking — one map cannot demonstrate the need, which is the argument for not designing it yet.
