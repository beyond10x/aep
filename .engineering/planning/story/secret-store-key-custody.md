---
format: aep.planning-md/1
id: story:secret-store-key-custody
kind: story
status: archived
title: Choose secret-store key custody and recovery
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- serves: vision:O1
- serves: vision:O4
revision: 2
---
## Context

Envelope encryption still needs a key-encryption key outside PostgreSQL. The design must compare a
dev-account KMS key with Kubernetes-held master material, including rotation, clean-cluster restore,
operator recovery, and the consequences of losing each component.

## Acceptance

An accepted key-custody design demonstrates rotation and a clean-cluster restore from separately
protected PostgreSQL backup and KEK recovery material without persisting plaintext or root material
in source, Helm values, manifests, logs, or temporary files.
