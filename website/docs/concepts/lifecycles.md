---
title: Lifecycles and rungs
sidebar_position: 3
description: The status ladder each artifact kind climbs is a YAML file. A rung can cost evidence or open on a date, and a refusal says which kind of no it is.
---

# Lifecycles and rungs

Every artifact is at a **status** and moves along its kind's **lifecycle**, a ladder of statuses
declared in a YAML file. The ladder is data, not code. You model a new kind of work by writing a
file in your own document tree, and `new`, `move`, `board`, `lifecycle` and `validate` pick it up
without a new AEP release.

## The shape of a ladder

```yaml
kind: outbound-claim
initial: draft
transitions:
  draft:           [cleared]
  cleared:         [sent]
  sent:            [standing, correction-owed]
  standing:        [correction-owed]
  correction-owed: [corrected]
  corrected:       []
requires:
  cleared:   [{ evidence: approval, at_least: 1 }]
  corrected: [{ evidence: approval, at_least: 2 }]
```

| Key | Means |
|---|---|
| `initial` | the status `new` creates the artifact at |
| `transitions` | which statuses may follow which; a status with `[]` is terminal |
| `requires` | what reaching a rung **costs**: evidence records of a kind, at least so many |
| `when` | a rung that opens only after (or before) a date the artifact itself records |
| `descriptions` | one line per status, shown by `board --format markdown` |

The full format is in [lifecycle files](../reference/lifecycle-file.md).

```shell-session
$ aep plan artifact lifecycle outbound-claim
outbound-claim starts at draft
  cleared -> sent
  corrected -> nothing
  correction-owed -> corrected
  draft -> cleared
  sent -> correction-owed, standing
  standing -> correction-owed
```

## Two kinds of no

A move can fail for two reasons, and the refusal says which:

```shell-session
$ aep plan artifact move outbound-claim:q3-uptime --to cleared
outbound-claim:q3-uptime is draft; cleared is on the ladder and not yet earned: reaching cleared needs at least 1 approval record(s). no approval record is held for this artifact — …
$ aep plan artifact evidence outbound-claim:q3-uptime --kind approval \
    --source "legal review" --ref https://example.invalid/approvals/814
outbound-claim:q3-uptime: approval recorded from legal review
  on hand: approval=1
$ aep plan artifact move outbound-claim:q3-uptime --to cleared
outbound-claim:q3-uptime moved draft -> cleared (revision 2)
$ aep plan artifact move outbound-claim:q3-uptime --to sent
outbound-claim:q3-uptime moved cleared -> sent (revision 3)
$ aep plan artifact move outbound-claim:q3-uptime --to draft
outbound-claim:q3-uptime is sent; an outbound-claim may move to: correction-owed, standing
```

- **Not on the ladder**: the status cannot follow the current one. The refusal lists every status
  that can.
- **On the ladder, not yet earned**: the move is legal but its price is unpaid. The refusal names
  the evidence kind and how many records are missing.

`explain` prints the price of every legal next rung, so you can read the cost before you are
refused: `next: implemented needs 1 test_result record(s); held: 0`.

## Where the evidence comes from

`move` counts evidence in two ways:

| Source | How | Recorded as |
|---|---|---|
| **recorded** | `aep plan artifact evidence <id> --kind … --source …` writes one immutable file about that artifact, and `move` finds it | `decided_on: {"recorded": {…}}` on the transition |
| **asserted** | `aep plan artifact move <id> --to … --evidence test_result=1` states a count on the command line | `decided_on: {"asserted": {…}}`, and `validate` reports the artifact as *closed on an assertion* |

Assertion exists for evidence that lives outside the store, such as a CI run nobody recorded. The
move prints that it rested on an assertion, and `validate --strict` exits `1` on it. See
[Gate a move on evidence](../guides/gate-a-move-on-evidence.md).

Evidence is counted per artifact. A `test_result` recorded against one story does not pay for
another story's rung.

## A rung can open on a date

```yaml
kind: obligation
initial: open
transitions:
  open: [met, slipped]
  slipped: [met]
  met: []
when:
  slipped:
    after: due
```

`after: due` names a key in the artifact's own front matter. Until the instant is past that date,
the rung is refused:

```shell-session
$ aep plan artifact move obligation:pci-attestation --to slipped
obligation:pci-attestation is open; slipped is on the ladder and not yet earned: slipped is not reachable until this artifact's due has passed
$ aep plan artifact move obligation:pci-attestation --to slipped --at 2030-02-01T00:00:00Z
obligation:pci-attestation moved open -> slipped (revision 2)
```

The instant defaults to now. It is read at the command-line edge and passed into the decision, which
never reads a clock. `--at` pins it, so a move can be replayed later with the same answer.

## `--via`: several rungs in one command

`move --via` walks the intermediate rungs to the target and records each hop as its own transition,
so the history still shows every status the artifact passed through. It crosses only rungs nothing
guards. A rung with `requires:` or `when:` stops the walk with that rung's own refusal.

## Who decides

The move is decided by `entity-core`, an IO-free kernel from
[Entity Runtime](https://github.com/beyond10x/entity-runtime). AEP builds the kind's definition from
the lifecycle document, passes it the current status, the evidence counts and the instant, and gets
back a verdict. The kernel cannot open a file, read a clock or reach a network. Its own tests scan
its source for those capabilities.

## Ladders that ship

The shipped tree declares lifecycles for `vision`, `initiative`, `epic`, `story`, `task`,
`specification`, `design`, `architecture-decision-record`, `executable-system-specification`,
`review-result`, `blocker`, `obligation` and `outbound-claim`. A few worth knowing:

| Kind | Ladder | Why it is shaped that way |
|---|---|---|
| `story` | `draft → proposed → active → implemented → archived` (also `rejected`) | `implemented` costs one `test_result` |
| `review-result` | `active → archived` | a review is immutable once recorded; a second look is a second review |
| `<type>-blocker` | `open → cleared` | `cleared` is terminal, so being stuck again is a new blocker with its own date |
| `obligation` | `open → met`, `open → slipped → met` | `slipped` opens on the artifact's `due` date and never gates other work |
| `outbound-claim` | `draft → cleared → sent → …` | sending cannot be undone: a wrong claim moves forward to `correction-owed`, then `corrected` at the cost of two approvals |

## Kinds with no ladder of their own

The ladder for a kind is chosen in this order, and the first match wins:

1. the lifecycle declared for exactly that kind;
2. one declared for a kind it specialises, nearest first (`weekly-digest` → `digest`);
3. the tree's fallback lifecycle, a document with no `kind:`. A tree may declare at most one.
