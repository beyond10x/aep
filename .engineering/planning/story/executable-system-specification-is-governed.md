---
format: aep.planning-md/1
id: story:executable-system-specification-is-governed
kind: story
status: implemented
title: A specification is a governed artifact with a lifecycle and a model digest
relations:
- serves: vision:O2
revision: 5
---
## What was missing

`ess-conformance` reports had nowhere governed to land. A specification could be written, validated
and run against an implementation, and the store held no artifact kind that could say so — so
*this specification conforms* was a sentence in a report and not a status anything decided.

## What this adds

- A `draft → validated → conforming` ladder for `executable-system-specification`, with
  `superseded` and `archived` beside them. `conforming` costs one `ess_conformance` record, so the
  rung is paid by a suite having run rather than by somebody moving the artifact.
- `model_digest` on the artifact, written by `aep artifact set --model-digest`, tying a report to
  the exact model it ran against. Scoped by `ArtifactKind::carries_model_digest`; refused by name
  on every other kind.

## What went wrong on the way

Shipped first as `0.48.0`, tagged on a branch that never reached `main`, with no test. `0.49.0` was
then cut from `main` and silently lacked both features. Two defects followed from having no test:

- `instance_of` did not carry `model_digest`, so `aep artifact set --model-digest` printed
  `model_digest set (revision 2)` and wrote nothing.
- `cargo xtask release` had no check that a tag is on `main`, so `0.48.0` read `5 of 5 complete`.

Both are fixed in `0.50.0`, which carries the features and three tests: the CLI round trip that
found the write defect, the frontmatter scoping on both sides of `carries_model_digest`, and the
`conforming` rung decided against the shipped ladder rather than a fixture.
