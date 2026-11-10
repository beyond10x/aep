---
format: aep.planning-md/3
id: review-result:store-pack-security-review-1
kind: review-result
status: active
title: Security review, store-pack feature
relations:
- reviews: story:lifecycle-declares-closed-states
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
revision: 1
---
```
unit: whole store-pack feature (epic:store-pack-closed-artifacts), branch review/store-pack-security, findings cover 285739349; tests committed as 98fbc56f8
verdict: red
cases: executed 77→85, red 6
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 16 entries in the assigned scratch directory, plus test fixtures and build output under the shared cargo target (full list in part 6)
needs-coordinator: yes. Invariant 2 as you stated it ("no reader serves bytes whose digest was not checked") conflicts with the design and the backend pack module's doc comment (crates/plan/aep-backend-markdown/src/pack.rs:33-35), which leave digest and name checks to `validate`. Someone has to choose which one wins. The shared target also means `CARGO_BIN_EXE_aep` can be rebuilt by another unit between runs.
```

Three of the four invariants do not hold. Invariant 1 holds.

## Invariant verdicts

| # | Invariant | Verdict | Evidence |
|---|---|---|---|
| 1 | No record lost | **holds**, with one caveat | No committed record lost in pack, unpack, interrupted runs, deleted committed packs (`pack_lost`) or merges. Caveat: when two packs hold one path with different bytes and there is no loose copy, readers pick one silently instead of refusing (same cause as finding 6). A clean merge can also leave `/5` with a pack (finding 7); `validate` refuses it. |
| 2 | Digest verification | **does not hold** | Readers serve bytes that fail their digest (finding 1). Readers serve records from a pack whose name is not its digest (finding 2). `unpack` re-signs a tampered record (finding 3). `GIT_CEILING_DIRECTORIES` turns provenance off (finding 4). |
| 3 | Path traversal | **does not hold** | `pack` follows a symlinked `.engineering/planning`, packs the files behind it and deletes them outside the store (finding 5). `unpack` holds: `unpack_refuses_ids_and_pack_names_outside_the_grammar_and_writes_nothing` is green. |
| 4 | Redaction literal | **does not hold** | `validate` refuses the re-exposure, but `show` prints the literal after the design's own two-unpack merge resolution (finding 6). Packing and unpacking keeps the replay valid: `a_redacted_review_result_packed_and_unpacked_keeps_the_replay_valid` is green. |

## 1. `git --no-pager diff --stat 285739349 HEAD`

```
 crates/edge/aep-cli/tests/store_pack_security.rs | 772 +++++++++++++++++++++++
 1 file changed, 772 insertions(+)
```

The only path changed is a test file. No source file was edited.

## 2. Cases added

All are in `crates/edge/aep-cli/tests/store_pack_security.rs`. Run alone first with `cargo.sh test --locked -p aep-cli --test store_pack_security -- --test-threads=4`, which gave `2 passed; 6 failed`. Excerpts below are verbatim, cut with […].

- **`readers_never_serve_a_packed_record_whose_bytes_fail_their_digest`** (:324), red. It edits a record inside its pack without changing the length, so the header digest and file name stay as they were.
  ```
  `show story:closed-a` served bytes that fail their digest (exit Some(0)): id        story:closed-a […] title     Forged A
  ```
- **`readers_never_serve_a_record_from_a_pack_whose_name_is_not_its_content_digest`** (:361), red. Every record matches its digest; only the file name is stale.
  ```
  `show story:closed-a` served a record from a pack whose name is not its digest (exit Some(0)): […] title     Forged A
  ```
- **`unpack_never_rewrites_a_record_that_fails_its_digest_into_one_that_passes`** (:402), red.
  ```
  `unpack story:closed-a` (exit Some(0)) re-signed the tampered closed-b in 838f390d7f32113c.pack: its digest now matches bytes no commit ever held: […] rewrite 3886064c794a8a84 -> 838f390d7f32113c
  ```
- **`git_ceiling_directories_does_not_skip_pack_provenance`** (:435), red. As a precondition, `validate` without the variable refuses the forged committed pack.
  ```
  GIT_CEILING_DIRECTORIES=<target tmp>/store-pack-security-ceiling turns provenance off and the forged pack validates: […] valid
    left: Some(0)
   right: Some(1)
  ```
- **`pack_never_follows_a_symbolically_linked_planning_directory`** (:501), red.
  ```
  `pack` (exit Some(0)) removed or changed files behind the symbolic link .engineering/planning: pack: a08176ffd5ad2b9c […]
    remove planning/story/closed-a.md
    remove planning/story/closed-b.md
  […] packed 3 record(s) into a08176ffd5ad2b9c.pack; removed 3 loose file(s)
  ```
