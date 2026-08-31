---
format: aep.planning-md/1
id: story:driven-run-cost-ceiling
kind: story
status: implemented
title: A driven run cannot start an unbounded paid session
summary: Every model session reserves an operator-declared assumed charge before launch, and the durable run stops before the next charge would cross its dollar cap.
relations:
- decomposes: epic:reference-driver
- serves: vision:O3
revision: 5
---
# Story: A driven run cannot start an unbounded paid session

## Outcome

`protocol drive run` refuses to launch an `llm` step unless the operator named a dollar cap and a
conservative charge per session. Before every metaharness spawn, the driver proves the next charge
fits; when it does not, the run stops with the cap, spent amount, and unlaunched step in its durable
report.

## Evidence

- `AGENTS.md` section *Paid runs* requires `METAHARNESS_LIVE=1`, `--budget-usd`, and
  `--assume-usd-per-run`, checked before each launch. `protocol eval run` implements that contract;
  `protocol drive run --help` currently offers neither budget flag.
- Metaharness native walk `NJQjho` cost an estimated $3.73 under an enforced $5 rate-card ceiling.
  The comparable driven launcher cannot currently state the same outer bound, so launching it would
  make the comparison's safety envelope differ by arm.

## Acceptance

- A map with an `llm` step is refused before allocating a run id, taking the store lock, or spawning
  metaharness unless `METAHARNESS_LIVE=1`, `--budget-usd`, and `--assume-usd-per-run` are all present.
- Dollar values are parsed as exact integer microunits; zero, negative, malformed, and sub-microunit
  values are refused by name rather than rounded or represented as floats.
- The assumed charge is reserved immediately before each metaharness spawn. A next launch that would
  exceed the cap does not spawn and stops the run as budget-exhausted with cap, spent, attempted
  charge, and launch count in its report.
- A resume remembers the cap and assumed charge from `launch.json`; it cannot silently acquire a new
  or larger budget. An explicit lower cap may narrow the remainder, while a higher one is refused.
- Command-only maps remain free and require neither the live opt-in nor a dollar budget.
- Tests mutation-prove both boundaries: removing the pre-launch comparison causes a fake executor to
  launch once too many, and moving the initial refusal after run allocation leaves a directory the
  test detects.

## Scope

- `crates/protocol-cli/src/drive.rs` — CLI flags, durable launch record, preflight and metaharness
  spawn reservation.
- `crates/aep-driver/src/executor.rs`, `crates/aep-driver/src/run.rs` — a typed spend-stop outcome
  that is neither model failure nor retry exhaustion.
- `crates/protocol-cli/tests/drive_cli.rs` and adjacent unit tests — observable CLI and mutation
  guards.
- `CHANGELOG.md` and CLI reference documentation — the paid-run contract users must name.

## Out of scope

Reading provider invoices or inventing a price when a transcript reports none. This bound is the
operator's conservative per-session assumption, checked before the effect; token-derived accounting
may make the assumption tighter later but cannot replace the launch-time ceiling.

## Implementation

The driver now represents spend exhaustion as a typed run stop, while the CLI parses dollar values into exact integer microunits, refuses every paid map before run allocation unless all three opt-ins are present, and persists the cap, assumed charge, reservations, launch count, and spent amount. Every metaharness spawn reserves its charge first; resume can inherit or narrow the durable ceiling but cannot raise it. Driver and CLI tests hold the pre-allocation and pre-spawn ordering, including the one-launch-too-many mutation guard, and command-only maps retain their free path.
