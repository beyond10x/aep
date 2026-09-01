---
format: aep.planning-md/1
id: story:devcenter-compose-encrypted-secret-store
kind: story
status: draft
title: Compose the released secret store in Devcenter
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- depends_on: story:encrypted-secret-store-operations
- serves: vision:O4
revision: 1
---
## Context

The private Devcenter deployment consumes released generic services; it does not fork their source
or carry their cryptographic policy. The deployment must supply concrete PostgreSQL, key-provider,
network, storage, and image coordinates without putting credential bytes into Git, GitLab, Helm, or
Kubernetes manifests.

## Acceptance

The private release lock and values render and deploy digest-pinned secret-store and PostgreSQL
workloads on internal networking, bind only the exact Connectors workload and tenant, expose no
credential or key material in rendered output, and pass health and authority-refusal checks.
