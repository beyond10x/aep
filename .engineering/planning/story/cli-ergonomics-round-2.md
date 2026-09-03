---
format: aep.planning-md/1
id: story:cli-ergonomics-round-2
kind: story
status: active
title: Six small refusals that each cost a session one retry
summary: 'Six one-retry refusals: --summary starting with a dash, describe with no store default, relations null, walk-up-only discovery, two-hop moves, reverse init without serves.'
tags:
- cli
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
scope:
- confidence: inferred
  path: crates/edge/aep-project/src/load.rs
- confidence: inferred
  path: crates/edge/protocol-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/protocol-cli/src/reverse.rs
- confidence: cited
  path: crates/govern/aep-domain/src/artifact.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 12
---
# Story: Six small refusals that each cost a session one retry

## Outcome

Each item is one retry in one or more sessions; together they are a round of friction the CLI can stop charging.

## Context

| # | item | evidence |
|---|---|---|
| 1 | `--summary "--flag …"` fails clap parsing; `--summary=…` works | `114c2340#196`; RETRO3 C12 |
| 2 | `describe <kind>` demands `--artifacts|--planning`; the store the project declares is not the default | `e70b8018 s1#226` |
| 3 | `list --format json` emits `relations: null`; the documented `jq` shape breaks | `3130470e#132` |
| 4 | project discovery walks up only; from the parent of the repository every call needs `cd` | `e70b8018 s1#182` (3413 `cd substrate` sites) |
| 5 | `draft → proposed → active` is two commands per story on every wave | `8cffc110#184`; `9da4f51c#3303` (python loop, 4 commands × 8 stories) |
| 6 | `reverse init` drafts carry no `serves` edge, so every wave hand-adds one before a story can leave `draft`; `reverse scan` returned 0 `todo_sites`/`disabled_tests`/`task_targets`/`api_surfaces` on a 51k-line Rust repo | `114c2340#43` (16 drafts, `serves=0`); `431986de` agent `a4bc555d3c698cc18#14` |

## Landed 2026-08-30

- 1 (`allow_hyphen_values` on `--title`/`--summary`, `new` and `set`); 3 for `list` and `show`
  (`graph --format json` still omits an empty `relations`: `skip_serializing_if` at
  `crates/govern/aep-domain/src/artifact.rs:1475`); 5 (`move --via`).

## Still open

- 2 (`describe` store default), 4 (`--project` / discovery from a parent directory), 6 (`reverse
  init --serves`, `reverse scan` naming empty scanners), and the `graph` half of 3.

## Acceptance

1. `--summary` and `--title` take values beginning with `-` (`allow_hyphen_values`).
2. `describe <type>` without a backend flag reads the project's planning store.
3. `relations` is `[]` when empty, in `list`, `show` and `graph --format json`.
4. `protocol --project <dir>` (or `AEP_PROJECT_DIR`, already read) is documented in `cli.md` beside every verb.
5. `move --to <status> --via` walks unconditional intermediate rungs and journals each; a rung with a
   requirement stops the walk with the usual refusal.
6. `reverse init` takes `--serves vision:<id>` and applies it to every draft; `reverse scan` reports which
   scanners found nothing rather than printing zeros.

## Out of Scope

`new --from` with a body file inside the store directory — the refusal is correct; move the file.
