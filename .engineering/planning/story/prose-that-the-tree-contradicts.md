---
format: aep.planning-md/1
id: story:prose-that-the-tree-contradicts
kind: story
status: draft
title: Three statements the tree makes about itself that are false
relations:
- decomposes: epic:reference-driver
revision: 1
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
