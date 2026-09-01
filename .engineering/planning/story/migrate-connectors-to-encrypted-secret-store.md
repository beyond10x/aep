---
format: aep.planning-md/1
id: story:migrate-connectors-to-encrypted-secret-store
kind: story
status: draft
title: Migrate Connectors custody to the encrypted secret store
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- depends_on: story:devcenter-compose-encrypted-secret-store
- serves: vision:O1
- serves: vision:O5
revision: 1
---
## Context

The live Connectors deployment remains on the predecessor Vault path. Migration must preserve exact
credential values and refresh semantics while keeping rollback possible and never routing durable
secrets through Devcenter, Agent Platform, Identity, Helm, or Kubernetes Secrets.

## Acceptance

Connectors migrates its selected tenant records with conflict refusal and byte-for-byte read-back,
then completes a Claude credential refresh and one user-bound agent task through the new store while
the old path remains recoverable until reconciliation succeeds.
