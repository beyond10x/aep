---
format: aep.planning-md/1
id: story:eval-run-passes-plugin-through
kind: story
status: draft
title: aep eval run --arm plugin passes a marketplace plugin through to metaharness
summary: Forward metaharness 0.5.0 --plugin <repo>@<name>@<pin> from aep eval run, record the attested plugins in the manifest; what bench needs for a third-party arm.
owner: eval
tags:
- bench
- eval
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
scope:
- confidence: cited
  path: crates/protocol-cli/src/eval.rs
- confidence: inferred
  path: crates/protocol-cli/tests/eval_run.rs
revision: 3
---
# Story: `aep eval run --arm plugin` passes a marketplace plugin through to metaharness

## Outcome

A bench arm that installs a third-party marketplace plugin runs under `aep eval run` the same way an arm with a checked-out `--plugin-dir` does, so `beyond10x/bench` can put `dev-team@bfinster` beside `aep-planning@beyond10x` through one driver.

## Context

metaharness 0.5.0 added `metaharness run claude --plugin <repo>@<name>@<pin>` (placement into the hermetic scratch home, attested). `aep eval run` at 0.44.0 knows `--plugin-dir` only; `bench run` refuses a third-party arm by name until this lands (`bench` `story:both-stacks-one-corpus`, `story:runner-wraps-aep-eval`).

## Acceptance

- `aep eval run --plugin <repo>@<name>@<pin>` (repeatable) is forwarded verbatim to `metaharness run claude`; an unpinned value is refused before spawning with metaharness's own wording.
- The run manifest records the plugins the attestation listed.
- `--plugin` and `--plugin-dir` may be combined; the manifest keeps them apart.
- A dry-run test shows the forwarded invocation; no paid run.

## Out of Scope

Codex and b10x arms; metaharness refuses them by name today.

## Ambiguities

- `inferable` — the option's shape and refusal: metaharness `docs/design/runs-side-by-side-v0.1.md` and `CHANGELOG.md` 0.5.0.

## Open Questions

None.
