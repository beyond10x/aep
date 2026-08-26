---
format: aep.planning-md/1
id: story:changelog-claims-are-checked
kind: story
status: implemented
title: A Fixed entry names a release the defect was actually in
summary: A `### Fixed` bullet describes something a user hit in a shipped version; the gate checks the claim against the history rather than taking it.
relations:
- decomposes: epic:evidence-gated-completion
revision: 4
---
# Story: A `### Fixed` entry names a release the defect was actually in

## Outcome

A reader of `CHANGELOG.md` can trust that everything under `### Fixed` was once broken for them. An
entry describing a defect that never shipped gets caught by the gate rather than by a reviewer three
weeks later.

## Context

0.6.0 of `entity-runtime` shipped three `### Fixed` entries. **Two described defects that never
existed in a release.** One named a requirement-numbering scheme, `R-90b`, that appears nowhere in
that repository's history except in the changelog entry claiming it was wrong; the other described a
`serde` defect in a file that did not exist before the release it was claimed to be fixed in.

Both were real — they were caught while the wave was being built. The tag message said so correctly:
*"three defects the wave's own tests caught on first run"*. The `### Fixed` heading did not, and
`### Fixed` is a promise about what a user of the previous version experienced.

## Acceptance

A gate step reads each released `### Fixed` bullet and checks that the identifiers it names —
requirement ids, file paths, symbol names — existed at the **previous** release tag; a bullet naming
something that did not exist yet is a finding, with the tag it was checked against. A bullet naming
nothing checkable is reported as unverifiable rather than passed, so the count of what was checked
is honest.

## Out of Scope

Prose. Nothing here reads English or judges whether a description is fair; it checks that the things
a bullet names were there to be broken.

## Open Questions

Where a defect caught during a wave belongs, if not `### Fixed`. Decides: whoever owns the changelog
convention. Default if nobody answers: **`### Changed`, or the commit message** — the tag message
already carries "what this wave's own tests caught", and that is the honest home for it.
