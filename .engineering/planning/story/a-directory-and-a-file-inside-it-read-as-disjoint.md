---
format: aep.planning-md/3
id: story:a-directory-and-a-file-inside-it-read-as-disjoint
kind: story
status: active
title: waves compares scope entries as strings, so a directory never collides with its own contents
relations:
- serves: vision:O3
- decomposes: epic:zero-open-issues-2026-11-09
scope:
- confidence: inferred
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning/waves.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/wave_derivation.rs
- confidence: cited
  path: crates/govern/aep-domain/src/artifact.rs
- confidence: inferred
  path: website/docs/concepts/waves.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:26Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:27Z", actor: "human:timo", revision: 7}
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

## Triage 2026-11-09

Still present at `824869fec`. `crates/edge/aep-cli/src/planning/waves.rs:206` places a story with `surfaces[wave].contains(path)`, an exact string match, and `:256` finds a collision with `entry.path == left.path`. The doc comment at `crates/govern/aep-domain/src/artifact.rs:1747-1750` and the `waves --help` text say nothing is normalised; both change with the fix.

Reproduced with aep 0.79.0 in a throwaway store: four stories scoped to `roles/`, `roles/decision.yaml`, `roles` and `roles.md` print `wave 1: story:dir story:file story:sibling story:slash` and `1 wave(s), 0 collision(s)`, exit 0.

The third item of *Shape* (a file and a directory of one name) is not in *Acceptance* and is not built here: the verb reads scope entries, not the tree (coordinator's reading).
