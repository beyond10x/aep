---
format: aep.planning-md/3
id: review-result:store-pack-adversary-pack-verbs-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:pack-and-unpack-verbs
relations:
- reviews: story:pack-and-unpack-verbs
revision: 1
---
unit: story:pack-and-unpack-verbs, branch impl/pack-and-unpack-verbs, findings cover 81b53743f (cases committed as f654107a5)
verdict: NEEDS-CHANGE
cases: executed 79→86, red 6
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 22 paths (part 6)
needs-coordinator: the fix for F2 changes the packs `.gitattributes` text, which was your decision (`*.pack -text`)

Six of my seven new cases fail against the unit's commit. The two that matter most: an interrupted `unpack --all` leaves the store stuck at `aep.project/6` with no pack to remove, and the conflict you already know about writes conflict markers into the pack files themselves.

**1. Diff stat** (`git --no-pager diff --stat 81b53743f HEAD`)
```
 crates/edge/aep-cli/tests/store_pack_adversary.rs | 521 ++++++++++++++++++++++
 1 file changed, 521 insertions(+)
```
I only added a test file. No non-test path changed.

**2. Cases added** in `crates/edge/aep-cli/tests/store_pack_adversary.rs`. Each was run alone through cargo.sh before the suite; the red lines below are from those runs.

| case | asserts | now |
|---|---|---|
| `unpack_all_rerun_after_an_interrupted_unpack_returns_project_yaml_to_v5` | `.engineering` is read-only while `unpack --all` runs, so it fails writing `project.yaml` after deleting the last pack. The rerun must set `/5` and remove `packs/`. | red: `left: "version: aep.project/6" right: "version: aep.project/5"`, message `nothing to unpack: no pack under .engineering/packs` (:263) |
| `a_rename_conflict_between_two_unpacks_resolved_by_keeping_both_packs_leaves_packs_that_read` | Two branches each unpack a different artifact; the merge uses Git's default rename detection, giving a rename/rename conflict. It is resolved by keeping both new packs and dropping the old one. Both packs must parse. | red: `58eaed539309ce4d.pack does not read after the merge: [pack_record_line_malformed] the record line at byte 11 does not start with`, the pack body shows `<<<<<<<< HEAD:.engineering/packs/58eaed539309ce4d.pack` (:325) |
| `pack_on_a_clone_with_core_autocrlf_writes_the_pack_a_plain_clone_writes` | One clone with `core.autocrlf=true` and a clean `git status`. It must write the same pack as a plain clone. | red: `left: ["b2c51679666f4dda.pack"] right: []`, with every candidate shown as `keep …: differs from HEAD` (:352 at the time of that run, :358 after rustfmt) |
| `pack_refuses_a_packs_directory_that_is_a_symbolic_link_and_keeps_the_loose_files` | `.engineering/packs` is a symlink to a directory outside the repo. `pack` must exit 1 and write nothing there. | red: `left: Some(0) right: Some(1)`; `packed 4 record(s) into 9329e183883017ac.pack; removed 4 loose file(s)` (:374) |
| `pack_refuses_a_store_whose_selector_is_not_git_native` | The store is `store: sqlite`. `pack` must exit 1 and change no file. | red: `left: Some(0) right: Some(1)`, with `project.yaml: aep.project/5 -> aep.project/6` (:397) |
| `unpack_all_after_pack_leaves_git_status_empty_for_an_executable_evidence_file` | One committed evidence file has mode 100755. After `pack` then `unpack --all`, `git status --porcelain` must be empty. | red: `left: " M .engineering/evidence/story/closed-a/20260928T100000Z-000-424990581920.json\n" right: ""` (:415) |
| `pack_interrupted_while_removing_loose_files_keeps_every_record_and_the_rerun_converges` | `planning/story` is read-only during `pack`. Every record must still be held by a loose file or a pack, and the rerun must converge. | green (kept as a guard) |

The first version of the rename-conflict case passed. With only two closed stories, Git treated the change as a rename/delete and did not content-merge the packs. I added three more archived stories that both branches keep, which gives the rename/rename conflict. The red output above is from that revised case, run alone.

**3. Suite run** (after the cases existed)
`cargo.sh test --locked -p aep-cli --test store_pack --test store_migrate_git --test store_pack_overlay --test git_store --test store_pack_adversary`, exit 101:
```
git_store:            test result: ok. 26 passed; 0 failed
store_migrate_git:    test result: ok. 30 passed; 0 failed
store_pack:           test result: ok. 15 passed; 0 failed
store_pack_adversary: test result: FAILED. 1 passed; 6 failed
```
Cargo stops at the first failing binary, so `store_pack_overlay` did not run there. I ran it on its own: `test result: ok. 8 passed; 0 failed`, exit 0.

So "before" is 79 (my file left out) and "after" is 86. I did not run the `aep-backend-markdown` binaries or `cli_reference`; my change touches neither.

**4. Findings**

- **F1** `crates/edge/aep-cli/src/store_command/unpack.rs:217`: NEEDS-CHANGE, introduced, warning.
  - The early return `if packs.is_empty()` skips the switch back to `/5`, so `/6` with no pack is a dead end.
  - `migrate git` refuses `/6` with "Run `aep plan store unpack --all` first", and `unpack --all` then does nothing. The empty `packs/` directory also stays.
  - What reaches it: any crash or failed write between deleting the last pack and writing `project.yaml`, or packs deleted by hand. No record is lost.
  - Fix: when no pack remains, including at the start, switch to `/5` and remove the attributes file and the empty directory.
