---
format: aep.planning-md/3
id: story:discarded-protocol-source-tree-is-removed
kind: story
status: draft
title: A protocol-source publisher that does not publish removes its sealed tree
summary: A temporary snapshot tree that is not published is removed even though seal_snapshot made its directories read-only.
tags:
- protocol-source
relations:
- serves: vision:O2
scope:
- confidence: inferred
  path: .github/workflows/ci.yml
- confidence: cited
  path: crates/edge/aep-project/src/project.rs
revision: 3
---
# Story: A protocol-source publisher that does not publish removes its sealed tree

## Outcome

When `materialize_git_source` does not publish its temporary tree, the tree is gone afterwards:
nothing named `.<rev>-<pid>-<n>.tmp` stays under `protocol-sources/<repo>/snapshots/`.

## Context

`seal_snapshot` makes every file 0444 and every directory below the tree's root 0555. Two paths
then discard the tree with `std::fs::remove_dir_all(...).ok()`, which cannot unlink entries inside a
0555 directory and whose error is ignored:

- `crates/edge/aep-project/src/project.rs:500`: `publish_snapshot` loses the rename race to another
  process that published the same revision first.
- `crates/edge/aep-project/src/project.rs:472`: the checkout or `seal_snapshot` fails after some
  directories were sealed.

The leak predates 0.76.0; the 0.76.0 change (story:protocol-source-snapshot-publishes-on-macos)
moved the root's seal after the rename and left these paths as they were. Found by the adversary
review of that change. Each leaked tree is a full checkout of the source revision, and nothing
removes it later.

The predicate below reproduces the layout `seal_snapshot` leaves (a 0555 directory holding a 0444
file) and the removal those paths make; it prints how many files survive. It checks the mechanism on
the filesystem, not the `aep` binary.

```expected_before
command: d=$(mktemp -d) && mkdir -p "$d/.r.tmp/profiles" && echo x > "$d/.r.tmp/profiles/p.yaml" && chmod 444 "$d/.r.tmp/profiles/p.yaml" && chmod 555 "$d/.r.tmp/profiles" && rm -rf "$d/.r.tmp" 2>/dev/null; find "$d" -name p.yaml | wc -l | tr -d ' '; chmod -R u+w "$d"; rm -rf "$d"
exit: 0
output: 1
source: crates/edge/aep-project/src/project.rs:500
```

## Acceptance

1. A unit test in `crates/edge/aep-project/src/project.rs` publishes a sealed tree onto a
   destination that already exists and asserts that the tree no longer exists afterwards; it fails
   on 04da2ef2f.
2. A unit test seals part of a tree, makes the materialization fail, and asserts that the tree no
   longer exists afterwards.
3. The published snapshot keeps its modes: files 0444, directories 0555, the root sealed after the
   rename.
4. The `macos` CI job runs both tests on macos-15.

## Out of scope

- Removing `.tmp` trees that earlier versions already leaked.
