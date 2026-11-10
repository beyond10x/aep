---
format: aep.planning-md/3
id: review-result:store-pack-adversary-pack-verbs-pass-2
kind: review-result
status: active
title: Adversary, pass 2, story:pack-and-unpack-verbs
relations:
- reviews: story:pack-and-unpack-verbs
revision: 1
---
unit: story:pack-and-unpack-verbs, branch impl/pack-and-unpack-verbs. Findings cover ed90850c3; my cases are commit c43352f45.
verdict: NEEDS-CHANGE
cases: executed 86→89, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 23 paths (part 6)
needs-coordinator: whether a store with Git filters on its files (git-crypt, LFS) is in scope. If it is, finding 2 should go up from INFEASIBLE.

Three new cases, all red. The worst one: on a store whose committed `.gitattributes` checks files out with CRLF (or a `core.autocrlf=true` clone), `pack` then `unpack --all` leaves every restored file modified in `git status` and not byte for byte. That breaks a line of the acceptance directly.

**1. Diff stat** (`git --no-pager diff --stat ed90850c3 HEAD`)
```
 crates/edge/aep-cli/tests/store_pack_adversary.rs | 143 ++++++++++++++++++++++
 1 file changed, 143 insertions(+)
```
Only a test file changed. I formatted it with `rustfmt` on that file alone, not `cargo fmt -p`, so no source file was touched.

**2. Cases added** at the end of `crates/edge/aep-cli/tests/store_pack_adversary.rs`. Each was run alone before the suite. All are red now.

| case | asserts | red line, from the run alone |
|---|---|---|
| `unpack_all_after_pack_on_an_eol_crlf_store_leaves_git_status_empty_and_the_bytes_restored` | The committed `.gitattributes` is `*.md text eol=crlf`, `*.json text eol=crlf`, and the work tree is checked out CRLF and clean. After `pack` then `unpack --all`, `git status --porcelain` must be empty and `.engineering` byte-identical. | `:589` `the round trip leaves Git's view unchanged`, `left: " M .engineering/evidence/story/closed-a/20260928T100000Z-000-424990581920.json\n M …/20260928T110000Z-000-773446777833.json\n M .engineering/planning/story/closed-a.md\n M .engineering/planning/story/closed-b.md\n"`, `right: ""` |
| `pack_leaves_every_record_the_store_reads_unchanged_when_git_filters_the_working_files` | A `filter=rot13` clean/smudge on `planning/**` and `evidence/**` stands in for git-crypt or transcrypt. The blob is encoded and the clean work tree is plain. What the overlay reads for every record must be the same before and after `pack`. | `:641` `the store reads every record as it did before pack`. The left side bytes decode to `{\n  "ng": "2026-09-28G10:00:00M"…` (rot13), the right side to `{\n  "at": "2026-09-28T10:00:00Z"…` |
| `unpack_refuses_a_packs_directory_that_is_a_symbolic_link` | After `pack`, `.engineering/packs` is moved outside the repository behind a symlink. `unpack story:closed-a` must exit 1 and write nothing there. | `:662` `left: Some(0) right: Some(1)`. Output: `rewrite fd6afda100ddb8f2 -> 0d02642b0d8c9bf9 … restored 3 file(s); rewrote 1 pack(s)` |

**3. Suite run**, after the cases existed. `cargo.sh test --locked --no-fail-fast -p aep-cli --test store_pack --test store_migrate_git --test store_pack_overlay --test git_store --test store_pack_adversary`, exit 101:
```
git_store:            test result: ok. 26 passed; 0 failed
store_migrate_git:    test result: ok. 30 passed; 0 failed
store_pack:           test result: ok. 15 passed; 0 failed
store_pack_adversary: test result: FAILED. 7 passed; 3 failed
store_pack_overlay:   test result: ok. 8 passed; 0 failed
```
- **Counts:** 89 after. 86 before is the same run minus my three cases, which matches pass 1's "after".
- **Pass-1 cases:** all 7 are green, so the fix holds for F1, F2 and F4–F6.
- **Re-pin ed90850c3:** it is a re-pin, not a relaxation.
  - closed-d, closed-e and closed-f are still in both kept packs after `unpack a` and `unpack b`. So `/6` is correct there, and pass 1's `/5` assertion was my mistake.
  - Every original assertion stays, including that both packs parse. `unpack --all` then `/5` is added.
  - It is green. So with `-merge`, the hand resolution (keep both new packs, delete the old one) does recover.

**4. Findings** (they cover ed90850c3)

