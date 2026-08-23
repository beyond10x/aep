---
format: aep.planning-md/1
id: story:eval-case-corpus
kind: story
status: implemented
title: A per-workflow eval case is three committed files the gate replays
summary: Task statement, trace expectations and a transcript per case, enumerated from disk and replayed to a declared verdict — with a deliberate violation beside the honest run, because a bound never observed to fail is a bound nobody has evidence discriminates.
owner: eval
tags:
- eval
- trace
relations:
- decomposes: epic:self-evaluation
- depends_on: story:workflow-plugin-coverage
revision: 1
---
# Story: A per-workflow eval case is three committed files the gate replays

## Outcome

Somebody adding an eval case writes three files into a new directory and runs the gate. Nothing has
to be registered, no test is edited, and a case whose transcript stops satisfying its own document
turns `task check` red naming the expectation that stopped holding.

## Context

The three-arm evaluation measures how well Claude Code and Codex follow this repository's workflows
under raw instructions, under the shipped plugin and under enforcement. It needs cases, and a case
that only exists as a shell script inside a paid runner is a case nothing holds between runs — which
is the state `.engineering/planning/specification/agent-charter-eval-cases.md` names in its own **Out
of Scope**: *"between live runs the new documents are held only by R17's offline mode, which nothing
in `task check` invokes."*

The judge stays expectation-based throughout. No case is scored, nothing is model-judged, and there
is no scalar anywhere: a case declares a verdict and the checker either reaches it or names the rows
that did not.

## Where the corpus lives

`conformance/eval/`. `conformance/` is where this repository keeps the material a claim is decided
against — `fixtures/`, `scenarios/` and `expected/` for a backend, `trace/` for the three shipped
trace documents. An eval case is the same class of object one domain further out. It is not a
protocol document kind, so it does not belong beside `workflows/` or `drivers/`; it is not generated,
so it does not belong under `generated/` or `suites/`. `conformance/README.md` gains rows for
`trace/` and `eval/`, the first of which had been an undocumented tenant since the migration.

## What shipped

- **`conformance/eval/`** — five cases. Each is a `case.yaml` (id, workflow, states, arm, declared
  verdict, and the task statement as prose), a `trace-spec/1` document and a committed
  `transcript.jsonl`.

  | case | workflow | verdict |
  |---|---|---|
  | `development-honest` | `adp/default` | held |
  | `development-tests-after-the-code` | `adp/default` | violated — two rows, named |
  | `release-progressive-honest` | `release/progressive` | held |
  | `decomposer-charter` | `adp/default` | held |
  | `plan-reviewer-charter` | `adp/default` | held |

- **`crates/protocol-cli/tests/eval_corpus.rs`** — four tests that enumerate the corpus root and
  replay every case, using `crates/trace-spec/tests/`' fixture-replay idiom unchanged.

## Acceptance

- Every case replays through the checker in the gate. **Met**:
  `every_case_replays_to_the_verdict_it_declares`, which reads the committed document with
  `trace_domain::raw::read_spec`, the transcript with `read_any`, checks, and prints
  `report_to_text` on any failure.
- A case declaring `held` must exit 0 with nothing contradicted; a case declaring `violated` must gap
  **exactly** the expectation ids it names. **Met**, same test, and pinned in both directions —
  repairing a transcript so a declared violation stops gapping is as red as breaking a row that was
  passing.
- `unk` is refused in every case, of both kinds. **Met**, same test. An undecidable row is how a
  check stops being a check without anybody noticing, and a violation case that went `unk` would look
  like a failure while reporting the opposite of what it claims.
- The violation case says exactly which expectation was violated and why. **Met**: `violated:` in
  `case.yaml` carries one entry per row with a prose account, and the test refuses an entry naming an
  expectation the document does not contain, or one whose account is a stub.
- Adding a case costs three files and no code. **Met**: the corpus root is enumerated at run time; a
  directory without a `case.yaml` is refused by name rather than skipped.
- Every case names a real workflow and real states of it. **Met**:
  `every_case_is_three_files_and_says_what_it_is_about`, which parses the workflow documents and
  refuses an unknown id or state by name.
- No case can pass by gating on nothing. **Met**: `every_case_gates_on_something` refuses a document
  of nothing but advisory rows, which reads in a report exactly like a case that passed.
- `task check` is green.

## The violation case is the control, and it is judged by its sibling's document

`development-tests-after-the-code` points its `expectations:` at
`../development-honest/expectations.trace.yaml` rather than carrying a copy, and
`the_two_development_cases_are_judged_by_one_document` asserts the two resolve to the same file. Two
documents, each quietly edited to suit its own transcript, would prove nothing about either — and
that drift is silent, because both would be green.

The two runs then differ in exactly one thing: whether the code was written before the test. That one
defect contradicts **two** rows, and the case says so rather than tidying it away —
`the-test-came-before-the-code` is about which file was authored first, and
`the-suite-was-run-before-there-was-code-to-pass-it` is about whether anybody ever watched the check
go red. A run that wrote the test first and never ran it would satisfy the first and contradict the
second, which is why the corpus carries both.

## The agent-charter cases, and the half of them that did not fold in

