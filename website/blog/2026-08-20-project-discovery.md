---
title: "0.2.1 — the first command needs no arguments"
description: >
  A project describes itself once in .engineering/project.yaml and the CLI finds it. Projects may
  add principles and profiles of their own, merged over the protocol tree's — without being able to
  shed what it enforces.
slug: project-discovery
tags: [release, aep, adoption]
---

An adopting team's very first command should not require them to have read the manual. This release
makes a project describe itself once, in `.engineering/project.yaml`, and the CLI find it.

{/* truncate */}

## What landed

* **Project discovery** — run `protocol` anywhere inside the tree and it locates the project.
* **Project-local principles and profiles**, merged over the protocol tree's. The important half is
  what a project **cannot** do: add its own rules, yes; **shed what the protocol tree enforces, no**.
  A vendored rulebook you can quietly delete rules from is a rulebook nobody downstream can rely on.

Plus two fixes from the wave-3 review, both worth naming because they are the class of bug review
exists to find:

* the approval floor was **not inherited through protocol extension** — so extending a protocol
  silently lowered its floor;
* the CLI crashed when its reader stopped reading, which is what a pipe closing looks like.

442 tests.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
