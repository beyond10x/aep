---
format: aep.planning-md/1
id: story:evidence-subject-binding
kind: story
status: draft
title: Evidence names its subject, and a guard checks it is the one being moved
summary: A fact whose subject is not the transition's subject is refused, so weeks of green about a component nobody ships cannot happen silently.
owner: protocol
tags:
- adoption
- evidence
relations:
- decomposes: epic:adopter-feedback-round-1
revision: 2
---
# Story: Evidence names its subject, and a guard checks it is the one being moved

## Outcome

A fact can no longer be green about something nobody is shipping. Evidence carries the subject it
observed, and a transition refuses a fact whose subject is not the subject being moved.

## Context

An early adopter's review, round 1 — **item C2** — fourth in the adopter's
ranked order, and the one with the most literal incident behind it: an e2e job port-forwarded a
legacy service while the deployment rolled its successor. **Weeks of green about a component
nobody was shipping.** Nothing in the run was broken; every assertion was true of the thing it
actually talked to, and the thing it actually talked to was not the thing under test.

The engine has the analogous rule one layer over: the approvals rule that *version 3 does not satisfy
version 7* already refuses a record bound to the wrong revision. C2 is the same refusal over a
different axis — not *which revision* but *which subject* — and C3 (the environment revision a test
observed) is the third axis, deliberately left to a later round.

Subject binding and horizons (`story:evidence-horizons`) are the two halves of the same sentence: a
fact is a claim about *what*, observed *when*. Neither implies the other and both are missing.

## Acceptance

- An evidence record names its subject, and the name survives a round trip through the store, the CLI
  and both renderings.
- A transition offered a fact whose subject differs from the transition's subject is refused, with a
  reason printing both names.
- A record with no subject is not silently admitted — it is refused, or it is admitted under a stated
  rule that says why the omission is safe.
- The reported case reproduces as a fixture: a fact observed of a legacy service does not move its
  successor, asserted rather than described.

## Out of Scope

Deciding what a subject *is* for every domain. The protocol takes a name and compares it; inferring
subject identity from a URL, a namespace or a port-forward is the adopter's problem and would be this
protocol guessing.

## Open Questions

Whether subject comparison is exact-string or admits a declared alias table. Decides: protocol owner.
Default if nobody answers: **exact string**, because the incident is precisely a near-miss between two
names that a fuzzy comparison would have called equal.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** or **inferred**.

**Read this first: most of this story appears to have shipped on 2026-08-26 and nobody moved the
artifact.** `9fa3a0c` + `51f55f4` land `Task::subject`, the engine guard, the envelope and the CLI
half; `docs/plan/gap-register.md:72` says the C2 axis was **Done** on that date and
`CHANGELOG.md:1487-1510` ships it. Three of this story's four acceptance lines are satisfied by
that code. It is still `draft`, with one revision and no evidence, and no store record explains the
gap. **Verify before implementing anything.**

- **Primary surface (the live remainder):** `crates/aep-domain/src/requirement.rs` — the evidence-requirement matcher — cited, `docs/plan/gap-register.md:79` names it as the one increment still owed
- **Files:** `crates/aep-domain/src/requirement.rs:374-382` (`matches`, the subject branch), `:266` (the `subject` field), `:315-363` (`from_node`, where a parse-time refusal would go) — cited
- **Files:** `crates/aep-engine/src/execution.rs:605-620` (`satisfies_evidence`) — cited, the only call site of `matches` outside `aep-domain`; a semantic change lands here in the same commit
- **Symbols:** `EvidenceRequirement::matches`, `::subject`, `::from_node` — cited
- **Symbols (already shipped — do not re-open):** `Task::subject` (`crates/aep-domain/src/task.rs:348`), `ProtocolError::EvidenceSubjectMismatch`/`EvidenceSubjectMissing` (`crates/aep-engine/src/error.rs:60,80`), the guard at `crates/aep-engine/src/engine.rs:376-391`, `EvidenceEnvelope::subject`/`with_subject` (`crates/aep-domain/src/evidence.rs:1939,1978`), `EvidenceInput::about` (`crates/aep-schema/src/parse.rs:337`) — cited
- **Documents:** `website/docs/status/limitations.md:127-136` and `:200`, `website/docs/concepts/evidence.md:120-123`, `website/docs/concepts/design-principles.md:150` — cited, **each still states the limitation as live**. `docs/plan/gap-register.md:79` and `docs/design/evidence-horizons-design-v0.1.md:188-195`, `:692` record finding **F26**.
- **Confidence:** **high** for the remaining increment — the register names the file, the function and the finding id, and the branch is still at `requirement.rs:378`. **Medium** if the story is read from its literal acceptance rather than from `:79`, because three of four lines are already satisfied across four crates and nothing in the store records that.
- **Would collide with:** any unit touching `aep-domain`'s requirement matcher or `EvidenceRequirement` parsing, or `aep-engine`'s evidence evaluation; any unit editing the horizons/evidence pages under `website/docs`; and `docs/plan/gap-register.md` and `CHANGELOG.md`, which every unit in any wave touches.

**Not established.** Whether the story is actually open at all — see the paragraph above. Whether
the fix to `matches` is a behaviour change or a parse-time refusal; F26 declines to require
`subject` alongside `horizon` and the register's mitigation is prose only. Whether the website pages
are stale or deliberately unrevised — `limitations.md` was touched 2026-08-30, four days *after* the
guard landed, and still says the limitation is live. **Both recorded citations for the defect site
are stale line numbers** — `gap-register.md:79` says `requirement.rs:311`, the design says `:243`,
and the branch is at `:378`; anyone sizing this off a citation alone opens the wrong function.
