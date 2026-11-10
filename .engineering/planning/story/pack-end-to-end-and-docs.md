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
revision: 2
---
# Story: a 1000-story store packs and unpacks with every answer unchanged, and the docs say how

## Outcome

An end-to-end test generates a Git store of at least 1000 closed stories with evidence, plus open
artifacts relating to some of them, and shows the tracked file count before packing, after packing
and after unpacking, with identical output from `validate`, `history <id>`, `explain <id>`,
`list --status <closed status>`, `findings <id>` and `graph` at all three points. The store's design
document and the website's store pages describe `pack`, `unpack`, the `aep.pack/1` format and
`aep.project/6`.

## Context

`architecture-design:store-pack`; `architecture-decision-record:packed-store-is-aep-project-6`.
The CLI reference rows and the changelog lines belong to the stories that add each change.

## Acceptance

On a generated store of at least 1000 closed stories with evidence, `pack`, a commit and
`unpack --all` change the tracked file count and nothing any read verb prints. Checked by:

- an end-to-end test that prints the tracked file count under `.engineering` before `pack`, after
  `pack` and its commit, and after `unpack --all`; it asserts the after-pack count equals the before
  count minus the files packed plus the packs written, and that after-unpack equals before;
- in that test, byte-identical stdout and identical exit codes of `validate`, `history <id>`,
  `explain <id>`, `list --status implemented`, `findings <id>` and `graph` at the three points;
- `docs/design/git-native-planning-store-v0.1.md` gaining a section that contains the strings
  `aep.pack/1`, `aep.project/6`, `aep plan store pack` and `aep plan store unpack`, and
  `website/docs/concepts/planning-store.md` and `website/docs/reference/planning-stores.md` each
  containing `aep.project/6` and `aep plan store pack`; `docs-check` and the website build pass.

## Out of scope

- The `aep:planning` skill and `references/store-conventions.md`, which live in `agentplugins`.
- The CLI reference rows (`story:pack-and-unpack-verbs`) and changelog lines (each story).
