---
title: "0.23.2 — the first tag this repository has verified"
description: >
  CI had been red for eleven consecutive releases behind a green local gate, because task check was
  missing exactly the two jobs CI also ran. One failure arrived through the lockfile with no source
  change at all.
slug: the-first-verified-tag
tags: [release, ci]
---

The least comfortable release note here.

**CI had been red for eleven consecutive releases** — every tag from `0.13.0` to `0.23.1` — with a
green local gate the whole time.

{/* truncate */}

## Two causes, both in the two jobs the gate did not run

| job | cause |
|---|---|
| `MSRV 1.85` | a transitive dependency raised **its own** `rust-version`, so the declared MSRV stopped holding |
| `Website` | a markdown link from a documentation page into the repository tree, which the site's build resolves |

The MSRV failure is the instructive one: it arrived **through the lockfile**. `idna_adapter@1.2.2`
pulled in `icu_*@2.3.0`, which need rustc 1.88. **No commit of ours touched a line of Rust**, and
nothing local noticed for eleven releases.

The fix was to pin `idna_adapter` to a version that uses `unicode-normalization` instead — which
also drops eight crates from the tree. The alternative was raising the declared MSRV, which would
have quietly broken the promise the README makes to anybody building this.

## The actual defect

`task check` did not run `msrv` or `website`, while **its own description said "everything CI
runs"**.

That is the whole mechanism by which a red CI survives eleven releases behind a green gate. **A gate
that covers less than the gate it claims to be is worse than one that admits its scope** — because
the second kind makes you go and look.

It runs twelve steps now, and this list and the CI workflow are checked against each other whenever
either changes.

## Why cut a release for it

No behavioural change at all. The tag exists so there is **one commit in the history whose green is
a fact rather than an assumption**, and so every tag after it can be compared to something.
