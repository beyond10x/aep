---
format: aep.planning-md/1
id: story:eval-matrix-assembler
kind: story
status: implemented
title: Many checked runs become one table of facts, and never a score
summary: A versioned run manifest says what an evaluation run was, and protocol eval matrix folds manifest-and-record pairs into per-expectation counts of held, contradicted and unobservable per harness, arm and workflow.
owner: eval
tags:
- eval
- evidence
- harness
relations:
- decomposes: epic:self-evaluation
- depends_on: story:contract-result-ingestion
revision: 1
---
# Story: many checked runs become one table of facts, and never a score

## Outcome

Somebody who has run the same cases three ways — raw instructions, the shipped plugin, a driven run
whose calls an enforcer decides — against two harnesses can put the runs in front of one verb and get
back a document saying, per expectation and per harness × arm × workflow, **how many facts held, how
many were contradicted and how many nobody could find out**, with the cost, tokens and wall time of
the runs that recorded them beside it.

What they cannot get back, from this verb or any flag on it, is a number that ranks the arms.

## Context

The three-arm programme produces transcripts; `protocol trace check` already turns a transcript and a
`trace-spec/1` document into a `trace-report/1` record with one row per expectation. Nothing joined
many of those records together, and nothing said *which run* a record was: a report carries the
specification's digest and the transcript's, and not the arm, the harness, the model or the plugin
that was installed. So a directory of reports was a heap of verdicts with the experiment missing from
it.

Two documents were needed and one existed. This story adds the other, and the verb that reads pairs.

## Why the deliverable is a matrix and not a score

A score would have to fold three columns into one number, and there are exactly two ways to do it.
Counting an unobservable expectation as a pass is the collapse invariant 5 exists to refuse — *nobody
found out* is not *it held*. Counting it as a failure blames an agent for a field a harness stopped
recording, which is what the third column exists to keep apart from the second: the Codex-side runs in
the committed fixtures carry `thinking_tokens: null`, and that is a fact about a wire, not about a
model's behaviour.

So the output is counts of facts. `no_rendering_of_a_matrix_contains_a_score` and the golden test
assert it on the bytes — no percent sign in either rendering, and the only occurrence of the word
*score* anywhere is the sentence saying none is computed.

## The manifest

`eval.run-manifest/1`, YAML, one beside each record:

| field | what it is |
|---|---|
| `arm` | `raw`, `plugin` or `driven` — closed, because the three arms are the design of the experiment |
| `harness` | which harness ran it; open, because a third harness is a run and not a redesign |
| `workflow` | the workflow the case is a run of |
| `case` | the case or task |
| `plugin_digest` | the plugin the run was given, or an explicit `null` — **always written** |
| `model` | the model, as the harness resolved it |
| `harness_version` | the version the arm is pinned to |
| `transcript_digest` | the run this manifest claims to describe |
| `observed_at` | when the run was observed |
| `cost_micro_usd`, `tokens`, `wall_time_ms` | optional, and reported as totals over the runs that state them |

Two of those rows are decisions rather than fields.

**`plugin_digest` is required to be written and may be `null`.** Serde maps an absent key and an
explicit `null` onto the same value, and here they are different facts: arm `raw`'s whole claim is
that it had no plugin, and a key somebody forgot must not be able to make that claim on their behalf.
The manifest reader keeps them apart (`written_down`), refuses an omitted key by name, refuses a
digest on arm `raw` and refuses `null` on arm `plugin`. Arm `driven` may answer either way — what
enforces a driven run is the driver at the seam, and whether the plugin was also installed is a fact
about that run.

**`transcript_digest` is required, and it is an addition to the field list the plan named.** It is
the only thing that makes *the manifest contradicts its record* checkable at all: a report states the
transcript it was judged over, so the pair either describes one run or is refused (`EVAL-PAIR-003`).
Without it a manifest could describe a Codex run under arm `raw` and be counted with the outcomes of
somebody else's Claude run, and neither document would object.

The three resource fields are optional keys, and the asymmetry with `plugin_digest` is deliberate: an
absent cost reads as *this run did not state one*, which is true and visible — every cell's total says
how many of its runs it covers, and `(1/2)` is in the committed rendering because one committed
manifest states none.

## The record it reads

The **check report**, not the evidence record. Both are called `trace_conformance` in conversation and
they are different documents: `protocol trace evidence` mints counts and the gapped ids and
deliberately drops the rows, because their citations quote the transcript
(`crates/trace-spec/src/evidence.rs`). A per-expectation matrix cannot be built from counts, so the
pair is a manifest beside `protocol trace check --format json`.

Sixteen refusals sit at that boundary, each with a stable code a test matches on rather than a
sentence: `EVAL-MANIFEST-001` … `-007`, `EVAL-RECORD-001` … `-003`, `EVAL-PAIR-001` … `-006`.

## Acceptance

