---
format: aep.planning-md/1
id: story:cross-repo-relations
kind: story
status: implemented
title: A relation whose target lives in another member
summary: blocks, depends_on and derived_from already exist as a vocabulary; what is new is that the target resolves elsewhere - and that an unresolvable one is a typed fact, not an error, because a member nobody has checked out is a normal condition rather than a broken plan.
relations:
- decomposes: epic:one-cli-many-repositories
- depends_on: story:assemble-across-sources
revision: 4
---
# Story: A relation whose target lives in another member

## Outcome

A story blocked by work in another repository says so in its own frontmatter, and anybody reading
the board sees the edge — instead of discovering it when the blocked work is picked up.

## Context

`blocks`, `depends_on` and `derived_from` already exist as a vocabulary; nothing about the relation
names changes. What is new is that the target resolves somewhere else, and that failing to resolve
it is a normal condition: people check out different subsets of a workspace, and a plan that was
"broken" because a colleague's repository is missing from your disk would be a plan nobody could
read.

## Acceptance

A relation target may name another member; it resolves through the assembly's index; an
unresolvable target is a **typed fact** carried in the report, not an error and not a silent drop;
`protocol workspace crossings` lists every relation whose two ends are in different members, naming
both; and cycle detection still holds over the assembled graph, including a cycle that exists only
once two members are read together.

## Out of Scope

Writing a relation into another member's store. `relate` still writes locally only.

## Open Questions

None outstanding.
