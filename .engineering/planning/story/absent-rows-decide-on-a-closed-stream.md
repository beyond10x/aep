---
format: aep.planning-md/1
id: story:absent-rows-decide-on-a-closed-stream
kind: story
status: draft
title: A negative expectation is decided when the stream is closed
summary: aep trace check answers tool.absent / never_occurred rows as ok or gap when metaharness marks the stream complete; eight P1 cases ended undecided on exactly those rows.
owner: eval
tags:
- bench
- trace
relations:
- decomposes: epic:checker-vocabulary-depth
- serves: vision:O3
revision: 1
---
# Story: A negative expectation is decided when the stream is closed

## Outcome

`the tool was never called` and `nothing was moved` are answered `ok` or `gap` on a complete transcript, and `unknown` only when the stream really is incomplete — so a case stops ending "undecided" for rows the run did settle.

## Context

Eight live eval cases on 2026-09-03 (P1 of the bench plan) ended "undecided: nothing was contradicted and N expectation(s) could not be judged"; every unknown row was a negative one (`nothing-was-moved`, `no-status-was-moved`, `no-store-command-was-run`, `nothing-was-written-to-tmp`, `no-judged-body-was-rewritten`). `aep trace check` cannot know whether an absence is a fact or a hole in the stream, so it says `unk`. metaharness knows: it owns the stream and ends it.

## Acceptance

- The checker reads a terminal completeness marker on the stream (the event metaharness emits when it closes a run, or the attestation's completeness statement) and, when present, decides `absent`/`never_occurred` rows as `ok` or `gap`.
- Without the marker the rows stay `unknown` and the report says which marker was missing.
- The eight P1 reports re-checked offline with the marker present drop to 0 unknown rows for those kinds; the golden-path case keeps its real gaps.
- `aep eval matrix` counts move from `unknown` to held/contradicted for those cells.

## Out of Scope

Deciding on a truncated stream; the marker is the only witness accepted.

## Ambiguities

- `inferable` — the marker's shape is metaharness's (`story:stream-closed-marker` there); until it ships the attestation's existing fields are read if they state completeness.

## Open Questions

None.