- **`unpack_refuses_ids_and_pack_names_outside_the_grammar_and_writes_nothing`** (:544), green.
- **`a_redacted_review_result_packed_and_unpacked_keeps_the_replay_valid`** (:632), green. Covers pack and unpack with and without `--dry-run`, `validate` after each, and no literal in any output or pack file.
- **`no_read_verb_prints_a_redacted_literal_a_two_branch_unpack_merge_leaves_packed`** (:693), red. Precondition: `validate` exits 1 with `pack_redacted_literal_exposed`.
  ```
  `show review-result:looked` (exit Some(0)) printed the redacted literal from 193197ce456a6289.pack: […]
  ## Verdict

  <the redacted literal> approved it.
  ```

## 3. Suite run, after the cases existed

Command: `cargo.sh test --locked -p aep-cli --test store_pack --test store_pack_adversary --test store_pack_overlay --test store_pack_overlay_adversary --test store_pack_overlay_adversary_2 --test validate_packs --test validate_packs_adversary --test validate_packs_adversary_2 --test store_pack_security --no-fail-fast`

```
store_pack                      test result: ok. 15 passed; 0 failed
store_pack_adversary            test result: ok. 10 passed; 0 failed
store_pack_overlay              test result: ok. 8 passed; 0 failed
store_pack_overlay_adversary    test result: ok. 5 passed; 0 failed
store_pack_overlay_adversary_2  test result: ok. 7 passed; 0 failed
store_pack_security             test result: FAILED. 2 passed; 6 failed
validate_packs                  test result: ok. 21 passed; 0 failed
validate_packs_adversary        test result: ok. 6 passed; 0 failed
validate_packs_adversary_2      test result: ok. 5 passed; 0 failed
error: 1 target failed:
exit 101
```

"Before" is 77: the same run with `store_pack_security` left out, which is a separate binary. "After" is 85.

## 4. Findings (all against 285739349)

1. **Readers do not check record digests.** `crates/plan/aep-backend-markdown/src/pack.rs:769`, NEEDS-CHANGE, introduced.
   - **Measured:** `Overlay::holders` hands out `record.bytes` without `digest_matches()`, and `show`/`list` exit 0 with tampered text.
   - **What reaches it:** any edit to a pack that keeps the lengths, for example someone fixing a typo in a pack, which the format is designed to be readable for. Only `validate` catches it.
   - **Fix to name:** `Overlay::parsed` (:732) refuses a pack with any digest mismatch, the same way the read verbs already refuse a pack that does not parse.
2. **Readers do not check the pack name.** `pack.rs:736`, NEEDS-CHANGE, introduced.
   - **Measured:** `parsed()` accepts the file stem and never compares it with `Pack::name()`.
   - **What reaches it:** the same hand edits as finding 1, with digests recomputed.
   - **Fix to name:** the same refusal in `parsed()`.
3. **`unpack` re-signs tampered records.** `crates/edge/aep-cli/src/store_command/unpack.rs:201`, NEEDS-CHANGE, introduced.
   - **Measured:** `render_without` re-renders the records it keeps and computes fresh digests, so the digest check stops flagging a tampered record.
   - **What reaches it:** `unpack <id>` on a pack that also holds a tampered record. In a versioned store, provenance still refuses it; in an unversioned copy nothing does.
   - **Fix to name:** before rewriting, refuse when any record of an affected pack fails its digest.
4. **An environment variable switches provenance off.** `crates/edge/aep-cli/src/planning/git_record.rs:66`, NEEDS-CHANGE, introduced.
   - **Measured:** `repository_above` honours `GIT_CEILING_DIRECTORIES`, so the store counts as Unversioned and provenance and `pack_lost` are skipped. `pack` itself clears the variable (`REDIRECTING_GIT_VARIABLES`) and works.
   - **What reaches it:** a shell or CI that sets the variable at or below the repository top.
   - **Fix to name:** ignore the ceiling when looking for a repository, and run the `versioning`, `blobs_before_pack` and `committed_packs` git calls with the redirecting variables removed, as `store_command/pack.rs:258` does.
5. **`pack` follows a symlinked planning directory.** `crates/edge/aep-cli/src/store_command/pack.rs:299`, INFEASIBLE, introduced.
   - **Measured:** `open_repo` refuses a symlink only at `.engineering` and `packs`. `loose_files` lists through a symlinked `planning`, `git hash-object` follows it, and the files outside the store are deleted.
   - **What reaches it:** a work tree where someone replaced `planning` (or `evidence`) with a link after committing. Nothing in aep creates that, which is why it is INFEASIBLE. You named symlinked directories in invariant 3, so it is still listed, at warning severity.
   - **Fix to name:** add `planning` and `evidence` to the symlink check at :299.
