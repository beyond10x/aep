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
revision: 2
---
# Story: every reader answers over packed records as over loose files

## Outcome

`aep-backend-markdown` reads a store as one tree of files: the loose files under `planning/` and
`evidence/` plus every record of every `.engineering/packs/*.pack` (`aep.pack/1`). Every reader that
walks the store today reads through it, so each verb answers identically whether a record is loose
or packed. `aep` accepts `aep.project/6`. A write that would rewrite a packed artifact, and `new` for
a packed id, refuse by name.

## Context

`architecture-design:store-pack`, sections *The packed representation* (items 1–2, format and name),
*Where reads go: one overlay* and the first three bullets of *Writes, and unpacking*. Readers to move
onto the overlay are cited in the design's table *How the store is read today*. Version:
`architecture-decision-record:packed-store-is-aep-project-6`.

The pack module (parse, render, digest, path grammar) is owned here; `pack`/`unpack` verbs and
`validate`'s pack checks build on it in `story:pack-and-unpack-verbs` and
`story:validate-checks-packs`.

## Acceptance

Every read verb prints byte-identical output over a fixture store whether its closed artifacts and
their evidence are loose or held in a pack written by this story's pack module, and every write that
would rewrite a packed artifact refuses by name and writes nothing. Checked by:

- a CLI test comparing captured stdout and exit codes of `list`, `list --status implemented`,
  `graph`, `show <id>`, `history <id>`, `explain <id>`, `findings <id>`, `review-value`, `blocked`,
  `waves --status active` and `spend` before and after the test moves the fixture's closed artifacts
  and their evidence files into a pack rendered by the pack module (no verb writes packs yet);
- in that test, an open artifact with a `depends_on` edge to a packed artifact still validates, and
  `relate <open> depends_on <packed>` succeeds and changes only the open artifact's file;
- a CLI test that `new story <name>` where `story:<name>` is packed exits 1 naming the pack, and that
  `move`, `body`, `set`, `scope`, `relate`, `unrelate` and `redact` on a packed artifact exit 1 naming
  `aep plan store unpack <id>` with the file tree unchanged;
- a CLI test that `evidence <packed-id> --kind test_result …` writes one new loose evidence file and
  changes no pack;
- unit tests of the pack module: `render` sorts by path and is a function of the (path, bytes) set;
  the name is the first 16 hex of the SHA-256 of the pack's bytes; `parse` refuses a truncated
  record, a length past the end, an unsorted or repeated path, and a path outside the grammar
  (`planning/<kind>/<name>.md`, `evidence/<kind>/<name>/<file>.json`; no `..`, absolute, backslash,
  empty or dot segment, control character), each with a stable code;
- a test that `version: aep.project/6` loads and every verb that accepts `/5` accepts it, and that
  `version: aep.project/7` is refused with `unsupported_protocol_version` naming the version (the
  mechanism by which aep 0.79.0 refuses `/6`).

## Shared surfaces

- `crates/edge/aep-cli/src/planning.rs`: this story changes `evidence_problems` and the `redact`
  evidence listing; `story:lifecycle-declares-closed-states` changes only `fn lifecycle`. Split by
  symbol.
- `CHANGELOG.md` (`## [Unreleased]` / `### Added`, one line for `aep.project/6` being read) and
  `website/docs/reference/project-file.md` (`aep.project/6`).

## Out of scope

- Writing packs from the store, unpacking: `story:pack-and-unpack-verbs`.
- `validate`'s checks of pack integrity and provenance, and the Git history walk across a pack
  commit: `story:validate-checks-packs`. Fixtures here commit loose files only, or commit no packed
  review-result, so this story's `validate` runs do not depend on that walk.
