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
revision: 1
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
