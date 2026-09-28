---
format: aep.planning-md/3
id: story:connectors-findings-patch-upstream
kind: story
status: draft
title: AEP decides whether to adopt the connectors findings patch
relations:
- serves: vision:O2
revision: 1
---
## Problem

The connectors repository gates its planning store with aep 0.64.0 plus a private patch,
`crates/connectors-build/aep-findings.patch` (connectors PR #41, 2026-09-28). The patch adds two things
AEP does not have:

- a `review-findings-supplement`: a `verification-report` that carries the findings of an older
  `review-result` whose body has no findings block, bound to that review by a sha256 of its body,
  read by `show`, `findings`, `review value` and `validate`;
- an `unspecified` severity beside `blocker`, `warning` and `note`.

Its store depends on both: 51 supplements, and `unspecified` findings in at least one review.
Every aep release makes connectors rebase the patch.

## Outcome

AEP decides, per feature, whether it adopts it (and connectors drops the patch) or refuses it (and
connectors converts its store). No code changes until that decision is recorded here.

## Acceptance

- This story records the decision for each of the two features, with the reason.
- If adopted: connectors pins a released aep without a patch and its gate is green.
