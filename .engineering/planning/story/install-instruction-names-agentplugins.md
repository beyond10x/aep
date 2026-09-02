---
format: aep.planning-md/1
id: story:install-instruction-names-agentplugins
kind: story
status: implemented
title: The install instruction names the marketplace that exists
relations:
- decomposes: epic:adopter-feedback-round-2
- serves: vision:O3
revision: 4
---
# Story: The install instruction names the marketplace that exists

## Outcome

A reader of this repository's README or website learns that the Claude Code and Codex plugins are
installed from `beyond10x/agentplugins`, and finds no instruction that names this repository's former
name as a marketplace source.

## Context

At tag 0.14.0 the Claude integration README said `/plugin marketplace add
beyond10x/engineering-protocols`. The split on 2026-09-01 removed `.claude-plugin/marketplace.json`
and `integrations/`; the adopter who reported on 2026-09-02 installed after that. Derived from the
epic this decomposes.

## Acceptance

- `README.md` has an "Agent plugins" paragraph that names `beyond10x/agentplugins` and its install
  page by URL, and says that this repository carries no plugin.
- `rg "marketplace add beyond10x/engineering-protocols"` over `README.md`, `docs/` and
  `website/docs/` returns nothing; any remaining mention of the former name is historical and says so.
- `CHANGELOG.md` gains an Unreleased line.
- `task check` passes.

## Out of Scope

The agent-plugins install page itself.

## Open Questions

None.
