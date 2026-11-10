---
format: aep.planning-md/3
id: review-result:store-pack-adversary-validate-packs-pass-2
kind: review-result
status: active
title: Adversary, pass 2, story:validate-checks-packs
relations:
- reviews: story:validate-checks-packs
revision: 1
---
```
unit: story:validate-checks-packs, branch impl/validate-checks-packs, findings cover 604978267 (cases committed as ef579e753)
verdict: NEEDS-CHANGE
cases: executed 184→189, red 5
origin: introduced 3 / pre-existing 0 / undecided 1
wrote-outside-worktree: 9 paths in the assigned scratch plus fixture repositories under the shared target's tmp (part 6)
needs-coordinator: yes (whether design item 7's "loose copy" applies to a store with no pack, finding 4)
```

`validate` still answers `valid` on a committed forged pack that the readers serve. Three new cases show this, all through the one answer the fix kept as a skip: `Unversioned`.

**1. Diff stat** (`git --no-pager diff --stat 604978267 HEAD`). One new test file and nothing else, so the bound held:
```
 .../aep-cli/tests/validate_packs_adversary_2.rs    | 542 +++++++++++++++++++++
 1 file changed, 542 insertions(+)
```

**2. Cases added**, all in `crates/edge/aep-cli/tests/validate_packs_adversary_2.rs` and all red now. Each red output below is from the first run of that binary, filtered to the case, before the suite ran.

In the three forgery cases the control steps passed first: validate refused the forged pack when Git could answer, and `show` served "Forged Z".

- `:298` `a_repository_git_refuses_to_read_for_its_owner_does_not_skip_provenance`. A committed forged pack, validated with `GIT_TEST_ASSUME_DIFFERENT_OWNER=1`. That is Git's own switch for the `safe.directory` ownership refusal.
  ```
  assertion `left == right` failed: a versioned store Git refuses to read is not an unversioned one; the forged pack 184a6721b569e32f is refused, not waved through: 3 file(s) in .../validate-packs-adv2-dubious-owner/.engineering/planning: 3 artifact(s)
  valid
    left: Some(0)
   right: Some(1)
  ```
- `:336` `no_git_program_on_the_path_does_not_skip_provenance_of_a_versioned_store`. The same forgery, with `PATH` set to an empty directory.
  ```
  assertion `left == right` failed: the store has a `.git`; with no Git to ask, the forged pack is not admitted: 3 file(s) in .../validate-packs-adv2-no-git/.engineering/planning: 3 artifact(s)
  valid
    left: Some(0)
   right: Some(1)
  ```
- `:368` `a_store_named_dot_is_held_to_provenance_as_the_same_store_named_by_its_path`. The store is outside a project directory. `validate --store store` refuses the forged pack; `cd store && validate --store .` accepts it.
  ```
  assertion `left == right` failed: `--store .` is the same committed store and its forged pack 7fa49656b05e072f is refused: 2 file(s) in .: 2 artifact(s)
  valid
    left: Some(0)
   right: Some(1)
  ```
- `:440` `a_shallow_clone_whose_boundary_is_the_removal_before_the_pack_names_the_shallow_clone`. The loose file was removed in one commit and packed in a later one, which is the flow the doc comment at `git_record.rs:314` describes. The clone is `--depth 3`, so its boundary is the removal commit.
  ```
  1 problem(s):
    - [pack_record_not_committed_bytes] .../packs/c29ce63fbb946973.pack: `planning/story/closed-b.md` has no committed blob at its original path before the commit that added this pack
  ```
- `:485` `a_loose_review_result_that_puts_back_a_redacted_literal_is_refused_without_a_pack`. A store with no pack and a redacted review-result whose body is edited back to the literal and committed.
  ```
  assertion `left == right` failed: a loose copy that re-exposes a redacted literal is refused: 2 file(s) in .../validate-packs-adv2-literal-loose/.engineering/planning: 2 artifact(s)
  1 review(s) recorded no findings block: ...
  valid
    left: Some(0)
   right: Some(1)
  ```

**3. Suite run**, after the cases existed:
```
cargo.sh test --locked -p aep-cli --no-fail-fast --test validate_packs --test git_store --test store_pack_overlay --test store_pack_overlay_adversary --test record_redaction_adversary --test planning_cli --test validate_packs_adversary --test validate_packs_adversary_2
git_store                     test result: ok. 26 passed; 0 failed
planning_cli                  test result: ok. 114 passed; 0 failed
record_redaction_adversary    test result: ok. 4 passed; 0 failed
store_pack_overlay            test result: ok. 8 passed; 0 failed
store_pack_overlay_adversary  test result: ok. 5 passed; 0 failed
validate_packs                test result: ok. 21 passed; 0 failed
validate_packs_adversary      test result: ok. 6 passed; 0 failed
validate_packs_adversary_2    test result: FAILED. 0 passed; 5 failed
exit=101
```
- **Executed 184→189:** 184 is the implementor's gate run at 604978267 (`scratch/gate-cli.txt`, same binaries without mine).
- **Clippy:** `cargo.sh clippy --locked -p aep-cli --test validate_packs_adversary_2 -- -D warnings` exit 0.
- **Format:** `cargo.sh fmt -p aep-cli -- --check` exit 0, after I wrapped one line of my own file by hand.

**4. Findings**

