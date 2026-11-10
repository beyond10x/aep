---
format: aep.planning-md/3
id: story:validate-checks-packs
kind: story
status: active
title: validate holds every pack to the bytes it replaced
relations:
- decomposes: epic:store-pack-closed-artifacts
- informed_by: architecture-design:store-pack
- serves: vision:O2
- depends_on: story:pack-format-and-overlay-read
- depends_on: story:lifecycle-declares-closed-states
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning/git_record.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/validate_packs.rs
- confidence: inferred
  path: crates/govern/aep-domain/src/error.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T10:17:37Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-11-10T10:17:38Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":5}}}
---
# Story: `validate` holds every pack to the bytes it replaced

## Outcome

`aep plan artifact validate` checks every pack: its format and name, each record's digest, path
safety, that no path is held twice with different bytes, that each packed record equals the last
committed blob at its original path, that a packed artifact is at a closed status of its kind and
packed evidence belongs to the artifact its directory names, that immutability and redaction replay
hold for packed review-results and evidence, that no copy re-exposes a redacted literal, and that the
store version agrees with whether packs exist.

## Context

`architecture-design:store-pack`, section *What `validate` checks about packs*, items 1–8. Git
history reading is `crates/edge/aep-cli/src/planning/git_record.rs` `committed_versions`, whose walk
today ends at a deletion; for a packed path it continues past the commit that moved the file into a
pack. Closed statuses come from `story:lifecycle-declares-closed-states`. Fixtures write their packs
with the pack module of `story:pack-format-and-overlay-read` (its `render`), not with the `pack`
verb, so this story does not wait for `story:pack-and-unpack-verbs`.

## Acceptance

`validate` answers `valid` on a fixture Git store whose closed artifacts and evidence were moved into
a committed pack unchanged, and refuses each corruption below with its own stable code, exit 1,
naming the pack file and the record path where one exists. Checked by CLI tests, one per case:

- a pack that does not parse (bad header, truncated record), and a pack whose file name is not its
  content digest: refused (format);
- a record byte changed in a pack: refused (digest);
- a record changed and its header digest recomputed, the pack renamed to match: refused (provenance:
  differs from the last committed blob at the record's original path before the commit that added
  the pack);
- record paths `planning/../x.md`, `/etc/x`, `evidence/a\b/c.json`: refused (path);
- the same path loose and packed with different bytes, and in two packs with different bytes:
  refused, naming both places; identical bytes in both places: admitted;
- a packed artifact whose status is not closed for its kind, and a packed evidence record whose
  `artifact` is not the one its directory names: refused;
- a pack in an `aep.project/5` store, and an `aep.project/6` store with no pack: refused (version;
  this case names `project.yaml`, there is no record path);
- an evidence file that had two different committed versions while loose (the first committed
  version differs from the last) and was then packed: refused (immutability), as it is today while
  loose — the history walk crosses the pack commit;
- a pack written and the loose files removed but not yet committed: `validate` does not report the
  packed evidence files as rewritten (`git_record::changed_evidence` reads a deletion whose bytes a
  pack holds as a move, not a change);
- a packed review-result with a redaction record: `valid` (replay over the first committed version
  reproduces the packed text); the same store with a packed or loose copy that still contains the
  redacted literal: refused, and the output does not contain the literal.

## Shared surfaces

- `CHANGELOG.md` (`## [Unreleased]` / `### Added`, one line for the pack checks).

## Out of scope

