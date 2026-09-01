---
format: aep.planning-md/1
id: story:retire-vault-secret-store-path
kind: story
status: draft
title: Retire the Vault credential-store path after cutover
relations:
- decomposes: epic:devcenter-encrypted-secret-store
- depends_on: story:migrate-connectors-to-encrypted-secret-store
- serves: vision:O4
revision: 1
---
## Context

Vault planning and implementation history must remain explainable. Neither the prototype nor the
live predecessor store is removed merely because a replacement design exists.

## Acceptance

After the PostgreSQL design is accepted and its migration, restore, refresh, and agent-task gates
pass, AEP records the supersession, the untagged Vault prototype is removed from the release line,
the predecessor deployment is deleted, and Atlas records the final runtime ownership with evidence.
