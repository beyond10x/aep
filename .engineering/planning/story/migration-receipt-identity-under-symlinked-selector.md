---
format: aep.planning-md/3
id: story:migration-receipt-identity-under-symlinked-selector
kind: story
status: archived
title: A migration apply reached through a symlinked path is decided like the real path
owner: aep
relations:
- informed_by: review-result:validate-v2-review-1
- informed_by: story:eventlog-planning-authority-migration
- decomposes: epic:zero-open-issues-2026-11-09
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T22:07:32Z", actor: "human:timo", revision: 3}
---
## Outcome

A public migration `apply` reached through a symlinked project path is decided the same way as one
reached through the real path: the receipt identity does not depend on how the selector's host path
was spelled.

## Observation this comes from

review-result:validate-v2-review-1, finding 3 (2026-09-20). Runtime observation, three runs, one
variable, mechanism **not isolated**:

- With `TMPDIR` set to a short symlink to the assigned scratch directory, the migration fixture in
  `crates/edge/aep-cli/tests/store_writer_control.rs:323` (`apply_selected_source`) refuses with
  `{"outcome":{"kind":"refused","value":{"refusals":[{"code":"receipt_conflict"}]}}}`, and the
  existing case `validate_answers_a_migrated_eventlog_plan_from_its_authority_not_the_frozen_legacy_journal`
  fails there.
- The same test passes unchanged with `TMPDIR` set to a short real directory.
- Reviewer's hypothesis, labelled as such: the receipt comparison compares selector host paths as
  given rather than canonicalised. Unproven.
- Separately, with `TMPDIR` longer than about 108 characters `sccache` refuses every cargo
  invocation with `path must be shorter than SUN_LEN`; that is tooling, not AEP, and is handled by
  assigning short scratch roots in briefs.

## Acceptance

- Reproduce the refusal with a fixture whose project root is a symlink; identify the comparison that
  differs; either canonicalise consistently or refuse with a code that names the path mismatch.
- The existing public-path cases stay green under both a real and a symlinked `TMPDIR`.

## Scope

inferred: crates/plan/aep-planning-migration/, crates/edge/aep-cli/src/store_command.rs,
crates/edge/aep-cli/tests/store_writer_control.rs.

## Closed 2026-11-09

Obsolete. The Eventlog migration `apply` and its receipts went in 0.62.0. Commit `742bb0713` ("refactor: remove Eventlog from AEP", released in 0.62.0) deleted the `aep-planning-migration` and `aep-backend-eventlog` crates; `CHANGELOG.md` 0.62.0 *Removed* lists it. A search for `receipt_conflict` and `ReceiptConflict` over `crates/` at `824869fec` finds nothing. The one migration left, `aep plan store migrate git`, writes no receipt.
