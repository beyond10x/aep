---
format: aep.planning-md/3
id: decision-blocker:release-verification-claims-writer
kind: decision-blocker
status: cleared
title: How a release run writes its verification claims
relations:
- blocks: story:release-verification-claims
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-15T06:21:47Z", actor: "human:timo", revision: 3}
---
# Decision: how a release run writes its verification claims

`workflows/releases/source.yaml` guards its transitions on four `verification` claims
(`release-on-default-branch`, `release-inputs`, `release-tag-pushed`, `source-release`). Nothing
in this repository writes them, and `EvidenceMapping::MINTABLE` keeps `verification` out of what
a command step may mint, so a release driven by `aep drive run` cannot reach `complete` outside a
fixture. `story:release-step-map` stays fixture-based; `story:release-verification-claims` waits
on this decision.

Options:

- **A. Dedicated check commands.** `cargo xtask release check <claim>` verifies the condition
  (branch, inputs, pushed tag, Release run and assets) and records the claim through
  `aep plan artifact evidence --kind verification`. The trust boundary of `MINTABLE` is
  unchanged; the claim's writer is a named tool, not any step. Cost: one story, xtask only.
- **B. Make `verification` mintable by a declared command step.** Any step map could then mint a
  verification claim from an exit status. Cost: a protocol change to the mapping rules, ESS first
  in `ess/domains/drive.yaml`, and a weaker boundary for every map.
- **C. Narrow the epic's Done When** to a release driven up to `publish`, with the claims written
  by an operator step. Cost: the epic does not deliver a release without a person.

Recommendation: A.

## Decided

Option A. The evidence boundary stays as it is: a command step does not mint `verification`. Each of the four claims is written by a `cargo xtask release check <claim>` step that verifies its condition and records the claim through `aep plan artifact evidence --kind verification`. `story:release-verification-claims` is drafted on that design for a later wave.
