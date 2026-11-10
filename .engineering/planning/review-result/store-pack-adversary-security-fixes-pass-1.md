---
format: aep.planning-md/3
id: review-result:store-pack-adversary-security-fixes-pass-1
kind: review-result
status: active
title: Adversary, pass 1, the store-pack security fixes
relations:
- reviews: story:pack-format-and-overlay-read
- reviews: story:pack-and-unpack-verbs
- reviews: story:validate-checks-packs
revision: 1
---
```
unit: fixes answering review-result:store-pack-security-review-1, branch review/store-pack-security; findings cover 65d6ccf24, cases committed as 3813bd04d
verdict: NEEDS-CHANGE
cases: executed 407→410, red 3
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 7 entries in the assigned adv-scratch directory, plus fixtures and build output under the shared cargo target (part 6)
needs-coordinator: decision 6 says every reader refuses a path held twice with different bytes, naming both places. The read verbs list or show it as missing and exit 0 (`show` exits 1, saying the store holds no such artifact). Do you want them to refuse the way a bad pack is refused?
```

The fixes leave two problems. Readers don't refuse a path held twice with different bytes as decision 6 asks. And the decision-4 change breaks a working setup: `safe.directory` given in the environment.

## 1. `git --no-pager diff --stat 65d6ccf24 HEAD`

```
 .../aep-cli/tests/store_pack_security_adversary.rs | 432 +++++++++++++++++++++
 1 file changed, 432 insertions(+)
```

The only change is one new test file. No source file was edited.

## 2. Cases added

All three are in `crates/edge/aep-cli/tests/store_pack_security_adversary.rs` and all are red now. Each was run alone first with `cargo.sh test --locked -p aep-cli --test store_pack_security_adversary`. Excerpts are verbatim, with the scratch path shortened to `<target-tmp>`.

- **`read_verbs_refuse_a_path_two_packs_hold_with_different_bytes_naming_both_packs`**
  - **Setup:** a second pack holds `planning/story/closed-a.md` with other bytes. Both packs pass their digest and name checks, there is no loose copy, and `validate` reports `pack_path_held_twice`.
  - **Asserts:** `list`, `show` and `history` each exit 1 and name both packs.
  ```
  `list` exit Some(0), names 4359470ae7d2ee74.pack: false, names eb27bf739b438705.pack: false:
  story:closed-b  story  archived  Closed B
  story:open-c    story  active    Open C
  warning: 1 planning document(s) could not be read and are missing from this answer; run `aep plan artifact validate`
  ---
  `show story:closed-a` exit Some(1), names 4359470ae7d2ee74.pack: false, names eb27bf739b438705.pack: false:
  warning: 1 planning document(s) could not be read and are missing from this answer; run `aep plan artifact validate`
  error: <target-tmp>/store-pack-security-adversary-two-packs-differ/.engineering/planning holds no `story:closed-a`; it would be at story/closed-a.md
  ---
  `history story:closed-a` exit Some(0), names 4359470ae7d2ee74.pack: false, names eb27bf739b438705.pack: false:
  1 journal line(s) could not be read and are not counted below
  2026-09-28T10:00:00Z  human:tester  test_result recorded from task check (revision 3)
  ```
- **`validate_still_reports_a_rewritten_evidence_file_where_safe_directory_comes_from_the_environment`**
  - **Setup:** a loose store with no pack and one committed evidence file edited. Git is told the repository belongs to another user (`GIT_TEST_ASSUME_DIFFERENT_OWNER=1`) and to trust it anyway through `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=safe.directory`, `GIT_CONFIG_VALUE_0=*`.
  - **Preconditions:** `validate` without that environment refuses the edit, and plain `git` reads the repository with it.
  - **Asserts:** `validate` still reports the edit with that environment.
  ```
  1 problem(s):
    - [pack_record_not_committed_bytes] <target-tmp>/store-pack-security-adversary-env-safe-directory/.engineering/packs: provenance could not be checked: a repository holds the store and `git` cannot read it (an ownership refusal, or a damaged repository)
  ```
  The line "differs from its committed blob" is missing: the check that existed before packs was skipped without a word.
