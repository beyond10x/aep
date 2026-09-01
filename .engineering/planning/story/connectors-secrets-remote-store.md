---
format: aep.planning-md/1
id: story:connectors-secrets-remote-store
kind: story
status: draft
title: Implement the Connectors remote-store adapter
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-postgres
- depends_on: story:encrypted-secret-store-api
- serves: vision:O1
- serves: vision:O5
revision: 1
---
## Context

Connectors is the first service consumer. It owns provider OAuth exchange, refresh concurrency,
upstream revocation, and credential semantics. Secrets supplies durable opaque versions and exact
workload authority without learning providers or exposing values to agents or Devcenter.

## Acceptance

The Connectors remote-store adapter creates and compare-and-swap updates non-revealable user-owned
secrets through delegated workload authority, reads values only for its own tenant and purpose,
observes immediate revocation, and passes refresh-race, cross-tenant denial, restart, and migration
tests against the released Secrets API.
