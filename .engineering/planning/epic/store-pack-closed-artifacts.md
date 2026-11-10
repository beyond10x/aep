---
format: aep.planning-md/3
id: epic:store-pack-closed-artifacts
kind: epic
status: draft
title: Closed work leaves the working tree without leaving the record
summary: Pack closed artifacts, their evidence and archived review-results into a few files, and unpack them, with every read verb answering the same.
relations:
- serves: vision:O2
revision: 1
---
# Epic: closed work leaves the working tree without leaving the record

## Outcome

A Git-native store can move its closed artifacts — those at a status their kind's lifecycle declares
closed — together with their evidence records and archived review-results, out of the loose layout
into a few pack files, and back. `validate`, `history`, `explain`, `list`, `findings` and `graph`
answer the same over packed records as over loose ones. Nothing is deleted.

## Why

Operator's request (2026-11-10, structure review of a downstream group): in one adopter repository
the planning store and its evidence are 4188 of 5860 tracked files (71 %), and no verb packs or
prunes closed artifacts. Reproduced on a fixture store with aep 0.79.0: no `pack`, `prune`,
`compact` or archive-out verb exists, and `move --to archived` keeps the file
(`architecture-design:store-pack`, section *Problem, measured*).

## Design

`architecture-design:store-pack` (representation, read overlay, writes and unpacking, determinism,
merge behaviour, what `validate` checks). Store version: `architecture-decision-record:packed-store-is-aep-project-6`.

## Constraints from the request

- Nothing is deleted from history or lost; `unpack` restores every file byte for byte.
- Closed = per each kind's lifecycle document, not status names in code.
- A record an open artifact still relates to stays resolvable.
- `pack --dry-run` prints what would be packed with file counts before and after.
- An older `aep` meeting a packed store refuses by name.

## Stories

- `story:lifecycle-declares-closed-states`
- `story:pack-format-and-overlay-read`
- `story:pack-and-unpack-verbs`
- `story:validate-checks-packs`
- `story:pack-end-to-end-and-docs`

## Out of scope

- Packing AEP's own store.
- Review-results that stay `active` after their subject closes:
  `story:review-results-stay-active-after-their-subject-closes` (draft, not accepted).
- The `aep:planning` skill text and `references/store-conventions.md`: they live in the sibling
  `agentplugins` repository.
