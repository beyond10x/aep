---
format: aep.planning-md/3
id: story:pagination-cursors
kind: story
status: implemented
title: Every query cursor advances through deterministic pages
summary: Parse and apply offsets for entity, relation and audit queries.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-30T22:08:51Z", actor: "human:operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-30T22:08:51Z", actor: "human:operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-30T23:50:44Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Finding

The memory backend emits `offset-<n>` cursors but ignores `after`, so following a cursor returns the first page again and durable adapters inherit the defect.

## Acceptance

Queries parse the existing opaque cursor, refuse malformed values with a typed error, filter and order deterministically, skip the offset, then apply the limit. A next cursor exists only when more matches remain and advances by the returned count. Entity, relation and audit pagination traverse complete result sets without duplicates; a deliberately faulty backend proves the conformance test is load-bearing.

## Scope

- `crates/aep-backend-memory/` query implementation and `crates/aep-contract/` errors/suites — cited.
- durable backends through shared conformance — inferred; no separate pagination implementation expected.
