---
title: "0.3.0 — the second half arrives, and the two halves meet"
description: >
  An executable system specification becomes a document this repository parses, validates and
  refuses — and the protocol can already require conformance to one. Nothing is generated from one
  yet, and the release says so.
slug: ess-the-join-and-the-model
tags: [release, ess]
---

Until now this repository had one half: **AEP**, which governs how engineering work is performed.
This release starts the other: **ESS**, which specifies what software must exist.

The join is the point. A task can be blocked until something proves a specification is satisfied —
so the two halves meet at evidence rather than at an integration.

{/* truncate */}

## What landed

* An **executable system specification** is a document this repository parses, validates and
  **refuses** — the refusals being the part that makes it a specification rather than a format.
* The protocol can already **require conformance** to one, which is the join.

## What did not

Stated in the release itself rather than discovered by a reader: **nothing is generated from a
specification yet.** The compiler, the projections and the test synthesis are waves 2 and 3.

That habit — a release naming what it did not do — is worth more than it costs. A reader who finds
the gap themselves has learned something about the documentation as well as about the tool.

## What the review found

Reviewed by three independent agents. Both blockers were **a guard that could not guard**, and both
fixes generalise rather than patch:

* every published schema is now checked against every document this repository ships;
* the validation-code list is **generated** rather than maintained by hand.

Fixing the instance and leaving the class is how the same bug comes back under a different name.

642 tests.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
