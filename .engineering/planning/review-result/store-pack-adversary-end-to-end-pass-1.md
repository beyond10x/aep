---
format: aep.planning-md/3
id: review-result:store-pack-adversary-end-to-end-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:pack-end-to-end-and-docs
relations:
- reviews: story:pack-end-to-end-and-docs
revision: 1
---
unit: story:pack-end-to-end-and-docs, covering head 3af04112f plus my case at cd55a7dcb on impl/pack-end-to-end-and-docs (tree wt-0dc9ce350bfb)
verdict: green: no red case; strongest finding NEEDS-CHANGE (warning, docs once the rev 5 fix lands)
cases: executed 1→2, red 0
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 3 (the scratch directory, one test project directory, build outputs in the shared target; listed in part 6)
needs-coordinator: yes. Two docs sentences have to change when the rev 5 security unit merges; that unit is not in this tree.

## 1. Diff stat (cd55a7dcb against 3af04112f)

```
 .../tests/store_pack_end_to_end_adversary.rs       | 518 +++++++++++++++++++++
 1 file changed, 518 insertions(+)
```
Only a test file changed. No source, docs or store file was edited.

## 2. Case added

`crates/edge/aep-cli/tests/store_pack_end_to_end_adversary.rs`, `every_compared_answer_is_a_real_answer_and_the_closed_ids_it_names_are_really_packed`. It is **green** at HEAD.

The store holds:
- a redacted archived review-result, redacted after it was committed;
- a `review_outcome` record;
- closed stories that decompose an open epic;
- an open story that depends on a closed one and has two archived reviews.

