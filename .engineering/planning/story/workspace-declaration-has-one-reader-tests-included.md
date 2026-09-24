---
format: aep.planning-md/2
id: story:workspace-declaration-has-one-reader-tests-included
kind: story
status: draft
title: The workspace declaration has one reader, reachable from tests
relations:
- informed_by: story:migration-mapper-reads-the-declared-workspace
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/sqlite_plan.rs
revision: 3
---
## Outcome

The workspace declaration (`.engineering/workspace.yaml`) has one reader in the AEP workspace,
reachable from every place that needs it, tests included. No test carries a mirror of the reader.

## Why

Unit 8 of wave-validate-v2-20260920 (story:migration-mapper-reads-the-declared-workspace) fixed
two readers of one rule disagreeing (the artifact graph and the migration mapper). Its correction
round 2 (commit d10fcebc) had to leave a fourteen-line mirror of the declaration reader in
`crates/edge/aep-cli/tests/sqlite_plan.rs`, doc-commented as a mirror, because `declared_membership`
is `pub(crate)` in `aep-cli`'s `planning` module and making it reachable from integration tests
means making `planning` a public module of `aep-cli`, an API decision a correction round should
not take. The implementor disclosed it; the sub-operator did not block on it.

A mirror in a test is a second reader of the rule: when the reader changes, the test's copy does not,
and the test keeps passing against the old rule.

## Acceptance

- Decide where the reader lives so tests can call it: a public function in `aep-cli` (module or
  re-export), or the reader moved into `aep-domain` next to `Membership` where the predicate already
  is. The commit message says which and why.
- `tests/sqlite_plan.rs` calls the reader instead of mirroring it; the mirror is deleted.
- Red first: a case that breaks if the test's reader and the command's reader could diverge (for
  example, the same malformed `workspace.yaml` refused identically by both).
- No other behaviour changes; lane counts otherwise unchanged; 16-step gate green.
