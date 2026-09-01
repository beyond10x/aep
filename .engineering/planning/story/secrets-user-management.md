---
format: aep.planning-md/1
id: story:secrets-user-management
kind: story
status: draft
title: Expose user-managed arbitrary secrets
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-api
- depends_on: story:encrypted-secret-store-postgres
- serves: vision:O1
- serves: vision:O5
revision: 1
---
## Context

People need a generic place to store arbitrary secrets and to control secrets created on their
behalf. The user surface must not let ownership widen a non-revealable Connector credential into a
plaintext disclosure path.

## Acceptance

An authenticated user can create, read, CAS-update, list metadata for, revoke, and delete an owned
revealable secret; can list metadata for and revoke/delete an owned non-revealable service-created
secret; cannot reveal that value or act across tenants; and receives stable lifecycle and refusal
responses suitable for Devcenter.
