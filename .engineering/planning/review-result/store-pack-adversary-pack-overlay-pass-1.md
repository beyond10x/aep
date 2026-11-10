---
format: aep.planning-md/3
id: review-result:store-pack-adversary-pack-overlay-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:pack-format-and-overlay-read
relations:
- reviews: story:pack-format-and-overlay-read
revision: 1
---
unit: story:pack-format-and-overlay-read, covering branch impl/pack-format-and-overlay-read at 357e6d17f plus my test commit 0243f1a57 (tree wt-6cee46e1d285)
verdict: CONFIRMED (two warnings and two notes; no blocker)
cases: executed 550→557, red 4
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 10 paths, all in the assigned scratch (part 6)
needs-coordinator: whether a corrupt pack should fail read verbs (exit 1) or keep them at exit 0 with a warning that names the pack; the test accepts either

**1. `git --no-pager diff --stat 357e6d17f HEAD`** (my commit only; the working tree is clean)
```
 .../aep-cli/tests/store_pack_overlay_adversary.rs  | 520 +++++++++++++++++++++
 .../aep-backend-markdown/tests/pack_adversary.rs   | 130 ++++++
 2 files changed, 650 insertions(+)
```
Both paths are new test files. I changed no source file.

**2. Cases added.** Each red output below is from the run of that case alone, made before the suite ran.

`crates/plan/aep-backend-markdown/tests/pack_adversary.rs`
- `every_record_a_pack_parses_with_is_listed_by_the_overlay` (**red**): every record the parser accepts must show up in the overlay's listings. If it doesn't, no reader ever reaches it.
  ```
  `planning/story/.hidden.md` parsed, and `planning/story` does not list `.hidden.md`: the record is lost to every reader that walks the store
  ```
- `a_pack_that_parses_is_the_bytes_render_writes_for_its_records` (**red**): a pack the parser accepts must be byte-for-byte what `render` writes for those records.
  ```
  assertion `left == right` failed: parse admitted `05` as a length, so pack da01111410e5040d holds the set pack 11e5ecb302b0dfe3 holds
  ```

`crates/edge/aep-cli/tests/store_pack_overlay_adversary.rs`
- `a_store_whose_every_artifact_is_packed_is_still_a_store_without_a_planning_directory` (**red**). The first run failed on my own fixture (an empty `git commit`), not on the claim. I removed that line and re-ran the case alone. `list` passes; `doctor` fails:
  ```
  doctor reports a store held in .engineering/packs/cfe95f00a872bbf8.pack as absent: ...
  fail  planning-store: there is no planning store at ./.engineering/planning
  ```
- `a_pack_that_does_not_parse_is_named_by_the_read_verbs` (**red**). The pack's line ends are rewritten to CRLF, as `core.autocrlf` would do on checkout:
  ```
  `list` exits 0 over a pack that does not parse and never names it: warning: 1 planning document(s) could not be read and are missing from this answer; run `aep plan artifact validate`
  ```
  stdout is empty. The loose `story:open-c` is hidden too.
- `findings_answer_the_same_when_the_review_results_are_packed_too` (green)
- `validate_says_the_same_before_and_after_the_closed_artifacts_are_packed` (green)
- `a_new_review_of_a_packed_story_writes_only_the_review` (green)

**3. Suite run** (`scratch/adv/suite.sh`, run after the cases existed; `--no-fail-fast`)
```
$ cargo.sh test --locked -p aep-backend-markdown --lib --test pack --test pack_adversary --test git_layout --test evidence --test history --test conformance
lib 95 ok; conformance 11 ok; evidence 13 ok; git_layout 11 ok; history 1 ok; pack 17 ok
pack_adversary: test result: FAILED. 0 passed; 2 failed
exit 101
$ cargo.sh test --locked -p aep-domain --lib
test result: ok. 323 passed; 0 failed
exit 0
$ cargo.sh test --locked -p aep-cli --test git_store --test store_migrate_git --test doctor_cli --test record_redaction_adversary --test store_pack_overlay --test store_pack_overlay_adversary
doctor_cli 11 ok; git_store 26 ok; record_redaction_adversary 4 ok; store_migrate_git 30 ok; store_pack_overlay 8 ok
store_pack_overlay_adversary: test result: FAILED. 3 passed; 2 failed
exit 101
```
- **Where 550 comes from:** the same run with my two binaries left out (148 + 323 + 79).
- **Clippy:** the first `clippy -p aep-backend-markdown -p aep-cli --tests -D warnings` failed on a leftover doc comment in my file. I fixed it and clippy is now clean. `cargo fmt` changed only my two files.
- **After formatting:** I re-ran both adversary binaries and still got 4 red, 3 green.

**4. Findings**

1. **`doctor` says a fully packed store is absent** (`crates/edge/aep-cli/src/doctor.rs:565`). CONFIRMED, introduced, warning.
   - **What was measured:** `absent()` checks `root.is_dir()`. When every artifact is packed, `.engineering/planning/` is empty, and a fresh clone has no such directory because Git keeps no empty directories. `doctor` then fails with "there is no planning store", while `list` answers from the pack.
   - **What reaches it:** a store whose open artifacts have all closed and been packed, followed by `git clone`. The check is old, but this unit is the first to make packed-only stores possible, so I count it as introduced.
   - **Fix:** use the overlay (or the packs directory) in `absent`.

