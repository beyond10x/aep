---
format: aep.planning-md/3
id: story:pack-and-unpack-verbs
kind: story
status: draft
title: aep plan store pack and unpack move closed work in and out of packs
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
- depends_on: story:lifecycle-declares-closed-states
- depends_on: story:pack-format-and-overlay-read
revision: 1
---
# Story: `aep plan store pack` and `unpack` move closed work in and out of packs

## Outcome

`aep plan store pack [--dry-run]` writes one new pack holding every closed artifact whose bytes Git
has committed, with every evidence file in its evidence directory, removes the loose files it packed
and sets `project.yaml` to `aep.project/6`. `aep plan store unpack <id>… | --all [--dry-run]` writes
those files back byte for byte, rewrites the packs that held them, and returns `project.yaml` to
`aep.project/5` when no pack remains.

## Context

`architecture-design:store-pack`, sections *Writes, and unpacking*, *Determinism* and *Git merge
behaviour*; closed states from `story:lifecycle-declares-closed-states`; the pack module and the
read overlay from `story:pack-format-and-overlay-read`.

## Acceptance

1. `pack --dry-run` on a fixture store prints, per kind and status, the number of artifacts and
   evidence records it would pack, the pack's name, each candidate it would not pack with the reason
   (untracked, differs from `HEAD`), and the file counts under `.engineering/planning`,
   `.engineering/evidence` and `.engineering/packs` before and after; it writes nothing (tree and
   index unchanged). Checked by a CLI test asserting the lines.
2. `pack` writes exactly the pack the dry run named, removes exactly the loose files it listed, and
   its output repeats the counts. Running `pack` on two clones of the same commit writes the same
   bytes under the same name.
3. `unpack --all` after `pack` leaves `git status --porcelain` showing no change against the commit
   before `pack` (every file restored byte for byte, packs and `/6` gone).
4. `unpack <id>` restores that artifact's file and its evidence files, rewrites the pack without
   them under its new name, and refuses a path that fails the pack path grammar without writing
   anything; when a loose file already holds the path it keeps the loose file and removes the packed
   copy.
5. Outside a Git work tree `pack` exits 1 naming the reason and writes nothing; a second `pack` with
   nothing eligible writes nothing and says so.
6. `aep plan store --help` lists `pack` and `unpack`.

## Out of scope

- `validate`'s pack checks: `story:validate-checks-packs`.
- The 1000-story end-to-end test and documentation: `story:pack-end-to-end-and-docs`.
