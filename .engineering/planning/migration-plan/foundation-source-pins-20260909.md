---
format: aep.planning-md/1
id: migration-plan:foundation-source-pins-20260909
kind: migration-plan
status: draft
title: Compose verified foundation source revisions
relations:
- informed_by: story:one-entity-runtime-pin
- serves: vision:O2
revision: 2
---
# Verified foundation source composition

## Scope

Use Entity Runtime revision `faadc04f2f273517e21815d32ba3866f3aea7642`
across all six workspace inputs in `Cargo.toml` and `Cargo.lock`. Use
Docs System revision `8eb302de78feba319fbdd8191270d9fe7b6c1244` in
`website/package.json` and `website/package-lock.json`. Preserve frozen
fixtures and the existing asynchronous documentation workflow bytes.

## Acceptance

The exact inputs remain committed, `task check` succeeds, and the candidate
passes dependency admission and applicable source checks before publication
on the default branch. This composition authorizes no tag or release.

## Compatibility finding

The first gate failed in `aep-backend-entity/tests/events.rs` because its
handwritten status ladder did not declare the emitted creation event.
`aep-backend-entity/src/lib.rs` explicitly persists `Decision::legacy_import`:
these events describe AEP commands, not recomputable Entity Runtime operations.
The updated test requires the exact replay refusal and still checks stored
state and audit completeness. The provider conformance and reopening tests
retain the persistence guarantees. No event bytes or runtime decisions change.

## Evidence

At discovery the default branch was
`35b5c9949d7b9e1caa3a5e5de7502af42c2dbc5e`. The provider default branches
matched the revisions above. ESS revision
`19de6406f97dca339136d7c9075ecc9b8fdb7af7` is protocol compatibility evidence;
AEP consumes the closed report formats without a compiled ESS dependency.

This is an automated upgrade. Initial creation used the CLI's configured default
actor; this record does not assert human review or approval of the candidate.
