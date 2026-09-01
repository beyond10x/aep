---
format: aep.planning-md/1
id: story:secrets-service-repository
kind: story
status: draft
title: Establish the top-level Secrets service repository
relations:
- decomposes: epic:encrypted-secret-store-service
- serves: vision:O4
- serves: vision:O5
revision: 2
---
## Context

Secrets is a top-level beyond10x service and must have the same delivery and provenance boundary as
the other service repositories before code or artifacts can depend on it.

## Acceptance

A public `beyond10x/secrets` repository with GitHub Issues disabled, bot-authenticated history,
Rust workspace gates, confidential-marker checks, public generic service images and chart,
configuration-neutral release policy, AEP references, and Atlas repository/release-unit/component
registration is ready for implementation without any deployment-specific identifier.
