---
title: "0.13 — the ladder is data"
description: >
  Release 0.13.0 takes the status ladder every plan item climbs out of Rust and puts it in a YAML
  file, decided by entity-core — a separate, IO-free kernel. The consequence: modelling a kind of
  work this repository never anticipated stops needing a pull request here and a release.
slug: the-ladder-is-data
tags: [release, aep, planning]
date: 2026-08-25T14:44:15+02:00
release_tag: "0.13.0"
release_commit: 0065cda3c40f03c9f9a294baec4e1e6c42db82b8
---

Every plan item in the planning store sits at a **status** and climbs a **ladder**: `draft` →
`proposed` → `active` → `implemented` → `accepted`. Until `0.13.0` that ladder was a hand-written
lookup in Rust, and the consequence for anybody using the tool was concrete and annoying: **a team
whose work does not fit the shapes we happened to think of had to send a pull request to this
repository and wait for a release.**

It is a YAML file now.

{/* truncate */}

## What changed

```yaml
# artifacts/lifecycles/story.yaml
kind: story
initial: draft
transitions:
  draft:       [proposed]
  proposed:    [active]
  active:      [implemented]
  implemented: [accepted]
  accepted:    []
```

Put a `<kind>.yaml` in your own document tree, point `--root` at it, and `new`, `move`, `board`,
`lifecycle` and `validate` all understand the kind. No crate changes on either side.

```shell-session
$ protocol artifact lifecycle blocker
blocker starts at open
  cleared -> nothing
  open -> cleared
```

## Who decides

Not this repository. `crates/plan/aep-backend-markdown/src/kernel.rs` hands the definition and the
attempted move to [`entity-core`](https://github.com/beyond10x/entity-runtime), a kernel that lives
in a separate repository and is taken here as a **git-pinned** dependency.

`entity-core` is IO-free **by test, not by convention**. A banned-token scan over its own sources
refuses the tokens for filesystem, network, clock, process, thread and randomness, and its entire
dependency list is `serde` and `serde_json`. It cannot open a file, read a clock or reach a network.

It is worth saying what that does *not* mean, because the phrase "runtime" invites the wrong
reading:

| entity-core does | entity-core does **not** do |
|---|---|
| take an entity type as data and answer *is this move permitted* | store anything — every byte the planning store writes is written here, by `aep-backend-markdown` |
| evaluate a rung's conditions against values handed to it | read a clock — the instant is read at the edge and passed in as an argument |
| refuse an undeclared status by name | know what other artifacts exist, or resolve a reference |

The arrow points one way and never back: nothing from this repository appears in a manifest of
`entity-runtime`'s, at any version. A kernel that depended on its adopter could be shaped by one,
and its verdicts would stop being a general answer.

## Why this was safe to do

Because it is reversible, and that was built before the dependency was taken.

`crates/plan/aep-backend-markdown/tests/kernel_equivalence.rs` holds the kernel's verdict identical to
the hand-written lookup it replaced, across every kind-and-status pair. Delete the module and the
lookup is still standing behind it. A dependency you cannot undo is a decision somebody else gets to
make later.

## The vocabulary is open to authors, not to typos

`ArtifactKind` and `ArtifactStatus` both gained an `Other(String)` variant, so a ladder may declare
rungs nobody here named. It is deliberately **not** open to any string: a status is accepted because
*some ladder declares it*, not because it parses.

That distinction earned its keep two releases later, when a new ladder was written starting at
`drafted` — one letter from the built-in `draft`, a typo wearing a vocabulary's clothes. It was
caught and corrected in `0.23.1`.

Openness is also not the default answer everywhere. `evidence_kinds` stays **closed** on purpose: an
open evidence vocabulary would let a caller invent the kind of proof a gate is asking for. Which
vocabularies are open, which are closed, and what each closure buys is written down rather than
inferred.

## What this bought

Everything in the eleven releases after it. A rung that costs evidence, a rung that opens on a date,
three new ladders — and, in `0.23.0`, a whole new kind of artifact added as **a YAML file with no
Rust change at all**, which was the point of the exercise.

See [Lifecycles, decided as data](/docs/concepts/lifecycles) for the full model.
