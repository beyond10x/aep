---
format: aep.planning-md/1
id: review-result:mapper-workspace-review-1
kind: review-result
status: active
title: 'Independent verification pass 1: does the crossing survive the migration as a record'
relations:
- reviews: story:migration-mapper-reads-the-declared-workspace
revision: 2
---
# Unit 8 — story:migration-mapper-reads-the-declared-workspace — independent verification, pass 1

```
unit: story:migration-mapper-reads-the-declared-workspace — worktree ess-evolution-mapper-workspace-review-1-20260921 at d6268b91d2d3478b7d38a0077980d2470d17bc2d (base 763d195be)
verdict: red
cases: executed 643→655, red 4
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 11 paths, all under ~/.cache/ess-wave-v2/u8r1/ — enumerated in part 6
needs-coordinator: yes — finding 1 is an acceptance clause that may not be implementable as written; the choice between correcting the code and correcting the story is not mine
```

---

## 1. `git --no-pager diff --stat` — proof of what I touched

```console
$ git --no-pager diff --stat
                                                   # empty: no tracked file modified

$ git status --porcelain
?? .engineering/.aep-planning-writer-27c29ce0d8bc725db7a26f64933e9cc539c37427ee5f1f98087222fde950fcea.lock
?? .engineering/.aep-planning-writer.lock
?? crates/edge/aep-cli/tests/migration_inventory_matches_what_is_imported.rs
?? crates/edge/aep-cli/tests/migration_reads_one_workspace_declaration.rs
?? crates/edge/aep-cli/tests/migration_refusal_coordinate_halves.rs
?? crates/edge/aep-cli/tests/migration_unreadable_declaration_is_not_an_empty_one.rs
?? crates/plan/aep-planning-migration/tests/workspace_crossing_records.rs
```

Five paths, all under a `tests/` directory. No implementation file, manifest, lockfile or document
was edited; no git write command was run.

**I am naming the two non-test paths myself.** The two zero-byte `.aep-planning-writer*.lock` files
are writer-fence artifacts created at 17:37 by the repository's own `aep-cli` suite while I ran it
(`planning_writer_fence.rs:16,104`); the worktree was clean when I took the lease. They are the same
pair the implementation report flagged in its own tree. I did not author them, they are zero bytes,
and they are left in place rather than deleted.

---

## 2. The cases I added

Written before anything was run, each run alone before the suite. Output verbatim.

| file | case | now |
|---|---|---|
| `crates/plan/aep-planning-migration/tests/workspace_crossing_records.rs` | `a_relation_to_a_local_artifact_is_migrated_as_a_relation_record` | green (control) |
| | `a_declared_crossing_is_migrated_as_a_relation_record` | **red** |
| | `a_crossing_and_a_local_edge_do_not_migrate_to_the_same_number_of_relation_records` | **red** |
| `crates/edge/aep-cli/tests/migration_refusal_coordinate_halves.rs` | `a_mapper_refusal_reaches_the_receipt_as_a_source_coordinate` | green |
| | `a_selector_failure_is_still_reported_at_the_selector_coordinate` | green |
| | `both_coordinate_halves_survive_the_text_renderer` | green |
| `crates/edge/aep-cli/tests/migration_reads_one_workspace_declaration.rs` | `a_declaration_beside_the_store_admits_the_crossing` (M1) | green |
| | `a_declaration_one_level_too_deep_is_not_this_store_s_declaration` (M4) | green |
| | `an_undeclared_or_misspelled_member_is_still_a_dangling_edge` (the bound) | green |
| | `the_declaration_is_found_from_an_absolute_selector_run_from_elsewhere` | green |
| `crates/edge/aep-cli/tests/migration_inventory_matches_what_is_imported.rs` | `the_dry_run_inventory_is_the_number_of_relations_the_migration_imports` | **red** |
| `crates/edge/aep-cli/tests/migration_unreadable_declaration_is_not_an_empty_one.rs` | `an_unparseable_declaration_is_distinguishable_from_no_declaration` | **red** |

### (a) The crossing does **not** survive as a relation record — red

