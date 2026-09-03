---
format: aep.planning-md/1
id: epic:external-sources-of-record
kind: epic
status: draft
title: An artifact whose record of truth lives outside this repository
summary: Jira, Linear and their kin hold artifacts this protocol reasons about. ArtifactLocation::External names them and nothing resolves one. Decide what an external source IS to AEP before building a connector.
owner: protocol
tags:
- adoption
- boundaries
- store
relations:
- decomposes: initiative:the-repo-governs-itself
revision: 4
---
# Epic: An artifact whose record of truth lives outside this repository

## Outcome

An adopter whose stories live in Jira can be governed by this protocol without moving them, and
everyone can say precisely what that means: which facts AEP owns, which the external system owns,
what happens when they disagree, and what a requirement over an external artifact is actually
asserting.

## Why Now

The **location** already exists and nothing resolves it. `ArtifactLocation::External { provider,
reference }` (`crates/govern/aep-domain/src/artifact.rs:913`) is documented as *"an object in an external
system, resolved by a connector rather than by AEP"* — and there is no connector, in this repository
or any other. The vocabulary invites a thing the toolchain cannot do, which is the shape of defect
`docs/plan/gap-register.md:76` was opened about.

It is not hypothetical, and the providers are already named in shipped examples.
`examples/billing-conformance/artifacts.yaml:11-13` and
`examples/development-passkeys/artifacts.yaml:11-13` both carry a `prd:*` at `status: active`, and
the passkeys example resolves artifacts to **`linear`** (`:15`, `:24`) and **`github`** (`:66`) by
provider name. So this repository ships documents that say *this artifact is active in Linear* and
has no way to find out whether that is true. Those statuses are believed today because nothing
checks them, and one of them — a `product-requirements` — is a kind for which no ladder is declared
at all, so the permissive fallback currently shrugs at whatever it says.

And the ground moved on 2026-08-25. The artifact status vocabulary is now **open and ladder-gated**:
a status is accepted for a write only if the kind's lifecycle declares it. That makes declaring a
ladder for an externally-tracked kind a bigger act than it was — it decides what an adopter's Jira
statuses are allowed to say. The right time to answer this is before somebody declares one by
reflex.

## Scope

The first deliverable is a **decision, not a connector**. Three readings are available and they are
not variations of each other:

1. **A backend.** The external system implements the `aep-contract` command/query surface and runs
   the sixteen conformance suites, exactly as `aep-backend-markdown` and `aep-backend-memory` do.
   Strongest, and it demands things Jira will not give — revision-guarded writes, an append-only
   journal, a history that is not editable by a person with the right role.
2. **A connector, read-only.** AEP resolves an `External` reference to enough of the object to
   answer a requirement, and never writes. Weakest and most honest; the external system stays the
   record of truth and AEP stays a reader.
3. **A mirror.** The external object is projected into the local store on a schedule, and the
   projection is what AEP reasons about. Familiar, and it introduces a staleness window that every
   verdict then silently depends on.

Whichever is taken, four questions have to be answered by name:

* **Status mapping.** Their vocabulary is theirs. With ours now open, mapping is a lifecycle
  document rather than a Rust change — but the direction of the mapping, and what an unmappable
  status means, is a decision.
* **Identity and freshness.** `ArtifactRef` carries `@version` and `ArtifactGraph::resolve` discards
  it (`artifact.rs:1624`). An external object that changes under a pinned reference is the version
  problem this repository already refuses to hand-wave elsewhere.
* **What a requirement over an external artifact asserts**, and how a reader tells that verdict from
  one over a local artifact. Invariant 5 applies with force: *unreachable* is `Unknown`, never
  `False`, and a connector that cannot reach Jira must not report a gate as failed.
* **Credentials and network.** Nothing in `task check` reaches the network (`AGENTS.md` §
  *Dependencies*). A connector cannot be reached from a gate step, and where it *is* reached from
  needs saying before anything is built.

## Out of Scope

* Writing to an external system. Every reading above is a read until the decision says otherwise,
  and a protocol that mutates somebody's Jira on a status move needs its own argument.
* Choosing a vendor. Jira is the example because adopters named it; the decision is about what an
  external source *is* to AEP, and a vendor connector is a story under whatever that answers.
* Migration tooling — importing a Jira project into the markdown store is a different thing from
  governing one in place, and conflating them is how this becomes a sync engine.

## Risks

* **This quietly becomes a sync engine.** Two systems that both believe they own a status is the
  oldest failure in this category, and reading 3 without deciding 1-versus-2 first is how it starts.
* **A verdict nobody can reproduce.** If a gate's answer depends on what a remote system said at a
  moment, the evidence model has to carry that moment or the audit trail is fiction. `story:evidence-horizons`
  is the existing shape for this and should be read before anything is designed.
* **`aep-contract`'s suites define what a backend means here.** Calling something a backend that
  cannot pass them would make the sixteen suites decorative, which costs more than not having the
  connector.

## Done When

A recorded decision says which of the three readings AEP takes and why, with the four questions
answered by name; `ArtifactLocation::External` either has a mechanism behind it or a written
statement of what it does not promise; and the `prd:*` artifacts in the two shipped examples are
either checkable or explicitly labelled as claims nothing verifies.
