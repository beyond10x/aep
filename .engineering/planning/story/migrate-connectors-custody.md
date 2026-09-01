---
format: aep.planning-md/1
id: story:migrate-connectors-custody
kind: story
status: active
title: Migrate Connectors credentials without exporting them to product APIs
summary: Copy the current tenant KV state through a bounded administrative migration and cut Connectors over.
relations:
- decomposes: epic:devcenter-owned-credential-store
- serves: vision:O1
- serves: vision:O5
revision: 3
---
## Acceptance

- Migration accepts source authority only on stdin and copies current live tenant values through zeroized memory.
- It skips byte-identical targets, refuses conflicts and tenant escapes, and reports counts without paths or values.
- Connectors survives restart, refreshes an OAuth credential in integration coverage, and completes a live user-bound agent task against the new store.
- Disconnect destroys both value and metadata in conformance coverage.
