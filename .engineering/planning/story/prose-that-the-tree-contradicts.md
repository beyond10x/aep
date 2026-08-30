---
format: aep.planning-md/1
id: story:prose-that-the-tree-contradicts
kind: story
status: active
title: Three statements the tree makes about itself that are false
relations:
- decomposes: epic:reference-driver
- serves: vision:O3
revision: 5
---
# Story: Three statements the tree makes about itself that are false

## Outcome

A reader who follows a comment in this repository to work out why something is the way it is arrives
at the reason it is actually that way.

## Context

The wave of 2026-08-30 disproved three claims written into shipped files. None of them was in any
unit's surface, so all three are still there. They are grouped because they are one defect — prose
asserting a mechanism nobody re-ran — and because each is one line.

**1. `crates/aep-engine/src/load.rs:29` names the wrong row.** The comment above the `drivers` row
says the order is load-bearing because *"the workflows are filled in by the row above this one"*.
The row above `drivers` is `artifacts/lifecycles` (`:27`); `workflows` is `:25`, two rows further
up. Worse, `:30-32` immediately explains that `Registry::validate` runs after the whole tree is
read, so the ordering is **not** what makes cross-validation work — the comment argues against
itself. Measured: moving `workflows` *after* `drivers` leaves the repository's own tree loading
clean. The real reason is the story's, and it is a better one: *so that no existing tree's load
order moves*. After the wave, the correct reason is stated in a test
(`crates/aep-engine/tests/document_tree_order.rs`) and the false one in the source that test
describes — the worse of the two arrangements, because the next reader reads the source.

**2. Seven copies of a claim `profiles/development-driven.yaml:78` contradicts.** *"No development
profile grants `command.execute`"* is false and has been since that profile shipped; the grant is
deliberate and its own header says why. `story:driver-router` corrected the two copies inside its
surface. These survive, and the first three are shipped artefacts rather than prose:

| file | kind |
|---|---|
| `crates/aep-driver/tests/shell_echo.rs:125`, `:196` | test doc |
| `drivers/development/default.yaml:11` | the shipped driver map |
| `conformance/eval/development-honest/expectations.trace.yaml:125` | a shipped conformance fixture |
| `docs/plan/harness-wave-4-governed-dogfood.md:125` | plan |
| `docs/reviews/2026-08-21-driver-feasibility-review.md:497` | review |

Two further copies — `docs/design/harness-planning-and-driver-design-v0.1.md:1758` and
`docs/plan/harness-wave-2-driver-decision.md:78` — already carry an inline correction and need
nothing.

**3. `crates/aep-engine/tests/adopting_guide.rs:19-26` is a second hand-maintained copy of the
loader's table.** `VENDORED` restates the six directories of `const TREE` in the same order, and its
doc comment claims to be *"the directories the loader walks"*. It is correct today. It is now
backstopped — any `TREE` change that would stale it reddens
`the_table_holds_one_row_for_every_kind_the_loader_accepts` — so this is the lowest-priority of the
three and is listed so the next person does not spend the discovery again.

## Acceptance

- `load.rs:29` states the reason the code actually has, and no comment in that file contradicts
  another.
- The five uncorrected copies of the `command.execute` claim are corrected or deleted; the shipped
  driver map and the conformance fixture first, because those are read by programs.
- `adopting_guide.rs`'s `VENDORED` derives from the loader's table or says in one line why it does
  not.

## Out of Scope

`crates/aep-driver-spec/tests/determinism.rs` is a weaker fork of `crates/aep-driver/tests/determinism.rs`
after the wave; whether the two converge is a separate question about the scan, not about prose.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong). **All three claims were re-verified against `3d86d5b`;
none has been fixed.**

