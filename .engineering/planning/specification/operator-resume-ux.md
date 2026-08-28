---
format: aep.planning-md/1
id: specification:operator-resume-ux
kind: specification
status: draft
title: The refusal names the holder's state, and a stolen lock is in the record
summary: 'Required behaviour for the one defect and three unnamed assertions left in operator-resume-ux: --take-lock threads the superseded lock into the new run''s cursor, the refusal carries the holder''s cursor state, and the resume, pause and host claims get tests that read the tree back.'
owner: driver
tags:
- driver
- operator
relations:
- specifies: story:operator-resume-ux
- derived_from: task:w4-3-operator-resume-ux
revision: 3
---
<!-- Starting point for a `specification` artifact. Headings are the section names a protocol may
     require; delete the italic guidance as you fill each one. Templates are conveniences — replace
     this file with your organisation's own without leaving the protocol. -->

# Specification: <name>

*What must be true of the implementation, stated so that a test could contradict it.*

## Context

*What exists now, and which PRD, epic or story this specifies.*

## Requirements

*Numbered, individually checkable statements of required behaviour. One claim per line.*

## Constraints

*Limits the implementation must respect: compatibility, latency, data residency, cost.*

## Out of Scope

*Behaviour a reader might reasonably assume is included, and is not.*

## Invariants

*What must hold at all times, including during migration and after failure.*

## Acceptance Criteria

*How satisfaction is demonstrated — the evidence, not the intent.*

## Open Questions

*Unresolved points, each with who decides.*
