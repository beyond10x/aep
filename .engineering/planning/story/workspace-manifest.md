---
format: aep.planning-md/3
id: story:workspace-manifest
kind: story
status: implemented
title: A workspace names its member repositories, and pins them
summary: 'One file naming the members and where each store is: a path, or a pinned git+...#<40-hex> locator. The pin is not decoration - a governing tree that can move under you is a dependency whose meaning changes with no commit in your repository.'
relations:
- decomposes: epic:one-cli-many-repositories
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-08-26T01:50:57Z", actor: "operator", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-08-26T01:50:58Z", actor: "operator", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-08-26T01:50:58Z", actor: "operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
# Story: A workspace names its member repositories, and pins them

## Outcome

Somebody can point one command at a set of repositories and get the same answer on their laptop and
in CI — because each member is named with a locator that cannot mean two different trees.

## Context

Nothing today says which repositories a question should be answered over. A project file says what
*this* repository runs under; there is no file that says what *several* repositories are. The
locator problem is already solved once, by `project.yaml`'s `protocols:`, and reusing that type
carries its two refusals with it for the same reasons.

## Acceptance

`.engineering/workspace.yaml` at `version: aep.workspace/1` with `members: [{name, source}]`;
`source` takes the same `ProtocolSource` locator, so an **absolute path is refused** — true on one
machine and false in CI — and an **unpinned git locator is refused**, because a tree that can move
under you is a dependency whose meaning changes with no commit in your repository; a pinned
`git+ssh://…#<40-hex>` is accepted; a generated JSON Schema is committed and checked; and the type
resolves nothing — a member nobody has checked out is a normal condition, not a broken workspace.

## Out of Scope

Fetching or cloning a member. The manifest declares; the shell resolves. Nothing here reaches the
network.

## Open Questions

None outstanding.
