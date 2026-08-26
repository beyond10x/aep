---
title: "0.2.0-wave-3 — a backend can prove it, instead of claiming it"
description: >
  Sixteen black-box conformance suites over the command and query surfaces, three levels — and a
  deliberately broken backend the suites are checked against, because a suite that passes
  everything tells you nothing.
slug: conformance-and-the-domain-profiles
tags: [release, aep, conformance]
---

Any storage backend can *say* it implements the contract. This release is about the difference
between saying and proving.

{/* truncate */}

## What landed

* **Sixteen black-box suites** over the command and query surfaces, at **three conformance levels**,
  runnable by any backend against itself.
* **A deliberately broken backend the suites are checked against.** This is the part worth copying:
  a suite that passes everything tells you nothing about whether it would *catch* anything. The
  broken backend is the suite's own test.
* **`adp-domain` and `aop-domain`** — development-specific and operations-specific vocabularies,
  so the core stays general and the specifics stay typed.
* **`protocol conformance`**, and an adopter's guide.

424 tests. The v0.2 scope is implemented.

## The idea that outlived the release

*Prove the checker before you trust the check.* The same move appears later: an oracle made to fail
on purpose, once per fault, before generated code was allowed to be judged by it; a guard that reads
a count out of the suite's own source rather than trusting prose; a self-test that runs before
release notes are generated rather than after.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
