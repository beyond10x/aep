---
format: aep.planning-md/1
id: story:secrets-resource-model
kind: story
status: draft
title: Define ownership, disclosure, grants, and lifecycle
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-threat-model
- serves: vision:O1
- serves: vision:O5
revision: 1
---
## Context

The service must represent both ordinary user-managed values and non-revealable values created by a
service on a user's behalf without encoding a special Connector or provider type. Ownership,
disclosure, use authority, and lifecycle are independent dimensions.

## Acceptance

An accepted resource model defines server-derived tenant and owner, creator provenance, opaque
value versions, revealable versus non-revealable disclosure, scoped workload grants, active,
revoked, tombstoned and destroyed semantics, retention, and audit events such that user ownership
allows revoke/delete without automatically allowing value disclosure.
