---
format: aep.planning-md/1
id: story:implementor-counts-what-ran
kind: story
status: implemented
title: The implementor reports how many cases ran, and answers the class not the instance
summary: Executed-case count per lane beside the exit status (a lane ran 8 of 58 and was green), corrections that answer the class, and two rules retyped into every prompt.
tags:
- plugin
- wave
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
- informed_by: story:implementor-and-adversary-agents
revision: 4
---
# Story: The implementor reports how many cases ran, and answers the class not the instance

## Outcome

`agents/implementor.md` requires the numbers that separate a green exit from a green run.

## Context

| # | change | evidence |
|---|---|---|
| a | report executed-case count per lane, before and after, beside the exit status | SUBRETRO §8 + addendum: `scripts/delegated-lane.sh:41` selected by substring; the lane ran 8 of 58 host cases; dropping the filter 8 → 64 and one had always been red (`6f1be0d9#172`) |
| b | a correction names the class the finding is an instance of and shows the rest of the class is clean; where machine-checkable, the fix is the check | SUBRETRO addendum 2: one refusal code added, four left absent from bundle `0.9.0` while `xtask/src/bundle.rs:880-883` states the rule |
| c | the build-directory rule (inside the worktree, `AGENTS.md:493-502`) lives in the charter | `9c286ad7#166` written into every prompt by hand |
| d | a `## Scope` mechanism claim is a labelled hypothesis, confirmed before it is built on | RETRO3 P12; `114c2340#177` "I applied exactly that one move and re-ran: 5 red of 5" |
| e | a file the unit does not own is returned as a patch, not edited | RETRO3 P4; `114c2340#205` |

## Acceptance

- The report template has a per-lane row `executed before / after / exit`.
- Correction handling has a "class" step.
- The hard rules list carries the build-directory rule and the not-owned-file rule.

## Out of Scope

The gate-side count check — `story:gate-lanes-count-what-ran`.
