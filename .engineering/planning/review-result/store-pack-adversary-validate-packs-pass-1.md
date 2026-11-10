---
format: aep.planning-md/3
id: review-result:store-pack-adversary-validate-packs-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:validate-checks-packs
relations:
- reviews: story:validate-checks-packs
revision: 1
---
```
unit: story:validate-checks-packs, branch impl/validate-checks-packs, findings cover 74d3c0400 (cases committed as b70c21e78)
verdict: NEEDS-CHANGE
cases: executed 176→182, red 4
origin: introduced 3 / pre-existing 0 / undecided 1
wrote-outside-worktree: 5 paths in the assigned scratch (part 6), plus fixture repositories under the shared target's tmp
needs-coordinator: yes (whether a vanished pack is in this story's scope, finding 3)
```

Validate can be made to answer `valid` while the store serves a forged record. That is the outcome the brief called most serious. Four new cases are red and two probes are green.

**1. Diff stat** (`git --no-pager diff --stat 74d3c0400 HEAD`). Only one test file changed, so the bound held:
```
 .../edge/aep-cli/tests/validate_packs_adversary.rs | 567 +++++++++++++++++++++
 1 file changed, 567 insertions(+)
```

**2. Cases added** in `crates/edge/aep-cli/tests/validate_packs_adversary.rs`. The red output below is from the first run of that binary alone, before the suite ran. `cargo fmt` reformatted the file after the suite run; only line wrapping changed.

- `a_packs_directory_linked_out_of_the_work_tree_does_not_skip_provenance` (:261), **red**. `.engineering/packs` is a symlink to a directory outside every work tree. It holds a pack whose `closed-a.md` was rewritten to "Forged Z", with digest and name recomputed. `show` serves "Forged Z". The test asserts exit 1 with `pack_record_not_committed_bytes` or `pack_path_unsafe`.
  ```
  assertion `left == right` failed: a record nobody committed, behind a linked packs directory, is refused: 3 file(s) in .../validate-packs-adv-linked-packs/.engineering/planning: 3 artifact(s)
  valid
    left: Some(0)
   right: Some(1)
  ```
- `a_packs_directory_that_is_a_repository_of_its_own_does_not_skip_provenance` (:418), **red**. Same forgery with no symlink: running `git init` inside `packs` is enough.
  ```
  assertion `left == right` failed: a record nobody committed, in a packs directory that is its own repository, is refused: 3 file(s) in ...: 3 artifact(s)
  valid
    left: Some(0)
   right: Some(1)
  ```
- `a_committed_pack_deleted_from_the_working_tree_does_not_lose_its_evidence_silently` (:322), **red**. The store has two committed packs and one is deleted from the working tree (uncommitted). A committed evidence record disappears and validate says `valid`. The same test checks that deleting that record when it is loose is refused (exit 1).
  ```
  assertion `left == right` failed: a committed pack gone from the working tree, and the committed evidence record `evidence/story/closed-a/20260928T100000Z-000-424990581920.json` with it, is refused: 2 file(s) in ...: 2 artifact(s)
  valid
    left: Some(0)
   right: Some(1)
  ```
- `a_shallow_clone_of_a_valid_packed_store_names_the_shallow_clone_not_forgery` (:384), **red**. A `git clone --depth 1` of a valid packed store (pack commit not at the tip) accuses every record of holding uncommitted bytes:
  ```
  3 problem(s):
    - [pack_record_not_committed_bytes] .../packs/6785da2b94d122bf.pack: `evidence/story/closed-a/20260928T100000Z-000-424990581920.json` has no committed blob at its original path before the commit that added this pack
    - [pack_record_not_committed_bytes] .../packs/6785da2b94d122bf.pack: `planning/story/closed-a.md` has no committed blob ...
    - [pack_record_not_committed_bytes] .../packs/ad04b80b73b6f740.pack: `planning/story/closed-b.md` has no committed blob ...
  ```
- `a_pack_holding_an_older_committed_version_is_refused` (:458), **green**. The record was committed A, then B, then packed as A; it is refused for provenance.
- `no_output_format_prints_a_redacted_literal_a_packed_copy_re_exposes` (:485), **green**. A packed copy re-exposes a redacted literal; `--format text`, `json` and `yaml` all print the `pack_redacted_literal_exposed` code and never the literal.

