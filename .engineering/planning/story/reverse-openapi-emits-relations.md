---
format: aep.planning-md/1
id: story:reverse-openapi-emits-relations
kind: story
status: implemented
title: aep reverse openapi drafts relations from $ref and id fields, UNMAPPED where cardinality is unknown
summary: A $ref or an <x>_id property becomes a references relation in the ess/1 draft; ownership and unknown cardinality are marked UNMAPPED, never guessed.
owner: protocol
tags:
- ess
- reverse
relations:
- informed_by: epic:adopter-feedback-round-2
- serves: vision:O2
revision: 4
---
# Story: `aep reverse openapi` drafts `relations:` from `$ref` and id fields, `UNMAPPED:` where cardinality is unknown

## Outcome

An adopter with an OpenAPI document gets an `ess/1` draft whose entities already carry the relations the document implies — and an `UNMAPPED:` marker on every relation whose cardinality or ownership the document does not state.

## Context

`aep reverse openapi` drafts an `ess/1` domain and emits `UNMAPPED:` comments for every decision it cannot take (`CHANGELOG.md`, 0.14.0). ESS is adding a `relations:` construct (`ess` `epic:entity-relations`); until it ships, this story is blocked on the vocabulary being decided there. A `$ref` to another schema, or a property named `<entity>_id` whose type matches another entity's identity, is the signal.

## Acceptance

- A `$ref` from one object schema to another becomes a `references` relation with cardinality `one`, or `many` when the property is an array.
- A property `<x>_id` whose schema matches entity `X`'s identity becomes a `references` relation marked `UNMAPPED: ownership` — the document cannot say whether it is `owns`.
- Every emitted relation validates under `ess validate` once ESS ships the construct; a fixture OpenAPI document with a known draft diffs byte-exact.
- Nothing is guessed: no relation is emitted without one of the two signals.

## Out of Scope

Reading a database schema. OpenAPI only.

## Ambiguities

- `inferable` — the `UNMAPPED:` convention already exists in this verb.
- `requires-stakeholder-input` — the relation vocabulary, decided in `ess` (`decision-blocker:relation-vocabulary` there). This story cannot start before it.

## Open Questions

None.
