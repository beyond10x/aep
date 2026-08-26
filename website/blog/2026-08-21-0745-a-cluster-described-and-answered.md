---
title: "0.7.1 — the same pattern, pointed at a Kubernetes cluster"
description: >
  A running cluster is observed, compiled, diagnosed, held to an authored desired state
  three-valued, and answered with patches. Nothing here holds a credential, reaches a network, or
  applies anything — the diff is the deliverable.
slug: a-cluster-described-and-answered
tags: [release, infrastructure]
date: 2026-08-21T07:45:45+02:00
release_tag: "0.7.1-infra-waves-1-4"
release_commit: 113c66752de790a735dfd215c69220dd733667dc
---

The ESS half of this repository compiles a specification and judges an implementation against it.
This release does the same thing to **a running Kubernetes cluster**, which is the first evidence
that the pattern was general rather than a description of one problem.

{/* truncate */}

## Observed, not queried

An **external actor** scans the cluster into an observation bundle, and **secret values never touch
disk**. Nothing in this repository holds a kubeconfig, which is why the scanner is a separate
repository.

The bundle compiles in about a tenth of a second into a **content-addressed IR where a dangling
reference is a typed fact rather than an error** — because observed infrastructure *is allowed to be
wrong*. That is the inversion that makes the rest work: a compiler for authored input refuses a
broken reference; a compiler for observed input has to record it.

## Diagnosed

A dependency graph with exact pod ownership, **twenty diagnosis rules**, invariant candidates that
carry their exceptions as evidence, and **directions that collapse a wall of warnings into a handful
of causes**. Rendered as text, JSON, Mermaid, or one self-contained HTML page whose components wear
their worst finding.

## Held to account, three-valued

An authored desired state is evaluated `ok` / `gap` / **`undecidable`**. An expectation the snapshot
cannot decide is undecidable with **one of six named reasons**, never collapsed to false — and a
`False` without its typed gap is **unrepresentable in the type**, so the collapse cannot be written
by accident.

On the operator's real cluster: **four hold, five gaps, nineteen undecidable**, each saying why.

Nineteen undecidable out of twenty-eight is not a bad result being hidden. It is the honest shape of
what a snapshot can answer, and a tool reporting four-hold-five-fail would have been lying about the
other nineteen.

## And the answer back

Every gap becomes:

* **a patch**, where the fix is mechanically safe;
* **a named obligation**, where a value is a human's to choose;
* **a refusal**, where the gap is not a field at all.

With the round trip asserted on every gate run: **applying the emitted tree closes exactly what it
claims and moves nothing else.**

Nothing here applies anything. The diff is the deliverable; the decision stays with a person.

1,811 tests, 106 suites.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
