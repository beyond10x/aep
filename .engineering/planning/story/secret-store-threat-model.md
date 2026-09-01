---
format: aep.planning-md/1
id: story:secret-store-threat-model
kind: story
status: archived
title: Define the encrypted secret-store threat model
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- serves: vision:O1
revision: 2
---
## Context

Replacing Vault removes a large operational dependency but does not remove custody, memory,
database-administrator, backup, or key-loss risks. The trusted computing base must be explicit
before selecting a service boundary or encryption mechanism.

## Acceptance

An accepted threat model names protected assets, trusted workloads, tenant boundaries, database and
cluster administrator assumptions, memory and backup exposure, failure modes, and explicit
non-goals for the encrypted secret store.
