# AEP service wire and client design v0.1

Status: **proposed**. This document is not normative and is not an implementation work order.
`story:aep-service-wire-and-client` is its review surface.

## 1. Decision sought

Publish one versioned network projection of the existing `CommandService` and `QueryService`, plus
the official Rust client used by `protocol`. The private `aep-service` repository implements the
server side. This repository continues to own command/query meaning, wire types, compatibility
rules and constructed conformance vectors.

This is not a remote `entity-store::Store`. Clients submit AEP intentions and ask AEP questions;
they cannot write Entity Runtime decisions, events, audit records or storage batches.

## 2. Existing semantic contract stays primary

The types in `aep-contract` remain the in-process semantic boundary. Network types are separate raw
documents because trust changes at the network edge:

- `CommandContext.actor`, `executor`, `request_id` and `issued_at` are trusted in-process values;
- a remote client must not populate them;
- transport authentication and version negotiation are not domain commands; and
- transport availability is not a semantic refusal.

The server adapter validates a raw wire document, derives trusted context, constructs the existing
semantic envelope and calls exactly one existing service method. The transport adds no second
mutation path.

## 3. Media type and version negotiation

Version 1 uses UTF-8 JSON with this media type:

```text
application/vnd.aep.service+json;version=1
```

Every request carrying a document sends that value as `Content-Type` and sends an `Accept` value
for a supported response version. The response uses the selected media type. A server that cannot
serve any requested version returns `406` with a version-1 problem document when version 1 itself
is acceptable, or an empty response otherwise.

Within a version, objects are closed: an unknown field is malformed rather than silently ignored.
Adding, removing or changing a field therefore creates a new wire version. During migration a
server may serve more than one version, and the client selects the newest mutually supported one.

Canonical fixture bodies are compact JSON: no insignificant whitespace, object members in declared
type order, map members in Unicode code-point order and one trailing line feed. Conformance checks
compare those bytes. HTTP header ordering is outside the corpus.

## 4. Addressing

Every operation is scoped by path before a body is read:

```text
/aep/v1/realms/{realm}/workspaces/{workspace}
```

`realm` is an administrative and storage isolation boundary. `workspace` is the planning and
implementation scope within it. Both are opaque URL segments; clients do not infer company or
repository identity from their spelling.

Version 1 exposes:

```text
POST /commands
GET  /entities/{entity_id}
POST /entities/query
POST /relations/query
GET  /entities/{entity_id}/history
POST /audit/query
GET  /types/{entity_type}
```

Complex queries use `POST` because their bodies are read-only structured questions, not because
they mutate. No route accepts a generic storage operation.

## 5. Authentication and trusted request context

The transport carries a bearer credential. Its exact token encoding and issuer profile are a
separate authentication design; the wire contract only requires the verifier to produce:

```rust
pub struct VerifiedPrincipal {
    pub authority: ActorRef,
    pub executor: ActorRef,
    pub realm: String,
    pub workspace_grants: Vec<String>,
    pub roles: Vec<String>,
    pub delegation_id: Option<String>,
}
```

For a human request, authority and executor are the same human. For a delegated request, authority
is the owner who signed the delegation and executor is the authenticated agent. The server checks
that current owner grants, delegation scope and executor restrictions all permit the attempt.
Delegation can only narrow authority.

The verifier and server create these `CommandContext` fields; they are absent from command JSON:

- `actor` from `VerifiedPrincipal.authority`;
- `executor`, omitted only when equal to `actor`;
- `request_id` from the receiving service or trusted ingress;
- `issued_at` from the server clock.

The caller still supplies logical, non-authorizing correlation data: command id, idempotency key,
correlation id, causation, optional execution id and optional task id. Those values are validated
and audited but grant no permission.

Authentication and workspace authorization happen before entity lookup or response
materialization. A caller without workspace read access receives `unauthorized` without learning
whether a referenced entity exists.

## 6. Command request

The version-1 command document is the raw transport counterpart of `CommandEnvelope`:

```json
{
  "command_id": "cmd-01K2R8JD3ZJME72AJGQY67E5F8",
  "idempotency_key": "retry-01K2R8JD3ZJME72AJGQY67E5F8",
  "command_type": "aep.entity.create/v1",
  "target": null,
  "expected_revision": null,
  "correlation_id": "corr-01K2R8JD3ZJME72AJGQY67E5F8",
  "causation": null,
  "execution_id": null,
  "task": null,
  "payload": {}
}
```

Nullable fields are written as `null` in version 1. This keeps fixture shape explicit and prevents
absence from acquiring a second meaning. `payload` is the raw JSON representation of the existing
versioned AEP command named by `command_type`; the pair is validated together before dispatch.

Idempotency is scoped by realm, workspace and authority. A retry by another authorized executor on
behalf of the same authority may retrieve the original result. Reusing a key for different intent
is `idempotency_mismatch`. The original command result remains the replay result; the replay attempt
is still attributable in audit.

