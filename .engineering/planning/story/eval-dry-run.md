---
format: aep.planning-md/1
id: story:eval-dry-run
kind: story
status: implemented
title: The whole three-arm pipeline is green in the gate for nothing
summary: Four recorded streams go through the runner and the assembler and the matrix is asserted byte for byte, with the injected plugin's digest read out of the session attestation and the treated arm without its treatment refused by name.
owner: eval
tags:
- eval
- harness
- evidence
relations:
- decomposes: epic:self-evaluation
- depends_on: story:eval-runner
revision: 1
---
# Story: the whole three-arm pipeline is green in the gate for nothing

## Outcome

`task check` runs the evaluation programme end to end — manifest assembly, the check, the matrix
layout, the matrix — over four committed streams, with no vendor binary, no credential, no network
and no spend. A change to the runner, to a case's expectations, to a case's transcript or to the
manifest's field list lands as a failing row here rather than as a surprise in a paid sweep.

## Context

R3.4 on `docs/plan/eval-program-three-arms.md`. Everything above it was proven in pieces: the matrix
against constructed pairs, the corpus against its own transcripts, the runner against its refusals.
Nothing had ever run the pieces **in sequence**, which is where a layout convention or a field name
drifts without any single test noticing.

## What runs

Four replays, declared in `REPLAYS` in `crates/protocol-cli/tests/eval_dry_run.rs`:

| harness | arm | case | stream | verdict |
|---|---|---|---|---|
| claude | raw | `development-tests-after-the-code` | the corpus's own transcript, unchanged | 2 contradicted |
| claude | plugin | `development-honest` | `fixtures/eval-run/claude-plugin-attested.jsonl` | held |
| claude | driven | `development-honest` | `fixtures/eval-run/claude-driven-attested.jsonl` | held |
| codex | plugin | `development-honest` | `fixtures/eval-run/codex-plugin-attested.jsonl` | held |

Both harnesses, all three arms, one workflow, and a contradiction on purpose: a pipeline test whose
every row held would be green against a checker that had stopped checking. The arm-a stream is the
corpus's **declared violation**, so the two ordering rows it breaks are the ones the case wrote down
before any of this ran.

The assembled matrix is asserted **byte for byte** in both renderings against
`fixtures/eval-run/dry-run.matrix.{json,txt}` — 34 facts held, 2 contradicted, 4 runs — on
`story:eval-matrix-assembler`'s reasoning: the deliverable *is* a document somebody commits and
diffs between waves, so a row order, key order or column set that moved must fail rather than
produce a diff nobody chose.

## Crossing #4, on this side

metaharness's `--plugin-dir` copies a plugin into the scratch home and attests what it installed in
`session.started.hermetic.installed_plugins`. The manifest's `plugin_digest` is that string
**verbatim**. It is not a hash of the directory on disk, and the difference is the whole claim: a
digest computed here would attest bytes the *session* never saw, so an edited plugin would be
indistinguishable from the shipped one.

It is the **instrument's** row and not the top-level `plugins` echo beside it, which is whatever the
vendor happened to say — `null`, on Codex. Reading the wrong one of the two cost the first live pilot
run two refusals; `the_digest_is_read_from_the_instruments_row_and_not_from_the_vendors_echo` now
asserts the manifest is unchanged when the echo is deleted outright.

`the_plugin_digest_in_the_manifest_is_the_one_the_session_attested_byte_for_byte` reads the expected
value **out of the fixture** rather than repeating it, so the test follows an edited attestation
instead of pinning a constant beside it.

Three refusals sit where the attestation is read, and two of them are statements about the
experiment rather than about a document:

| code | refused |
|---|---|
| `EVAL-STREAM-006` | arm `plugin` over a stream attesting **no** plugin — the treated arm without its treatment |
| `EVAL-STREAM-007` | arm `raw` over a stream attesting **one** — the control arm with the treatment |
| `EVAL-STREAM-008` | a plugin attested with no `digest` — a manifest that cannot say which bytes cannot say what was measured |

