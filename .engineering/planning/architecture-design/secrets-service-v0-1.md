---
format: aep.planning-md/1
id: architecture-design:secrets-service-v0-1
kind: architecture-design
status: draft
title: Secrets service 0.1 architecture and delivery plan
relations:
- designs: epic:encrypted-secret-store-service
- informed_by: story:secrets-resource-model
- informed_by: story:encrypted-secret-store-key-custody
- informed_by: story:encrypted-secret-store-api
- informed_by: story:connectors-secrets-remote-store
revision: 2
---
## Decision scope

Create `secrets` as a top-level beyond10x service, parallel to the other foundation services. Its
first released capability is encrypted remote storage for Connectors. The same resource model later
supports user-managed arbitrary secrets and user revocation/deletion of secrets created on their
behalf.

## Repository and ownership

The public `beyond10x/secrets` repository owns:

- a Rust multi-crate workspace;
- domain types and validation;
- authenticated envelope encryption and key-provider ports;
- PostgreSQL migrations and transaction adapters;
- principal verification and authorization policy;
- HTTP/OpenAPI and Rust client contracts;
- the service binary, operator commands, embedded documentation, tests, container image, and a
  configuration-neutral Helm chart.

It owns no provider integration, OAuth refresh logic, agent lease, deployment hostname, tenant,
cluster coordinate, or live key. AEP is the issue tracker; GitHub Issues remain disabled.

## Workspace shape

- `secrets-domain`: validated resources, lifecycle, grants, versions, refusals, and commands; no IO.
- `secrets-crypto`: envelope format, authenticated context, zeroizing buffers, and key-provider port.
- `secrets-postgres`: schema migrations, CAS transactions, audit append, and repository adapter.
- `secrets-auth`: generic verified principal and trusted workload context ports.
- `secrets-http`: versioned routes, OpenAPI, error/refusal mapping, health, and byte-free metrics.
- `secrets-client`: generated or contract-tested Rust client used by Connectors and Devcenter.
- `secrets-app`: configuration and service composition.
- `secretsctl`: migration, key rewrap, verification, and restore-drill operations.

Crate boundaries may be consolidated while small, but domain, crypto, storage, and transport remain
one-way dependencies and independently testable.

## Resource and authority model

A secret resource carries server-derived tenant and owner, creator provenance, opaque metadata,
disclosure policy, lifecycle state, current version, and grants. Values are opaque bytes.

- `revealable`: an authorized owner may read the value.
- `non_revealable`: ownership permits metadata, revoke, and delete, but not plaintext disclosure.
- `active`: authorized reads and leases may succeed.
- `revoked`: all future reads and leases fail immediately and idempotently.
- `tombstoned`: absent from ordinary listings; retained only for audit/retention obligations.
- `destroyed`: wrapped DEK is removed after retention, making retained ciphertext unusable once
  backup retention also expires.

Capabilities are explicit: metadata read/list, value read, create-version, grant, revoke, delete,
and rotate. Tenant, owner, workload, and delegated creator are never accepted as unverified payload
authority.

## Encryption and PostgreSQL

Each value version gets a random 256-bit DEK and unique nonce. An accepted AEAD encrypts the value
with authenticated tenant id, secret id, version, disclosure policy, and envelope-format version.
The DEK is independently wrapped by the current KEK version. PostgreSQL stores ciphertext, nonce,
wrapped DEK, wrap nonce, algorithm and key versions, resource metadata, grants, lifecycle, and
append-only audit events. It never stores a plaintext value or KEK.

The dev profile starts with a random KEK in a pre-created Kubernetes Secret mounted read-only as a
file. The secret name is a deployment coordinate; key bytes never enter Helm values, Git, CI
variables, argv, or environment. Operator recovery material is held separately from the cluster.
The key-provider port permits a KMS adapter later without changing records or API. Rotation writes
with the new KEK and transactionally rewraps DEKs without decrypting values.

The first PostgreSQL profile is one retained persistent replica for the dev cluster. The service
accepts an external PostgreSQL DSN Secret so production HA remains a deployment concern rather than
a domain change.

## Authentication

The application consumes verified-principal and trusted-context ports rather than parsing product
semantics in the domain.

