---
format: aep.planning-md/1
id: epic:wave-derivation
kind: epic
status: active
title: A wave is derived from the store, not read from prose
summary: A typed scope on stories and an aep artifact waves verb that derives waves, names collisions and lists the unassessed, replacing pairwise prose in the wave skill.
owner: protocol
tags:
- store
- wave
relations:
- serves: vision:O2
- informed_by: story:a-story-records-where-it-lands
- informed_by: story:wave-as-a-surface
revision: 3
---
# Epic: A wave is derived from the store, not read from prose

## Outcome

A coordinator asks the store which stories can be implemented at once and gets an answer computed from declared scope and `depends_on` edges — with every story whose scope is unknown listed as unassessed rather than silently placed — so a parallel wave rests on facts the CLI checked, not on a pairwise reading of `## Scope` prose.

## Why Now

The wave skill selects on non-overlap by reading `## Scope` sections and judging pairs in prose (`agentplugins/plugins/adp/skills/wave/SKILL.md:149`, "Name the overlap risk honestly, per pair"). The comparison of 2026-09-02 against `bdfinst/agentic-dev-team` found their equivalent is a 208-line script that derives waves deterministically and exits 2 on a cycle, a missing dependency or a same-wave file collision (`scripts/plan_waves.py`), reviewed by a critic that "honors the collisions array". This repository's own store showed the cost of not having it: 24 of 40 draft stories cited no path at all when the wave skill was dry-run on 2026-08-30 (`story:a-story-records-where-it-lands`). That story argued for a `## Scope` section first and put a typed field out of scope for lack of a verb; this epic is that verb and what it enables.

## Scope

A typed `scope` on a story, set by a verb; `aep artifact waves`, which derives waves and names collisions; and the wave skill's use of it (filed in `agentplugins`, `story:wave-skill-selects-with-the-waves-verb`).

## Out of Scope

- Deciding that two overlapping stories may still run together. The verb reports the collision; the operator decides. `story:wave-as-a-surface` puts that judgement in front of the operator on purpose and this epic keeps it there.
- A `wave` artifact kind. Not earned until the verb has been used on two real waves.
- Running anything. `waves` reads and prints.

## Risks

- Scope declared at the wrong granularity (a whole crate) makes every pair collide and the verb useless. Mitigation: the verb reports collision *at the declared granularity* and says so; the scoper agent already distinguishes cited from inferred lines.
- Retroactive scoping of the existing backlog is a run of the scoper, not part of this epic, and the verb's usefulness on this store depends on it.

## Ambiguities

- `inferable` — the field lives on `story` only, per the default recorded in `story:a-story-records-where-it-lands` § Open Questions.
- `inferable` — `depends_on` is a tiebreaker rather than a filter (`story:wave-as-a-surface` § Context: 36 of 40 draft stories were dependency-ready); the verb uses it as an ordering constraint, not a readiness filter.
- `requires-stakeholder-input` — whether a story with no scope may ever appear in a proposed wave. Decides: protocol owner. Default: never; it is listed under `unassessed`.

## Done When

`aep artifact waves` on this repository's store prints at least two waves with zero collisions inside a wave, names every collision it excluded, and lists the unassessed stories; the wave skill's selection step cites its output.