- **F1 (NEEDS-CHANGE, blocker, introduced)**, `planning/git_record.rs:44`, consumed at `planning.rs:7015`.
  - `unversioned` answers true whenever `git rev-parse --is-inside-work-tree` fails. That includes a versioned repository Git refuses to read because of its owner, and a machine with no `git` program. `blobs_before_pack` then returns `Unversioned`, and `pack_problems` skips provenance with `continue`.
  - The fix promised that only a store with no history skips and any Git failure refuses. Both of these are Git failures and both skip.
  - What was measured: cases :298 and :336, exit 0 and `valid` with forged bytes served.
  - What reaches it: `docker run -v "$PWD:/work" …` with a different uid in the container. Git 2.35.2 and later refuse such a repository as "dubious ownership" (Git's "dubious ownership" refusal), so a CI container job reaches this with no attacker setup. An image that has aep but no Git reaches it too.
  - `unversioned` itself is unchanged from base (`git show 978bce9d0:…/git_record.rs`), so the other history checks share the hole. Treating it as a skip for the security check is new in this unit.
  - Fix, not applied: decide "unversioned" by whether a `.git` file or directory exists above the store (respecting `GIT_CEILING_DIRECTORIES`). If one exists and Git cannot answer, return `Failed`.
- **F2 (NEEDS-CHANGE, warning, introduced)**, `planning/git_record.rs:38`.
  - `Path::new(".").file_name()` is `None`, so `unversioned(".")` returns true. `--store .` on a store outside a project directory therefore skips provenance and `committed_packs`.
  - What was measured: case :368.
  - What reaches it: `aep plan artifact validate --store .` run inside a non-project store. Project stores are not affected, because `project_plan_at` turns the path into an absolute one.
  - Fix, not applied: make `base` absolute (`std::path::absolute`) in `pack_problems` before any Git call.
- **F3 (NEEDS-CHANGE, note, introduced)**, `planning/git_record.rs:352` and the fallback at `:389-405`.
  - The shallow answer is given only when the pack's add commit is the boundary. If the boundary is an earlier commit that removed the loose file, the fallback `log` stops at the grafted root and the record is accused of forgery.
  - What was measured: case :440.
  - What reaches it: the two-commit flow the doc comment describes. The `pack` verb's own flow is a single commit and is handled.
- **F4 (CONFIRMED, warning, undecided)**, `planning.rs:7236`.
  - Design item 7 says "no copy of an artifact (loose or any pack)" may contain a redacted literal. The check runs only when the store holds a pack.
  - In a store with no pack, a review-result whose body is put back to its pre-redaction text is `valid`. `review_problems` sees that the body equals the first committed version and continues without replaying.
  - What was measured: case :485.
  - What reaches it: a hand edit, or a partial `git checkout <before-redact> -- file`, in any `/5` store.
  - Base has no item-7 check at all; I did not run the case against base. Whether item 7 covers stores with no pack is for you to decide.

**5. Attacked, could not break**
- **Ignore patterns:** `.gitignore` with `.engineering`, `.engineering/`, `*` or `/.engineering` over a store with tracked files does not trigger `Unversioned`; `check-ignore` exits 1 (probe in `scratch/probe2`).
- **Hook environment:** a pre-commit hook in a linked worktree exports an absolute `GIT_DIR`. By reading the code, provenance then answers Never, which is a false refusal and not a skip. Not run through aep; `install-hooks` is not implemented.
- **Pack renamed or moved in the working tree:** a new name is caught by the name check. A move to a subdirectory, a hidden name or a `.PACK` extension leaves the reader ignoring it, and `pack_lost` reports it. A `git rm` deletion is `pack_lost`. After `git rm --cached` the pack is still present and valid.
- **Pack deletion committed:** silent, but a committed deletion of loose evidence is silent too (`changed_evidence` compares the working tree with HEAD only). Consistent with loose, so not raised.
- **Unpack then commit with changed bytes:** `valid`. This is the delete-and-recreate gap the unit already reported, and committed edits to loose evidence are not checked either.
- **Merge history:** history simplification on a merge hides a side-branch edit from `committed_versions`. That reaches nothing the linear delete-and-recreate gap does not already reach.
- **`.engineering` linked outside any repository:** Git runs in the link's target and finds no history, so every record answers Never and is refused.
- **Pass-1 cases kept as written:** `git diff b70c21e78 604978267 -- crates/edge/aep-cli/tests/validate_packs_adversary.rs` is empty.

**6. Paths written outside the worktree**
- Scratch: `<scratch>/pack/units/validate-checks-packs/scratch/{adv2-red-run.log, adv2-red-run-2.log, adv2-suite.log, adv2-clippy.log, adv2-commit-msg.txt, adv2-more.rs}` and `scratch/probe2/{ignore.sh, hookenv.sh, msg.txt, r/, h/, hl/}`. `h` and `hl` are a throwaway repository and its linked worktree, used for the hook-environment probe.
- Fixture repositories: `<scratch>/target/tmp/validate-packs-adv2-*`.
- Lease `aep-adv-validate-packs-2` is released.

**7. Findings block**
```findings
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 44
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: unversioned() answers true when git cannot read a versioned repository (safe.directory ownership refusal, no git on PATH), so provenance is skipped and a committed forged pack the readers serve validates as valid
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 38
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: unversioned(".") returns true because "." has no file name, so `validate --store .` on a store outside a project directory skips provenance and answers valid on a forged pack
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 352
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the shallow answer is given only when the pack's add commit is the boundary; with the boundary on an earlier loose-removal commit the record is accused of uncommitted bytes
- file: crates/edge/aep-cli/src/planning.rs
  line: 7236
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: the redacted-literal check runs only when a pack exists, so in a pack-free store a review-result whose body is put back to the redacted literal validates, against design item 7's "loose or any pack"
```
