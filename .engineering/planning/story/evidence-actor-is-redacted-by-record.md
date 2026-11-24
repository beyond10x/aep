---
format: aep.planning-md/3
id: story:evidence-actor-is-redacted-by-record
kind: story
status: draft
title: An actor already committed in evidence records can be redacted by record
relations:
- depends_on: story:journal-actor-from-configuration
revision: 1
---
# Story: an actor already committed in evidence records can be redacted by record

## Goal

A store that ran without `AEP_ACTOR` holds the operator's login as `human:<login>` in the
`actor` of every evidence record and transition it wrote. `story:journal-actor-from-configuration`
stops new records from carrying it; nothing removes it from the records already committed.

- `aep plan artifact redact` replaces a literal in an artifact's title, summary and body only.
- `validate` refuses any change to a committed evidence file ("evidence is written once and never
  rewritten; record a new file instead").
- Transition actors in an artifact's frontmatter can be edited by hand and `validate` accepts the
  edit, which is a hole of its own: the edit leaves no record.

Measured on 2026-11-24 in one public repository: 238 occurrences of `human:<login>` in 214 files
under `.engineering/`, 200 of them in evidence JSON files that cannot be changed by any verb. The
operator's only options were to keep the login public or to delete evidence.

## Acceptance

- `aep plan artifact redact <id> --actor <old> --with <new> --reason <text>` replaces the actor in
  every transition of the artifact and in every evidence record under
  `.engineering/evidence/<kind>/<slug>/`, and writes one redaction record per artifact holding the
  SHA-256 of the old actor, never the actor itself, the replacement, the count and the reason.
- `--all` applies the same replacement to every artifact of the store in one run.
- `validate` admits a changed evidence file only when replaying the redaction records over its
  first committed version reproduces it byte for byte; any other change is still refused.
- `validate` refuses a hand-edited transition actor that no redaction record explains.
- Red first: a store with a `human:<login>` evidence record; `redact --actor` makes `validate`
  pass with the record changed, and a hand edit of the same field fails `validate`.
