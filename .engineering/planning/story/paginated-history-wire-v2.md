---
format: aep.planning-md/1
id: story:paginated-history-wire-v2
kind: story
status: implemented
title: Paginated history over service wire v2
summary: Bound history responses while retaining byte-compatible version-1 service clients.
relations:
- serves: vision:O2
revision: 5
---
## Context

Version 1 projects complete history as one array. A central realm needs bounded responses, but changing that route or response is a coordinated wire migration under Atlas ADR 0009.

## Acceptance

The semantic query contract and official client expose bounded cursor-based history through strict wire version 2, while version 1 request and response bytes remain accepted and its compatibility path returns complete history by draining all pages.

## Implementation

The contract now exposes `HistoryQueryV2` and `PageV2` under the strict `application/vnd.beyond10x.aep.v2+json` media type. The official client posts bounded queries to `/history/query`, follows opaque cursors until complete, and falls back to the byte-compatible v1 history route only when the first v2 negotiation is unsupported. Client tests prove multi-page draining and complete v1 fallback.
