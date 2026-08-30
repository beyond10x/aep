---
format: aep.planning-md/1
id: story:body-edits-have-a-verb
kind: story
status: implemented
title: Editing part of a body, or one frontmatter field, has a verb
summary: 'Every hand edit of the store in five sessions happened because nothing appends to a body, replaces a section or sets a title: body --append/--section, show --body-only, set.'
tags:
- cli
- store
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
- informed_by: story:partial-edits-cost-more-replay-than-they-save
revision: 4
---
# Story: Editing part of a body, or one frontmatter field, has a verb

## Outcome

Appending a section, replacing one section, and changing a title, summary, owner or tag are each one
`protocol artifact` command, so the reason every hand edit of the store happened is gone.

## Context

Five sessions (`SYNTHESIS.md` CL-2). `body --from` replaces the whole body; nothing edits a field:

- `11727595#818-#850`: a python patch of `revision: 2` → `3` caught as drift, then `edit`/`update`/`set`/`write`
  each `unrecognized subcommand`; three ad-hoc frontmatter splitters (`s.split('\n---\n',1)`) across sub-agents.
- `cc946bc3#427`: four `## Scope` sections added by python frontmatter-stripping plus whole-body rewrite.
- `9da4f51c#3310`: `cat >> .engineering/planning/story/….md` — skipped the journal.
- `ed007513#209-#274`: ~25 turns of heredoc writes; no verb changes title/summary/owner (`git show 0.27.3:crates/protocol-cli/src/planning.rs`).

`story:partial-edits-cost-more-replay-than-they-save` argues whole-body writes for a *driven* step; that
is about replay cost inside a run, not about the verbs an interactive session has.

## Acceptance

- `protocol artifact body <id> --append --from <path|->` appends; `--section "<heading>" --from …`
  replaces the section under that heading (or adds it at the end); both journal one `update`.
- `protocol artifact show <id> --body-only` prints the body bytes and nothing else.
- `protocol artifact set <id> [--title …] [--summary …] [--owner …] [--tag …] [--untag …]` changes
  frontmatter through the same command path as `body`; `status`, `revision`, `id`, `kind` are refused.
- `website/docs/reference/cli.md` Planning surface documents each; `docs-check` passes.

## Out of Scope

Editing `relations` — `story:one-spelling-for-an-edge`.
