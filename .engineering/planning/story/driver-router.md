---
format: aep.planning-md/1
id: story:driver-router
kind: story
status: active
title: 'aep-driver: the three-valued router'
summary: The pure half of the driver — the router, the LlmStepExecutor seam and tool_config over CapabilityPolicy::decide — with no clock, no network and no randomness.
owner: driver
tags:
- driver
relations:
- decomposes: epic:reference-driver
- depends_on: story:driver-spec-crate
revision: 4
---
# Story: `aep-driver` — the three-valued router

## Outcome

Given a state, a step map and what the engine said, one function says what happens next — and it says
it the same way every time, from the same inputs, with nothing ambient in the answer.

## Context

This is the half of the driver that decides *nothing about the protocol* and everything about
sequencing: which step is next, whether the step's verdict was true, false or unknown, and which
tools that step may hold. `tool_config` derives the tool set from `CapabilityPolicy::decide` rather
than from `allow` alone — `allow`, `approval_required` and `deny` are three independent sets and a
capability can legitimately be in all three. The purity claim here is stronger than `aep-engine`'s,
so it is held by a test rather than by a sentence.

## Acceptance

- `crates/aep-driver/tests/determinism.rs` ships with the crate and refuses a clock read, a random
  source and an ambient environment read.
- An approval-gated capability never appears in a step's tool set, asserted against a fixture whose
  capability is in `allow`, `approval_required` and `deny` at once.
- No development profile grants `command.execute`, so an `llm` step holds no shell — asserted, not
  observed once.
- The router is handed a `LockState` and never probes for one; a source scan finds no process or
  filesystem access in the crate.
- No `Producer::Human` and no `Evidence::Approval` is constructible anywhere in the crate.

## Re-scoped on evidence — 2026-08-28

`cargo test -p aep-driver --test determinism --test tool_config --test routing --test evidence_scan`
→ 2 / 4 / 7 / 2 passed, 2026-08-28. Two lines hold; one is **falsified by a shipped profile**; two
are narrower than they read.

| line | state | what remains |
|---|---|---|
| `determinism.rs` refuses a clock, a random source **and an ambient environment read** | **partial** — `tests/determinism.rs:21-29` bans `SystemTime`, `Instant::now`, `rand::`, `getrandom`, and the unordered collections, and bans **no** environment token | add `std::env` / `env::var` to `BANNED`, and watch it fail once on a planted read |
| an approval-gated capability never appears in a step's tool set, asserted against a fixture whose capability is in `allow`, `approval_required` **and** `deny` at once | **partial** — `a_wide_allow_entry_does_not_hand_out_a_narrowly_gated_deploy` (`tests/tool_config.rs:32`) uses three *different* capabilities across the three sets. The one-capability-in-all-three fixture lives in `crates/aep-domain/src/capability.rs:719` and asserts `decide`, not the tool set | one fixture, one capability, in all three sets, asserted through `tool_config` |
| no development profile grants `command.execute`, so an `llm` step holds no shell | **false as written.** `profiles/development-driven.yaml:60,78` grants `command.execute`, and `AGENTS.md:461` says so | reword to the mechanism the code actually holds and the tests actually assert: *a shell is rendered exactly when `command.execute` is admitted, and never otherwise* (`tests/shell_echo.rs:735`). A profile that grants it gets a shell on purpose |
| handed a `LockState`, never probes for one; a source scan finds no process or filesystem access | **half false.** The `LockState` half holds (`src/lock.rs:3,66`). The crate **does** touch the filesystem: `src/run.rs:53,216,982,999,1003` create, read, write and rename the run directory — which this story's *Out of Scope* places in `protocol-cli`. No scan exists | decide which is true and make the other match: either the run directory moves, or the purity claim is narrowed to *no clock, no randomness, no ambient environment* and the story says the run directory is the crate's one impurity, with a scan that bans process spawning only |
| no `Producer::Human`, no `Evidence::Approval` constructible | **holds** — `tests/evidence_scan.rs:119`, `:158` | — |

The fourth row is the one that matters: it is a claim in a published story contradicted by the
crate's own source, and it is written down here rather than left for a reader to discover.

## Out of Scope

The lock file, the liveness probe and the run directory, which sit on the impure side of the line in
`protocol-cli`. Also out: any evidence an `llm` step could carry — a model-calling step has no field
to put it in, by type.

## Open Questions

Whether the executor seam stays a function selected by harness name or becomes a trait. Decides:
driver owner, **after** the second implementation exists — designing it before that is the mistake
the gap register exists to catch.
