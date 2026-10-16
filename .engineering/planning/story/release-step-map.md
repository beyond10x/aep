---
format: aep.planning-md/3
id: story:release-step-map
kind: story
status: implemented
title: A release is cut by a step map with no model step
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:release-cutting-commands
- depends_on: story:command-step-expects-failure
- serves: vision:O2
scope:
- confidence: cited
  path: crates/drive/aep-driver/tests/release_source_map.rs
- confidence: cited
  path: drivers/releases/source.yaml
- confidence: cited
  path: generated/instructions/
- confidence: cited
  path: workflows/releases/source.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T06:23:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":12}}}
- {from: "proposed", to: "active", at: "2026-10-15T06:23:57Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":12}}}
- {from: "active", to: "implemented", at: "2026-10-16T01:52:55Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":12}}}
---
# Story: A release is cut by a step map with no model step

## Outcome

`drivers/releases/source.yaml` (id `releases/source`, workflow `release/source/1`) drives this
repository's release with `kind: command` and `kind: operator` steps only. It is this repository's
own release, not a generic one: its steps are this repository's commands (`cargo xtask release …`,
`task check`, `b10x-gates`, read-only `gh`), in the order of `AGENTS.md` (Releases and commits). A
run takes two inputs, `--input version=<x.y.z>` and `--input date=<YYYY-MM-DD>`, and is started with
`aep drive run --map releases/source --input version=… --input date=… --pause-on-approval`.

Each row is one step, and a command step's `run` is the one argv shown (`{v}` is
`{input.version}`). `retries` is `0` on every step that changes Git state, a ref or the planning
store, so a nonzero exit never re-pushes or re-commits; the others keep the default.

| # | state | kind | `run`, or what the person does | retries | evidence |
|---|---|---|---|---|---|
| 1 | inspect | command | `b10x-gates bot -- fetch origin main --tags` | 0 | |
| 2 | inspect | command | claim writer, `release-on-default-branch` | default | `verification` via `record:` |
| 3 | qualify | command | `git switch -c release/{v} origin/main` | 0 | |
| 4 | qualify | command | `cargo xtask release prepare {v} --date {input.date}` | 0 | |
| 5 | qualify | command | `b10x-gates bot -- commit -a -m "chore: release {v}" -m "Release {v}."` | 0 | |
| 6 | qualify | command | `cargo xtask release tag` | 0 | |
| 7 | qualify | command | `cargo xtask release pin` | 0 | |
| 8 | qualify | command | claim writer, `release-inputs` | default | `verification` via `record:` |
| 9 | qualify | command | `cargo build --release -p aep-cli` | default | |
| 10 | qualify | command | `env AEP_BIN=target/release/aep task check` | default | `test_result`, suite `repository-gate` |
| 11 | await_approval | operator | approve releasing `{v}` at the tagged commit | — | `approval` `release-create`, by a human |
| 12 | publish | command | `b10x-gates bot -- push origin release/{v} {v}` | 0 | |
| 13 | publish | operator | open the release PR and merge it through `b10x-gates api`, a merge commit, once required checks are green | — | |
| 14 | publish | command | claim writer, `release-tag-pushed` | default | `verification` via `record:` |
| 15 | verify | command | `b10x-gates bot -- fetch origin main --tags` | 0 | |
| 16 | verify | command | `sh -c 'gh run watch --exit-status "$(gh run list --workflow release.yml --branch {v} --limit 1 --json databaseId --jq ".[0].databaseId")"'` | default | |
| 17 | verify | command | `git switch -c plan/release-{v}-record origin/main` | 0 | |
| 18 | verify | command | `cargo xtask release record` | 0 | |
| 19 | verify | command | `b10x-gates bot -- commit -a -m "plan: record the {v} release" -m "Release {v}."` | 0 | |
| 20 | verify | command | `cargo xtask status` | 0 | |
| 21 | verify | command | `b10x-gates bot -- commit -a -m "docs: move the release pins to {v}" -m "Release {v}."` | 0 | |
| 22 | verify | command | `b10x-gates bot -- push origin plan/release-{v}-record` | 0 | |
| 23 | verify | operator | open the record PR and merge it through `b10x-gates api`, a merge commit | — | |
| 24 | verify | command | claim writer, `source-release` (includes `cargo xtask release`, 6/6 `ok`) | default | `verification` via `record:` |