The story's `## Acceptance`, verbatim:

> the cross-member relation **survives migration as a relation record** and is read back
> identically on the Eventlog arm (history equality on the artifact that carries it).

`mapping.rs:323-327` — unchanged text, newly reachable:

```rust
for (ordinal, declared) in document.frontmatter.relations.iter().enumerate() {
    let Some(target) = identities.get(declared.target.id()) else {
        // Workspace crossings have no destination entity in this authority. The exact
        // frontmatter relation remains in the entity body and is restored by projection.
        continue;
    };
```

`identities` holds one entry per *captured document*. A workspace crossing points outside the
store by definition, so it never has an entry and is always skipped. `markdown_boundaries_raw`'s
return value is exactly `ApplyInputs.histories` (`store_command.rs:311→2041→2054`, then
`apply_with_control`'s `ApplyInputs { …, histories }`), so what it does not produce is what the
destination authority does not receive.

```console
$ TMPDIR=~/.cache/ess-wave-v2/u8r1/tmp CARGO_NET_OFFLINE=true \
  cargo test -p aep-planning-migration --test workspace_crossing_records
   Compiling aep-planning-migration v0.55.0 (…/ess-evolution-mapper-workspace-review-1-20260921/crates/plan/aep-planning-migration)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 16.40s
     Running tests/workspace_crossing_records.rs (target/debug/deps/workspace_crossing_records-ebf83032ab56f86c)

running 3 tests
test a_declared_crossing_is_migrated_as_a_relation_record ... FAILED
test a_relation_to_a_local_artifact_is_migrated_as_a_relation_record ... ok
test a_crossing_and_a_local_edge_do_not_migrate_to_the_same_number_of_relation_records ... FAILED

failures:

---- a_declared_crossing_is_migrated_as_a_relation_record stdout ----

thread 'a_declared_crossing_is_migrated_as_a_relation_record' (1780520) panicked at crates/plan/aep-planning-migration/tests/workspace_crossing_records.rs:111:5:
assertion `left == right` failed: the cross-member relation survives migration as a relation record; subjects produced: [Subject { entity: "aep.entity", id: "MIG881606f8fb114c1d519724ea53dc0fba23bc280551381b1412496c3a94892062" }, Subject { entity: "aep.entity", id: "MIGbbf44d14d491f11b066306066fa9431a11365ef0d1eea0f3677c278d5ea16079" }]
  left: 0
 right: 1

---- a_crossing_and_a_local_edge_do_not_migrate_to_the_same_number_of_relation_records stdout ----

thread 'a_crossing_and_a_local_edge_do_not_migrate_to_the_same_number_of_relation_records' (1780519) panicked at crates/plan/aep-planning-migration/tests/workspace_crossing_records.rs:133:5:
assertion `left == right` failed: declaring a member changes which edges are checkable, not which edges are recorded
  left: 0
 right: 1

failures:
    a_crossing_and_a_local_edge_do_not_migrate_to_the_same_number_of_relation_records
    a_declared_crossing_is_migrated_as_a_relation_record

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-planning-migration --test workspace_crossing_records`
EXIT=101
```

The control in the same file — the identical fixture with a **local** target — produces one
`aep.relation` subject and passes, so nothing about the fixture, the accessor or the assertion is
what decides the crossing case.

**Why the unit's own case did not catch this.** `a_relation_crossing_into_a_declared_member_migrates_and_reads_back_unchanged`
(`store_command.rs:2947`) asserts the *document* reads back with the crossing, and it genuinely
does — but not because the relation was migrated. `instance_of` (`provider.rs:332`) copies the whole
authored `relations` list into the subject entity's body, and `document_from_entity`
(`projection.rs:732-745`, unchanged at base) merges that retained copy with the authority's real
relation rows. The document round-trips through a body field; the authority's relation surface does
not carry the edge. `inventory.relations 1` in the same case is counted from the source's
frontmatter (`store_command.rs:1941`), not from the histories, so it measures the source and not
the migration.

### (a, second half) The dry-run receipt promises a relation the migration does not import — red

This is the operator-facing face of the same defect and the reason it matters during a cutover: the
receipt's `inventory.relations` is the number a cutover records as the thing that must come out the
other side, and nothing in the command compares it with what is imported.

```console
$ TMPDIR=… cargo test -p aep-cli --test migration_inventory_matches_what_is_imported
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.48s
     Running tests/migration_inventory_matches_what_is_imported.rs (target/debug/deps/migration_inventory_matches_what_is_imported-97f840c01beb469c)

running 1 test
test the_dry_run_inventory_is_the_number_of_relations_the_migration_imports ... FAILED

failures:

---- the_dry_run_inventory_is_the_number_of_relations_the_migration_imports stdout ----

thread 'the_dry_run_inventory_is_the_number_of_relations_the_migration_imports' (1801513) panicked at crates/edge/aep-cli/tests/migration_inventory_matches_what_is_imported.rs:104:5:
assertion `left == right` failed: the dry-run receipt promises 1 relation(s) and the migration imports 0
  left: 0
 right: 1

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

The fixture is driven through the built `aep` binary — `plan store migrate dry-run --format json`,
admitted — and the same capture is then mapped in process. Applied to the real AEP store, whose
brief records exactly one crossing, the receipt will say `relations 564` and the authority will
receive 563 relation records.

**Origin, settled by running.** `git archive 763d195be` into scratch, base-shaped copy of the same
fixture, `cargo test -p aep-planning-migration --test workspace_crossing_records_at_base`:

```console
running 2 tests
test at_base_a_crossing_is_refused_before_any_relation_is_mapped ... ok
test at_base_a_local_edge_is_a_relation_record ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

At base a local edge is a relation record and a crossing is refused before any relation is mapped:
the `continue` exists at base byte for byte (`git show 763d195be:…/mapping.rs` line 251) and nothing
reached it. The change under review is what reaches it — **introduced**.

### (b) The refusal coordinate, both halves — green

```console
$ TMPDIR=… cargo test -p aep-cli --test migration_refusal_coordinate_halves
     Running tests/migration_refusal_coordinate_halves.rs (target/debug/deps/migration_refusal_coordinate_halves-d36eca3409667d1a)

running 3 tests
test a_selector_failure_is_still_reported_at_the_selector_coordinate ... ok
test a_mapper_refusal_reaches_the_receipt_as_a_source_coordinate ... ok
test both_coordinate_halves_survive_the_text_renderer ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Both halves hold, measured through the binary in `--format json` and `--format text`. The converse
half — a genuine selector failure still reported at the selector, carrying the operator's own
`--project` path — was **not measured anywhere in the repository** before these cases: at `d6268b91`
`DiagnosticCoordinateV1::Selector` matches exactly one line in the whole tree, the constructor at
`store_command.rs:4037`, and no test. A change routing every refusal through a source coordinate
would have left the suite green. That gap is finding 5; the behaviour itself is correct.

### (c) Stores without `workspace.yaml` are unchanged — green

sha256 verified against the `.sha256` file beside each archive **before** extracting:

```console
$ sha256sum -c pre-cutover-engineering-20260921.tar.sha256
…/er-activation-20260921/pre-cutover-engineering-20260921.tar: OK
…/eventlog-activation-20260921/pre-cutover-engineering-20260921.tar: OK
```

Neither archive contains a `workspace.yaml` (`.engineering/` holds only `planning/` and
`project.yaml` in both). Dry-run with the **new** binary, `target/debug/aep` built at `d6268b91`:

| archive | `outcome.kind` | `inventory.subjects` | `inventory.relations` | exit |
|---|---|---|---|---|
| `er-activation-20260921` | `admitted` | **104** | 163 | 0 |
| `eventlog-activation-20260921` | `admitted` | **75** | 95 | 0 |

104 and 75 are the counts the brief requires. 104 + 163 = 267 and 75 + 95 = 170, which are the
history counts the brief's controls record — and which incidentally confirm that in a store with no
crossings every relation becomes a relation record. Neither extraction gained a `.engineering/`
entry from the dry-run.

### (d) The six mutations

I may not edit the source under review, and a second build tree for a mutated copy was not a cost
worth paying against a disk that was at 22 GiB mid-pass. Instead each mutation's *detection
condition* is expressed as a black-box case that knows nothing of the unit's own fixtures:

| # | re-verified how | result |
|---|---|---|
| M1 `graph_in_workspace(members)` → `empty()` | `a_declaration_beside_the_store_admits_the_crossing` — a declaration beside the store admits the crossing, through the binary | held |
| M2 `strip_prefix("artifacts.")` → `"artifact."` | `a_mapper_refusal_reaches_the_receipt_as_a_source_coordinate` — asserts `markdown_path`, which under M2 degrades to the Markdown root | held |
| M3 `if raw.nodes.iter().any(…)` → `if true` | **not re-verified black-box.** Reaching a synthetic mapper coordinate through the CLI needs a graph defect whose location resolves to no captured document, which I could not construct from outside. The unit's in-process case stands unchallenged. |
| M4 `declared_members(engineering.parent())` → `declared_members(&engineering)` | `a_declaration_one_level_too_deep_is_not_this_store_s_declaration` — a declaration at `.engineering/.engineering/workspace.yaml` is refused | held |
| M5 `dry_refusal_at(*refusal)` → `dry_refusal(code)` | `a_mapper_refusal_reaches_the_receipt_as_a_source_coordinate` | held |
| M6 hybrid root `Local` → `Replica` | **not re-verified.** In-process only; no hybrid store is reachable from a black-box fixture without a SQLite replica I would have had to build. |

Plus the story's own bound, independently: `an_undeclared_or_misspelled_member_is_still_a_dangling_edge`
refuses both an absent declaration and a declaration naming `otherr`.

### (e) One reader of the rule — green, with one note

Every reader of `workspace.yaml` in the tree:

| site | verdict |
|---|---|
| `aep-project/src/project.rs:336` | the definition |
| `aep-cli/src/planning.rs:2214` (`declared_members`) | the one reader on the read path and, now, on the migration path |
| `aep-cli/src/store_command.rs:1710` (`Resolved::declared_members`) | calls `crate::planning::declared_members` — the same function, not a copy |
| `aep-cli/src/workspace.rs:124, 400` | the `aep plan workspace` command's own readers, not the migration path |
| `aep-cli/src/store_command.rs:2903`, `aep-cli/tests/sqlite_plan.rs:32` | **test** helpers that re-implement the read longhand — see finding 4 |

No third view was added. `the_declaration_is_found_from_an_absolute_selector_run_from_elsewhere`
pins the derivation the migration uses (selector path, not the working directory), which differs by
construction from the read commands' `StoreLocation::repository_root` (`planning.rs:143`) and is the
one place the two rules could drift again.

### An invariant-5 boundary the declaration reader crosses — red

```console
$ TMPDIR=… cargo test -p aep-cli --test migration_unreadable_declaration_is_not_an_empty_one
     Running tests/migration_unreadable_declaration_is_not_an_empty_one.rs (target/debug/deps/migration_unreadable_declaration_is_not_an_empty_one-40bb27f72e799086)

running 1 test
test an_unparseable_declaration_is_distinguishable_from_no_declaration ... FAILED

---- an_unparseable_declaration_is_distinguishable_from_no_declaration stdout ----

thread 'an_unparseable_declaration_is_distinguishable_from_no_declaration' (1829518) panicked at crates/edge/aep-cli/tests/migration_unreadable_declaration_is_not_an_empty_one.rs:95:5:
assertion `left != right` failed: a `workspace.yaml` that does not parse is unknown, not a declaration of nothing; both stores refused with [{"at":{"kind":"source","value":{"coordinate":{"kind":"markdown_path","value":{"relative":{"kind":"unix","value":"hex:73746f72792f63726f7373696e672e6d64"}}}}},"code":"semantic_mismatch"}]
  left: Array [Object {…"hex:73746f72792f63726f7373696e672e6d64"…}]
 right: Array [Object {…"hex:73746f72792f63726f7373696e672e6d64"…}]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`declared_members` is `load_workspace(root).map_or_else(|_| Vec::new(), …)` — a `workspace.yaml`
that exists and does not parse becomes *this store declares no members*, which `AGENTS.md`
invariant 5 names: "Unknown differs from false … not rewritten as a contradiction." The two
receipts above are byte-identical, and both name `story/crossing.md` — the document that is not
wrong. That is the same failure this unit exists to remove ("the refusal named neither the document
nor the reason"), reproduced one level up. `hex:73746f…` decodes to `story/crossing.md`.

---

## 3. The suite, after the cases existed

```console
$ TMPDIR=~/.cache/ess-wave-v2/u8r1/tmp CARGO_NET_OFFLINE=true \
  cargo test --no-fail-fast -p aep-planning-migration -p aep-cli
…
     Running unittests src/lib.rs (target/debug/deps/aep_planning_migration-c4fba35e88409efc)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.14s
     Running tests/workspace_crossing_records.rs (…)
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/aep_cli-f243fdb053c0084e)
test result: ok. 172 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.46s
     Running tests/migration_inventory_matches_what_is_imported.rs (…)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/migration_reads_one_workspace_declaration.rs (…)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/migration_refusal_coordinate_halves.rs (…)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/migration_unreadable_declaration_is_not_an_empty_one.rs (…)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
