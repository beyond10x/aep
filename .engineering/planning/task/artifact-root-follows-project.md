---
format: aep.planning-md/3
id: task:artifact-root-follows-project
kind: task
status: archived
title: Planning documents follow the project's configured protocol tree
summary: Make artifact lifecycle and template lookup honor .engineering/project.yaml when --root is omitted.
owner: protocol
tags:
- adoption
- bug
relations:
- derived_from: story:adopter-bugs
- decomposes: epic:zero-open-issues-2026-11-09
revision: 5
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T22:07:31Z", actor: "human:timo", revision: 5}
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

The original split was in `crates/edge/aep-cli/src/planning.rs`: `StoreLocation::store()` discovered
the project while `StoreLocation::lifecycles()` and `create()` used `StoreLocation.root` directly.
The lightweight resolver belongs in `aep-engine::project`; using the full `project::load` would
incorrectly couple source lookup to validation of unrelated project documents. Repository locators
are typed in `aep-domain`; the engine alone owns Git, cache, environment, and filesystem effects.

## Closed 2026-11-09

Fixed on main; not implemented by this epic. `StoreLocation::document_root` (`crates/edge/aep-cli/src/planning.rs:218-235`) takes `--root`, then the discovered project's `protocols` source, then `.`. Landed in `15009eda8` ("feat: resolve project protocol sources"). Held by `planning_documents_follow_the_protocol_tree_named_by_the_project` (`crates/edge/aep-cli/tests/planning_cli.rs:2016`), read at `824869fec` and run in this epic's gate.
