---
format: aep.planning-md/1
id: story:secrets-revocation-lifecycle
kind: story
status: draft
title: Separate local and upstream secret revocation
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:secrets-resource-model
- depends_on: story:encrypted-secret-store-api
- serves: vision:O1
revision: 1
---
## Context

Local access revocation and provider-side credential revocation are different operations. Secrets
can deny future local reads immediately, while only the creating integration understands how to
revoke an upstream credential.

## Acceptance

The contract and event model distinguish immediate local revoke, asynchronous upstream-revocation
request/observation, tombstone, and cryptographic destruction; remain provider-neutral; are
idempotent under retries; and never report deletion complete while an accepted retention or
upstream-revocation obligation remains unresolved.
