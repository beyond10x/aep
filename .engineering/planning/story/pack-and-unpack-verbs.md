---
format: aep.planning-md/3
id: story:pack-and-unpack-verbs
kind: story
status: active
title: aep plan store pack and unpack move closed work in and out of packs
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
- depends_on: story:lifecycle-declares-closed-states
- depends_on: story:pack-format-and-overlay-read
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T10:17:37Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-10T10:17:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":4}}}
---
# Story: `aep plan store pack` and `unpack` move closed work in and out of packs

## Outcome

`aep plan store pack [--dry-run]` writes one new pack holding every closed artifact whose bytes Git
has committed, with every evidence file in its evidence directory (including loose evidence recorded
against an artifact that is already packed), removes the loose files it packed, removes a loose copy
a pack already holds byte for byte, and sets `project.yaml` to `aep.project/6`.
`aep plan store unpack <id>… | --all [--dry-run]` writes those files back byte for byte, rewrites the
packs that held them, resolves a path a merge left in two places, and returns `project.yaml` to
`aep.project/5` when no pack remains.

## Context

`architecture-design:store-pack`, sections *Writes, and unpacking*, *Determinism* and *Git merge
behaviour*; closed states from `story:lifecycle-declares-closed-states`; the pack module and the
read overlay from `story:pack-format-and-overlay-read`.

## Acceptance

Every file `pack` or `unpack` moves is a file its dry run named, and the opposite verb restores it
byte for byte. Checked by CLI tests on fixture Git stores:

- `pack --dry-run` prints, per kind and status, the number of artifacts and evidence records it
  would pack, the pack's name, each candidate it would not pack with the reason (untracked, differs
  from `HEAD`), and the file counts under `.engineering/planning`, `.engineering/evidence` and
  `.engineering/packs` before and after; tree and index are unchanged afterwards; on a store whose
  protocol tree declares no `closed:` for a kind, the dry run says that kind packs its terminal
  states only;
- `pack` writes exactly the pack the dry run named, removes exactly the loose files it listed,
  repeats the counts, and leaves `project.yaml` at `version: aep.project/6`; running `pack` on two
  clones of the same commit writes the same bytes under the same name;
- after `pack` and a commit, `evidence <packed-id> …` adds a loose file; the next `pack` folds it into
  a new pack and removes it; a loose file byte-identical to a packed record is removed by `pack`
  without being packed twice;
- `unpack --all --dry-run` lists what it would restore and writes nothing; `unpack --all` after
  `pack` leaves `git status --porcelain` empty against the commit before `pack` (files restored,
  packs gone, `aep.project/5` back);
- `unpack <id>` restores that artifact's file and its evidence files, rewrites the pack without them
  under its new name, and refuses without writing anything a record path that fails the pack path
  grammar, and a target whose resolved location is outside `.engineering` or passes through a
  symbolic link (a symlinked `planning/story` directory in the fixture);
- merge cases, built by two branches that each `unpack` a different artifact from one pack and are
  then merged: a path left loose and packed is resolved by `unpack <id>` keeping the loose copy and
  removing the packed one; a path held by two packs with different bytes and no loose copy is refused
  by `unpack <id>` naming both packs, and `unpack <id> --from <pack-name>` restores that pack's copy
  and removes the other;
- outside a Git work tree `pack` exits 1 naming the reason and writes nothing; `pack` with nothing
  eligible writes nothing and says so;
- `aep plan store --help` lists `pack` and `unpack`.

## Shared surfaces

- `website/docs/reference/cli.md`: rows for `aep plan store pack` and `aep plan store unpack`
  (`docs-check` and the `cli_reference` test in `crates/edge/aep-cli/src/app.rs` require them).
- `CHANGELOG.md` (`## [Unreleased]` / `### Added`, one line for `pack` and `unpack`).

## Out of scope

- `validate`'s pack checks: `story:validate-checks-packs`.
- The 1000-story end-to-end test and the store documentation: `story:pack-end-to-end-and-docs`.
