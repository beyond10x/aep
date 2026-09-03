---
format: aep.planning-md/1
id: story:bulk-create-from-a-manifest
kind: story
status: draft
title: Many artifacts arrive in one command
summary: new creates one artifact per call, so every adopter with a backlog writes the same loop and answers partial failure differently.
scope:
- confidence: cited
  path: crates/edge/protocol-cli/src/planning.rs
revision: 4
---
## Context

`aep artifact new` creates one artifact per invocation. A repository adopting AEP with an existing
backlog therefore runs it once per item, and every caller that does this has had to write the loop
itself: `story-migration` in `beyond10x/agentplugins` is a six-step procedure whose fourth step is
"call `new` once per row of the table you produced in step two", and the migration it drove in
`sbf/acd` issued 22 of them.

The loop is not the hard part. What each caller reimplements is what to do when one of the 22 fails
— whether to stop, whether to roll back the 14 that landed, and how to report a partial write — and
each reimplementation answers it differently.

## Acceptance

- One verb creates many artifacts from a manifest: a file of documents, or a directory of them.
- Its failure behaviour is one documented answer rather than a caller's choice, and the answer is
  written in the refusal when it happens.
- Re-running it over a manifest whose artifacts already exist is not an error and creates nothing —
  the property `story-migration` currently asserts by counting files afterwards.
- Relations inside the manifest resolve, so a manifest can name an epic and the stories under it in
  one file.

## Evidence for the gap

`crates/edge/protocol-cli/src/planning.rs` — `create()` takes one `NewArgs` and builds one
`PlanningDocument`. The `story-migration` skill's § 3 is the workaround, and its
`aep-planning` 0.3.7 revision documents that each duplicate create exits non-zero, which is the
detail every caller's loop has to handle.
