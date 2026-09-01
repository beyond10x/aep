---
title: Roadmap and proposals
sidebar_position: 3
description: Where delivery, accepted work, open gaps and proposal status are authoritatively recorded.
---

# Roadmap and proposals

This page does not copy the live roadmap. The repository separates four questions so a prose table
cannot quietly become a second planning system:

| Question | Authoritative surface |
|---|---|
| What shipped? | [Annotated-tag status](https://github.com/beyond10x/aep/blob/main/docs/status.md) |
| What work is accepted, and where is it in its lifecycle? | [Planning store](https://github.com/beyond10x/aep/tree/main/.engineering/planning) |
| What remains open, and what closes it? | [Gap register](https://github.com/beyond10x/aep/blob/main/docs/plan/gap-register.md) |
| Which designs are still proposals? | [Vision, “Proposed, not accepted”](https://github.com/beyond10x/aep/blob/main/docs/VISION.md#proposed-not-accepted) |

A design file is not a work order, however new or detailed it is. A plan page or planning-store
story accepts work; an annotated tag records delivery. `cargo xtask status --check` keeps the
reader-facing release stamps derived from those sources.
