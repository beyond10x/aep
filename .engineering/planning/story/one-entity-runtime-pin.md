---
format: aep.planning-md/1
id: story:one-entity-runtime-pin
kind: story
status: implemented
title: One entity-runtime pin, and a gate step that keeps it one
summary: Two versions of entity-core are compiled into this workspace (0.5.2 and 0.8.0); one tag, a dep-check gate step, and the Dependencies section corrected.
owner: store
tags:
- backend
- store
relations:
- decomposes: epic:planning-store-as-backend
revision: 7
---
# Story: One `entity-runtime` pin, and a gate step that keeps it one

## Outcome

Somebody reading `Cargo.lock` finds one `entity-core`, and a story that says "the runtime does X" is
about the runtime this workspace actually compiles.

## Context

`cargo tree -i entity-core` answered *"specification is ambiguous: entity-core@0.5.2, entity-core@0.8.0"*.
`crates/aep-backend-markdown/Cargo.toml:31` pinned `tag = "0.5.2"`; `crates/aep-backend-sqlite/Cargo.toml:21-23`
pinned `tag = "0.8.0"`; `entity-runtime` was at `0.9.1`. Two kernels were compiled, and the one that
decided every `protocol artifact move` was four releases behind the one under the SQLite backend —
including 0.8.0's fixes to `entity-sqlite`'s locking and `FileStore`'s write path, which the markdown
side's kernel predated.

`AGENTS.md:528` read *"the eleventh is `entity-core` in `aep-backend-markdown`, the only
dependency"*. Three crates from that repository were in two of ours, and `entity-sqlite` brings a
bundled SQLite (a C build) that the § *Dependencies* policy says must be named with the refusal
alternatives considered.

Wave F, story 1 (`docs/plan/store-waves-f-g-h.md`). First because F2 cannot be written against two kernels.

## Acceptance

- Every `entity-*` dependency in the workspace names the same tag, and it is the newest
  `entity-runtime` release on the day this lands (`0.9.1` at writing). **Done** — three lines, one
  tag (`crates/aep-backend-markdown/Cargo.toml:31`, `crates/aep-backend-sqlite/Cargo.toml:21-23`).
- `cargo tree -i entity-core` is unambiguous; `tests/kernel_equivalence.rs` and both conformance
  suites pass against the single version. **Done** — `entity-core v0.9.1 (…tag=0.9.1#dc5b25a4)` is
  the one answer; `kernel_equivalence` 5 passed, markdown `conformance` 11 passed, sqlite
  `conformance` 4 passed, all 113 tests of the two backend crates green on 2026-08-28.
- A gate step (`dep-check`, in `task check` and in CI) fails when any crate whose name starts with
  `entity-` resolves to two versions, and its failure message names both. **Done** —
  `cargo xtask deps` (`xtask/src/main.rs`, `fn deps`), `dep-check` in `Taskfile.yml` after
  `version-check`, and a step in the `status` job of `.github/workflows/ci.yml`. It also refuses two
  *pins* — `entity-*` crates from two sources — because one version each from two tags is still two
  runtimes. Seven tests in `xtask`'s `dep_tests`, including one over the committed lockfile and one
  over the exact shape the lockfile had before this story, which fails with
  `entity-core at 0.5.2 and 0.8.0`.
- `AGENTS.md` § *Dependencies* names `entity-core`, `entity-store`, `entity-sqlite` and the bundled
  SQLite, with the alternatives refused, and § *Boundaries* no longer says one crate. **Done** — and
  § *Gate* names `dep-check` beside the four other source-only steps the list had omitted; the same
  justification sits in `crates/aep-backend-sqlite/Cargo.toml` beside the lines.
- `CHANGELOG.md` names the bump as a `### Changed` entry a user can act on: which runtime behaviours
  the move-deciding kernel gains. **Done** — `DomainEvent` gains `from_state`/`to_state`/`changed`
  (their R-89) and `entity_core::rehydrate` folds and refuses a forged creation (R-97); no ladder
  verdict changes.

## Decision taken

D-F2 (`docs/plan/store-waves-f-g-h.md` § 4): the pin is the newest release on the day, `0.9.1`.
F4 (`story:sqlite-hydrates-on-open`) needs `entity-runtime`'s `story:store-enumeration`, which is
not in `0.9.1`; F4 moves the pin to the release that carries it, and `dep-check` is what keeps that
move one line in three places rather than one line in one of them.

## Out of Scope

Advancing to a floating `main`. The pin is the reversible half of `atlas/architecture/adr/0002`, and
a tag is what makes "the runtime at this commit" a sentence a review can check.

## Open Questions

None outstanding.
