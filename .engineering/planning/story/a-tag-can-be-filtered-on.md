---
format: aep.planning-md/1
id: story:a-tag-can-be-filtered-on
kind: story
status: draft
title: list and board filter on a tag
summary: 'Tags are writable and unqueryable: --tag exists on new and set, and no read verb takes one.'
scope:
- confidence: cited
  path: crates/protocol-cli/src/planning.rs
revision: 2
---
## Context

`tags` has been a field on every planning artifact since the markdown store existed, and nothing
queries it. `aep artifact list` filters on `--kind` and `--status`; `board` filters on `--kind`. A
label somebody wrote is therefore findable with `grep` and with nothing that knows what an artifact
is.

`--ref` (0.41.0) is the same shape and did get a filter, which is what makes the omission visible:
two set-valued frontmatter fields, one queryable.

## Acceptance

- `aep artifact list --tag <label>` returns the artifacts carrying it.
- `aep artifact board --tag <label>` does the same.
- Repeating the flag means *all of these*, not *any of these*, and `--help` says which.
- A tag nothing carries returns an empty list and exits 0 — it is a question with the answer *none*,
  not a failure.

## Evidence for the gap

`crates/protocol-cli/src/planning.rs` — the `List` variant declares `kind`, `status` and (since
0.41.0) `reference`; `select()` filters on those three. `Board` declares `kind` alone.
