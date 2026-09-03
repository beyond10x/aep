---
format: aep.planning-md/1
id: story:eval-run-caps-the-run
kind: story
status: draft
title: aep eval run --budget-usd is the session's own ceiling
summary: Pass what is left of the cap to each Claude run as metaharness run claude --max-budget-usd; the between-runs check stays.
owner: eval
tags:
- bench
- eval
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: `aep eval run --budget-usd` is the session's own ceiling

## Outcome

The cap a sweep is launched under stops a run while it runs, instead of being compared against the bill afterwards.

## Context

`--budget-usd` was checked before each spawn against the assumed per-run cost and nowhere else. The golden-path case stated $10.96 against `--budget-usd 5` on 2026-09-03 and the only thing the number did was appear in the report's cost row. Claude Code has `--max-budget-usd` for print mode, which is the mode metaharness launches; metaharness 0.6.1 adds `run claude --max-budget-usd` and passes it through.

## Acceptance

- Each spawned Claude run receives `--max-budget-usd <what is left of the cap>` on its metaharness argv, computed from the cap and what earlier runs in the sweep charged.
- The flag is Claude Code only; codex and b10x argvs are unchanged (metaharness refuses the option for codex by name).
- A run the vendor stops on budget ends with a terminal record the checker reads as not completed; nothing here turns that into a pass.
