---
format: aep.planning-md/1
id: story:retire-predecessor-vault
kind: story
status: active
title: Remove the predecessor Vault runtime dependency
summary: Destroy the old runtime only after migration, restart, backup, and restore proofs pass.
relations:
- decomposes: epic:devcenter-owned-credential-store
- serves: vision:O4
revision: 3
---
## Acceptance

- The old Vault remains untouched until source/target equality, task execution, automatic unseal, snapshot, and restore gates pass.
- After those gates, its release, data and audit claims, copied trust material, obsolete keyring entries, bootstrap declarations, endpoint, and network edge are removed.
- The deployed namespace and private deployment configuration contain no reference to the predecessor service.