2. **A pack that does not parse hides the whole store, and the warning undercounts** (`crates/plan/aep-backend-markdown/src/store.rs:131` and `crates/edge/aep-cli/src/planning.rs:7195`). CONFIRMED, introduced, warning.
   - **What was measured:** `MarkdownStore::load` returns as soon as one pack fails, so loose documents are hidden too. `warn_unclean` then prints "1 planning document(s) could not be read" without naming the pack, `list` exits 0 with empty output, and the journal readers' `unwrap_or_default` returns an empty history.
   - **What reaches it:** CRLF conversion of a `.pack` text file (`core.autocrlf=true`, or no `.gitattributes` entry), or a hand edit.
   - **Fix:** name the pack in the warning, or fail read verbs on a corrupt pack. A `.gitattributes` entry `*.pack -text` would block the CRLF route.

3. **The path grammar accepts names that readers skip** (`crates/plan/aep-backend-markdown/src/pack.rs:214`, with the skip at `:745` and `:769`). INFEASIBLE, introduced, note.
   - **What was measured:** `check_path` accepts dot-prefixed segments (`planning/story/.hidden.md`, `evidence/story/.x/r.json`), but `Overlay::children` skips dot-names. The record parses, and later `validate` will pass it, yet no reader sees it and nothing reports an error.
   - **What reaches it:** nothing I found. I built the pack by hand; the future `pack` verb packs real ids, which never start with a dot.
   - **Fix:** refuse a leading `.` in `check_path`.

4. **The parser accepts a non-canonical length** (`crates/plan/aep-backend-markdown/src/pack.rs:384`). INFEASIBLE, introduced, note.
   - **What was measured:** a length of `05` is accepted, so two different pack files (with different names) hold the same set of records.
   - **What reaches it:** only a hand-written pack.
   - **Fix:** refuse a leading zero unless the length is exactly `0`.

5. **Two different guesses for where packs live** (`crates/plan/aep-backend-markdown/src/store.rs:87`). NEEDS-CHANGE, introduced, note. This one is a judgement call with no test case.
   - `MarkdownStore::open(root)` without `with_evidence` falls back to `Overlay::beside_planning`, which decides by the directory name `planning`.
   - The CLI's `evidence_beside` (`planning.rs:252`) decides by whether the parent is `.engineering`.
   - For `--store /x/planning` outside a project, the two put packs in different places: artifact verbs look in `/x/planning/packs`, while `graph_at` (`aep backend … --planning`), `specification` and workspace `Assembly::read` look in `/x/packs`.
   - **What reaches it:** an explicit store path named `planning` outside `.engineering`, which is rare.
   - **Fix:** one derivation, shared by both.

**5. Attacked and not broken**
- Packed archived review-results: `findings`, `review-value`, `show`, `history` and `explain` give identical output.
- `validate` gives identical stdout and exit code before and after packing (`files_read` holds).
- `new review-result --relate reviews:<packed>` writes only the new file.
- By reading the parser:
  - length-prefixed records survive `\n--- ` inside record bytes;
  - CRLF, uppercase hex, trailing garbage, a missing header, a huge length (no overflow, no allocation), and duplicate or unsorted paths are all refused with codes.
- The commit_batch refusal comes before `PENDING_BATCH` is written; the unit's test checks the whole tree, so that file would show up.
- Every source file that checks `aep.project/5` accepts `/6` where it should.

**6. Paths written outside the worktree** (all under `<scratch>/pack/units/pack-format-and-overlay-read/scratch/adv/`):
`build.log`, `red-backend.log`, `red-cli.log`, `red-cli-2.log`, `suite.sh`, `suite.log`, `helpers.rs.txt`, `fixture.rs.txt`, `tail.rs.txt`, `commit-msg.txt`.

Builds and tests also wrote into the shared target `<scratch>/target` (test fixtures under its `tmp` directory) and the `TMPDIR` that `cargo.sh` sets, `<scratch>/tmp`. I took and released the lease `aep-adv-pack-overlay`.

**7. Findings block**
```findings
- file: crates/edge/aep-cli/src/doctor.rs
  line: 565
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: doctor reports "there is no planning store" for a store whose every artifact is packed and whose empty planning directory a clone does not have, while list answers from the pack
- file: crates/plan/aep-backend-markdown/src/store.rs
  line: 131
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: one pack that does not parse (e.g. CRLF-converted) makes load return before any loose document, so list exits 0 with empty output and a warning that counts one missing document and never names the pack
- file: crates/plan/aep-backend-markdown/src/pack.rs
  line: 214
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: check_path admits dot-prefixed segments that Overlay::children skips, so a parsed record is invisible to every reader with no error; only a hand-built pack reaches it
- file: crates/plan/aep-backend-markdown/src/pack.rs
  line: 384
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: parse admits a length with leading zeros, so two different pack files and names hold one record set; only a hand-written pack reaches it
- file: crates/plan/aep-backend-markdown/src/store.rs
  line: 87
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Overlay::beside_planning guesses the packs directory by directory name while the CLI's evidence_beside decides by the .engineering parent, so for --store /x/planning outside a project the artifact verbs and graph_at/specification/Assembly read different pack sets
```
