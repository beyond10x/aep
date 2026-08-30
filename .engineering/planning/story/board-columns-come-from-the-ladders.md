---
format: aep.planning-md/1
id: story:board-columns-come-from-the-ladders
kind: story
status: implemented
title: The board's columns come from the ladders, not from a list compiled into the binary
summary: 'A blocker at open, or any adopter-declared status, appears in no board column: board iterates ArtifactStatus::ALL (planning.rs:2142). Found landing story:blocker-relation.'
tags:
- bug
- store
relations:
- informed_by: story:blocker-relation
- serves: vision:O2
revision: 5
---
# Story: The board's columns come from the ladders, not from a list compiled into the binary

## Outcome

An artifact whose status is one an adopter declared — a `blocker` at `open`, a kind of their own at a rung of their own — appears on `protocol artifact board` in a column of that name. Today it appears in no column at all, and the only trace of it is the `[blocked: …]` marker on the items it is holding up.

## Context

Found while landing `story:blocker-relation` (2026-08-30). `protocol artifact board` builds its columns from `ArtifactStatus::ALL` (`crates/protocol-cli/src/planning.rs:2142`), the list of statuses compiled into the binary — while `ArtifactStatus` is an open vocabulary (`Other(String)`, gated by each kind's ladder at `parse_status_in`, see `docs/guide/open-vocabulary.md`, row *Artifact status values a lifecycle document may name*). A `blocker` starts at `open` and ends at `cleared` (`artifacts/lifecycles/blocker.yaml`); neither is in `ALL`, so the blocker itself is invisible on the board although the story it blocks is marked correctly. `list` is unaffected: it prints rows, not columns.

## Acceptance

- `board` derives its columns from the union of the ladders the store's kinds declare (`protocol artifact lifecycle <kind>` for every kind present), in ladder order, and the compiled list is used for nothing but the default ordering of the statuses it knows.
- A store holding one `blocker` at `open` shows an `open` column with that blocker in it, asserted by a test on the passkeys fixture plus one blocker.
- A status a ladder declares and no artifact holds still gets no column — an empty column is noise, not information.
- `--kind` narrows the columns to that kind's ladder.

## Out of Scope

Rendering blockers differently from other artifacts; `story:blocker-relation` already marks the blocked side.

## Open Questions

Whether an artifact whose status is on no ladder at all (a document `validate` already refuses) should still be shown. Decides: protocol owner. Default if nobody answers: **no** — `validate` is the surface for that finding.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/protocol-cli` — the planning verbs, the `board` renderer — cited
- **Files:** `crates/protocol-cli/src/planning.rs:2132-2177` (`fn board`, the column build at `:2142`) — cited
- **Files:** `crates/protocol-cli/src/planning.rs:3741` (`struct Column`, whose `status: &'static str` cannot hold an adopter's rung) — inferred, the field type forces a change the story does not name
- **Files:** `crates/protocol-cli/tests/planning_cli.rs:579-587` (`the_board_groups_the_fixture_into_status_columns`) — inferred, where the new assertion belongs
- **Symbols:** `ArtifactStatus::ALL`, `board`, `Column`, `ladders_or_none`, `select` — cited
- **Symbols:** `ArtifactLifecycle::statuses` (`crates/aep-domain/src/artifact.rs:1753`), `LifecycleRegistry::iter` (`:1859`), `for_kind` (`:1835`) — inferred, the union-of-ladders reads the acceptance implies
- **Also likely:** `crates/aep-domain/src/artifact.rs` — inferred, only if ladder *order* needs a helper: `statuses()` returns a `BTreeSet` and `transitions` a `BTreeMap`, so nothing today walks a ladder from `initial`
- **Documents:** `website/docs/reference/cli.md:63`, `website/docs/concepts/lifecycles.md:167` — inferred, both describe the columns; `docs-check` holds the first
- **Confidence:** high — the story names the defect site and `git grep` confirms `planning.rs:2142` is the only column build in the tree. Medium only on where the ordering helper lands.
- **Would collide with:** any unit touching `protocol-cli`'s planning surface (`src/planning.rs`, `tests/planning_cli.rs`); and, if the acceptance's "passkeys fixture plus one blocker" adds a document rather than building a scratch store, the `examples/planning-passkeys` store, whose counts are asserted verbatim by fifteen `FIXTURE` sites.

**Not established.** Whether ladder *order* needs a new symbol — nothing orders statuses by walking `transitions` from `initial`. Whether the fixture gains a blocker (breaking asserted counts) or a scratch store is used, as `planning_cli.rs:1259` does. How `--kind` narrows when `select` matches by `is_a`, so `--kind design` admits `architecture-design` — one ladder, or several.
