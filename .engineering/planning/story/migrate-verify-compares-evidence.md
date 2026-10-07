---
format: aep.planning-md/3
id: story:migrate-verify-compares-evidence
kind: story
status: active
title: migrate git --verify compares evidence records, not counts
refs:
- provider: github
  reference: beyond10x/aep#91
relations:
- serves: vision:O2
- informed_by: epic:planning-on-entity-runtime
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/store_command/migrate_git.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_migrate_git.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/journal.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:06:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-07T08:06:12Z", actor: "human:timo", revision: 5}
---
## Outcome

`aep plan store migrate git --verify` compares every migrated evidence record with the record the
old store answered, field by field and in order, not only how many records of each kind exist. A
migration that changes one evidence field while keeping the counts is reported as a difference.

## Acceptance

- In a migration fixture, altering only one evidence record's `reference` (and, in a second case,
  only a review outcome) after the write makes `--verify` report a difference naming the artifact,
  the record's position and the field. Each case first fails on the current verifier.
- An unmodified migration verifies clean (positive control).
- Two distinct observations recorded in the same second, and two byte-identical duplicate
  observations, verify clean and keep their order and multiplicity; dropping one duplicate is
  reported.
- A difference names the field, never its value (no reference, source or body text in the message).
- Per-kind counts stay in the report as a diagnostic.
- The immutable-record checks are not weakened.

## Origin

https://github.com/beyond10x/aep/issues/91, a source-confirmed verification gap at aep 0.68.0.

## Source

- `crates/edge/aep-cli/src/store_command/migrate_git.rs:161` `Answered.evidence` is
  `BTreeMap<EvidenceKind, usize>`.
- `crates/edge/aep-cli/src/store_command/migrate_git.rs:661` `verify` rebuilds it with
  `journal::evidence_on_hand_git`, again a count per kind.
- `crates/edge/aep-cli/src/store_command/migrate_git.rs:672` `differ` prints both values in full,
  which for evidence would print references.

## Specification

No ESS specification exists in this repository. This fix changes no noun, command, outcome or
configuration: `--verify` keeps its outcomes (clean, or a list of differences) and compares more of
what it already reads.

## Scope

- cited: `crates/edge/aep-cli/src/store_command/migrate_git.rs`
- cited: `crates/edge/aep-cli/tests/store_migrate_git.rs`
- inferred: `crates/plan/aep-backend-markdown/src/journal.rs`

## Found during the fix

Recorded 2026-10-07 from the implementation and two adversary passes; not fixed in this story.

- An evidence file for an artifact with no document is not checked by `--verify` (pre-existing).
  The migration's writer never produces one.
- The per-second evidence sequence is three digits (`{:03}`), so more than 999 records in one second
  for one artifact would misorder. The largest seen in 20 real `/1` journals is 8.
- A document that fails to read back is still reported with the store loader's detail text, which
  can quote content.
- Transitions are compared in journal order, not re-sorted. 0 of 1063 moves in 20 real `/1`
  journals are out of order, so no faithful migration fails on it.
