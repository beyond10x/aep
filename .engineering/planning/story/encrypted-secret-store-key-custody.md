---
format: aep.planning-md/1
id: story:encrypted-secret-store-key-custody
kind: story
status: draft
title: Define provider-neutral key custody and recovery
relations:
- decomposes: epic:encrypted-secret-store-service
- depends_on: story:encrypted-secret-store-threat-model
- serves: vision:O1
- serves: vision:O4
- depends_on: story:secrets-service-repository
revision: 1
---
## Context

Envelope encryption needs a key-encryption key outside PostgreSQL. The service must expose a narrow
provider contract that can support a cloud KMS and a Kubernetes-held development key without
making either deployment model part of the domain.

## Acceptance

An accepted key-custody design specifies provider-neutral wrap/unwrap and rotation semantics and
demonstrates clean-cluster restore from separately protected PostgreSQL backup and KEK recovery
material without persisting plaintext or root material in source, Helm values, manifests, logs, or
temporary files.
