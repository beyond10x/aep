---
format: aep.planning-md/3
id: review-result:redacted-literal-scan-adversary-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:redacted-literal-scan-reads-only-redactable-text
relations:
- reviews: story:redacted-literal-scan-reads-only-redactable-text
revision: 1
---
```
unit: story:redacted-literal-scan-reads-only-redactable-text, branch fix/redacted-literal-scan-prose-only, findings cover 9522a5c61 plus my test commit 35e992c90
verdict: NEEDS-CHANGE
cases: executed 171→174, red 2
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: whether `prose_only` counts as text `redact` must cover. The story's Fix lists only identity fields as out of scope, so this is a scope decision.
```

**1. Diff stat** (9522a5c61..HEAD; only a test file, no source file touched)
```
 crates/edge/aep-cli/tests/validate_packs.rs | 124 ++++++++++++++++++++++++++++
 1 file changed, 124 insertions(+)
```

**2. Cases added** (`crates/edge/aep-cli/tests/validate_packs.rs`, commit 35e992c90)
- `a_loose_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused` (:966), **red**. A review is recorded with `new --prose-only "Quillon asked for prose only"` and a body that names Quillon. `redact --literal Quillon` runs, the review is archived and the store committed. The setup checks that the redacted copy still holds `prose_only: Quillon ...`. The case expects `pack_redacted_literal_exposed`. It is red because validate says `valid`. Output when I ran it alone, verbatim:
  ```
  thread 'a_loose_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused' (2049075) panicked at crates/edge/aep-cli/tests/validate_packs.rs:283:5:
  assertion `left == right` failed: validate refuses: 2 file(s) in <scratch>/aep/target/tmp/validate-packs-adv-prose-only-loose/.engineering/planning: 2 artifact(s)
  1 review(s) recorded no findings block:
    - review-result:looked states its findings as prose only — nothing can enumerate what it found, so the next review starts from nowhere
  valid

    left: Some(0)
   right: Some(1)
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 2.51s
  ```
- `a_packed_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused` (:985), **red**. Same store, then packed. It fails the same way, at :283 with exit 0. This one first ran inside the suite, not alone. That breaks the order the procedure asks for, so I am stating it.
- `a_packed_copy_that_does_not_parse_and_holds_the_literal_is_still_refused_for_it` (:1005), **green**. It covers the branch for a copy that does not parse, which no earlier case exercised. Without it, the mutant `Err(_) => 0` survives the suite. I did not run that mutant: I argued it from the code.

After the run I applied `rustfmt` to my own lines. That changed formatting only, and `rustfmt --check` is clean.

**3. Suite run**, after the cases existed:
`cargo.sh test --locked --no-fail-fast -p aep-cli --test validate_packs --test validate_packs_adversary --test validate_packs_adversary_2 --test store_pack_security --test store_pack_security_adversary --test store_pack_security_adversary_2 --test planning_cli`
```
planning_cli:                    test result: ok. 114 passed; 0 failed; ...
store_pack_security:             test result: ok. 13 passed; 0 failed; ...
store_pack_security_adversary:   test result: ok. 3 passed; 0 failed; ...
store_pack_security_adversary_2: test result: ok. 5 passed; 0 failed; ...
validate_packs:                  test result: FAILED. 26 passed; 2 failed; ...
  a_loose_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused ... FAILED
  a_packed_copy_that_holds_the_literal_in_its_prose_only_reason_is_refused ... FAILED
validate_packs_adversary:        test result: ok. 6 passed; 0 failed; ...
validate_packs_adversary_2:      test result: ok. 5 passed; 0 failed; ...
exit 101
```
The total is 174, and 171 of those cases are not mine.

**4. Findings**
- **F1** `crates/edge/aep-cli/src/planning.rs:7325`, NEEDS-CHANGE, introduced, warning.
  - **The problem:** `prose_only` is free text in front matter. `new --prose-only <reason>` writes it, and `show` prints it. `redact` (`redacted_text`, :4061) does not rewrite it. The old check 7 searched the whole file, so a literal left in `prose_only` after a redaction was refused. The narrowed scan lets it through, loose and packed.
  - **Origin:** the base line `replace_digest(&text, …)` searched every byte of the copy. That is my reading of the diff; I did not run the base.
  - **What reaches it:** the documented verb `new review-result --prose-only "<reason naming a person>"`, followed by `redact`. `redact` reports success, and `validate` now says `valid` while `show` still prints the name.
  - **Fix:** add `prose_only` to `redacted_text`, to the replay at about :6686, and to check 7's field set. The alternative is for the story to say explicitly that `prose_only` is out of scope.
- **Note, no row:** `a_review_that_never_held_the_literal_in_its_actor_is_still_valid` only asserts that validate passes. It would also pass with check 7 deleted, and it passes on the base. It guards nothing the fix changed. The tests that expect a refusal cover what it was meant to show, so I am not raising it.

**5. What I attacked and could not break**
- **The four new cases against mutants:** "search the body only", "skip the summary" and "skip packed copies" are each killed by the title or summary cases, loose and packed. "Treat an unparseable copy as clean" was not killed until my third case. Even under that mutant, check 6 or the store's read failure still makes validate refuse, so the invariant holds.
- **Parser:** check 7 uses the same `PlanningDocument::parse` as `store.rs:180`. The fence split is the same, so no `---` trick moves the literal out of the searched text: anything after the first closing fence counts as body.
- **YAML tricks:** a duplicate `title:` key fails to parse, so the copy is searched whole. Escaped or continued titles are caught better than before, because the parsed value is searched.
- **Replay at about :6686 and check 7:** both cover title, summary and body, so neither reports something the other does not.
- **Case and Unicode:** both use the same folding matcher. A combining-sequence spelling or a word split across a line wrap is missed by `redact` and by both versions of the scan alike, so this change adds nothing there.
- **Literal in an owner, tag, relation target, scope path, extra key or YAML comment:** `redact` could never have removed it from those places. A stale copy of the artifact also holds the literal in title, summary or body whenever `redact` removed it there, so it is still caught.

**6. Paths written outside the worktree**
- `<scratch>/aep/pack/units/redact-scope/adv-scratch/` (cases.rs, run1.log, suite.log, commit-msg.txt)
- `<scratch>/aep/target/tmp/validate-packs-adv-prose-only-loose`, `…-adv-prose-only-packed` and `…-adv-unparseable-packed`: test projects in the shared target, created by the tests.
- I started a `cargo.sh fmt --check` and killed it after 100 s while it waited for a build slot. It wrote nothing.

The lease `aep-adv-redact` is released.

```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 7325
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the narrowed scan no longer refuses a literal left in a review's prose_only reason, which redact does not rewrite and show prints, so validate answers valid on a redacted copy that still names the person, loose and packed
```
