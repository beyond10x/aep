---
format: aep.planning-md/1
id: story:encrypted-secret-store-postgres
kind: story
status: draft
title: Implement transactional PostgreSQL envelope encryption
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-threat-model
- depends_on: story:encrypted-secret-store-key-custody
- depends_on: story:encrypted-secret-store-api
- serves: vision:O1
- serves: vision:O4
- depends_on: story:secrets-service-repository
revision: 1
---
## Context

The data plane needs authenticated envelope encryption, transactional versioning, migrations, and
useful audit evidence without secret bytes. Concurrent writers must not overwrite a newer value.

## Acceptance

Integration and adversarial tests prove random per-record DEKs and nonces, authenticated
tenant/resource/version context, tamper and wrong-key refusal, zeroizing plaintext buffers,
transactional CAS under races, tombstones, schema migration, key rotation, and append-only
byte-free audit events against PostgreSQL.
