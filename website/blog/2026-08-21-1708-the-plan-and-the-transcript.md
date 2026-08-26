---
title: "0.8.0 — the repository starts planning itself, and a transcript gets judged"
description: >
  Planning artifacts become markdown in the repository, moved only through lifecycle-validated
  verbs. And an agent transcript normalises into an event IR judged against a typed specification —
  the third observation domain after a specification and a cluster.
slug: the-plan-and-the-transcript
tags: [release, aep, planning, trace]
date: 2026-08-21T17:08:47+02:00
release_tag: "0.8.0-harness-wave-1-trace-wave-1"
release_commit: 5ff4f9fb6f895b532162a2bf4c6d437be7405c53
---

Two waves in one release, and they are the same idea applied twice: **take something that was prose
and make it a typed thing a program can refuse.**

{/* truncate */}

## Harness wave 1 — the plan becomes data

Planning artifacts live in the repository as markdown under `.engineering/planning/`, and are
**moved only through lifecycle-validated CLI verbs**. The consequence of choosing markdown over a
database is concrete: the diff of a status move is one line, and `git log` already knows who made
it.

A **Claude Code plugin** — one skill, two agents, no hooks at this point — teaches the store's rules
and **discovers its vocabulary at use time** rather than hard-coding it, so the plugin does not go
stale when the vocabulary grows.

And a **repeatable hermetic eval** checks the plugin's behaviour with mechanical assertions, run
metrics, and an advisory adversarial review.

## Trace wave 1 — a run becomes a judgeable object

A harness transcript normalises into a **content-addressed event IR** and is judged against a
`trace-spec/1` document: **forty-nine expectation kinds**, verdicts `ok` / `gap` / `unk`, exit codes
0 / 1 / 3.

The third value earns its place here more than anywhere: `unk` means **the adapter did not
understand the event**, which is a different thing from the expectation failing, and a checker that
reported those as failures would be unusable against any harness it did not fully model.

A passing check **mints `trace_conformance` evidence the engine admits**, produced by the
`trace-checker` verifier class and fed back through `protocol evaluate --evidence`. That is the same
join as the ESS half: an observation becomes a fact a completion rule can read.

## And a narrowing, stated

The vision's refusal of "a workflow engine" was **narrowed to admit a reference driver** — decided
and designed, not built. A boundary that moves should move in writing, with the reason attached, or
it looks later like it was never there.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
