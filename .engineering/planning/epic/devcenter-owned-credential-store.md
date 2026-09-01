---
format: aep.planning-md/1
id: epic:devcenter-owned-credential-store
kind: epic
status: active
title: Devcenter owns its credential store lifecycle
summary: Package, operate, recover, and verify an optional Vault without a predecessor deployment dependency.
relations:
- serves: vision:O4
- serves: vision:O5
revision: 3
---
## Outcome

Devcenter can provision and operate its own optional internal Vault with durable Raft storage, automatic unsealing, bounded workload authority, and tested backup and recovery. Existing Connectors credentials move without entering application, Helm, Kubernetes Secret, or log surfaces.

## Acceptance

- The public chart remains configuration-neutral and packages a pinned optional Vault dependency.
- The private application and deployment CLI own lifecycle, migration, verification, and recovery semantics.
- A live deployment survives Vault and Connectors restarts and completes a user-bound agent task.
- A native snapshot is restored successfully before the predecessor Vault is removed.
- Atlas records the ownership and runtime-edge change with deployment evidence.
