---
format: aep.planning-md/1
id: story:machine-readable-service-contract
kind: story
status: draft
title: Publish the service wire as machine-readable schema and routes
summary: Give consumers schema-enabled wire DTOs and one typed route/media catalog without changing v1 or v2 bytes.
relations:
- serves: vision:O2
- serves: vision:O6
- informed_by: story:aep-service-wire-and-client
scope:
- confidence: cited
  path: crates/aep-client
- confidence: inferred
  path: crates/aep-conformance
- confidence: inferred
  path: crates/protocol-cli/src/serve
revision: 4
---
## Context

The independently implemented service needs to publish OpenAPI from the EP-owned wire. Deriving schemas or routes again in the service would create a second source of truth beside the DTOs and client paths already verified by the conformance corpus.

## Acceptance

Every public request/response wire DTO implements JSON Schema from its actual Serde shape; one typed catalog names every method, path template and accepted media type; the official client consumes the catalog rather than private path literals; generated schemas accept every constructed conformance document; and no existing v1/v2 encoded byte changes.
