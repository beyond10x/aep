---
format: aep.planning-md/1
id: story:governed-dogfood-run
kind: story
status: draft
title: One story from this backlog, driven end to end
summary: protocol drive over the default step map, closing a real story of this repository's own plan, with every transition evidence-permitted and every status move made by the verb.
owner: driver
tags:
- dogfooding
- driver
relations:
- decomposes: epic:reference-driver
- depends_on: story:own-engineering-store
- depends_on: story:protocol-drive-verb
- depends_on: story:driven-eval-acceptance
- supersedes: story:driven-eval-acceptance
scope:
- confidence: cited
  path: .engineering/checks/run.sh
- confidence: cited
  path: crates/drive/aep-driver/src/run.rs
- confidence: cited
  path: crates/edge/aep-cli/src/drive.rs
- confidence: cited
  path: docs/plan/harness-wave-4-governed-dogfood.md
- confidence: cited
  path: drivers/development/checks.yaml
- confidence: cited
  path: drivers/development/default.yaml
revision: 12
---
# Story: One story from this backlog, driven end to end

## Outcome

A story in this store is closed by a run rather than by a person deciding it was finished — and a
second person, who was not there, can reconstruct what happened from the run directory without asking
anyone.

## Context

This is the centre of the dogfood: not a task invented for the driver, but a real item of this
repository's own plan, walked by `protocol drive` over the default step map under
`development.standard`. One session per `llm` step; `cargo test` and `clippy` executed **by the
driver** as `command` steps, because no development profile grants `command.execute` and the model
therefore holds no shell at any point; and the review as an `operator` step that persists, releases
the lock and exits 0.

## Acceptance

- No state is entered except through a transition the engine returned as `Moved`, and every gate the
  driver wanted and the engine refused appears with one reason per unmet requirement — asserted
  against the snapshot's audit trail, not against the driver's own log.
- The project gate is green before the review step, and the `test_result` and `static_analysis`
  records submitted are the ones that run produced, not a summary a model wrote about it.
- Each `llm` step's transcript is checked and submitted as `trace_conformance`, and the completion
  gate reads it — the run cannot complete without it.
- Every status move went through `protocol artifact move` and by no other means, asserted by
  inspecting the store afterwards, with the write-guard hook as enforcement and `validate` as audit.
- A run that wedges is a **recorded result**: where it stopped, what the cursor said, and which
  decision was wrong. Quietly retrying until it works does not close this.

## Read against two real runs — 2026-08-28

**This story has been attempted twice and has never reached `complete`.** Both attempts are
recorded on `docs/plan/harness-wave-4-governed-dogfood.md`, which is the acceptance line *a run that
wedges is a recorded result* holding:

| run | story driven | stopped in | cost |
|---|---|---|---|
| `W4-1/1`, 2026-08-21 | `story:agent-eval-cases` | `establish_verifiers` | $15.42 |
| `W4-2/1` | `story:open-vocabulary-audit` | `adversarial_verify` | $31.46 |

| line | state | what remains |
|---|---|---|
| no state entered except through a `Moved`; refusals carry one reason per unmet requirement | **holds** — `crates/drive/aep-driver/src/run.rs:695-711`; `W4-1/1` `cursor.json` carries both unmet reasons; `W4-2/1` 25 events, 2 blocked | — |
| the gate is green before the review step; the `test_result` and `static_analysis` submitted are the ones the run produced | **partial** — `drivers/development/checks.yaml:195-225` mints all three from `command` steps and `W4-2/1` produced them, but **`review` was never entered**. The "gate" is `.engineering/checks/run.sh`, not `task check` | reach the review step; and say which gate the line means |
| each `llm` step's transcript is checked and submitted as `trace_conformance`, and the completion gate **reads** it | **missing on the gate half** — `checks.yaml:59-65` says so itself: *"it does not block the run, because nothing gates on `trace_conformance` yet"*. Only `implement`'s transcript is checked; `W4-2/1` submitted one record | the completion gate that reads it — the thing that makes this line more than a submission |
| every status move went through `protocol artifact move`, with the write-guard hook as enforcement and `validate` as audit | **partial, and the mechanism changed** — the audit half runs (`checks.yaml:220-225`); the *hook* named as enforcement no longer exists. It is the metaharness policy seam (`decide_tool`, `store_integrity` in `crates/edge/aep-cli/src/drive.rs`), and `W4-2/1` ran with **0 hook decisions** | reword to the seam that enforces today, and assert the store afterwards |
| a run that wedges is a recorded result | **holds**, twice | — |

**The story to drive next is no longer `story:retry-budgets`.** That story closed on 2026-08-28
(`implemented`, on `crates/drive/aep-driver/tests/driving.rs` and `routing.rs`). The candidate that fits
this story's own default — *mechanical acceptance, blast radius one crate* — is now
`story:operator-resume-ux` as re-scoped on the same day: one field threaded into the cursor
(`took_lock_from`, built and printed today but never persisted), the holder's state added to the
lock refusal, and three assertions. One crate, two files, nothing to argue about.

`depends_on: story:driven-eval-acceptance` is **stale**: that story is superseded, half of it by this
one and half of it by metaharness. The edge cannot be removed — there is no `unrelate` — so it is
named here.

## Out of Scope

Byte-repeatability. Two runs of one story produce different transcripts and different digests, and a
resumed run is a new session by design. Every assertion here is over the store, the audit trail and
the gate's exit code — never over the model's prose.

## Open Questions

Which story goes first. Decides: driver owner. Default if nobody answers: one whose acceptance is
already mechanical and whose blast radius is one crate, because the point of the first run is the
loop, not the difficulty.
