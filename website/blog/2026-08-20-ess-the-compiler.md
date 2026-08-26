---
title: "0.3.1 — a specification that cannot hold a dangling reference"
description: >
  The compiler resolves a specification into an IR where a dangling reference cannot exist, and
  models the three layers above the domains — components, bindings, topology. A binding must state
  its delivery guarantee and what happens on failure.
slug: ess-the-compiler
tags: [release, ess]
---

A specification is only worth compiling if the compiled form can hold fewer wrong things than the
source did. This release resolves a specification into an **IR that cannot hold a dangling
reference** — not "is checked for", cannot hold.

{/* truncate */}

## What landed

* **The compiler and the IR.** Resolution happens once, and what comes out the other side is a model
  where every reference points at something.
* **Components, bindings and topology** — the three layers above the domains — so half of the design
  document's rejection list becomes checkable rather than advisory.
* **A binding states its delivery guarantee and what happens on failure**, both as *required* words.
  Not defaulted. A default here is a system-wide assumption nobody made on purpose, and the failure
  mode arrives in production.
* **A type crossing between two bounded contexts must be declared with the reason somebody had for
  allowing it.** The declaration is cheap; the reason is the thing that gets read two years later by
  whoever is deciding whether the coupling can go.

## One rule, one implementation

The compiler **bridges rather than duplicates**: each rule exists once, in `ess-domain`, with the
compiler adding a code, a structured body and a `file:line` on top. Two copies of a rule are two
rules the moment one of them is edited.

## Fixed from wave 1

An expression tree that reached itself through a union was refused as a forbidden dependency cycle —
a false refusal, which is the more expensive kind: it teaches people the tool is wrong and they stop
reading its output.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
