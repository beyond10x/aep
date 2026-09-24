---
format: aep.planning-md/2
id: task:validate-v2-projection-awareness
kind: task
status: implemented
title: validate answers a migrated Eventlog plan from its authority
owner: aep
relations:
- decomposes: story:eventlog-planning-authority-migration
- serves: vision:O2
revision: 5
---
## Outcome

`aep plan artifact validate` on an `aep.project/2` Eventlog plan reconciles its documents against
the authority and not against the frozen legacy `journal.jsonl` under the projection. An **owned**
projection file that was edited or deleted by hand is reported as drift decided by the authority.
A document added by hand at a path the ownership marker does not list is not reported by this
change; that gap is story:unowned-document-in-eventlog-projection-is-reported (review-result:validate-v2-review-1, finding 2).

## Why

On 2026-09-19 one governed `move` on a migrated disposable copy had whole-plan `validate` report
"drifted from its log" and "claims revision 2, and no write produced it", while `plan store verify`
on the same tree answered `current` (.ess-evolution/waves/0005-aep-migration/validate-v2-drift-20260919/report.md §3).
`task plan-check` is step 3 of the 16-step gate (Taskfile.yml:124), so every migrated store would
fail its own gate on its first status move. Before this correction nothing in `validate` read the
projected files at all: a genuinely hand-edited projection was invisible (report.md §1).

## Implementation state at filing

Implemented 2026-09-19 as an uncommitted diff on `integrate/ess-evolution-public-control-20260918`
at base effef5b1a30019f4c9c2e143abe614cc9d5db89b, frozen at
.ess-evolution/waves/0005-aep-migration/validate-v2-drift-20260919/frozen.diff, sha256
6d4f49412ba06cfcd7c864287f131d58d165fec43447515d134cfc8834394aa5. `Opened::journal()` answers
`None` for an Eventlog plan; `findings` reconciles drift, forged, deleted and `pre_provider`
through it and, for Eventlog, calls `store_command::projection_current`. Evidence: aep-cli executed
578 -> 580 (05-green-aep-cli.log), `task check` 16 steps exit 0 with actual PostgreSQL
(10-task-check.exit), mutation probe named the hand-edit case (04-mutation-run.log).

## Acceptance

- `validate_answers_a_migrated_eventlog_plan_from_its_authority_not_the_frozen_legacy_journal` and
  `validate_reports_a_hand_edited_eventlog_projection_as_drift_decided_by_the_authority` pass in
  `crates/edge/aep-cli/tests/store_writer_control.rs`; `tests/drift.rs` is unchanged and passes.
- Two independent review passes recorded as `review-result`, with no open `introduced` finding.
- One bot commit on the integration branch; `task check` per-step exit 0; common check and verify
  receipts retained.

## Scope

cited (diff-stat.txt): CHANGELOG.md, crates/edge/aep-cli/src/planning.rs,
crates/edge/aep-cli/src/store_command.rs, crates/edge/aep-cli/tests/store_writer_control.rs.
