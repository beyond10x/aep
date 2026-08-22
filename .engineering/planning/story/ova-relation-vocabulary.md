---
format: aep.planning-md/1
id: story:ova-relation-vocabulary
kind: story
status: draft
title: 'Relation names a relations document may use are a closed enum: state the reason or open it'
summary: The open-vocabulary audit found artifacts/relations/ open as a document and closed at the value layer, with no reason written anywhere an adopter reads. Decide which it is.
relations:
- derived_from: story:open-vocabulary-audit
revision: 1
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

## Out of Scope

Opening the enum in this story. Deciding is the work; a migration for the artifact graph, the
relation validator and every shipped `relations.yaml` is its own change.

## Open Questions

**Does an open relation vocabulary keep any of the graph guarantees?** Decides: protocol owner. The
cycle check and the target-exists check are defined over relation *kinds*; whether they survive a
free-form name is the question that decides the answer above, not a detail of it.