- **Primary surface:** `crates/aep-engine` — cited, two of the three acceptance bullets land there and it holds the only non-comment change
- **Files:** `crates/aep-engine/src/load.rs:28-32` — cited, the `const TREE` comment above the `drivers` row. The false clause is `:29`; `workflows` is `:25` and the row above `drivers` is `artifacts/lifecycles` at `:27`
- **Files:** `crates/aep-engine/tests/adopting_guide.rs:19-27` — cited, `VENDORED` and its doc comment. The story says `:19-26`; the array closes at `:27`
- **Files:** `crates/aep-driver/tests/shell_echo.rs:125`, `:196` — cited, both still uncorrected
- **Files:** `drivers/development/default.yaml:11` — cited, a `#` comment in the shipped map
- **Files:** `conformance/eval/development-honest/expectations.trace.yaml:125` — cited, a `#` comment in the shipped fixture
- **Documents:** `docs/plan/harness-wave-4-governed-dogfood.md:125`, `docs/reviews/2026-08-21-driver-feasibility-review.md:497` — cited, both still uncorrected
- **Documents (need nothing):** `docs/design/harness-planning-and-driver-design-v0.1.md:1758`, `docs/plan/harness-wave-2-driver-decision.md:78` — cited, both already carry the inline strikethrough correction
- **Symbols:** `TREE` (`crates/aep-engine/src/load.rs:22`), `VENDORED` (`crates/aep-engine/tests/adopting_guide.rs:20`), `command.execute` (`profiles/development-driven.yaml:78`, the grant that falsifies the claim) — cited
- **Also likely:** `docs/plan/harness-wave-2-driver-decision.md:204` — inferred, a sixth flat restatement the story does not list, in a document whose `:78` is corrected
- **Not touched:** `crates/aep-driver-spec/src/tool.rs:13`, `crates/aep-driver/tests/tool_config.rs:72` — cited, `story:driver-router`'s two already-corrected copies
- **Not touched:** `crates/aep-engine/tests/document_tree_order.rs`, `document_tree_order_adversarial.rs` — cited, both parse `load.rs` for `TREE`'s rows and both skip lines whose trimmed form starts `//`, so rewriting the comment cannot redden them
- **Confidence:** **high** — the story cites every file and line, and each was read at `3d86d5b` and found unchanged; the only drift is one line on `VENDORED`'s extent
- **Would collide with:** any unit touching `crates/aep-engine/src/load.rs` or `crates/aep-engine/tests/adopting_guide.rs`; any unit touching `crates/aep-driver/tests/shell_echo.rs`; any unit editing `drivers/development/default.yaml` or `conformance/eval/development-honest/expectations.trace.yaml` — note `crates/protocol-cli/src/drive.rs`, `crates/aep-driver/tests/coverage.rs` and `crates/protocol-cli/tests/eval_*` **read** those two files without editing them; any unit editing `docs/plan/harness-wave-4-governed-dogfood.md` or `docs/reviews/2026-08-21-driver-feasibility-review.md`

**Not established.** The story says *seven copies* and accounts for seven, but `git grep -n "no development profile"` outside `.engineering/planning` returns twelve hits. `docs/plan/harness-wave-2-driver-decision.md:204` states the claim flatly in a *consequence* cell with no local correction, and it is unclear whether the story counted it as covered by the same document's corrected `:78`. `docs/design/harness-planning-and-driver-design-v0.1.md:1467` and `:1490` also state it; `:1490` is corrected a few lines below and `:1467` defers, so both were judged out of scope — inferred, not cited. Acceptance bullet 3 asks `VENDORED` to *derive from the loader's table or say in one line why it does not*, and which of the two is undecided: deriving would add a source-parsing helper to `adopting_guide.rs` duplicating `tree_directories()` in `document_tree_order.rs`, a third copy of the same parser, since integration tests are separate binaries. The story's *"moving `workflows` after `drivers` leaves the repository's own tree loading clean"* is a claim about a run the scoper did not reproduce.

## Left the wave of 2026-08-30 — the work is on a branch, unmerged

**Status: two full adversarial passes spent, still red on its own acceptance.** The budget is two
attacks; after them a unit goes to a person. This one did. The branch holds every commit and case.

| | |
|---|---|
| branch | `impl/prose-that-the-tree-contradicts` |
| commits | `f4dc459` (first), `fcf8b71` (correction), `b68fe00` (case conversion) |
| gate at `b68fe00` | four-package suite exit 0, 56 targets; `cargo test --workspace` exit 0, 200 targets; `task audit-check` 61 pass 0 fail; `cargo xtask guards` exit 0 |
| with attack 2's cases | exit 101, 5 targets failed, all 5 the adversary's |
| adversarial cases on the branch | 10 — 5 from pass 1, 5 from pass 2 |

**Why it did not land, and it is the story's own defect class.** This story exists because the tree
makes statements about itself that are false. `drivers/development/default.yaml` has now been
rewritten **three times** in this wave, and each rewrite removed a false sentence and wrote a new
one. That pattern is the finding, and it is why a fourth unverified rewrite was not commissioned.

