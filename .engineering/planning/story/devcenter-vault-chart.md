---
format: aep.planning-md/1
id: story:devcenter-vault-chart
kind: story
status: active
title: Package an internal Vault in the Devcenter chart
summary: Render an internal-only, persistent, auto-unsealed Vault and wire only its CA to Connectors.
relations:
- decomposes: epic:devcenter-owned-credential-store
- serves: vision:O4
revision: 3
---
## Acceptance

- Vault is disabled by default and the locked upstream chart is packaged into the public OCI chart.
- Enabled values require immutable image identity, TLS, retained Raft and audit claims, an explicit seal, and internal-only networking.
- Connectors receives the Vault service address and CA projection without receiving the serving private key.
- Generic fixtures and rendered artifacts contain no downstream deployment identifiers.
