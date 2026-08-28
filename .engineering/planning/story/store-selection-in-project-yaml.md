---
format: aep.planning-md/1
id: story:store-selection-in-project-yaml
kind: story
status: draft
title: project.yaml names the store, and every verb opens it
summary: 'aep.project/1 gains store: markdown | sqlite: <path> | hybrid: {four policy words, local, replica}; every protocol artifact verb opens through it; a hybrid missing a word is refused by name.'
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:sqlite-hydrates-on-open
- depends_on: story:markdown-backend-is-the-adapter
revision: 5
---
# Story: `project.yaml` names the store, and every verb opens it

## Outcome

A team keeps its plan in SQLite by writing one line in `.engineering/project.yaml`, and every
`protocol artifact` verb they already use keeps working — the acceptance `story:sqlite-backend`
promised in 2026-08-21 and nothing has delivered.

## Context

`crates/protocol-cli/src/planning.rs:501-503` opens `MarkdownBackend` and nothing else; `BackendArgs`
(`main.rs:200-219`) has no store selector; `aep.project/1` (`crates/aep-domain/src/project.rs`) has
no `store` key. After F4 a SQLite store can be reopened and after G2 the markdown store is the same
adapter, so the choice is one type parameter — which is exactly what a configuration line should
select.

Wave H, story 1 (`docs/plan/store-waves-f-g-h.md`). After F4 and G2.

## Acceptance

- `aep.project/1` gains `store:` with three forms — `markdown` (the default, so no existing project
  changes meaning), `sqlite: <path>`, and `hybrid: {authority, read, on_unreachable, on_divergence,
  local: <form>, replica: <form>}`; a `hybrid` missing any of its four policy words is refused at
  `protocol validate` (runtime R-106, enforced at our edge), and the refusal names the word.
- Every `protocol artifact` verb, `protocol conformance --backend project` and the driver's store
  lock open through the configured store; `--store <path>` remains the override for the markdown form.
- A relative `sqlite:` path resolves against `.engineering/`, as every other project path does
  (`ProjectPaths::resolved`).
- `examples/planning-passkeys/` gains a second `project.yaml` variant on SQLite, and a test seeds the
  fixture plan into both and asserts every verb's output is identical.
- `docs/guide/backend.md` and the schema under `schemas/generated/` carry the key.

## Out of Scope

The hybrid's behaviour (H4) — this story only carries its words. Migration between stores.

## Open Questions

None outstanding.