- Connector background access uses a short-lived Kubernetes service-account token with an exact
  audience. Deployment configuration binds that exact service account and tenant; TokenReview and
  authorization fail closed.
- User access uses Identity-issued access tokens for the Secrets audience and generic Secrets
  scopes. Identity registers bytes opaquely and learns no consumers or providers.
- An interactive Connector operation preserves both the user owner and Connector workload actor in
  audit provenance; neither is accepted from an unsigned request field.

## API slices

### 0.1 — Connector remote store

Create a non-revealable user-owned secret under delegated authority; read metadata; read the value
only as the exact Connector workload; CAS a new value version; revoke; tombstone; and observe stable
refusals. Add the Connectors adapter behind configuration while retaining its existing backend for
rollback.

### 0.2 — user-managed secrets

Create, read, CAS-update, list metadata, revoke, and delete owned revealable secrets. Permit an owner
to list metadata for and revoke/delete a non-revealable service-created secret without revealing its
value. Local revoke is immediate; upstream provider revocation remains a Connector operation whose
request and observation may be correlated through provider-neutral events.

## Chart composition

The Secrets repository publishes a configuration-neutral optional chart supporting external
PostgreSQL and an existing KEK Secret. The Devcenter chart consumes the released Secrets chart as an
optional dependency and enables a retained single-replica PostgreSQL profile for development.
Connectors receives only the internal Secrets address and workload configuration. The chart exposes
no Secrets ingress for the first slice; later user routes pass through the Devcenter BFF or a
separately authorized internal route.

Private Devcenter deployment values pin the public charts and private images by version and digest,
supply storage classes and Secret names, and never contain key, database password, or secret values.

## Delivery order

1. Register the repository and service/runtime arrows in Atlas and create the public repository
   with Issues disabled.
2. Accept threat model, resource model, API, envelope format, key-provider, and recovery design.
3. Implement the domain/crypto/PostgreSQL kernel with unit, property, known-answer, tamper, wrong-AAD,
   nonce-uniqueness, CAS-race, and migration tests.
4. Implement exact workload authentication, HTTP/OpenAPI, client, audit, health, and byte-free
   metrics; run PostgreSQL integration and API conformance suites.
5. Publish Secrets 0.1.0 image/client/chart and verify anonymous public visibility plus
   configuration neutrality.
6. Add the Connectors remote-store adapter, migration command, and dual-configuration rollback path;
   release Connectors.
7. Add optional Secrets/PostgreSQL dependencies to the Devcenter chart, render both disabled and
   enabled profiles, release Devcenter, and pin all artifacts in the private deployment repository.
8. Bootstrap only the dev namespace's PostgreSQL credentials and KEK Secret, with KEK recovery held
   outside the cluster; deploy through the existing dev-cluster delivery path.
9. Quiesce Connector writes briefly, migrate the selected tenant records in memory, refuse
   conflicts, verify byte-for-byte read-back, switch the backend, and retain rollback.
10. Prove restart, wrong-tenant refusal, Connect Claude refresh, one user-bound agent task,
    PostgreSQL backup, and a clean-namespace restore with separately recovered KEK material.
11. Remove the predecessor Vault path only after those gates pass and record final evidence in AEP
    and Atlas.
12. Deliver the 0.2 user-managed API and Devcenter Secrets UI without widening non-revealable
    Connector credentials.

## Release gates

- strict Rust formatting, linting, tests, documentation, and dependency checks;
- crypto known-answer/property/tamper/wrong-key tests and secret-bearing failure-output scans;
- PostgreSQL migration, CAS concurrency, restart, backup, and restore tests;
- API conformance for every allowed and refused capability/lifecycle combination;
- chart lint/render with disabled, embedded-dev, and external-PostgreSQL profiles;
- confidential-marker and deployment-coordinate scans over source, docs, OCI labels, and packages;
- dev-cluster end-to-end evidence before any predecessor deletion.

## Explicit non-goals for 0.1

Public ingress, a user UI, provider integrations, upstream token revocation, dynamic database
credentials, PKI, arbitrary code execution, agent-readable secrets, multi-region replication, and
production PostgreSQL HA.