…
EXIT=101
```

Summed across every `test result` line: **651 passed, 4 failed, 0 ignored — 655 executed.**

`<before>` is **643**, the implementing state's own `cases: executed 635→643` after-figure for the
same two package lanes. I did not run the suite before my cases existed. 643 + 12 added = 655, and
the two lanes that carry the change are unmoved at 32 and 172, so no pre-existing case regressed and
none was deleted, skipped or weakened. A red suite here is the successful outcome.

---

## 4. Findings

Covering worktree `ess-evolution-mapper-workspace-review-1-20260921` at commit
`d6268b91d2d3478b7d38a0077980d2470d17bc2d`, base `763d195be`.

### 1 — the cross-member relation is not migrated as a relation record — `NEEDS-CHANGE` / `introduced` / blocker

- **what was measured** — `crates/plan/aep-planning-migration/tests/workspace_crossing_records.rs:111`, exit 101: a declared crossing produces **0** `aep.relation` subjects where the identical fixture with a local target produces 1. `mapping.rs:324`.
- **what reaches it** — the AEP planning store's own cutover, the last of the six, on the default `aep plan store migrate dry-run` / `apply` path with no flag: `story:assemble-across-sources` `- informed_by: entity-runtime/story:typed-references`, the one document this whole unit exists for. This is the case area (a) of my brief named in advance: *"a fix that admits the store but silently drops the cross-member relation would pass every count in the report and be worse than the refusal it replaced."*
- **the correction, named not applied** — two mutually exclusive routes, and choosing between them is the coordinator's:
  1. record the crossing on the authority's relation surface (an `aep.relation` whose target is an unresolved external reference), so the migrated authority's relation graph is complete; or
  2. amend the story's `## Acceptance` clause to state what the implementation actually guarantees — the crossing is retained in the subject entity's body (`provider.rs:332`) and restored by `document_from_entity`'s merge (`projection.rs:732-745`) — and, either way, fix finding 2.
  Route 1 may not be implementable as the wire vocabulary stands; route 2 is a document change. **This is why `needs-coordinator` is yes.** What is not in question is that the code and the acceptance statement disagree today.

