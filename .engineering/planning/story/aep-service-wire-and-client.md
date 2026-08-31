---
format: aep.planning-md/1
id: story:aep-service-wire-and-client
kind: story
status: draft
title: Versioned AEP service wire and official client
summary: Project the semantic command/query contract across a strict authenticated network boundary.
relations:
- serves: vision:O2
- serves: vision:O6
revision: 1
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

## Acceptance

An independently released EP client and `aep-service` verify the same versioned constructed corpus for accepted, replayed, refused, conflicting, malformed, unavailable and unauthorized command/query exchanges, while actor, executor, request identity and recorded time can only originate in verified server context.
