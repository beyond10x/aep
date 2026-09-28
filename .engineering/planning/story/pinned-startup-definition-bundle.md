---
format: aep.planning-md/3
id: story:pinned-startup-definition-bundle
kind: story
status: implemented
title: Load one pinned startup definition bundle
summary: Validate and content-address the definitions a service process evaluates.
relations:
- serves: vision:O2
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-08-31T08:25:35Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-31T08:25:35Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-31T09:41:20Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Context

The first service process needs an immutable definition registry, but dynamic registration and activation belong to a later lifecycle wave.

## Acceptance

A deterministic loader validates one definition tree, computes its SHA-256 digest from sorted relative paths and bytes, and refuses startup when the supplied expected digest does not name that validated bundle.

## Implementation

The project loader now walks a definition tree in sorted relative-path order, validates every definition before admitting the bundle, hashes path and file bytes with SHA-256, and compares the resulting digest with an optional startup pin. Tests cover stable digests, content and path changes, invalid definitions, and expected-digest refusal.
