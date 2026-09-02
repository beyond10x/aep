---
format: aep.planning-md/1
id: story:artifact-waves-verb
kind: story
status: draft
title: aep artifact waves derives waves and names collisions, and exits 2 on a cycle
summary: Waves from scope and depends_on; collisions per pair per path; unassessed stories never placed; read-only; byte-exact on a fixture store.
owner: protocol
tags:
- cli
- wave
relations:
- decomposes: epic:wave-derivation
- depends_on: story:scope-as-a-typed-field
- serves: vision:O2
revision: 1
---
# Story: `aep artifact waves` derives waves and names collisions, and exits 2 on a cycle

## Outcome

A coordinator runs one command and reads which stories can be implemented at once, which pairs collide on which path, which stories could not be assessed, and in what order the waves must run.

## Context

The comparison of 2026-09-02 found `bdfinst/agentic-dev-team`'s `scripts/plan_waves.py` (208 lines) does this over a prose plan: cycle detection, missing `Depends-on`, same-wave file collision, exit 2 on each. Doing it over a validated store is strictly stronger, because the inputs were checked when they were written. This story needs `story:scope-as-a-typed-field` for its input.

## Acceptance

- `aep artifact waves [--kind story] [--status draft|proposed|active]` prints waves in order; inside a wave no two stories share a `scope` path; `depends_on` edges never point forward within or across waves in the wrong direction.
- A `depends_on` cycle is reported with the cycle's ids and the verb exits 2.
- A pair excluded for a shared path is printed as `collision: <a> <b> <path>`; a story with empty `scope` is printed under `unassessed` and never placed.
- `--format json` carries waves, collisions and unassessed as arrays; a test fixture store with a known answer diffs byte-exact.
- The verb reads and prints only; the store is unchanged after it, asserted by a test.

## Out of Scope

- Choosing the wave to run. The skill and the operator do.
- Weighting stories by size or cost.

## Ambiguities

- `inferable` — scope granularity is whatever the entries say; the verb does not normalise `crates/x/src/lib.rs` to `crates/x`. Stated in the help text.
- `requires-stakeholder-input` — whether `inferred` scope entries count as collisions or as unassessed. Decides: protocol owner. Default: they count as collisions and are marked `inferred` in the output.

## Open Questions

None.
