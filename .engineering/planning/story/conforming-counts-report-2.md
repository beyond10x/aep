---
format: aep.planning-md/3
id: story:conforming-counts-report-2
kind: story
status: active
title: A passed report/2 moves a specification to conforming
refs:
- provider: github
  reference: beyond10x/aep#85
relations:
- serves: vision:O2
- informed_by: epic:evidence-gated-completion
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T15:45:20Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T15:45:20Z", actor: "human:timo", revision: 3}
---
## Outcome

A passed ESS conformance report of any admitted shape (report/1, report/2, report/2 with coverage) moves an `executable-system-specification` to `conforming`; a failed or inconclusive one does not.

## Acceptance

- `move --to conforming` succeeds after a passed `ess_conformance_v2` or `ess_conformance_coverage_v1` record whose model digest matches the artifact's `model_digest`, as it does after a passed `ess_conformance` record.
- It is refused after a failed or inconclusive record of any of the three kinds, and after a record for another digest, naming why.
- The refusal text names the kinds that would satisfy the rung.

## Origin

beyond10x/aep#85, reported downstream on aep 0.67.0 (a report/2 of 677 of 677 passed could not move its specification to conforming).

## Fit review

Per the ESS repository's `.agents/skills/assessing-external-requests` rules: class defect (aep contradicts its documented flow: the planning skill says a report/2 beside `--suite` moves a specification to `conforming`). No new authored surface beyond, possibly, an any-of requirement form.

## Decisions

- **accept** (coordinator, 2026-09-30); the implementor chooses between an any-of requirement in the lifecycle grammar and naming the three kinds, and states why.
