---
format: aep.planning-md/1
id: story:agreed-work-serves-an-objective
kind: story
status: implemented
title: Agreed work names the objective it serves, and validate holds it to that
summary: A fourteenth relation, serves, points at a vision artifact; once a store declares one, every proposed, approved or active story or task must serve one
owner: protocol
tags:
- grounding
- store
relations:
- decomposes: epic:evidence-gated-completion
- serves: vision:O2
revision: 5
---
# Story: Agreed work names the objective it serves, and validate holds it to that

## Outcome

atlas `ROADMAP.md` gives the collection six objectives, `O1`–`O6`, and every repository's
`AGENTS.md` says which it serves. That grounds a repository and not a story. Here the objectives
are `vision:O1`…`vision:O6`, a story `serves` one, and `protocol artifact validate` names each
`proposed`, `approved` or `active` story or task that serves none — once the store has declared an
objective at all. atlas ADR 0005 records the decision and its exemptions.

## Acceptance

- `serves` is a relation the engine acts on: `validate` refuses a `serves` into any kind but
  `vision`, by name.
- Where the store holds at least one `vision`, a `proposed`/`approved`/`active` story or task with
  no `serves` edge is a validation problem; `draft`, `implemented` and `archived` are exempt.
- A store with no `vision` is untouched by the rule.
- This store declares the six and every live story serves one; `plan-check` is green.
- `docs/guide/adopting.md` counts fourteen relations and says what `serves` does; `CHANGELOG.md`
  carries the entry; schemas regenerated.

## Evidence

`crates/protocol-cli/tests/planning_cli.rs::validate_holds_agreed_work_to_an_objective_once_the_store_declares_one`;
the full gate.
