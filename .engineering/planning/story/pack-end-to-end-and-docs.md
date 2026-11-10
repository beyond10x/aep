---
format: aep.planning-md/3
id: story:pack-end-to-end-and-docs
kind: story
status: implemented
title: A 1000-story store packs and unpacks with every answer unchanged, and the docs say how
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
- depends_on: story:pack-and-unpack-verbs
- depends_on: story:validate-checks-packs
scope:
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_end_to_end.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_end_to_end_adversary.rs
- confidence: cited
  path: crates/govern/aep-domain/src/store_catalog.rs
- confidence: cited
  path: docs/design/git-native-planning-store-v0.1.md
- confidence: cited
  path: website/docs/concepts/planning-store.md
- confidence: cited
  path: website/docs/reference/planning-stores.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T10:17:38Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-10T10:17:38Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-10T19:36:22Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":5,"verification":1}}}
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

A reader of the store's documentation can pack and unpack a 1000-story store as written, and only
the tracked file count changes. Checked by (the generated store holds implemented, rejected and
archived stories, archived review-results with findings blocks, and open stories relating to closed
ones):

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

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** the store documentation: `docs/design/git-native-planning-store-v0.1.md`, `website/docs/concepts/planning-store.md`, `website/docs/reference/planning-stores.md` — cited, the story's Acceptance names all three
- **Files:** `docs/design/git-native-planning-store-v0.1.md`, a new section after § 14 "Not in this design" (:380, the file is 384 lines) — inferred placement, the file is cited
- **Files:** `website/docs/concepts/planning-store.md`, the "Older stores" version table (:115–:122) gains an `aep.project/6` row, plus a pack/unpack section near "What `validate` checks" (:94) — inferred placement, the file is cited
- **Files:** `website/docs/reference/planning-stores.md`. Its tables (:13–:30) sit inside a generated region rendered from `aep_domain::store_catalog::CATALOG` by `xtask/src/planning_stores.rs`, and `status-check` fails a hand edit there. The `/6` text goes either in the hand-written prose after the region ends (:30–:34) or into the catalog followed by `cargo xtask status` — cited (generator and region markers read); the choice is inferred
- **Files:** a new integration test target under `crates/edge/aep-cli/tests/`, for example `crates/edge/aep-cli/tests/store_pack_end_to_end.rs` — inferred. No large-store test, bench target or `benches/` directory exists anywhere in the repository, and xtask has no bench command. `story:repository-gate-carries-a-scale-lane` (draft, not built) is the only planned scale lane. Use a separate file: `story:pack-and-unpack-verbs` will add its own CLI tests in that directory, and a separate file keeps the two disjoint
- **Fixture pattern:** each test file carries its own copies of `repository()`, `relative()`, `project(name)`, `run_as` and `ok`, and `aep-cli` has no shared `tests/common`. `project` makes `CARGO_TARGET_TMPDIR/<prefix>-<name>` (removed first if present), runs `git init`, and writes `project.yaml` as `aep.project/5` with `store: {git: {}}` and `protocols:` as a relative path to the repository root, so the live `artifacts/lifecycles/*.yaml` (the ones that gain `closed:`) is the protocol tree. Commits go through `run_git` with `-c user.name/user.email`, then `add -A` and `commit` — cited, `crates/edge/aep-cli/tests/git_store.rs:10-58`, `crates/edge/aep-cli/tests/store_migrate_git.rs:37-83`
- **Fixture pattern:** hand-written store files are copied from a committed fixture tree in `a_store_written_by_0_64_0_validates_…` (`git_store.rs:731-776`, `copy_tree` at :778, source `crates/plan/aep-backend-markdown/tests/fixtures/written-by-0.64.0`). Committed review-results with findings are built by `committed_review_with` (`git_store.rs:~800-880`) — cited
- **Generation route:** write one template of each shape through the CLI (an implemented story with evidence, a rejected story, an archived story, an archived review-result with a findings block, an open story relating to closed ones). Clone each by renaming to reach 1000 or more, then make one `git add -A` / `commit`. Running `new`/`move`/`evidence` once per story would mean about 4000 or more debug-binary spawns — inferred
- **Gate time:** the gate runs it in `task test` = `cargo test --workspace` (`Taskfile.yml:213-216`), inside `task check`. CI's gate job has `timeout-minutes: 45` (`.github/workflows/ci.yml:32`). The repository sets no per-test budget, and no `#[ignore]` slow lane exists (the only `#[ignore]`s are child-process entry points in `crates/edge/aep-project/tests/`). The whole workspace suite was recorded at 204 s (`verification-report:ess-conformance-coverage-integrated`) — cited
- **Gate time:** target under 30 s in a debug build. Measured with release aep 0.79.0 on this repository's own store (450 planning files, 422 evidence files): `list --status implemented` 0.04 s, `graph` 0.03 s, `validate` 0.21 s. The test makes 18 read calls plus `pack`, `unpack` and three commits of about 3000 files — inferred, the debug multiplier was not measured
- **Also likely:** `crates/govern/aep-domain/src/store_catalog.rs:69-79`, the Git-native entry's `project_version` or `note`, only if `/6` goes into the generated table — inferred
- **Documents:** the three pages above. `website/docs/reference/cli.md` and `CHANGELOG.md` are not in scope: the story's Context and Out of scope give them to the stories that add each change. The same goes for `website/docs/reference/project-file.md`, which `story:pack-format-and-overlay-read` claims for `aep.project/6` — cited
- **Confidence:** high for the documents, because the story names every page and string. Medium for the test file, because the story asks for "an end-to-end test" without naming a file, and no existing large-store test exists to add it to
- **Would collide with:** any unit editing the git-native design doc or the website store pages (concepts/planning-store, reference/planning-stores). If the catalog route is taken: any unit touching `aep_domain::store_catalog` or running `cargo xtask status`. A new file under `crates/edge/aep-cli/tests/` collides only with a unit that adds a file of the same name
- **Safety fact:** `validate`'s Git history read is one `git log … -- <every path>` for all paths (`crates/edge/aep-cli/src/planning/git_record.rs:55-80`), not one process per file. A 1000-story/2000-evidence store therefore costs a few Git spawns per `validate`, and the argv is about 150 KB, under Linux `ARG_MAX` — step 2, unproven
