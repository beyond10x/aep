---
format: aep.planning-md/3
id: story:spend-against-baseline
kind: story
status: active
title: Spend per task kind is reported against a baseline and a target
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:evidence-records-task-spend
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning/spend.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/artifact_spend.rs
- confidence: cited
  path: crates/govern/aep-domain/src/project.rs
- confidence: cited
  path: ess/domains/evidence.yaml
- confidence: cited
  path: ess/domains/plan.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T06:23:57Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-10-15T06:23:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":8}}}
---
# Story: Spend per task kind is reported against a baseline and a target

## Outcome

A read verb, `aep plan artifact spend [--kind <k>] [--since <YYYY-MM-DD>] [--baseline <n>]`,
prints spend per task kind against the baseline and target the project declares. It writes
nothing and reads no clock.

**Task kind** is the kind of the artifact a `task_spend` record names: the evidence file's `kind`
field (`story`, `task`, …). A **task** is one artifact. An artifact with several `task_spend`
records counts once.

- Records of one artifact that share a `source_input` are snapshots of one cumulative source, such
  as a driven run's cursor or a manifest recorded twice. Only the latest by `at` counts. At equal
  `at`, the record whose evidence id sorts last counts.
- Each figure of the artifact is the sum over the records that count, one per `source_input`. A
  figure absent from any of them is unknown for that artifact.

**Rows.** One row per task kind and figure, kinds in name order, figures in this order:
`model_calls`, `total_tokens`, `cost_micro_usd`, `wall_time_ms`, `llm_steps_run`
(`SpendFigures` names). Columns:

| column | value |
|---|---|
| `kind`, `figure` | as above |
| `tasks` | artifacts of that kind with a `task_spend` record |
| `skipped` | of those, artifacts whose figure is unknown; they take no part in the median |
| `median` | median over the others; `unknown` when none is known, never `0` |
| `baseline`, `target` | from the project file; `none` when not declared |
| `standing` | `under` (median < target), `at` (median = target), `over` (median > target), `unknown` (median unknown), `undeclared` (no target) |

**Median.** Over the known values sorted ascending: the middle value for an odd count, the lower
of the two middle values for an even count. Always an integer the store holds.

**Filters.** `--kind <k>` prints only rows of kind `k`. `--since <date>` keeps only records whose
`at` is at or after 00:00 UTC of that date, before artifacts are summed; a value that is not a
calendar date is refused, naming it, and no table is printed.

**Baseline.** `--baseline <n>` computes each median over the first `n` artifacts of each kind,
ordered by their earliest record's `at`, then by artifact id. A person copies the medians into the
project file. The verb never writes the project file.

**Project file.** `.engineering/project.yaml` gains an optional `spend` mapping: task kind, then
figure (one of the five above), then `baseline` and `target`, both non-negative integers and both
required:

```yaml
spend:
  story:
    model_calls: { baseline: 12, target: 10 }
    cost_micro_usd: { baseline: 4200000, target: 3500000 }
```

A target not strictly below its baseline, an unknown figure name, or an entry missing either value
refuses the project file, naming the kind, the figure and the values.

## Context

The `review-value` table reads cost only from an eval manifest that a review's `--ref` points to
(`planning.rs`, the table `review-value` prints). No view aggregates spend over tasks. Spend is
recorded by `aep plan artifact evidence <id> --kind task_spend --from <file>` (`TaskSpend`,
`SpendFigures`; `aep.evidence.RecordSpend`). The project file is `ProjectConfig` in
`crates/govern/aep-domain/src/project.rs`, modelled as `aep.plan.Project` in `ess/domains/plan.yaml`;
`findings_required_since` is the precedent for an optional key refused on load.

## Acceptance

Specification first, green before any code:

- `ess/domains/plan.yaml` declares the `spend` field on `aep.plan.Project`, a bound type with
  `baseline` and `target` and the invariant `target < baseline`, and the refusal
  `aep.plan.SpendTargetNotBelowBaseline`.
- `ess/domains/evidence.yaml` declares the read command `aep.evidence.ReportSpend` (display
  `aep plan artifact spend`), the enum `aep.evidence.SpendStanding` (`under`, `at`, `over`,
  `unknown`, `undeclared`), the outcomes `reported` and `since-not-a-date`, and an ESS-LIMIT note
  naming the host function and test for the median and the sum, which ESS 0.60.0 cannot express.
- `ess specify validate` passes under the pinned `ess` 0.60.0; `cargo xtask ess` rewrites
  `generated/ess/`; `ess-gate` passes with `SYNTHESIZED` in `xtask/tests/ess_gate.rs` raised by the
  scenarios synthesized for `reported`, `since-not-a-date` and the refused bound.

Over a fixture store, in `crates/edge/aep-cli/tests/artifact_spend.rs`, with three stories whose
`task_spend` records state `model_calls` 8, 12 and 20 (`at` on 2026-01-01, -02 and -03), one task
with no `model_calls`, and `spend.story.model_calls: { baseline: 25, target: 10 }`:

- The `story`/`model_calls` row prints `tasks 3`, `skipped 0`, `median 12`, `baseline 25`,
  `target 10`, `over`; the `task`/`model_calls` row prints `tasks 1`, `skipped 1`,
  `median unknown`, `standing unknown`. Exit 0.
- With target 15 the row prints `under`; with target 12, `at`; with no `spend` key, `none`, `none`,
  `undeclared`.
- `--baseline 2` prints `median 8` for `story`/`model_calls` (records of 01-01 and 01-02; the lower
  middle of 8 and 12).
- In a second store, a story with two records stating 3 and 4 calls, from two different
  `source_input`s, prints `tasks 1`, `median 7`. Two more records for that story share one
  `source_input`: 5 calls at 2026-01-04 and 9 at 2026-01-05. The row then prints `median 16` (3 + 4
  + 9), not 21.
- `--kind story` prints no `task` row. `--since 2026-01-02` prints `story`/`model_calls` with `tasks 2`, `median 12` (the
  lower middle of 12 and 20). `--since 2026-13-40` exits nonzero naming the value and prints no
  table.
- `target: 25` against `baseline: 25` makes `aep plan artifact validate` exit 1, naming `story`,
  `model_calls`, 25 and 25; a figure key `tokens` is refused, naming it.
- Two runs over one store print identical bytes.
- `cargo xtask schema` regenerates `schemas/generated/project.schema.json`; `schema-check` and
  `docs-check` pass.

## Scope

Specified first:

- `ess/domains/plan.yaml`, `ess/domains/evidence.yaml`
- `generated/ess/` (by `cargo xtask ess`)
- `xtask/tests/ess_gate.rs` (`SYNTHESIZED`)

Then:

- `crates/govern/aep-domain/src/project.rs` (`ProjectConfig` `spend`, its refusal)
- `schemas/generated/project.schema.json` (by `cargo xtask schema`)
- `crates/edge/aep-cli/src/planning.rs` (the verb), and a new `crates/edge/aep-cli/src/planning/spend.rs`
- `crates/edge/aep-cli/tests/artifact_spend.rs` (new)
- `website/docs/reference/cli.md`, `CHANGELOG.md` (Unreleased)

## Depends on

`story:evidence-records-task-spend`.
