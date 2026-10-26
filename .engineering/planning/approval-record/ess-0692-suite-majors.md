---
format: aep.planning-md/3
id: approval-record:ess-0692-suite-majors
kind: approval-record
status: approved
title: 'Approved: evidence admits the suite majors ESS 0.69.2 writes'
summary: 'Orchestrator routing of the agency report, 2026-10-26: aep 0.77.1 refuses an ess-conformance/54 suite; decide the admitted range, fix, release.'
owner: project
tags:
- evidence
relations:
- decides: story:evidence-admits-ess-0692-suites
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-10-26T17:31:55Z", actor: "human:timo", revision: 3}
---
# Approval record: evidence admits the suite majors ESS 0.69.2 writes

## Decision

An item routed to this repository by `b10x-chef-orchestrator` on 2026-10-26 (dispatch-queue row
15), in these words:

> `aep plan artifact evidence --from <suite file>` on aep 0.77.1 refuses a current ESS conformance
> suite document with `UnsupportedSuiteVersion` (reported today by the agency repository on an
> `ess-conformance/54` suite produced by ess 0.69.2 […]). Reproduce […], then through the store: a
> story with Scope and conformance acceptance […], implement so that `evidence --from` admits the
> suite formats current ESS releases emit (read ess's suite-format history […] to decide the
> admitted range and whether a forward-compatible rule is right; record the decision in the store),
> adversary pass, gate (`task check`), bot PR, merge by the route AGENTS.md names, release per
> AGENTS.md […]. Approved under the grant in […].

The grant named is the workspace operator's standing approval for class-C items relayed by the
orchestrator. Decider: operator.

## What this record authorizes

- The story it decides, as one unit, the ESS pin move to 0.69.2 that goes with it, and the source
  release that ships them, each through this repository's own process (`AGENTS.md` § *Releases and
  commits*).

## Decisions taken inside the item

- D1 Admitted range: every major ESS 0.69.2 reads, ordinary 6–56 and coverage 5–57, by explicit
  list. No forward-compatible rule: ESS's own readers refuse a later major, 52/53 added root
  members a closed reader cannot know and 56/57 changed what an existing id spelling names, so
  admission by number would trade a named refusal for an unknown-field refusal or a misreading.
  The lists carry the ESS release they were transcribed from (`SUITE_MAJORS_TRANSCRIBED_FROM`),
  and the ESS gate refuses a pin other than it, so the next major is met where the pin moves.
- D2 A `--suite-input` lineage whose suites carry a suite-level catalog (52 and later) is refused
  by name: ESS's selection re-spells survivors against a trimmed catalog, and this build compares
  survivors byte for byte. The complete suite, the reported case, is admitted.
