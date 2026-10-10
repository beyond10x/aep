---
format: aep.planning-md/3
id: decision-blocker:approve-wave-suites-dups
kind: decision-blocker
status: cleared
title: Approve the ordinary-suites and duplicate-transitions wave
relations:
- blocks: story:evidence-admits-ordinary-ess-suites
- blocks: story:migration-writes-each-move-once
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-10T01:30:07Z", actor: "human:timo", revision: 3}
---
# Decision: approve the ordinary-suites and duplicate-transitions wave

## Question

Approve one wave of two units, one integration branch `wave/suites-dups` and one pull request,
then a patch release?

- U1 `story:evidence-admits-ordinary-ess-suites`: spec first in a new `ess/domains/evidence.yaml`
  declaring only the `--from <report/2> --suite <suite>` admission outcomes (recorded, digest
  mismatch, version mismatch, unknown ordinary major); then an ordinary-suite route for even majors
  from /6 on in `crates/edge/aep-cli/src/planning.rs`, `aep-ess-evidence` and `aep-domain`; fixture
  pair written by ess 0.57.0.
- U2 `story:migration-writes-each-move-once`: spec first in `ess/domains/plan.yaml` (`MigrateGit`
  outcomes); `crates/edge/aep-cli/src/store_command/migrate_git.rs` drops byte-identical repeated
  moves, and the repair path for an already migrated store.

## Options for the repair path (U2)

- A: `aep plan store migrate git` on an `aep.project/5` store repairs byte-identical repeated
  transitions instead of answering nothing to migrate; one new outcome on a declared command.
- B: a new verb `aep plan store repair`; a new command in the spec and the CLI reference.
- C: no repair verb; only the migration is fixed, already migrated stores keep editing by hand.

Recommendation: A.

## Decision

Approved as proposed, 2026-10-10: both units, branch `wave/suites-dups`, one pull request, patch
release after the merge on green CI. Repair path: option A, `store migrate git` on an
`aep.project/5` store drops byte-identical repeated transitions as one new outcome of that command;
no new verb. Units build one at a time.
