---
title: "0.24.0 — the docs say what the tooling does, and a tag cuts its own release"
description: >
  The site had drifted to the point of misdescribing the repository: since 0.13.0 the code gained
  7,805 lines and the website gained three. Plus a release workflow, and a self-test that found a
  real defect on its first run.
slug: the-docs-catch-up
tags: [release, docs, ci]
---

Since `0.13.0` this repository gained **7,805 lines** of code and its website gained **three**.
`protocol reverse` — four verbs and 2,782 lines — had **no mention anywhere on the site**, and the
status page still opened *"current as of `0.10.0-horizons-dogfood-lab`"*.

{/* truncate */}

## The page that was missing

[Lifecycles, decided as data](/docs/concepts/lifecycles) — the status ladder is a YAML file decided
by `entity-core`, a rung may cost evidence or open on a date, and every write is journalled.

It also states what that kernel **does not** do, because *"entity runtime"* invites the reading that
it stores something. It performs **no IO at all** — a scan over its own sources refuses the tokens
for filesystem, clock, network and randomness — so **every byte written here is written here**. The
dependency buys a verdict, not a store.

## The rest of the site

| page | now says |
|---|---|
| **CLI reference** | `protocol reverse`, plus `artifact history` and `artifact evidence` |
| **Where this stands** | current as of `0.23.2`, with the ladder, the engine's four mechanisms, and adoption from the other end |
| **Roadmap** | the delivered table ran to `0.10.0` and now runs to `0.24.0` |
| **Limitations** | the store **does** have a journal since `0.19.0`; the contract gap is unchanged and says so |
| **Vocabulary** | kinds and statuses are open to authors, and why `evidence_kinds` is closed |
| **/releases** | **one post per release, all 33 of them** — the backported ones marked as written retrospectively, with no invented command output |

## Counts, each re-derived rather than copied forward

CLI verbs 17 → **20**. Artifact lifecycles 8 → **12**. This repository's own plan 59 → **101**
artifacts. The document tree 45 → **49** files. The gate ten → **twelve** steps, in five files.

`AGENTS.md` also still claimed the billing suite runs **27** scenarios where its own guard asserts
**29** — a count the gap register closed in two places and missed in a third.

## A tag now cuts its own release

`.github/workflows/release.yml` fires on a version tag, runs `ci.yml` **itself** — called rather
than copied, so a tag cannot ship against a shorter gate than `main` does — and takes its notes from
the tag's own section of the changelog.

Release notes render as **GFM**, where a single newline becomes a `<br>`, so a file hard-wrapped at
100 columns arrives broken after *"added"* and before *"the"*. The file stays wrapped, because that
is the right shape for something reviewed in a diff, and only the notes are joined.

## The self-test earned its place immediately

`cargo xtask notes --self-test` holds the eight shapes the reflow must not damage, and it runs
**before** the notes are generated — because the failure it catches is silent, and nobody re-reads a
release they already cut.

On its first run it failed. **A line ending in two spaces is Markdown asking for a break**, and the
reflow was ending the paragraph but dropping the two spaces. The break survived only because GFM
turns a bare newline into `<br>` — standing on **the exact quirk the reflow exists to remove**.

The same defect was in the original Python implementation in `entity-runtime`, whose own self-test
had a case for it **asserting the wrong expectation**. Both are fixed.
