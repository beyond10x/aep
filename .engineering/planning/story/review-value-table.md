---
format: aep.planning-md/1
id: story:review-value-table
kind: story
status: draft
title: One table says, per critic, what its findings changed and what its verdicts cost
summary: 'aep artifact review-value: per reviewer, reviews, findings, no-op, fixed, escalated, cost or unknown; counts only, no score.'
owner: protocol
tags:
- cli
- review
relations:
- decomposes: epic:review-facts
- depends_on: story:review-outcome-field
- depends_on: story:structured-findings-on-review-result
- serves: vision:O6
revision: 1
---
# Story: One table says, per critic, what its findings changed and what its verdicts cost

## Outcome

The operator reads one table — per reviewer: reviews, findings, `no-op`, `fixed`, `escalated`, and cost where a run manifest recorded it — and decides which lenses stay, on numbers this store produced.

## Context

bdfinst publishes per-arm cost and finding counts from their `review-value.jsonl` and experiments; this stack has the facts nowhere. `aep eval matrix` already renders counts per cell and refuses to score; this table takes the same stance.

## Acceptance

- `aep artifact review-value [--since <date>]` prints one row per reviewer (the `owner` or `reviewer` field of the `review-result`) with the counts above.
- Cost is read from a run manifest referenced by the review's `--ref`, when present; absent cost prints as `unknown`, never as 0.
- No column is a ratio or a score; the help text says why, in the words `eval matrix` uses.
- `--format json` for the site.

## Out of Scope

A time series. One table per invocation.

## Ambiguities

- `inferable` — depends on `story:review-outcome-field` for its input and on `story:structured-findings-on-review-result` for finding counts.

## Open Questions

None.