### 2 — the dry-run receipt promises relations the migration does not import — `CONFIRMED` / `introduced` / warning

- **what was measured** — `crates/edge/aep-cli/tests/migration_inventory_matches_what_is_imported.rs:104`, exit 101: the binary's own admitted receipt reports `inventory.relations 1`; the same capture maps to 0 relation records. `store_command.rs:1941`.
- **what reaches it** — every cutover of a store that declares members and carries a crossing. The AEP store's dry-run will advertise 564 and deliver 563 relation records, and the post-migration read agrees with 564 only because `report_from_backend` re-merges the retained body copy — so no count anyone takes will show the difference.
- **severity, stated plainly** — `warning` rather than `blocker` only because route 1 of finding 1 would dissolve it. If the coordinator takes route 2, this becomes a standalone live defect and should be re-rated.

### 3 — an unparseable `workspace.yaml` is read as a declaration of nothing — `CONFIRMED` / `introduced` / warning

- **what was measured** — `crates/edge/aep-cli/tests/migration_unreadable_declaration_is_not_an_empty_one.rs:95`, exit 101: a store with a mistyped `workspace.yaml` and a store with none produce byte-identical refusals, both naming `story/crossing.md`. `planning.rs:2214`.
- **what reaches it** — a hand-edited `workspace.yaml` on any store being migrated. The AEP store declares three members; one typo silently disables all three, refuses the migration, and points the operator at an artifact document that is correct.
- **origin, honestly** — the `map_or_else(|_| Vec::new(), …)` swallow at `planning.rs:2214` is unchanged from base and very likely affects the ordinary read commands too. I did **not** measure it at base on the read path (that needed a second `aep-cli` build), so I am not calling it `pre-existing` on a guess; the migration path it now sits on is new, which is `introduced`.
- **the correction, named not applied** — distinguish `Ok(None)` (no file) from `Err` (a file that did not parse), and refuse the migration at the `workspace.yaml` coordinate rather than continuing with an empty member list.

