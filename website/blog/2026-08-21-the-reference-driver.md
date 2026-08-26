---
title: "0.9.0 — something walks the workflow, on evidence alone"
description: >
  The reference driver exists: protocol drive walks a workflow, blocks with the engine's reasons
  verbatim, and is held to the protocol verbs by enforcement hooks. Preceded by an adversarial
  review of six architecture decisions, with all seventeen corrections applied before a line was
  built.
slug: the-reference-driver
tags: [release, aep, driver]
---

Everything before this release *answered questions*. This one **walks a workflow** — and the design
of it was argued out and corrected before any of it was written.

{/* truncate */}

## Wave 2: the decisions, judged before the build

The driver's six open architecture questions became taken decisions, and an **adversarial
feasibility review judged every one against the code**: 23 confirmed, 14 needs-change, 3 infeasible.
**All seventeen corrections were applied before a line was built.**

Seventeen corrections is what a design document costs when somebody is genuinely trying to break it.
Paying that before implementation is the whole argument for having the document.

## Wave 3: the driver

* `aep-driver-spec` and `aep-driver`, with step maps under `drivers/`.
* **`protocol drive` walks a workflow on evidence alone**, and blocks **with the engine's reasons
  verbatim**. Not summarised, not re-worded — the driver is not allowed to become a second place
  where the meaning of a refusal is decided.
* **Enforcement hooks** holding the planning store's frontmatter and a driven shell to the protocol
  verbs, **with every adjudication logged**.
* A **driven eval** running a live session under the hooks, answering an open question empirically
  rather than by argument.
* The `development.driven` profile — the only one that grants a shell, and it is held to the
  `protocol` CLI by the driver's own per-call policy.
* `env.tool_available` as the fiftieth trace expectation kind.
* **`protocol workflow render`**, drawing a workflow and its live run in four formats.

## The split that makes it safe

**Gates are evaluated only by the engine. The driver asks and does what it is told.** A driver that
could evaluate a gate would be a second protocol implementation, and the two would disagree —
eventually, quietly, on the case that mattered.

Also in this release: the repository begins **planning itself** in `.engineering/`.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
