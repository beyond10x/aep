---
format: aep.planning-md/2
id: story:early-tree-exports-carry-no-home-path-in-legacy-hex
kind: story
status: draft
title: The two trees exported before the hex rewrite carry no home path in their legacy bytes
relations:
- serves: vision:O2
- decomposes: epic:planning-on-entity-runtime
revision: 1
---
## Outcome

The eventlog and entity-runtime tree stores carry no home path inside hex-encoded legacy bytes,
as the four stores exported after aep#23 do not.

## Why

Those two stores were exported before the export rewrote inside hex. Their legacy provenance
blobs still decode to home-relative paths (the tilde form) (entity-runtime: 428 occurrences; eventlog not counted),
the same bytes their `aep.project/2` history already holds, and no names. `validate` S9 does not
decode hex, so it does not see them.

## Open question

A tree store's history files are immutable (V2), so removing them means either a second export
that replays the writes made since the cutover, or a store-level rewrite verb. Neither exists.
