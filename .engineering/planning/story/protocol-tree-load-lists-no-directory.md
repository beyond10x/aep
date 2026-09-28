---
format: aep.planning-md/3
id: story:protocol-tree-load-lists-no-directory
kind: story
status: draft
title: Loading a pinned protocol tree opens no directory once its snapshot is stamped
relations:
- serves: vision:O2
revision: 1
---
## Problem

After 0.64.0 a stamped protocol snapshot is verified without opening a directory, but loading the
protocol tree still lists `protocols/` and `principles/`: 40 directory opens under
`protocol-sources` per `aep plan artifact new task`, 20 per `list` (measured on a store pinned to
`88836a30`, strace, 2026-09-28).

## Outcome

The verification stamp's directory map, or the sealed manifest, gives the loader its file list, so a
stamped command opens no snapshot directory at all.

## Acceptance

- strace of `aep plan artifact new task` on a stamped pin: 0 directory opens under `protocol-sources`.