- A run manifest is a versioned document whose refusals are by name, and validation accumulates.
  **Met**: `an_arm_this_evaluation_does_not_have_is_refused_by_name`,
  `a_missing_field_is_refused_by_its_own_name_and_every_other_refusal_is_reported_beside_it` (three
  fields removed, three refusals reported at once),
  `a_document_that_does_not_claim_the_format_is_refused_before_its_fields_are_believed`,
  `a_digest_that_is_not_a_digest_is_refused`.
- `plugin_digest` is written always, `null` exactly on arm `raw`. **Met**:
  `an_omitted_plugin_digest_is_refused_and_an_explicit_null_is_not`,
  `a_plugin_digest_on_arm_raw_is_refused_because_arm_raw_is_the_arm_without_one`,
  `a_null_plugin_digest_on_arm_plugin_is_refused_because_the_plugin_is_the_subject`, and
  `arm_driven_may_answer_either_way_because_the_enforcer_is_not_the_plugin` for the boundary —
  without which the three rules could have been written as one and every other test would still pass.
- A manifest contradicting the record beside it is refused. **Met**:
  `a_manifest_that_describes_another_run_than_its_record_is_refused` (`EVAL-PAIR-003`), plus
  `one_transcript_cannot_arrive_twice_because_one_run_would_be_counted_twice` and
  `one_specification_at_two_digests_is_refused_because_the_rows_share_a_name_only`.
- One verb assembles the matrix, deterministically and sorted. **Met**: `protocol eval matrix`
  (`crates/protocol-cli/src/eval.rs`), every aggregate grouped in a `BTreeMap` keyed by the tuple it
  is grouped by, and the arms sorted in the experiment's order rather than alphabetically
  (`the_arms_sort_in_the_order_the_experiment_runs_them` — alphabetically the table reads `driven`,
  `plugin`, `raw`, which is the experiment backwards).
- The exact assembled bytes are asserted. **Met**:
  `the_committed_pairs_assemble_into_the_matrix_byte_for_byte`
  (`crates/protocol-cli/tests/eval_matrix.rs`), over seven committed pairs in both renderings —
  `crates/protocol-cli/fixtures/eval-matrix/matrix.json` and `matrix.txt`. Byte equality rather than a
  field comparison, because the deliverable *is* a document somebody commits and diffs between waves.
- The fixture set reaches every state the assembler has a column for. **Met**:
  `the_matrix_reports_every_arm_of_every_harness_and_all_three_answers` — six cells, 23 facts held, 9
  contradicted, 3 nobody found out, and one resource total that covers 1 of a cell's 2 runs.
- A null or missing outcome is counted unobservable and never held, verified by mutation. **Met**:
  `a_row_whose_verdict_is_null_is_unobservable_and_never_held` beside the rule, plus
  `a_null_verdict_is_counted_unobservable_and_never_held` and
  `a_row_the_record_does_not_mention_at_all_is_the_same_answer_as_a_null_one` through the binary.
  Verified by breaking it: making `outcome_of` answer `Held` where the record says nothing turns all
  three red, and no other test in the workspace notices.

## What the committed fixtures are, and what they are not

**No three-arm run has happened yet.** The seven pairs are constructed, and the test file says so in
its first paragraph rather than leaving a reader to infer it from a date.

What is real in them is everything the assembler reads: every record is this repository's own checker's
output over a committed transcript, minted with `protocol trace check … --format json --redact`, and
`--redact` because a report that quotes a transcript is not a thing to commit to a public repository.
Four transcripts are recordings already in this tree; four are written for this fixture set, three of
them for a harness whose transcripts this repository has never held — which is the only reason the
`unobservable` column has anything in it: that wire carries `thinking_tokens` and writes `null`.

## Out of Scope

- **Anything gating on the matrix.** `protocol eval matrix` exits `0` whatever it says, on the same
  reasoning `protocol trace inspect` does: a matrix is a report, and an exit code that moved with the
  counts would be the scalar this story refuses to compute wearing a different name. Stated rather
  than implied.
- **Minting evidence from a matrix.** No `eval.matrix/1` fact enters the engine. The evidence a run
  produces is its own `trace_conformance` record, which `protocol trace evidence` already mints per
  run; a record about a *set* of runs is a different question and nothing asks it yet.
- **Running the arms.** Nothing here starts an agent, calls a model or reaches a network — the runs
  happen in the metaharness repository, and this verb reads what they left behind.
- **A second workflow in the fixtures.** All seven pairs are runs of `adp/default`, so the workflow
  column is grouped but never varies in the committed golden. Cheap to extend when a real programme
  produces a second one.

## Open Questions

**Should a resource total be refused when only some of a cell's runs recorded one?**
Decides: eval owner. Default if nobody answers: **no** — the total is reported with the coverage
beside it (`(1/2)`), which is the same shape `protocol evidence scan` uses for a partial answer.

**Should `harness` be a closed vocabulary, as `arm` is?**
Decides: eval owner, when a third harness appears. Default: **no** — an arm is a design of the
experiment and a harness is a subject of it, so a new harness should be a new row rather than a
refusal.
