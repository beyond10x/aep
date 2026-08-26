---
title: "0.3.2 — one source of truth stops being a claim"
description: >
  The specification now produces the documentation and the contracts — Markdown with Mermaid, JSON
  Schema, OpenAPI 3.1, AsyncAPI 3.0 — all drift-checked in CI. And the three projections turned out
  to disagree with each other on all 17 comparable pairs.
slug: ess-the-projections
tags: [release, ess]
---

"One source of truth" is a claim until something is actually generated from it. This release
generates four things, commits them, and fails CI if they drift.

{/* truncate */}

## What landed

Markdown with Mermaid, JSON Schema per message and named type, OpenAPI 3.1 and AsyncAPI 3.0 per
component — all four behind one `Generator` trait. **27 artifacts and a generated index**, committed
and drift-checked, each carrying the specification version, a digest of the resolved IR, the compiler
version and the generator version.

Entities, views and actors reach the IR, so the documentation renders every construct the
specification language has, and **the gap allowlist is empty** — with a test asserting the emptiness
rather than a comment claiming it.

## The bug that justified the wave

The three projections each carried **their own copy of the type mapping**, and **all 17 comparable
pairs disagreed**.

The AsyncAPI documents were the permissive side. A service validating an event against them would
accept a `Money` with a non-numeric amount, and unknown extra fields, that the JSON Schema tree
refused. Two documents generated from one specification, describing different systems, and nothing
would have noticed.

One mapping now, and `tests/agreement.rs` compares every shared construct **keyword by keyword** to
keep it that way.

## What the generator will not invent

Where the model says nothing, no document makes something up. The HTTP path and the channel address
are a **stated convention, recorded in the generated document** so a reader knows it was a
convention. `may:` is published as an annotation rather than as a security scheme, because a scheme
would describe an authentication mechanism the specification says nothing about.

## Not met, in writing

The OpenAPI and AsyncAPI **envelopes** are checked structurally rather than against their own
meta-schemas, neither of which is vendored here. Every schema they *embed* is validated for real.

916 tests. ESS at roughly 60% of its design.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
