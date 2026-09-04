---
format: aep.planning-md/1
id: story:a-directory-and-a-file-inside-it-read-as-disjoint
kind: story
status: draft
title: waves compares scope entries as strings, so a directory never collides with its own contents
relations:
- serves: vision:O3
revision: 1
---
## What is wrong

`aep artifact waves` compares scope entries as strings, so a directory and a file inside it are two
unrelated surfaces. A story scoped to `roles/` and a story scoped to `roles/decision.yaml` compute as
**disjoint**, and a wave puts them in the same round.

Observed 2026-09-04 while scoping a real wave. `story:a-correction-no-signal-established` declares
both `roles/` and `roles/lookup.yaml`, because one route of the story rewrites every role document
and the story quotes one of them. Nothing in the store relates the two entries, and the scoper that
wrote them said so in its own report rather than leaving it for the merge:

> `roles/` and `roles/lookup.yaml` are declared as two surfaces and nothing normalises one into the
> other, so a unit declaring `roles/decision.yaml` will not compute as a collision even though it is
> one.

## Why it matters more than it looks

The verb exists so that disjointness is computed rather than asserted — the wave skill says the
verb wins over a coordinator's own reading of the bodies, and that deference is the whole point:
one is a record anybody can re-read, the other is one agent's inference that will not survive the
session. A verb that answers *disjoint* for two units that share a directory makes the deference
wrong in exactly the case a human reading the two bodies would have caught.

The cost lands late. Two agents each spend a full implement-and-attack cycle, and the collision
surfaces at merge — which is the outcome the scoping step exists to prevent.

## Shape

- A scope entry is compared as a **path prefix**, not as a string: `roles/` contains
  `roles/decision.yaml`, and two entries collide when either contains the other.
- A trailing separator is not what decides it. `roles` and `roles/` are one surface, and a story
  that writes one spelling must not read as disjoint from a story that wrote the other.
- Where a repository has a file and a directory of the same name, say which was meant rather than
  guessing — this is rare enough to refuse rather than resolve.

## Acceptance

- Two stories, one scoped to a directory and one to a file inside it, are reported as a collision
  and never placed in one wave.
- The same two, spelled with and without a trailing separator, give the same answer.
- A story scoped to a sibling file of that directory is still disjoint from it.
