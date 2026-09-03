---
format: aep.planning-md/1
id: task:artifact-root-follows-project
kind: task
status: draft
title: Planning documents follow the project's configured protocol tree
summary: Make artifact lifecycle and template lookup honor .engineering/project.yaml when --root is omitted.
owner: protocol
tags:
- adoption
- bug
relations:
- derived_from: story:adopter-bugs
revision: 2
---
# Task: Planning documents follow the project's configured protocol tree

## What

Make every `protocol artifact` operation that needs lifecycle or template documents resolve them from
the `protocols` source in the discovered `.engineering/project.yaml` when `--root` is omitted. A
source may be a local tree or an immutable Git repository locator. Preserve `--root` as the explicit
override and preserve vocabulary-only commands outside a project.

## Why

The planning store already follows project discovery, but lifecycle and template loading independently
defaults to `.`. That split makes an adopter reorganize its repository around an implementation defect
or remember an extra flag even though its project configuration already names the document tree. It is
the same adopter-facing configuration-integrity concern captured by `story:adopter-bugs`, but is a
distinct reproduced defect rather than A3's question about merging project-local workflow documents.

## Done When

- From a nested directory of a project whose configured protocol tree is outside the repository,
  `protocol artifact lifecycle story` reads that tree without `--root`.
- `protocol artifact new story ...` uses both the configured lifecycle's initial state and the
  configured template body.
- A `git+ssh://...#<full-commit>` source carries no cross-repository filesystem path, materializes
  under the protocol cache, verifies the commit, and remains usable from that cache without the
  repository being reachable.
- An explicit `--root` still wins over project discovery.
- Resolving the protocol path does not require the project's unrelated artifact manifest, task, local
  profiles, or full protocol/profile pairing to load successfully.
- A regression test fails with the former `root = "."` behavior and passes with the repair.
- The repository's targeted tests and required gates pass.

## Notes

The original split was in `crates/edge/protocol-cli/src/planning.rs`: `StoreLocation::store()` discovered
the project while `StoreLocation::lifecycles()` and `create()` used `StoreLocation.root` directly.
The lightweight resolver belongs in `aep-engine::project`; using the full `project::load` would
incorrectly couple source lookup to validation of unrelated project documents. Repository locators
are typed in `aep-domain`; the engine alone owns Git, cache, environment, and filesystem effects.
