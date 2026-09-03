---
format: aep.planning-md/1
id: story:drive-watch-is-a-verb
kind: story
status: draft
title: Following and scoring a driven run are `protocol drive` verbs
summary: The operator asked six times for one command that follows a run; drive-watch and drive-score are scripts that broke five times; drive plan is unrecognized; resume re-reads none of its flags.
tags:
- cli
- driver
relations:
- decomposes: epic:reference-driver
- serves: vision:O3
- informed_by: story:protocol-drive-verb
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: crates/edge/protocol-cli/src/drive.rs
- confidence: cited
  path: docs/plan/harness-wave-4-governed-dogfood.md
- confidence: cited
  path: scripts/drive-score
- confidence: cited
  path: scripts/drive-watch
revision: 7
---
# Story: Following and scoring a driven run are `protocol drive` verbs

## Outcome

`protocol drive watch <run>` follows a run's event stream and `protocol drive score <run>` reports its
waste and cost, replacing `scripts/drive-watch` and `scripts/drive-score`.

## Context

- `431986de#2089` (operator, after six requests over 2 h): "can you just create ONE … command which follows
  the actions of the test-runs"; the answer was a python script broken five times (`#2104`, `#2265`, `#2462`).
- `431986de#1283`: `protocol drive plan` → `unrecognized subcommand`, six times across states.
- `431986de#3389`: `drive-score` reported 0.0 % waste on a run that threw away 10 of 82 calls.
- F-W4.2-4 (`docs/plan/harness-wave-4-governed-dogfood.md:397-495`): `drive resume` re-reads none of its
  five flags and `--max-iterations` is cumulative — the printed resume line does not work. Not in the store.

## Acceptance

- `protocol drive watch <run-id> [--follow]` prints `tool.decided`, state entries and refusals as they
  land, from any directory inside the project.
- `protocol drive score <run-id>` prints calls, refused calls, waste %, cost, per-state; the native arm's
  refusal channel is counted.
- `protocol drive plan <task>` prints the states a run would walk and the tools each admits, without running.
- `resume` re-reads every flag the run was started with, or refuses naming the one it cannot; the
  printed resume line works as printed.
- `scripts/drive-watch` and `scripts/drive-score` are removed; `AGENTS.md` § Watching and scoring names the verbs.

## Out of Scope

A TUI.