### 4 — the unit's own case re-implements the reader it is testing — `CONFIRMED` / `introduced` / note

- **what was measured** — read, not run: `store_command.rs:2902` `fixture_members` writes out `load_workspace(...).map_or_else(...)` longhand instead of calling `crate::planning::declared_members`, the `pub(crate)` function the code under test calls and the whole point of the fix.
- **what reaches it** — nothing at runtime; it is a measurement gap. The unit's thesis is *one reader of the rule*, and its own case holds a second one, so a change in how `declared_members` reads the declaration cannot fail that case. My four CLI cases in `migration_reads_one_workspace_declaration.rs` close it from outside.

### 5 — the selector half of the coordinate contract was unmeasured — `CONFIRMED` / `introduced` / note

- **what was measured** — read, then closed: at `d6268b91`, `DiagnosticCoordinateV1::Selector` matches one line in the entire tree, its constructor at `store_command.rs:4037`. The story requires "the selector coordinate is used only for selector failures" and nothing asserted it.
- **what reaches it** — nothing today; the behaviour is correct and my three green cases now say so. Recorded because the unit rewrote every refusal-routing site in `store_command.rs` and the suite would not have caught the converse going wrong.

---

## 5. Reviewed and could not fault

- The admission half of the fix: `graph_in_workspace(members)` at `mapping.rs:253`, the members read once at the edge, and the whole `StoreReport::graph()` caller enumeration in the implementation report — re-checked against `rg`, every remaining caller is a `#[cfg(test)]` body or `PlanSource::graph`, which already passes the declaration.
- The bound: an undeclared or misspelled member is still a dangling edge and still refused, at the mapper and through the binary.
- `graph_coordinate`'s longest-match document resolution — the ambiguous cases (an id that is a prefix of another, an id containing `.`, a location that is exactly `artifacts.<id>`, several errors of which only the second resolves) all resolve correctly by reading; none needed a case.
- The claim that `members` is admission input only and cannot change a migrated byte: verified by reading — the relation loop keys on `identities`, never on `members`. (This is also exactly why finding 1 is not a small slip: declaring a member can never, by construction, turn a crossing into a record.)
- `verify`, `rebuild`, `inspect` and `projection_current` build no artifact graph and take no workspace, so the post-apply half of a cutover is untouched by the change.
- The two regression controls, run through the new binary: 104 and 75 subjects, admitted, exit 0, no `.engineering/` state written.
- The CHANGELOG entry: accurate. It says the crossing "survives migration as the artifact's own relation and is read back from the Eventlog authority unchanged", which is true of the document and carefully does not claim a relation record. The story's `## Acceptance` does claim one; that is finding 1.
- No `rev` moved, no contract or wire change, no schema regeneration.