"Claim writer" is the command `story:release-verification-claims` provides; this map names it in
`run` and reads its record through `evidence.record:`.

Ordering that matters:

- The local tag (step 6) exists before the gate because `version-check` needs it. The gate (step
  10) is the last step of `qualify`, so nothing runs after a failing gate, and nothing reaches the
  remote until it has passed and a person has approved.
- Steps 15 and 16 come before the record branch: `record` refuses a tag not on `origin/main` and a
  Release run that has not succeeded, and the fetch after the merge and the wait for the run are
  what make both true. Step 16 is read-only and exits nonzero when the run fails.
- Opening and merging each PR is one `operator` step: a person-approved write through the bot route
  (`b10x-gates api … POST …/pulls`, then `PUT …/pulls/<n>/merge`). The driver issues no forge write.

This story's contribution to the epic's Done When is conditional on
`story:release-verification-claims`: a real release cut by this map reaches `complete` only once
that story's claim writer exists.

## Context

`release/source` has no step map; only `adp/default/2` has one (`drivers/development/`). This
repository's release is a numbered list in `AGENTS.md` that an agent follows. `cargo xtask
release` checks a release (`release_check`) and cuts its steps (`ReleaseStep`: `prepare`, `tag`,
`pin`, `record`). A command step can mint only `EvidenceMapping::MINTABLE` kinds; a `verification`
record is read through `evidence.record:`. A command step retries `DEFAULT_COMMAND_RETRIES` (2)
times unless it says otherwise. The driver runs every remaining step of a state after a failing
verdict. A run stopped at an `operator` step under `--pause-on-approval` exits 0 with status
`AwaitingOperator`; an approval recorded while it is stopped is settled on `aep drive resume`
(`run.rs` `settle`). The workflow is not changed: a failed gate already leaves the run in `qualify`.

## Acceptance

In `crates/drive/aep-driver/tests/release_source_map.rs`, driving the shipped map with a scripted
command executor (the stub) that records each argv, answers exit statuses, and writes the
`verification` record a step's `record:` names:

- The map validates against `release/source/1` with no errors and has no `kind: llm` step.
- Every step the table gives `retries 0` declares `retries: 0`.
- A run without `version` or `date` in its inputs is refused before any step runs, naming the
  missing input.
- With `version=9.9.9` and `date=2026-01-02`, the recorded argv list equals the table's command
  steps 1–10, 12, 14–22 and 24, one argv per step, in that order, with the inputs filled in.
- The run stops at step 11 with status `AwaitingOperator` and no argv of step 12 recorded. A
  granted `release-create` approval by a human, recorded into the stopped run's snapshot (as
  `tests/attested.rs` `approve` does), followed by a resume, moves the run to `publish`, and the
  cursor's `answers` names that human.
- The run stops `AwaitingOperator` at steps 13 and 23, and no recorded argv contains `/pulls`.
  Resumed, it carries on with the next step.
- After the last resume the run reports `Completed` in `complete`, and its cursor counts zero llm
  step attempts.
- With step 10 answering exit 1: the run reports blocked in `qualify`, `publish` is never entered,
  and the last recorded argv is step 10's (no push of the branch or the tag; step 6 did run).

## Scope

- `drivers/releases/source.yaml` (new)
- `crates/drive/aep-driver/tests/release_source_map.rs` (new; not `driving.rs`)
- `AGENTS.md` (Releases and commits): names the `aep drive run` invocation above
- `CHANGELOG.md` (Unreleased)

`workflows/releases/source.yaml` and `generated/instructions/` are unchanged: instructions render
from workflows, not maps.

## Depends on

- `story:release-cutting-commands`: `prepare`, `tag`, `pin`, `record`.
- `story:command-step-expects-failure`: `{input.version}` and `{input.date}`.

`story:release-verification-claims` depends on this story and provides the claim writer the map
names; this story's acceptance is fixture-based and does not wait on it, the epic's Done When does.

## Open Question

Changelog prose for a reader may need a model. The default is that the changelog is written during
the work that changed it (`AGENTS.md`, Conventions), so `prepare` only renames the heading.
