---
format: aep.planning-md/1
id: story:skill-loads-before-the-store-is-touched
kind: story
status: implemented
title: The skill is in context before the store is touched, and a dispatch names its agent
summary: An adopter hand-authored 13 artifacts 3 min before the planning skill loaded; a coordinator dispatched general-purpose and called it the implementor.
tags:
- plugin
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 4
---
# Story: The skill is in context before the store is touched, and a dispatch names its agent

## Outcome

An adopter's first store is written the way § 5 of the planning skill says, and a coordinator that
substitutes a built-in agent for a plugin one says so.

## Context

- `e70b8018 s1#91-#104`: 13 artifacts hand-authored from `AGENTS.md`/`docs/`; the planning skill entered
  context at `s1#195`, 3 min later. `reverse scan`, `reverse history`, `reverse-engineer`: 0 uses in the
  one repository whose store was empty.
- `cc946bc3#496`: "I demoted a tool grant to a paragraph and then reported it as 'running the
  implementor'" — two `general-purpose` dispatches, caught by the operator.
- `11727595`: 47 of 47 sub-agents `general-purpose`, charters retyped per launch.

## Acceptance

- `skills/planning/SKILL.md` `description` triggers on adoption words and on a repository without
  `.engineering/`.
- `skills/wave/SKILL.md`: the dispatch line in the report names the `subagent_type`; a non-plugin agent
  where a plugin one exists is a deviation to report, not a substitution to make.

## Out of Scope

Making Claude Code load a skill earlier than its trigger fires.
