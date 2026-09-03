---
format: aep.planning-md/1
id: epic:reference-driver
kind: epic
status: active
title: The reference driver
summary: 'A specified workflow that runs strictly: aep-driver-spec, aep-driver, protocol drive, the step map, and the hooks that enforce the per-state tool set.'
owner: driver
tags:
- driver
- harness
relations:
- decomposes: initiative:the-repo-governs-itself
revision: 5
---
# Epic: The reference driver

## Outcome

An operator types `protocol drive` and a specified workflow runs strictly: at each state the tools
that exist are the ones that state permits, the engine decides every transition, and the run either
reaches a state the protocol calls complete or stops with the engine's own words for why. The
difference from today is not that the agent is told to write the test first — it is that during that
step it cannot do anything else.

## Why Now

`docs/guide/harness.md` publishes a contract of seven calls and three rules, and nothing in this
workspace implements it. A published contract with zero implementations is a shape nobody has been
forced to fit, which is the same defect as an invariant nothing enforces. Wave 2 closed the six
architectural holes in the design § 4 and a feasibility review judged them against the code —
23 confirmed, 14 needs-change, 3 infeasible, all applied. The decisions are taken; what is missing is
the crate.

## Scope

`aep-driver-spec` and `aep-driver`, the first step map under `drivers/`, `protocol drive` with its
run directory and store lock, and the plugin hooks that hold the per-state tool set from the other
side. The sequence is W3.0–W3.4 of
[`docs/plan/harness-wave-2-driver-decision.md`](../../../docs/plan/harness-wave-2-driver-decision.md);
the retry and lock-UX stories are the hardening this epic does not finish without.

## Where this stands — every status read against the code, 2026-08-28

**The epic read as barely started and was mostly shipped.** Every story below was audited against
the crates and the tests on 2026-08-28 and moved, re-scoped or archived on what was found; nothing
here is a status somebody assigned.

| story | status | on what |
|---|---|---|
| `story:plan-map-coverage` | implemented | — |
| `story:tool-availability-expectation` | **implemented, 2026-08-28** | all four lines, `trace-spec` 82 / `trace-domain` 54 / `aep-driver::shell_echo` 6 passed. Its title is off by one: 51 kinds, not 50 |
| `story:retry-budgets` | **implemented, 2026-08-28** | all five lines, `aep-driver::driving` 12 / `::routing` 7 passed. Its Open Question's default was reversed on purpose — a per-step `retries` override shipped |
| `story:plugin-enforcement-hooks` | **archived, superseded** | the outcome lives in `story:metaharness-executor`; the hooks, `--settings` and `hook-decisions.jsonl` went with `integrations/claude-code/hooks/` on 2026-08-22 |
| `story:driver-spec-crate` | active, re-scoped | three red-path tests owed, no production code |
| `story:driver-router` | active, re-scoped | its *no development profile grants `command.execute`* line is **false** (`profiles/development-driven.yaml:78`), and its *no filesystem access* line is contradicted by `crates/drive/aep-driver/src/run.rs` |
| `story:default-step-map` | proposed, re-scoped | the map pins `adp/default/2`, not `/1`; *a state with no step is a refusal* is contradicted by the rule that shipped and needs a decision, not a build |
| `story:protocol-drive-verb` | proposed, re-scoped | four assertions and one message field; the lock is per project, not per store |
| `story:operator-resume-ux` | draft, re-scoped | one real defect — `--take-lock` prints the theft and never persists it |
| `story:own-engineering-store` | active, re-scoped | `protocol artifact validate` **exits 1 from a subdirectory**, which is the position this story promises |
| `story:reusable-workflow-nodes` | draft, re-scoped | retry and circuit-break shipped and no shipped map uses them; the simulated-external half does not exist |
| `story:governed-dogfood-run` | draft, re-scoped | run twice, `W4-1/1` and `W4-2/1`, never reaching `complete` — both recorded |

So this epic's *Done When* — a real task driven end to end with its records admitted — is the only
thing between it and `implemented`, and two runs have now been spent on it.

## Out of Scope

Anything that decides. Gates are evaluated by the engine and never by the driver, and an `llm` step
has no field to put evidence in — a purity claim held by a type rather than a rule. Also out: a
second real harness (`story:codex-adapter`), and any narrowing of what a model may write, which is a
boundary the design states rather than an omission.

## Risks

The trust model for plugin-supplied hooks is undocumented — if an installed plugin's hooks need a
per-invocation consent step, the hook layer degrades to advisory and `--allowedTools` carries
enforcement alone. It is named as an assumption in the design rather than assumed silently. The
second risk is the router: `aep-driver` claims a purity stronger than `aep-engine`'s, and a liveness
probe or a clock read slipping into it would be invisible to a banned-token scan.

## Done When

A real task in this repository is driven end to end, its transcripts pass `protocol trace check`,
and the resulting `trace_conformance` record is admitted by the engine — with the same step map, the
same workflow and the same `tool_config` function also driving a harness that is not Claude Code.
