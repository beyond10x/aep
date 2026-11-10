---
format: aep.planning-md/3
id: review-result:store-pack-adversary-security-fixes-pass-2
kind: review-result
status: active
title: Adversary, pass 2, the store-pack security fixes
relations:
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
revision: 1
---
unit: aep store-pack security fixes, round 2 (de0b4409e on review/store-pack-security); findings cover 962b79647
verdict: red (strongest: CONFIRMED)
cases: executed 410→415, red 4
origin: introduced 4 / pre-existing 1 / undecided 0
wrote-outside-worktree: 3 locations (listed in part 6)
needs-coordinator: no

**Bottom line:** both pass-1 fixes hold, and `first_conflict` refuses correctly on every read verb, including for evidence paths. Four new cases are red: `validate` stops reading every document while any path is held twice, and two environment-only Git variables make provenance accept a forged committed pack.

Pass-1 check: `git diff 3813bd04d de0b4409e -- '*/tests/*'` is empty (0 lines). `store_pack_security_adversary` passes 3 of 3.

### 1. Diff stat (worktree is clean; this is the stat of my commit)
```
$ git --no-pager diff --stat de0b4409e HEAD
 .../tests/store_pack_security_adversary_2.rs       | 669 +++++++++++++++++++++
 1 file changed, 669 insertions(+)
```
Only a test file. Commit 962b79647.

### 2. Cases added: `crates/edge/aep-cli/tests/store_pack_security_adversary_2.rs`
| Case | Asserts | Now |
|---|---|---|
| `validate_reports_other_documents_problems_while_a_path_is_held_twice` (:324) | two packs hold `closed-a.md` with different bytes and `open-c`'s status was edited by hand; `validate` reports both | red |
| `an_evidence_path_held_twice_is_refused_by_read_verbs_and_unpack_from_resolves_it` (:383) | list/show/history/explain/board/graph/blocked exit 1 naming both packs; `unpack story:closed-a --from` resolves it | green |
| `git_graft_file_does_not_make_provenance_pass_a_forged_pack` (:502) | forged committed pack plus `GIT_GRAFT_FILE`: `validate` still exits 1 with `[pack_record_not_committed_bytes]` | red |
| `git_replace_ref_base_does_not_make_provenance_pass_a_forged_pack` (:563) | same, with `GIT_REPLACE_REF_BASE` and a tag pointing the original blob at the forged one | red |
| `validate_still_reports_a_rewritten_evidence_file_where_git_dir_selects_the_repository` (:622) | `.git` moved out of the tree, repository selected by `GIT_DIR`/`GIT_WORK_TREE`, evidence rewritten; `validate` exits 1 | red |

Red output from running this binary alone, quoted verbatim:
```
---- git_graft_file_does_not_make_provenance_pass_a_forged_pack stdout ----
assertion `left == right` failed: GIT_GRAFT_FILE rewrites the parent of the commit that added the pack and the forged pack validates: 3 file(s) in …/store-pack-security-adversary-2-graft-file/.engineering/planning: 3 artifact(s)
…
valid
  left: Some(0)
 right: Some(1)
---- git_replace_ref_base_does_not_make_provenance_pass_a_forged_pack stdout ----
assertion `left == right` failed: GIT_REPLACE_REF_BASE replaces the committed blob and the forged pack validates: … 3 artifact(s)
…
valid
  left: Some(0)
 right: Some(1)
```
On its first run, case 1 failed its own precondition because I expected the wrong message: the hand edit shows up as a parse failure, not a walk problem. I corrected the expected text and ran it again with case 5:
```
---- validate_reports_other_documents_problems_while_a_path_is_held_twice stdout ----
while a path is held twice, validate no longer reports the hand-edited status of another document (the store stops loading documents): 0 file(s) in …/store-pack-security-adversary-2-validate-still-reads/.engineering/planning: 0 artifact(s)
3 problem(s):
  - …/.engineering/planning: is not a pack: `planning/story/closed-a.md` is held with different bytes by …/9b3a2eea6833bf7d.pack and by …/aa0fa8f67e3bf8ce.pack; a copy to keep has to be chosen before it can be read
  - [pack_path_held_twice] …
  - [pack_record_not_committed_bytes] …
---- validate_still_reports_a_rewritten_evidence_file_where_git_dir_selects_the_repository stdout ----
assertion `left == right` failed: with the repository selected by GIT_DIR the rewritten evidence file validates: … 3 artifact(s)
…
valid
  left: Some(0)
 right: Some(1)
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 3 filtered out
```

