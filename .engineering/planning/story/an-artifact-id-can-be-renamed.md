---
format: aep.planning-md/3
id: story:an-artifact-id-can-be-renamed
kind: story
status: active
title: An artifact id can be renamed through the store's write path
refs:
- provider: github
  reference: beyond10x/aep#152
relations:
- decomposes: epic:zero-open-issues-2026-11-09
- depends_on: story:relate-and-unrelate-write-the-revision-they-print
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/aep-cli/src/planning/git_record.rs
- confidence: inferred
  path: crates/edge/aep-cli/src/planning/rename.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/rename.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:42Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:43Z", actor: "human:timo", revision: 8}
---
# Story: an artifact id can be renamed through the store's write path

## Outcome

Somebody whose store holds an artifact id that must disappear renames it with one command, and the
store stays valid: no hand `git rm`, no edge left pointing at the old id, and a record that a
rename happened.

## Context

GitHub issue beyond10x/aep#152, reported from an adopter's store on aep 0.79.0:

- `aep plan artifact set` refuses the `id` field (`aep plan artifact set --help`: "`id` and
  `kind`, which are the artifact's identity. Each is refused by name").
- `aep plan artifact redact` changes title, summary and body, and writes its evidence under the
  artifact id, `.engineering/evidence/<kind>/<id>/`, so the id itself stays in two paths.
- So an id carrying a name that may not appear in the store can leave only by a hand `git rm` of
  the artifact file and its evidence directory.

The issue asks for either a `rename` verb or a `retire` verb. `rename` is taken (coordinator's
decision at filing): it keeps the artifact, its edges and its evidence, where `retire` would delete
records the `AGENTS.md` invariant *Audit is append-only* keeps.

## Acceptance

On an `aep.project/5` store, `aep plan artifact rename <kind>:<old> <new-name> --reason <text>`
exits 0 and afterwards: the artifact file is at `<kind>/<new-name>.md` with `id: <kind>:<new-name>`
and its transitions, relations and body unchanged; every relation in any other artifact that named
the old id names the new one; the evidence records are under
`.engineering/evidence/<kind>/<new-name>/`; a search for the old name over `.engineering/` finds it
only where a title, summary or body wrote it as prose (which `redact` owns); one record of the
rename exists that holds the old id's SHA-256, the new id and the reason, and never the old id;
`aep plan artifact history <kind>:<new-name>` lists the rename; and `aep plan artifact validate`
prints `valid`, before and after the change is committed.

It is refused, changing nothing, when the new id exists, the new name is not a legal name, the
reason is blank or holds the old name, or the store is SQLite or Postgres (named as `redact` names
them).

## Out of scope

- Changing an artifact's kind.
- Rewriting prose that mentions the old id; `aep plan artifact redact` does that.
- A `retire` verb.

## Notes for the implementor

- `website/docs/reference/cli.md` must list the verb (`docs-check`).
- Evidence files are immutable and `validate` compares them with their committed Git blob; a moved
  record must still be admitted. `crates/edge/aep-cli/src/planning/record_redaction.rs` and
  `git_record.rs` show how `redact` is admitted.
- A rename writes several files. It is one command under the planning writer lock, and a refusal or
  a failure part-way leaves every file as it was (invariant *Refusals change nothing*).
