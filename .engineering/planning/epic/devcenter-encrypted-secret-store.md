---
format: aep.planning-md/1
id: epic:devcenter-encrypted-secret-store
kind: epic
status: draft
title: Devcenter encrypted secret-store lifecycle
summary: Evaluate and deliver a minimal PostgreSQL-backed encrypted credential store without provider or agent semantics.
relations:
- informed_by: epic:devcenter-owned-credential-store
- serves: vision:O1
- serves: vision:O4
- serves: vision:O5
- depends_on: epic:encrypted-secret-store-service
- supersedes: epic:devcenter-owned-credential-store
revision: 2
---
## Outcome

Devcenter composes the released generic encrypted secret-store service and Connectors uses it for
opaque credential custody. Provider exchange and refresh remain in Connectors, Identity remains
service-agnostic, and agents receive only attempt-bound authority rather than durable secrets.

## Acceptance

- Private deployment values pin released chart and image digests without carrying secret bytes.
- Connectors migrates selected records with conflict refusal and byte-for-byte read-back.
- Restart, restore, Claude credential refresh, and one user-bound agent task pass through the new
  custody path before the predecessor store is retired.
- Atlas records the released service and runtime dependency with deployment evidence.
