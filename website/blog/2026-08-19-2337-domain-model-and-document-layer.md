---
title: "0.1.0 — the model everything else is built on"
description: >
  The first release is a vocabulary, not a feature. Typed identifiers, a three-valued predicate
  language, capabilities that default to deny, evidence that carries its provenance, and documents
  read with syntax separated from semantics.
slug: domain-model-and-document-layer
tags: [release, aep]
date: 2026-08-19T23:37:37+02:00
release_tag: "0.1.0"
release_commit: 6b0230804f049b2820bd309ee6a73ab0531926dc
---

The first release does nothing you can run against your own repository. It is the vocabulary
everything after it is written in, and the decisions taken here are the ones that could not be
changed later without changing what the tool means.

{/* truncate */}

## What landed

* **Typed identifiers and an artifact graph with lifecycles** — a plan item is a kind at a status,
  and the statuses it may reach are a property of the kind rather than of the code reading it.
* **A three-valued predicate language.** `True`, `False` and **`Unknown`**, from the first commit.
  *Nobody looked* is not the same answer as *somebody looked and it failed*, and a tool that
  collapses them into "not satisfied" has thrown away the distinction that decides what you do next.
* **Capabilities that default to deny**, with an approval floor. Widening a grant is a change to a
  document, not a convenience in code.
* **Evidence with provenance** — a fact carries who produced it and how, because a completion rule
  that cannot tell a verifier's result from an author's claim is a rule that can be talked past.
* **Principles with phase-timed obligations**, so a rule can say *when* in the work it applies
  rather than only *what* it demands.
* **Document reading that separates syntax from semantics.** A malformed file and a well-formed file
  saying something forbidden are different failures with different fixes, and they are reported
  differently.
* **JSON Schema generated from the Rust types**, so the published schema cannot drift from what the
  code accepts.

98 tests, and roughly 30% of the v0.2 scope.

## Why the three-valued choice matters more than it looks

Every gate in this project is downstream of it. A requirement whose evidence has decayed past its
horizon reads `Unknown`, never `False` — a lapsed check has not failed, nobody has run it. Ten
releases later that is what lets a rung on a status ladder refuse as *not yet earned* rather than
*not permitted*, which are different sentences leading to different next actions.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag. Unlike
the current posts, it carries no re-run command output — the code has moved on, and inventing a
transcript for a released version would be worse than describing it.*
