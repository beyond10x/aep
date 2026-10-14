---
format: aep.planning-md/3
id: decision-blocker:approve-detflow-wave-1
kind: decision-blocker
status: cleared
title: The first deterministic delivery wave waits for approval
relations:
- blocks: story:task-spend-is-modelled
- blocks: story:llm-step-declares-judgement
- blocks: story:release-cutting-commands
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T07:23:41Z", actor: "human:timo", revision: 2}
---
# Decision: approve the first deterministic delivery wave

Three units, the stories of `epic:deterministic-delivery-workflows` that depend on nothing:

- `story:task-spend-is-modelled`: the per-task spend record in `ess/domains/evidence.yaml`, a
  specification change with no Rust implementation.
- `story:llm-step-declares-judgement`: spec first in a new `ess/domains/drive.yaml`, then the
  `judgement:` line on every `kind: llm` step and the llm step counts of `aep drive status`.
- `story:release-cutting-commands`: `cargo xtask release prepare | tag | pin | record`. Build
  tooling, outside the specification.

Scopes are disjoint except the regenerated `generated/ess/` and the counts in
`xtask/tests/ess_gate.rs`, which the integration branch regenerates once. Package gates of the
touched crates per unit; one full gate on the integrated head; one pull request; a minor release, since a map whose llm step has no `judgement:` is now refused.

Width: 3. The epic's other six stories depend on these three. The other draft stories that
`aep plan artifact waves` lists as ready either belong to other repositories, are already
implemented, or are obsolete as written (`story:a-stale-binary-refuses-itself` cites the
`aep.project/1` journal, which the `aep.project/5` store no longer has, and needs re-scoping).

## Decided

Approved as proposed: three units in one wave, an adversary pass on the step-map refusal, pull
request on green CI, then a minor release.
