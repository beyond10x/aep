---
format: aep.planning-md/1
id: story:eval-run-pins-the-model
kind: story
status: draft
title: aep eval run --model pins the model a paid arm asks for
summary: Forward metaharness --model from aep eval run and record model_requested on the manifest; the bench plan fixes claude-sonnet-4-6 and the default is opus at $7.52 per planning run.
owner: eval
tags:
- bench
- eval
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 1
---
# Story: `aep eval run --model` pins the model a paid arm asks for

## Outcome

A bench phase that fixes the model — every paid phase in `beyond10x/bench`'s plan fixes `claude-sonnet-4-6`, the model the compared experiments used — can state it on the run, and the manifest records what was asked for and what the harness resolved.

## Context

`metaharness run claude --model <MODEL>` exists at 0.5.0 ("passed through to the vendor, which resolves it"); `aep eval run` at 0.44.0 has no `--model` and the spawn takes the harness default. On 2026-09-03 the first driven adopter case ran on `claude-opus-5[1m]` at $7.52 for the planning half alone; a per-run cap of $1.66 (P3's $50 over 30 runs) cannot be met on that model, so the first side-by-side pass ran at n=1 with $5 caps and the default model, labelled exploratory.

## Acceptance

- `aep eval run --model <MODEL>` is forwarded verbatim to `metaharness run claude --model`; refused by name on `codex` and `b10x` until their adapters take it.
- The run manifest records `model_requested` beside the existing `model` the attestation reports; `eval matrix` keeps them apart.
- A dry-run test shows the forwarded invocation; no paid run.

## Out of Scope

Choosing a model for the caller; the flag states, the harness resolves.

## Ambiguities

- `inferable` — metaharness's flag and semantics: `metaharness run claude --help` at 0.5.0.

## Open Questions

None.
