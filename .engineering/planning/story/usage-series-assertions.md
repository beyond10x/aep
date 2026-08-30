---
format: aep.planning-md/1
id: story:usage-series-assertions
kind: story
status: draft
title: Assertions over the per-request usage series
summary: A vocabulary for sequences — the cache-read ramp is monotone, cache creation is front-loaded, no request takes more than a share of the total — over data the IR already keeps.
owner: trace
tags:
- trace
relations:
- decomposes: epic:checker-vocabulary-depth
revision: 2
---
# Story: Assertions over the per-request usage series

## Outcome

An author can state how a run's usage should *move* — the cache-read ramp is monotone, cache creation
is front-loaded, no single request takes more than a share of the total — instead of only what it
totalled, which is what catches a context strategy that has quietly stopped working.

## Context

Deferred by the design itself, not by the wave running out of time. The data is already retained:
`TraceIr::requests` keeps every assistant event's usage. What is missing is a vocabulary for
*sequences*, which a single-field matcher does not have — and designing one under the previous wave's
deadline would have been designing it for a different feature.

## Acceptance

- A specification can assert a monotone trend over a named usage field across the request series, and
  gaps with the index of the first request that broke it.
- A specification can assert that no single request exceeds a stated share of a run's total.
- A run with a single request satisfies a trend assertion vacuously rather than gapping — an
  assertion about a sequence of one is not a failure.
- A field this transcript does not carry is `unk`, as everywhere else.

## Out of Scope

Statistics. No means, no percentiles, no smoothing — a trend the reader cannot recompute by looking at
the cited events is a verdict nobody can check.

## Open Questions

Whether the sequence vocabulary is shared with any future ordering assertions. Decides: trace owner,
when the second consumer exists — one consumer cannot justify a general mechanism.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** or **inferred**.

- **Primary surface:** the `trace-spec/1` expectation vocabulary — declared in `crates/trace-domain`, evaluated in `crates/trace-spec` — cited, `TraceIr::requests` grepped to `crates/trace-domain/src/ir.rs:668`
- **Files:** `crates/trace-domain/src/spec.rs:198` (`ExpectationKind`), `:823` (`name`), `:884` (`NAMES`) — cited, where a kind is declared and published
- **Files:** `crates/trace-domain/src/raw.rs:348` (`RawExpectationKind`), drift test at `:3144` — cited, the authored spelling has its own enum
- **Files:** `crates/trace-spec/src/check.rs:90` (`evaluate`) — cited; `:87` states the dispatch is exhaustive on purpose, so an unevaluated kind fails to compile
- **Files:** `crates/trace-spec/src/check.rs:2499` — cited, the per-kind coverage guard demands a positive **and** a negative case per published name
- **Files:** `crates/trace-domain/src/ir.rs:569` (`AssistantRequest`), `:668` (`TraceIr::requests`) — cited, the retained data. Nothing in `check.rs` reads it today, so this story is its first consumer
- **Files:** `schemas/generated/trace-spec.schema.json` — cited, regenerated from `RawTraceSpec`, held by `task schema-check`
- **Symbols:** `ExpectationKind`, `::NAMES`, `::name`, `RawExpectationKind`, `check::evaluate`, `TraceIr::requests`, `AssistantRequest` — cited
- **Also likely:** the hard-coded kind count in **six** places — `crates/trace-domain/src/spec.rs:1087` (`assert_eq!(NAMES.len(), 51)`), `crates/trace-domain/src/lib.rs:12` and `:25`, `crates/trace-spec/tests/check.rs:3`, `README.md:78`, `website/docs/status/where-this-stands.md:181` — cited, each says "fifty-one" and each goes stale
- **Documents:** `docs/plan/trace-wave-1-transcript-checker.md:55` — cited, the deferred row naming this story's three assertions
- **Confidence:** high — the story's own symbol grepped to a path, the vocabulary is one enum behind a compiler-enforced exhaustive dispatch, and two precedent commits (`a0780f1`, `180f441`) show the exact file set an added kind touches
- **Would collide with:** any unit touching the `trace-spec/1` expectation vocabulary — the `ExpectationKind`/`RawExpectationKind` pair, `check::evaluate`'s dispatch, the generated schema, or the kind-count literal. **Note the literal reaches `README.md`**, so this unit conflicts with any wave-mate that edits the README.

**Not established.** Whether `AssistantRequest` must gain an IR event index — the acceptance wants "the index of the first request that broke it", but it carries only `source_line`. The wire spelling of the new kinds; nothing like `usage.trend` or `series.*` exists, and `NAMES` is asserted sorted, so the insertion point depends on a name nobody has chosen. Whether the two blind readers (`codex.rs:146`, `shell_echo.rs:577`, both passing an empty `requests`) are in scope — every series assertion is permanently `unk` under them. Whether the committed fixtures can satisfy the coverage guard's negative case.
