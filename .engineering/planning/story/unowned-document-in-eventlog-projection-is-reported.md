---
format: aep.planning-md/2
id: story:unowned-document-in-eventlog-projection-is-reported
kind: story
status: draft
title: A document added by hand to an Eventlog projection is reported
owner: aep
relations:
- informed_by: review-result:validate-v2-review-1
- informed_by: story:eventlog-planning-authority-migration
revision: 1
---
## Outcome

On an `aep.project/2` Eventlog plan, a planning document written by hand into the Markdown
projection directory, at a path the ownership marker does not list, is reported by
`aep plan artifact validate` and by `aep plan store verify` as a change nothing decided.

## Finding this comes from

review-result:validate-v2-review-1, finding 2 (2026-09-20, pass 1 over af2af7e74c0230dbb5f8a202e68b719dbfbed7a4):

- `crates/edge/aep-cli/src/store_command.rs:1584` — `projection_current` inherits from
  `projection_owned_inventory`; `read_owned_files` (`crates/plan/aep-planning-migration/src/projection.rs:693`)
  reads only the paths the ownership marker lists, so a path the marker does not name is in no digest.
- Measured: `tests/store_writer_control.rs` case
  `validate_reports_a_foreign_document_planted_in_an_eventlog_projection` (evidence patch
  `.ess-evolution/waves/0005-aep-migration/wave-validate-v2-20260920/evidence/review-1-cases.patch`):
  after a governed move, `.engineering/planning/story/two.md` written by hand; `validate` answers
  `{"artifacts":1,"problems":[]}` and `verify` answers `"drift":"current"`, exit 0.
- Origin: pre-existing (reproduces by reading at effef5b1a: `open_plan` and `log_findings` untouched;
  `journal::reconcile` reports only artifacts the journal names).
- What reaches it: writing a `.md` file into `.engineering/planning/`, the habit every Markdown-plan
  repository carries into migration. Not an observed incident; the case constructs the file.

## Acceptance

- Red first: the reviewer's case above, adopted unchanged, fails on the base and passes on the correction.
- `plan store verify` reports the same foreign path; `validate` carries one finding in both `drift`
  and `problems` naming the path.
- Owned-file edit and owned-file deletion behaviour unchanged (existing cases in
  `tests/store_writer_control.rs` stay green); Markdown and Hybrid plans unchanged (`tests/drift.rs`).

## Scope

inferred: crates/plan/aep-planning-migration/src/projection.rs, crates/edge/aep-cli/src/store_command.rs,
crates/edge/aep-cli/tests/store_writer_control.rs.
