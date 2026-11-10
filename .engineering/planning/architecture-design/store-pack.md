---
format: aep.planning-md/3
id: architecture-design:store-pack
kind: architecture-design
status: draft
title: 'Packing closed artifacts: aep.pack/1, one read overlay, unpack, validate'
relations:
- designs: epic:store-pack-closed-artifacts
revision: 1
---
# Design: packing closed artifacts out of the working tree

Status of this text: the design for `epic:store-pack-closed-artifacts`, written 2026-11-10 by the
coordinating session. The store-version half is decided in
`architecture-decision-record:packed-store-is-aep-project-6`.

## Problem, measured

A Git-native store (`aep.project/5`) keeps one file per artifact and one file per evidence record,
forever. Retiring work (`move --to archived`) keeps the file. In `ai-agent-platform/ai-agent-platform`
(operator's measurement, aep 0.79.0, not re-verified here) planning plus evidence are 4188 of 5860
tracked files (71 %): 1347 stories (605 implemented, 302 archived, 5 rejected), 460 review-results
(217 archived), 2305 evidence files in 714 directories.

Reproduced on a fixture store (aep 0.79.0, script `repro.sh` in the session scratch, output quoted):

```text
tracked .engineering/planning files: 21
tracked .engineering/evidence files: 20
--- aep plan store --help (verbs)
Commands:
  migrate  Move the store to another layout
--- aep plan artifact --help verbs matching pack|prune|compact|archive
(none)
--- aep plan store pack
error: unrecognized subcommand 'pack'
--- move story:closed-01 --to archived
story:closed-01 moved implemented -> archived (revision 5)
file still present: yes
tracked + modified planning files after archive: 21
```

## How the store is read today (cited)

Every verb reads loose files through a handful of functions; nothing else in the store is read.

| reader | file:symbol | verbs that answer through it |
|---|---|---|
| every document, sorted | `crates/plan/aep-backend-markdown/src/store.rs` `MarkdownStore::load`, `collect_documents` | `list`, `graph`, `waves`, `blocked`, `board`, `show`, `findings`, `review-value`, `validate`, `relate` target checks |
| one document, ids of a kind, kinds | `crates/plan/aep-backend-markdown/src/provider.rs` `MarkdownProvider::read`, `StateProvider::ids`, `kinds` | every write (`new`, `move`, `relate`, `body`, `set`, `scope`, `redact`) through `MarkdownBackend` |
| transitions plus evidence files | `crates/plan/aep-backend-markdown/src/journal.rs` `read_git`, `history_git`, `evidence_on_hand_git`, `evidence_records_git`, `redaction_records_git`, `files_with`, `subdirectories` | `history`, `explain`, `move` evidence gates, `spend`, `evidence` file naming (`write_evidence_occurrence`) |
| evidence files as records | `crates/edge/aep-cli/src/planning.rs` `evidence_problems` | `validate` |
| Git history of a path | `crates/edge/aep-cli/src/planning/git_record.rs` `committed_versions` (the walk stops at a deletion and at the add), `cat_file_batch` | `validate` immutability of evidence and review-results, redaction replay |
| evidence directory listing | `crates/edge/aep-cli/src/planning.rs` `redact` (`records` closure) | `redact` |
| the store version | `crates/govern/aep-domain/src/project.rs` `ProjectVersion`, `PROJECT_VERSION_V5`, `GitStore` (no fields) | every verb |

`aep plan store migrate git` on a `/5` store only drops duplicate transitions (operator's request);
it moves no file.

## The packed representation

1. **A pack is one text file** at `.engineering/packs/<name>.pack`, outside `planning/` (no
   digest-named file under `planning/`, git-native design N4). Format `aep.pack/1`:

   ```text
   aep.pack/1
   --- evidence/story/x/20261110T000000Z-0-3f2a….json 412 sha256:<64 hex>
   <exactly 412 bytes: the original file>
   --- planning/story/x.md 1873 sha256:<64 hex>
   <exactly 1873 bytes: the original file>
   ```

   One header line, then per record a `--- <path> <byte length> sha256:<hex>` line, the original
   file's bytes verbatim, and one `\n`. The path is relative to `.engineering`. Records are sorted by
   path bytes; a path appears once per pack. The bytes are the original files', so a reviewer reads a
   packed story as the story and a packed evidence record as its JSON.
2. **The name is content-addressed**: the first 16 hex of the SHA-256 of the whole pack's bytes. A
   pack is never edited in place; a pack whose record set changes is written under its new name and
   the old file removed in the same command.
3. **What is packed**: an artifact whose status is *closed* for its kind (below), together with every
   evidence file under `evidence/<namespace>/<name>/` (observations, redaction records,
   review outcomes recorded against it). Archived review-results are closed by the same rule
   (`archived` is terminal in `artifacts/lifecycles/review-result.yaml`). Only bytes Git has
   committed are packed: a candidate whose file is untracked or differs from `HEAD` is not packed and
   is named in the output. Outside a Git work tree `pack` refuses: Git is the record of what moved.
4. **Closed is declared by the lifecycle document, never by a status name in Rust.** A lifecycle
   gains an optional `closed: [<state>…]` list; a terminal state (no successor) is always closed;
   a kind with no lifecycle (`approval-record`) has no closed state and is never packed. The
   protocol tree's `story`, `epic`, `task`, `initiative`, `vision`, `design` and `specification`
   ladders declare `closed: [implemented, rejected]` where those states exist, so with this tree a
   story is closed at `implemented`, `rejected` or `archived`. A store pinned to an older protocol
   tree (no `closed:` key) packs terminal states only (`archived`); that is stated in the dry run.
   The domain type gains the field and `cargo xtask schema` regenerates
   `schemas/generated/artifact-lifecycle.schema.json`; `aep plan artifact lifecycle <kind>` prints
   the closed set.

## Where reads go: one overlay

`aep-backend-markdown` gains one read abstraction, the **store files**: the loose files under
`planning/` and `evidence/` plus every record of every pack, listed and read as one tree. Every reader
in the table above takes it instead of walking directories itself (`collect_documents`,
`files_with`, `subdirectories`, `MarkdownProvider::read`/`ids`/`kinds`, `evidence_problems`, the
redact listing). Because a packed record's bytes are the original file's bytes, each verb parses the
same documents and the same evidence and prints the same output. Per verb:

- `list`, `graph`, `waves`, `blocked`, `board`, `show`: `MarkdownStore::load` over the overlay; the
  location printed for a packed artifact is its original relative path (unchanged output).
- `history`, `explain`, `move` gates, `spend`: `journal.rs` readers over the overlay; transitions are
  in the artifact bytes, evidence records are overlay files.
- `findings`, `review-value`: review-results and `review_outcome` evidence come from the overlay.
- `validate`: the same document checks over the overlay, plus the pack checks below.
- `redact` replay: redaction records are evidence files (overlay); the first committed version comes
  from Git history, whose walk continues across a pack commit (below).
- relation resolution: a packed target resolves, so an open artifact that relates to a packed one
  stays valid and `relate <open> depends_on <packed>` works (it writes only the open artifact).

## Writes, and unpacking

- `new <kind> <name>` refuses an id that is packed, naming the pack (no duplicate id).
- A write that rewrites an artifact file (`move`, `body`, `set`, `scope`, `relate`/`unrelate` with
  the packed artifact as source, `redact`) refuses a packed artifact by name:
  `<id> is packed in .engineering/packs/<name>.pack; run aep plan store unpack <id> first`. One write
  still changes one file (N1).
- `evidence <packed-id>` is admitted: it adds one new loose evidence file and touches no pack; the
  next `pack` folds it in. File naming (`write_evidence_occurrence`) sees packed records, so a record
  already packed is answered, not written twice.
- **`aep plan store unpack <id>… | --all [--dry-run]`** writes the artifact file and its packed
  evidence files back to their original paths, byte for byte, and rewrites each pack that held them
  without them (new name, old file removed; a pack left empty is removed). When a loose file already
  holds that path, unpack keeps the loose file and only removes the packed copy. When no pack
  remains, `project.yaml` returns to `aep.project/5`.
- **`aep plan store pack [--dry-run]`** writes one new pack with every eligible record, removes the
  loose files it packed, and sets `project.yaml` to `aep.project/6`. A record already held
  byte-identically by a pack is not packed twice; its loose copy is removed. `--dry-run` writes
  nothing and prints, per kind and status, how many artifacts and evidence records would be packed,
  the pack name, the candidates not packed and why, and the file count under `.engineering/planning`,
  `.engineering/evidence` and `.engineering/packs` before and after.

## Determinism

No clock and no randomness: the pack's bytes are a function of the set of (path, bytes) it holds,
sorted by path; its name is a function of its bytes; dry-run output is sorted. Packing the same
committed store twice, on two machines, writes the same file. `unpack` writes the original bytes.

## Git merge behaviour

- Two branches pack different artifacts: two different pack files, two disjoint sets of deleted loose
  files. Git merges cleanly; the overlay reads both packs.
- Two branches pack the same artifact: both delete the same loose file (clean); the same path is held
  by two packs with identical bytes, which `validate` admits.
- One branch packs X, the other changes loose X: Git reports a modify/delete conflict on X's file, as
  it reports any concurrent change today (R4 holds).
- Two branches unpack from the same pack: each deletes the pack and adds a different rewrite. The
  merge is clean and may leave a path both loose and packed, or packed twice with different bytes.
  `validate` refuses a path held in two places with **different** bytes, naming both places;
  `aep plan store unpack <id>` resolves it (loose copy kept, packed copies removed).

## What `validate` checks about packs

1. Each pack parses as `aep.pack/1`: header, record lines, exact lengths, sorted unique paths, the
   name equal to its content digest.
2. Each record's SHA-256 equals its header's digest.
3. Each path is a safe relative path in the store's own grammar: `planning/<kind>/<name>.md` or
   `evidence/<kind>/<name>/<file>.json`; no `..`, no absolute path, no backslash, no empty or dot
   segment, no NUL or control character. `unpack` writes only paths that pass the same check, inside
   `.engineering`, and never follows a symbolic link.
4. A path held in two places (loose and packed, or two packs) is refused unless the bytes are
   identical.
5. Provenance against Git: each packed record equals the last committed blob at its original path
   before the commit that added its pack (`HEAD` when the pack is not committed). The history walk in
   `git_record.rs` treats "deleted while a pack holds the same bytes" as a continuation, not the end
   of the file's incarnation, so immutability of review-results and evidence and the redaction replay
   (first committed version, then every redaction record) hold for packed records as for loose ones.
   Outside a work tree the check is skipped, as the other history checks are.
6. A packed artifact passes every document check a loose one does, and its status is closed for its
   kind; packed evidence belongs to the artifact whose directory names it.
7. A redacted literal is never re-exposed: no copy of an artifact (loose or any pack) may contain a
   window matching one of its redaction records' digest and length, which the replay already
   computes.
8. Version agreement: a store holding a pack is `aep.project/6`; a `/5` store with a pack, or a `/6`
   store with none, is refused naming the mismatch.

## Compatibility

A packed store is `aep.project/6` (decision record). aep 0.79.0, given `version: aep.project/6`,
refuses every verb by name, measured 2026-11-10 in the session scratch:

```text
[unsupported_protocol_version] project.version: this build reads `aep.project/5`, not `aep.project/6` (hint: upgrade the tooling rather than reinterpreting the document)
```

A store that never packs stays `aep.project/5`, byte for byte; nothing changes for it. Readers outside
this repository that read `.engineering/planning/` files directly (conductor's `collect/repositories.rs`
reads `version` and story files; metaharness's drive) see `/6` and are not changed here: noted for
their owners, not edited.

## Not in this design

- Deleting any record: a pack holds every byte it removed from the working tree, and Git holds the
  rest.
- Packing AEP's own store.
- Treating an `active` review-result whose subjects are all closed as closed: that is
  `story:review-results-stay-active-after-their-subject-closes` (draft).
- The skill text (`aep:planning`, `references/store-conventions.md`): it lives in the sibling
  `agentplugins` repository and is not edited here.
