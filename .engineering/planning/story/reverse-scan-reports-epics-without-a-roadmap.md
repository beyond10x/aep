---
format: aep.planning-md/3
id: story:reverse-scan-reports-epics-without-a-roadmap
kind: story
status: draft
title: reverse scan reports a store with epics and no roadmap
tags:
- requested-by-sdc
relations:
- serves: vision:O2
- depends_on: story:roadmap-kind
revision: 1
---
# Story: reverse-scan-reports-epics-without-a-roadmap

## Problem

A repository adopting AEP with epics and no roadmap has nothing that says so. The `sdc` direction
stage needs an approved roadmap before any epic opens (`sdc/docs/orchestration.md` § 4.3 and § 4.4
gap 1), so a brownfield store is blocked until somebody notices and drafts one.

`aep plan reverse scan` reads and does not interpret (`crates/edge/aep-cli/src/reverse.rs` module
documentation), does not read the planning store, and proposes no artifact per kind or per profile.
Proposing a draft roadmap is therefore new behaviour for the scan, not a kind added to an existing
proposal list, and is split out of `story:roadmap-kind`.

## Outcome

`aep plan reverse scan` on a repository whose planning store holds epics and no `roadmap` reports a
located fact naming those epics (id, status, file), so the `aep:reverse-engineer` agent can draft a
roadmap through `aep plan artifact new roadmap` citing it. The scan still writes nothing.

## Acceptance

- On a store with one `active` epic and no roadmap, the bundle carries one entry naming the epic and
  the absence of a roadmap; on a store with a roadmap, it carries none.
- Two runs over one tree produce identical stdout.

## Out of scope

- Writing the roadmap: the agent does that, in `agentplugins`.
