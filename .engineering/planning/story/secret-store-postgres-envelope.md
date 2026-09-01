---
format: aep.planning-md/1
id: story:secret-store-postgres-envelope
kind: story
status: archived
title: Implement transactional PostgreSQL envelope encryption
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- depends_on: story:secret-store-threat-model
- depends_on: story:secret-store-key-custody
- depends_on: story:secret-store-contract
- serves: vision:O1
- serves: vision:O4
revision: 2
---
## Context

The data plane needs authenticated envelope encryption, transactional versioning, and useful audit
evidence without secret bytes. Concurrent OAuth refresh must not overwrite a newer credential.

## Acceptance

Integration and adversarial tests prove random per-record DEKs and nonces, authenticated
tenant/resource/version context, tamper and wrong-key refusal, zeroizing plaintext buffers,
transactional CAS under refresh races, tombstones, key rotation, and append-only byte-free audit
events against PostgreSQL.
