---
format: aep.planning-md/2
id: story:tree-store-migrates-to-texts-stored-once
kind: story
status: draft
title: A tree planning store migrates to texts stored once
relations:
- serves: vision:O2
- decomposes: epic:planning-on-entity-runtime
revision: 2
---
# A tree planning store migrates to texts stored once

## Outcome

`aep plan store migrate texts` moves an `aep.project/3` tree store from `eventlog-tree/1` to
`eventlog-tree/2` (Eventlog 0.6.0, `docs/design/tree-text-blobs-v0.1.md` there), with `--dry-run`
and `--check`, a read-back per blob and a whole-store comparison afterwards. The plan reads exactly
as before: the same `list`, the same `show`, the same projection, `validate` clean. Entity Runtime
and Eventlog are pinned at releases that read both layouts.

## Why

A tree planning store keeps each import anchor and record as one blob of its exact bytes. On a copy
of the ESS planning store the largest blob is 75,332,948 bytes and two exceed the 8 MiB per-file
limit of the Gates scanner, so a push of the store is refused. The operator's direction,
2026-09-27: store the text once and point to it by its hash; every existing store must still read
exactly as before; it needs a migrate command.

## Acceptance

- `an_older_store_migrates_to_texts_stored_once_and_reads_exactly_as_before`
  (`crates/edge/aep-cli/tests/tree_text_migration.rs`): dry run writes nothing, `--check` exits 1
  before and 0 after, `list` and `show` are byte-identical, projection and event files untouched,
  `validate` passes, a second run changes nothing.
- `a_store_that_is_not_a_tree_is_refused_by_name`.
- On a copy of the ESS planning store every file is under 8 MiB after the migration.
