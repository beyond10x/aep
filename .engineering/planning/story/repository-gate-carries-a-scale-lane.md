---
format: aep.planning-md/2
id: story:repository-gate-carries-a-scale-lane
kind: story
status: draft
title: The repository gate runs the store commands on a real-size fixture with counted budgets
relations:
- serves: vision:O2
- informed_by: story:migration-import-costs-one-capture-per-batch
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/edge/aep-cli/tests
revision: 3
---
## Outcome

The AEP repository gate carries a scale lane: on a committed fixture of real-store size (hundreds of
artifacts, journal in the tens of MB), `plan artifact list`, `history`, `validate`, `plan store
migrate apply` and `verify` each run inside a stated budget, and the lane goes red when a change
makes any of them cost more than the budget. A cost that grows with the store's size is caught
before a cutover, not on one.

## Why

2026-09-21, wave-validate-v2-20260920. Four separate places did a full re-read or re-verification of
the whole store on every operation, each locally defensible, none measured at a real store's size:

| layer | per-operation cost | found by |
| --- | --- | --- |
| Eventlog file provider | whole chain and every blob re-verified per transaction | measuring `list` on a migrated copy: 79.7 s (units 3, 6) |
| Entity Runtime adapter | whole tenant captured per read | profile of `list`: 222 captures per command (unit 7) |
| AEP store open | one capture per record (`hydrate`) | same profile (unit 7): 76 s → 1.25 s |
| AEP migration import | one capture and one transaction per subject | the ESS cutover: 448 artifacts, ~5.5 h (unit 9) |

Every test fixture in the repositories has 3–10 artifacts; the rehearsal copy had 64. No gate lane
ever saw the cost, and the operator did on the fourth real store.

## Acceptance

- A fixture store of at least 400 artifacts with relations and history, generated deterministically
  in the test (not a copy of a real store), committed as a generator, not as files.
- One lane, in `task check`, that opens it and runs `list`, `history <id>`, `validate`, `migrate
  dry-run`, `migrate apply` on a copy, `verify`, each with a wall-clock budget and a counted budget
  (captures, transactions, bytes hashed, through the counting seams units 7 and 9 added). The
  counted budgets are the assertion; the wall clock is recorded, not asserted, so the lane is not
  flaky on a loaded machine.
- Budgets at adoption: `list` and `validate` ≤ 2 captures; `apply` ≤ 2 captures and one append
  group; bytes hashed ≤ 2 × the fixture's size per command. Numbers measured at adoption and stated
  in the lane's own doc comment with their date.
- The lane runs in the 16-step gate and in CI; its wall-clock numbers go into the gate log so a
  drift is visible before a budget is broken.
- The same shape is filed for the Eventlog repository (provider-level: bytes hashed per capture and
  per transaction on a 20 MB journal) as its own story.
