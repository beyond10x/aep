---
format: aep.planning-md/1
id: story:migration-receipt-identity-under-symlinked-selector
kind: story
status: draft
title: A migration apply reached through a symlinked path is decided like the real path
owner: aep
relations:
- informed_by: review-result:validate-v2-review-1
- informed_by: story:eventlog-planning-authority-migration
revision: 1
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
