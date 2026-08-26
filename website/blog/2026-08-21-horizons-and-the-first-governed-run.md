---
title: "0.10.0 — evidence gets a shelf life, and the first governed run stops"
description: >
  A fact decays to Unknown past its horizon, never to False, and no API can extend one. Then the
  protocol governs a real story out of this repository's own backlog — and blocks four states short
  of where it was meant to.
slug: horizons-and-the-first-governed-run
tags: [release, aep, evidence, driver]
---

Two things, and the second is the one that matters.

{/* truncate */}

## Evidence horizons

An evidence record carries `observed_at` as **the identity of the fact** — not metadata about it. A
requirement may declare a **horizon**, past which the fact decays to **`Unknown`, and never to
`False`**.

That asymmetry is the design. A lapsed check has not failed; **nobody has run it**. A tool that
reported a stale pass as a failure would train people to re-run things to clear noise; one that
reported it as a pass would be lying.

**No API can extend a horizon.** There is deliberately no `extend` verb: if extending were as easy
to call as re-checking, it is the one that gets called by whoever is trying to get a gate green.

Against the adopter's vendored corpus: **42 of 42**, with self-reported coverage. It closed the
first adopter's ranked-first finding, and was designed and adversarially reviewed the same day it
was built.

## The first governed run — and it stopped

`protocol drive` walked a real story out of this repository's own store under `development.driven`.
Four headless model sessions, hooks as the enforcement arm, **80 decisions, 11 denies one-for-one
with the transcripts**.

**It blocked in `establish_verifiers`, four states short of the person it was meant to stop at.**

Both reasons the engine printed were correct refusals. The model wrote its failing checks as shell
scripts — the idiom the story's own acceptance is written in — while the map it was driving under
ran `cargo` in every state naming a verifier, so the suite came back green and `test-driven` refused
to advance.

**Nothing was changed to make the run go through.** The run is the finding: four new gap-register
rows, none patched to pass.

That is the whole reason to dogfood. A run engineered into a pass measures the engineering.

## The lab

`/lab` executes the synthesised billing realization as WebAssembly over its real boundary —
deterministic, asserted outside a browser, and **a page without a module says so instead of
pretending**.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
