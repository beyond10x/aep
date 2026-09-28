---
format: aep.planning-md/3
id: story:postgres-test-schema-cleanup-race
kind: story
status: implemented
title: PostgreSQL backend tests serialize shared cleanup
summary: Prevent parallel test cleanup from racing migrations in the shared public schema.
relations:
- serves: vision:O2
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-08-31T10:58:30Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-31T10:58:30Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-31T11:01:25Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Context

The 0.36.2 release gate failed after all product changes landed because parallel PostgreSQL tests each opened an ER store in `public` solely to drop their private schema. Those incidental migrations raced in PostgreSQL's type catalog even though every test's actual data schema was isolated.

## Acceptance

Schema cleanup remains test-only, serializes the shared public-schema connection, the complete gate passes with PostgreSQL enabled, and the corrected patch is published without moving the failed 0.36.2 tag.

## Implementation

The PostgreSQL backend test module now serializes only its shared cleanup helper. Each product test still runs in parallel against a private schema, while the incidental `public`-schema migration used to obtain a drop handle can no longer race another cleanup. The backend suite passed ten consecutive live runs and the complete gate passed with PostgreSQL enabled.