1. **`drivers/development/default.yaml:14` — measured.** The header says the driver's per-call
   policy refuses every `Bash` call that is not one simple invocation of `protocol artifact …`,
   `protocol trace …` or one of the seven readers. That policy is `driven_surface`, reached only
   from `decide_tool` (`drive.rs:2780`), reached only when `event["decision_required"] == true`
   (`drive.rs:2655`). **All six `llm` steps in this map declare `harness: b10x`**, and
   `Harness::B10x` adjudicates nothing (`drive.rs:3439`); `adjudicates` is
   `matches!(self, Self::ClaudeCode)` (`drive.rs:3500`). What the native arm gets is
   `driven_programs` as `--allow-program` (`drive.rs:4508`) — program names only, no composition
   rule, no `artifact`/`trace` verb restriction. The clause has to be scoped to the vendor arm and
   the native arm described as the allow-list it is.
2. **`drivers/development/default.yaml:23` — measured.** Says *"any of the three profiles"* and
   names `development.fast`, `development.standard`, `development.driven`. The map pins
   `adp/default/2`; **four** profiles resolve to `adp/default` — `development.critical`
   (`profiles/development-critical.yaml:14`) is unnamed.
3. **`profiles/development-driven.yaml:73` and `:54-62` — measured.** The `decide_tool` clause is
   correctly scoped; the `summary:` and the store-guard argument are not, and neither holds on the
   b10x arm. *What reaches it: nothing found* — whether `b10x-harness`'s `run` composes is decided
   in a binary outside this tree, so the argument is shown not to cover that arm, not the conclusion
   shown false.
4. **`crates/aep-engine/tests/documents.rs:361` — measured.** `DENIALS` misses **6 of 6** phrasings
   that name `command.execute`: *grants no* (the wording this unit's own correction used at
   `shell_echo.rs:125`, `:198` and `harness-wave-4-governed-dogfood.md:125`), *never grant*, *cannot
   grant*, `_do not_`, *don't*, *not one*. The guard scans 160 files across 8 directories; a
   re-stated false claim in any of them passes. It should match content words near a negation set,
   not four sentences.
5. **`crates/aep-engine/tests/documents.rs:297` — measured.** `examples/` is in `READ_BY_PROGRAMS`
   because *"`examples/billing/` is the reference specification `crates/ess-conformance/src/reference.rs`
   compiles"*. `reference.rs` contains no `read_to_string`, `read_dir`, `File::open` or
   `include_str!`, and its own doc says the example is *"implemented by hand and in memory"*
   (`reference.rs:5-6`). The directory **is** read — by `crates/ess-compiler/tests/billing.rs:23-27`
   and `crates/aep-schema/tests/published.rs:66`. Name those.

**A defect the coordinator introduced, recorded because it is instructive.** Pass 1 wrote two cases
as proof-of-finding demonstrations that asserted the negation of a true property — their own doc
comments said *"the finding is that they are equal"* and *"Red as written."* The coordinator ruled
them the *wrong now* row and authorised converting them to assert what was decided. One conversion
produced a **tautology**: `assert_eq!(walk(moved), read)` over a fixture with no `drivers/`
documents is list-concatenation identity. Attack 2 proved it by mutation — reversing the loader's
sort at `load.rs:146` leaves the case green. Converting a proof-of-finding into a regression test is
correct in principle and needs a mutation check to confirm the result discriminates anything. The
prose half of that case is live: deleting the `TREE` comment does redden it.

**Two more, low.** `documents_guard_adversarial.rs:9` cites `documents.rs:322-323` for a quote
`fcf8b71` deleted in the same commit — the story's own defect class in a file it shipped.
`driven_surface_prose_adversarial.rs:126` is gated on a literal (`"bash call that is not one
simple"`), so rewording the shipped clause makes the whole case skip silently and pass.

**What did land on the branch and is worth keeping:** `shipped_documents` widened to `md` and
`json`; `READ_BY_PROGRAMS` gained `examples`; a development profile redefined as one written
against `adp/1` rather than one whose id starts `development.` (checked: `adp/1` is exactly the four
`development.*` and `aop/1` the two operations ones); the `TREE` comment rewritten to seven lines
naming the test that holds it; and `vendored_is_the_directories_the_loader_walks`, which derives
`VENDORED` parser-free from `DocumentKind::ALL` plus `directory()` plus one `load_tree_report` —
closing acceptance bullet 3's seventh-kind hole.

**What attack 2 could not break:** both comment parsers fail loud rather than vacuous;
`expectations.trace.yaml:125`'s rewritten claim is **true** (the transcript's `session.started`
records `adapter: "claude"`); `driven_surface` itself matches its prose exactly; the widened scan
produces no panic and no false positive across all 160 files.
