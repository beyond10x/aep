---
format: aep.planning-md/1
id: story:agents-md-is-generated-where-it-counts
kind: story
status: implemented
title: '`AGENTS.md` carries generated counts and an index, not a 67 KB re-read'
summary: AGENTS.md says twenty gate steps, Taskfile has 21; the 67 KB file was re-read as an oversized tool result nine times in one session.
tags:
- docs
relations:
- decomposes: epic:declared-configuration-invariants
- serves: vision:O6
- informed_by: story:prose-that-the-tree-contradicts
revision: 4
---
# Story: `AGENTS.md` carries generated counts and an index, not a 67 KB re-read

## Outcome

Numbers in `AGENTS.md` that describe the tree are generated like the website's, and a sub-agent can read
the section it needs without the whole file.

## Context

- `AGENTS.md:384,475`: "Twenty steps in `Taskfile.yml`"; `check:` has 21 `- task:` entries
  (`9c286ad7#155`; verified 2026-08-30). The same drift was fixed once already at `:361/:437`.
- `431986de` `tool-results/`: `AGENTS.md` (897 lines, 67 KB) re-read as an oversized tool result 9 times in
  one session, ~450 KB of context.
- `story:prose-that-the-tree-contradicts` (active) is the general rule; this is its `AGENTS.md` instance.

## Acceptance

- `cargo xtask status` owns a generated region in `AGENTS.md` for the gate step count and step list, and
  `status-check` fails when it drifts.
- `AGENTS.md` opens with a section index (`## <name>` → one line), and the wave skill's dispatch brief names
  the sections a sub-agent must read rather than "read `AGENTS.md`".

## Out of Scope

Splitting `AGENTS.md` into files.
