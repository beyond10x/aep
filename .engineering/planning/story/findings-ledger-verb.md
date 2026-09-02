---
format: aep.planning-md/1
id: story:findings-ledger-verb
kind: story
status: implemented
title: aep artifact findings says what the second review found that the first did not
summary: Carried, new, resolved between two review-results by signature (reviewer, file, category, normalised message, line +-3).
owner: protocol
tags:
- cli
- review
relations:
- decomposes: epic:review-facts
- depends_on: story:structured-findings-on-review-result
- serves: vision:O2
revision: 4
---
# Story: `aep artifact findings` says what the second review found that the first did not

## Outcome

After a second adversary attack or a second critic round, the coordinator reads three lists — carried, new, resolved — and decides from them whether a third round is worth anything, instead of comparing two prose reports by eye.

## Context

The wave skill's attack budget is a sentence: "After two attacks, hand it over — do not open a third" (`agentplugins/plugins/adp/skills/wave/SKILL.md:421` at 0.4.0). `agentplugins` `story:finding-signature-ledger` (draft) describes the instruction form; this verb is the computation. The signature follows bdfinst's: `(reviewer, file, category, normalised message)` with a line tolerance of ±3 (`finding_signature.py` at `dev-team-v13.0.0`).

## Acceptance

- `aep artifact findings <artifact-id>` takes the two most recent `review-result` records that `reviews` the artifact (or `--from <id> --to <id>`) and prints `carried`, `new`, `resolved` with the signature of each.
- The signature ignores line drift within ±3 and whitespace/case in the message; a test shows a moved finding classified as carried.
- Exit 0 always; `--format json` for the skill.
- Two reviews by different reviewers are compared by file+category+message, and the reviewer is printed, not matched.

## Out of Scope

Deciding whether to attack again. The skill reads the counts and the trend and the operator decides.

## Ambiguities

- `inferable` — depends on `story:structured-findings-on-review-result` for its input.

## Open Questions

None.
