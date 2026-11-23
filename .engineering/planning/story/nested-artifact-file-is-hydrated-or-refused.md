---
format: aep.planning-md/3
id: story:nested-artifact-file-is-hydrated-or-refused
kind: story
status: draft
title: A nested artifact file is hydrated or refused
summary: story/a/b.md validates and is listed but is not hydrated; validation and listing should agree with hydration.
owner: aep
tags:
- planning-store
relations:
- serves: vision:O2
revision: 1
---
# Story: A nested artifact file is hydrated or refused

## Outcome

Validation, listing and hydration agree on which files in a kind directory are artifacts.

## Context

A file `story/a/b.md` validates and is listed, but hydration of the store into the backend does not
read below the kind directory, so the artifact is absent from every answer the backend gives. The
store reads as consistent to one tool and as missing the artifact to the other.

Found by the planning-store review of 2026-11-23 (`aep-planning-store-review`).

## Acceptance

- Either `aep plan artifact validate` refuses a store holding `story/a/b.md`, naming the file, or
  the backend answers for that artifact like for a flat one. A test holds the chosen outcome; the
  files `validate` and `list` accept are exactly the files hydration reads.

## Out of Scope

Nested directories for any other purpose, such as a store of stores.

## Open Questions

- Refuse or hydrate? Refusing is the default of this story, because the file stem is the id's
  name part (`aep plan artifact new <kind> <name>`) and a nested path has no id of its own.
