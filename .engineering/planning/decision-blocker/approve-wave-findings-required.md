---
format: aep.planning-md/3
id: decision-blocker:approve-wave-findings-required
kind: decision-blocker
status: cleared
title: Approve the findings-required wave and its two choices
relations:
- blocks: story:review-result-requires-findings
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T22:40:08Z", actor: "human:timo", revision: 3}
---
# Decision: approve the findings-required wave

The wave delivering story:review-result-requires-findings waited for approval of its units and two
choices.

## Decided (2026-10-09)

- The wave is approved: U0 (specification) then U1 (behaviour), one integration branch.
- D1 = B: `new review-result` refuses a body with no findings block only in a store whose
  `project.yaml` sets `findings_required_since`. A store without the key behaves as before, because
  an unconditional refusal would break every existing caller. `--prose-only <reason>` works in every
  store.
- D2 = A: this store does not opt in within the wave; it opts in with a later commit, once the
  released version is the one installed.
