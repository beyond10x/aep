---
format: aep.planning-md/1
id: story:adversary-names-whose-defect-it-is
kind: story
status: implemented
title: The adversary names whose defect it is, and attacks the contract the unit wrote
summary: An origin column, the unit's own vectors as the first attack surface, a specified mutation-to-probe bound, and a README line that contradicts the charter.
tags:
- plugin
- wave
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
- informed_by: story:implementor-and-adversary-agents
revision: 4
---
# Story: The adversary names whose defect it is, and attacks the contract the unit wrote

## Outcome

`agents/adversary.md` carries six changes both 2026-08-30 waves needed and invented per dispatch.

## Context

| # | change | evidence |
|---|---|---|
| a | an `origin` column: introduced / pre-existing / undecided | SUBRETRO §5 — both adversaries patched it in prose: "CONFIRMED (out of scope of this change)", "Pre-existing rather than introduced" |
| b | when the unit adds or changes a vector, fixture or schema, drive the implementation against that document first | SUBRETRO addendum 1; `6f1be0d9`: a unit shipped `pty-session-output-bound-ends-the-session.json` asserting `cancelled` and code returning `exited`, every gate step green |
| c | mutation-to-probe is specified: on a scratch copy only; the report leads with `git diff --stat` proving the tree is restored | RETRO3 P7; `114c2340#162` operator: "why are adversarial review agents making changes to the code" |
| d | the "check the scenario is one somebody reaches" rule moves from `skills/wave/SKILL.md:291-313` into the charter the adversary reads | `2e81f991#678` |
| e | scratch root is the one the coordinator assigned; never `/tmp` | `9c286ad7` 15 files under `/tmp`; `114c2340` two agents |
| f | `integrations/claude-code/README.md:32` says the adversary records a `review-result`; `agents/adversary.md:73,84` says it runs no `protocol artifact` command | README stale since `5a04cb8` |

## Acceptance

- The findings table has an `origin` column and the routing text in `skills/wave/SKILL.md` reads it.
- The attack-surface list opens with the unit's own new contract documents.
- The charter states where a probe may edit and what proves the restore.
- `README.md:32` matches the charter.

## Out of Scope

Emitting evidence — the adversary is not a verifier (`story:implementor-and-adversary-agents`).
