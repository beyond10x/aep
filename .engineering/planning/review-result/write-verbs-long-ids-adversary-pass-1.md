---
format: aep.planning-md/3
id: review-result:write-verbs-long-ids-adversary-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:write-verbs-refuse-long-artifact-ids
relations:
- reviews: story:write-verbs-refuse-long-artifact-ids
revision: 1
---
```
unit: story:write-verbs-refuse-long-artifact-ids, branch fix/write-verbs-long-artifact-ids at 707ced375 (adversary cases committed as da5472f13)
verdict: CONFIRMED (one warning, a test gap; no defect found in the code)
cases: executed 153→160, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 scratch directory (part 6)
needs-coordinator: none
```

I couldn't break the fix. Replay of short ids is byte-identical to the base on every write verb and on seeding. The one finding is a test gap: before my cases, nothing in the suite ran body, set or scope on a long id, so undoing that part of the fix would have stayed green.

**1. Diff stat (707ced375..HEAD)**
```
 crates/edge/aep-cli/tests/long_artifact_ids.rs | 433 +++++++++++++++++++++++++
 1 file changed, 433 insertions(+)
```
Only a test file changed. No source file was edited.

**2. Cases added** in `crates/edge/aep-cli/tests/long_artifact_ids.rs`. All 7 are green now; none was red when first run.

| Case | What it checks |
|---|---|
| `every_write_verb_on_short_ids_records_the_identifiers_it_recorded_before_the_bound` | On a SQLite store, new, move, relate, unrelate, relate again, evidence, body, set and scope on short ids record the exact identifiers the base recorded (idempotency key, command id, request id, correlation; timestamps masked). Expected values are copied as literals from a run of the base binary. |
| `an_identifier_exactly_at_the_cap_is_the_one_built_before_and_one_over_is_bounded` | A name whose `req-` and `cmd-` identifiers are exactly 200 characters is unchanged. One character more and those two are shortened, while the idempotency key (197 characters) is still unchanged. |
| `body_set_scope_and_redact_succeed_on_an_id_near_the_limit` | body (whole and `--section`), set, scope and redact work on a 198-character id. |
| `every_identifier_a_sqlite_store_keeps_for_over_cap_ids_is_valid_and_distinct` | Every one of the 14 recorded identifiers is valid and has its prefix. Command ids and keys are all distinct, including for two targets that differ only in their last character. |
| `an_over_cap_command_id_is_the_same_in_every_process_and_across_releases` | The shortened relate command id is the same in two separate processes, and is pinned as a literal (`cmd-story-a…-640cdeaeca912bc6`). |
| `relating_a_long_edge_twice_is_nothing_to_do_as_it_is_for_a_short_one` | A repeated relate answers "nothing to do", the same for long and short ids. |
| `a_crossing_between_two_200_character_ids_is_written_and_taken_back` | A cross-member relate and unrelate between 200-character ids in a workspace. |

Output of the cases run alone:
```
test a_crossing_between_two_200_character_ids_is_written_and_taken_back ... ok
test body_set_scope_and_redact_succeed_on_an_id_near_the_limit ... ok
test relating_a_long_edge_twice_is_nothing_to_do_as_it_is_for_a_short_one ... ok
test an_identifier_exactly_at_the_cap_is_the_one_built_before_and_one_over_is_bounded ... ok
test an_over_cap_command_id_is_the_same_in_every_process_and_across_releases ... ok
test every_write_verb_on_short_ids_records_the_identifiers_it_recorded_before_the_bound ... ok
test every_identifier_a_sqlite_store_keeps_for_over_cap_ids_is_valid_and_distinct ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 12.06s
```

**3. Suite run**, after the cases existed, with `scratch/suite.sh`:
```
$ cargo.sh test --locked -p aep-cli --test long_artifact_ids --test planning_cli --test git_store --test cross_member_relations
test result: ok. 10 passed; ... (cross_member_relations)
test result: ok. 26 passed; ... (git_store)
test result: ok. 10 passed; ... (long_artifact_ids)
test result: ok. 114 passed; ... (planning_cli)
exit=0
$ cargo.sh test --locked -p aep-domain --lib ids::
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 322 filtered out
exit=0
$ cargo.sh test --locked -p aep-cli --test long_artifact_ids -- --skip <the 7 adversary cases>
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out
exit=0
$ cargo.sh clippy --locked -p aep-cli --all-targets -- -D warnings
exit=0
$ cargo.sh fmt -p aep-cli -- --check
Diff in .../tests/long_artifact_ids.rs:370:
Diff in .../tests/long_artifact_ids.rs:439:
exit=1
```
The fmt failure was in my new code. I ran `cargo.sh fmt -p aep-cli`, the check then passed (exit 0), and the formatted file is what I committed. "Before" is 153 (160 minus my 7 cases, which the `--skip` run excluded).