### 3. Suite run (after the cases existed)
```
$ cargo.sh test --locked -p aep-cli --no-fail-fast --test store_pack --test store_pack_adversary --test store_pack_overlay --test store_pack_overlay_adversary --test store_pack_overlay_adversary_2 --test store_pack_security --test store_pack_security_adversary --test store_pack_security_adversary_2 --test validate_packs --test validate_packs_adversary --test validate_packs_adversary_2 --test git_store --test planning_cli --test doctor_cli --test cross_member_relations --test store_migrate_git
cross_member_relations 10 ok, doctor_cli 11 ok, git_store 26 ok, planning_cli 114 ok, store_migrate_git 30 ok,
store_pack 15 ok, store_pack_adversary 10 ok, store_pack_overlay 8 ok, store_pack_overlay_adversary 5 ok,
store_pack_overlay_adversary_2 7 ok, store_pack_security 13 ok, store_pack_security_adversary 3 ok,
store_pack_security_adversary_2: test result: FAILED. 1 passed; 4 failed
validate_packs 21 ok, validate_packs_adversary 6 ok, validate_packs_adversary_2 5 ok
exit 101
$ cargo.sh test --locked -p aep-backend-markdown --no-fail-fast --lib --test pack --test pack_adversary --test assembly
lib 95 ok, assembly 12 ok, pack 17 ok, pack_adversary 2 ok
exit 0
```
- **Counts:** 415 cases ran in total. 410 is the same run without my binary's 5 cases.
- **Other checks:** `cargo.sh fmt -p aep-cli` changed only my file. `cargo.sh clippy --locked -p aep-cli --test store_pack_security_adversary_2 -- -D warnings` exits 0.

### 4. Findings
1. **`crates/plan/aep-backend-markdown/src/store.rs:131`**: CONFIRMED, warning, introduced by de0b4409e.
   - **What was measured:** case :324. Once the conflict failure is pushed, `load` returns early, so `validate` reports "0 artifact(s)" and none of the other document problems. The comment next to the new check says the opposite ("the documents that read are still loaded, so `validate` reports on them").
   - **What reaches it:** any merge that leaves a path held twice. `validate` still exits 1, but it hides every other problem until the conflict is resolved. That includes the loose-document redaction checks that iterate `report.documents`.
   - **Fix:** do not return early for the conflict failure. Read verbs refuse through `is_pack()` anyway.
2. **`crates/edge/aep-cli/src/planning/git_record.rs:20`** (`repository_git`, `store_command.rs:227`): CONFIRMED, warning, pre-existing. At base 98fbc56f8 this was plain `Command::new("git")`; I read that, I did not run it. Neither round strips these variables.
   - **What was measured:** cases :502 and :563. `GIT_GRAFT_FILE` re-parents the commit that added the pack. `GIT_REPLACE_REF_BASE`, through a tag (tags come with a default clone), swaps the original blob. Either way provenance compares the forged record with forged bytes and `validate` prints "valid". This is a false pass, not a skip.
   - **What reaches it:** an environment set by whoever runs `validate`, plus a forged object in the clone. It is the same threat class as the `GIT_CEILING_DIRECTORIES` finding. `GIT_CONFIG_*` itself could not do this: core.bare, core.worktree and core.excludesFile from the environment did not change discovery or check-ignore on a tracked store.
   - **Fix:** remove `GIT_GRAFT_FILE`, `GIT_REPLACE_REF_BASE` and `GIT_SHALLOW_FILE`, and set `GIT_NO_REPLACE_OBJECTS=1`. That last setting does not turn grafts off; I measured it.
3. **`crates/edge/aep-cli/src/planning/git_record.rs:66`** (`repository_above`), together with `repository_git`: INFEASIBLE, note, introduced by 171689714.
   - **What was measured:** case :622. With no `.git` anywhere in the tree and the repository selected by `GIT_DIR`/`GIT_WORK_TREE`, the pre-existing committed-evidence check is skipped and a rewritten evidence file validates. At base, plain git found it.
   - **What reaches it:** no aep workflow. The hook case the brief asked about has a `.git` and works, and `--separate-git-dir` leaves a `.git` file, which works too.
   - **Fix:** when a repository-selecting variable is set and no `.git` is found, return `Failed`, not `Unversioned`.