Both cases from `specification:agent-charter-eval-cases` fold in as their transcript half: R12's rows
for each agent, R6's D9 and R9's P7 (`subagent.spawned`), and R13's positive control over the same
tool every absence is scoped to.

**What does not fold in is named in each case file rather than left to inference.** R6's D1–D7 and
R9's P1–P3 — the created-set arithmetic, the status baseline, the file digests and the clean working
tree — are tree-side facts, and no `trace-spec/1` kind reads a file or the git index. The
specification establishes this itself and settles it in R11: those facts stay in the shell. So a
green `plan-reviewer-charter` does **not** establish that no file changed, and its document says so
in its own header.

R12's `tool.absent` over `Write` under `.engineering/planning` is dropped from both cases. Neither
charter grants `Write`, so the row is true of every possible run — R14's argument, which the
specification applies to the plan-reviewer and which holds for the decomposer for the same reason. A
vacuous row is worse than a missing one because it reads like coverage.

## What the first live pilot changed, 2026-08-23

The corpus was written against synthesized transcripts and met a real one. Two rows were wrong, and
both were wrong in the direction that matters least to a passing gate and most to a reader.

**A write is not one verb.** `the-test-came-before-the-code` and
`the-suite-was-run-before-there-was-code-to-pass-it` scoped to `tool: Write`. The pilot did the work
with `Edit` — the files already existed — so no `Write` event ever occurred and both rows reported
`never_occurred`: the checker shrugging at work whose result is visible in the run's own working
clone. Fixed by giving `CallSelector` a **set** of names (`tools: [...]`, `trace-domain`) and scoping
every witnessing row to `crates/protocol-cli/src/drive.rs`'s own `repository.write` list. The
ordering claims are untouched; only what can witness them grew, and
`crates/trace-spec/tests/write_selectors.rs` mutation-tests both directions plus the `Read` negative
control that stops the scope being dropped altogether.

**A forbidding row is a different set from a witnessing one.** Widened to
`[Edit, NotebookEdit, Write]`, the store guardrail immediately contradicted a run that had done
nothing wrong: it edited artifact *bodies*, which the skill's second guardrail asks for. The store's
real rule denies whole-file replacement and permits a targeted edit, so the row is now
`[NotebookEdit, Write]` and renamed `no-artifact-file-was-rewritten-whole`, with an
`env.tool_available` attribution control beside it in the idiom the driven-step document already
uses. The fence half is not transcript-decidable and is named as staying with `protocol artifact
validate`.

**One row that looked like a checker bug was not one.** `the-editor-was-used-at-all` returned
`unk` citing two opaque `system` events in a stream of 56 readable tool events. The checker was
right: the selector matched **zero** calls, and with nothing observed an unread event genuinely could
have been the missing write. `decide_count` already does the three-valued reasoning — an unread event
can only add calls, so `at_least: 1` with one match holds regardless — and the row went `ok` the
moment the selector was correct. **No checker change was made**, and both polarities are now pinned
by test so the semantics is held by something that runs rather than by having been read once.

Re-ingesting the recorded stream, free: **3 ok / 2 gap / 4 unknown → 7 ok / 2 gap / 1 unknown**. The
remaining `unk` is the absence row, which cannot be proved past an unread event and should not be.
The two gaps are real and are the run's, not the corpus's: the session ended on an API error after
the work was done.

## The transcripts are synthesized, and the corpus says so out loud

Every transcript here is hand-written against the `metaharness.event/1` event stream, the same
construction and for the same reason as `crates/trace-spec/tests/fixtures/metaharness-driven-*.jsonl`.
They are structurally faithful and not observed: a number in one of these files is a number this
corpus chose, so a failing assertion is a change in the checker or a document and never a finding
about a harness. A green corpus means the bounds are satisfiable and self-consistent, and means
nothing about any model.

The seam format is deliberate. It is the one reader that serves every harness metaharness drives, so
a case written against it is checkable for a Claude Code arm and a Codex arm alike. `read_any`
detects the format from the transcript's own first line, so a recorded run replaces a synthesized
fixture in place — same directory, same document, no test change.

## Out of Scope

- **A `stream-json` case.** The interactive arms produce Claude Code's own transcript format and the
  reader handles it, but no case here is recorded in it. Stated as a corpus gap rather than a checker
  gap: `a_recorded_transcript_and_a_driven_stream_take_the_same_arguments` already holds both.
- **Cases for `incident/standard` and `migration/forward-only`.** Both are wholly uncovered in
  `integrations/workflow-coverage.yaml`, so a case for either measures raw instruction-following with
  no plugin arm to compare against. Worth writing; not this story.
- **Running any of this against a model.** Nothing here reaches a network, and `task check` stays
  hermetic — the same line the two evals beside it are kept on.
- **A scalar.** No case carries a score, a percentage or a grade, and the corpus has no aggregate.
  The output of a case is a verdict per row with the events that produced it.

## Open Questions

**Should a case be able to declare a `state` it does not exercise?**
Decides: eval owner. Default if nobody answers: **no** — `states:` is what ties a case to the
coverage map, and a case listing states its transcript never reaches would make that tie decorative.
