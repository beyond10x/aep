---
format: aep.planning-md/1
id: story:communication-publish-capability
kind: story
status: draft
title: The irreversible act in a corpus pipeline has no capability under it
summary: Publishing outward — a Slack post, a thread reply, a ticket comment — is non-idempotent and cannot be withdrawn, and no profile can gate it because the capability does not exist.
owner: protocol
tags:
- adoption
- protocol
relations:
- decomposes: epic:ingestion-vocabulary
- informed_by: story:outbound-claims-and-status-vocabulary
revision: 4
---
# Story: The irreversible act in a corpus pipeline has no capability under it

## Outcome

An agent cannot post to a customer channel, reply in a thread or comment on a ticket without a
recorded approval — and a profile that forgot to require one is refused by the floor rather than
trusted. The same protection `production.write` has had since the first version, for the act that in
a corpus pipeline is the one nobody can undo.

## Context

Adopter register row **`D-I4`**, 2026-08-21, and by their own ranking the cheapest serious one:
*"This is the row with the highest ratio of risk to effort."* Their words:

> `approval_floor` forbids granting `production.write` and `deployment.create:production` outright —
> correct, and the reasoning generalises. In ingestion the irreversible act is **publishing**: a Slack
> post, a thread reply, a Jira comment. It is visible to customers, non-idempotent on retry, and cannot
> be withdrawn. There is no `communication.publish` capability, so no profile can gate it and no floor
> can require the gate.

Their closing condition: *"`communication.publish` in `capabilities` and in `approval_floor`."*

**Measured 2026-08-28 UTC, `target/debug/protocol` 0.32.1**, on a scratch extension of `aep/1` declaring
the capability and the floor entry:

```console
$ protocol validate --root <scratch>
1 problem(s):
  - protocol document (…/aip/1.yaml): capabilities: unknown capability "communication.publish":
    "communication.publish" is not a capability; known capabilities are repository.read,
    repository.write, tests.execute, command.execute, network.read, network.write, telemetry.read,
    production.read, production.write, secret.read, artifact.read, artifact.write, planning.read,
    planning.write, review.request, approval.request, deployment.create[:env] and
    deployment.rollback[:env] at line 5 column 15
```

**Everything except the name is already built, which is why this row is cheap.** A profile declares
`capabilities.require_approval:` (`CapabilityPolicy`, `crates/aep-domain/src/capability.rs:482-498`);
a workflow **state** carries the same policy — `RawState.capabilities`, *"Capability adjustments while
here"* (`crates/aep-domain/src/workflow.rs:296-298`) — so the gate can sit on the publishing state
itself; and `approval_floor` is inherited by every extension on purpose, *"a derived protocol that
forgot to restate it would silently let a profile grant production access outright"*
(`crates/aep-domain/src/protocol.rs:145-149`). One name is the whole distance between the adopter's
document and a gate the engine enforces.

Worth recording because it will be met again: the adopter's workflow marks its publishing state
`irreversible: true` — which `RawState` does have (`workflow.rs:299-301`), and which already forces a
forward recovery plan (`workflow.rs:495-506`) — and then writes `requires_approval: true` **on the
state**, which is not a key `RawState` has. `RawState` is `deny_unknown_fields`. Approval is a
capability decision, not a state flag, and the state's own `capabilities:` block is where it goes.
That is a documentation finding riding along with this one, not a second gap.

**How this sits against the row round 1 already closed.** `story:outbound-claims-and-status-vocabulary`
built the *lifecycle* of a claim that has left the building — `draft → cleared → sent →
correction-owed → corrected | retracted`, with `cleared` as the approval gate — and its *Out of Scope*
says, deliberately: *"Nothing here sends anything… the protocol models the claim's lifecycle and the
gate before it, and the act itself stays where `production.write` already puts it."* The act is what
this story is. And `production.write` is not in fact where it can stay: a profile that grants
`production.write` to authorise a Slack post has granted the capability that also writes to a
production database, which is the opposite of least privilege and would trip the floor for a reason
that has nothing to do with the thing being done.

**What it costs an adopter to leave open, concretely.** Round 1's own incident is this one:
*"resolved"* was told to a customer roughly **seven hours before** the contradicting verification
landed. That ordering is now sayable as a lifecycle and still ungateable as an act — the claim can
reach `sent` with no approval anywhere in the profile, because nothing in the vocabulary names
sending.

## Acceptance

- `communication.publish` is a `Capability`, and it is in `aep/1`'s `approval_floor`. A profile that
  grants it outright is refused, naming the floor; a profile that puts it behind approval resolves.
  The test reaches the state where the rule is load-bearing — a profile granting it outright, not a
  profile that never mentions it.
- The floor's overlap logic is exercised for it, the way `deployment.create` is: whatever scoping the
  name ends up with, granting a broader form does not slip past a floor on a narrower one
  (`Protocol::needs_approval_floor`).
- A workflow state can carry the gate in its own `capabilities.require_approval`, and an
  `irreversible: true` state that publishes without one is visibly a hole in a resolved plan rather
  than an invisible one.
- What the name means in **every** harness is written beside the variant: an outward, non-idempotent
  assertion to a human audience — a chat post, a thread reply, a ticket comment, an email — and
  explicitly not an internal write to a store the same agent owns. A harness that cannot tell them
  apart denies.
- `website/docs/reference/vocabulary.md`, the `known capabilities are …` diagnostic and
  `docs/guide/open-vocabulary.md`'s closed capability row all agree after the change.
- `CHANGELOG.md` says what now refuses that used to pass: a profile granting the new capability
  outright.

## Out of Scope

Sending anything, and any transport, mailbox or helpdesk integration — the same boundary
`story:outbound-claims-and-status-vocabulary` drew, and for the same reason.

A `corpus.write` capability. The adopter's profile marks it unrepresentable in the same block —
*"`repository.write` is granted above and does the work today, but it cannot distinguish 'edit a
script' from 'assert a fact about a customer'"* — and it is a weaker case: both sides of that
distinction are reversible, which is the property that makes the publish row urgent. If it is wanted,
it is a row this epic did not take.

Retraction and correction mechanics. Round 1 shipped the rungs; nothing here re-decides them.

## Open Questions

**Is it one capability or a scoped one?** `communication.publish` covers a Slack post and a Jira
comment alike; `communication.publish:external` would let an internal-channel post be granted while a
customer-facing one stays behind approval. Decides: protocol owner. Default if nobody answers: **one
unscoped capability, on the floor**. The floor is the point of the row, an unscoped name cannot be
granted outright at all, and a scope can be added later exactly as `deployment.create` carries one —
whereas a scope shipped now invites `communication.publish:internal` to be granted outright on day
one, which is the failure the row is about.

**Does the floor entry belong in `aep/1` or in a corpus-shaped protocol?** Publishing outward is not
specific to ingestion — an incident profile posts status updates to customers. Decides: protocol
owner. Default: **`aep/1`**, because a floor that only some protocols inherit is a floor somebody can
step around by choosing a different base.
