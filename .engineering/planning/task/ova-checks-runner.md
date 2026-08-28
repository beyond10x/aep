---
format: aep.planning-md/1
id: task:ova-checks-runner
kind: task
status: implemented
title: 'run.sh: every check, one row each, an honest exit code'
summary: 'The runner the driver''s verifier invokes: one row per decomposed unit, a missing check script as a failed row, the table printed on every path, non-zero while any check fails.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
revision: 4
---
# Task: the runner, its table and its exit code

## What

**R16–R17.** `.engineering/checks/run.sh`: the one file
`drivers/development/checks.yaml`'s verifier invokes. It runs every check in the suite, prints one
row per decomposed unit naming the `task:` id that owns it, prints its table **on every path
including failure**, and exits non-zero while any check fails.

A unit whose `check-<unit>.sh` does not exist is a **failed row**, never a skipped one. The model is
`integrations/claude-code/eval/checks/run-checks.sh`, which is read and not edited.

This is the first task and it lands **red**: at `establish_verifiers` no check exists, so every row
reads `FAIL  no check exists`. That table is the state's product.

## Why

Twelve checks nobody runs together is twelve scripts. The exit code is what the driver reads, and a
suite that reports success for a check that did not happen is worse than no suite: it converts an
absent verifier into a green one.

## Done When

| # | Acceptance |
|---|---|
| N1 | `bash .engineering/checks/run.sh` from the repository root prints one row per unit, each naming the `task:` id that owns it. Every unit in this decomposition appears. |
| N2 | Exit 0 when every check passes; non-zero when any one fails. Shown both ways with a temporary always-red check placed in the directory and removed after. |
| N3 | A unit whose `check-<unit>.sh` is absent prints a `FAIL` row naming the unit, and the run exits non-zero. Never `SKIP`, never omitted from the table. Shown by moving one check aside. |
| N4 | The summary table is printed even when a check exits non-zero, writes to stderr, or kills itself. Shown with a check calling `exit 1` and one calling `kill -9 $$`. |
| N5 | `bash .engineering/checks/run.sh <unit> <unit>` runs only the named units and its table has exactly those rows. An unknown unit name is a failed row, not a silent no-op. |
| N6 | No check invocation reaches the network: `run.sh` contains no `curl`, `wget`, `nc`, `ssh` or `git fetch`/`clone`/`pull`, and the whole suite completes with those names shadowed by stubs that exit 127. |
| N7 | Scratch files go under `${TMPDIR:-$HOME/.cache/claude-tmp}`. No literal `/tmp` path appears in `run.sh`, and a run with `TMPDIR` pointed at an empty scratch directory creates its temporaries there and removes them on exit. |
| N8 | At `establish_verifiers`, before any sibling task lands, the run exits non-zero with every row reading that no check exists. Recorded as the red baseline this suite is measured from. |
| N9 | The summary table is printed when the runner's **own** scratch base cannot be created, not only when a check dies. Added by the adversarial pass. |

## What the adversarial pass found

N4 covered a check dying. It did not cover the runner failing before it reached one, and R16's
invariant — *the runner prints its table on every path* — has no exception for that.

`TMPDIR` pointed at a path that cannot be made produced a bare `mkdir:` line, exit 1, and **no
table**: `mkdir -p … || exit 1` and `mktemp … || exit 1` both left before the report. A report that
did not print is indistinguishable from a suite with nothing to say, which is this runner's own
stated reason for never using `set -e`. Failing to allocate scratch is now a harness failure like
any other — every selected unit red, the reason under it, the table printed.

## Notes

- Ordering in `ALL` follows the dependency order of the tasks, so a reader of a red table sees the
  root cause first — as the model runner does.
- `set -uo pipefail`, not `-e`: `-e` would abort before the report and break N4, which is the
  invariant *the runner prints its table on every path*.
- The unit names are these task slugs with the `ova-` prefix dropped, so a row and its owning
  artifact are one lookup apart.

## Verifier

`.engineering/checks/check-checks-runner.sh`. N1–N9 are its rows.

N2–N5 and N7 exercise the runner against scratch check directories under
`${TMPDIR:-$HOME/.cache/claude-tmp}`, never by mutating the real one. The check is the only member
of the suite that runs the runner, so it must not run the full suite recursively: it invokes
`run.sh` with an overridden checks directory.
