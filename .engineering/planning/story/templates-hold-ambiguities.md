---
format: aep.planning-md/3
id: story:templates-hold-ambiguities
kind: story
status: implemented
title: Templates hold an ambiguity as a classified entry
relations:
- decomposes: epic:adopter-feedback-round-2
- serves: vision:O2
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-02T17:40:43Z", actor: "human:timo", revision: 2, imported: true}
- {from: "proposed", to: "active", at: "2026-09-02T17:40:43Z", actor: "human:timo", revision: 3, imported: true}
- {from: "active", to: "implemented", at: "2026-09-02T18:18:56Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
# Story: Templates hold an ambiguity as a classified entry

## Outcome

An epic, story or specification drafted from the template has a place to record each gap the author
found, classified `inferable` (with the citation that settles it) or `requires-stakeholder-input`
(with who decides), so an undecided domain relation is a line somebody reads rather than a sentence
that gets improvised later.

## Context

The second adopter's plan was inconsistent about a relation between two entities; the templates have
`## Open Questions` for stories but no classification, and the epic and specification templates have
nowhere for a gap at all. The agent-plugins decomposer story makes the decomposer classify; this
story gives the classification a home. Derived from the epic this decomposes.

## Acceptance

- `artifacts/templates/epic.md`, `story.md` and `specification.md` gain an `## Ambiguities` section
  with guidance text stating the two classes and that a `requires-stakeholder-input` entry should
  become a `decision-blocker` with a `blocks` edge.
- The story template's `## Open Questions` guidance points at the new section rather than duplicating
  it.
- Any test or fixture that pins template bytes is updated, and `aep artifact new` seeds the new
  section.
- `CHANGELOG.md` gains an Unreleased line; `task check` passes.

## Out of Scope

Validating the section's contents (a classification is prose until a lifecycle says otherwise).

## Open Questions

None.
