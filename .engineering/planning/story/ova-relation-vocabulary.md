---
format: aep.planning-md/1
id: story:ova-relation-vocabulary
kind: story
status: implemented
title: 'Relation names a relations document may use are a closed enum: state the reason or open it'
summary: The open-vocabulary audit found artifacts/relations/ open as a document and closed at the value layer, with no reason written anywhere an adopter reads. Decide which it is.
relations:
- derived_from: story:open-vocabulary-audit
revision: 5
---
# Story: Relation names a relations document may use

## Outcome

An adopter who writes a relation the engine does not know reads *why* that is refused, in a page they
were already going to open — or writes it and it works. Either answer is fine; the current state,
where the invitation and the refusal are in different layers and neither is explained, is not.

## Context

Raised by the open-vocabulary audit, `docs/guide/open-vocabulary.md`. The row reads:

| layer | verdict |
|---|---|
| the document — `artifacts/relations/relations.yaml` | open: an adopter declares which pairings are meant |
| the value — `RelationKind` in `crates/aep-domain/src/artifact.rs` | closed: thirteen variants, no `Other` |

The audit's own rule is that a closed vocabulary is not a defect and an unexplained one is. This row
has no guarantee written in the audit and no reason written anywhere in the published corpus, so it
left the audit **unsettled** and pointing here.

## Acceptance

- Either a stated guarantee that the closure buys, written where an adopter reads it — the vocabulary
  reference or the limitations page — or a decision recorded that the enum opens.
- The audit's row for it moves from unsettled to settled, or its verdict changes, and the suite that
  decides the audit stays green either way.

## Verdict — closed, with the reason written, 2026-08-28

`RelationKind` **stays a closed enum**, and the closure is now stated where an adopter meets
relations rather than left to be discovered by trying to invent one:
`docs/guide/adopting.md#relation-names` carries all 13 kinds, the reason, and the way to ask for a
new one.

**The reason, in the form a reader can disagree with:** a relation name is a graph semantic the
engine *interprets*, not a label it carries. `decomposes` is what builds the tree `protocol artifact
board` prints; `depends_on` is what the driver's coverage pre-flight and `validate` follow;
`supersedes` is the one relation with a lifecycle consequence attached. An open vocabulary here
would let an adopter write a name that nothing can act on — a relation that reads like a fact and
moves nothing — which is worse than a refusal that names the 13 that work.

Held by the audit's own suite: the relation vocabulary gets **one row per layer** rather than one
averaged verdict (`task:ova-layered-rows`, green), so *the document may name any relation* and *the
engine knows thirteen* stay separate claims.

Evidence: `bash .engineering/checks/run.sh` → 119 pass, 0 fail, 2026-08-28.

## Out of Scope

Opening the enum in this story. Deciding is the work; a migration for the artifact graph, the
relation validator and every shipped `relations.yaml` is its own change.

## Open Questions

**Does an open relation vocabulary keep any of the graph guarantees?** Decides: protocol owner. The
cycle check and the target-exists check are defined over relation *kinds*; whether they survive a
free-form name is the question that decides the answer above, not a detail of it.
