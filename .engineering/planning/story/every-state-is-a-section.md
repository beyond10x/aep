---
format: aep.planning-md/1
id: story:every-state-is-a-section
kind: story
status: proposed
title: workflow flow makes every state a section, so the governor is asked at every state boundary
summary: The projection groups only multi-step states and retreats; a single-step state is a bare node and the transition hook never fires for it, so a bare-workflow native walk was governed at the root only
owner: protocol
tags:
- flow
- governor
relations:
- decomposes: epic:cross-harness-portability
- serves: vision:O3
revision: 3
---
# Story: `workflow flow` makes every state a section

## Outcome

`b10x-harness workflow run` asks its `transition` hook on each side of a **group** boundary
(harness design 0003 § 3). `protocol workflow flow` emits a group for a state with more than one
step and for a retreat span, and a bare step node for everything else — so which states the
governor is consulted about depends on how many steps a map gave them. The fifth paid native walk
(metaharness `native-eval.II7pgK`, 2026-08-29, `EVAL_FLOW_MAP=none`) was consulted four times, all
at `root`; `receive` and `specify` came out clean with nothing asked of the engine. The mapped walks
were consulted twelve times, at `root`, `receive` and `specify` — the two states the eval's map
gave two steps — and never at `decompose`.

After this story every state is a section: a one-step state is a group of one, named after the
state, so `enter` and `leave` are asked of `protocol drive transition` at every state boundary,
and "governed at every section boundary" means every state.

## Acceptance

- `protocol workflow flow --id adp/default` (with or without `--map`) emits one group per
  non-terminal state; retreat spans stay groups of groups. `workflow plan` shows every state as a
  section.
- A native walk's `hook-ran` count at `transition` equals 2 × states entered (+ attempts), not 2 ×
  groups-that-happened-to-have-two-steps.
- The projection header says so; the website's integrate-a-harness page says so.

## Evidence

A `flow` CLI test asserting the shape; a re-run of `run-native.sh` whose census shows the
consultations per state.
