---
title: "0.2.0-wave-2 — every object addressable, every mutation auditable"
description: >
  Opaque identity, logical locators, monotonic revisions, and one command boundary with idempotent
  replay and conflicts that refuse rather than merge. Plus the audit record that keeps the refusals.
slug: identity-and-the-interaction-contract
tags: [release, aep]
date: 2026-08-20T01:02:32+02:00
release_tag: "0.2.0-wave-2"
release_commit: 17342c737baacd327de548e0e2449439f47d1833
---

A protocol that decides things has to be able to say **which thing** it decided about, and **who**
asked. This release makes every object addressable and every mutation auditable.

{/* truncate */}

## What landed

* **Opaque identity and logical locators** — an object's stable identity is separate from the name
  you look it up by, so renaming is not a new object.
* **Versioned types and monotonic revisions.**
* **Actor versus executor**, held apart. The person the work is attributed to and the process that
  performed it are different facts, and a system that conflates them cannot answer who approved
  something once an agent is in the loop.
* **One command boundary**, with idempotent replay and revision conflicts that **refuse rather than
  merge**. A concurrent write that silently merged would produce a state nobody chose and an audit
  trail that reads as if somebody had.
* **Domain events and audit records, including the refusals.** What was *not* allowed is kept, which
  is the half most systems throw away and the half you need when something goes wrong.
* **An in-memory backend** that passes the specification's nineteen-step reference scenario.

276 tests, roughly 83% of the v0.2 scope.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
