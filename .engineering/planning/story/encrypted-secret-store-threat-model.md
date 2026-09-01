---
format: aep.planning-md/1
id: story:encrypted-secret-store-threat-model
kind: story
status: draft
title: Define the generic secret-store threat model
relations:
- decomposes: epic:encrypted-secret-store-service
- serves: vision:O1
revision: 1
---
## Context

Replacing Vault removes a large operational dependency but does not remove custody, memory,
database-administrator, backup, or key-loss risks. The generic service's trusted computing base
must be explicit before selecting encryption or authentication mechanisms.

## Acceptance

An accepted threat model names protected assets, trusted workloads, tenant boundaries, database and
cluster administrator assumptions, memory and backup exposure, failure modes, abuse cases, and
explicit non-goals for the generic encrypted secret store.
