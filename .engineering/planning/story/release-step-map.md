---
format: aep.planning-md/3
id: story:release-step-map
kind: story
status: draft
title: A release is cut by a step map with no model step
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:release-cutting-commands
- depends_on: story:command-step-expects-failure
revision: 3
---
# Story: A release is cut by a step map with no model step

## Outcome

`drivers/releases/source.yaml` drives `workflows/releases/source.yaml` with `kind: command` and
`kind: operator` steps only. Its steps run the commands `story:release-cutting-commands` provides:

- version, changelog heading, tag and status pins;
- gate, push, PR and merge;
- verify, then record the release in the planning store. `record` finds the Release run from the
  tag itself, so no URL is needed when the run starts.

This repository declares its own release in that map. Its first release cut that way is the
evidence for the epic's Done When.

## Context

`release/source` has no step map; only `adp/default/2` has one (`drivers/development/`). This
repository's release is a numbered list in `AGENTS.md` (Releases and commits) that an agent
follows. `cargo xtask release` checks a release but does not cut one (`xtask/src/main.rs:70-73`).
`workflows/releases/source.yaml` declares no transition out of `qualify` on a failed gate.

## Acceptance

- The map validates and declares no llm step.
- `aep drive run` over a fixture repository, with the forge calls stubbed, reaches `complete`. Its
  trace shows zero model calls, and every step named in the Outcome ran, the record step included.
- The merge step is `kind: operator`, or is a command gated on required checks reported green.
  The map says which.
- A failing gate step leaves the run at `qualify`, reported blocked, with no tag created and nothing
  pushed: `publish` is never entered.

## Scope

- `drivers/releases/source.yaml` (new)
- `workflows/releases/source.yaml`
- a new test file under `crates/drive/aep-driver/tests/`

Inferred: how the forge stub is declared.

## Depends on

- `story:release-cutting-commands`
- `story:command-step-expects-failure`: the map names the version as `{input.version}`.

## Open Question

Changelog prose for a reader may need a model. The default is that the changelog is written during
the work that changed it (`AGENTS.md`, Conventions), so the release step only renames the heading.
