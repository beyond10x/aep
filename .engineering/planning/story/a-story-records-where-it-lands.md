---
format: aep.planning-md/1
id: story:a-story-records-where-it-lands
kind: story
status: draft
title: A story records where it lands, so a wave can be sequenced
summary: 24 of 40 draft stories cite no source path, so a wave's disjointness claim is an assertion. Found dry-running the wave skill's selection step, 2026-08-30.
owner: protocol
tags:
- store
- wave
relations:
- informed_by: story:wave-as-a-surface
- decomposes: epic:self-evaluation
scope:
- confidence: cited
  path: artifacts/kinds
- confidence: inferred
  path: crates/edge/protocol-cli/src/planning.rs
- confidence: inferred
  path: crates/govern/aep-domain/src/artifact.rs
revision: 5
---
# Story: A story records where it lands, so a wave can be sequenced

## Outcome

Somebody selecting several stories to implement at once can tell, from the store, which of them
touch the same surface — and the ones that cannot answer say so, instead of reading as safe.

## Context

**Found 2026-08-30 while dry-running the wave skill's selection step on this repository's own
store.** Of 40 draft stories, **24 cite no source path at all**. Of the 16 that do, 9 name
`aep-domain` and 8 name `protocol-cli`. So the property a concurrent wave rests on — that its units
touch different surfaces — is not derivable from the store for 60% of it, and is a collision risk
for most of the rest.

**An unassessed story reads exactly like a safe one.** That is the defect. A selection step that
cannot establish a radius has two honest options — establish it, or leave the story out — and the
failure mode is the third: assume it is fine, dispatch N agents, and discover the overlap at merge
time with all of their work already spent. The wave page that found this excluded
`story:evidence-subject-binding` on exactly these grounds, and had to mark two of its own three
units *inferred* rather than *cited*.

**Half of the answer already shipped.** `integrations/claude-code/agents/story-scoper.md` derives
the section for one artifact and marks every line `cited` or `inferred`, read-only so that many run
at once. What it does not do is make the section expected: a story without one is not flagged by
anything, so the gap closes only where somebody remembers to run the scoper.

**Why `inferred` is a first-class value here and not a weasel word.** A scope that mixes what was
read with what was guessed is worse than no scope, because it is trusted exactly where it is
weakest. Separating them is what lets a selection step say *this wave's disjointness rests on two
cited surfaces and one inferred one* — which is a sentence an operator can act on, and *these three
do not overlap* is not.

## Acceptance

`artifacts/kinds/story.yaml` declares a `Scope` section, and `protocol artifact validate` reports —
without failing — every non-draft story that has none, so the gap is a list somebody can work
through rather than a silence.

## Out of Scope

- **Failing the gate on a missing scope.** 24 stories would go red on the day it landed, which is
  how a check gets muted. Report first; `story:advisory-enforcement-tier` is the argument for what
  the reporting tier should be.
- **A machine-readable scope field in frontmatter.** A `## Scope` section is writable through
  `protocol artifact body` today; a typed field needs a CLI verb to set it, since `artifact` has
  `new`, `move`, `relate` and `body` and nothing that edits one frontmatter key.
- **Scoping the 24 retroactively.** That is a run of the scoper, not a change to the store's rules,
  and it wants doing after the section exists to put the answers in.
- **Deciding disjointness automatically.** The store records surfaces; whether two surfaces may be
  worked at once is a judgement, and the wave skill puts it in front of the operator on purpose.

## Open Questions

**Does `Scope` belong on `task` as well as `story`?** Decides: protocol owner. Default if nobody
answers: **story only, for now.** A task is already the decomposed unit and usually inherits its
story's surface; adding a required section to both doubles the retroactive work for a property only
one of them is selected on. Revisit when a wave is ever assembled from tasks rather than stories.
