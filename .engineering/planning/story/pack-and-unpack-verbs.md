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
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/app.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command/migrate_git.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command/pack.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command/unpack.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_adversary.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/pack.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 18
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

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/edge/aep-cli/src/store_command.rs`, the `aep plan store` verb group — cited, the design names `aep plan store pack`/`unpack` and this file holds the group
- **Files:** `crates/edge/aep-cli/src/store_command.rs` `StoreCommand` (declared at :17, gains `Pack` and `Unpack` variants), `run` (:260, dispatch), module doc :1-6 ("the one store operation this build keeps") — cited
- **Files:** `crates/edge/aep-cli/src/app.rs` `PlanCommand::Store` (:275-281) and its dispatch `PlanCommand::Store { command } => store_command::run(command)` (:995); the `/// Move the selected planning store to another layout` doc comment is now incomplete — inferred, the wording edit is optional
- **Files:** `crates/edge/aep-cli/src/app.rs` `mod cli_reference` (:2874), `every_verb_the_cli_answers_has_an_entry_in_the_reference` (:2926). It walks the clap tree, so it needs no edit but fails until `cli.md` has the rows — cited
- **Files:** new `crates/edge/aep-cli/src/store_command/pack.rs` (and `unpack.rs`, or one module), beside `store_command/migrate_git.rs` — inferred, follows the `mod migrate_git;` pattern (store_command.rs:13)
- **Files:** new `crates/edge/aep-cli/tests/store_pack.rs`, CLI tests on fixture Git stores, modelled on `crates/edge/aep-cli/tests/store_migrate_git.rs` (`scratch`, `run_git`, `commit_all`, `contents`) — inferred
- **Symbols to reuse, not change:** `migrate_git::git_clean` (migrate_git.rs:396, `git -C <engineering> status --porcelain -- .`, private) and the clean-tree refusal in `run` (:292-299) and `repair` (:541-548); `PlanningWriterFence::acquire` (planning_writer_fence.rs:219, taken at migrate_git.rs:290); `lifecycles_of` (migrate_git.rs:251); `selector_v5` (migrate_git.rs:897), the only project.yaml rewrite today (`serde_yaml` round trip, then `aep_schema::parse::project` reads it back); `fs::write(&selector_path, …)` at :338; `engineering_of` (:233) — cited
- **Also likely:** `crates/edge/aep-cli/src/store_command/migrate_git.rs`, if `git_clean`, `selector_v5`, `engineering_of` or `lifecycles_of` are moved or made `pub(super)` to share them; otherwise untouched — inferred
- **Also likely:** `crates/plan/aep-backend-markdown/src/pack.rs` (name assumed; created by `story:pack-format-and-overlay-read`), extended with a "rewrite a pack without these paths" helper and called for `render`, name, path grammar and the holder lookup — inferred, the module does not exist yet
- **Also likely:** `crates/govern/aep-domain/src/project.rs`, read only: `PROJECT_VERSION_V5` (:56), a `PROJECT_VERSION_V6` added by the overlay story, `PROJECT_FILE` (:43), `GIT_PLANNING_DIRECTORY` (:58), `GIT_EVIDENCE_DIRECTORY` (:60) — inferred
- **Also likely:** `crates/govern/aep-domain/src/artifact.rs`, read only: the closed-set accessor that `story:lifecycle-declares-closed-states` adds next to `ArtifactLifecycle::is_terminal` (:2298) — inferred
- **Shared files:** `CHANGELOG.md`, `## [Unreleased]`: adds `### Added` (only `### Fixed` exists today), one line for `pack` and `unpack` — cited
- **Shared files:** `website/docs/reference/cli.md`, section `## Plan: store, browser, workspace`: two rows after `aep plan store migrate git` (:98) — cited
- **Shared files:** `crates/edge/aep-cli/src/planning.rs`: not changed. Pack and unpack read through the overlay and add no planning verb, and `evidence <packed-id>` is the overlay story's — inferred
- **Shared files:** `crates/edge/aep-cli/src/planning/git_record.rs`: not changed. Its `git` (:19) is private and `committed_versions`/`changed_evidence` are `pub(super)` to `planning`, so the HEAD check belongs in `store_command`. The history walk across a pack commit is `story:validate-checks-packs` — inferred
- **Documents:** `website/docs/reference/cli.md` and `CHANGELOG.md` only; store documentation is `story:pack-end-to-end-and-docs` — cited
- **Confidence:** medium — the verb group, the dispatch, the reference test and the shared rows are read from the tree, but the module this story builds on is unwritten and so is the closed-set API, so the new file names are guesses
- **Would collide with:** any unit touching the `aep plan store` command group (`store_command.rs` `StoreCommand`/`run`, `store_command/`), the `## Plan: store` table of `website/docs/reference/cli.md`, `CHANGELOG.md` `[Unreleased]`, and the `aep-backend-markdown` pack module if it is extended rather than only called
- **Safety fact:** `pack` removes only loose files whose bytes Git has committed unchanged at `HEAD` (untracked or differing candidates are named and kept), so every byte it removes is held both by the new pack and by Git. Design *The packed representation* item 3 and the story's dry-run acceptance — level 1, stated, unproven
