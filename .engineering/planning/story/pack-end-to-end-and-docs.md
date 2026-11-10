---
format: aep.planning-md/3
id: story:pack-end-to-end-and-docs
kind: story
status: draft
title: A 1000-story store packs and unpacks with every answer unchanged, and the docs say how
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
- depends_on: story:pack-and-unpack-verbs
- depends_on: story:validate-checks-packs
revision: 1
---
# Story: a 1000-story store packs and unpacks with every answer unchanged, and the docs say how

## Outcome

An end-to-end test generates a Git store of at least 1000 closed stories with evidence, plus open
artifacts relating to some of them, and shows the tracked file count before packing, after packing
and after unpacking, with identical output from `validate`, `history <id>`, `explain <id>`,
`list --status <closed status>`, `findings <id>` and `graph` at all three points. The store design
document, the website's CLI reference and store pages, and `CHANGELOG.md` (Unreleased) describe
`pack`, `unpack`, the `aep.pack/1` format and `aep.project/6`.

## Context

`architecture-design:store-pack`; `architecture-decision-record:packed-store-is-aep-project-6`.
The CLI reference must list every verb (`docs-check`).

## Acceptance

1. The test prints the tracked file count under `.engineering` before `pack`, after `pack` (and its
   commit), and after `unpack --all`; it asserts the after-pack count is lower than the before count
   by at least the number of packed files minus the number of packs, and that after-unpack equals
   before.
2. The test asserts byte-identical stdout and identical exit codes of the six verbs above at the
   three points, over at least 1000 closed stories with evidence.
3. `docs/design/git-native-planning-store-v0.1.md` (or a successor design it points to),
   `website/docs/reference/cli.md` and the website page for the store describe the verbs, the pack
   format and the version; `docs-check` passes.
4. `CHANGELOG.md` has an `## [Unreleased]` entry for `pack`, `unpack`, `aep.project/6` and the
   lifecycle `closed:` key.

## Out of scope

- The `aep:planning` skill and `references/store-conventions.md`, which live in `agentplugins`.
