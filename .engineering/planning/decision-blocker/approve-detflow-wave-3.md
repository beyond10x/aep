---
format: aep.planning-md/3
id: decision-blocker:approve-detflow-wave-3
kind: decision-blocker
status: cleared
title: The third deterministic delivery wave waits for approval
relations:
- blocks: story:concurrent-command-steps
- blocks: story:release-step-map
- blocks: story:spend-against-baseline
- blocks: story:bugfix-workflow
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-15T06:23:56Z", actor: "human:timo", revision: 2}
---
# Decision: approve the third deterministic delivery wave

The four remaining stories of `epic:deterministic-delivery-workflows`, after plan-critic rounds 3
and 4 (`review-result:detflow-*-round-3`, `review-result:detflow-*-round-4`), plus the move to
ess 0.60.0:

- `ess/ess-inputs.yaml` requires ess 0.60.0, CI installs it and `xtask/tests/ess_gate.rs` pins it;
  `generated/ess/` is regenerated under it. First commit on the integration branch.
- `story:concurrent-command-steps`: `concurrent: true` on a state; its consecutive command steps
  run as one batch, one cursor unit; llm and operator steps are barriers. Spec first in
  `ess/domains/drive.yaml` (the `concurrent` field and `aep.drive.DeclareConcurrentOutput`).
- `story:release-step-map`: the release step map of this repository, no `kind: llm` step,
  fixture-checked with a stub for the four `verification` claims.
- `story:spend-against-baseline`: per-kind spend against a recorded baseline and target. Spec
  first in `ess/domains/plan.yaml` (the `spend` project key) and `ess/domains/evidence.yaml`.
- `story:bugfix-workflow`: the `bugfix/standard` workflow and its map. It depends on
  `story:concurrent-command-steps` (diagnose sets `concurrent: true`), so it starts when that unit
  is merged into the integration branch.

Scopes are disjoint (`review-result:detflow-parallel-safety-round-4`: approve, no findings) except
the regenerated `schemas/generated/`, `generated/ess/`, `CHANGELOG.md` and the counts in
`xtask/tests/ess_gate.rs`, which the integration branch regenerates once after the units merge.

Width: 3 at once, then 1. `story:bugfix-workflow` waits on `story:concurrent-command-steps`. The
other stories `aep plan artifact waves` lists as ready are owned by other planes (eval, harness,
protocol, other repositories' spellings), carry open questions that need a design first, or are
obsolete as written. `story:release-verification-claims` depends on `story:release-step-map` and
goes in a later wave.

Package gates of the touched crates per unit; an adversary pass on each unit alongside the build;
one full gate on the integrated head in CI; one pull request; a minor release, since a step map
gains a field and the project file gains a key.

## Decided

Approved as proposed, under the operator's standing approval of this repository's waves.
