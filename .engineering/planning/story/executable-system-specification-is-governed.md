---
format: aep.planning-md/1
id: story:executable-system-specification-is-governed
kind: story
status: implemented
title: A specification artifact has a ladder, and a digest to be held to
relations:
- serves: vision:O2
revision: 4
---
## What was missing

`ess-conformance` binds a run to a specification *revision*: a report counts only where its
`spec_digest` is the `model_digest` the specification artifact records. Two things made that
unreachable for every adopter.

- **`executable-system-specification` had no lifecycle.** `aep artifact lifecycle
  executable-system-specification` printed nothing, which the engine reads as *every status is legal
  and every move permitted*. The one kind whose purpose is to be held to a generated suite was the
  one kind nothing held to anything.
- **`model_digest` had nowhere to be written.** The rule lived in the type and the field it reads
  could not be set, so the requirement failed closed — correctly and uselessly.

## What was done

`artifacts/lifecycles/executable-system-specification.yaml`:
`draft -> validated -> conforming -> superseded -> archived`, with `conforming` costing one
`ess_conformance` record so it cannot be asserted into.

`aep artifact set --model-digest <hex>`, accepted only on a kind that carries a compiled model and
refused on any other by the CLI and by the frontmatter validator both.

## How it was decided

Found by adopting both in a real project on 2026-09-03: babelforce's ACD generates a Go conformance
suite from its own ESS model, and the loop could not be closed because the specification artifact
could record no digest and its status could move anywhere.

## Acceptance

- `aep artifact lifecycle executable-system-specification` prints the five rungs and the cost of
  `conforming`.
- `aep artifact set <es-spec-id> --model-digest <hex>` writes it; the same flag on any other kind is
  refused by name.
- `task check` green on the tagged tree.