**Replay check, base binary against head binary.** The base binary is `bin/aep`, built at 5b1a7b9e2. The head binary I built at 707ced375. Both ran the same short-id sequence of new, move, relate, unrelate, relate, evidence, redact, body, set and scope, then validate:
- **SQLite store:** the database dump, every file under `.engineering` and the command output are identical after masking timestamps.
- **Git store:** identical except the redaction evidence file name. That name carries a timestamp and a hash that changes with it, so the difference is only the run time. The evidence file written with a fixed `--at` has the same name from both binaries.
- **Seeding this repository's `.engineering`:** `list --format json`, `graph --format json` and `validate` give identical output from both binaries. The longest seed command name here is 133 characters (0 over 196), so no seed identifier is shortened.

**4. Findings**

1. **The suite would not catch undoing the body/set/scope part of the fix.** Category: mutant. Verdict: CONFIRMED. Origin: introduced (a gap in the unit's own tests).
   - **What was measured:** `crates/edge/aep-cli/src/planning.rs:3645` (`update_through_a_command`) builds identifiers separately from `envelope_for`. Before this pass, nothing in the suite ran body, set or scope on a long id, so putting this site back to the base code left everything green. The base binary run on a 198-character id in a SQLite store shows what that looks like: `error: invalid request identifier "req-protocol-artifact-body-story-cccccccccc…` (set and scope fail the same way).
   - **What reaches it:** `aep plan artifact body`, `set` and `scope`.
   - **Covered now by:** `body_set_scope_and_redact_succeed_on_an_id_near_the_limit` and the SQLite over-cap case.

**5. What I attacked and couldn't break**
- Byte-identical replay at or under the cap: the SQLite comparison above, plus a literal pin.
- The exact boundary: 200 characters unchanged, 201 shortened.
- Deterministic output across processes.
- Two ids that share a long prefix and differ only after the cut get distinct ids.
- Every prefix is valid: `req-`, `cmd-`, `rel-`, `unrel-`, `upd-`, `req-unrel-`, `cmd-unrel-`, and seed `req-`. All identifier types are the `Loose` charset, which has no forbidden-segment rule.
- Slicing can't panic: the cut is always inside the name and steps back to a character boundary.
- No construction site was missed. A grep found every identifier built from a name in planning.rs, seed.rs, store.rs and the other backends. The remaining ones are hashed or sequence numbers.
- redact and crossings work on 200-character ids.
- Relating the same long edge twice is a no-op ("nothing to do"), as it is for short ids.
- The unit's existing tests do assert real behaviour. For example, `show` on a draft story doesn't contain "proposed", so the move check can fail.
- Hash choice: the unit brief asked for SHA-256 and the code uses FNV-1a 64. The CHANGELOG states this correctly, and `store.rs::derived` already uses FNV, so it follows existing practice. A deliberately built collision would cause a loud `IdempotencyMismatch` refusal, not a silent merge. I'm not raising it as a finding.

**6. Paths written outside the worktree**
- `<scratch>/pack/units/write-verbs-long-ids/adv-scratch/` (the assigned scratch directory): the scripts `replay-probe.sh`, `long-probe.sh`, `cross-probe.sh`, `normalize.sh`, `seedlen.py` and `suite.sh`; the binaries `aep-base` and `aep-head`; the logs `build-head.log`, `cases-alone.log`, `suite.log` and `commit-msg.txt`; the probe directories `run-base/`, `run-base-sql/`, `b-git/`, `h-git/`, `b-sqlite/`, `h-sqlite/`, `l-git/`, `l-sqlite/`, `l-sqlite-base/` and `x-head/`; and `repo-{base,head}-{list,graph,validate}.out`.
- Through cargo.sh, the shared target directory now holds a build of this tree: `.../aep/target/debug/aep` and the test binaries.
- The tests write their fixtures under the target's `CARGO_TARGET_TMPDIR`: `long-ids-*` and `long-ids-ws-crossing`.

The lease is released.

**7. Findings block**
```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 3645
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: body, set and scope build their bounded identifiers outside envelope_for and no unit case ran them on a long id, so reverting that site left the suite green; now covered by body_set_scope_and_redact_succeed_on_an_id_near_the_limit
```
