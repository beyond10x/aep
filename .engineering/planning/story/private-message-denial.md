---
format: aep.planning-md/1
id: story:private-message-denial
kind: story
status: draft
title: A capability a profile can deny before an agent reads a direct message
summary: network.read cannot separate a public channel from a DM, so the one control that stops a private conversation reaching a shared corpus is prose in a skill file.
owner: protocol
tags:
- adoption
- protocol
relations:
- decomposes: epic:ingestion-vocabulary
revision: 3
---
# Story: A capability a profile can deny before an agent reads a direct message

## Outcome

A profile can say *this agent may read public channels and may never read a direct message*, and a
profile that forgets to say it fails validation instead of passing it. Nobody's private conversation
reaches a shared corpus because the rule lived in a paragraph an agent was asked to follow.

## Context

Adopter register row **`D-I2`**, 2026-08-21. Their words, and the row they call the load-bearing one:

> The load-bearing rule of this pipeline is: **public channels readable, DMs and group DMs
> categorically not**, regardless of the token's membership. `aep/1` has `network.read` and nothing
> finer. So the control that prevents the worst outcome — an agent reading a private conversation and
> restating it into a shared corpus — is enforceable only by prose in a skill file, and a profile that
> forgot it would validate clean.

Their own closing condition:

> either a `private_message.read` capability that a profile can `deny`, or a general mechanism for
> scoping a capability by resource class. The second is the better shape and the larger change; the
> first is one line and covers the observed hazard.

**Measured 2026-08-28 UTC, `target/debug/protocol` 0.32.1**, on a scratch extension of `aep/1`:

```console
$ protocol validate --root <scratch>
1 problem(s):
  - protocol document (…/aip/1.yaml): capabilities: unknown capability "private_message.read":
    "private_message.read" is not a capability; known capabilities are repository.read,
    repository.write, tests.execute, command.execute, network.read, network.write, telemetry.read,
    production.read, production.write, secret.read, artifact.read, artifact.write, planning.read,
    planning.write, review.request, approval.request, deployment.create[:env] and
    deployment.rollback[:env] at line 5 column 15
```

This is the closed row of [`docs/guide/open-vocabulary.md:167`](../../../docs/guide/open-vocabulary.md)
— *Capability value names the engine accepts*, closed at `crates/aep-domain/src/capability.rs:144`,
and the guarantee it buys is *a capability name resolves to the same authorisation decision in every
harness*. Nothing here asks to open that. A new name arrives as this story, with what it means in
every harness written down, which is the route that page names.

**The scoping shape already exists in the type, and that is the finding worth having.** `Capability`
is not a flat set of names: `Deploy(Environment)` and `Rollback(Environment)`
(`crates/aep-domain/src/capability.rs:164,166`) are two capabilities scoped by resource class today.
`Capability::covers` (`capability.rs:240-246`) resolves an overlap between a scoped grant and a
scoped request, `Environment::covers` (`capability.rs:83-85`) makes `*` cover every member, and
`Protocol::needs_approval_floor` already reasons about overlap **in either direction** — a floor on
`deployment.create:production` is violated by granting `deployment.create` for every environment.
`protocols/aep/1.yaml:39` carries `deployment.create:production` as a floor entry, so a scoped
capability in a document is not hypothetical. The adopter's *"better shape and larger change"* is
therefore a generalisation of a mechanism this repository has shipped and tested, not a new one.

**Why the narrow spelling is not obviously the right answer.** `private_message.read` names a Slack
concept in a vocabulary that names no vendor. `network.read:public` / `network.read:private` — or a
resource class that reads the same for a mailbox, a ticket's internal comment and a DM — says the
same thing without the vendor in it, and a corpus pipeline is not the only place the distinction
bites: an incident profile reading a support inbox has it too.

## Acceptance

- A profile can deny reading private correspondence, and the denial survives the merge chain: the
  resolved plan for a profile that denies it shows a denial, and one that forgot shows the floor's
  refusal rather than silence. Refused with the engine's own words, and a test that reaches the state
  where the rule is load-bearing — a profile granting the broad read *and* the narrow denial, not a
  profile granting nothing.
- What the name means in **every** harness is written down beside the variant: which reads it covers,
  what a harness that cannot tell a DM from a channel is expected to do (deny, not guess), and the
  fact that membership is irrelevant — a token that *can* read it is exactly the case the rule is for.
- `docs/guide/open-vocabulary.md`'s capability row still says `closed`, with the same guarantee, and
  the vocabulary reference lists the new name. A widening that leaves that page stale is the defect
  the audit exists to catch.
- `website/docs/reference/vocabulary.md` and the `known capabilities are …` diagnostic both list it —
  the diagnostic is the surface an adopter actually meets, and it is generated from `Capability::SIMPLE`
  (`capability.rs:185`), so an entry missing from either is a name nobody can find.
- A denial is not downgradable to an approval. The existing guard
  `a_denied_capability_is_not_downgraded_to_requiring_an_approval` (`crates/aep-domain/src/capability.rs`)
  is the shape; whatever this lands, it holds for it too.

## Out of Scope

Enforcing it. This repository holds no credential and reaches no network; a capability is a declared
authorisation decision and the actor that honours it is a harness. Shipping the name without a harness
that reads it is still worth it — a profile that *validates clean while forgetting the rule* is the
defect the adopter reported, and the name fixes that half on its own.

Any Slack-, Jira- or vendor-specific vocabulary beyond the one class name. If the answer is
`private_message`, that is a resource class, not an integration.

## Open Questions

**Narrow name or general scoping?** The adopter ranks them and the ranking is inverted against
effort: *"The second is the better shape and the larger change; the first is one line and covers the
observed hazard."* Decides: protocol owner. Default if nobody answers: **the general scoping
mechanism**, modelled on `Deploy(Environment)`, because a one-line name for one vendor's concept is
the widening that a closed vocabulary's guarantee is least able to absorb — and because the narrow
name can be spelled as a scope of `network.read` afterwards, while the reverse cannot.

**What is the resource class over?** `network.read:private-message` scopes the *conversation kind*;
`network.read:{public,private}` scopes the *audience*, and would also catch a private channel the
token is a member of. The row does not say which side of the line a private channel falls on — it
names only *"DMs and group DMs"* against *"public channels readable"*. Unclear — ask the adopter.
