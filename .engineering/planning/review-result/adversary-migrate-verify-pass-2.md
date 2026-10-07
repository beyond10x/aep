---
format: aep.planning-md/3
id: review-result:adversary-migrate-verify-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: migrate git --verify compares evidence records'
relations:
- reviews: story:migrate-verify-compares-evidence
revision: 1
---
```
unit: story:migrate-verify-compares-evidence, commit 9583f0f09 plus my uncommitted test additions in aep-u91
verdict: blocked. Disk read 14G before clippy (under the 15G floor); the findings are complete, only clippy on my 2 added cases is not run
cases: executed 802→804, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 (harness background-task output files, part 6)
needs-coordinator: yes. Run clippy on my 2 added cases once disk is back above 15G; decide whether the fallback red is fixed in this unit or dropped as out of reach
```

**1. `git --no-pager diff --stat`** (against 9583f0f09)
```
 crates/edge/aep-cli/src/store_command/migrate_git.rs | 75 +++++
```
The 75 lines are 2 test functions in its `#[cfg(test)] mod tests`, at `:1441` and `:1481`. I changed no implementation line.

**2. Cases added** (the red output was captured from that case run alone, before the suite)

| case | asserts | now |
|---|---|---|
| `migrate_git.rs:1441` `beyond_the_aligned_table_equal_records_at_their_positions_are_not_reported` | old and new hold 2050 records each, and only the first and last differ. Exactly 2 differences should be reported, as the `ALIGNED_CELLS` doc promises ("every item is compared with the one at its position") | **red** |
| `migrate_git.rs:1481` `same_second_records_at_falling_revisions_verify_clean_in_revision_order` | records in one second at revision 2, then 1, then a copy of the first: the new store answers rev 1 first, then file order, and `verify` is clean | green (pin) |

Red output, verbatim:
```
assertion `left == right` failed: only the first and last records differ; 2050 lines were reported
  left: ["story:observed: evidence record 1 differs in change.source", "story:observed: evidence record 2 differs in the record", "story:observed: evidence record 3 differs in the record", …
 right: ["story:observed: evidence record 1 differs in change.source", "story:observed: evidence record 2050 differs in change.source"]
EXIT=101
```

**3. Suite**
- `cargo test -p aep-cli -p aep-backend-markdown --no-fail-fast`: 803 passed, 1 failed (the red case above), exit 101.
- The same run with `--skip` on my 2 cases: 802 passed, exit 0.
- `cargo fmt -p aep-cli -- --check`: exit 0. It ran after disk read 14G, which breaks the floor; it writes nothing.
- Clippy: not run (blocked).

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `migrate_git.rs:851` | INFEASIBLE | introduced | Above `ALIGNED_CELLS`, `unpaired` returns every middle position as unpaired. It does not compare each item with the one at its position, as the doc at `:830-831` says. So every equal record in the run is reported as "differs in the record". In the 2050-record run, 2048 of the 2050 lines were false. The exit status is still correct. | Nothing found. The fallback needs more than 2048 records on each side for one artifact. The largest per artifact in the 20 real `/1` journals is 24 evidence records and 3 moves. Fix: in the fallback, keep only positions where `old[i] != new[i]`, plus the length overhang. |

**5. Attacked and could not break**
- **Backdated evidence:** my pass-1 case now passes. Old records are stable-sorted by `history_order`, and the new store writes in journal order, so the per-second sequence matches.
- **Same second:** different revisions sort by revision, then file sequence. A duplicate split by another record in the same second keeps A, B, A.
- **Transitions in journal order:** no faithful `/1` migration fails on this. `/1` `move` took `clock_at_the_edge()` (`--at` only judged dated rungs), and 0 of 1063 real moves are out of order.
- **`executor`/`correlation` dropped by `transition_of`:** not reachable. Both fields were added in 267e4f5b2, after the `/1` retirement 7ec47c09d, and 0 of 1063 real moves carry them.
- **Values in messages:**
  - Title and body lines now print byte counts.
  - Transition lines print positions and field names; the new title and transition pins (`:1384`, `:1413`) hold.
  - The relations line prints `ArtifactRef` vocabulary only.
- **Tampering:**
  - The reorder (`:1355`), my 7 single-field edits, the extra record and the moved record are all reported.
  - My moved pin now asserts the exact list with "record 3 of 5 is missing". That is stricter than what I measured, and it still holds the count line.

**6. Paths written outside the worktree**
- `<harness task output> bief0mz6p.output`
- `<harness task output> bmr9mgpw3.output`
- `<harness task output> ` for the pass-1 runs (`bk33nhddj.output`, `b2shfrq33.output`) were already reported. The third path for this pass is the format-check-free scratch: none. Scratch stayed under `<worktree>/target/scratch/adversary-2/`.

Correction to the header count: this pass wrote 2 paths outside the worktree, the two output files above.

```findings
- file: crates/edge/aep-cli/src/store_command/migrate_git.rs
  line: 851
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: above ALIGNED_CELLS the fallback reports every middle position as unpaired instead of comparing each item with the one at its position as its doc says, so equal records are reported as "differs in the record"
```
