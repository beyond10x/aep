---
format: aep.planning-md/1
id: story:scope-as-a-typed-field
kind: story
status: implemented
title: A story's scope is a typed field a verb sets
summary: 'aep artifact scope <id> --add <path> [--inferred] writes a scope: list in frontmatter; show and list --format json return it as data.'
owner: protocol
tags:
- store
- wave
relations:
- decomposes: epic:wave-derivation
- serves: vision:O2
- informed_by: story:a-story-records-where-it-lands
revision: 4
---
# Story: A story's scope is a typed field a verb sets

## Outcome

A coordinator or the scoper agent records which paths a story touches through the CLI, and `aep artifact show` and `list --format json` return them as data rather than as a markdown section somebody parses.

## Context

`story:a-story-records-where-it-lands` (draft) declares the `## Scope` section and reports stories without one; it puts a machine-readable field out of scope because `artifact` had no verb that edits one frontmatter key. This story adds that verb. The section stays: the field is what a computation reads, the section is what a person reads, and the scoper writes both.

## Acceptance

- `aep artifact scope <id> --add <path> [--inferred]` and `--remove <path>` write a `scope:` list in the frontmatter, each entry carrying `cited | inferred`, and bump the revision once per change.
- `aep artifact show` prints the list; `--format json` carries it as an array.
- `aep artifact validate` reports (without failing) a non-draft story whose `scope` is empty, in the same tier `story:a-story-records-where-it-lands` chooses for the section.
- The journal records the write like any other mutation.

## Out of Scope

Deriving scope from a diff after the fact. That is a different verb over git, and belongs to evidence, not planning.

## Ambiguities

- `inferable` — the field name and its values: `scope`, entries `path` + `confidence: cited|inferred`, matching the scoper's vocabulary (`agentplugins/plugins/adp/agents/story-scoper.md:59-72` at 0.4.0).

## Open Questions

None.
