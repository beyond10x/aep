---
format: aep.planning-md/1
id: story:acquisition-phase-set
kind: story
status: draft
title: Six ingestion states flatten onto three phases, and nothing told the adopter they could declare their own
summary: An ingestion phase set — acquisition, normalisation, classification, deduplication, routing, publication — published upstream, or a recorded decision that a phase set is an own-a-tree matter.
owner: protocol
tags:
- adoption
- protocol
- workflow
relations:
- decomposes: epic:ingestion-vocabulary
scope:
- confidence: cited
  path: docs/guide/adopting.md
- confidence: cited
  path: examples
- confidence: cited
  path: principles
- confidence: cited
  path: protocols
revision: 9
---
# Story: Six ingestion states flatten onto three phases, and nothing told the adopter they could declare their own

## Outcome

Somebody writing a workflow whose states are not `intake … completion` finds out, from the documents
they are already reading, that a phase set of their own is a document they may write — and if the
ingestion set is worth sharing, they find it published rather than invented six times.

## Context

Adopter register row **`D-I1`**, written 2026-08-21 against `protocols/aep/1.yaml` and
`protocols/aop/1.yaml`. Their words:

> `aep/1` declares 8 phases (`intake … completion`); `aop/1` adds 12 operational ones. A workflow may
> only declare phases its protocol knows. Ingestion's real states — **acquisition, normalisation,
> classification, deduplication, routing, publication** — have no home, so `workflow.yaml` maps six
> distinct states onto `intake`/`specification`/`implementation`, which erases exactly the
> distinctions the workflow exists to enforce (classify-before-dedupe, dedupe-before-route).

Their own closing condition: *"adding an ingestion phase set, most naturally as a third protocol
`aip/1 extends aep/1`, the way `aop/1` already extends it for operations."*

**Measured 2026-08-28 UTC, `target/debug/protocol` 0.32.1.** A scratch tree holding `aep/1` unchanged
plus a document `id: aip / extends: aep/1 / phases: [acquisition, normalisation, classification,
deduplication, routing, publication]` validates:

```console
$ protocol validate --root <scratch>
2 file(s): 1 protocol(s), 0 principle(s), 0 workflow(s), 0 profile(s), 0 lifecycle(s), 0 step map(s)
valid
```

So the row's premise holds — a workflow may only declare phases its protocol knows — and its
conclusion does not: `PhaseId` is an open identifier (`crates/govern/aep-domain/src/ids.rs:252`, charset
kebab), `Protocol::merge` unions a base's phases into an extension's (`crates/govern/aep-domain/src/protocol.rs:153`),
and `docs/guide/open-vocabulary.md:158` already records *A protocol document's `phases:` block —
open*. The adopter could have written `aip/1` in their own tree on the day, and `protocols/README.md`
even says how — *"`adp/1` and `aop/1` extend it rather than restating it; extension is additive"*.
They did not. The sentence that would have told them sits in a table row in
[`docs/guide/adopting.md:186`](../../../docs/guide/adopting.md) — *"A state machine your work actually
has — different states, different phases | No. Own a tree"* — which reads as a refusal rather than as
the instruction it is, and the two extensions in the tree are examples of *this repository's* work
rather than of an adopter's.

**What is still genuinely open, and it is not the mechanism.** Phases are the join between principles
and workflows: a principle times an obligation against a phase, and *any workflow that declares that
phase is bound by it* (`protocols/aep/1.yaml:97-98`). An **upstream** principle — `provenance-tracking`
and `least-privilege` are the two this adopter's profile names — cannot time an obligation against a
phase that exists only in one adopter's tree. So a privately declared `classification` phase gets no
shared obligation, ever, and the sixth adopter to write an ingestion pipeline declares a sixth set of
names none of them can write a shared principle against. That is the argument for publishing, and it
is a decision rather than a build.

## Acceptance

- A decision is recorded, either way, with its reason: an ingestion phase set is published upstream
  (the `aop/1` shape — a protocol document extending `aep/1`, in `protocols/`, with the six names and
  what each is for), **or** it is not, and the reason is that a phase set is an own-a-tree matter.
- If published: at least one principle in `principles/` times an obligation against one of the new
  phases, or the phase set is decoration. A phase nothing is bound by buys the adopter nothing they
  did not already have by declaring it themselves.
- Whichever way the decision goes, `docs/guide/adopting.md` says **in the same place the reader is
  refused** that owning a tree means writing a protocol document that `extends:` an upstream one, with
  a three-line example of the extension. The row that reads "No. Own a tree" gains the sentence that
  turns it from a boundary into a route.
- The example is real: a document tree under `examples/` declares a phase of its own by extension and
  a workflow state uses it, so `protocol validate` proves the route on every run rather than a guide
  asserting it.
- `docs/guide/open-vocabulary.md`'s phases row is unchanged. Nothing here opens anything; the row was
  already `open` and correct, and this story is the evidence that a correct `open` verdict does not
  reach the person who needs it.

## Out of Scope

An ingestion **workflow** — states, transitions, requirements. Six phase names and a guide sentence
are what this story is; a workflow document for corpus work is a different piece of work and nobody
has asked for it.

Renaming any of the eight `aep/1` phases or the twelve of `aop/1`. `intake` is not wrong; it is a
different thing from `acquisition`, and the adopter says which — *"intake mis-hears a person,
acquisition silently fetches a partial window and reports success"* is the distinction they drew
between the two failure modes.

## Open Questions

**Is the ingestion set six names or fewer?** The adopter's register names six; their own later tree
collapsed them differently, so the six are one reading and not a settled shape. Decides: protocol
owner. Default if nobody answers: **do not publish**, record the decision, and ship the
`adopting.md` fix alone — it is the half that helps every adopter rather than every ingestion
adopter, and an unpublished set costs nothing to publish later while a published one cannot be
withdrawn.

**Does `deduplication` belong in a phase set at all?** It is the only one of the six that names a
technique rather than a stage of work. Decides: protocol owner, with the same default.
