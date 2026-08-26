---
format: aep.planning-md/1
id: epic:one-cli-many-repositories
kind: epic
status: implemented
title: One CLI across repositories, with dependencies
summary: 'Every repository is an island: protocol artifact reads one store. A story here blocked by a story in another repository cannot say so, and the limitations page names the gap - no federated artifact graphs across repositories. Needs no provider, no database and no network: every store involved is already markdown in a git checkout.'
revision: 4
---
# Epic: One CLI across repositories, with dependencies

## Outcome

Somebody planning work that spans two repositories can see it as one plan. A story here that is
blocked by a story in `entity-runtime` says so in its own frontmatter, one `board` shows the work
rather than three, and a reference that means two different things in two repositories is refused
by name instead of resolved to whichever store was read first.

## Why Now

Every repository is an island: `protocol artifact` reads one store, and the limitations page names
the gap in as many words — *no federated artifact graphs across repositories*. The cost is already
being paid: this repository and `entity-runtime` block each other regularly, and each of those
edges currently lives in somebody's head, where the first anybody hears of it is when the blocked
work is picked up. It is also unusually cheap right now — every store involved is markdown in a git
checkout, so this needs no provider, no database and no network.

## Scope

The workspace manifest and the locator it reuses; identity across members and the refusal for an
ambiguous one; relations whose target resolves in another member; the assembled graph; and the verb
surface over it — `list`, `crossings`, `show`, `members` — with every refusal naming the member that
refused.

## Out of Scope

Writing to another member. Every verb here reads; a command that edited a neighbouring repository
would need a permission model, a lock and a review path, none of which exist. Also out: renaming or
rewriting any id on the way into the assembly — membership is carried beside the id, never folded
into it.

## Risks

The identity decision is the one that is expensive to change later, because every reference written
under the wrong spelling has to be rewritten. It is taken once, in `story:namespaced-identity`, and
the refusal for an ambiguous reference is what stops the wrong answer being given quietly in the
meantime. The second risk is a cycle that exists only once two members are read together —
`find_cycle` has never seen a graph it did not fully hold.

## Done When

`protocol workspace list`, `crossings`, `show` and `members` answer across members; an ambiguous
reference is refused by name; an unresolved member is reported and exits `0`; and a member that
failed to load is reported with its failures rather than silently skipped.
