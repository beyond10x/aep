---
format: aep.planning-md/3
id: story:protocol-source-snapshot-publishes-on-macos
kind: story
status: implemented
title: A repository protocol source publishes its snapshot on macOS
summary: Seal the snapshot root after the rename into its snapshot path, so macOS no longer refuses the rename with Permission denied.
owner: project
tags:
- darwin
- protocol-source
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: .github/workflows/ci.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-project/src/project.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T13:34:18Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T13:34:18Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T14:17:21Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":2}}}
---
# Story: A Git protocol source publishes its snapshot on macOS

## Outcome

A project whose `protocols:` names a Git source loads on macOS as it does on Linux: the first command
materializes the pinned revision into the cache, publishes it at its snapshot path, and the snapshot is
read-only.

## Context

A downstream project's darwin CI (macos-15, aep 0.74.0, `aep-0.74.0-aarch64-apple-darwin.tar.gz`) failed
160 tests with `publishing snapshot <cache>/protocol-sources/<repo>/snapshots/<rev>: Permission denied (os
error 13)`; the same tests pass on Linux. The message comes from `materialize_git_source`
(`crates/edge/aep-project/src/project.rs`). It seals the checked-out tree, its root directory included
(mode 0555), and then renames the tree to its snapshot path in the same parent. 0.75.0 has the same code.

## Acceptance

1. `a_sealed_snapshot_is_published_with_a_read_only_root` (`aep-project`) seals a tree, publishes it, and
   asserts that the temporary tree is gone, the snapshot root and its subdirectories are mode 0555, its
   files 0444, and the published bytes verify against the manifest. It fails on macos-15 before the fix
   and passes on macos-15 and Linux after it.
2. The snapshot stays read-only: only the order of sealing and publishing changes.
3. CI runs that test and the Git-source CLI test `pinned_tree_before_judgement` on macos-15 for every
   pull request.
4. `CHANGELOG.md` gains an Unreleased `### Fixed` entry.

## Out of scope

- Removing a sealed temporary tree after a failed publish.
