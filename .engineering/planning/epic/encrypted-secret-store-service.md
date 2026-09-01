---
format: aep.planning-md/1
id: epic:encrypted-secret-store-service
kind: epic
status: draft
title: Generic encrypted secret-store service
summary: Publish a provider-neutral PostgreSQL envelope-encryption service and configuration-neutral chart.
relations:
- serves: vision:O1
- serves: vision:O4
- serves: vision:O5
revision: 2
---
## Outcome

The beyond10x organization publishes a general `secrets` service for encrypted, tenant-scoped
secret resources. Its first consumer is Connectors through a remote-store adapter; later, people
can create and manage arbitrary secrets directly through the same resource and authority model.

## Resource boundary

- PostgreSQL stores ciphertext and wrapped per-record data-encryption keys only.
- A key-encryption key provider lives outside PostgreSQL behind a narrow recovery contract.
- Every secret has server-derived tenant, owner, creator, version, disclosure policy, lifecycle
  state, and grants; its value remains an opaque byte string to the domain.
- Ownership does not imply disclosure. A Connector-created secret may be non-revealable to its user
  owner while still allowing that owner to inspect metadata, revoke future use, and delete it.
- Revocation immediately denies future reads and leases. Deletion tombstones and crypto-shreds
  according to retention. Revoking an upstream vendor credential remains a Connector operation.
- The service has no provider integrations, OAuth refresh logic, PKI, dynamic database credentials,
  agent-facing API, or deployment-specific defaults.

## Delivery order

1. Publish the generic resource, authority, encryption, audit, and recovery contracts.
2. Fulfil Connectors' remote secret-store contract and migrate one tenant with rollback.
3. Add user APIs for arbitrary secrets, including value management where disclosure policy permits
   and revoke/delete authority over user-owned Connector-created secrets.
4. Expose the user management surface through Devcenter without moving credential custody into its
   frontend or backend.

## Acceptance

- Database inspection, manifest rendering, logs, metrics, and backups reveal no plaintext secret.
- Wrong-key, tamper, cross-tenant, unrelated-workload, stale-version, revoked, deleted, and
  expired-authority paths are refused deterministically.
- A user can create and manage a revealable arbitrary secret and can revoke/delete a non-revealable
  Connector-created secret without gaining its value.
- Connectors refresh, restart, key rotation, PostgreSQL backup, and clean-cluster restore are
  demonstrated before the predecessor store is retired.
- Source, contracts, chart, docs, release history, and OCI artifacts live in a dedicated beyond10x
  repository; concrete deployment values remain downstream.