- **F2** `crates/edge/aep-cli/src/store_command/pack.rs:28`: NEEDS-CHANGE, introduced, warning.
  - `*.pack -text` does not stop Git from merging packs as text. On the rename/rename conflict, Git writes `<<<<<<<<` markers into both new packs.
  - Resolving with `git add` leaves packs that do not parse, and every read verb then refuses the store. It fails closed and nothing is lost, but recovery needs `git checkout --ours`/`--theirs`.
  - What reaches it: the default merge of two unpacks on any store with enough closed work, which is the merge case the design describes.
  - Recovery after hand resolution (your question): with `-text` only, `unpack` and `validate` do not recover. In a scratch probe with `*.pack -text -merge`, the same hand resolution gave packs that read; `unpack x`, `unpack y` and `unpack --all` reached `/5` and `validate` printed `valid`.
  - Fix: `*.pack -text -merge`, and `ensure_attributes_removed` must compare against the new text. `binary` would also work but turns off diffs, and the design wants packs readable in review.
- **F3** `crates/edge/aep-cli/src/store_command/pack.rs:318`: CONFIRMED, introduced, warning.
  - `hash-object --no-filters` compares the CRLF working files with the LF blobs, so a clone with `core.autocrlf=true` packs nothing while `git status` is clean.
  - This breaks the acceptance line "two clones of the same commit write the same bytes". Nothing is lost.
  - What reaches it: per-clone `core.autocrlf=true` (Git for Windows' default). I found no Windows target in the crates, so on Linux and macOS this is rare.
  - Fix: compare the filtered hash (`hash-object` without `--no-filters`) and pack the committed blob bytes (`cat-file`).
- **F4** `crates/edge/aep-cli/src/store_command/pack.rs:701`: INFEASIBLE, introduced, note.
  - `pack` writes the pack and `.gitattributes` through a symlinked `packs/` to outside the repo, then deletes the loose files. That breaks the module's own claim that every removed byte is held by the pack and by Git.
  - What reaches it: nothing found; I built the symlink myself.
- **F5** `crates/edge/aep-cli/src/store_command/pack.rs:57`: INFEASIBLE, introduced, note.
  - `Store::open` never checks the `store:` kind, so a `store: sqlite` project gets packed and switched to `/6`.
  - What reaches it: nothing found; no verb leaves Markdown files under `planning/` in a sqlite project.
- **F6** `crates/edge/aep-cli/src/store_command/pack.rs:303`: INFEASIBLE, introduced, note.
  - The mode from `ls-tree` is thrown away and the pack format holds bytes only, so an executable record loses its x bit on the round trip.
  - What reaches it: nothing found; aep writes its files 0644.

**5. Attacked and could not break**
- `pack` interrupted while removing loose files: every record stays held, and the rerun converges (guard case above).
- `unpack` interrupted while rewriting packs (read-only `packs/`): the restored files and the old pack both stay, and the rerun converges (probe).
- Staged changes, a closed artifact not closed at `HEAD`, uncommitted evidence: each is kept as untracked or "differs from HEAD" (read and probe).
- `swap_version` with a quoted value, JSON, CRLF or a comment: one line changes and the result is parsed back.
- `--from ../x` and ids containing `..`: refused by the name check and by `check_path`.
- A rewrite that comes out under an existing pack's name, and a crash between writing the pack and removing loose files: no record loss.
- Determinism: bytes, names and dry-run output have no clock, locale or ordering input.

**6. Paths written outside the worktree**
Under `<scratch>/pack/units/pack-and-unpack-verbs/scratch/`:
- probe scripts: `probe1.sh`, `probe2.sh`, `probe3.sh`, `probe4.sh`, `probe4-body.sh`, `run-each.sh`, `commit-msg.txt`, `design.md`
- probe stores: `probe1/`, `p-symlink/`, `p-symlink-outside/`, `p-crlf/`, `p-crlf-clone/`, `p-sqlite/`, `p-exec/`, `p-int/`, `p-merge/`, `p-merge2/`, `p-uint/`
- logs: `build.log`, `build-test.log`, `each.log`, `case-*.log` (8 files), `suite.log`, `suite-overlay.log`

Under `<scratch>/target/tmp/` (the shared target's test scratch, written by my cases): `store-pack-adv-*` (10 directories).

The other files already in that scratch directory (`base-*.txt`, `gate*.txt`, `*.impl` and so on) are not mine.

The lease (`aep-adv-pack-verbs`) is released.

**7. Findings block**
```findings
- file: crates/edge/aep-cli/src/store_command/unpack.rs
  line: 217
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: with no pack left at start, unpack returns before switching project.yaml to /5, so an interrupted unpack --all leaves /6 permanently and migrate git points back at the no-op unpack
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 28
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the packs gitattributes holds only -text, so Git merges packs as text and the rename/rename conflict of two unpacks writes conflict markers into both rewritten packs, which then fail to parse after hand resolution
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 318
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: comparing unfiltered working bytes with HEAD blobs makes a core.autocrlf=true clone with a clean status pack nothing, so two clones of one commit do not write the same pack
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 701
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: pack writes through a symlinked .engineering/packs to outside the repository and then deletes the loose files it packed
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 57
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: pack never checks the store selector kind and packs a sqlite-selected project, switching it to aep.project/6
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 303
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the committed file mode is discarded, so pack then unpack --all of a 100755 record leaves a mode change in git status
```
