---
format: aep.planning-md/3
id: decision-blocker:approve-relative-inputs-wave
kind: decision-blocker
status: cleared
title: The relative evidence inputs wave waits for approval
relations:
- blocks: story:evidence-records-relative-inputs
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T20:13:47Z", actor: "human:timo", revision: 3}
---
# Decision: approve the wave for relative evidence inputs

One unit, `story:evidence-records-relative-inputs`: spec first in `ess/domains/evidence.yaml`, a
failing test, then the fix in `crates/edge/aep-cli/src/planning.rs` (the three ESS record paths
and the after-record line). Package gates of `aep-cli` before the push; the pull request CI is
the full gate; then a patch release. Waiting for approval of the wave.

## Decided

Approved as proposed: a small defect, spec first, one unit, no adversary pass. Pull request on
green CI, then patch release 0.71.3.
