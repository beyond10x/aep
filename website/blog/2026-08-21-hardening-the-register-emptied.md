---
title: "0.6.1 — no new capability, the existing claims made mechanical"
description: >
  Three invariants that were enforced by nothing are enforced by scans. The model digest widens to
  the full SHA-256. Property tests with fixed seeds. And the fault matrix reaches thirteen rows,
  zero uncaught.
slug: hardening-the-register-emptied
tags: [release, ess, conformance]
---

A release with no new capability. Everything here takes a claim the repository was already making
and makes it **mechanical** — enforced by something that fails, rather than by a doc comment that
asserts.

{/* truncate */}

## Three invariants that were enforced by nothing

* **The engine cannot manufacture evidence** — a scan that reads the payload list **off the
  `Evidence` type itself**, so adding a payload variant cannot quietly escape the check.
* **The domain crates are provably clock- and randomness-free.**
* **The contract's one write path is pinned by name.**

Each of these was true before and true by nobody's guarantee. The pattern in all three is the same:
derive the check from the type rather than restating the type in the check.

## The digest widens

From 16 hex characters to the **full 64 of the SHA-256**, because completion decisions and suite
acceptance came to rest on it. A truncated digest is fine for a cache key and not fine for something
a gate reads.

## Property-based testing, with fixed seeds

Kleene laws over generated expressions, and the compiler property that **any document is either
refused with reasons or compiles byte-identically twice**. Fixed seeds, so a failure is a failure
somebody else can reproduce.

## The last uncaught fault, caught

The model gains `payload:` on a command outcome — which event fields the command's input determines
— and the one fault the matrix admitted was caught by nothing is now caught. **Thirteen rows, zero
uncaught.**

Value-object invariants are asserted at the view positions that hold them, closing wave 4's last
synthesis refusal **with scenarios rather than promises**. The billing suite grows 27 → 29 as a
result.

## The diff refuses to narrow on ignorance

A change in an uncompared family **invalidates the whole suite over an empty delta**, rather than
reporting a small impact it cannot justify. Reporting less impact than you can prove is worse than
reporting all of it.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
