---
format: aep.planning-md/1
id: story:journal-carries-digests-not-bodies
kind: story
status: draft
title: The journal records a body by digest and diff, and refuses a home directory
summary: harness journal is 670 KB over 251 events for 166 KB of artifacts; an absolute path journalled is permanent and got sed -i'd.
tags:
- store
relations:
- decomposes: epic:evidence-gated-completion
- serves: vision:O2
scope:
- confidence: cited
  path: crates/aep-backend-markdown
- confidence: inferred
  path: crates/aep-domain
- confidence: cited
  path: crates/protocol-cli
revision: 4
---
# Story: The journal records a body by digest and diff, and refuses a home directory

## Outcome

`journal.jsonl` stops being four times the store it records, and an absolute path cannot become
permanent by being journalled.

## Context

- harness `.engineering/planning/journal.jsonl`: 670 KB over 251 events against 166 KB of artifacts
  (`114c2340` audit `wc -c`) — every `update` carries the whole body.
- `114c2340#32`: harness exempts the journal from `check-no-home-paths.py` (`:85-86`) because "editing
  either to satisfy this check would forge the record"; `97e74f6d#39` ran `sed -i` over the journal anyway.

## Acceptance

- An `update` event carries `body_digest` and a unified diff against the previous revision; `history`
  and `explain` render the diff; `validate` recomputes the digest chain.
- `body`, `new --from` and `set` refuse a body or field containing `/home/<user>/` or `$HOME` unless
  `--allow-home-path` is given, and the refusal names the line.
- Existing journals with full bodies stay readable; `validate` reports how many events predate the digest form.

## Out of Scope

Rewriting existing journals.
