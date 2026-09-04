---
format: aep.planning-md/1
id: epic:consumers-use-grouped-spellings
kind: epic
status: active
title: The consumer repositories use the grouped CLI spellings
summary: Migrate 469 authored aep/protocol/ess call sites across agentplugins, metaharness, atlas, harness and aep-service to the grouped area spellings, leaving recorded bytes untouched.
relations:
- derived_from: story:cli-first-level-is-the-four-areas
revision: 3
---
# Epic: The consumer repositories use the grouped CLI spellings

## Context

AEP 0.52.0 (tag `0.52.0`, commit `719380f`) moved the first level of `aep`/`protocol` under four
areas plus `doctor`; ESS 0.12.0 did the same for `ess` under four areas. Every former flat spelling
survives as a hidden alias with identical stdout, stderr and exit status, so nothing outside this
repository is broken. What is now wrong is the *teaching*: 469 authored call sites across five
repositories show a reader a spelling the `--help` page no longer offers.

Counts read at 2026-09-04, `rg -c` over authored files at each `origin/main`:

| repository | flat call sites | planning store of its own |
|---|---|---|
| agentplugins | 275 | no |
| metaharness | 138 | no |
| atlas | 36 | yes |
| harness | 15 | yes |
| aep-service | 5 | yes |

The five stories live here rather than one per repository because two of the five repositories have
no planning store at all, and a migration driven by one release is one thing to see the state of.

## Acceptance

Every authored document in the five repositories that teaches an `aep`, `protocol` or `ess`
invocation uses the grouped spelling; each repository's own gate exits 0; and every flat spelling
left behind is either a predicate matched against recorded bytes or dated history, named as such in
the story that left it.

## Notes

Removing the hidden aliases is not part of this epic. That is a later decision, and it wants a
measurement of external callers first.
