---
format: aep.planning-md/1
id: story:frame-subjects-from-the-step-map
kind: story
status: draft
title: Vendor arms carry the same scope, sealed into the frame
summary: Compile the step map's coarse scope into Frame.subjects so claude and codex are bounded by the document rather than by prose they happen to load.
owner: eval
tags:
- eval
- metaharness
relations:
- decomposes: epic:self-evaluation
revision: 1
---
# Story: the vendor arms get the same boundary

## Outcome

A run of `claude` or `codex` is bounded by the same document a run of `native` is. Today it is bounded
by a 157-line skill file it happens to load, which is prose that teaches rather than a boundary that
holds.

## Context

The comparison in the eval matrix is currently unfair in both directions. Before 2026-08-24, arm
`native` carried no domain knowledge and `claude+plugin` carried a skill; that gap is closed by
`context:`. What is still open is the reverse: `native` is now bounded by its toolset and the vendor
arms are bounded by nothing a document can see.

`Frame.subjects` already exists, is sealed into the frame's digest, and is read by the seam's verdict.
`tool.requested.subjects` carries `file:<path>` on every arm. The missing piece is the compile from
the step map's coarse, operation-agnostic words (`allowed`, `partial-only`, `denied`) into the frame's
precise per-operation form — and that compile belongs in the adapter, because *which of my operations
replace a file whole* is the adapter's own fact.

## Acceptance

- A step carrying `scope:` produces a frame whose `subjects` say the same thing in operation terms.
- `partial-only` compiles to: the whole-file operation denied for those subjects, the partial one
  admitted. A step map author never writes an operation name.
- A denial at the seam names the path and the admitted alternative, in that harness's own vocabulary.
- One live driven run reaches for a whole-file store write and is refused.

## Out of Scope

`proc:` subjects. The declared-program set already bounds execution, and a second bound before anybody
has wanted one is speculative.

## Open Questions

None. The layering is settled by design § 3.
