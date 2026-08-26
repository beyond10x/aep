---
title: "0.4.0 — the specification judges the implementation"
description: >
  A specification now generates its own conformance suite, runs it, and produces evidence the
  protocol reads to decide whether the work may be called done. Both halves proven: a correct
  implementation completes the task, a faulted one is refused by name.
slug: the-specification-as-oracle
tags: [release, ess, conformance]
date: 2026-08-20T18:09:55+02:00
release_tag: "0.4.0-ess-wave-4"
release_commit: 643ebdb50c924ddce1e6367621850ecb1e3a7053
---

This is the release where the two halves of the repository close a loop. A specification generates
its own conformance suite, runs it against an implementation, and produces **evidence the protocol
reads** to decide whether the work may be called done.

Both halves are proven, which matters more than either alone:

* a correct implementation passes **27 of 27 scenarios** and the task completes;
* the same implementation **with one fault injected** fails the scenario that exists to catch it,
  and the engine refuses to let the task complete, **naming the principle that refused**.

{/* truncate */}

## Five scenario families

Outcomes, lifecycle transitions, wrong-state refusals, entity invariants, and bindings with their
mapping, delivery and failure clauses. 27 scenarios from the normative example, 31 from a fixture
built for the corners the normative example cannot reach.

Suites are committed and drift-checked as a gate step, and **the index lists every construct that
got no scenario**. A suite quietly holding fewer checks than you think is the one failure a passing
run cannot show you.

## Twelve deliberately wrong implementations

Each with a matrix row asserting it is caught by the scenario meant to catch it, and a blast-radius
allowance per fault.

**One fault is caught by nothing, and says so**: an event may be published with any payload, because
nothing in the model relates a command's input to an emitted payload. Naming the hole in the release
is the only version of this that is useful.

## The oracle criticised the specification language first

Four model gaps closed — and **two were found by a generated suite refusing to generate**, rather
than by anyone reasoning about it: which instance a command acts on, and what a command does when
attempted in the wrong state.

Both were writable before and unassertable. It took something trying to generate a test from them to
notice.

## An honest limit, in the code

`independent: true` is **structural rather than attested**, and the code says so: it proves a record
came from the runner and not the agent. Nothing signs it, and the provenance digest is **left empty
rather than faking tamper evidence**.

1,216 tests, 50 suites.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
