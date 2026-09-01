---
title: Lifecycles, decided as data
sidebar_position: 3
description: The status ladder every plan item climbs is a YAML file, not a Rust enum. What that buys — a new artifact kind with its own rungs, evidence-gated and date-gated moves, and an append-only history — plus the IO-free kernel that decides it.
---

# Lifecycles, decided as data

Every plan item in the planning store sits at a **status** and climbs a **ladder**: `draft` →
`proposed` → `active` → `implemented` → `accepted`, or whatever its kind declares instead. Until
`0.13.0` that ladder was a hand-written lookup in Rust, and the consequence for the person using it
was concrete: **a team whose work does not fit `story` had to send a pull request to this repository
and wait for a release.**

It is now a YAML file. Modelling a kind of work nobody here anticipated is a file you write in your
own repository, and the tooling — the board, the refusals, the validator — picks it up with no code
change on either side.

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

`transitions` is the ladder. `requires` is what a rung **costs**. Both are read; neither is
compiled in.

## Who decides

The decision is not made here. `crates/aep-backend-markdown/src/kernel.rs` hands the definition and
the attempted move to [`entity-core`](https://github.com/beyond10x/entity-runtime), a separate
kernel taken as a pinned dependency, and reads back a verdict.

`entity-core` is **IO-free by test, not by convention**: a banned-token scan over its own sources
refuses `fs`, `io`, `net`, `os`, `env`, `process`, `thread`, `time`, `sync` and `rand`, and its
entire dependency list is `serde` and `serde_json`. It cannot open a file, read a clock or reach a
network.

That is worth being precise about, because it is easy to over-read:

| entity-core does | entity-core does **not** do |
|---|---|
| take an entity type as data and answer *is this move permitted* | store anything — every byte the planning store writes is written here |
| evaluate a rung's declared conditions against values it is handed | read a clock; the instant is read at the edge and passed in as an argument |
| refuse an undeclared status by name | know what other artifacts exist, or resolve a reference |

The arrow points one way only, and never back: nothing from this repository appears in a manifest
of `entity-runtime`'s, at any version. A kernel that depended on its adopter could be shaped by one,
and then its verdicts would stop being a general answer.

The dependency is also **reversible**, which is what made it safe to take.
`crates/aep-backend-markdown/tests/kernel_equivalence.rs` holds the kernel's verdict identical to
the hand-written lookup it replaced across every kind/status pair; delete the module and the lookup
is still standing behind it.

## A rung can cost evidence

A ladder that only says *what may follow what* models permission, not earning. `requires` says what
a rung costs, and the refusal names which kind of `no` it is — which is the part that matters when
you are the person being refused:

```console
$ protocol artifact move outbound-claim:q3-uptime --to cleared
outbound-claim:q3-uptime is draft; cleared is on the ladder and not yet earned: reaching cleared
needs at least 1 approval record(s). Nothing was presented at $args.evidence.approval
```

*On the ladder and not yet earned* is a different sentence from *not on the ladder*, and the two
lead to different next actions. Record the observation and the same move goes through:

```console
$ protocol artifact evidence outbound-claim:q3-uptime \
    --kind approval --source "legal review" --ref https://example.invalid/approvals/814
outbound-claim:q3-uptime: approval recorded from legal review
  on hand: approval=1

$ protocol artifact move outbound-claim:q3-uptime --to cleared
outbound-claim:q3-uptime moved draft -> cleared (revision 2)
```

Evidence here is **three-valued**, the same as everywhere else in AEP: a rung whose condition cannot
be observed refuses as `Unknown` rather than passing as `False`. *Nobody looked* is not the same
answer as *somebody looked and it failed*, and neither is a pass.

## A rung can open on a date

A condition may compare instants, so a rung can be shut until a date arrives. The clock is still
read **at the edge** and handed to the kernel as an argument — never read inside it — so the same
inputs give the same verdict forever, which is what lets a decision be recorded and re-checked
later.

## The vocabulary is open to authors, not to typos

`ArtifactKind` and `ArtifactStatus` both carry an `Other(String)` variant, so a ladder may declare
rungs nobody here named. It is deliberately **not** open to any string: a status is accepted because
*some ladder declares it*, not because it parses. `drafted` was rejected in exactly this way — one
letter from the built-in `draft`, and a typo wearing a vocabulary's clothes.

Openness is also not the default answer everywhere. `evidence_kinds` stays **closed** on purpose:
an open evidence vocabulary would let a caller invent the kind of proof a gate is asking for.
[Open vocabularies](../reference/vocabulary.md) records which vocabularies are open, which are
closed, and what each closure buys.

## Three ladders that needed no Rust

Each of these is a file in `artifacts/lifecycles/`, added with no change to any crate.

| kind | models | the rung that is the point |
|---|---|---|
| `blocker` | something stopping work, typed by what would clear it | `cleared` is terminal and has no successor, which is what `list`, `board` and `blocked` read: while the rung is short of the end, everything the blocker points at with `blocks` is marked `blocked: <type>` |
| `obligation` | a commitment on a clock nobody here controls | `slipped` — and it must never gate a transition, because a missed external deadline is a fact to report, not a permission |
| `outbound-claim` | a statement that left the boundary — a number in a customer's inbox | `correction-owed`: *sent, known wrong, audience not yet told* |

The **type** is where `blocker`'s value is, and it is the kind rather than a field:
`credential-blocker`, `decision-blocker` and a team's own `procurement-blocker` all resolve to that
one ladder by their last hyphen segment, so a new type costs a name and no release.
`artifacts/kinds/blocker.yaml` writes down six as a starting set and checks nothing against them —
see [blocker types](../reference/vocabulary.md).

That is also what makes unblocking a *move* rather than an edit:

```console
$ protocol artifact blocked
credential-blocker:api-token-scope  credential  open, withholding test_result  CI cannot mint a read-scope token
  blocks story:ci-evidence      active  Evidence job for the contract suite
  blocks story:contract-checks  active  Contract checks in CI

$ protocol artifact move credential-blocker:api-token-scope --to cleared
credential-blocker:api-token-scope moved open -> cleared (revision 2)
```

Nothing was edited out of a file: `protocol artifact history` still says the blocker was opened, and
`cleared` is terminal, so being stuck again is a **new** blocker with its own date rather than this
one reopened — otherwise *how long were we stuck* has no answer.

`outbound-claim` is the one worth reading in full, because it runs the opposite way to everything
else here. Every other ladder models evidence flowing **inward**. An outbound claim is an assertion
that already left, and **sending is not undoable**:

```console
$ protocol artifact move outbound-claim:q3-uptime --to sent
outbound-claim:q3-uptime moved cleared -> sent (revision 3)

$ protocol artifact move outbound-claim:q3-uptime --to draft
outbound-claim:q3-uptime is sent; an outbound-claim may move to: correction-owed, standing
```

There is no path from `sent` back to `draft`. A ladder that allowed one would model retraction as an
edit — the claim would simply stop having been made — and the customer would still have the email.
A wrong claim moves *forward*: `correction-owed`, then `corrected`, which is a **new outbound act**
with its own evidence, costing two approvals rather than one. Fixing a claim costs a second claim,
which is true in the world, and the ladder should not be kinder than the world.

`correction-owed` is a rung rather than a flag for the same reason: it is the most expensive state
an organisation can be in and the easiest one to leave undocumented, and a rung is a column
`protocol artifact board` prints — and a column `protocol serve` draws in a browser, where the rungs
an artifact may take next are buttons and the ones it has not earned carry their price.

## Every write is journalled

Moves, creations and evidence records all append to a per-store journal, so an artifact's past is a
question you can ask rather than a `git log` you have to read:

```console
$ protocol artifact history outbound-claim:q3-uptime
2026-08-26T00:08:16Z  operator  created as draft (revision 1)
2026-08-26T00:08:20Z  operator  approval recorded from legal review (https://example.invalid/approvals/814) (revision 1)
2026-08-26T00:08:20Z  operator  moved draft -> cleared (revision 2)
2026-08-26T00:08:20Z  operator  moved cleared -> sent (revision 3)
```

The journal is **append-only JSONL**, and a corrupt line is skipped *and counted* rather than
silently dropped — a history that quietly shortens is worse than one that says it is damaged.

A move also records the **provenance** of what decided it, separating evidence that was *recorded*
from evidence that was *asserted* at the moment of the move. A move leaning on an assertion says so
when it happens, so the weaker claim is visible at the point of decision rather than discoverable
later.

## Writing your own

```console
$ protocol artifact lifecycle outbound-claim
outbound-claim starts at draft
  cleared -> sent
  corrected -> nothing
  correction-owed -> corrected
  draft -> cleared
  sent -> correction-owed, standing
  standing -> correction-owed
```

Put a `<kind>.yaml` beside the shipped ones in your own document tree, point `--root` at it, and
`new`, `move`, `board`, `lifecycle` and `validate` all understand it. `protocol artifact validate`
checks the whole store against the ladders, so a status no ladder declares is caught where it lands
rather than where it is read.

## Next

* [Govern a task](../guides/govern-a-task.md) — the workflow side, where evidence moves an execution
* [Evidence and completion](./evidence.md) — where the facts come from, and how they decay
* [CLI reference](../reference/cli.md) — the planning surface in full
