---
format: aep.planning-md/3
id: story:migration-writes-each-move-once
kind: story
status: implemented
title: Migration never writes a duplicate transition, and drops one already written
relations:
- serves: vision:O2
- informed_by: story:migrate-verify-compares-evidence
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T01:30:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T01:30:13Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-10T03:10:12Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Migration never writes a duplicate transition, and drops one already written

## Outcome

`aep plan store migrate git` writes each journalled move once, and a store already migrated with a
byte-identical duplicate transition can be repaired through `aep` rather than by hand.

## Context

Read against `main` at e4acc2916 (aep 0.71.0), 2026-10-10:

- `compute` in `crates/edge/aep-cli/src/store_command/migrate_git.rs` turns every `Moved` journal
  entry of an artifact into one transition (`transition_of`), with no check for a repeated entry.
  A journal carrying the same move line twice (as a Git merge of `journal.jsonl` can leave) yields
  two identical transitions.
- A consumer store migrated to `aep.project/5` got exactly that in `epic:acyclic-latest-upgrades`:
  `{from: "draft", to: "proposed", at: "2026-09-07T19:51:43Z", ..., revision: 4, imported: true}`
  twice. `aep plan artifact validate` then refuses "transition 2 moves from draft, walk stands at
  proposed" and "revision 4 not above 4", and no 0.71.0 verb (`move`, `set`, `body`,
  `store migrate`) can remove it.
- A scan of the 31 `aep.project/5` stores in the local workspace on 2026-10-10 found that one file
  and no other with a repeated transition line.
- Evidence records may legitimately repeat (`an_unmodified_migration_with_same_second_and_duplicate_records_verifies_clean`);
  only moves are in scope.

## Acceptance

1. A `/1` journal holding one move line twice for an artifact migrates to a document with that
   move once; `--dry-run` reports how many duplicate moves it dropped, and `--verify` passes.
2. A journal holding two moves that differ in any field keeps both (no dedup beyond byte-identical
   entries).
3. On an `aep.project/5` store, the repair path (`store migrate` or a named repair verb) removes a
   transition byte-identical to the one before it, writes nothing else, and `validate` is clean
   afterwards; on a store with none it writes nothing and says so.
4. A failing test built from the frontmatter quoted above comes first.
5. Spec first: the command, its outcomes and any new verb are declared in `ess/`, projections
   regenerated, `task ess-gate` green.

## Out of Scope

Non-identical transitions that disagree with the walk; evidence records.
