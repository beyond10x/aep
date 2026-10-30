---
format: aep.planning-md/3
id: story:a-snapshot-parent-its-group-can-write-is-not-trusted-silently
kind: story
status: draft
title: A snapshot whose parent directory its group can write is not trusted silently
tags:
- protocol-source
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-project/src/project.rs
revision: 2
---
# Story: A snapshot whose parent directory its group can write is not trusted silently

## Outcome

A protocol source snapshot cannot be swapped, together with its manifest, by a principal that can
write a directory above it, without the operator having declared that principal trusted.

## Context

Found by the adversary pass on `story:a-group-writable-snapshot-is-admitted-explicitly`
(`crates/edge/aep-project/tests/group_writable_adversary.rs::a_sealed_snapshot_in_a_directory_its_group_can_write_is_refused_without_the_declaration`):
the writability check (`refuse_shared_write`, `crates/edge/aep-project/src/project.rs`) covers the
snapshot's own entries. If the group can write `protocol-sources/<repo>/snapshots/`, it can replace the
whole snapshot directory and its manifest, and the snapshot loads with no declaration. This predates
that story: no release checked the parent directories.

Measured by the adversary's test; what reaches it is a shared cache whose directories were created
under umask 002. Not established: whether any deployment shares a cache with another principal that
way. A per-user group (umask 002 with one user per group, a common Linux default) makes the parents
0775 with no other member, so refusing every group-writable parent would refuse ordinary caches.

## Acceptance

To be decided before this story leaves draft: which ancestors are checked (up to the cache root, or
the repository directory only), and how a per-user group is told apart from a shared one (owner and
group ids, or a declaration). The adversary case above is then held to the decision.

## Out of scope

- The snapshot's own entries: `story:a-group-writable-snapshot-is-admitted-explicitly`.
