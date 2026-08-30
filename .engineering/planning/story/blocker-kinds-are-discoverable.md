---
format: aep.planning-md/1
id: story:blocker-kinds-are-discoverable
kind: story
status: implemented
title: '`kinds` lists the blocker kinds, and `blocked` says when there are none'
summary: kinds lists no blocker kind while lifecycle third-party-blocker works; blocked says nothing is blocked where no blocker ladder exists.
tags:
- cli
- store
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
- informed_by: story:blocker-relation
revision: 4
---
# Story: `kinds` lists the blocker kinds, and `blocked` says when there are none

## Outcome

The verb the skill names as the authority on "what can I create" lists what can be created.

## Context

- `fcf5873a#361-#362` (reproduced live 2026-08-30): `protocol artifact kinds | grep -i block` → 0 lines;
  `protocol artifact lifecycle third-party-blocker` works. `kinds` iterates `ArtifactKind::NAMED`
  (`crates/protocol-cli/src/planning.rs:2773`), which has no blocker family; the family is open
  (`<type>-blocker`, `crates/aep-domain/src/artifact.rs`, `blocker_type`).
- `431986de#7007`: `blocked` answered `nothing is blocked` in a store whose pin predates
  `artifacts/kinds/blocker.yaml`; operator `#7024`: "what are you talking about blockers".
- `11727595#3566`: guardrail 6's mechanism unavailable in the harness store.

## Acceptance

- `kinds` lists every kind `artifacts/kinds/*.yaml` and `artifacts/lifecycles/*.yaml` declare, plus the
  built-in list, plus one row `<type>-blocker  planning  (open family: credential-blocker, decision-blocker, …)`.
- `blocked` prints "this store's lifecycles declare no blocker kind; `protocol artifact kinds` lists what
  can be created" when no blocker ladder resolves, and `nothing is blocked` only when one does.
- `cli.md` row for `kinds` no longer says "the 26 artifact kinds".

## Out of Scope

Adding a blocker ladder to older `adp/1` pins.
