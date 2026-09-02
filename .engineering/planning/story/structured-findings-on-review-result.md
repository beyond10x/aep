---
format: aep.planning-md/1
id: story:structured-findings-on-review-result
kind: story
status: draft
title: A review-result carries its findings as data, not only prose
summary: A fenced findings block parsed at new, returned by show --format json, reported by validate when absent.
owner: protocol
tags:
- review
relations:
- decomposes: epic:review-facts
- serves: vision:O2
revision: 1
---
# Story: A review-result carries its findings as data, not only prose

## Outcome

A reader, a verb or a later review can enumerate a review's findings — file, line, category, severity, verdict, origin — without parsing sentences.

## Context

`review-result` is immutable and its body arrives with `new --from`. The adversary's report already names `what was measured` and `what reaches it` per finding (`agentplugins/plugins/adp/agents/adversary.md:113-132` at 0.4.0) and the critics return `approve | needs-revision` with findings. Nothing in the store reads them.

## Acceptance

- A `review-result` body may carry a fenced `findings` block (YAML) with entries `{file, line, category, severity, verdict, origin, message}`; the CLI parses it at `new` and refuses a malformed block.
- `aep artifact show --format json` returns the findings as an array.
- `aep artifact validate` reports (without failing) a `review-result` with no findings block.
- The critic rubric and the adversary file (in `agentplugins`) are updated to emit the block — filed there, referenced here.

## Out of Scope

Retro-fitting existing `review-result` records; they are immutable.

## Ambiguities

- `inferable` — `verdict` values: the adversary's `CONFIRMED | NEEDS-CHANGE | INFEASIBLE` and origin `introduced | pre-existing | undecided` (`adversary.md:154-169`); critics use `blocker | warning | note` as severity (`critic-rubric.md` at 0.4.0).

## Open Questions

None.
