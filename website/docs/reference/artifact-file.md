---
title: The artifact file
sidebar_position: 3
description: The aep.planning-md/3 format — YAML front matter and a Markdown body — with a fully annotated example and the owner of every field.
---

# The artifact file (`aep.planning-md/3`)

Each artifact in an `aep.project/5` store is one file at `.engineering/planning/<kind>/<name>.md`:
YAML front matter between `---` lines, then a Markdown body. The CLI writes the front matter in a
fixed key order, one transition per line, so a move is a small diff.

## An annotated example

The `#` comments are annotations for this page. The CLI does not write them.

```markdown
---
format: aep.planning-md/3          # CLI: always this value
id: story:save-card                # CLI: <kind>:<name>, equal to the path
kind: story                        # CLI: fixed at `new`
status: active                     # CLI: changed only by `move`; equals the last transition's `to`
title: Save a card for next time   # you: `new --title`, `set --title` or a hand edit
summary: Returning guests skip card entry.   # you: optional one line
owner: payments                    # you: optional
tags:                              # you: optional labels (`set --tag`, `--untag`)
- pci
refs:                              # you: records of the same work elsewhere (`--ref`, `--unref`)
- provider: jira
  reference: SHOP-12
relations:                         # you or the CLI: edges, in the relation vocabulary
- decomposes: epic:guest-checkout
- depends_on: accounts/story:legacy-login    # into another workspace member: hand-written only
scope:                             # CLI (`scope`): stories only; read by `waves`
- confidence: cited
  path: src/checkout.rs
- confidence: inferred
  path: src/vault.rs
revision: 6                        # CLI: 1 + the number of CLI writes to this file
transitions:                       # CLI (`move`): append-only, one line per move
- {from: "draft", to: "proposed", at: "2026-09-28T08:55:01Z", actor: "human:alex", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-28T08:55:01Z", actor: "human:alex", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Save a card for next time

## Outcome

A returning guest can pay with a card saved on their previous order.

## Acceptance

- A guest who opted in sees their saved card at checkout.
```

## Front matter

| Key | Owner | Rule |
|---|---|---|
| `format` | CLI | exactly `aep.planning-md/3` |
| `id` | CLI | `<kind>:<name>`; must match the file's path |
| `kind` | CLI | a kebab-case kind; see [artifacts and kinds](../concepts/artifacts.md) |
| `status` | CLI | a status the kind's lifecycle declares; equals the last transition's `to` |
| `title` | author | non-empty |
| `summary`, `owner` | author | optional strings |
| `tags` | author | optional list of strings |
| `refs` | author | optional list of `{provider, reference}`, written by `--ref <provider>:<reference>` |
| `relations` | author or CLI | list of one-key maps, `<relation>: <id>`; the id may be prefixed `<member>/` for a workspace member |
| `scope` | CLI | stories only; list of `{confidence: cited\|inferred, path}` |
| `withholds` | CLI | on a blocker: the evidence kind it stops anybody from producing |
| `model_digest` | CLI | on an `executable-system-specification`: the compiled model's digest |
| `prose_only` | CLI | on a `review-result` with no `findings` block: why, as `new --prose-only <reason>` recorded it |
| `revision` | CLI | an integer ≥ 1, incremented by every CLI write |
| `transitions` | CLI | see below |
| kind-specific keys | author | keys a lifecycle's `when:` reads, such as `due` on an `obligation` |

## `transitions`

One entry per move, oldest first, each on one line:

| Key | Present | Meaning |
|---|---|---|
| `from`, `to` | always | the statuses before and after |
| `at` | always | the instant of the move, UTC |
| `actor` | always | `human:<name>`, `agent:<name>`, `service:<name>` or `system`, from `AEP_ACTOR` |
| `revision` | always | the artifact's revision after the move |
| `decided_on` | when the rung cost evidence | `{"recorded": {<kind>: <n>}}` for evidence files the move found, `{"asserted": {<kind>: <n>}}` for counts given with `move --evidence` |
| `imported` | on migrated moves | `true` for a move carried over from an older store |
| `executor` | when `move --executor` names something other than the actor | what ran the move, such as `agent:release-17` |
| `correlation` | when `move --correlation` names an activity | the run, wave or other activity the move belongs to |

`validate` checks that the list is a continuous walk (each `from` is the previous `to`) and that it
ends at `status`.

## The body

Free Markdown. Two things in it are read as data:

- A fenced block whose info string is `findings`, on a `review-result`: a JSON (or YAML) list of
  findings. See [reviews and findings](../concepts/reviews.md). A `findings` fence inside a longer
  fence (an example quoted inside a four-backtick block) is prose.
- Headings: `body --section <heading>` replaces the prose under one `##` heading.

`review-result` is immutable, and every edit after `new` is refused, so its body arrives with
`new --from` (and `--findings`).

## Editing by hand

You may edit the body, `title`, `summary`, `owner`, `tags`, `refs`, `relations` and kind-specific
keys. Leave `format`, `id`, `kind`, `status`, `revision` and `transitions` to the CLI. `validate`
reports a `status` that disagrees with the last transition, and a path that disagrees with the id.

The JSON Schema for the front matter is
[`planning-document.schema.json`](https://github.com/beyond10x/aep/blob/main/schemas/generated/planning-document.schema.json).
