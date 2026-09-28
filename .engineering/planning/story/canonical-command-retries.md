---
format: aep.planning-md/3
id: story:canonical-command-retries
kind: story
status: implemented
title: Compare canonical intent for scoped command retries
summary: Make durable replay depend on caller-controlled intent rather than command identity alone.
relations:
- serves: vision:O2
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-08-31T08:25:37Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-31T08:25:37Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-31T09:41:21Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Context

Version 1 already decides that realm, workspace, authority and key identify a retry. The existing memory and entity adapters persist only command id and result, so different intent can currently be mistaken for a replay.

## Acceptance

A stored applied command carries canonical caller-controlled intent, equal intent returns the original result across executors of one authority, different intent is refused by name, and legacy intent-less records never replay as though equivalence had been proven.

## Implementation

Command idempotency now persists canonical `CommandIntent`: caller-controlled command type, payload, realm, workspace, authority, and key, excluding executor identity. Equal intent returns the original result across executors of one authority; different intent is refused as an idempotency conflict; legacy records without intent cannot be treated as proven replay equivalence. Memory and entity-backed tests cover all three cases.
