---
format: aep.planning-md/1
id: story:eval-runner
kind: story
status: implemented
title: One verb runs an arm of a case and leaves the three documents the matrix reads
summary: protocol eval run drives metaharness as a tool, assembles the run manifest runner-side out of what the stream states, and refuses to spawn without a live flag, a cap and a working tree — with arm driven left to protocol drive by name.
owner: eval
tags:
- eval
- harness
- evidence
relations:
- decomposes: epic:self-evaluation
- depends_on: story:eval-matrix-assembler
- depends_on: story:eval-case-corpus
- depends_on: story:instruction-render
revision: 1
---
# Story: one verb runs an arm of a case and leaves the three documents the matrix reads

## Outcome

Somebody who wants a row in the evaluation matrix runs one command per run. It finds `metaharness`,
spawns the arm's treatment into a hermetic scratch home, judges the transcript with the case's own
document, and writes the pair `protocol eval matrix` reads — beside the raw event stream, so the
pair can be re-derived by anyone holding the bytes.

Somebody without the binary is told so by name and gets exit `2`. Somebody who did not mean to spend
money cannot: nothing is spawned without `METAHARNESS_LIVE=1` **and** a cap.

## Context

`story:eval-matrix-assembler` built the reader and `story:eval-case-corpus` built the cases, and
between them sat a hole: seven constructed pairs and no way to produce a real one. The plan page
called that hole R3.2 and R3.3 (`docs/plan/eval-program-three-arms.md`).

## The decision this story is mostly about

**The run manifest is assembled runner-side. The seam gains no field and no crossing.**

R3.2 proposed that metaharness emit the manifest as an event field. It does not, and the reason is
that a manifest has two kinds of field:

| field | who knows it | where it comes from |
|---|---|---|
| `harness_version` | the session | `session.started.adapter` + `.harness_version` — `claude 2.1.239`, because two harnesses at `0.145.0` are not one pin |
| `model` | the session | `session.started.model`, `Written` — key required, explicit `null` legal |
| `plugin_digest` | the session | `session.started.hermetic.installed_plugins[].digest`, **verbatim** — the *instrument's* row, not the vendor echo beside it |
| `transcript_digest` | the check | what this runner's own check states about the bytes it judged |
| `arm`, `workflow`, `case` | the runner | nothing in a stream could know them |
| `observed_at` | the caller | `--observed-at`, required |

The second half is why the proposal was refused. metaharness runs a *session*; *this session is arm
b of case X* is a claim about an **experiment**, and emitting it would have put `raw`, `plugin` and
`driven` into a repository that has no reason to hold those words — and made every change to the
manifest a two-repository release.

One narrowing to the plan's own field list: **`model` is read from the stream, not from what the
runner asked for.** A runner writing down the model it *requested* would record a model the run may
not have used, and a resolution that moved is exactly the fact a later reader of the matrix needs.

What keeps the decision honest is that reading fail-closed is the only reading. A stream whose
`session.started` does not state what the manifest needs is **refused by name** and no manifest is
written — a runner that filled a hole with a plausible value would be writing the one document the
matrix trusts.

## What the first live run corrected, and what it did not

The first live pilot run — codex, arm a, `development-honest` — refused twice, on `model` and on
`plugins`. **Both refusals were right and both fields were wrong**, which is the distinction worth
keeping: the boundary did its job, and what it was told to look at was mistaken.

**`plugin_digest` came from the vendor's echo.** `session.started` carries two plugin lists.
Top-level `plugins` is the vendor's own init list — Claude Code writes one, Codex writes `null`,
because metaharness will not mint a vendor field it did not receive (a9 discipline, the same rule
that leaves `thinking_tokens` null rather than zero). `hermetic.installed_plugins` is the
**instrument's** record of what it injected, and it is written on every adapter. Crossing #4 is the
second row. `the_digest_is_read_from_the_instruments_row_and_not_from_the_vendors_echo` asserts it by
deleting the vendor echo and requiring the manifest to be unchanged.

**`model` could not be written down as unstated.** Codex names no model at session start — the whole
62-event run never states one — so `model` became a `Written` on `plugin_digest`'s exact reasoning:
the key is required, an explicit `null` is legal and read as *the harness did not say*, and an absent
key is still refused, because a runner that dropped it would produce the same document. The matrix
writes `"model": null`; the runner's own line says `(unstated)`, which is obviously not a model name.

## The three arms, and the one that is a refusal

| arm | what is spawned | prompt |
|---|---|---|
| `raw` | `metaharness run <harness> --hermetic --cwd DIR --decisions observe -p …` | the workflow's committed instruction document from `generated/instructions/`, then the case's task |
| `plugin` | the same, plus `--plugin-dir integrations/<harness>` | the task **alone** |
| `driven` | **nothing** — `EVAL-RUN-004` | — |

Two of those rows are decisions.

**Arm b gets the task alone.** Arm a is *text and hope*; arm b's treatment **is** the plugin, whose
skills and agents are what are supposed to carry the workflow. Giving arm b the instructions as well
would measure a and b at once and attribute the result to b.

