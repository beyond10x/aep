---
title: "0.12.0 — a workflow declares where a run may write"
description: >
  A step now declares the files it is handed and the paths it may write, with partial-only as the
  word that earns the shape. Measured on four live runs rather than asserted — and the toolset
  stopped a run the prose had only warned about.
slug: where-a-run-may-write
tags: [release, aep, driver]
date: 2026-08-24T16:18:54+02:00
release_tag: "0.12.0"
release_commit: f794aa41bb1511ba902495b91afc51e866b32cb1
---

A workflow step now declares two things it never could: **the files it is given**, and **the paths
it may write**.

{/* truncate */}

## The shape

`context:` names files a run is **handed** rather than has to find. `scope:` is an ordered list of
paths and one of three words — `allowed`, `partial-only`, `denied` — **first match wins**, and **the
last rule must name `**`**, so a path nobody thought of has an answer rather than a default.

Requiring the catch-all is the small decision that matters. A default is a policy nobody wrote; a
required final rule is a policy somebody had to type.

## Why `partial-only` earns the shape

It is the planning store's own rule. **The CLI owns the frontmatter**, so a body edit is legitimate
and a whole-file rewrite re-types it by hand.

**No set of operations can express that**, because a file write and a file edit are both writes. So
the document speaks **granularity, not identity** — and which of an adapter's operations replace a
file whole stays the adapter's own fact.

The rule already existed, in the driver's `store_integrity` check, **written in one vendor's tool
names** — so every arm but one walked past it for a year. A policy written in one harness's
vocabulary is a policy that only holds for that harness.

## The corpus stopped naming vendors

A call selector now takes a set of tools, an operation set and a subject, so a row meaning *"the
test was written before the code"* **decides on any harness** instead of being blind to the one that
used a differently-named edit verb.

The three-arm programme became four. The fourth is our own loop, **where there is no decision seam
at all and the published toolset is the policy**.

## Measured, not asserted

Four live runs, recorded with their transcript digests:

| run | result |
|---|---|
| unscoped | 10 of 11 expectations held; **three artifact files rewritten whole** |
| scoped and stated | 11 held; discovery calls down from **ten of fifty-two to three of forty-nine** |
| scoped and unstated | **five calls refused and the rule still held** |

The last one is the only run that measures this feature's own claim: with the prose warning removed,
**the toolset stopped the run** rather than the instructions doing it. Everything else could have
been the model being agreeable.

## And the gate

Green for the first time that session, **read from its own exit status rather than a pipeline's** —
the same mistake this repository already paid for once and wrote down.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
