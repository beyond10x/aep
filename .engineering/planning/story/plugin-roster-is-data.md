---
format: aep.planning-md/1
id: story:plugin-roster-is-data
kind: story
status: implemented
title: The plugin's agent roster is data an adopter can read
summary: Shipping two agents turned an adopter's gate red because it pins the roster by hand.
tags:
- plugin
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 4
---
# Story: The plugin's agent roster is data an adopter can read

## Outcome

An adopter that pins the plugin's agent list reads it from one file the plugin publishes, not from a
hand-copied array.

## Context

`11727595` #3806: shipping `adversary` and `implementor` turned harness's gate red — `crates/harness-cli/src/agents.rs:479`
pins the roster. Fixed there by hand (`5493cea`).

## Acceptance

- `integrations/claude-code/.claude-plugin/plugin.json` or a sibling file lists skills and agents;
  `plugin-check` fails when the list and the directories disagree.

## Out of Scope

Changing harness.
