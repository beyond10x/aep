---
title: "0.11.0 — the corpus becomes ground truth, and a gate that lied gets caught"
description: >
  An outside adopter re-issued the horizons corpus as ground truth, and the scanner followed. Plus
  the process lesson that cost the most: two gates that aborted at their first step were read as
  green through a piped exit status.
slug: ground-truth-and-docs
tags: [release, aep, evidence]
---

The most useful thing in this release is a mistake it documents.

{/* truncate */}

## The corpus becomes ground truth

The adopter fixed **their** reference against this repository's scanner and re-issued
`expected.json`: **43 raw, 43 parsed, `missed_by_reference` zero** — adding a seventh position and a
rule running the other way. **An annotation inside a fenced code block is an illustration**,
excluded from parsing *and* from the coverage denominator both.

The scanner follows, with the **fence-stripping implemented twice on purpose** so the denominator
stays independent evidence. Two implementations agreeing is evidence; one implementation checking
itself is a tautology.

This is what ground truth is supposed to look like: the corpus is not ours, and it corrected us as
often as we corrected it.

## The gate that was read as green

Two gates had **aborted at their first step** and were misread as green **through a piped exit
status** — `$?` was the pipeline's, not the gate's.

The record is corrected **in git notes** rather than by rewriting history, and **the first honestly
captured gate immediately caught a stale test**. Which is the point: the reason to fix the reading
is not tidiness, it is that the gate had something to say and nobody heard it.

The rule that came out of it — *read the gate's own exit code, never a pipeline's* — has been
applied on every gate run since, including the one that cut this release.

## Also

`protocol evidence inspect` no longer refuses a record **the day it is written**: the future check
now runs at the reference date's own granularity.

And the documentation caught up with the tree under one rule: **every number from a command, every
reference resolving, every quoted output reproduced.**

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