4. **`crates/plan/aep-backend-markdown/src/store.rs:131-134`**: CONFIRMED, note, introduced.
   - **What was measured:** the conflict is printed as `<planning dir>: is not a pack: … held with different bytes …`, by `validate` and by every read verb. That names the planning directory as "not a pack" (it reuses the `PACK_FAILURE` prefix so `is_pack()` matches).
5. **`crates/plan/aep-backend-markdown/src/journal.rs:487` and `:568`**: INFEASIBLE, note, introduced.
   - **What was measured:** `evidence_on_hand_git` drops the unreadable count, and `redaction_records_git` silently skips a record whose `read` refuses. So library readers drop a path held twice instead of refusing it.
   - **What reaches it:** nothing found. Every CLI caller runs after `open`, `refuse_unparsed_packs` or `require_clean`, and those already refuse.

### 5. Attacked and could not break
- Every read verb refuses a path held twice, whether the conflict is in planning or evidence, and names both places (case :383 and the pass-1 case).
- Workspace `assemble` (`is_pack`), the driver's `graph()` (`is_clean`), `graph_at` and write verbs all refuse through `load`. I read the code; I wrote no case for these.
- `unpack --from` still resolves the conflict, because it does not go through `first_conflict` (case :383).
- `first_conflict` matches `read` exactly: a loose copy is compared with the first pack, then pack with pack, transitively. Identical copies pass. Duplicate paths inside one pack are refused by the parser.
- Cost of `first_conflict`: one pass over all packed records per `load`, with the parse cached, plus one failed open per packed path. No hazard found.
- The length+mtime cache lives per process and serves bytes it already verified, never unverified ones.
- The specification reader refuses a conflicting specification through `read`.
- The conflict error message holds no record text and no path outside the store.
- `GIT_CONFIG_*` kept for `safe.directory`: none of core.bare, core.worktree, core.excludesFile (via the environment or `XDG_CONFIG_HOME`), or log signatures (these fail closed) skipped provenance or made it pass falsely.

### 6. Paths written outside the worktree
- `<scratch>/pack/units/security-review/adv2-scratch/`:
  - scripts: `probe-git.sh`, `probe-graft.sh`, `suite.sh`
  - probe repositories and inputs: `probe-repo/`, `probe-graft/`, `probe-excludes`, `probe-excludes2`, `probe-xdg/`, `probe-grafts`
  - logs: `red-run.log`, `red-run-2.log`, `suite-cli.log`, `suite-md.log`, `clippy.log`
  - `commit-msg.txt`
- Test fixtures under the shared target: `<scratch>/target/tmp/store-pack-security-adversary-2-*`, including `…-env-git-dir.git`. Build output went to the same shared target through cargo.sh.
- Harness output files for background tasks: `<harness>/tasks/{bvowuh4mi,b5dd40zrh,bre75bp10,bbjg71d01}.output`

Lease `aep-adv-sec-2` is released (`session-end` done).

### 7. Findings block
```findings
- file: crates/plan/aep-backend-markdown/src/store.rs
  line: 131
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a path held twice now makes load return before reading any document, so validate reports 0 artifacts and hides every other document problem, against the comment beside the check
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 20
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: provenance git keeps GIT_GRAFT_FILE and GIT_REPLACE_REF_BASE, and either makes validate accept a forged committed pack as the committed blob
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 66
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: with the repository selected by GIT_DIR/GIT_WORK_TREE and no .git in the tree, the store counts as unversioned and the pre-existing committed-evidence check is skipped
- file: crates/plan/aep-backend-markdown/src/store.rs
  line: 131
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'a path held twice is printed as "<planning dir>: is not a pack: …", naming the planning directory as a pack that does not read'
- file: crates/plan/aep-backend-markdown/src/journal.rs
  line: 487
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: evidence_on_hand_git and redaction_records_git silently drop a record held twice with different bytes; no caller reaches them without the CLI refusal
```
