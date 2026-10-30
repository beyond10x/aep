---
format: aep.planning-md/3
id: story:a-group-writable-snapshot-is-admitted-explicitly
kind: story
status: active
title: A group-writable protocol snapshot is admitted only when the operator says so
tags:
- confinement
- protocol-source
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-project/src/project.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T16:38:54Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-30T16:38:55Z", actor: "human:timo", revision: 6}
---
# Story: A group-writable protocol snapshot is admitted only when the operator says so

## Outcome

A protocol source cache on a Kubernetes volume with `fsGroup` loads when the operator declares the
volume's group trusted, with every byte still verified against the sealed manifest. Without that
declaration, a snapshot another principal can write is refused, and the refusal names the
declaration.

## Context

Under `fsGroup` the kubelet makes a cache's snapshot files 0664 and its directories 2775. aep records
`mode & 0o777` per file in the snapshot manifest (`file_mode`, `crates/edge/aep-project/src/project.rs:1122-1127`)
and compares the whole manifest (`verify_snapshot`, `project.rs:958-980`), so the load fails with
"protocol source snapshot … does not match its path, mode and byte manifest". Verified twice by the
reporter: a probe with `chmod -R g+w` on a populated cache (aep 0.77.1), and a pod run whose cache sat
on an `fsGroup: 10001` volume. The consumer works around it by re-sealing modes with `chmod -R
u-w,g-w,o-w` before every run.

What the mode check protects (`story:immutable-project-loader`: "pinned loads cannot move"): bytes are
hashed into the manifest, so a change to a file's bytes is caught whatever its mode. The modes add two
things: the executable bit of each file is part of the pinned content, and a snapshot that a different
principal can write could be rewritten together with its manifest, which no hash in the same directory
can catch. Owner write protects against nothing the owner cannot undo with `chmod`; the kubelet sets it
too (0444 becomes 0664).

## Decision

Taken by the wave coordinator under the operator's grant: make the check explicit rather than drop it.

- The manifest compares read and execute bits (`mode & 0o555`) and no write bits. Manifests already on
  disk were sealed at 0444 or 0555 and compare unchanged.
- A snapshot file or directory writable by others is always refused.
- A snapshot file or directory writable by its group is refused unless `AEP_CACHE_GROUP_WRITABLE=1`;
  the refusal names that variable and the first offending path.
- Owner write is admitted and nothing is re-sealed on a found snapshot that is group-writable: the
  kubelet re-applies the modes on every mount.

## Acceptance

1. A sealed snapshot whose files are made 0664 and directories 2775 loads with
   `AEP_CACHE_GROUP_WRITABLE=1`, and every file's bytes are still verified: changing one byte in it is
   refused.
2. The same snapshot without the variable (or with any value other than `1`) is refused, and the
   message names `AEP_CACHE_GROUP_WRITABLE` and a group-writable path.
3. A snapshot with any file or directory writable by others is refused with or without the variable.
4. A snapshot whose files gained only owner write (0644) loads with no variable.
5. A file whose executable bit was added or removed is refused (the manifest still holds it).
6. The cross-process stamp does not carry the admission over: a snapshot verified and stamped with
   the variable set is refused by a later load without it.
7. `CHANGELOG.md` gains an Unreleased `### Fixed` entry; `website/docs/reference/cli.md` lists
   `AEP_CACHE_GROUP_WRITABLE`.

## Out of scope

- A bounded fetch when the cache is empty: `story:an-unreachable-protocol-source-fails-within-a-deadline`.
- Changing the consumer's pod security context.
