---
format: aep.planning-md/1
id: story:eval-run-preflights-the-child-path
kind: story
status: implemented
title: aep eval run refuses to spawn against a stale child aep
summary: Resolve aep and ess on the PATH metaharness constructs for the session, compare versions, refuse a mismatch before any spend (EVAL-RUN-017/018).
owner: eval
tags:
- bench
- eval
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 4
---
# Story: `aep eval run` refuses to spawn against a stale child `aep`

## Outcome

A live spawn is refused, before any money is spent, when the `aep` the session will execute is not the version that launched it; a case whose subject names an `ess-schema` skill is refused when the session has no `ess` to run.

## Context

metaharness gives the session a constructed `PATH` — `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin` — and never the runner's own. On 2026-09-03 the golden-path case ran with a 0.40.1 copy in `~/.local/bin` beside the 0.44.0 that launched it: step 3 was drafted by hand because `ess` was not there, and step 8 stopped at `aep doctor: unrecognized subcommand` after $10.96 had been spent. Nothing said the two binaries disagreed.

## Acceptance

- Before a live spawn the runner resolves `aep` on the child's `PATH` exactly as metaharness constructs it, reads its `--version`, and refuses a mismatch by name (`EVAL-RUN-017`) with both paths and both versions; an absent child `aep` is a printed warning.
- A case whose `subject.skills` names `ess-schema:*` is refused (`EVAL-RUN-018`) when the child's `PATH` has no `ess`.
- `--stream` ingestion is untouched; the test suite runs under a scratch `HOME` so a developer's own `~/.local/bin` does not decide what it reports.
- `task install` refreshes a real `~/.local/bin/aep` and `~/.local/bin/protocol` after the cargo install.
