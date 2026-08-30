---
format: aep.planning-md/1
id: story:fan-out-survives-a-rate-limit
kind: story
status: implemented
title: A fan-out survives a rate limit without re-dispatching finished work
summary: Four sessions lost 4-9 agents to HTTP 429 and re-dispatched finished work; the skill says nothing about it.
tags:
- plugin
- wave
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
- informed_by: story:wave-as-a-surface
revision: 4
---
# Story: A fan-out survives a rate limit without re-dispatching finished work

## Outcome

The wave skill says what a coordinator does when HTTP 429 kills agents mid-flight, and sizes N so it
happens less.

## Context

- `431986de#1500-#2074`: 9 of 32 sub-agents killed by a model limit; ~8 relaunches of identical work.
- `2e81f991#138-#145`: four adversaries and one implementor killed at once; recovered at `#183`.
- `4d4c15a4#215`: N=6 opus agents → 429, four killed, 47 min stall, each resumed by hand.
- `e70b8018 s1#1201`: one agent died on a limit; the session idled 3 h.

## Acceptance

- Pre-flight in `skills/wave/SKILL.md` names N from the remaining budget the operator states, defaulting to 4.
- On a 429: record the unit's branch head and stage in the wave page; resume the same unit with a brief that
  says what is already on the branch; never re-dispatch a unit whose branch has commits.

## Out of Scope

Reading the account's limits programmatically.
