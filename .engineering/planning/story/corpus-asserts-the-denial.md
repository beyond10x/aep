---
format: aep.planning-md/1
id: story:corpus-asserts-the-denial
kind: story
status: draft
title: A corpus row that asserts the refusal, not the absence
summary: no-artifact-file-was-rewritten-whole now passes by construction on the scoped arm; add the sibling that fails when nothing was refused.
owner: eval
tags:
- eval
- trace
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: a control that can still fail

## Outcome

Whoever reads the eval report can tell the difference between *the model behaved* and *the harness
stopped it*. Right now the row that was supposed to show this passes either way.

## Context

`no-artifact-file-was-rewritten-whole` asserts an **absence**: no call matched `file.write` on a
planning path. On a scoped arm the toolset refuses that call before it runs, so the absence is
guaranteed by construction — and a control that cannot fail has stopped testing anything.

The 2026-08-24 runs make the distinction concrete. With the scope stated the run produced zero
refusals and the row passed; with it unstated the run produced five refusals and the row passed
identically. One of those runs demonstrated the harness; the other demonstrated the prompt.

## Acceptance

- A sibling expectation asserts that the reach was **refused**, and fails on a transcript where the
  scope was never tested.
- It is written in the neutral vocabulary — operations and subjects — so it is not blind on any arm.
- Both rows appear in the case's expectations, and the report distinguishes them in one reading.

## Out of Scope

Failing the run on a denial. Denials are counted, never fatal — design § 5.

## Open Questions

- Should the new row be advisory? A run that never reached outside its scope is not a bad run.
  Decided when the row is written; the default is advisory.
