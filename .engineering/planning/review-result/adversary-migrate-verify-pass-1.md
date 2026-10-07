---
format: aep.planning-md/3
id: review-result:adversary-migrate-verify-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: migrate git --verify compares evidence records'
relations:
- reviews: story:migrate-verify-compares-evidence
revision: 1
---
```
unit: story:migrate-verify-compares-evidence, uncommitted working tree of aep-u91 on base 65937c3c1
verdict: NEEDS-CHANGE
cases: executed 793→799, red 2
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 2 (harness background-task output files, part 6)
needs-coordinator: yes. Decide whether "in order" means journal order (no faithful migration of a backdated store can then pass) or the Git layout's order (fix below)
```

**1. `git --no-pager diff --stat`** (against HEAD; this includes the implementor's uncommitted work)
```
 crates/edge/aep-cli/src/store_command/migrate_git.rs | 538 ++++-
 crates/edge/aep-cli/tests/store_migrate_git.rs       | 172 +++
 crates/plan/aep-backend-markdown/src/journal.rs      | 211 ++++
```
My edits are only test code:
- `migrate_git.rs:1059-1229`, inside its `#[cfg(test)] mod tests` (the brief allows this).
- `tests/store_migrate_git.rs:422-519`.

I changed no implementation line.

**2. Cases added** (each red output was captured from that case run alone, before the suite)

| case | asserts | now |
|---|---|---|
| `tests/store_migrate_git.rs:427` `a_journal_with_backdated_evidence_migrates_and_verifies_clean` | a `/1` journal whose second evidence record was observed earlier still migrates and passes `--verify` | **red** |
| `migrate_git.rs:1084` `a_body_difference_names_the_field_never_the_body_text` | a body difference does not quote the body | **red** |
| `tests/store_migrate_git.rs:470` `records_of_different_kinds_at_one_instant_verify_clean_in_journal_order` | review, approval and test_result recorded in the same second keep journal order | green (pin) |
| `migrate_git.rs:1105` `a_change_to_any_one_field_of_a_record_is_reported_naming_that_field` | 7 single edits (actor, at, revision, source, reference removed, review added, review changed) are each reported as exactly one field with no value | green (pin) |
| `migrate_git.rs:1174` `an_extra_record_in_the_new_store_is_reported` | count line plus "record 6 (file) is in the new store only" | green (pin) |
| `migrate_git.rs:1204` `a_record_moved_to_another_artifacts_directory_is_reported_missing` | the count line names the lost kind and a record is reported missing | green (pin) |

Red output, verbatim:
```
  story:observed: evidence record 1 (story/observed/20260928T093000Z-000-50e3701b14d8.json) differs in at, change.source, revision
  story:observed: evidence record 2 (story/observed/20260928T100000Z-000-8bf5bbba74b4.json) differs in at, change.source, revision
verification failed: the new store differs from the old in 2 place(s); `git checkout -- … && git clean -fd -- …` restores the old store
EXIT=101
```
```
a difference printed the value "confidential-body-text-4242": story:observed: body was "\n# observed\n" and is "\n# observed\n\nconfidential-body-text-4242\n"
EXIT=101
```

**3. Suite**
- `cargo test -p aep-cli -p aep-backend-markdown --no-fail-fast`: 797 passed, 2 failed (exactly the two red cases above), exit 101.
- The same run with `--skip` on my 6 cases: 793 passed, exit 0.
- `cargo clippy -p aep-cli --all-targets -- -D warnings`: exit 0.
- `cargo fmt -p aep-cli -p aep-backend-markdown -- --check`: exit 0.
- After the fmt fix, I reran `--test store_migrate_git`: 11 passed, 1 failed (the backdated case, same reason).

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `migrate_git.rs:445` | NEEDS-CHANGE | introduced | Old records are kept in journal order, but the new store answers them sorted by `at` (`journal.rs:508`). The comparison at `migrate_git.rs:760` is position by position, so a faithful migration fails `--verify` and the operator is told to restore the old store. Red at `tests/store_migrate_git.rs:459`. | Real stores. The `/1` reader `history` (`git show 7ec47c09d^`) answered append order with no sort. `evidence --at` existed in the `/1` era (9739786c9). The `/1` store in `beyond10x/mandate` has 12 out-of-order pairs across 11 artifacts and 0 out-of-order moves, so it passes `--verify` at base and fails now. All 20 local `/1` journals use second-precision `at`; backdating is the only trigger found. |
| `migrate_git.rs:702` | CONFIRMED | pre-existing | The body difference prints the full body; the transitions difference (`:703-707`) prints `Transition` Debug, which includes actor, executor and correlation. The acceptance says "no reference, source or body text in the message". Red at `migrate_git.rs:1099`. | Only a body or transition that does not read back after the write. Whether this bullet covers document fields or only evidence is a scope question for the coordinator. |
| `migrate_git.rs:760` | CONFIRMED | introduced | Comparing by position misattributes one lost record. Moving record 3 out printed "record 3 (the intact record 4's file) differs in at, change.kind, change.outcome, …" and "record 5 of 5 is missing". A reorder (finding 1) likewise reads as two corrupted records. | Any dropped or inserted middle record. Diagnostic quality only; the failure is still reported. |

Fix to name for finding 1, not applied: sort `old.records` stably by `(at, artifact, revision)` before comparing. That is the order the Git layout answers. Within one second it keeps journal order, as the file sequence does, so the same-second and duplicate acceptance cases still hold.

**5. Attacked and could not break**
- **Values in messages:** the per-kind count line, the "missing" and "only" lines and `differing_fields` print no values. Field paths come from fixed serde keys.
- **Field escapes:** none found. Edits to actor, at, revision, kind, source, a removed reference and an added or changed review are all caught.
- **Immutable-record checks:** not weakened. The 7 deleted lines are a message, the `differ("evidence")` call and a `use`; `write_evidence_occurrence` is unchanged.
- **Same instant, different kinds:** order is kept, because the per-second sequence is per artifact directory.
- **The `{:03}` sequence:** it misorders at 1000 records in one second. The real maximum seen is 8, so it is out of reach.
- **Other escapes:** an evidence file for an artifact with no document is not checked (pre-existing). The migration's write never produces one.

**6. Paths written outside the worktree**
- `<harness task output> bk33nhddj.output`
- `<harness task output> b2shfrq33.output`

Scratch stayed under `<worktree>/target/scratch/adversary/`. It holds logs, this repository's pre-`/3` journal and a base source copy. I read the `mandate` journal with `jq` to stdout only.

Deviations:
- One cargo run (the moved-record observation) was piped into `tail`, which the invariants forbid. Its exit status was not used.
- The moved-record pin carries an `eprintln!` of its differences, left in so the tree matches what I measured.

```findings
- file: crates/edge/aep-cli/src/store_command/migrate_git.rs
  line: 445
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: old evidence records are compared in journal append order against the new store's at-sorted order, so an unmodified migration of a /1 store with backdated evidence (11 artifacts in the local mandate store) fails --verify and is told to restore the old store
- file: crates/edge/aep-cli/src/store_command/migrate_git.rs
  line: 702
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the body and transitions differences still print full values (body text, transition actors), against the acceptance bullet that a difference never prints body text
- file: crates/edge/aep-cli/src/store_command/migrate_git.rs
  line: 760
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: positional comparison reports one lost middle record as intact records differing plus the last position missing, naming the wrong position and file
```