---

## 6. Every path written outside the worktree

All under `~/.cache/ess-wave-v2/u8r1/`, the assigned scratch. Nothing under `/tmp`;
`TMPDIR` was exported to `…/u8r1/tmp` (38 bytes) for every cargo and binary invocation;
`CARGO_TARGET_DIR` was never set; `CARGO_NET_OFFLINE=true` throughout.

| path | what it is | still there |
|---|---|---|
| `~/.cache/ess-wave-v2/u8r1/tmp/` | `TMPDIR` for every cargo run and every `aep` invocation | yes |
| `~/.cache/ess-wave-v2/u8r1/build.log` | the initial `--no-run` compile | yes |
| `~/.cache/ess-wave-v2/u8r1/case-a.log` | area (a), the case run alone | yes |
| `~/.cache/ess-wave-v2/u8r1/case-a2.log` | the inventory case run alone | yes |
| `~/.cache/ess-wave-v2/u8r1/case-b.log` | area (b), run alone | yes |
| `~/.cache/ess-wave-v2/u8r1/case-e.log` | areas (d)/(e), run alone | yes |
| `~/.cache/ess-wave-v2/u8r1/case-int.log` | the invariant-5 case, run alone | yes |
| `~/.cache/ess-wave-v2/u8r1/base-origin.log` | the origin run at `763d195be` | yes |
| `~/.cache/ess-wave-v2/u8r1/er-dry.txt`, `el-dry.txt` | the two control-archive dry-runs | yes |
| `~/.cache/ess-wave-v2/u8r1/suite.log`, `suite2.log` | the suite runs (`suite.log` stopped at the first failing target; `suite2.log` is the `--no-fail-fast` run quoted in part 3) | yes |
| `~/.cache/ess-wave-v2/u8r1/base/` | the `git archive 763d195be` extraction and its 1.2 GB build directory, including `crates/plan/aep-planning-migration/tests/workspace_crossing_records_at_base.rs` | **deleted** after the origin run |
| `~/.cache/ess-wave-v2/u8r1/ctl/` | the two verified control-archive extractions | **deleted** after the dry-runs |

