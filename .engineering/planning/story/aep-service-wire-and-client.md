---
format: aep.planning-md/1
id: story:aep-service-wire-and-client
kind: story
status: implemented
title: Versioned AEP service wire and official client
summary: Project the semantic command/query contract across a strict authenticated network boundary.
relations:
- serves: vision:O2
- serves: vision:O6
revision: 9
---
## Context

`aep-contract` already defines storage-independent `CommandService` and `QueryService` semantics,
but it deliberately defines no network transport. The private `aep-service` needs an independently
versioned HTTP projection, while `protocol` needs an official client without learning the service's
storage or deployment details.

This story is the review surface for
`docs/design/aep-service-wire-v0.1.md`. The design remains proposed until its review questions are
resolved; implementation must not start merely because the document exists.

## Scope

- raw network documents distinct from trusted in-process command context;
- explicit version negotiation and compatibility rules;
- an official client boundary owned by this repository;
- constructed, credential-free request/response vectors consumed by client and server; and
- stable mapping between transport failures and existing semantic command/query errors.

The service implementation, database schema, concrete identity-token profile and deployment remain
outside this repository.

## Review Record — 2026-08-31

The operator decided four questions:

1. nullable request members are mandatory and explicitly `null` when absent;
2. idempotency is scoped by realm, workspace and authority, not executor;
3. after workspace authorization, entity absence and entity-level denial both return `not_found`;
4. failed media-type negotiation advertises served versions in `AEP-Supported-Versions`; there is
   no separate discovery endpoint.

5. the official client implements `CommandService` and `QueryService` directly; its injected transport
   maps a no-response failure locally to `Unavailable` without forging server response bytes, and
   there is no parallel remote semantic facade.

All five review questions are resolved. A tenant is the control-plane owner of one or more globally
unique realms; realm remains the AEP storage and authority boundary, and tenant identity does not
enter version-1 routes or entity coordinates. Implementation begins only after this story is
explicitly moved to `active`.

## Implementation Record — 2026-08-31

`aep-client` now carries strict version-1 request and response documents, direct semantic-trait
implementations, injected transport and credential ports, typed semantic problem mappings and an
embedded constructed corpus. An implementation audit found that the reviewed route table omitted
`QueryService::resolve`; version 1 now projects it as `POST /entities/resolve`.

The remaining acceptance work is release and independent consumption: publish the EP crate, pin it
in `aep-service`, prove the service against the same embedded cases, and then select the client from
`protocol` configuration.

## Acceptance

An independently released EP client and `aep-service` verify the same versioned constructed corpus for accepted, replayed, refused, conflicting, malformed, unavailable and unauthorized command/query exchanges, while actor, executor, request identity and recorded time can only originate in verified server context.

## Independent service consumption — 2026-08-31

`aep-service` commit `8793c50` pins released EP `0.35.0` at commit `1406c06` and realizes all
eight version-1 routes through the EP-owned runtime-neutral request/response boundary. Its
conformance test iterates `aep_client::conformance::CASES` directly and holds status, headers and
body bytes to the published corpus while independently proving dispatch and trusted attribution.

The service gate passed after its versioned API, trusted context and shared-conformance stories
moved to `implemented`. Concrete identity-token verification, transactional Entity Runtime storage
and `protocol` remote configuration remain later service/client stories; they are not claimed by
this boundary release.
