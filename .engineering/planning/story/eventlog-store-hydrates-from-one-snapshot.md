---
format: aep.planning-md/2
id: story:eventlog-store-hydrates-from-one-snapshot
kind: story
status: implemented
title: An Eventlog-backed planning store hydrates from one capture, not one per record
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/plan/aep-backend-eventlog/Cargo.toml
- confidence: cited
  path: crates/plan/aep-backend-eventlog/src/lib.rs
revision: 10
---
## Outcome

Opening an Eventlog-backed planning store costs one consistent capture of the authority, not one
capture per record. `aep plan artifact list` on the migrated real store copy runs in seconds, and
the number is measured on that copy and written into the CHANGELOG entry.

## Why

Measured 2026-09-21 by the wave sub-operator on one migrated copy of the AEP planning store
(`events.jsonl` 3,273,126 bytes, 1,116 files, 64 artifacts), three interleaved pairs per binary:

| binary | provider | `list` median | `history` |
| --- | --- | --- | --- |
| 340eb7f0 (2026-09-19) | f802eb8 | 93.2 s | 94.9 s |
| b48de121 (pin only) | db608cd | 93.5 s | |
| 70ff9868 (unit 6) | 76aad5e | 77.9 s | 80.2 s |

Profile of `list` with 70ff9868 (`perf record -g`, evidence/profile-list-reading-20260921.md):
`sha2::sha256::soft::compress` 51.17 % of samples; libc memcpy/malloc ~12 %; nothing from the
Eventlog journal walk, nothing from Entity Runtime's model build in the top 30. Syscalls per `list`:
222 `events.jsonl` opens, 244,862 blob opens, unchanged from the old binary.

Where the 222 come from, read from source at 1a5ceda4:

- `aep-backend-entity/src/lib.rs:944-993` `hydrate`, run once per process from `EntityBackend::over`
  (`:778`), does `store.ids(kind)` then `store.load(kind, id)` for each id, for four kinds.
- `aep-backend-eventlog/src/lib.rs:455-458` `ids()` fetches a `CompleteStoreSnapshot` through the
  bridge; `:446` `load()` makes a separate `bridge.load(subject)`; `:483` `records()` another.
- Each bridge read is one `capture_tenant` on the provider (`entity-eventlog/src/adapter.rs:475-493`),
  and each capture reads and hashes every bound object: ~1,103 objects, ~11 MB, per capture. Unit 6
  (story:file-capture-reuses-the-verified-view, Eventlog store) removed the journal half of that
  cost; the blob half stays by decision (story:file-capture-blob-digests-without-rehashing).
- The `CompleteStoreSnapshot` that `ids()` already holds carries, per subject, `history` and
  `terminal: EntityInstance` (`entity-store/src/asynchronous/types.rs:617-635`): everything `load`
  and `records` return.

So the store fetches a complete, consistent snapshot and then discards it 222 times. Retaining it
for the reads that follow, until a write goes through the same handle, makes `hydrate` one capture.
A snapshot is also the more consistent read: today's 222 captures are 222 instants.

## Scope

`crates/plan/aep-backend-eventlog/src/lib.rs` only, plus its tests and one CHANGELOG entry. No
change to Eventlog or Entity Runtime; the pins stay at 76aad5e / 8569da2. Markdown and Hybrid arms
untouched.

## Acceptance

- Red first: through a counting bridge or provider, one `hydrate` over a store of N records makes
  one capture (one `complete_snapshot`, zero per-subject `load`s reaching the bridge). Fails at
  1a5ceda4 with N+4.
- A write through the handle (`commit`, `commit_recorded`, `commit_planning_batch`) retires the
  retained snapshot; the next read captures again and sees the write. Red first.
- The snapshot is retained from `open`: `open` runs `validate_legacy_boundaries`, which takes the
  first capture, so no process ever holds a never-captured handle (pass 2 finding at lib.rs:573,
  2026-09-21). The doc comment and the CHANGELOG say exactly that: retained from open, retired by a
  write through the handle, captured again on the next read. The post-retirement fall-through to
  the bridge is pinned by a type-level case that constructs the store directly.
- What the retained snapshot answers equals what the bridge would answer, compared as values, for
  every kind and for an id that does not exist: `load` → `None`, history → the bridge's empty
  shape, lookup → `None`. Never a synthesized genesis. (Pass 1 findings F2, F4, F5, 2026-09-21: the
  count was pinned, the content was not.)
- Decision, 2026-09-21 (pass 1 finding F1): a handle is one command. Reads within it are mutually
  consistent as of the open; nothing in the repository holds an `EventlogPlanningStore` across two
  commands (`aep serve` holds a `StoreLocation`, the writer-control holder holds no store,
  `DrivenPlan` and `FileProjectionPublisher` open per call, `EventlogMutationLedger` has no
  caller). The retained-snapshot type's doc comment states this so a future long-lived holder
  meets the rule where the field is.
- `tests/drift.rs` 6, `planning_cli.rs` 86, `wave_derivation.rs` 13 unchanged; `store_writer_control`
  lane unchanged; repository 16-step gate green.
- `aep plan artifact list` and `history` measured on the same copy as the table above, three
  interleaved pairs against 70ff9868; the medians go into the CHANGELOG entry. Target: seconds, not
  tens of seconds; the number is reported as measured either way.
