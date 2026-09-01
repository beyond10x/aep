---
format: aep.planning-md/1
id: story:devcenter-vault-backup
kind: story
status: active
title: Back up and restore the Devcenter Vault
summary: Continuously retain encrypted Raft snapshots and prove one can be restored.
relations:
- decomposes: epic:devcenter-owned-credential-store
- serves: vision:O4
revision: 3
---
## Acceptance

- A least-privilege workload streams a Vault-native Raft snapshot to a private, encrypted, versioned object store every six hours.
- Snapshot object names contain no tenant, actor, provider, or credential identifiers.
- A disposable isolated restore drill proves the latest snapshot becomes an initialized, unsealed, readable Raft store.
- Backup age, backup failure, sealed state, and storage pressure are observable.
