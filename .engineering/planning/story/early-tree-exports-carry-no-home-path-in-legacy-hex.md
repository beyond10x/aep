---
format: aep.planning-md/2
id: story:early-tree-exports-carry-no-home-path-in-legacy-hex
kind: story
status: rejected
title: The two trees exported before the hex rewrite carry no home path in their legacy bytes
relations:
- serves: vision:O2
- decomposes: epic:planning-on-entity-runtime
revision: 4
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


## Rejected (2026-09-26)

Not built, by decision in wave 0021 (`.ess-evolution/waves/0021-followups/WAVE.md`).

- The bytes are not only in the tree. The same legacy provenance, with the same home-relative paths, is in the eventlog and
  entity-runtime repositories' `aep.project/2` history on `main`, which every clone carries. Removing it from
  the tree files would not remove it from Git.
- The tree's history files are immutable by design (V2). Removing the paths needs either a second export that
  replays every write since the cutover, or a store-level rewrite verb, and neither exists. Building one only to
  hide bytes that stay in Git history is cost without benefit.
- What the bytes hold: home-relative paths (the tilde form), no names; `validate` S9 passes. Exports made
  since aep#23 rewrite inside hex, so no new store gains such bytes.

Reopen if a repository has to remove the paths from its history too; that is a history rewrite, not a store
change.
