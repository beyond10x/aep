# Planning artifacts as Entity Runtime entities on a mergeable Eventlog v0.1

Status: accepted, 2026-09-24 (#17), with the implementation plan approved the same day. It
supersedes `eventlog-planning-authority-v0.1.md` for the planning store. Released for it:
Eventlog 0.4.0 and Entity Runtime 0.21.0; recorded as Atlas ADR 0063. It serves the ESS evolution plan:

- ESS lowers to Entity Runtime (ER) definitions;
- ER decides and records every change;
- Eventlog persists it (`ESS-EVOLUTION.md` steps 3–5 and line 41).

It needed `ESS-EVOLUTION.md` revision 2, which amends step 4 (`:33`), step 5 (`:38`) and the
interfaces (`:49`, `:53`), and Atlas ADR 0063 (§ 13 unit P1). Both exist.

Sources:

- The storage survey of 2026-09-24 ("survey"), which read `origin/main` of AEP `d85adc86`, ER `bf825f0`, Eventlog `04ae526` and Gates `9da4707`.
- The design review of 2026-09-24 ("review", H1–H7, M1–M19, L1–L7), which added EKR `d865a82`, Atlas `77e45d4`, ESS `01dcc4c6`, and a Git lab running real `merge` and `rebase` over the § 3.2 layout.

**Inferred** marks a claim not read from code, a command or the lab.

## 0. Decision in one table

| layer | today | after |
|---|---|---|
| planning document | one ER entity type `aep.entity/1`: a single opaque JSON field `document`, one state `recorded`, one operation `replace` (`aep-backend-eventlog/src/lib.rs:2080-2116`) | **one real ER entity type per artifact kind** (`aep.story/1`, `aep.epic/1`, …) with typed fields, the real lifecycle and one operation per AEP command |
| a change | `replace` of the whole document, after AEP's kernel decided (`aep-backend-entity/src/kernel.rs`) | an **ER command** on that entity. ER validates it and records the decision, **including refusals** (E-R6) |
| lifecycle definition | built per command in `kernel.rs`, then thrown away | registered in the store as ER definitions; later lowered from ESS |
| ER → Eventlog | the decision sits in 4 blobs per member, and the event holds only `{"blob": digest}` (survey § 1.2) | the typed ER record **is** the event data, with state stored once (E-R1) |
| repo-local store | `eventlog-file`: one hash chain per store; it cannot merge in Git | **`eventlog-tree`**, a sibling file-store crate in the eventlog repository: one file per event and one per atomic group, with no store-wide mutable file. It merges in Git |
| read cost | every call re-hashes the whole log; every new process verifies from scratch | an unchanged file is never hashed again, across processes (Part B) |
| Postgres | not wired | later, through the same ER and Eventlog APIs: `eventlog copy` linearizes the history (§ 9.6) |

## 1. Defects

| id | defect | cause | evidence |
|---|---|---|---|
| D1 | planning documents are not structured objects in the store | a one-field, one-state ER definition; the Eventlog event holds a blob digest | `aep-backend-eventlog/src/lib.rs:2080-2116`; `entity-eventlog/src/adapter.rs:1284-1430` |
| D2 | a branch that writes planning cannot merge | `eventlog-file` frames carry `sequence` and `previous`; `decode_chain` refuses a fork; there is no merge driver, and GitHub ignores drivers anyway | `eventlog-file/src/journal.rs:969-984`; `eventlog/docs/design/file-provider.md:33` |
| D2 | the same, from store-wide files | `manifest.json`, `events.jsonl` and `.aep-projection-ownership.json` are rewritten on every write | survey § 1.3 |
| D3 | every read costs the whole log | one transaction per call; each resume re-reads and hashes the whole prefix; ER captures the whole tenant per read | `eventlog-file/src/lib.rs:142, :948`; `journal.rs:529-570`; `adapter.rs:901, :2106` |
| D4 | 6.7 frames per command | 1 batch blob + 3 blobs per member + 1 group | `adapter.rs:1361-1390` |
| D5 | refused commands leave no ER record | `RecordedEntry` is `Decision \| Observation`; `Executor::decide` returns `Err` before any append | `entity-store/src/asynchronous/types.rs:104-125`; `entity-executor/src/lib.rs:295-410` |

## 2. Requirements

| id | requirement | check |
|---|---|---|
| R1 | each artifact kind is an ER entity type with typed fields and its lifecycle; ER refuses an invalid field or an illegal move | per-kind definition fixtures |
| R2 | every AEP command is **received, decided and recorded** by ER, whether accepted, no-op or refused | ER suite "a refused command is recorded once and replays under its key"; AEP `audit`, `rejected_audit` |
| R3 | two branches writing different artifacts merge with GitHub's button | merge matrix (§ 11) |
| R4 | two branches writing the same artifact merge textually, a fork is **detected by `validate --strict` rule S3** whether or not Git conflicts, and `aep plan artifact resolve` joins it through ER | merge matrix, including the rows Git merges clean |
| R5 | no store-wide file changes on a write | `eventlog verify` rule V1 |
| R6 | AEP's 16 suites, ER's recorded-execution suites and Eventlog's conformance pass over `eventlog-tree`; Eventlog's conformance runs filtered by the provider's declared capability profile | the three suites |
| R7 | measured in a **fresh process**: `list` < 1 s and a single-artifact write < 2 s at 1,000 artifacts | bench (§ 10) |
| R8 | reading an unchanged store hashes 0 bytes, including in a fresh process; a changed file is hashed once | Part B § B6 |
| R9 | `validate --strict` is a required check from cutover, and `main` accepts no update that bypasses it | § 8 |
| R10 | each store is exported once, recovery appends included, and passes a history-fidelity check | § 9 |
| R11 | no committed byte carries an absolute home path, by the same test Gates applies | S9 |
| R12 | moving to SQLite or Postgres needs no schema change; a forked history is linearized with its origin recorded | round-trip suite |

## 3. Directory and file structure

### 3.1 Today (ESS, `origin/main` `f928c781`, survey § 1.3)

```
.engineering/
  project.yaml                          aep.project/2
  planning/                             tracked Markdown projection
    <kind>/<name>.md                    469 files
    .aep-projection-ownership.json      STORE-WIDE, rewritten on every write (71,900 B)
    journal.jsonl                       legacy Markdown-store journal, retained
  state/                                eventlog-file authority
    manifest.json                       STORE-WIDE, rewritten on every write
    events.jsonl                        STORE-WIDE hash chain, 240 frames, 18,628,219 B
    blobs/…                             8,239 blobs, 156,708,370 B
    writer.lock
```

### 3.2 After (every repository with an `aep.project/3` store)

```
.engineering/
  project.yaml                          aep.project/3
  planning/                             tracked Markdown projection, derived
    <kind>/<name>.md                    render(state); carries `format: aep.planning-md/2`, `revision`, `head`
  state/                                eventlog-tree authority
    store.json                          {format: "eventlog-tree/1", identity: <store id>}; written once at creation
    tenants/<tenant>/
      streams/
        er.definition/<entity type>/<digest>.json          ER definition registrations
        er.subject/<entity type>/<name>/<digest>.json
                                        one ER decision or observation per file, immutable
        er.refusal/<kk>/<key-digest>/00000001-<digest:16>.json
                                        one ER refusal record per refused command (E-R6), immutable
      groups/<kk>/<key-digest>.json     one per atomic group (= one command), immutable; COMMIT POINT
      blobs/<kk>/<digest>               content-addressed, immutable; used only above the size bound (E-R1)
    .cache/                             untracked: state/<stamp-digest>.{sqlite,json}, the kept replay
                                        (§ 6); ignored by its own .cache/.gitignore, and V1 skips it
    .lock                               untracked: one writer per checkout, skipped by V1 (review N15)
```

Verified properties from the review lab:

- Event files are named by digest (§ 6, implementation decisions), so no filename carries an ordering.
- Canonical JSON has no trailing newline and survives CRLF conversion.
- A torn event file commits fine in Git. That is why V3 is a required check.

| property | today | after |
|---|---|---|
| files changed per command | 3 store-wide + 1 `.md` + ~5 blobs | 1 new event file per touched entity + 1 new group file + 1 rewritten `.md` per touched entity |
| store-wide files | 3, all rewritten | 1 (`store.json`), written once |
| files ever rewritten | 4 | `.md` only, plus event files under § 6.5 redaction |
| Git conflict, different artifacts | always | never |
| Git conflict, same artifact | always | maybe in that `.md`; event files never conflict. **The fork is detected by S3, not by Git** (review M7) |
| entity content | in blobs | typed JSON in the event file, state once |
| size per command | ≈6.7 frames + blobs; 37.4 MB scanned per ESS PR | the event file ≈ the artifact's state (body included) plus ≈1 KB metadata, plus a ≈1 KB group file. **Inferred**; measured at the first cutover |

`journal.jsonl` and `.aep-projection-ownership.json` are removed at cutover. The export carries
their history, and Git keeps the old bytes.

## 4. AEP planning on ER

### 4.1 Entity types

There is one ER entity type per artifact kind: `aep.<kind>/1`.

| definition part | source |
|---|---|
| fields | the kind's `aep-domain` artifact schema, as ER `FieldKind`s. `body` is `string` with `max_length` 1,048,576 bytes (**inferred** bound; ER has only `max_length`, `definition.rs:503-507`) |
| identity | natural key `name`, unique within the type |
| states and transitions | the kind's ladder from the lifecycle YAML, the input `kernel.rs` reads today |
| evidence | designed: an ER **observation** at the entity's current revision, as today (`aep-backend-entity/src/lib.rs:1116`; `ESS-EVOLUTION.md:30`), so it does not advance the revision. The shell derives the per-kind counts from the observations and passes them as `$args.evidence.<kind>`, as today (`kernel.rs:165-180`). A concurrent `move` and evidence record therefore do not conflict (review N7). **Built so far: an `edit` of the typed entity**, which advances its revision, so a concurrent `move` and evidence record on two branches fork the artifact. Moving evidence to observations is open work (§ 13, A2 follow-up) |
| relations | ER `RelationDefinition` + `Ref` fields. Existence is not checked by the kernel (`definition.rs:521-529`); see § 4.2 step 2 |

**Operations: all 11 `aep_domain::command::Command` variants** (`aep-domain/src/command.rs:438-458`).

| AEP command | ER |
|---|---|
| create | `create` |
| edit fields and body | `edit` |
| move to status | `move_to_<status>`, one per target status |
| relate, unrelate | `relate`, `unrelate` |
| archive, supersede | `archive`, `supersede` |
| `RecordEvidence` | an ER `Observation` carrying the evidence reference and its kind; not a state change |
| `SubmitDesignReview` | operation `submit_review` |
| `ApproveDesign` | operation `approve` |
| `AcceptAdr` | operation `accept` |

Unit A1 enumerates the 11 variants against the table. A variant that fits no row blocks A1.

**Definition source:**

1. **Now:** generated by AEP from the lifecycle YAML plus the domain schema (unit A1, extending `kernel.rs`).
2. **Once ESS phase C is released** (`land/ess-evolution-phase-c-20260923`, unmerged): AEP's planning model is written as an ESS `system.yaml` and lowered by `ess-entity-runtime`. A test asserts the result equals the generated definitions (unit A9), and the generator is then retired.

Definitions are registered on `er.definition/<type>`. A lifecycle change registers a new version.

If two branches both change a lifecycle YAML, their definition streams fork. `resolve` does not
re-decide those, because a definition stream holds no commands. It regenerates the definition from
the merged YAML (S7's input) and registers it with `Merge` over the heads (review N10).

### 4.2 Command flow

1. AEP parses the CLI input into one ER command per touched entity, forming one atomic batch, which is one group.
2. **Cross-entity rules** are the one documented exception to "ER decides". They cover relation targets that must exist, required relations, and ladder rules that read other entities. The ER kernel is handed one instance (`entity-core/src/runtime.rs:561-575`), so AEP's shell evaluates these rules on the current fold, as `kernel.rs` does today. It passes the observations as decision arguments, and ER records them with the decision. S6 re-checks them at the merged head (§ 8).
3. ER validates the fields and evaluates the transition and its predicates: `Permitted`, `NotOnTheLadder` or `Unobservable`.
4. ER records the outcome.

**Only mutation verbs submit commands** (review N3). The read-only paths evaluate the kernel without
the recording path: `describe`, `validate`, the driver's legality reads, and every dry run.
Otherwise refusal files would accumulate from reads.

| outcome | recorded (E-R1, E-R6) |
|---|---|
| accepted | one decision per touched entity on `er.subject/<type>/<name>`: the command as received, arguments, verdict, resulting state (once), new revision |
| accepted, nothing changed (no-op) | an accepted zero-event decision, as ER records today (`ESS-EVOLUTION.md:30`) |
| refused: kernel refusal, a failed precondition, an unobservable precondition, an invalid transition, an unknown operation, validation, or an expectation conflict | one `RecordedRefusal` on `er.refusal/<kk>/<key-digest>`; entity streams untouched |
| replay: same key, same request | nothing new is written; the group answers after checking its members (§ 6.2) |
| same key, changed request | `IdempotencyConflict`, not recorded; the caller re-issues under a new key |
| malformed request (`InvalidInput`, `DuplicateRecordId`) | not recorded; these are not decisions |

**Decision paths.**

- `MemoryBackend` stays as the in-memory contract model for tests.
- `kernel.rs::decide` stays, because the 26 `aep.project/1` repositories (Markdown, SQLite, Postgres backends) still decide through it (review M17).
- Both paths coexist until the `/1` migration design.

### 4.2a How unit A2 maps AEP's store onto typed entities (implementation decision, 2026-09-24)

AEP's contract logic (`aep-backend-memory` behind `EntityBackend`) stays as it is. The mapping lives
at the one place AEP's writes reach Entity Runtime: `aep-backend-eventlog`'s `append_planning_batch`,
`registry` and `unpack`.

| AEP write (a `PlanningCommit` on `aep.entity`) | Entity Runtime action on subject `(<kind>, <id>)` |
|---|---|
| creation | `create` with the typed fields |
| a change to title, summary, owner, tags, refs, scope, body or other keys | `edit` with every typed field, plus `document` |
| a change of status | the ladder's operation named for the target status |
| both in one command | `edit` then the move, in one batch |
| an observation (evidence, a relation's source) | a `RecordedObservation`, which advances no revision |
| relation, audit and applied-command records | unchanged: the one-field `document` types, until each has its own entity type |

Details:

- The **kind** comes from the metadata's `entity_type` (`aep.<kind>/v1`).
- The **id** stays AEP's entity id.
- A read of `aep.entity` resolves the kind by id from the retained capture.
- Each typed entity also keeps a `document` field holding AEP's own packed metadata and events, as
  today. `unpack` and `decision_document` therefore read history and audit unchanged, while Entity
  Runtime validates the typed fields and the ladder on every write.
- **Evidence preconditions** stay in AEP's kernel decision (§ 4.2 step 2): `append_planning_batch`
  sees the decided result, not the counts.
  - The stored definition therefore uses the ladder's transitions without its evidence rules.
  - Passing the counts through `PlanningCommit` is follow-up work, not part of A2.
- **Typed mode** is selected by `aep.project/3`. `/2` stores keep the `document` form, which is
  what the export reads.

### 4.3 Identity

Every persisted identifier is caller-supplied or derived. The memory backend's counters (`01MEM…`,
`rel-`, `aud-`, `evt-`, `seq-`; `aep-backend-memory/src/store.rs:103-141`) are never written.

| identifier | form |
|---|---|
| entity | `{type, name}` |
| ER record id | `<command_id>/<type>/<name>` for decisions; `<command_id>/refused` for refusals |
| event id | `<command_id>/<n>` |
| audit id | the record id |

ER's global record-id conflict check runs at open over the derived ids.

## 5. ER changes (entity-runtime repository)

| id | change | today | after |
|---|---|---|---|
| E-R1 | record as event data, state once | `er.recorded_entry {"blob": <wrapper digest>}`; `RecordedCommit` holds `instance` and `record.result`, validated equal (`entity-store/src/lib.rs:283-320`) | the event data is the canonical record. The resulting state is stored once; the validator derives the other copy. Blobs are used only for a record above 256 KiB (**inferred** bound) |
| E-R2 | per-entity reads | `capture_tenant` per read (`adapter.rs:901, :2106`) | a command reads only its entities' streams. The `stream_identity` check (`adapter.rs:905-908`) is answered from `store.json` |
| E-R3 | derived record ids | global index | § 4.3; the conflict check runs at open |
| E-R4 | inline projections not persisted on `eventlog-tree` | `er_binding_v1`, `er_records_v1`, `er_batches_v1`, `er_subjects_v1` in-store (`projection.rs:15-20`) | rebuilt at open into `.cache/`, **consuming positions in § 6.3 order**; SQLite and Postgres as today |
| E-R5 | fork-aware expectations | `Expect` = `Absent \| Revision(u64)`, `Copy` (`entity-store/src/lib.rs:70-76`) | adds `Merge(HeadSetDigest)`: a fixed 32-byte digest of the sorted head digests, so `Expect` stays `Copy` and its 110 by-value sites are unchanged (review N4). The head list travels on the batch request. A merge decision has every head as a parent. `recover_from` keys heads by digest |
| E-R6 | refusal records | none | `RecordedEntry::Refusal(RecordedRefusal)` + `RecordKind::Refusal`; both enums `#[non_exhaustive]` |

**E-R6 shape:**

```
RecordedRefusal {
  members: Vec<{                 // one per batch member; a relate names two (review N9)
    entity, id,                  // the subject named; may not exist for a refused create
    expected: Expect,            // what the command claimed
    command: DecisionCommand,    // as received; request bytes bound as today
    refusal: Option<Kernel { outcome, error, message } | Conflict { found: Option<u64> }>,
                                 // None for a member that was not itself refused
    legal: Vec<String>,          // legal moves, when the ladder names them
  }>,
  envelope: { record_id, recorded_at, correlation, causation, actor },
}
```

One file per key, `er.refusal/<kk>/<key-digest>/00000001-<digest>.json`.

- A refusal is committed by the same batch path under the same key, on `er.refusal/…`. It is **never** written on `er.subject/…`: the model takes the last subject record as head (`adapter.rs:1310`).
- `RecordedRefusal::validate` refuses any `changed`, `events` or `result` content.
- A provider `Expected` conflict is retried into the executor pre-check and recorded there.

**Consumer impact** (review census):

| consumer | change |
|---|---|
| ER | ≈12 exhaustive matches |
| AEP | ≈5 matches |
| service-sdk phase D | 1 arm, `service-eventlog/src/v4.rs:183-184` |
| connectors phase E | 2 arms, `metadata/er.rs:2725, :3877` |
| the ER ↔ Eventlog envelope | version bump |

`#[non_exhaustive]` makes this the last forced round of matches.

## 6. `eventlog-tree` (eventlog repository)

This is a sibling crate `crates/eventlog-tree`. It is not a second format inside
`eventlog-file/src/lib.rs`, which is 2,350 lines of manifest, sequence and prefix-hash authority
(review Q2). The separate crate keeps `verify_once_review*.rs` and the six non-planning consumers of
`eventlog-file` untouched.

**Implementation decisions (V2, 2026-09-24):**

- **Engine.** Every read and projection is served by an in-memory `eventlog-sqlite` store. The tree's files are replayed into it at open, in § 6.3 order. This follows eventlog's invariant 2, "in-memory remains SQLite `:memory:`", and reuses a provider that already passes the shared conformance exercises.
  - The tree layer adds only persistence, forks and the refusal of claims.
  - `eventlog-sqlite` gains `restore_events`, `restored_pending` and `restore_stream_identity`, so a replay keeps original event ids, instants and tenant identities.
  - While a replay is queued, `eventlog-sqlite` does not check an append's expectation, because it was checked when the event was first written.
- **Versions are replay order.** A stream's version is its event's place in the § 6.3 order. So versions stay gapless like every other provider's, and a later copy to SQLite or Postgres keeps them.
  - A merge can renumber the side that sorts second, so **no file stores its version**.
  - Event files are named `<digest>.json`, not `<version>-<digest>.json`.
  - This removes most of review H2: § 9.6's `copy` needs no renumbering, only the `origin` record of each event's parents.
- **No shared-module extraction (B0 dropped).** The tree crate keeps its own writer lock (`std::fs::File::lock` on `.lock`) and its own content-addressed blob files. It shares no code with `eventlog-file`, so V2 and B1–B4 no longer touch the same files.
- **Stream directories.** Every identifier becomes one path segment: bytes outside `[A-Za-z0-9._-]` become `%XX`, and a leading dot is escaped.
- **Commands.** Each group file records what the engine was asked to expect, per member. A replay passes the same expectation again, so a retry still matches the group's request fingerprint.
- **Cross-process writes.** Under the writer lock, a writer compares the set of group files with the one its state was built from. If another writer added one, it rebuilds the state before deciding. Reads serve the state as of open or of the last write.

**Implementation decisions (read cost, 2026-09-24).** Measured on an export of this repository's own store (313 artifacts, 9,939 files, 57 MB of blobs), release build, no SHA-NI:

| command | `aep.project/2` file store | tree, before | tree, after |
|---|---|---|---|
| `plan artifact list`, fresh process | 3.8 s | 5.1 s | 1.19 s (3.0 s on the first open after a change) |
| `plan artifact new` | 23.0 s | — | 8.2 s |
| `plan artifact validate --strict` | 12.6 s | 17.6 s | not re-measured |

- **The fold is keyed by stamps, all or nothing.** `.cache/state/<key>.{sqlite,json}` holds the engine image (`SqliteEventStore::image` / `from_image`, rusqlite `serialize`) and the tree's heads, records and group-file set. `<key>` digests every history file's `{dev, ino, len, mtime_ns, ctime_ns}`, the registered projector names and the running executable's stamp. A projector has no version, so a rebuilt program replays once rather than trust a projection an older one wrote. There is no `verified.json`: an open whose listing and stamps all match loads the fold, and any other open replays and verifies everything. The per-file incremental verification of § B3 rule 2 for trees (review N19) is not built.
- **Rule 4 as written.** No fold is kept while any file's `ctime` is within 2 s of the open, and none is kept if the listing changed while the replay ran. `.cache/.gitignore` is `*`; `verify` and `validate` S5/S9 skip `.cache/`.
- **One replay per open.** `TreeEventStore::open_with_inline` registers ER's projector before the replay; `open` followed by `attach_inline_existing` replayed twice.
- **An in-memory engine does not re-hash its own blobs.** `eventlog-sqlite` re-checked every blob's SHA-256 integrity column on each read; for a `:memory:` connection it now checks the metadata only, because its rows change only through its own statements. File databases are unchanged (mutation-tested: two file-database tests fail without the check). A blob group's fingerprint and its integrity column share one hash.
- **Not done: the write path.** Of `new`'s 8.2 s, 36 % is `authority_snapshot_identity`, which serializes and hashes the whole store on every publish. Its value is persisted by the `/2` projection metadata, so replacing it is a coordinated change and waits for the cutover.

### 6.1 Event file

Canonical JSON: sorted keys, no whitespace, integers only, absent fields omitted.

| field | meaning |
|---|---|
| `stream`, `version`, `event_id`, `name`, `schema_version` | as today |
| `parents` | digests of this stream's previous event(s): none for the first, one normally, n for a merge |
| `meta` | `CommandMeta` (`eventlog-core/src/lib.rs:304-327`) |
| `recorded_at`, `group` | caller clock (a tie-break only, § 6.3); the group's key digest |
| `data_digest`, `data` | event data and its digest |
| `digest` | covers everything except `data` and `digest`, so redaction keeps the chain |

`version = 1 + max(version of parents)`. The group file holds `idempotency_key`, `request_hash`,
and each member's path and digest.

### 6.2 Write protocol

1. `flock` on `.lock`.
2. If the group file exists, **check that every member exists and matches its listed digest**. If one does not, refuse `Corrupt` (review M18). Otherwise answer: a replay if `request_hash` matches, `IdempotencyConflict` if not. Stop.
3. Check each member's expectation against its stream heads. A forked stream accepts only `Merge { heads }` naming every head.
4. Write each event file: temp file, `fsync`, `rename`.
5. Write the group file: temp file, `fsync`, `rename`, then `fsync` the directory. **The group rename is the commit point.**
6. Release the lock. No state is kept under `.cache/` while any file is less than 2 s old (Part B § B3 rule 4, § 6).

- **Torn write:** an event file whose group file does not exist. Reads ignore it, and `eventlog repair` deletes it; `repair` is the only file deletion.
- **Git does not stop a torn file** (review L6). The required check V3 does.

### 6.3 Open, read and positions

- **Open:** follows Part B § B3.
  - Files whose stamps match are trusted without hashing.
  - Other committed event files are verified: digest, `data_digest`, parents present, version rule.
  - Each stream's head set is built.
  - Projections are rebuilt, or loaded from `.cache/state/`.
- **Positions** come from a topological order of the group DAG, not from clocks (review H1):
  - a group follows every group that holds a parent of one of its members;
  - within a group, members keep their index;
  - ties are broken by the earliest `recorded_at` among the group's members, then by the key digest.
  - Positions are `1..n` in that order. They are process-local and a pure function of the tree, so two checkouts of one commit agree.

  Wall-clock skew between machines cannot reorder a child before its parent.
- `validate_captured_order` (`eventlog-core/src/capture.rs:544-568`) gets a rule variant for branchable stores: positions strictly increasing, and every parent precedes its child. The gapless per-stream version rule applies only to non-branchable providers.
- **Capability profile.** A new `eventlog-core` type the provider declares:
  - `read_feed` positions are process-local;
  - claims and durable cursors are refused with `EventLogError::Unsupported`;
  - forks are supported.

  `eventlog-conformance` filters its suites by the profile (review M11). ER uses none of the unsupported calls (review census: `stream_identity`, `put_blob`, `capture_tenant`, `append_group_guarded`, `attach_inline_existing`, `rebuild_inline_projection`, `get_blob`, `append_group_guarded_with_blobs`).

### 6.4 Forks

| operation on a stream with more than one head | result |
|---|---|
| `read_stream` | all events in `(version, digest)` order, plus `EventLogError::Forked { heads }` from `BranchableEventStore::heads` |
| `append` with `Exact` or `Absent` | refused: `Forked { heads }` |
| `append` with `Merge { heads }` naming every head | accepted; the event's `parents` are all the heads |

`eventlog-core` changes for this (review H5):

- `RecordedEvent` gains `digest: Option<…>` and `parents: Option<…>`, with `Default`.
- A constructor helper replaces the struct literals every provider uses (`eventlog-file/src/lib.rs:522-528`).
- `EventLogError` gains `Forked` and `Unsupported`, and becomes `#[non_exhaustive]`.
- The additive trait `BranchableEventStore` is implemented only by `eventlog-tree`.

### 6.5 Redaction

`redact` replaces `data` with `{"redacted": {"group": <key-digest>}}`. It keeps `data_digest` and
`digest`, and writes a redaction group. V2 accepts a changed event file only in that form.

### 6.6 `eventlog verify` (tree)

| rule | refuses |
|---|---|
| V1 | a tracked file outside the § 3.2 layout under `state/` |
| V2 | against a base commit: a deleted or changed event or group file, except a § 6.5 redaction; a changed `store.json` |
| V3 | a torn event file; a group whose member is missing or mismatched |
| V4 | a failed digest, `data_digest`, parent or version check |
| V5 | non-canonical bytes |

`verify` never reads `.cache/` and always hashes.

## 7. Merging and `aep plan artifact resolve`

| case (lab row) | Git | detected by |
|---|---|---|
| different artifacts (1) | clean | nothing to detect |
| same artifact, `.md` lines differ (2a) | event files clean; `.md` conflicts | S3 |
| same artifact, `.md` unchanged or merging textually (2b) | **clean**; silent fork | S3 |
| same name created twice (3a, 3b) | event files clean (two roots); `.md` add/add conflict, or clean when the renders are identical | S3 |
| same idempotency key, different command (4) | add/add conflict on the group file | Git; `resolve` refuses and one side re-issues under a new key |

Merge direction and rebase make no difference (lab: rebase gives the same fork and the same `.md`
conflict).

**`resolve`**, in the merging worktree, after `git merge <target>`. Git's index then holds only
`.md` entries unmerged; any other unmerged path under `.engineering/` is refused.

1. **Choose `first`** among the stream's heads:
   - `--first <digest>` if given;
   - otherwise the unique head whose event file exists in `--onto <ref>` (default `origin/main`, checked with `git ls-tree`). `resolve` fetches `<ref>`'s remote first, and it refuses when the fetch fails (review N18);
   - if none or more than one head is in `--onto`, refuse and name the heads. That covers a feature branch merged into a feature branch, and a fork already on `main` (review H4). The operator re-runs with `--first`.
2. The events to re-decide are those **reachable from the other heads and not from `first`**, in § 6.3 order (DAG ancestry, so a second `resolve` on top of a merge decision works; lab row 5).
   - Merge decisions are skipped; only ordinary decisions are re-evaluated (review N6).
   - Observations and refusals are carried over, not re-decided.
3. ER re-evaluates each of them against the definition and `first`'s state. A refused `create` on another head means that head's later commands apply to `first`'s entity, and that is stated in the output.
4. ER records one merge decision with `expect = Merge { heads: [all heads] }`, holding `kept`, `refused` (reason and legal moves) and the merged state.
5. `.md` is re-rendered and staged. `refused` is printed; follow-ups are ordinary commands.

No event file is deleted or rewritten.

## 8. `aep plan artifact validate --strict`

It runs `eventlog verify` V1–V5 against the merge base with the target, then:

| rule | refuses |
|---|---|
| S1 | a tracked file under `.engineering/planning/` that is not a projection of a stream |
| S2 | an `er.subject/aep.*` stream with no `.md`, or a `.md` whose stream is absent. `er.definition` and `er.refusal` streams have no `.md` (review N16) |
| S3 | a forked entity stream, i.e. an unresolved merge |
| S4 | a `refused` merge entry with no follow-up decision naming it, reported as a warning, never an error |
| S5 | an `.md` not byte-equal to `render_<format>(state)`, where `<format>` is the file's own `format:` tag. A renderer change bumps the format and ships a re-render step (review M9) |
| S6 | cross-entity rules at the merged head: targets exist, required relations present, ladder rules that read other entities hold |
| S7 | a lifecycle YAML or domain-schema change with no matching definition registration. The check compares against the **registered** `er.definition` version. It warns, but does not fail, when the running generator would produce a newer version, so a generator change does not turn every repository red (review N8) |
| S8 | a group whose `request_hash` does not match its recorded command |
| S9 | any Gates `personal-paths` finding in the planning tree. S9 runs the Gates scanner (`gates/src/scan.rs:268`: `/home/`, `/Users/`, `c:\users\`, `\\host\users\`) plus `/root/`, `~/` and `$HOME/` (review M19) |

**Required, with no bypass** (review H4):

- On every adopting repository, `validate --strict` is a required status check on `main`, and "require branches to be up to date" is on.
- The ruleset's bypass list is empty for it, and non-PR updates to `main` are refused.
- A repository pre-push hook runs `validate --strict` for any push that touches `.engineering/`. That covers the bot's direct pushes that `beyond10x/AGENTS.md` admits.
  - `aep plan store install-hooks` installs the hook. Gates only preserves the hook chain; it never runs repository tools (`gates/README.md:21, :31`; review N5).
  - The hook refuses when the installed `aep` is not the version that installed it, and when the pushed commit is not the clean checked-out tree, so the tree validated is the tree pushed.
- Release cuts in the adopting repositories (`CHANGELOG` commits) go through a PR as well (review N13).
- "Require up to date" gives "checked tree = merged tree" for PR merges (review Q4).

## 9. Export, cutover, rollout

### 9.1 Export (`aep plan store export`, once per store)

It reads through the path `history` uses today (`PlanBackend::entries_of`,
`aep-cli/src/planning.rs:615-700`; review Q5: achievable). It writes a fresh `eventlog-tree` store
using **ER's import vocabulary**, not invented decisions (review H7). `RecordedCommit::validate`
requires the full resulting state (`entity-store/src/lib.rs:283-347`), so:

| legacy item | written as |
|---|---|
| every legacy revision and every accepted legacy command, in both eras: the journal era (no body) and the `aep.project/2` era (whole document in the old shape) | ER imported evidence (`HistoryOrigin::Imported`, `LegacyEvidence`), with reserved record ids (`asynchronous.rs:603, :844, :897`). No old document is validated against the new typed definition (review N2) |
| legacy evidence records and reviews | imported evidence, as above |
| the artifact's current document, body included | one **anchor decision** carrying the full state; the only legacy state that must pass the new definition |
| each refused legacy command | a `RecordedRefusal` (E-R6) |
| definitions | registered first |

**Fixups.** An anchor state that ER refuses is repaired by a **recorded export-time fixup**. The
pattern is `.ess-evolution/waves/0019-store-recovery/fixups.sh`. The export does not fix it at the
source, because store writes are frozen.

- The script and its log are kept **outside the repository**, like the home-path map. The AEP recovery log carried 15 personal-path findings (review N22).
- The cutover record lists each fixup site by artifact and field.

### 9.2 Fidelity check (`export --verify`, required before a cutover PR)

- per artifact, `history` output equals the old output over evidence plus anchor, with legacy ids compared through the map, except at the recorded fixup sites (review N11);
- the rendered `.md` equals the old one, minus the new `format`, `revision` and `head` keys, except at the recorded fixup sites;
- relation sets are equal;
- artifact counts are equal;
- ER validation accepts every state after the recorded fixups;
- `b10x-gates check` over the produced tree finds nothing.

### 9.3 Home paths

| path | rewritten to |
|---|---|
| under the workspace root | `workspace:<repo>/<path>` |
| any other path that the S9 patterns match | `home-path:sha256:<digest>` |

The map is kept outside every repository.

### 9.4 Inputs

The requirement is on **store bytes, not Git branches** (review L1). The source's `events.jsonl`
must extend `main`'s `events.jsonl` as a byte prefix, and every `main` blob must be present.
Verified: eventlog `main`'s 3,162,501 B and ER `main`'s 3,714,084 B are prefixes of their recovery
branches' stores. ER's branch `3dd35b6` is 3 commits behind `main` in Git.

| repository | export source |
|---|---|
| eventlog | `plan/ess-evolution-store-recovery-20260923` @ `56be98a` |
| entity-runtime | `plan/ess-evolution-store-recovery-20260923` @ `3dd35b6` |
| aep | recovery archive `.ess-evolution/waves/0019-store-recovery/aep-recovery-engineering.tar`, unpacked outside the repository |
| ess, connectors, service-sdk | `main` |

The review found these other refs carrying `.engineering/state/`. Each cutover record labels each
one source, prefix or discarded with a reason, and an unlabelled ref blocks the cutover. ESS,
connectors and service-sdk are enumerated at their own cutovers; the machine has 40+ ESS worktrees.

| repository | refs |
|---|---|
| aep | `docs/planning-log-design`, `impl/ess-evolution-a1-store-write-cost`, `plan/ess-evolution-store-recovery-20260923`, `plan/planning-store-on-eventlog-authority-20260922` (and origin copies) |
| entity-runtime | `ci/shared-gates-pin-0-1-7-20260923`, `fix/track-the-store-writer-lock-20260922`, `impl/ess-evolution-r1-model-build-once`, `plan/ess-evolution-store-recovery-20260923`, `plan/planning-store-on-eventlog-authority-20260922`, `main` |
| eventlog | `ci/shared-gates-pin-0-1-7-20260923`, `fix/track-the-store-writer-lock-20260922`, `plan/ess-evolution-store-recovery-20260923`, `plan/planning-store-on-eventlog-authority-20260922`, `main` |

### 9.5 Selector and rollout

The selector is `aep.project/3` with
`store: { entity_runtime: { eventlog: { provider: tree, path: state } } }`. Later the provider
becomes `postgres` with `url_env`. Old readers refuse `/3` before opening a store, verified:
`project.rs:913-929` raises `UnsupportedProtocolVersion`, and the refusal accumulates before any
open. `aep.project/2` stays readable for export only.

The sequence below is relative to the in-flight ESS 0.31.0, service-sdk 0.6.0 and connectors
0.12.0 (review M15):

1. P1: `ESS-EVOLUTION.md` rev 2 and Atlas ADR 0063, the coordinated migration.
2. The in-flight releases (ESS 0.31.0, SDK 0.6.0, Connectors 0.12.0) proceed on ER 0.20.0 as planned. None of them writes a planning store.
3. Eventlog release: `eventlog-tree`, the core changes and Part B.
4. ER release: E-R1–E-R6. service-sdk and connectors add the E-R6 arm at their next re-pin.
5. AEP release. `cargo xtask deps` gains rule 3: AEP's Eventlog pin equals the Eventlog pin of the ER commit AEP pins (review M10).
6. Adopter pins bumped.
7. Cutover PR per repository, in the order eventlog, entity-runtime, aep, ess, connectors, service-sdk. Each PR contains:
   - the export;
   - `.engineering/state/` replaced;
   - `journal.jsonl` and the ownership file removed;
   - `/3` selected;
   - the required check and ruleset;
   - the Gates baseline move. Review L3 said the org-secret write needs a personal account; that is not tested. The bot tries first, and the operator does it only if GitHub refuses the App. The ruleset change is the bot's: its token reads `bypass_actors` as `[]`, which a token without administration write does not.
8. Freeze lifted for that repository. The queued writes in `FOLLOW-UP.md` are replayed as ordinary commands. For AEP they include the `test_result` naming its release tag that `cargo xtask release` requires, since the store is frozen when the release is cut.

The uncommitted Gates appended-tail scan is not needed by `eventlog-tree` stores. It stays useful
for the `eventlog-file` runtime stores and the 26 `/1` repositories.

### 9.6 SQLite and Postgres later

`eventlog copy --from <tree> --to <sql>` **linearizes** the history (review H2):

- Groups are appended in § 6.3 position order.
- Each stream is renumbered gaplessly.
- Each event's original `{version, digest, parents}` goes into an `origin` object **beside `meta`, not in `data`**, so redaction cannot erase it (review N14).

SQL stores hold gapless unique `(stream, version)` (`eventlog-core/src/lib.rs:530-533`). The
round-trip suite asserts equality modulo the origin map. Postgres is trunk-only and never forks.

## 10. Performance

All measured in a fresh process, release build, median of 5, `cargo xtask bench planning` over a
generated store.

| operation | target |
|---|---|
| `list` at 1,000 artifacts, unchanged store | < 1 s; 0 event files opened; 0 bytes hashed |
| single-artifact `move` | < 2 s; only written files hashed |
| `validate --strict` at 1,000 × 10 events | < 10 s (it always hashes) |
| a planning PR | measured at the first cutover (Gates scan units) |

## 11. Test obligations

| test | repository | asserts |
|---|---|---|
| Eventlog conformance over `eventlog-tree`, filtered by capability profile | eventlog | R6 |
| merge matrix with real `git`: every § 7 row, including 2b and 3b | eventlog, AEP (`resolve`) | R3, R4 |
| two-writer clock-skew fixture | eventlog | rebuild equals sequential replay (H1) |
| torn group, crash at each § 6.2 step, group with a missing member | eventlog | no change or a repairable tear; missing member refused |
| redaction | eventlog | V2 accepts only § 6.5 |
| ER recorded suites over `eventlog-tree`; record is event data with state once; "a refused command is recorded once and replays under its key" | entity-runtime | E-R1–E-R6 |
| AEP's 16 suites over ER on `eventlog-tree` and on SQLite | AEP | R6 |
| per kind: ER refuses an invalid field and an illegal move; all 11 commands exercised | AEP | R1 |
| round trip tree → SQLite → tree, including a resolved fork | AEP | R12 |
| export fidelity on copies of the six stores | AEP | R10 |
| Part B tests (§ B7) | eventlog | R8 |
| every guard mutation-tested | all | invariant 15 |

## 12. Not in this design

- a central deployment;
- branch overlays on Postgres;
- a merge driver;
- a replay-branch tool;
- deleting artifacts;
- migrating the 26 `aep.project/1` repositories;
- lowering from ESS before phase C is released;
- merging the eventlog and entity-runtime repositories (review Q9: keep separate; revisit after cutover only if they still release in lockstep).

## 13. Units

| unit | repository | ships | files | depends on | acceptance |
|---|---|---|---|---|---|
| P1 | workspace, atlas | `ESS-EVOLUTION.md` rev 2; Atlas ADR 0063 | `ESS-EVOLUTION.md`; `atlas/architecture/adr/0063-…` | — | accepted by the operator |
| B0 | eventlog | extract the lock and blob modules shared by `eventlog-file` and `eventlog-tree` | `crates/eventlog-file/src/` → a shared module (**inferred** location) | — | `eventlog-file` suites unchanged |
| B1 | eventlog | persisted stamps, format 1 (Part B § B3) | `crates/eventlog-file/src/journal.rs`, `lib.rs`, `cost.rs`, `docs/design/file-provider.md:59-75` | — | § B7 |
| B2 | eventlog | blob stamps | `crates/eventlog-file/src/lib.rs` (`blob :647`), `capture.rs` | B1 | § B7 |
| B3 | eventlog | `read_many` | `crates/eventlog-core/src/lib.rs` (`EventStore :698`), `eventlog-file/src/lib.rs` | B1 and V1 (both edit these files) | `read_many` equals the loop on every provider |
| B4 | eventlog | view retention after a read-only refusal | `crates/eventlog-file/src/lib.rs:142-175` | B3 | § B7 |
| V1 | eventlog | `RecordedEvent.{digest, parents}`, constructor helper, `Forked`, `Unsupported`, `#[non_exhaustive]`, `BranchableEventStore`, capability profile, capture-order variant | `crates/eventlog-core/src/{lib,capture}.rs`; the mechanical constructor change in every provider | — | every provider compiles after that change; SQL conformance vectors unchanged |
| V2 | eventlog | `eventlog-tree` crate (§ 6) | `crates/eventlog-tree/` (new); `eventlog-conformance` capability filter | V1, B0 | filtered conformance; merge matrix; torn, crash and skew cases |
| V3 | eventlog | `verify`, `repair`, `copy` (linearizing) | eventlog CLI (**inferred** location) | V2 | one failing fixture per V-rule; the copy round trip |
| V4 | eventlog | release | `CHANGELOG.md`, tag | B0–B4, V1–V3 | tag on `origin/main` |
| R1 | entity-runtime | E-R1; `entity-eventlog` feature `tree` and its `eventlog-tree` dependency (review N21) | `crates/entity-eventlog/src/{adapter,encoding}.rs` (`adapter.rs:1284-1430`), `crates/entity-eventlog/Cargo.toml`; `entity-store/src/lib.rs:283-320` | V4 | no wrapper blob; state stored once |
| R2 | entity-runtime | E-R2 | `adapter.rs:901, :905-908, :2106` | R1 | counting case: a command reads only its streams |
| R3 | entity-runtime | E-R3 | `crates/entity-store/` (**inferred**) | R1 | two-branch id collision refused |
| R4 | entity-runtime | E-R4 | `crates/entity-eventlog/src/projection.rs` | V2 | projections equal after rebuild in § 6.3 order |
| R5 | entity-runtime | E-R5 | `entity-store/src/lib.rs:71-76`, `recover_from`, `entity-eventlog` | R1 | an n-ary merge decision replays |
| R7 | entity-runtime | E-R6 | `entity-store/src/asynchronous/types.rs`, `entity-executor/src/lib.rs`, `entity-eventlog/src/{adapter,encoding,projection,facade}.rs` | R1 | suite "refused once, replays under its key" |
| R6 | entity-runtime | release | tag | R1–R5, R7 | tag on `origin/main` |
| A1 | aep | ER definition generator: fields, lifecycle, evidence object, all 11 operations | `crates/plan/aep-backend-entity/src/kernel.rs` → `definition.rs` | — | per-kind refusal fixtures; 11/11 commands mapped |
| A2 | aep | `aep-backend-eventlog` rewritten: commands → ER commands, cross-entity observations as arguments, E-R6 refusals; the post-cutover `history` reader over `er.subject/aep.*`, imported evidence and `er.refusal/…` (review N12) | `crates/plan/aep-backend-eventlog/`, `aep-cli/src/planning.rs` (`history`) | A1, R6 | 16 suites over ER on SQLite, then `eventlog-tree` |
| A3 | aep | `resolve` | `aep-backend-eventlog/src/resolve.rs`, `crates/edge/aep-cli/src/planning.rs` | A2, V2 | merge matrix |
| A4 | aep | `validate --strict` S1–S9 | `aep-cli/src/planning.rs:6301` | A2, V3 | one failing fixture per rule |
| A5 | aep | renderer `aep.planning-md/2`; ownership file removed | `aep-backend-eventlog/`, `aep-planning-migration/src/projection.rs:22, :1862` | A2 | render fixtures; no store-wide file |
| A6 | aep | `aep.project/3`; `xtask deps` rule 3 | `crates/govern/aep-domain/src/project.rs`, `aep-cli/src/planning.rs:381-410`, `xtask/src/main.rs:712-800` | A2 | old-reader fixture; alias equivalence; rule 3 mutation-tested |
| A7 | aep | export, `--verify`, home-path map, fixup hook | `aep-cli/src/store_command.rs`, reusing `planning.rs:615-700` | A2, A6 | fidelity on copies of the six stores |
| A8 | aep + six repositories | `aep plan store install-hooks` (pre-push `validate --strict`, pinned-version check); cutover PRs (§ 9.5 step 7) | `aep-cli/src/store_command.rs`; per repository `.engineering/`, `project.yaml`, CI, ruleset, hook | V4, R6, AEP release | `validate --strict` required, green, no bypass |
| A9 | ess, aep | AEP planning model as ESS spec, lowered | ess + aep | ESS phase C released | lowered definitions equal A1's |

Parallelism:

- P1, B0, V1 and A1 start together.
- B0 → B1 → B2, and B1 + V1 → B3 → B4: one sequence over `eventlog-file/src/lib.rs` and `eventlog-core/src/lib.rs`.
- V2 follows B0 and V1. It is a separate crate, so it runs in parallel with B1–B4 (review M16, N20).
- R1–R5 and R7 need V4. R4 needs only V2.
- A2 starts after R6, on SQLite first.
- A3–A7 follow A2.
- A8 goes repository by repository.

---

# Part B: store read cost without re-hashing

**Goal:** reading an unchanged store hashes nothing. This holds in a fresh process, not only within
one handle. A changed file is hashed once. Part B applies to `eventlog-file` (EKR and other runtime
stores) and to `eventlog-tree`.

## B1. Measured

| measurement | value | source |
|---|---|---|
| EKR `head`, revision 40, `events.jsonl` 627 KB | 257 log opens, 161 MB read; 80% of perf samples in `sha2::sha256::soft` (no SHA-NI) | EKR session, 2026-09-24, eventlog `28e57856` |
| EKR `propose` / `commit`, revision 40 | 2,487 / 2,655 ms on file; 184 / 205 ms on SQLite | same |
| 200 read transactions, 1.51 MB history | 1,045 ms at `9f234c5`; 7 ms at `c698923` before the correction | eventlog `story:incremental-history-digest-for-a-resumed-handle` |
| AEP `list` on a planning store copy | 3,245,699,133 bytes read; 51% of 77.9 s in SHA-256 | eventlog `story:file-capture-blob-digests-without-rehashing` |

## B2. Cause

| step | code (eventlog `04ae526`) |
|---|---|
| each store call is one transaction | `read_stream` `lib.rs:1314`, `get_blob` `:1480` → `transaction` `:142` → `enter` `:948` |
| each resume re-reads and hashes the whole prefix, after checking the intent names | `journal.rs:529-570` `resumed_committed`, `:538` |
| every new process verifies from scratch | one `FileEventStore::open` per process (`ekr-store/src/eventlog.rs:94`; AEP #16) |
| a refusal drops the verified view | `lib.rs:152-156` |
| callers read call by call | EKR `ekr-store/src/eventlog.rs:313` `load_history` (at EKR `d865a82`), `:259` per-blob `get_blob` → O(n²) |

The re-hash exists because review `verify-once-review-1` F1–F3 showed that a prefix damaged in place
was served intact without it.

## B3. Resume by persisted file stamps (unit B1)

1. **`.cache/verified.json`**, written **under the writer lock** after a verification. It holds `{format, store, epoch, verifier_version, stamp_time, files: {path → {dev, ino, len, mtime_ns, ctime_ns}}, content_digest}`.
   - For `eventlog-file`: `events.jsonl`, `manifest.json` and the store directory. The directory's `mtime` moves when `append.json` or `privacy.json` appears (review M2).
   - For `eventlog-tree`, as built (§ 6): no `verified.json`. The fold goes in `.cache/state/<stamp-digest>.{sqlite,json}`, named by every history file's stamp, the projectors and the program; an open whose files all match loads it and anything else replays everything. Per-file verification of only the moved files (rule 2's last bullet) is not built.
2. **Open, any process:**
   - `flock`;
   - `stat` the intent names, as `journal.rs:538` does;
   - `statx` every stamped path.
   - All equal → **read the log once with no hash** (format 1: decode the frames; tree: read only unstamped files and load the fold).
   - A missing, unreadable or foreign cache (`store` or `epoch` mismatch) → full verification, then rewrite the cache.
   - Otherwise, for `eventlog-file`: any mismatch → today's full path, then rewrite the cache.
   - Otherwise, for `eventlog-tree`: verify only the files that are unstamped or whose stamp moved (review N19).
3. **Every later transaction:** `flock` + the same `stat`s. Hash only a file whose stamp moved or that is unstamped.
4. **Racy by omission, own writes included:** a file whose `ctime_ns ≥ read_start − 2 s` is **left out of** the stamp set.
   - The 2 s margin is 1 s of filesystem timestamp granularity plus the coarse clock tick (review M1).
   - A handle's own write gets no exemption (review N1). A same-length in-place damage microseconds after its commit can carry the same coarse-clock `ctime`, so that case is caught only because the written file stays unstamped for 2 s (`verify_once_review.rs:82`).
   - Within the window, a resume re-reads and hashes that file, as today; `read_many` collapses a burst of reads into one transaction.
   - The first open ≥ 2 s after the write hashes it once and stamps it.
5. **Trust envelope** (recorded in `file-provider.md`):

| change | detected |
|---|---|
| `write`, `pwrite`, `truncate`, `rename`-over by any process | yes: `ctime` or `ino` moves; userspace cannot set `ctime` |
| `cp -a`, `git checkout`, `git reset` | yes: `ino` or `ctime` moves, costing one full verification |
| a forged `.cache/verified.json` | not a new adversary: whoever can write `.cache/` can write the log. CI `verify` never reads `.cache/` |
| `mmap` stores after the first write fault of a dirty cycle | **no** (review M1) |
| block-device writes; root clock-and-`ctime` forgery; network filesystems | **no**; outside the envelope |

Every case in `verify_once_review*.rs` must pass unchanged, and must also exercise the
trusted-stamp branch.

## B4. Blob stamps (unit B2)

Blob files are content-addressed and immutable.

- Their stamps go in `verified.json`.
- `blob()` (`lib.rs:647`) skips the SHA-256 when the stamp is equal and not omitted.
- This decides eventlog `story:file-capture-blob-digests-without-rehashing` in favour of its option 2, plus rule 4.

## B5. Many reads in one transaction (B3) and view retention (B4)

**`read_many` (B3).**
- `EventStore::read_many(reads: &[Read]) -> Vec<ReadResult>`, where `Read` is `Stream { stream, after, limit } | Blob { tenant, digest }`.
- The default implementation loops, so SQLite and Postgres are unchanged; `eventlog-file` runs all the reads in one transaction.
- EKR's `load_history` adopts it in EKR's repository.
- It is safe: reads mutate no state.

**View retention (B4).**
- After a refusal, keep the **pre-work** `Verified { manifest, transactions, state, content }` (`lib.rs:63-71`), never `tx.state`. `Transaction::record` mutates state before commit (`lib.rs:441-443`; review M3).
- Condition: `tx.written.is_empty() && tx.pending.is_empty()`.

## B6. Targets

| measurement | now | target |
|---|---|---|
| 200 read transactions (counting case) | 1,045 ms | 0 hashes when the reads start ≥ 2 s after the last write. Within 2 s, one hash per transaction, collapsed to one by `read_many` |
| EKR `propose` / `commit` right after its own write | — | one 627 KB hash per transaction for 2 s (≈2 ms each; review N1 estimate) |
| fresh `ekr head`, revision 40 | 257 opens, 161 MB read | 0 bytes hashed; log read once; ≈10 ms of `flock` + `fstat` (review estimate) |
| EKR `propose` / `commit`, revision 40 | 2,487 / 2,655 ms | ≤ 400 ms |
| fresh `aep plan artifact list`, 1,000 × 10, unchanged | — | 0 event files opened; ≈10⁴ `statx` + one fold read |

## B7. Tests

- Clock-injected non-racy stamp, then in-place damage → refused.
- A second process opens with `prefix_bytes_hashed == 0`. `cost.rs` gains `files_hashed` and `files_stated`.
- A forged `verified.json` with a foreign `store` → ignored.
- A pending intent file appears → the full opener runs.
- A refusal mid-group, then a read → serves the pre-work view.
- `read_many` equals the loop on file, SQLite and Postgres.
