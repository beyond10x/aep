---
format: aep.planning-md/1
id: story:fresh-transactional-command-session
kind: story
status: implemented
title: Evaluate central commands through a fresh transaction session
summary: Replace hydrate-all command execution with repository reads scoped to one command.
relations:
- serves: vision:O2
revision: 5
---
## Context

The existing durable adapters delegate semantics through a completely hydrated memory backend. That remains useful for local providers but is not the service write path accepted by Atlas ADR 0008.

## Acceptance

EP command evaluation can use a repository session that reads and locks only the targets, references and records one command needs, while the memory repository remains a conformant reference implementation and the complete accepted or refused result is returned for one outer transaction to commit.

## Implementation

`SessionPostgresBackend` opens one Entity Runtime PostgreSQL session per command, reserves identity space, loads and locks only the applied-intent record plus the command's direct entity and relation dependencies, evaluates through the existing semantic backend over that bounded view, and commits the complete atomic batch in the same outer transaction. Semantic refusals commit their audit result; provider failures roll the session back. The memory-backed adapter remains the conformance reference.
