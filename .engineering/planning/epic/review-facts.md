---
format: aep.planning-md/1
id: epic:review-facts
kind: epic
status: implemented
title: Reviews leave facts the next review and the operator can compute over
summary: Structured findings on review-result, a findings ledger verb, a recorded outcome per review, and a counts-only review-value table.
owner: protocol
tags:
- evidence
- review
relations:
- serves: vision:O2
- serves: vision:O6
revision: 4
---
# Epic: Reviews leave facts the next review and the operator can compute over

## Outcome

A second review of the same work knows what the first found; a critic's verdict records what became of it; and the operator can read, per critic, how often its findings changed anything and what each verdict cost — from the store, not from memory.

## Why Now

`agentplugins` 0.4.0 added four plan-time critics whose verdicts are immutable `review-result` artifacts, and the wave's adversary already produced findings before that. Both leave prose. The 2026-09-02 comparison against `bdfinst/agentic-dev-team` found two things they compute that this stack asserts: a finding-signature ledger (`finding_signature.py`, 428 lines) that classifies each finding as carried or new so a loop knows whether it is converging, and a per-checkpoint review-value record (`no-op | fixed | escalated`) that decides which lenses earn their cost. Their numbers rest on those records ($0.341 vs $0.117 per small-tier run, in their corpus). This repository publishes no such number because it records no such fact. `agentplugins` holds the instruction-form ledger (`story:finding-signature-ledger`); this epic is the data it needs.

## Scope

Structured findings on `review-result`; a verb that compares two reviews; an `outcome` on `review-result`; and a table over them in the shape `aep eval matrix` already uses (counts, never a score).

## Out of Scope

- Scoring critics. The table counts; ranking is the operator's.
- Changing `review-result`'s immutability. `outcome` is recorded as a follow-up record that references the review, not as an edit to it.
- Cost attribution below the level a run manifest already carries.

## Risks

- Findings written as free text keep being written as free text. Mitigation: the critic and adversary agent files (in `agentplugins`) emit the structured block, and `validate` reports a `review-result` without one.

## Ambiguities

- `inferable` — the three outcome values: `no-op | fixed | escalated`, as bdfinst's `review-value.jsonl` uses them (`plugins/dev-team/skills/build/SKILL.md:234-259` at `dev-team-v13.0.0`).
- `inferable` — `review-result` refuses `body` after creation (`aep artifact new --help`), so the outcome cannot live on the same record.
- `requires-stakeholder-input` — whether the outcome record is a new kind or an evidence record. Decides: protocol owner. Default: an evidence record of kind `review_outcome` against the reviewed artifact, referencing the `review-result` id.

## Done When

`aep artifact findings <id>` classifies the findings of the two latest reviews of an artifact as carried, new or resolved; every `review-result` created by the critic step or the wave carries structured findings and later an outcome; `aep artifact review-value` prints the table for this store.
