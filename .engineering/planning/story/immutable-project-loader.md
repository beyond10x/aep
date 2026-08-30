---
format: aep.planning-md/1
id: story:immutable-project-loader
kind: story
status: implemented
title: Project acquisition is an immutable edge outside the engine
summary: Move IO loading out of the engine and verify pinned Git snapshots before reuse.
relations:
- derived_from: epic:architecture-hardening
- serves: vision:O2
revision: 4
---
## Finding

The semantic engine owns filesystem, environment, schema and Git acquisition, including a reusable checkout whose tracked or untracked bytes can change after the requested commit is pinned. Registry also carries driver-specific maps.

## Acceptance

A new edge crate owns discovery, parsing and acquisition. Engine registry contains semantic protocol documents only and loses schema/driver dependencies. Git objects live in a bare cache; a verified full commit is archived into a read-only snapshot with a manifest over paths, modes and bytes. Reuse revalidates the manifest. Symlinks and credential-bearing URLs are refused. Tests tamper with tracked and added files and prove pinned loads cannot move.

## Scope

- `crates/aep-engine/src/load.rs`, `project.rs`, `registry.rs` and manifests — cited.
- CLI and driver imports — inferred from callers; confirm with `rg`.
