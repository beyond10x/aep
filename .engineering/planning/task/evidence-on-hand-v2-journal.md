---
format: aep.planning-md/1
id: task:evidence-on-hand-v2-journal
kind: task
status: implemented
title: Evidence, reviews and outcomes on a migrated Eventlog plan read the authority
owner: aep
relations:
- decomposes: story:eventlog-planning-authority-migration
- serves: vision:O2
- depends_on: task:validate-v2-projection-awareness
revision: 4
---
## Outcome

Every remaining reader of `opened.files` that answers a history question reads through
`Opened::journal()`, so on a migrated Eventlog plan an evidence-gated `move`, `explain`,
`history`, `findings` and `review-value` answer from the authority and not from the frozen legacy
journal under the projection. Markdown and Hybrid plans keep byte-identical behaviour.

## Why

The validate correction (task:validate-v2-projection-awareness) left four same-class sites open,
enumerated in .ess-evolution/waves/0005-aep-migration/validate-v2-drift-20260919/report.md §5:

- `Opened::evidence_on_hand` (crates/edge/aep-cli/src/planning.rs:838) still matches on
  `self.files`; it feeds `move` at planning.rs:2857 and `explain` at 7072, 7321 and 8672. On a
  migrated plan it reads the frozen legacy journal, so evidence recorded after migration is
  invisible to a `move` whose rung requires it. The `None` arm `evidence_from_events` is already the
  Eventlog answer.
- `reviews_of` (planning.rs:5489, ordering only), `review_records` (planning.rs:5797) and
  `outcomes_of` (planning.rs:6650) read `opened.files` directly.

Migrating a real store with this open reproduces the M4a halt one rung later: the first
evidence-gated move on a cut-over store is refused by the store's own reader.

## Acceptance

- Red first, on a migrated Eventlog plan fixture (reuse `migrated_eventlog_plan_after_one_governed_move`
  in `tests/store_writer_control.rs`): record `aep plan artifact evidence` after migration, then
  `move` to a status whose lifecycle rung requires that evidence kind. Base refuses; corrected
  admits and `explain` names the recorded evidence.
- A second red case: a `review_outcome` recorded after migration is shown by `findings` and
  `review-value` and by `explain` on the reviewed artifact.
- `tests/drift.rs`, `tests/planning_cli.rs`, `tests/wave_derivation.rs` unchanged and passing.
- The `asserted` provenance list on v2 stays as decided in the validate correction: no journal.
- Two independent review passes recorded as `review-result`; one bot commit on the integration
  branch; `task check` per-step exit 0.

## Scope

cited (report.md §5, planning.rs line numbers read 2026-09-20): crates/edge/aep-cli/src/planning.rs,
crates/edge/aep-cli/tests/store_writer_control.rs, CHANGELOG.md.
inferred: crates/edge/aep-cli/src/store_command.rs.