## 7. Command response

An accepted command returns `200` for both first application and replay. `CommandOutcome` tells the
client which occurred. The body is:

```json
{
  "request_id": "req-01K2R8JD3ZJME72AJGQY67E5F8",
  "result": {
    "command_id": "cmd-01K2R8JD3ZJME72AJGQY67E5F8",
    "outcome": "accepted",
    "affected": [],
    "events": [],
    "audit": [],
    "consistency": "seq:1"
  }
}
```

The service does not use `201`: the same logical request may be a replay, and HTTP status must not
force the client to guess whether a resource was newly created.

## 8. Queries

Wire query documents project the existing `EntityQuery`, `RelationQuery`, `AuditQuery` and
`QueryConsistency` types. Their objects are closed and optional fields are explicitly nullable in
version 1. Page responses always write both `items` and nullable `next`.

`GET` routes carry no query document. `GET /entities/{entity_id}` accepts an optional consistency
token in `AEP-Consistency`; history and type description use the same response envelope and error
shape as body-based queries.

Authorization narrows the candidate set before traversal, pagination or serialization. A page
cursor is scoped to the verified principal, realm, workspace, query digest and selected wire
version; using it in another scope is `invalid`.

## 9. Problem document

Every answered failure uses one closed shape:

```json
{
  "request_id": "req-01K2R8JD3ZJME72AJGQY67E5F8",
  "error": {
    "code": "revision_conflict",
    "message": "the entity changed since the expected revision",
    "retryable": false,
    "details": {}
  }
}
```

Stable codes come from the EP command/query taxonomy plus transport codes. Messages are diagnostic,
not machine contracts. `details` is a closed object selected by code and contains only safe values.

The status mapping is:

| condition | status | code |
|---|---:|---|
| malformed JSON, field or cursor | 400 | `invalid` |
| missing or invalid credential | 401 | `unauthenticated` |
| insufficient effective grant | 403 | `unauthorized` |
| authorized lookup found nothing | 404 | `not_found` |
| revision or idempotency conflict | 409 | semantic conflict code |
| unsupported command type | 422 | `unsupported` |
| no mutually supported wire version | 406 | `unsupported_version` |
| service cannot answer now | 503 | `unavailable` |

Only `unavailable` is retryable unchanged. A network failure with no HTTP response is represented by
the client as a transport error, never forged into a server problem document.

## 10. Client boundary

The official client lives in this repository beside the wire types. It exposes `CommandService`
and `QueryService` semantics to callers and hides HTTP routing, negotiation and serialization. Its
transport is injected so the specification crate takes no async runtime and conformance can run
without network access.

Authentication material enters through a credential-provider interface. The client neither parses
delegation to grant itself authority nor constructs actor/executor fields. It refreshes credentials
only through that provider and never writes tokens to diagnostics.

`protocol` selects this client from project configuration in a later adoption story. The initial
contract does not change the CLI's backend selection or make remote service access a gate
dependency.

## 11. Constructed conformance corpus

This repository owns a versioned, synthetic corpus with no credentials or adopter data. Each case
pins:

- method, path, selected headers and request body bytes;
- verifier outcome and trusted principal when authentication succeeds;
- expected semantic service call, or an assertion that no call occurs;
- response status, selected headers and body bytes; and
- whether retrying unchanged intent is permitted.

The first corpus covers accepted, replayed, refused, revision-conflicting, malformed, unsupported
and unavailable commands; authorized and unauthorized reads; human and delegated-agent attribution;
and version negotiation. Both the official client and `aep-service` consume the same pinned bytes.

The corpus is evidence about the wire projection. The existing in-process `aep-conformance` suites
remain evidence about semantic backend behavior; neither replaces the other.

## 12. Compatibility and release rule

Wire types and fixtures are released with EP. `aep-service` pins a released EP version rather than a
branch. Changing any fixture byte, media type, route, status mapping, trusted-field rule or token
audience is a coordinated migration under an Atlas ADR. The old version remains served until every
registered client has moved or an explicit retirement decision says otherwise.

## 13. Deliberately outside v0.1

- PostgreSQL schema and transaction implementation;
- a concrete OIDC/JWT/delegation token profile;
- bulk export and Markdown projection;
- streaming subscriptions;
- Jira ingestion;
- definition-bundle activation and instance migration; and
- company-brain realms.

Those concerns depend on this boundary but do not belong in its bytes.

## 14. Review questions

1. Should nullable request members be mandatory as proposed, or omitted when absent?
2. Is authority-scoped idempotency correct when an agent executor changes between retries?
3. Should entity-level denial deliberately collapse to `not_found` after workspace authorization?
4. Which discovery mechanism advertises concurrently served versions without adding another
   mutable endpoint contract?
5. Does the official client implement the semantic traits directly, or expose a parallel remote
   facade where generic associated types make trait implementation awkward?
