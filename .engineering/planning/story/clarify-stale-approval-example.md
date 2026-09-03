---
format: aep.planning-md/1
id: story:clarify-stale-approval-example
kind: story
status: implemented
title: Explain the stale-approval failure without invented shorthand
summary: Replace an opaque title-based review sentence with the actual revision-3 to revision-7 failure sequence.
tags:
- adopter-experience
- design-principles
- documentation
refs:
- provider: public-docs
  reference: https://beyond10x.github.io/docs/aep/concepts/design-principles/
relations:
- serves: vision:O2
revision: 5
---
## Finding

The failure table on the public Design principles page says: “The version-3 review is attached to the version-7 design by title.” The stale-approval problem is real, but the title-based mechanism is not: AEP matches a typed artifact subject and compares `reviewed_version` with the artifact `version`; titles do not participate. The sentence was invented shorthand and reads like a nonexistent protocol operation.

## Correction

State the unsafe sequence directly: an approval says which design was approved but omits which version Ada saw, so the old approval is still treated as current after the design changes from version 3 to version 7. Then name the fields AEP actually checks and retain the committed CLI refusal for a different version.

## Acceptance

A first-time reader can tell from the table alone what changed after review, why reusing the old approval is unsafe, and that AEP compares the review subject and `reviewed_version` rather than a document title.

## Source

Operator review on 2026-09-03 of `website/docs/concepts/design-principles.md`, public route `/docs/aep/concepts/design-principles/`.
