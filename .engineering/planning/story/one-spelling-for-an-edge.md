---
format: aep.planning-md/1
id: story:one-spelling-for-an-edge
kind: story
status: active
title: One spelling for an edge, and a way to take one back
summary: relate takes three positionals while new --relate takes rel:id; a wrong edge is permanent because there is no unrelate.
tags:
- cli
- store
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/protocol-cli
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/plan/aep-backend-markdown
- confidence: cited
  path: website/docs/reference/cli.md
revision: 10
---
# Story: One spelling for an edge, and a way to take one back

## Outcome

`relate` accepts the `<relation>:<id>` form `new --relate` already uses, and `unrelate` removes an edge.

## Context

- `cc946bc3#486`: `protocol artifact relate <id> serves:vision:O2` refused — `relate` takes three
  positionals; both stories were already `active`, so `validate` went red mid-run.
- `2e81f991#89`: `relate story:… --add decomposes:epic:self-evaluation` → `unexpected argument '--add'`.
- `9da4f51c#495`: "`protocol artifact` has `relate` and no `unrelate` — the stale `depends_on` edge from
  phase 1 to phase 0 cannot be removed"; the wrong edge is still in entity-runtime's store.

## Landed 2026-08-30

- `relate <id> <relation>:<target>` beside the three-positional form, journalled identically.

## Still open — `unrelate` needs the markdown backend first

`aep-domain` declares `Command::RemoveRelation` (`crates/govern/aep-domain/src/command.rs:71,113,326`) and
`aep-backend-memory` implements it, but `aep-backend-markdown` refuses to project it on purpose:
`MarkdownProjection::before` (`crates/plan/aep-backend-markdown/src/projection.rs:486-494`) excludes it
("the frontmatter adds and does not remove") and `apply_relations` (`:364-403`) never deletes. A
CLI verb today would issue a command the store silently drops. The edit is three-fold: `before`
resolves the relation's source; `apply_relations` removes exactly the named `(kind, target)` and
nothing else (a hand-written edge the backend never authored must survive); `journal::Change` gains
`Unrelated`. Then ~40 lines in `protocol-cli`.

## Acceptance

- `protocol artifact relate <id> <relation>:<target>` and `relate <id> <relation> <target>` both work and
  journal identically.
- `protocol artifact unrelate <id> <relation> <target>` removes exactly that edge through a command,
  bumps the revision, and is refused when the edge is not there, naming the edges that are.
- `cli.md` documents both.

## Out of Scope

Renaming a relation kind.
