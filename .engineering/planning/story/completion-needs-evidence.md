---
format: aep.planning-md/1
id: story:completion-needs-evidence
kind: story
status: draft
title: A story cannot reach implemented on somebody's word
summary: The move to implemented is refused unless the engine has admitted the evidence the protocol requires, and the refusal names what is missing.
owner: protocol
tags:
- evidence
- store
relations:
- decomposes: epic:evidence-gated-completion
scope:
- confidence: inferred
  path: crates/govern/aep-engine
- confidence: cited
  path: docs/design/story-completion-evidence-design-v0.1.md
- confidence: cited
  path: docs/plan/harness-wave-4-governed-dogfood.md
- confidence: inferred
  path: principles/development
- confidence: cited
  path: profiles/development-standard.yaml
revision: 8
---
# Story: A story cannot reach `implemented` on somebody's word

## Outcome

A reviewer looking at a closed story knows that something the engine admitted stands behind it — and
somebody trying to close one without that is refused, and told exactly what is missing.

## Context

The store checks that a move is **legal** and says nothing about whether it is **earned**. Everywhere
else this repository refuses that gap: the engine never manufactures evidence, an agent's own
statement never satisfies an independence requirement, and the verifier class that can mint a
transcript verdict is a type rather than a convention. Then a person writes `status: implemented` into
a file and none of it applied. `adp.story.complete/v1` already exists and means *record that a story
is done, and what did it*; it has no consumers.

## Acceptance

- A lifecycle document may declare what reaching a status costs, and the engine decides the move
  against evidence the caller presents. **Shipped** — `entity-core` decides it, three-valued, and
  `story:guard-tests` was refused `implemented` until evidence was presented.
- The store distinguishes **asserted** from **recorded** provenance, and says which one a status
  rests on. **Shipped** — `protocol artifact validate` reports every status reached on an assertion,
  by name.
- Reported, not refused. Refusing an assertion outright would stop anybody closing a story on the
  day a runner is down, which is the day it matters most; what it must not be is invisible. The
  count does not affect the exit code, and that is deliberate.

## Verdict — accepted in part, 2026-08-28

The design this story asked a verdict on,
[`story-completion-evidence-design-v0.1.md`](../../../docs/design/story-completion-evidence-design-v0.1.md),
is **accepted in part**; the full text is its § 10, and it is recorded on
[`harness-wave-4-governed-dogfood.md`](../../../docs/plan/harness-wave-4-governed-dogfood.md) § W4.3
as that page's acceptance requires.

- **Shipped and accepted:** the two lines above, plus the `delivers` row in
  `artifacts/relations/relations.yaml`.
- **Accepted, not yet in force:** the principle over facts and the evidence gate on the existing
  terminal move (option B). Measured on this store on the day of the verdict — 125 artifacts, 0
  problems, 38 stories at `implemented`, 4 of them reached on an assertion, and **0 artifacts
  carrying a `trace_conformance` record** — option B would put 38 of 38 in deviation today. So the
  principle file is not written, and it is listed in `development.standard` only when
  `story:governed-dogfood-run` has closed one story by a driven run.
- **Refused:** the engine judging whether a producer was *independent* of what it reports on, as
  part of this rule. It is engine work with its own surface, and it is carried by
  `story:evidence-producers-for-the-driven-map`.

*Still open* below is unchanged by the verdict: admitting a recorded result automatically is what
the driven run adds, not what this decision settles.

## Still open

Admitting a **recorded** result automatically — a gate run that lands in the store as an
observation rather than as a caller's claim about one. Every status in this store that rests on a
record got there because somebody typed the record in, which is a shorter chain than *the agent said
so* and a longer one than *the store watched it happen*.
## Out of Scope

Approval evidence. A person saying *I approve* is already modelled as `Evidence::Approval` with a
`Producer::Human`; this story is the mechanical half.

## Open Questions

Whether the gate is a refusal or a warning in its first release. Decides: protocol owner. Default if
nobody answers: a refusal, because a warning that can be ignored is the state the store is in today.