Scratch is 17 MB at hand-back. Disk was 29 GiB free at the start, reached a low of 22 GiB, and is
36 GiB now; it never approached the 20 GiB stop.

No `plan store migrate apply` was run, on any store, anywhere. No store outside my own scratch was
written. No other tree was touched. The lease `ess-wave-v2-u8-review-1` was taken with
`worktree hook session-start`, heartbeated, and released with `worktree hook session-end` before
this report was handed back.

---

```findings
- file: crates/plan/aep-planning-migration/src/mapping.rs
  line: 324
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the story's acceptance requires the cross-member relation to survive migration as a relation record, and the mapper skips every relation whose target is not a captured document, so a declared crossing is migrated as zero relation records while the identical local edge is migrated as one.
- file: crates/edge/aep-cli/src/store_command.rs
  line: 1941
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the dry-run receipt counts inventory.relations from the source's frontmatter while the migration imports only the relations the mapper produced, so an admitted store that declares members advertises one relation per crossing more than it delivers and nothing compares the two.
- file: crates/edge/aep-cli/src/planning.rs
  line: 2214
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: declared_members turns every workspace.yaml read failure into an empty member list, so a store whose declaration exists and does not parse produces a refusal byte-identical to one with no declaration at all, naming an artifact document instead of the file that is wrong.
- file: crates/edge/aep-cli/src/store_command.rs
  line: 2902
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's own case builds its expected member list with a hand-written second copy of declared_members rather than calling it, so the case cannot fail when the one reader the fix exists to establish changes how it reads.
- file: crates/edge/aep-cli/src/store_command.rs
  line: 4037
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: nothing in the repository asserted that a genuine selector failure is still reported at the selector coordinate, so a change routing every refusal through a source coordinate would have left the suite green; the behaviour is correct and is now measured.
```
