---
format: aep.planning-md/1
id: story:postgres-advisory-lock-runtime-pin
kind: story
status: implemented
title: Adopt valid PostgreSQL advisory locks
summary: Publish the fresh command session against Entity Runtime's corrected text lock key.
relations:
- serves: vision:O2
revision: 7
---
## Context

A live central-service run found that Entity Runtime 0.17.1 injected a NUL into every absent-identity advisory lock. After that correction, the first live EP indexed lookup exposed a second provider binding defect. ER 0.17.3 corrects both boundaries, but EP's released fresh-session backend continues to compile against the faulty generation until EP republishes its coordinated pin.

## Acceptance

Every EP Entity Runtime dependency resolves to 0.17.3, the fresh PostgreSQL command-session conformance test passes against a live server, and a patch release makes that dependency generation consumable by aep-service.

## Implementation

The workspace now resolves every Entity Runtime dependency from tag 0.17.3. EP's fresh PostgreSQL command session therefore uses valid pairwise advisory-lock hashing and text-to-JSONB containment binding without changing EP's semantic or wire contracts.
