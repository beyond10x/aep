---
format: aep.planning-md/1
id: story:waves-derived-from-scope
kind: story
status: draft
title: Waves are derived from scope, not paired by hand
relations:
- decomposes: epic:adopter-feedback-round-2
revision: 1
---
# Story: Waves are derived from scope, not paired by hand

## Outcome

`aep artifact waves` reads `depends_on` edges and each story's machine-readable scope and prints
the waves that can run in parallel, exiting non-zero on a dependency cycle or two stories on one
surface.

## Context

Filed from the 2026-09-02 review of a third-party plugin, which does this in a script from a plan's
scope lines. Here it belongs in Rust. Not scheduled; recorded so the idea has a place. Depends on
`## Scope` becoming a frontmatter field or a fenced list the CLI can parse.

## Acceptance

- The verb exists, is deterministic, and the wave skill in the agent-plugins repository stops
  describing pairwise overlap in prose.

## Out of Scope

Running the waves.

## Open Questions

Where scope lives on the artifact — operator decides.