**3. Suite run**, after the cases existed:
```
cargo.sh test --locked -p aep-cli --no-fail-fast --test validate_packs --test git_store --test store_pack_overlay --test store_pack_overlay_adversary --test record_redaction_adversary --test planning_cli --test validate_packs_adversary
git_store                    test result: ok. 26 passed; 0 failed
planning_cli                 test result: ok. 114 passed; 0 failed
record_redaction_adversary   test result: ok. 4 passed; 0 failed
store_pack_overlay           test result: ok. 8 passed; 0 failed
store_pack_overlay_adversary test result: ok. 5 passed; 0 failed
validate_packs               test result: ok. 19 passed; 0 failed
validate_packs_adversary     test result: FAILED. 2 passed; 4 failed
exit 101
```
176 cases before my file (the same run with `validate_packs_adversary` left out), 182 after. `cargo.sh clippy --locked -p aep-cli --test validate_packs_adversary -- -D warnings`: exit 0. `cargo fmt --check` now finds no diff in the crate.

**4. Findings**

- **F1 (NEEDS-CHANGE, blocker, introduced)**, `planning/git_record.rs:309` and `planning.rs:6988`. `blobs_before_pack` runs `git log` with `-C` set to the pack's own directory and returns `None` if that fails. The caller treats `None` as "skip provenance" (`else { continue; }`). Any packs directory where git fails therefore turns the check off, while every other pack check passes.
  - What was measured: cases :261 and :418, exit 0 and `valid` with forged bytes being served.
  - What reaches it: no aep verb creates such a directory. A committed or local symlink, or a nested repository, is enough, and the symlinked packs directory itself is never refused (only symlinked pack *files* are, `planning.rs:6894`).
  - Fix, not applied: find the pack's add commit from `base` with a base-relative pathspec. Skip only when `unversioned(base)`; any other git failure should be a refusal. Refuse a packs directory that is a symlink.
- **F2 (NEEDS-CHANGE, warning, introduced)**, `planning/git_record.rs:331`. In a shallow clone the pack appears to be added by the boundary commit. That commit has no parent, so the code returns `Committed::Never` for every record and reports it as forgery.
  - What was measured: case :384.
  - What reaches it: any CI checkout without `fetch-depth: 0`. The CI guide asks for depth 0 (`website/docs/guides/validate-in-ci.md:40`), but its reason there is review dating only. The established precedent is `findings_standing.rs:74`, which names the shallow clone and its remedy.
- **F3 (CONFIRMED, warning, undecided)**, `planning/git_record.rs:197` (`changed_evidence`). Deleting a committed loose evidence file is refused. Deleting the committed pack that holds the same record is not.
  - What was measured: case :322.
  - What reaches it: `rm` or a re-pack that drops a pack. Neither the acceptance nor design items 1–8 require this check. The brief asks for it ("no record lost"), so whether it belongs to this story is for you to decide.
- **F4 (note, judgement, introduced)**, `planning.rs` check 6. A packed artifact that is not UTF-8 or does not parse as a `PlanningDocument` skips the closed-status check silently. Provenance bounds it to bytes that were committed, so I did not build a case.

**5. Attacked, could not break**
- An older committed version packed after a newer one: refused.
- Uppercase, truncated or wrong-length digest: the parser refuses it, and a misnamed file is caught by the name check.
- Redacted literal in text, json or yaml output: absent.
- A path held twice, loose plus packed with different bytes: refused.
- Permissive or missing lifecycle: refused as not closed, because `is_closed` is false.
- Pack cache: per file length and modification time, so no per-document re-parse.
- `--diff-filter=A` and merges: no forging path found. On a branch-then-merge history, a later edit on main is accepted as dropped, which matches the design's wording.

**6. Paths written outside the worktree**
- Scratch: `<scratch>/pack/units/validate-checks-packs/scratch/{adv-red-run.log, adv-red-nested.log, adv-suite.log, adv-clippy.log, adv-commit-msg.txt}`
- Fixture repositories the tests create: `<scratch>/target/tmp/validate-packs-adv-*`
- Lease released (`session-end aep-adv-validate-packs`).

**7. Findings block**
```findings
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 309
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a git failure inside the packs directory (symlink out of the work tree, nested repository) makes blobs_before_pack return None and validate skips provenance, answering valid on a forged record
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 331
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: in a shallow clone every packed record is refused as pack_record_not_committed_bytes instead of the refusal naming the shallow clone
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 197
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: deleting a committed pack from the working tree loses committed evidence silently while the same loose deletion is refused
- file: crates/edge/aep-cli/src/planning.rs
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a packed artifact that is not UTF-8 or does not parse skips the closed-status check without a problem line
```