**Arm c is a named refusal, and pointing at `protocol drive run` is the answer rather than a
placeholder.** A driven run is a walk of a step map whose every `llm` step goes through the seam with
the engine deciding each call. A second way to launch one would be a second policy to forget, which
is the mistake `epic:metaharness-migration` retired. What this verb does with a driven run is
**read** it: `protocol drive run` writes the stream, `protocol eval run --arm driven --stream <it>`
turns it into a manifest, and the matrix cannot tell which verb produced the pair.

## The gates before a cent

* `metaharness` on `PATH`, or `METAHARNESS_BIN`. Absent → refused by name, **exit 2** — its own code
  beside `1`, so *install something* and *fix what you wrote* are distinguishable without parsing
  stderr. Design constant 4: an absent binary is a skip, never a red gate.
* `METAHARNESS_LIVE=1`, or refused. *Installed* is not *permitted to spend*.
* `--budget-usd`, or refused. Checked **before** each launch, against `--assume-usd-per-run`
  (default `$0.25`), because a cap enforced afterwards is a receipt.
* `--cwd DIR`, or refused, with no default: arm a's agent has a shell and no enforcement, and the
  checkout holding the specification it is being measured against is the last tree to start it in.

A cost the wire writes as `null` is **unknown**, in both directions at once: it counts against the
budget at the assumed rate *and* the manifest states no cost at all. Writing `0` would make the run
look free; letting it count as nothing would let an unpriced wire spend without limit.

**Only a `null` gets the assumption**, and that took a live run to get right — see below. The ledger
charges the stream's stated cost where there is one, prints which of the two numbers it used
(`charged:  $0.797785 (stated)`), and a cost it cannot convert stops the run rather than quietly
becoming an estimate.

Amounts never touch a float, and there are **two** readers rather than one. `micro_usd` reads an
amount a person typed — `--budget-usd 5.00` — and refuses anything it cannot convert exactly, because
`--budget-usd 1e-7` is a mistake worth naming. `micro_usd_stated` reads a number a harness computed
and rounds it half-up to the nearest millionth, because a harness computes in binary floating point.
Neither multiplies by `1_000_000.0`: that is how `0.0714` becomes `71399`.

## Acceptance

- One verb produces the pair the matrix reads, plus the stream it was judged over. **Met**:
  `protocol eval run` (`crates/protocol-cli/src/eval.rs`),
  `a_spawn_gives_arm_raw_the_committed_instructions_and_arm_plugin_the_plugin` asserts all three
  products exist after a spawn.
- The instrument is constant and only the treatment varies. **Met**:
  `every_arm_is_spawned_by_one_instrument_and_only_the_treatment_varies` — the two argvs differ in
  exactly two words, and the test asserts the prefix is identical rather than listing both.
- The manifest's stream-side fields are read from the stream, and a stream that states none of them
  is refused. **Met**: `a_session_that_states_no_harness_version_is_refused_and_no_manifest_is_written`
  (`EVAL-STREAM-004`), `a_stream_of_another_harness_than_the_run_claims_is_refused`
  (`EVAL-STREAM-005`), `a_stream_that_stops_before_the_session_ends_is_refused_rather_than_reported_as_a_whole_run`
  (`EVAL-STREAM-010`), `a_session_whose_hermetic_row_is_missing_is_refused_by_the_field_that_is_missing`.
- A field the wire states as `null` is written down as `null`, and a field nobody wrote is refused.
  **Met**: `a_wire_that_names_no_model_assembles_a_manifest_that_says_so` and
  `a_session_that_omits_the_model_key_altogether_is_still_refused` through the binary,
  `an_omitted_model_is_refused_and_an_explicit_null_is_not` and
  `a_model_that_is_written_and_empty_is_refused_rather_than_read_as_unstated` beside the rule. The
  recorded live pilot stream re-ingests through this path and assembles `model: null`.
- The runner never writes a manifest its own reader refuses. **Met**: the assembled text is parsed
  back through `RawRunManifest`/`RunManifest` before it reaches the disk
  (`EVAL-STREAM-012`), and `the_manifest_the_runner_assembles_is_one_the_matrixs_own_reader_reads`
  covers the two functions directly.
- A missing binary is a skip and never a red gate. **Met**:
  `without_the_binary_the_runner_refuses_by_name_and_exits_two` asserts the refusal on a machine
  without it and **skips by name** on one with it, so the suite is green either way. Proven both
  directions: `task check` is green here with nothing named `metaharness` on `PATH`, and green again
  with a stub on it.
- Nothing spawns without the live flag, a cap and a tree. **Met**:
  `a_spawn_without_the_live_flag_is_refused_by_name_and_nothing_is_started`,
  `a_spawn_with_no_cap_on_what_it_may_spend_is_refused_by_name` — both assert the tool was **never
  started**, by checking the file a stub would have written.
