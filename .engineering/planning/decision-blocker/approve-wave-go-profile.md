---
format: aep.planning-md/3
id: decision-blocker:approve-wave-go-profile
kind: decision-blocker
status: cleared
title: Approve the generated-runner profile wave
relations:
- blocks: story:evidence-admits-generated-runner-profile
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-10T03:23:33Z", actor: "human:timo", revision: 2}
---
# Decision: approve the generated-runner profile wave

## Question

Approve one wave of one unit, integration branch `wave/go-profile` and one pull request, then
patch release 0.71.2?

- U1 `story:evidence-admits-generated-runner-profile`: spec first in `ess/domains/evidence.yaml`
  (the admitted producer profiles and the `go-scenario-status/2` execution rule); then a
  `ProducerProfile` variant for `go-scenario-status/2` in `aep-domain`, read by both
  `aep-ess-evidence` readers; tests from the consumer's report shape.

## Recommendation

Approve: the unit adds one admitted input value and its outcome, no command, flag or
configuration key.

## Decision

Approved as proposed (2026-10-10): one unit worked in the wave tree, one pull request, an
adversary pass after green, merge on green CI, then patch release 0.71.2.
