---
format: aep.planning-md/1
id: story:skill-text-cannot-instruct-a-direct-store-write
kind: story
status: draft
title: A skill that instructs a direct store write fails the build
summary: The prohibition on hand-editing planning artifacts is guarded by a test that reads each installed skill's text, not its path.
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
revision: 1
---
# Story: a skill that instructs a direct store write fails the build

## Outcome

A skill file that tells an agent to patch a planning artifact by hand cannot reach `main`. The guard
reads the skill's **text**, not its path.

## Context

`story:adopter-schema-contract-tooling` made `protocol artifact body --from <path|->` the sole
body-mutation path and rewrote both installed planning skills to say so
(`integrations/claude-code/skills/planning/SKILL.md:54`, the codex skill's `:53`). Its last
Acceptance line asked for a test per defect, and eight of nine defects have one.

*Direct store-write instruction* does not. Nothing in this repository reads a `SKILL.md`'s content:
the only code that touches one joins its path and asserts it exists
(`crates/protocol-cli/src/drive.rs:7747`). The prohibition is prose guarded by nothing, which is the
same shape as the defect it was written to fix — the skill previously **told** agents to patch bodies
directly, and no test noticed.

The cost is asymmetric. A skill regressing to "edit the frontmatter directly" ships green, installs
into every adopter, and is discovered only when a store's revision counter stops matching its
history.

## Acceptance

- A test enumerates every installed skill under `integrations/` and refuses one whose text instructs
  a direct write to a planning-store file — editing frontmatter, patching a body, or writing
  `status:` by hand.
- The check is over content, not over a path or a byte count, and it names the file and the offending
  line when it fails.
- Planted on a copy of a real skill, the instruction is caught; the shipped skills pass unmodified.
- The pattern set is written where a reader can extend it, and adding a phrase does not require
  touching the test's control flow.

## Out of Scope

- Grading a skill's prose quality, or any check that a skill teaches the *right* thing beyond this
  one prohibition.
- Validating skills a third party installs outside `integrations/`.
- Reading `SKILL.md` at run time to enforce behaviour. This is a build-time guard on what ships.

## Notes

Derived from the 2026-08-30 audit of active artifacts; see
`story:adopter-schema-contract-tooling` § *Closed on evidence — 2026-08-30* for the enumeration of
which eight of nine lines held.