- **1. The round trip is not clean when Git converts line endings**
  - `crates/edge/aep-cli/src/store_command/unpack.rs:286` (and `pack.rs:680`): NEEDS-CHANGE, introduced, warning.
  - **What was measured:** `pack` now packs the LF committed blob and removes the CRLF working file. `unpack` writes the LF bytes back. Git sees the size change against the index and reports each restored file as modified. The bytes are also not the ones `pack` removed.
  - A probe on a `core.autocrlf=true` clone gave the same four ` M` lines, with different sha256 before and after (`scratch/p2-crlf.sh`).
  - **What reaches it:** any clone with `core.autocrlf=true` (Git for Windows' default) or a committed `eol=crlf`/`text` attribute. That is the same reach that made pass-1 F3 worth fixing.
  - **Origin:** introduced by 8f6f231c3. Before it, these files were kept as "differs from HEAD", as pass 1's red autocrlf run showed.
  - **Fix:**
    - In `unpack`, write what `git checkout` would write. For example, put the record into Git with `git hash-object -w --no-filters --stdin`, then write the output of `git cat-file --filters --path=<prefix><path> <oid>`.
    - Or have `pack` keep a loose file whose raw bytes differ from its blob.
- **2. A filter that encodes files makes the store read encoded records**
  - `crates/edge/aep-cli/src/store_command/pack.rs:664` (with `:378` and `:843`): INFEASIBLE, introduced, warning.
  - **What was measured:**
    - Eligibility and the re-check before removal both compare through Git's clean filter. So a plain working file whose clean form is the blob counts as unchanged.
    - `pack` packs the encoded blob and deletes the plain file. After that the store reads encoded records: a probe got `show story:closed-a` exit 1, and `validate` reported `missing field id` and `not an evidence record`.
    - `unpack --all` restores the encoded bytes and `git status` shows 4 ` M`.
    - Nothing is lost from Git, but the store is unreadable until someone repairs it by hand.
  - **What reaches it:** git-crypt, transcrypt or LFS covering `.engineering` files. I found nobody doing that, which is why it is INFEASIBLE.
  - **Fix:** keep any candidate with a `filter` or `ident` attribute (`git check-attr filter ident --`) and give the reason, e.g. "filtered by Git". Packing the plain bytes instead would commit git-crypt-protected text unencrypted under `packs/`.
- **3. `unpack` writes through a symlinked packs directory**
  - `crates/edge/aep-cli/src/store_command/unpack.rs:234`: INFEASIBLE, introduced, note.
  - **What was measured:** `unpack` has no check on `.engineering/packs` like the one the fix added to `pack`. It reads, rewrites and deletes pack files through a symlink to outside the work tree. The records restored inside the tree are intact.
  - **What reaches it:** nothing found. `pack` refuses to create this layout, so only a hand-made or committed symlink reaches it.
  - **Fix:** run `require_work_tree`'s symlink and containment check on `unpack` too.

**5. Attacked and could not break**
- The re-check before deleting a loose file compares its filtered hash with the blob id it decided on (`pack.rs:843`), so a file edited mid-run is kept.
- A `core.autocrlf=true` clone packs the same bytes as a plain clone (pass-1 case, green).
- Interrupted `unpack --all` reruns to `/5`, and the packs directory and attributes are removed (pass-1 case, green).
- Records restored through a symlinked `planning/<kind>` or `planning` are refused by `check_target`.
- A symlinked `.engineering` or `.engineering/packs` on `pack` is refused, and a `store: sqlite` selector is refused by both verbs.
- Committed mode 100755 is kept with the reason `mode 100755`.

**6. Paths written outside the worktree**

Under `<scratch>/pack/units/pack-and-unpack-verbs/scratch/`:
- **probe scripts and copies:** `p2-mkstore.sh`, `p2-filter.sh`, `p2-crlf.sh`, `p2-cases.rs`, `p2-commit-msg.txt`, `p2-aep` (a copy of the binary)
- **probe stores:** `p2-filter/`, `p2-crlf-origin/`, `p2-crlf-clone/`, `p2-eol/`
- **logs and snapshots:** `p2-build.log`, `p2-suite.log`, `p2-crlf-before.txt`, `p2-crlf-after.txt`, `p2-eol-before.txt`, `p2-eol-after.txt`, and 3 `p2-case-*.log` files

Under `<scratch>/target/tmp/`, written by my cases: `store-pack-adv-eol-crlf`, `store-pack-adv-filtered`, `store-pack-adv-unpack-packs-symlink`, `store-pack-adv-unpack-packs-symlink-outside`.

The lease `aep-adv-pack-verbs-2` is released.

**7. Findings block**
```findings
- file: crates/edge/aep-cli/src/store_command/unpack.rs
  line: 286
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: on an eol=crlf or core.autocrlf=true work tree pack removes CRLF working files and unpack writes the LF blob bytes back, so the round trip is not byte for byte and git status reports every restored file modified
- file: crates/edge/aep-cli/src/store_command/pack.rs
  line: 664
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: with a clean/smudge filter on store files, eligibility through Git's filters packs the encoded committed blob and deletes the plain working file, so the store reads encoded records after pack and unpack restores encoded bytes
- file: crates/edge/aep-cli/src/store_command/unpack.rs
  line: 234
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: unpack has no symlink or containment check on .engineering/packs and rewrites and deletes pack files outside the work tree through a symlinked packs directory
```
