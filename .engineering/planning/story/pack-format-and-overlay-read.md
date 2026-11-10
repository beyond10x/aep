---
format: aep.planning-md/3
id: story:pack-format-and-overlay-read
kind: story
status: draft
title: Every reader answers over packed records as over loose files
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
revision: 1
---
# Story: every reader answers over packed records as over loose files

## Outcome

`aep-backend-markdown` reads a store as one tree of files: the loose files under `planning/` and
`evidence/` plus every record of every `.engineering/packs/*.pack` (`aep.pack/1`). Every reader that
walks the store today reads through it, so each verb answers identically whether a record is loose
or packed. `aep` accepts `aep.project/6`. A write that would rewrite a packed artifact, and `new` for
a packed id, refuse by name.

## Context

`architecture-design:store-pack`, sections *The packed representation* (items 1–2, format and name)
and *Where reads go: one overlay* and the first three bullets of *Writes, and unpacking*. Readers to
move onto the overlay are cited in the design's table *How the store is read today*. Version:
`architecture-decision-record:packed-store-is-aep-project-6`.

The pack module (parse, render, digest, path grammar) is owned here; `pack`/`unpack` verbs and
`validate`'s pack checks build on it in `story:pack-and-unpack-verbs` and
`story:validate-checks-packs`.

## Acceptance

1. On a fixture store, the outputs of `list`, `list --status implemented`, `graph`, `show <id>`,
   `history <id>`, `explain <id>`, `findings <id>`, `review-value`, `blocked`, `waves --status active`
   and `spend` are byte-identical before and after the fixture's closed artifacts and their evidence
   files are moved into a pack written by the pack module's renderer (the test writes the pack; no
   verb exists yet). Checked by a CLI test comparing captured stdout.
2. An open artifact with a `depends_on` edge to a packed artifact still validates, and
   `relate <open> depends_on <packed>` succeeds and writes only the open artifact's file.
3. `new story <name>` where `story:<name>` is packed exits 1 naming the pack; `move`, `body`, `set`,
   `scope`, `relate`, `unrelate` and `redact` on a packed artifact exit 1 naming
   `aep plan store unpack <id>`, and write nothing (file tree unchanged).
4. `evidence <packed-id> --kind test_result …` writes one new loose evidence file and changes no pack.
5. The pack module: `render` sorts records by path and is a function of the (path, bytes) set; the
   name is the first 16 hex of the SHA-256 of the pack's bytes; `parse` refuses a truncated record, a
   length past the end, an unsorted or repeated path, and a path outside the grammar
   (`planning/<kind>/<name>.md`, `evidence/<kind>/<name>/<file>.json`; no `..`, absolute, backslash,
   empty or dot segment, control character), each with a stable code. Checked by unit tests asserting
   the code.
6. A project file with `version: aep.project/6` loads; every verb that accepts `/5` accepts `/6`.

## Out of scope

- Writing packs from the store, unpacking: `story:pack-and-unpack-verbs`.
- `validate`'s checks of pack integrity and provenance: `story:validate-checks-packs`.
