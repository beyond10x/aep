---
format: aep.planning-md/1
id: story:retire-store-integrity-paths
kind: story
status: implemented
title: store_integrity keeps its fence and loses its paths
summary: Delete the path half of drive.rs's store_integrity now the step map declares it; the frontmatter-fence half stays, because content is not a scope.
owner: protocol
tags:
- driver
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 4
---
# Story: one rule, one place

## Outcome

A person reading the workflow can see every rule that governs a run. Today one of them is a Rust
function that only the Claude arm can reach, written in that vendor's tool names.

## Context

`crates/protocol-cli/src/drive.rs` holds `store_integrity`, and it is two rules wearing one name:

1. *no whole-file replacement under `.engineering/planning/**`* — a question about path and
   granularity, now declared in the step map as `write: partial-only` and enforced by the toolset.
   Measured holding on 2026-08-24 with the rule unstated: five refusals fired and no artifact file was
   rewritten whole.
2. *no edit whose text crosses the closing `---`* — a question about the **content** of an edit. Not
   expressible as a scope, and the corpus already says so: not transcript-decidable from a path.

The first is now declared twice. Two copies of one rule disagree the first time one moves, and this
copy is the one nobody outside the Claude arm can even read: it matches on `Write`, `NotebookEdit`,
`file_path` and `old_string`, so every other arm walked straight past it for a year.

## Acceptance

- The path half is gone from `drive.rs`; the fence half stays, with a comment saying why it is not a
  scope.
- The driven Claude arm is still refused a whole-file store rewrite — from the declaration, through
  the seam, not from the function.
- A test proves the fence rule still fires on an edit that crosses `---`.

## Out of Scope

Deleting `store_integrity` entirely. Content rules stay in code; only the path rule moves.

## Open Questions

None. The split is settled by design § 2.
