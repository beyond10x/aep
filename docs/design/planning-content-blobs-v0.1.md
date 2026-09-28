# Planning content blobs v0.1

Superseded for the planning store by git-native-planning-store-v0.1.md (2026-09-28).

Status: implemented. `aep.project/4` and `aep plan store migrate content` ship in the release that
carries this page; `aep.project/3` stores read and write exactly as before. Recorded as Atlas ADR
0065. It builds on [`planning-on-entity-runtime-v0.1.md`](planning-on-entity-runtime-v0.1.md),
whose tree store it keeps.

## 1. Defect

An `aep.project/3` store records one artifact body many times per write, and every write adds
those copies to Git history. The copies come from two layers stacked on each other.

| layer | source | copies of one body |
|---|---|---|
| AEP document | `aep-backend-eventlog/src/lib.rs:1443-1446` (0.60.1) puts the contract instance's `fields` and the decision's `events` into `document` | `document.fields.body` |
| AEP events | `aep-backend-entity/src/lib.rs:215` makes the command payload an event's `args`; `:245-257` its `changed` | `events[].args.changes.body`, `events[].changed.body` |
| AEP typed fields | `aep-backend-eventlog/src/typed.rs:125` copies each content key into a typed field, `:133` puts `document` beside it | `body` |
| ER decision | `entity-core/src/runtime.rs:768-790` (create) and `:1406-1418` (execute), Entity Runtime 0.24.2, record the instance as the command's `fields`/`arguments`, as `result` and as `changed` | ×3 |
| ER commit | `entity-store/src/lib.rs:283-288` keeps the resulting `instance` beside the envelope | ×1 |
| ER batch | `entity-eventlog/src/adapter.rs:1607` writes every member record into the `er.batch/1` blob (`entity-store/src/asynchronous/encoding.rs:302-323`); `:1612-1613` writes the same record again as its `er.record/1` blob; `:1616` the request as `er.request/1` | ×2, plus the request |

Four copies per AEP instance, four instances per ER record and a record twice per batch: one body is
written a few dozen times by one edit, and a status move or a piece of evidence repeats it again,
because the instance it records still carries it. The applied-command record
(`aep-backend-entity/src/lib.rs:648`) adds one more per command through its `intent`.

The migration into `aep.project/2` also left provenance behind: the raw capture of the legacy
Markdown store and the exact bytes of each legacy journal line, both hex-encoded inside import
anchors (`planning-raw-capture-v0.1.md`). The capture holds the journal file whole, so each line is
kept twice, and hex doubles both.

## 2. Decision

`aep.project/4` is `aep.project/3` with one change below the planning contract: **every large value
is stored once, as a content-addressed blob, and named by digest everywhere Entity Runtime would
have repeated it.** AEP makes the substitution at its boundary with Entity Runtime, so ER, Eventlog
and their formats are unchanged (§ 5).

| reference | stands for | the blob holds |
|---|---|---|
| `aep-blob:text:sha256:<hex>` | a string of 256 bytes or more | its UTF-8 bytes |
| `aep-blob:hex:sha256:<hex>` | a canonical `hex:` byte string of that size | the decoded bytes |
| `aep-blob:lines:sha256:<hex>` | such a byte string of JSON lines | the JSON list of its lines' digests |
| `aep-blob:json:sha256:<hex>` | the value of a top-level `document` field | its JSON, with its own references |

- The digest is SHA-256 over the blob's bytes; the file is `blobs/<hex[..2]>/<hex>` beside the
  authority (`store.eventlog.blobs`, default `blobs`). Two branches that store one value write one
  identical file, so blobs never conflict in a merge.
- A string that already begins with `aep-blob:` is always stored, whatever its size, so a reader
  never has to guess whether a prefixed string is a reference.
- A `hex:` string is stored as the bytes it encodes when re-encoding them gives the same string;
  any other spelling is stored as text. A byte string of JSON lines — at least two, each `{…}`,
  the last one ended — is stored line by line, so a captured journal names each line by the blob
  that line's legacy record already has.
