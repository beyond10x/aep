---
format: aep.planning-md/1
id: story:encrypted-secret-store-operations
kind: story
status: draft
title: Package and recover the generic secret store
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-postgres
- serves: vision:O4
revision: 1
---
## Context

The generic chart may compose the service and PostgreSQL, but deployment coordinates and recovery
material belong in the private deployment boundary. Backups are incomplete unless key recovery is
tested separately.

## Acceptance

The configuration-neutral chart deploys digest-pinned internal-only workloads with durable storage,
bounded networking, health and byte-free metrics, scheduled encrypted backups, and a passing
clean-namespace restore drill; the private deployment supplies all environment coordinates.
