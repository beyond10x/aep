---
format: aep.planning-md/3
id: story:pack-format-and-overlay-read
kind: story
status: active
title: Every reader answers over packed records as over loose files
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/doctor.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/specification.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command/migrate_git.rs
- confidence: cited
  path: crates/edge/aep-cli/src/workspace.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_overlay.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_overlay_adversary.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_overlay_adversary_2.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_security.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_security_adversary.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/store_pack_security_adversary_2.rs
- confidence: cited
  path: crates/govern/aep-domain/src/project.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/assembly.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/journal.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/lib.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/pack.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/provider.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/store.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/tests/pack.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/tests/pack_adversary.rs
- confidence: cited
  path: website/docs/reference/project-file.md
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T10:17:36Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-10T10:17:36Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":4}}}
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

Every verb treats a packed record as the same record it was when loose. Checked by (the fixture
holding a pack is `version: aep.project/6`, and the move into the pack is committed before any
`validate` runs, so this story does not depend on `story:validate-checks-packs`):

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
- a test of the overlay's holder lookup: for a path, it lists every place that holds it (the loose
  file and each pack), with the bytes each holds; a fixture with two packs (as two branches that
  packed different artifacts leave behind) reads both;
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
  commit: `story:validate-checks-packs`. This story owns the holder lookup both
  `story:pack-and-unpack-verbs` and `story:validate-checks-packs` use; they call it and do not add
  their own.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/plan/aep-backend-markdown` (the new pack module, the overlay, and every Git-native reader and writer) — cited
- **Files:** `crates/plan/aep-backend-markdown/src/store.rs`: `MarkdownStore::load` (:89, its `report.files_read += 1` at :109), `collect_documents` (:421), `MarkdownStore::create` (:216, `path.exists()`), `MarkdownStore::update` (:231, `!path.is_file()` gives `StoreError::Missing`) — cited
- **Files:** `crates/plan/aep-backend-markdown/src/provider.rs`: `PlanStore::kinds` (:144, `read_dir`), `MarkdownProvider::read` (:309), `StateProvider::ids` (:705, `read_dir`); the packed-target refusal goes in `Store::commit` (:743, the `existing.is_some()` branch that calls `update`), and `AtomicBatchStore::commit_batch` (:776) reaches it through `recover_pending`/`apply_recoverable` — cited for the readers, inferred for where the refusal goes
- **Files:** `crates/plan/aep-backend-markdown/src/journal.rs`: `read_git` (:423), `history_git` (:451, `document.is_file()` at :455 chooses between one artifact's directory and the whole store), `evidence_on_hand_git` (:483), `evidence_records_git` (:508, `is_file` at :516), `redaction_records_git` (:559), `write_evidence_occurrence` (:620, the sequence and "already written" checks via `files_with` at :658 and :672, `path.exists()` at :687), private `document_entries` (:719), `evidence_in` (:733), `subdirectories` (:756), `files_with` (:761), `listed` (:768) — cited
- **Files:** `crates/edge/aep-cli/src/planning.rs`: `evidence_problems` (:6736, its own `children` `read_dir` walk, called from `git_findings` :6448) and the `records` closure in `redact_literal` (:3927, `read_dir` on `evidence/<ns>/<name>`) — cited
- **Files:** `crates/govern/aep-domain/src/project.rs`: `ProjectVersion` (:100, gains `V6`), `ProjectVersion::as_str` (:110), `PROJECT_VERSION_V5` (:56, a sibling `PROJECT_VERSION_V6` is added), the version branch of `impl TryFrom<RawProjectConfig> for ProjectConfig` (:1175–1190, whose `else` is the `unsupported_protocol_version` refusal for `/7`), and the tests in the same file (:1368ff) — cited
- **Symbols:** new module `crates/plan/aep-backend-markdown/src/pack.rs` (parse, render, digest, path grammar, the holder lookup) and its `pub mod` line in `crates/plan/aep-backend-markdown/src/lib.rs` (:90–100) — inferred, the file name is a guess
- **Also likely:** `crates/edge/aep-cli/src/planning.rs` `create_through_a_command` (:2175): the overlay makes a packed id hydrate, so `new` fails on the contract's "already addresses an entity", and the mapping at :2213 prints "already exists at <path>" without naming the pack; the story's *Shared surfaces* line does not list this symbol — inferred
- **Also likely:** `crates/edge/aep-cli/src/doctor.rs` `evidence_files` (:543), a separate `read_dir` count of evidence `.json` files that `doctor` reports for a `Plan::Git` (:502) — inferred, a store reader the design's table leaves out
- **Also likely:** `crates/edge/aep-cli/src/specification.rs` `specifications` (:239), which reads `<planning>/specification/*.md` straight from disk with `read_dir`; a closed specification that gets packed would disappear from it — inferred, a store reader the design's table leaves out
- **Also likely:** `crates/edge/aep-cli/src/store_command/migrate_git.rs` `selected_layout` (:146): it accepts only `/5` (:161) and refuses `/6` ("this build migrates /1 to /5"); on `/5` its repair rewrites documents with `fs::write` (:605), which bypasses the provider's refusal — inferred
- **Documents:** `CHANGELOG.md` (`## [Unreleased]` / `### Added`, one line) and `website/docs/reference/project-file.md` (the `version` row at :32 and the example at :15) — cited
- **Not touched, by reading:** `projection.rs` `documents_of` (:512) reads only through `kinds`/`ids`/`load` and counts `files_read` per instance, so it follows the provider; `assembly.rs`, `aep-driver/src/run.rs` (`PlanSource for MarkdownStore`), `DrivenPlan::load`, `open_plan` and `graph_at` reach the store only through `MarkdownStore::load`; `Opened::judged_evidence`/`log`/`history`/`entries_from_the_contract` call `journal.rs` only; `planning/git_record.rs` (`committed_versions`, `first_committed`, `changed_evidence`) is the history walk, which the story puts out of scope; `schemas/generated/project.schema.json` types `version` as a free string (:166), so it does not need regenerating — cited
- **Confidence:** high for the backend files, `planning.rs` and `project.rs` (named by the story or the design and confirmed by grep); medium for doctor, specification, migrate_git and `create_through_a_command` (found by grep and not named by either)
- **Would collide with:** any unit on `aep-backend-markdown`'s `store.rs`, `provider.rs` or `journal.rs`; on `aep-cli` `planning.rs` `evidence_problems`, `redact_literal` or `create_through_a_command` (`fn lifecycle` at :7082 is not touched); on `aep-domain` `project.rs` version parsing; on the `CHANGELOG.md` `[Unreleased]` section; on `website/docs/reference/project-file.md`
- **Safety fact:** besides `git_record.rs`'s Git-history reads, every read of a Git-native store's `planning/` and `evidence/` goes through `MarkdownStore::load`/`collect_documents`, the provider's `read`/`ids`/`kinds`, the `journal.rs` readers, or four direct walks in `aep-cli` (`evidence_problems`, the redact `records` closure, doctor's `evidence_files`, `specifications`), so moving those onto the overlay covers every verb. Also, `evidence <packed-id>` writes no document, because `Store::commit` returns before `update` when the document did not change (provider.rs:766–768) — step 2 (`git grep` for `read_dir|is_file|exists|read_to_string` over the backend and `aep-cli/src`), unproven
- **Coordinator's decision on the open points (2026-11-10):** doctor's `evidence_files` and `specifications` move onto the overlay in this story; `aep plan store migrate git` refuses an `aep.project/6` store by name, telling the caller to run `aep plan store unpack --all` first, so its repair never writes a loose copy of a packed artifact; the packs directory is derived as `<root>/../packs` beside `evidence` (no new field); `files_read` counts packed records as files read so `validate`'s count is unchanged by packing