- **`validate_admits_a_clean_store_without_packs_where_safe_directory_comes_from_the_environment`**
  - **Setup:** the same environment on a clean store with no pack.
  - **Asserts:** `validate` exits 0, as it does without the environment.
  ```
  a clean store without packs is refused where safe.directory is in the environment: […]
    - [pack_record_not_committed_bytes] <target-tmp>/store-pack-security-adversary-env-safe-directory-clean/.engineering/packs: provenance could not be checked: a repository holds the store and `git` cannot read it (an ownership refusal, or a damaged repository)
  ```

## 3. Suite run, after the cases existed

The script `adv-scratch/suite.sh` ran these two commands through `cargo.sh`:
- `test --locked -p aep-cli --no-fail-fast` with these `--test` binaries: store_pack_security_adversary, store_pack_security, store_pack, store_pack_adversary, store_pack_overlay, store_pack_overlay_adversary, store_pack_overlay_adversary_2, validate_packs, validate_packs_adversary, validate_packs_adversary_2, git_store, planning_cli, doctor_cli, cross_member_relations, store_migrate_git.
- `test --locked -p aep-backend-markdown --no-fail-fast --lib --test pack --test pack_adversary --test assembly`.

```
cross_member_relations          test result: ok. 10 passed; 0 failed
doctor_cli                      test result: ok. 11 passed; 0 failed
git_store                       test result: ok. 26 passed; 0 failed
planning_cli                    test result: ok. 114 passed; 0 failed
store_migrate_git               test result: ok. 30 passed; 0 failed
store_pack                      test result: ok. 15 passed; 0 failed
store_pack_adversary            test result: ok. 10 passed; 0 failed
store_pack_overlay              test result: ok. 8 passed; 0 failed
store_pack_overlay_adversary    test result: ok. 5 passed; 0 failed
store_pack_overlay_adversary_2  test result: ok. 7 passed; 0 failed
store_pack_security             test result: ok. 13 passed; 0 failed
store_pack_security_adversary   test result: FAILED. 0 passed; 3 failed
validate_packs                  test result: ok. 21 passed; 0 failed
validate_packs_adversary        test result: ok. 6 passed; 0 failed
validate_packs_adversary_2      test result: ok. 5 passed; 0 failed
error: 1 target failed:
    `-p aep-cli --test store_pack_security_adversary`
exit 101
aep-backend-markdown lib        test result: ok. 95 passed; 0 failed
assembly                        test result: ok. 12 passed; 0 failed
pack                            test result: ok. 17 passed; 0 failed
pack_adversary                  test result: ok. 2 passed; 0 failed
exit 0
```

"Before" is 407: this same run without the new binary, which is separate. "After" is 410. All three red cases are the new ones; every existing case is green. After the red runs, `cargo fmt` changed only whitespace in the new file.

## 4. Findings (against 65d6ccf24)

1. **Readers drop a doubly-held path instead of refusing it.** `crates/edge/aep-cli/src/planning.rs:790`, NEEDS-CHANGE, introduced.
   - **Measured:**
     - `Overlay::read` now fails for such a path, as decision 6 asks.
     - `MarkdownStore::load` records that as an ordinary "cannot be read" failure, not a pack failure. So `open_plan_reporting` passes it to `warn_unclean`, which prints a count and keeps going.
     - Result: `list` exits 0 without the story, and `show` says the store "holds no" story.
     - `history` (`journal.rs:744`, `read_entry`/`document_entries` with `.ok()??`) exits 0, keeps the evidence and drops all of the story's moves.
     - No reader names either pack.
   - **What reaches it:** the design's own two-branch merge resolution, which the reviewer's redaction-merge case builds with the real verbs.
   - **Fix to name:** have `load` mark a doubly-held path as a pack failure, so the existing check at `planning.rs:782` refuses it with the message `read` already writes. Have `history_git` refuse it too.