- The cap stops the sweep before the run that would pass it. **Met**:
  `the_cap_stops_the_sweep_before_the_run_that_would_pass_it` — two cases, a `$0.60` cap and a
  `$0.5216` first run; the second is never launched and the stop names `EVAL-RUN-006` with both
  numbers.
- Arm `driven` is refused by name, pointing at the verb that does launch one. **Met**:
  `arm_driven_is_not_launched_here_and_the_refusal_names_the_verb_that_does`, and
  `a_run_of_arm_driven_is_read_even_though_it_is_not_launched_here` for the other half.
- Costs are exact. **Met**: `an_amount_becomes_millionths_by_integer_arithmetic_and_never_by_a_float`
  and `an_amount_this_reader_cannot_convert_exactly_is_refused_rather_than_rounded`. Verified by
  breaking it — making `cost_of` answer `Some(0)` for a `null` turns
  `a_cost_the_wire_writes_as_null_leaves_the_manifest_silent_and_never_says_zero` and the dry run's
  golden red together.
- A run whose stream states a cost is charged **that** cost, and the assumption reaches only a run
  that stated none. **Met**: `a_run_whose_stream_states_a_cost_is_charged_that_cost_and_never_the_assumption`,
  `the_assumed_rate_is_charged_only_where_the_stream_priced_nothing` — the second is what stops the
  first passing against a ledger that had merely stopped consulting the assumption — and
  `a_stated_cost_this_reader_cannot_convert_stops_the_run_instead_of_becoming_an_estimate`
  (`EVAL-STREAM-011`). Beside the rule: `a_cost_a_harness_computed_in_floating_point_is_read_and_not_refused`,
  `a_stated_cost_this_reader_cannot_convert_is_refused_rather_than_read_as_no_cost` and
  `a_person_typing_an_amount_is_still_held_to_an_exact_one`. Verified by breaking it in both places:
  charging `assumed` unconditionally turns the ledger tests red, and restoring the `.ok()` collapse
  turns the float-noise test red.

## What the second live run corrected

The ledger charged `$0.250000` for a Claude run whose stream stated `0.7977854999999999`.

Neither the ledger nor the field it read was wrong. **The cost reader refused the number and the
refusal was thrown away.** `0.7977854999999999` is the shortest text that round-trips the `f64` sum
of that run's per-turn costs, and `micro_usd` — written for amounts a *person* types — refuses more
than six decimal places on the reasoning that a cost it cannot convert exactly does not belong in a
committed document. `cost_of` then called `.ok()` on that refusal, which collapsed *there is a number
here I cannot convert* into *there is no number* — and the second of those is the one the ledger is
allowed to charge an estimate for.

So eighty cents were charged as twenty-five, **and the manifest carried no cost at all**, which is
the worse half: the matrix would have reported the sweep as cheaper than it was, with the coverage
column saying the run had simply not priced itself.

Three changes, each closing a different part of it:

| change | closes |
|---|---|
| `micro_usd_stated`, half-up to the nearest millionth by integer arithmetic | a wire's float noise is a cost, not a malformed amount |
| an unreadable stated cost is `EVAL-STREAM-011`, refused | unreadable is not unstated — invariant 5's reasoning one domain out |
| the ledger prints `charged:  $X (stated\|assumed)` per run | the failure was silent, and a silent ledger is one nobody audits |

`micro_usd` itself is unchanged and still strict, because a person typing an amount and a harness
computing one are different callers with different failure modes.

## How the whole thing is proven without spending anything

Two mechanisms, and neither is a mock inside the binary.

`--stream FILE` is the runner minus the spawn: the ingest half, reachable directly. It is not a test
hook — it is how a driven run enters the matrix and how a paid run is re-ingested when the manifest's
rules change — and it is what `story:eval-dry-run` runs the whole pipeline through.

For the spawn half, the tests write a `sh` script and point `METAHARNESS_BIN` at it. A **process** is
started with the argv the runner built, and its canned stream is ingested; a mock inside the binary
would have tested a seam this verb does not have.

## Out of Scope

- **Launching arm c.** Named above; `EVAL-RUN-004` is the deliverable, not a gap.
- **Assembling a matrix.** The runner writes pairs; `protocol eval matrix` reads them. A runner that
  also assembled would make the layout a private convention rather than a document.
- **Deciding anything on a run's verdict.** `eval run` exits `0` for a run that gapped, on
  `protocol trace evidence`'s reasoning: the verdict is in the record and the engine decides on it.
  Its exit code answers *was a run ingested*.
- **A paid run.** Nothing in this repository has spawned one. The runner is what R4 will use.

## Open Questions

**Should `--redact` be the default for the record beside a manifest?**
Decides: eval owner. Default if nobody answers: **no** — it stays opt-in, matching
`protocol trace check`, whose argument is that a report is most useful with its evidence visible.
Every record committed here is written with it, and the fixture README says so.

**Should the runner enforce that a case's `states:` are covered by the plugin under arm b?**
Decides: eval owner. Default: **no** — `integrations/workflow-coverage.yaml` already names the gaps,
and a case measuring what nothing teaches is a legitimate thing to run deliberately.
