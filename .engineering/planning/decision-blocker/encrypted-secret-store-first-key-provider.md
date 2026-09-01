---
format: aep.planning-md/1
id: decision-blocker:encrypted-secret-store-first-key-provider
kind: decision-blocker
status: open
title: Choose the first secret-store key provider and recovery root
relations:
- blocks: story:encrypted-secret-store-key-custody
- blocks: story:encrypted-secret-store-postgres
- blocks: epic:encrypted-secret-store-service
revision: 1
---
## Decision required

Choose the first supported key-encryption-key provider and recovery root for the generic service.

A cloud KMS keeps the KEK out of the cluster but requires deployment-owned cloud provisioning. A
Kubernetes-held development key removes that cloud dependency but makes cluster-secret backup and
operator recovery part of the root of trust. The service domain must remain provider-neutral either
way; this decision chooses the first adapter and the release acceptance evidence.

Implementation of persistent encrypted records is paused until the threat model and this recovery
choice are accepted.
