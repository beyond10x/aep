---
format: aep.planning-md/1
id: story:wave-as-a-surface
kind: story
status: draft
title: A wave is a surface, so that it can become a document
summary: The wave procedure existed only in git log. It ships as a skill first because the surface is how the shape is found; the YAML is what the shape becomes.
owner: plugin
tags:
- plugin
- wave
relations:
- decomposes: epic:self-evaluation
- informed_by: story:implementor-and-adversary-agents
revision: 1
---
# Story: A wave is a surface, so that it can become a document

## Outcome

An operator says *pick the next wave*, reads a proposal naming what would run at once and why,
approves it, and watches N stories implemented in parallel and closed on one gate run — with the
selection, the branch topology and the evidence rule written down instead of rediscovered.

## Context

**The procedure already existed, in `git log` and nowhere else.** `5b6259b` is *"pick the next wave
of work"*, written out: five stories chosen from 44 for being *"implementable in this tree without
a credential, a paid run or a second harness"* and for *"touching surfaces that do not overlap"*,
each on its own `impl/<slug>` branch off one base, merged serially into `wave/five`, gated **once**
on the merged result, and closed by one `test_result` against that merge (`c203308`). `AGENTS.md`
§ Commits said nothing about any of it.

**The specification has no concept above one unit of work.** `adp/default` governs a single task
from `receive` to `complete`. Nothing in `artifacts/`, `workflows/` or `protocols/` says *these N
run at once*. A wave today is three untyped things — a `docs/plan/` page, an annotated tag, and a
branch convention — which is why this ships as a skill first: the surface is how the shape gets
found, and the YAML is what the shape becomes.

**Three facts the selection step has to face, measured 2026-08-30 against this store:**

- **36 of 40 draft stories are dependency-ready.** `depends_on` is a tiebreaker, not a filter, so
  selection is judgement and belongs in front of the operator rather than in a query.
- **24 of 40 draft stories cite no `crates/` path at all**; of the 16 that do, 9 name `aep-domain`
  and 8 name `protocol-cli`. Disjointness cannot be computed, so the skill reports *blast radius
  unknown* rather than assuming safety it has not established.
- **1 of 40 draft stories carries a `serves:` edge.** Every story the wave moves out of `draft`
  needs one or `protocol artifact validate` goes red, so the edge is added as part of the proposal
  and the objective is shown to the operator before anything runs.

**Why the coordinator owns every store write.** `.engineering/planning/journal.jsonl` is committed,
append-only, has no `.gitattributes` and no merge driver. N branches each moving their own story
each append to the tail, and the textual merge yields a document whose `revision` no event supports
— which `validate` reports as forgery. Implementors touching only `crates/**` makes it impossible;
`implementor.md` already forbids the write.

**Why a skill and not an agent.** A Claude Code sub-agent cannot ask the operator anything. Only the
main session can pause for approval, hold the wave across turns, and hand a red result back to the
implementor that still holds its context. The stop between proposing and running is the whole point,
so it has to live where a stop is possible.

**What it does not do.** It evaluates no gate. Green is `task check`'s exit status, and the skill
says so in the same words the driver does: sequencing is the coordinator's, deciding is not.

## Acceptance

`protocol artifact validate` exits 0 with the wave's stories moved and their evidence recorded
against the merge commit, after a wave run end to end from a proposal the operator approved.

## Out of Scope

- **The YAML formalization.** Deliberately after, not instead: writing a vocabulary for a procedure
  nobody has run twice is the thing `crates/ess-domain/src/actor.rs:24-33` refuses when it declines
  to ship a `RoleSpec` before the evidence that made it necessary. The four questions a run has to
  answer are in the wave page's own section.
- **A `wave` artifact kind, or a `protocol artifact ready` verb.** Both are plausible and neither is
  earned yet. The skill composes readiness from `list` and `graph`.
- **Anything under `protocol drive`.** This wave runs interactive sub-agents. Driving N runs needs N
  project directories and is `story:confined-driven-workspace`.
- **A merge driver for the journal.** Unnecessary while implementors never write to the store. If
  that ever changes it becomes mandatory, and this paragraph is the reason.

## Open Questions

**Is N bounded by a measured number or a configured one?** Decides: protocol owner. Default if
nobody answers: **measured** — the pre-flight measures one worktree's package-scoped build and the
free disk, and refuses below a floor, because a wave of five that fills the disk at agent four has
destroyed four agents' work and produced nothing. A configured N that nobody re-measures is the
number that was true on the day it was written.