2. **Removing `GIT_CONFIG_*` from every planning `git` breaks env-given `safe.directory`.** `crates/edge/aep-cli/src/planning/git_record.rs:20`, NEEDS-CHANGE, introduced.
   - **Measured:**
     - The fix sends every `git` call in `git_record` through `isolated_git()`. That includes `changed_evidence`, `committed_versions` and `first_committed`, which existed before packs.
     - With `safe.directory` in the environment, `validate` stops reporting a rewritten evidence file.
     - It also refuses a clean store that has no pack with `pack_record_not_committed_bytes`.
     - Origin was read from the code, not run: `git show 98fbc56f8:…/git_record.rs` calls plain `Command::new("git")`.
   - **What reaches it:** a container or CI job that trusts a checkout mounted from the host by putting `safe.directory` in the environment. Git counts that as command-line config and honours it; the probe confirmed plain `git` reads the repository.
   - **Fix to name:** for `git_record`, remove only the variables that change which repository is read (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_CEILING_DIRECTORIES`, …), and keep `GIT_CONFIG_*` or at least `safe.directory`. Or record this as an accepted cost of decision 4.

## 5. Attacked and could not break

- **Decision 1+2:** every caller of `Overlay::packs`, `PackFile::pack`, `Pack::parse`, `holders`, `parsed_packs`, the driver plan load, the specification reader, the journal and redaction readers, and doctor. The unverified `.pack()` is read only by `validate` (`packed_store_paths`, `pack_problems`, `pack_lost`).
- **Length+mtime cache:** a pack rewritten in place with its length and mtime restored is served from the cached copy, which was already verified. Those bytes are stale, but not unverified.
- **Decision 3:** `parsed_packs` refuses the whole verb before planning, for an id, `--from` and `--all` alike.
- **Hook regression for decision 4:** `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE` from a hook. Before the fix, `git -C <dir>` with a hook's `GIT_DIR` read the wrong directory, so removing them helps. `changed_evidence` diffs the work tree against HEAD, so the index does not matter. Code read only, no case.
- **Decision 5:**
  - A link at `planning`, `evidence`, `planning/<kind>`, `evidence/<kind>` or `evidence/<kind>/<name>` is either refused or kept untouched by `loose_files`.
  - `unpack`'s `check_target` refuses a link on any segment of the path.
  - `stays_in_store` can only trigger in a race, after the pack is already written.
- **Decision 6:** `unpack --from` still resolves two packs; `validate` still reports `pack_path_held_twice`. The error text names store paths only, with no literal in it.
- **Decision 7:** the existing v5 cases are green; `set_version` replaces only the version string.
- **Decision 8:** `create_new`, retry on a taken name, and the temporary removed on a failed write or rename (code read). The unit tests for this are in the `aep-cli` lib, which this run did not include.

## 6. Paths written outside the worktree

- In `<scratch>/pack/units/security-review/adv-scratch/`:
  - `commit-msg.txt`
  - `probe-owner/` (a `git init` repository)
  - `red-alone.log`
  - `red-alone-3.log`
  - `suite.sh`
  - `suite-cli.log`
  - `suite-backend.log`
- `<scratch>/target/`: shared build output, which also rebuilt `CARGO_BIN_EXE_aep`.
- `<scratch>/target/tmp/store-pack-security-adversary-*`: test fixtures. The suite run also recreated the other binaries' existing fixtures there.
- `<scratch>/tmp/`: the `TMPDIR` that `cargo.sh` sets.

The lease was released with `session-end`.

## 7. Findings block

```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 790
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a path two verified packs hold with different bytes is turned into a warned-about missing document, so list and history exit 0 without it and show says the store holds no such artifact, naming neither pack
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 20
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: removing GIT_CONFIG_* from every planning git call drops safe.directory given in the environment, so validate skips the existing rewritten-evidence check and refuses a clean store with no pack
```