6. **`show` prints a redacted literal.** `crates/plan/aep-backend-markdown/src/pack.rs:780`, NEEDS-CHANGE, introduced.
   - **Measured:** `Overlay::read` serves the first holder in pack-name order. After the merge, that can be the copy that still holds the literal.
   - **What reaches it:** the design's documented resolution (`merge.renames=false`, keep both rewrites) when one branch unpacks, redacts and repacks. The scratch probe printed the literal in 2 of 3 variants.
   - **Fix to name:** `read` refuses a path held in two places with different bytes, naming both, as `unpack` and `validate` already do.
7. **A clean merge leaves `/5` with a pack.** `crates/edge/aep-cli/src/store_command/unpack.rs:313`, CONFIRMED by scratch probe (not a committed test), introduced.
   - **Measured:** one branch runs `unpack --all` (back to `/5`), the other packs a newly closed story. The merge is clean and leaves `version: aep.project/5` plus `f038aae1f83ba1d9.pack`. `validate` reports `[pack_version_disagrees]`.
   - **What reaches it:** two ordinary branches.
   - **Not measured:** that an aep build from before packs would silently undercount this store, which is the outcome the ADR exists to prevent. That follows from the ADR and was not run; no pre-feature binary was available here.
8. **The temporary file can be written through a link.** `crates/edge/aep-cli/src/store_command/pack.rs:173`, INFEASIBLE, introduced.
   - **Measured:** `write_atomic` writes a temporary whose name is predictable from the process id with `fs::write`, which follows a link already sitting at that name.
   - **What reaches it:** nothing found; it needs a link planted under the exact process id.
   - **Fix to name:** create the temporary with `create_new`.

## 5. Checked and could not fault

- `unpack` id and `--from` arguments outside the grammar, its target walk through symlinks, and a symlinked packs directory (green case plus existing cases).
- Write ordering in `pack` and `unpack`, and reruns after interruption (code read plus existing adversary cases).
- Every write verb on a packed artifact refuses (existing case).
- Provenance under an ownership refusal, no git, a shallow clone, `--store .`, a nested repository and a symlinked packs directory (existing cases, run in the suite).
- `GIT_DIR` pointing elsewhere makes `validate` refuse (`Committed::Never`) rather than skip. Code read only, no case.
- Redaction replay across real `pack` and `unpack` (green case). The redacted-literal check runs on pack-free stores.
- `validate` output never prints the literal (existing case plus the precondition in finding 6's case).

## 6. Paths written outside the worktree

- Scratch directory `<scratch>/pack/units/security-review/scratch/`, 16 entries:
  - `aep-review`: a copy of the binary built from this tree
  - `body-0.md`, `body-1.md`, `body-2.md`
  - `build.log`, `run1.log`, `suite.log`
  - `commit-msg.txt`, `msg`
  - `hookprobe/`
  - `probe-merge-0/`, `probe-merge-1/`, `probe-merge-2/`
  - `probe_merge.sh`
  - `probe-version/`, `probe_version.sh`
- `<scratch>/target/`: shared build output from `cargo.sh`.
- `<scratch>/target/tmp/store-pack-security-*`: test fixtures.
- `<scratch>/tmp/`: the `TMPDIR` that `cargo.sh` sets.

The lease was released with `session-end`.

## 7. Findings block

```findings
- file: crates/plan/aep-backend-markdown/src/pack.rs
  line: 769
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Overlay::holders serves packed record bytes without checking their header digest, so show and list exit 0 with bytes that fail it
- file: crates/plan/aep-backend-markdown/src/pack.rs
  line: 736
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Overlay::parsed accepts a pack whose file name is not its content digest, and every reader serves its records
- file: crates/edge/aep-cli/src/store_command/unpack.rs
  line: 201
  category: integrity
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: unpack re-renders kept records with fresh digests, so a record that failed its digest comes out of the rewrite passing it
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 66
  category: integrity
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: repository_above honours GIT_CEILING_DIRECTORIES, which pack clears, so the variable makes validate skip provenance and accept a forged committed pack
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 299
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: pack refuses a symbolic link only at .engineering and packs, so through a linked planning directory it packs and deletes files outside the store
- file: crates/plan/aep-backend-markdown/src/pack.rs
  line: 780
  category: integrity
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Overlay::read serves the first of two packs holding one path with different bytes, so after the design's two-unpack merge resolution show prints a redacted literal
- file: crates/edge/aep-cli/src/store_command/unpack.rs
  line: 313
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a clean merge of unpack --all with a branch that packs leaves aep.project/5 holding a pack, which only validate refuses and an older reader would read without the packed records
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 173
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: write_atomic writes a temporary named by process id with fs::write, which follows a link already planted at that name
```
