---
format: aep.planning-md/1
id: story:devcenter-vault-operations
kind: story
status: active
title: Operate Vault through devcenterctl
summary: Initialize, reconcile, verify, snapshot, and recover Vault without persisting root material.
relations:
- decomposes: epic:devcenter-owned-credential-store
- serves: vision:O4
revision: 3
---
## Acceptance

- Rust commands provision bounded cloud resources and initialize or reconcile Vault idempotently.
- Automatic unseal is the normal restart path; one recovery share is held only in the operator keyring.
- The initial root token is zeroized and revoked after restricted Kubernetes roles exist.
- Tokens and recovery material never enter argv, environment, Helm, Kubernetes Secrets, temporary files, or logs.
