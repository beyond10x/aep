---
format: aep.planning-md/1
id: story:instruction-render
kind: story
status: implemented
title: A workflow and the principles timed against it, rendered as instructions
summary: The rules an agent is handed become a committed artifact rendered from the typed documents, held byte-identical, instead of prose somebody typed into a prompt once.
owner: eval
tags:
- eval
- render
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: A workflow and the principles timed against it, rendered as instructions

## Outcome

Somebody who wants an agent to follow this repository's methodology can hand it
`generated/instructions/adp/default.md` — 519 lines saying which states work moves through, what
opens each move, and which principles come due where — and know that the file says what the
specification says, because a check goes red the day it stops.

## Context

A three-arm evaluation measures how well a coding harness follows this methodology given (a) the
rules as prose, (b) the shipped plugin, (c) full enforcement. Arm (a) needs a treatment, and the
obvious way to produce one is for a person to write the rules into a prompt. That treatment measures
the person. It cannot be diffed, nobody can tell whether it still matches `workflows/` and
`principles/`, and the first workflow change silently invalidates every result gathered under it.

The material was already here and already typed. A workflow declares its states, its guards and the
phases each state belongs to; a principle times its obligations against those phases. What did not
exist was the join — *you may not enter `implement` until a failing test exists* — because neither
document names the other, and nothing in this repository ever wrote that sentence down.

## What was built

**A text backend beside the picture ones.** `aep-render` had four renderings of a `Scene` and all
four were pictures. `prose` is the fifth and is not: the same scene as instructions, in words.
`obligations` is the resolution step it reads, and it is where the join lives — each obligation's
timing resolved to the states of *this* workflow it reaches, by phase.

**A principle binds a workflow three ways**, and dropping any of them loses an instruction:
an obligation or a verification requirement lands on one of its states; or it withdraws a
capability, which holds everywhere (`least-privilege` has no obligations at all, and *you may not
read a secret* is the whole of it); or it requires evidence. A principle that reaches none of the
three is left out — a rule for a different workflow.

**The prose is connectives and nothing else.** Titles, summaries, guards, requirements, timings and
failure policies are the documents' own text, printed verbatim. What the renderer supplies is
*You may not enter this state until*, *From here you may move to*, *only while*, and the order
things are said in. The moment a renderer starts explaining what a state means, the explanation is a
second specification that nothing validates.

**`protocol workflow instruct`**, beside `protocol workflow render`. Its own verb rather than a
`--format markdown` on the drawing one, for the reason that verb's format enum already gives: every
value `render` takes produces a picture, and a rendering that is not a picture is a different
question.

**Four committed documents**, under `generated/instructions/`, one per workflow this tree declares,
with an index rendered from the same list. 1960 lines. `adp/default` is bound by 20 of the 22
principles, `incident/standard` by 21, `release/progressive` and `migration/forward-only` by 19
each — and where a principle's obligation is timed against a phase a workflow has no state for, the
document says so rather than dropping it or pretending it comes due.

## Acceptance

- The committed documents are what the verb writes, byte for byte, checked in both directions.
  **Met**: `the_committed_instruction_documents_are_what_the_verb_writes`
  (`crates/protocol-cli/tests/instructions.rs`) renders the tree with the real binary into a scratch
  directory and compares every file, then refuses a committed document that nothing renders any
  more. Verified by mutation: two appended lines fail it, naming the file, the first differing line
  and the command that fixes it.
- Determinism is over *content*, not over the run. **Met**:
  `a_workflow_that_moved_renders_different_bytes_and_leaves_its_neighbours_alone` copies the
  document tree, changes one guard predicate in it, and requires that workflow's document to move
  while `release/progressive.md` stays byte-identical to what is committed — a renderer emitting a
  constant would pass a byte-identity check and fails this one.
- Byte-identical across processes. **Met**: `two_runs_of_the_verb_write_the_same_bytes`, two
  separate invocations compared as whole trees, beside `two_renderings_of_one_workflow_are_the_same_bytes`
  in the crate and `the_committed_document_carries_no_date`, which reads the output rather than the
  sources — the determinism scan already refuses a clock in `aep-render`, and this would also catch
  a date copied out of a document.
- The join is in the output, not merely computed. **Met**:
  `the_state_a_principle_times_against_is_named_in_the_instruction` asserts the rendered sentence
  names `implement` for an obligation the principle times against the `implementation` phase, and
  `an_obligation_timed_against_a_phase_lands_on_every_state_declaring_it` asserts the resolution
  under it. Its complement — `a_workflow_without_the_phase_reports_the_obligation_as_owed_elsewhere`
  — asserts the difference between two real workflows rather than one side of it.
- Every way a principle can bind is covered, each by a fixture that reaches the state the rule is
  about: `a_principle_that_only_withdraws_a_capability_still_binds`,
  `a_principle_that_reaches_nothing_is_left_out_entirely`,
  `an_obligation_owed_at_every_transition_says_so_without_naming_a_state`.
- The tree has exactly one owner. **Met**: `generated/instructions` is carved out of the projection
  task's orphan scan, and `no_two_tasks_own_one_committed_tree_unless_the_outer_carves_the_inner_out`
  refuses an exclusion nothing owns. Verified by mutation: removing the owner fails that test with
  the subtree named.

## Two decisions worth the sentence

**The owner is a test, not an `xtask` task.** Every other committed tree here is written and
drift-checked by `cargo xtask <something> --check`, and this one is written by the verb a person
runs and checked by `crates/protocol-cli/tests/instructions.rs`. The reason is the rule `xtask`'s
own module documentation states: one writer per tree. A task would be a second implementation of a
verb that already exists — the thing that file avoids elsewhere by shelling out to `protocol` rather
than linking the generators — and the check it would add is one `cargo test` already runs.

**A conditional requirement is expanded, not elided.** `requirement_lines` wrote `if X then …`,
which is right on a canvas where the box is 260 pixels wide and wrong in a document that tells
somebody what to do. It now expands, and no committed workflow carries a conditional requirement, so
no figure moved; four principles do, and those are the lines a reader has to act on.

## Out of Scope

- **Evaluating anything.** A principle's `applies_when` is printed as the condition it is, never as
  a verdict: this crate cannot see a task's facts, and a rendering that answered would be a second
  protocol implementation with no conformance suites behind it.
- **Running the evaluation.** This is arm (a)'s *treatment*. The arms, the harnesses and the scoring
  live in metaharness `evals/engineering-protocols/`, and nothing here scores anything.
- **Profiles, protocols and step maps as instructions.** A profile decides which principles are in
  force for a task; these documents state what binds a workflow, which is the broader set. Rendering
  a profile's narrowed view is a second verb and a second artifact.
- **Deleting a stale document.** The verb writes and never removes: a directory the caller named is
  not a directory this verb may prune. An instruction for a workflow that no longer exists is found
  by the orphan scan and removed by hand.

## Open Questions

- Whether the eval's arm (a) should be handed the whole directory or one workflow's document. The
  index exists so that either is a defensible treatment; which one is measured is metaharness's
  call. Decides: whoever writes the eval's arm definitions.
