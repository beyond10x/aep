---
format: aep.planning-md/1
id: story:encrypted-secret-store-api
kind: story
status: draft
title: Define the opaque secret-store API
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-threat-model
- serves: vision:O1
- serves: vision:O5
- depends_on: story:secrets-resource-model
- depends_on: story:secrets-service-repository
revision: 2
---
## Context

The API serves people and workloads from one generic resource model. Provider exchange, OAuth
refresh, upstream revocation, agent leases, and product behavior stay with callers. Identity
validates generic authority but acquires no consumer or provider vocabulary.

## Acceptance

A versioned contract implements create, authorized value read, compare-and-swap update, metadata
read/list, grant management, immediate revoke, tombstone/delete, and key rotation; derives tenant,
owner and delegated creator server-side; enforces disclosure policy; and refuses cross-tenant,
unrelated-workload, stale-version, revoked, deleted, and expired-authority operations by name.