It asserts:
- Every compared call exits 0 before packing and prints output. `show`, `explain` and `findings` name the id; `graph` names the packed and open nodes.
- After pack, each closed planning file and each of its evidence files is absent from the tree and present in the pack as `--- <path> `. The open files stay loose.
- The tracked count equals the loose count minus the closed files (counted from the fixture, not from pack's output) plus 2.
- `project.yaml` is `/6`.
- After `unpack --all`, `git ls-files -s .engineering` matches the loose store blob for blob.
- `validate`, `list`, `list --status implemented`, `graph`, and `show`/`history`/`explain`/`findings` for 6 ids give identical stdout and exit codes at all three points, both in default format and with `--format json`.

The first run was red because of my own typo: `graph` has no `text` format, so it exited 2. That is not a finding. After the fix the case alone gave:
```
test every_compared_answer_is_a_real_answer_and_the_closed_ids_it_names_are_really_packed ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.41s
```

**How I checked that the existing test cannot catch a partial pack.** I did not mutate the source. Instead I built the state a buggy pack would leave, with the feature build, in `adv-scratch/probe5.sh`. An `ident` attribute on the closed story the open one depends on makes `pack` keep it loose without changing a byte. The probe's output:
```
  keep evidence/story/impl-1/20260927T100000Z-000-2217763393dd.json: filtered by Git (ident)
  keep planning/story/impl-1.md: filtered by Git (ident)
before=6 removed=2 after=6 expected=6
0 validate -> same / 0 list --status implemented -> same / 0 graph -> same
0 history story:impl-1 -> same / 0 explain story:impl-1 -> same / 0 findings story:impl-1 -> same
```
The equation and all answers still hold. Applied to the end-to-end store, leaving `impl-0000..0039` and their evidence loose gives removed=2073 (≥1000), after-pack 163 = 2234−2073+2, and 163 < 223. Every assertion stays green, and `story:impl-0001`, a compared id and the target of the open-story edges, is never packed. My case's check "`planning/story/impl-a.md` is absent and in the pack" fails in that state.

## 3. Suite run (after the case existed, on the committed file)

```
.../cargo.sh clippy --locked -p aep-cli --tests -- -D warnings      -> Finished ... exit 0
.../cargo.sh test --locked -p aep-cli --test store_pack_end_to_end --test store_pack_end_to_end_adversary
test a_thousand_closed_stories_pack_and_unpack_with_every_answer_unchanged ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 53.82s
test every_compared_answer_is_a_real_answer_and_the_closed_ids_it_names_are_really_packed ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.09s
exit 0
```
The first clippy run failed on my file (`too_many_lines` and a file-extension comparison). I fixed both before committing.

## 4. Findings (they cover 3af04112f)

| # | file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|---|
| 1 | `crates/edge/aep-cli/tests/store_pack_end_to_end.rs:442` (also :447, :450, :410) | The removal count comes from pack's own `  remove ` lines and the bounds are loose (`>= 1000`, `< loose/10`). Nothing checks that a compared closed id is in the pack and gone from the tree, and only `validate`'s exit 0 is checked before packing. A pack that leaves the depended-on closed stories loose passes, and then the open-to-packed edges named in the acceptance are never exercised. | CONFIRMED (closed by my case) | introduced | Any change to pack's choice of what to pack, for example skipping artifacts that have inbound edges. Demonstrated in part 2. |
| 2 | `website/docs/concepts/planning-store.md:139-140`, `docs/design/git-native-planning-store-v0.1.md:427` and the § 15 `validate` bullet | These say a path held loose and packed with different bytes is named by `validate`. That is true at HEAD: I measured that readers silently serve the loose copy and `validate` reports `pack_path_held_twice`. Design rev 5 makes every read verb refuse such a store, and refuse a pack whose digest or name fails. Once that lands the docs leave out that `list`, `show` and the rest refuse until `unpack <id>` runs. They also never say a pack must not be edited by hand. At HEAD every verb already refuses a pack that does not parse (`store_pack_overlay_adversary_2.rs`). | NEEDS-CHANGE | introduced | A Git merge (the docs' own merge section) or a hand edit of a pack, which the design invites by keeping packs as readable text. |
| 3 | `docs/design/git-native-planning-store-v0.1.md:392` | Says a pack holds "every evidence file under its `evidence/<kind>/<name>/` directory" and that only untracked or changed files stay loose. `pack.rs:641` keeps non-`.json` files, `:743` files under a Git filter or `ident` attribute, `:745` modes other than 100644, and `:533` symbolic links. | CONFIRMED | introduced | A hand-placed attachment or an executable evidence file. Rare. |
| 4 | `website/docs/reference/planning-stores.md:19` | The generated "Supported" table, which the page calls authoritative, lists Git-native as `aep.project/5` only. The new prose under it says a packed store is `/6`. `store_catalog.rs` was never changed, though the brief allowed the catalog route. | CONFIRMED | pre-existing | A reader checking the table for `/6`. The catalog is identical at 285739349, where `/6` is already read. |

## 5. What I attacked and could not break

- **JSON forms:** `validate`, `list`, `graph`, `history`, `explain`, `findings` and `show` with `--format json`, plus `list --format yaml`, are identical at all three points.
- **Store contents:** the redaction replay, `review_outcome` records, packed stories pointing at an open epic, and a closed story with a dangling edge (`validate` exit 1, identical output) all stay byte-identical.
- **`--store .engineering/planning`:** still sees the packs.
- **The documented flow:** `validate` exits 0 after pack and after unpack, whether committed or not.
- **Writes and gates:** moving an open story to `implemented` and an epic whose child is packed works. So does evidence on a packed id followed by a second pack and `unpack <id>` across two packs; `history` is unchanged.
- **Docs against code:** CLI `--help` for `pack` and `unpack` matches the docs. The refusal list, `.gitattributes` contents, version swap, `--dry-run` contents and `--from` all match the code.
- **Committed bytes:** no private path or real redaction literal in any committed byte.

## 6. Paths written outside the worktree

- `<scratch>/pack/units/pack-end-to-end-and-docs/adv-scratch/`, all of it: probe scripts `probe.sh` and `probe2.sh` to `probe5.sh`, probe stores `probe-store` and `probe2-store` to `probe5-store`, output directories `probe-*` and `p3-*`/`p5-*`, the `*.out`, `*.log` and `body*.md` files, and `commit-msg.txt`.
- `<scratch>/target/tmp/store-pack-end-to-end-adversary/` (my test's project). My suite run also re-created the implementor's `.../target/tmp/store-pack-end-to-end/`.
- Build outputs in the shared target, including `.../aep/target/debug/deps/store_pack_end_to_end_adversary-37fbe485db59640e`, plus cargo.sh's shared `TMPDIR` `.../aep/tmp`.

The lease `aep-adv-e2e` is released. No build or test of mine is still running.

## 7. Findings block

```findings
- file: crates/edge/aep-cli/tests/store_pack_end_to_end.rs
  line: 442
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the end-to-end test counts removals from pack's own output with loose bounds and never checks a compared closed id is packed, so a pack leaving the depended-on closed stories loose passes every assertion
- file: website/docs/concepts/planning-store.md
  line: 139
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the docs say only validate names a path held twice with different bytes, but after design rev 5 every read verb refuses that store and any pack whose digest or name fails, and the docs never say a pack must not be edited by hand
- file: docs/design/git-native-planning-store-v0.1.md
  line: 392
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: section 15 says a pack holds every evidence file in the artifact's directory, but pack keeps non-json files, Git-filtered or ident files, non-100644 modes and symbolic links loose
- file: website/docs/reference/planning-stores.md
  line: 19
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the generated Supported table lists the Git-native store as aep.project/5 only while the prose under it says a packed store is aep.project/6
```
