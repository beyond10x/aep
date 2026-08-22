---
format: aep.planning-md/1
id: story:ova-predicate-operator-vocabulary
kind: story
status: draft
title: 'Predicate operators in mapping form are fixed and unexplained: state the reason or open it'
summary: The open-vocabulary audit found the mapping-form operator set closed in the engine with no guarantee stated and no adopter-facing reason. Decide which it is.
relations:
- derived_from: story:open-vocabulary-audit
revision: 1
---
# Story: Predicate operators in mapping form

## Outcome

An adopter who needs a comparison the operator set does not carry — a substring match, a set
intersection, a regular expression — finds out from the documentation whether that is a boundary or
an omission, instead of finding out from a validation error.

## Context

Raised by the open-vocabulary audit, `docs/guide/open-vocabulary.md`. The operator set is published
as a flat list in the vocabulary reference and is fixed in the engine; unlike every other row in the
table, there is no document key anywhere under `protocols/` that names it, so an adopter has nothing
to extend even in their own tree.

The audit left it **unsettled** for the stronger of the two reasons: not only is no reason written
for adopters, no guarantee is claimed for the closure either. That is the shape the story
`story:open-vocabulary-audit` exists to stop passing unremarked.

## Acceptance

- A stated guarantee for the fixed operator set, written where an adopter reads it, or a recorded
  decision that predicates gain an extension point.
- The audit's row moves from unsettled to settled, or its verdict changes, and the suite that decides
  the audit stays green either way.

## Out of Scope

Adding an operator. The question here is whether the set is a boundary; which operator is missing is
a separate argument, and answering it first would settle this one by accident.

## Open Questions

**Is a fixed operator set what makes a predicate decidable, or only what makes it convenient?**
Decides: protocol owner. The three-valued evaluation and the scale comparison both depend on knowing
what an operator means; whether that survives an adopter-declared one is the whole question.
