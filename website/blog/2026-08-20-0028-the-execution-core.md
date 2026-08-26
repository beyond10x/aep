---
title: "0.2.0-wave-1 — a task becomes a plan"
description: >
  The first release you can run against something. A task plus a document tree produces a plan,
  evidence decides what may be done and whether the work is finished, and a capability decision
  names the rule that decided it.
slug: the-execution-core
tags: [release, aep]
date: 2026-08-20T00:28:13+02:00
release_tag: "0.2.0-wave-1"
release_commit: d43ed035185a56b1c709a854b4c3342508e75556
---

`0.1.0` was a vocabulary. This is the first release where a question goes in and an answer comes
out: **a task plus a document tree produces a plan**, and evidence decides both what may be done
and whether the work is finished.

{/* truncate */}

## What landed

* **Resolution** — a task resolves against the document tree into the principles, capabilities and
  requirements actually in force for *it*, rather than a global rulebook you read yourself.
* **Live execution with derived facts**, so the state of a run is computed from what has been
  recorded rather than tracked in parallel.
* **Capability decisions that name the rule that decided.** A refusal that says "denied" is a
  refusal you argue with; one that names the document, the rule and the line is one you act on.
* **Completion as a checklist**, not a declaration — the beginning of the idea that finishing is a
  predicate over recorded facts.
* **49 protocol documents**, the `protocol` CLI, and a worked example that walks a task to
  completion.

163 tests, roughly 62% of the v0.2 scope.

## The design decision inside it

The engine evaluates; it never manufactures. Nothing in this release can produce the evidence that
would satisfy a rule it is evaluating, and that separation is why a later driver could be given a
shell without also being given the ability to approve its own work.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