The third is why the corpus's own `development-honest/transcript.jsonl` cannot stand in for arm b:
it predates the attestation, names a plugin and says nothing about its bytes.

The other half of this crossing is metaharness's `c1-plugin-injection` vector, named in
`crates/protocol-cli/fixtures/eval-run/README.md`. **Until that side replays these exact bytes, this
is one implementation agreeing with a transcription of another** — the same sentence, deliberately,
as the frame contract's.

## What the fixtures are, and what they are not

Structurally faithful and **not observed**. Three streams are derived from the corpus's honest
transcript by a derivation the README states line by line, so every number in them is a number this
fixture set chose. A failure here is a change in this repository's code or documents and **never a
finding about Claude Code or Codex**.

**Nothing is invented.** `digest` is written because R2.6 specifies it; `hermetic.installed_plugins`,
`hermetic.decisions` and the Codex leg's `model: null` are copied from the shape of the first live
pilot run. Before that run this section said observe mode's own attestation was deliberately *not*
written here, because the spelling had never been seen and a fabricated key would be a crossing
nobody agreed to. It has been seen now; the sentence stands as the rule and the fixtures have caught
up with the evidence.

When a paid sweep produces real streams they replace these **in place**: same directory, same verb,
same flags, no test change. That is the same property `conformance/eval/` claims for its transcripts,
and it is a property of reading the seam rather than a vendor.

## Acceptance

- The whole pipeline is green in the gate with zero spend. **Met**:
  `the_whole_pipeline_runs_on_committed_streams_and_assembles_the_matrix_byte_for_byte`. The test
  process removes `METAHARNESS_LIVE` and `METAHARNESS_BIN` from its own environment, so a developer
  who exported them for a sweep cannot turn this file into one.
- The dry run reaches both harnesses, all three arms and a contradiction. **Met**:
  `the_dry_run_reaches_both_harnesses_all_three_arms_and_a_contradiction`.
- The manifests carry the stream's fields and the runner's own. **Met**:
  `the_manifests_the_runner_wrote_are_the_documents_the_matrix_refuses_to_guess_at` — the arm-a
  manifest writes `plugin_digest: null` rather than omitting the key, the harness version says whose
  it is, and the codex run states no cost rather than zero.
- The injected plugin's digest is read out of the attestation byte for byte, and the treated arm
  without its treatment is refused. **Met**:
  `the_plugin_digest_in_the_manifest_is_the_one_the_session_attested_byte_for_byte`,
  `arm_plugin_over_a_stream_that_attests_no_plugin_is_refused_by_name`,
  `arm_raw_over_a_stream_that_attests_a_plugin_is_refused_by_name`,
  `an_attested_plugin_with_no_digest_is_refused_because_the_manifest_cannot_say_which_bytes`.
  Verified by breaking it: making the digest read optional turns the third red and no other test in
  the workspace notices.

## What the golden's third column says, and why it is zero

`0 nobody found out`, and that is a property of the **document** rather than of the pipeline: this
case's expectations read tool calls and orderings, and none of them reads a field a wire writes
`null` into. `crates/protocol-cli/tests/eval_matrix.rs`'s golden is where the column is exercised —
its Codex runs carry `thinking_tokens: null`. Written down in the test, so a later reader does not
take the zero for evidence that nothing here can be undecidable.

## Out of Scope

- **Any claim about a model.** Stated twice above and once in the test's first paragraph, because a
  fixture set that implied a measurement nobody made would be the same defect as a matrix implying a
  score nobody can compute.
- **The spawn.** Proven separately by `story:eval-runner`'s stubbed tests. This story is the half
  that needs no process at all.
- **A second workflow.** All four runs are of `adp/default`, so the workflow column is grouped and
  never varies. Cheap to extend when the corpus grows a case whose transcript attests a plugin.