- Writing packs: `story:pack-and-unpack-verbs`.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/edge/aep-cli/src/planning/git_record.rs` and the Git-native check block of `crates/edge/aep-cli/src/planning.rs` — cited, the story's Context names `git_record.rs` and the design names `validate` as the reader of both
- **Files:** `crates/edge/aep-cli/src/planning/git_record.rs` — cited
- **Files:** `crates/edge/aep-cli/src/planning.rs` — cited, the `validate` path is in this file (lines below)
- **Files:** `CHANGELOG.md`, `## [Unreleased]` / `### Added` (a new subsection; today only `### Fixed` exists), one line — cited, the story's Shared surfaces
- **Symbols, git_record.rs:** `committed_versions` (:55); its deletion branch at :99-103 ends the walk, and it must continue when a pack holds the same bytes — cited
- **Symbols, git_record.rs:** `changed_evidence` (:185, `diff HEAD --diff-filter=MTD` at :192-199) must stop reporting a deletion whose bytes a pack holds — cited
- **Symbols, git_record.rs:** a new helper for "the last committed blob at a path before the commit that added the pack, or `HEAD` when the pack is not committed", for the provenance check (design item 5) — inferred
- **Symbols, git_record.rs:** `first_committed` (:229) probably stays as it is. It walks `--diff-filter=A` and keeps the first add it meets (:279), so deleting a file into a pack leaves its dating unchanged — inferred, walked step by step and not run
- **Symbols, planning.rs:** entry `validate` (:6075) → `findings` (:6136) → `git_findings` (:6395, call at :6150). The pack checks, the history read crossing a pack (:6431) and the `changed_evidence` mapping (:6449-6458) go in `git_findings` or a new sibling it calls — cited for the site, inferred for the new function
- **Symbols, planning.rs:** `review_problems` (:6559) does the redaction replay: first Git-native version, then `redaction_records_git` filtered by revision, then `record_redaction::replace_digest` (:6604-6639). The baseline must come from across a pack commit, and the check that no copy re-exposes a redacted literal (design item 7) goes beside it — cited for the site, inferred for where item 7 lands
- **Symbols, planning.rs:** how problems print. `Summary.problems: Vec<String>` (:9095); `print_validation` (:6888) prints `N problem(s):` and then one `  - {problem}` line each; yaml/json go through `crate::print_serialised` (:6088). No change to the printer is needed — cited
- **Symbols, planning.rs:** the version agreement check (item 8) needs the `project.yaml` version, but `Plan::Git { root, evidence }` (:302) does not carry it — inferred, placement open
- **Also likely:** `crates/govern/aep-domain/src/error.rs`, new variants in the `validation_codes!` invocation (:208), which generates `ValidationCode` (:188) and `ALL`. Today no Git-native `validate` problem carries a code (`git_findings` pushes plain strings). The only coded lines come from `ValidationErrors` in `graph_problems` (:6101), rendered `[code] path: message` — inferred
- **Also likely:** `crates/edge/aep-cli/tests/validate_packs.rs`, a new CLI test file, one test per acceptance case, with fixtures written by the pack module's `render` — inferred name. The existing Git-validate tests are in `crates/edge/aep-cli/tests/git_store.rs`, e.g. :548 — cited
- **Reads, does not change:** the pack module and the holder lookup from `story:pack-format-and-overlay-read` (`parse`, `render`, digest, path grammar); `aep_backend_markdown::journal::redaction_records_git` (journal.rs:559, moved onto the overlay by that story); `record_redaction::replace_digest` (record_redaction.rs:107) — cited from the stories' Out of scope and Context
- **Documents:** `CHANGELOG.md` — cited. `website/docs/reference/cli.md:89` (the `validate` row lists what it checks) — inferred, the story does not name it
- **Confidence:** medium — `git_record.rs` and the `validate` path in `planning.rs` are cited down to the line. Where the stable codes are declared, the test file's name and the pack module's API are inferred, because the module is not written yet.
- **Would collide with:** any unit touching the Git-native `validate` block of `planning.rs` (`findings`/`git_findings`/`review_problems`/`evidence_problems`, :6136-6800), including `story:pack-format-and-overlay-read`'s change to `evidence_problems`; `planning/git_record.rs`; the `validation_codes!` list in `aep-domain` `error.rs`; `CHANGELOG.md` `[Unreleased]` / `### Added`; `website/docs/reference/cli.md` — cited for the first two, inferred for the rest
- **Safety fact:** `committed_versions` and `changed_evidence` each have one caller (planning.rs:6431, :6450), both inside `git_findings`. So changing what they mean reaches only `validate` and `aep doctor`, which accumulates through the same `findings` via `store_findings` (:6871-6881), so doctor's planning-store line gains the pack checks too — step 2 (`git grep -n 'committed_versions\|changed_evidence'`), unproven
