---
format: aep.planning-md/1
id: story:landing-page-states-what-is-true
kind: story
status: implemented
title: The landing page's status panel says three things that are not true
summary: HonestStatus publishes a hand-written suite/test count the repository forbids, a 26-tag-stale tag, and 'there is no durable backend' — false since 0.27.0.
owner: docs
tags:
- docs
relations:
- serves: vision:O2
- informed_by: story:website-version-stamps-are-generated
revision: 4
---
# Story: The landing page's status panel says three things that are not true

## Outcome

A stranger's first page about this project stops telling them there is no durable backend. There
have been two since `0.27.0` — `website/docs/status/limitations.md:34` says so, two clicks away on
the same site.

## Context

Found while wiring `cargo xtask status` to own the page's release chip
(`story:website-version-stamps-are-generated`). The chip was the stale thing anyone would notice;
the paragraph under it is the one that misleads.

`website/src/pages/index.tsx`, the `HonestStatus` panel, as of 2026-08-30:

| claim | state |
|---|---|
| "**106 suites and 1811 tests**, with 0 clippy warnings and 0 rustdoc warnings" | a hand-written count. `task check` at 0.33.0 measured 191 suites and 2883 tests |
| "as of the tag `0.7.1-infra-waves-1-4`" | 26 tags stale; now a generated region |
| "There is no durable backend" | false since `0.27.0`; markdown, SQLite, Postgres and hybrid all implement the contract |

The count is not merely stale, it is **forbidden**. `docs/status.md` and
`website/docs/status/where-this-stands.md` both state the rule in their own words: this repository
publishes no hand-written suite or test count, because four of them drifted apart within its first
48 hours. The landing page is the one surface that never got the memo, and it is the surface with
the most readers.

The panel is named `HonestStatus`.

## Acceptance

- The suite and test counts leave the page. What replaces them is the claim the gate can actually
  support without a number that rots: that the gate is what measures, and where to run it.
- "There is no durable backend" is replaced by what is true, with the same bluntness the panel has
  everywhere else — the panel's value is that it lists what is *not* done, so the replacement names
  a real remaining limit rather than deleting the sentence.
- The tag in the prose comes from the generated chip rather than being typed a second time.
- Nothing else in the panel is softened. The three remaining limits — no generated behaviour, no
  team governed by this yet, no identity bound to evidence — are unchanged and still first-person.