- Blobs are written before the Entity Runtime batch that names them. A batch that is refused
  leaves blobs nothing names, which is harmless: they are content, not history, and the next write
  of the same value reuses them.

The substitution sits in one place, `AuthoritySession`, the provider every tree read and write
already goes through (`aep-backend-eventlog/src/lib.rs`): a write stores its fields, arguments,
observation records and anchors; a read resolves the terminal instance, the anchor, and every
recorded entry before anything above sees them. A record's stored bytes are left as stored — they
are what the provider hashed — and nothing above the provider parses them.

### The import anchor

The raw capture is not replaced by a digest of the Markdown it captured, because that Markdown no
longer exists anywhere a reader can reach: it is the legacy store as it was before the
`aep.project/2` migration, in a format and layout the projection no longer writes, and `history`
reads the oldest events from it. It must stay in the store. What changes is how: each captured
file is now one blob named by its digest, stored as bytes rather than hex, and the journal file is
a list of its lines' digests, each of which is also the blob of the legacy record that line is.

## 3. Compatibility

- **`aep.project/3` is untouched.** A session opened without a blob directory stores and resolves
  nothing; every read and write is the code path it was.
- **Readers refuse what they cannot read.** A build that predates `aep.project/4` refuses the
  selector before opening a store (`project.version`), so an old `aep` cannot read references as
  bodies. The selector refuses `store.eventlog.blobs` under any earlier version.
- **What reads above the provider is identical.** Artifacts, statuses, relations, evidence,
  reviews, `history`, `explain` and the rendered projection are computed from resolved values.
  `the_sixteen_suites_pass_over_a_tree_store_that_keeps_content_blobs` runs every conformance
  suite over an `aep.project/4` store.
- **A missing or altered blob is refused, never read as a reference.** Every read re-hashes the
  blob it loads; `validate` reports a file under a digest it does not hash to (C1), and checks the
  blob directory for home paths (S9) as it checks the authority.
- **V2 across the migration.** `validate --against <rev>` skips the immutability rule when `<rev>`
  selects an earlier version: the migration writes every authority file anew by design.

## 4. Migration

`aep plan store migrate content --engineering .engineering` turns an `aep.project/3` store into an
`aep.project/4` one. Nothing is decided again by AEP:

1. It captures the source and refuses a store holding a fork or a merge decision; resolve those
   first (`aep plan artifact resolve`).
2. It writes a new tree beside the source (`<path>.aep-project-4`) under the source's store and
   stream identities, so every recorded coordinate that names the authority still names it.
3. It imports every anchor with its large values stored, then replays every recorded batch in the
   order the source committed it — same batch key, recording, command and expected revision.
   Entity Runtime decides each replayed command again.
4. **Verify.** With every reference resolved, the new store must hold the same subjects, anchors
   and terminal instances as the source and, record by record, the same entries, receipts,
   positions and expectations; and every artifact must read identically through the planning
   contract. Any difference is printed and nothing is replaced.
5. Only then does it move the source aside, put the new tree in its place, set the selector to
   `aep.project/4` and remove the old tree (`--keep-source` keeps it). Git history holds the old
   store.

The writer fence is held throughout. A refused migration leaves the project as it was.

## 5. What this does not change, and what is left

| left | where | why it is not here |
|---|---|---|
| each ER record embeds the entity definition it was decided under | `entity-core/src/runtime.rs:769`, `:974`, `:1407` | an ER record format change (`er.record/2`) |
| a batch blob embeds every member record, and each record is also its own blob | `entity-eventlog/src/adapter.rs:1607`, `:1612` | an ER batch format change (`er.batch/2`) |
| the instance appears four times in one record | `entity-core/src/runtime.rs`, `entity-store/src/lib.rs:283-288` | the kernel's verifiable decision record |

All three now repeat references, not bodies. Each is a coordinated migration of Entity Runtime's
own formats and is left to a follow-up there.
