---
format: aep.planning-md/1
id: story:secret-store-contract
kind: story
status: archived
title: Define the opaque secret-store contract
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- depends_on: story:secret-store-threat-model
- serves: vision:O1
- serves: vision:O5
revision: 2
---
## Context

The service boundary must stay smaller than Connectors: opaque bytes and versions in, authorized
opaque bytes out. Connectors owns OAuth exchange and refresh. Identity validates generic authority
but must not acquire provider or relying-party vocabulary.

## Acceptance

A versioned contract implements create, authorized read, compare-and-swap update, metadata read,
tombstone, and rotation through exact short-lived workload identity, derives tenant server-side,
and refuses plaintext enumeration, cross-tenant access, and unrelated services.
