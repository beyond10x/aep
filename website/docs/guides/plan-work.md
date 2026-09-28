---
title: Plan work
sidebar_position: 1
description: Create, relate, write, tag, move and read artifacts day to day — the commands a planning session actually uses.
---

# Plan work

This guide covers the everyday planning commands. It assumes a repository with an
`aep.project/5` project file. If you do not have one yet, the [quickstart](../getting-started.md)
creates it in two commands.

## Before the first write

```bash
aep plan artifact list          # what is already there
aep plan artifact kinds         # what can be created
aep plan artifact lifecycle story
```

Set who you are once per shell, so every move records the right actor:

```bash
export AEP_ACTOR=human:alex      # or agent:<name>, service:<name>, system
```

A value that does not parse is refused rather than replaced. When `AEP_ACTOR` is unset, the actor is
`human:$USER`.

## Break work down

Create from the top down, with the edge on the child:

```bash
aep plan artifact new epic guest-checkout --title "Guest checkout" \
  --summary "Buy without creating an account"
aep plan artifact new story pay-by-card --title "Pay by card as a guest" \
  --relate decomposes:epic:guest-checkout --owner payments --tag pci
aep plan artifact new task pay-by-card-api --title "Card payment endpoint" \
  --relate decomposes:story:pay-by-card
```

`--relate` repeats. `--summary` and `--title` accept values that begin with `-`.

## Write the body

A new artifact's body is its kind's template. Replace it, add to it, or rewrite one section:

```bash
aep plan artifact body story:pay-by-card --from story.md                  # whole body
aep plan artifact body story:pay-by-card --from notes.md --append         # add at the end
aep plan artifact body story:pay-by-card --from acc.md --section Acceptance  # replace one ## section
```

`--from -` reads standard input. `show --body-only` prints the body bytes and nothing else, so you
can read a body out, edit it and hand it back:

```bash
aep plan artifact show story:pay-by-card --body-only > story.md
$EDITOR story.md
aep plan artifact body story:pay-by-card --from story.md
```

Editing the body directly in the file also works. The CLI only needs you to leave `status`,
`revision` and `transitions` alone.

## Change a field

```shell-session
$ aep plan artifact set story:save-card --owner payments --tag pci
story:save-card owner, tags set (revision 3) at …/planning/story/save-card.md
$ aep plan artifact set story:save-card --status active
error: `status` is not a field `set` changes: a status is a decision taken against the kind's lifecycle, and `… move <id> --to <status>` is what takes it and records what it rested on
```

`set` takes `--title`, `--summary`, `--owner`, `--tag`/`--untag`, `--ref`/`--unref`, and
`--model-digest` on an executable system specification. It refuses `status`, `revision`, `id` and
`kind` by name.

## Relate and unrelate

```bash
aep plan artifact relate story:refund-guest depends_on story:guest-receipt
aep plan artifact relate story:refund-guest depends_on:story:guest-receipt   # same edge, other spelling
aep plan artifact unrelate story:refund-guest depends_on story:guest-receipt
```

`unrelate` removes exactly the edge you name. An edge the artifact does not declare is refused, and
the refusal lists the edges it does declare:

```shell-session
$ aep plan artifact unrelate story:refund-guest depends_on:story:nope
error: story:refund-guest does not declare `depends_on story:nope`; it declares:
  - decomposes epic:guest-checkout
  - depends_on story:guest-receipt
```

## Move

```bash
aep plan artifact move story:pay-by-card --to proposed
aep plan artifact move story:pay-by-card --to active --via     # walk every unguarded rung between
```

A refusal lists the legal moves, or names the evidence a rung still needs. Record the evidence with
`aep plan artifact evidence` and move again. See
[Gate a move on evidence](./gate-a-move-on-evidence.md).

## Read the plan

| Command | Shows |
|---|---|
| `list [--kind …] [--status …] [--ref …]` | one line per artifact; a blocked one says `blocked: <type>` |
| `board [--kind …] [--format markdown]` | status columns; `markdown` renders a page with each column's description |
| `show <id>` | fields, scope, relations, findings, then the body verbatim |
| `blocked [--type …]` | what is stopped, grouped by the blocker |
| `graph [--format dot\|mermaid\|json]` | the artifact graph |
| `history <id>` | moves and evidence records, oldest first |
| `explain <id>` | what each status rested on, and what each next rung costs |
| `aep plan serve` | all of the above in a browser, on `127.0.0.1`, with moves as buttons |

Every reading command takes `--format json` or `--format yaml`. In JSON, `list` and `show` always
carry `relations`, and `list` always carries `blocked_by`. Both are `[]` when empty, never missing.

## Blockers

When something stops work, record it as a blocker typed by what would clear it, and point it at
what it blocks:

```bash
aep plan artifact new decision-blocker card-vault-provider \
  --title "Which card vault do we use?" --relate blocks:story:save-card
aep plan artifact blocked
aep plan artifact move decision-blocker:card-vault-provider --to cleared   # unblocks
```

## Finish a batch

```bash
aep plan artifact validate
git add .engineering && git commit
```

`validate` reads the whole store and exits `1` on any problem. A person reviews the plan change in
the pull request like any other diff.
