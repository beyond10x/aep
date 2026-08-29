---
format: aep.planning-md/1
id: story:cli-reference-covers-every-verb
kind: story
status: implemented
title: The CLI reference is held against the CLI, so a verb cannot ship undocumented
summary: A gate step walks the binary's --help tree and fails on any verb absent from website/docs/reference/cli.md; 7 of 77 are absent today.
owner: docs
tags:
- docs
- gate
relations:
- serves: vision:O2
revision: 4
---
# Story: The CLI reference is held against the CLI

## Outcome

Somebody reading `website/docs/reference/cli.md` for `protocol workspace` finds it. Today they find
nothing: the four `workspace` verbs shipped in 0.25.0/0.26.0 — `website/docs/status/roadmap.md:51`
says so on the site itself — and have never had a reference entry, eight releases later.

## Context

The gate's only website step is `npm run build` (`Taskfile.yml:55-63`). Docusaurus resolves every
markdown link at build time, so a page pointing at nothing fails the build; a page *describing* a
CLI that has moved underneath it builds green for ever. Link integrity is not claim integrity.

Measured 2026-08-30 at 0.33.0, by walking `protocol … --help`: 7 of 77 verbs appear nowhere in
`website/docs/reference/cli.md`.

| absent from the reference |
|---|
| `protocol workspace list` |
| `protocol workspace crossings` |
| `protocol workspace show` |
| `protocol workspace members` |
| `protocol artifact divergences` |
| `protocol artifact catch-up` |
| `protocol property evidence` |

This is the failure `lab-check` was written for — a copy of a generated thing, drifting silently —
and it takes the same answer: derive the claim rather than type it. `docs/status.md` § *the tags are
the record* makes the argument once already, for a different surface.

## Acceptance

- A gate step enumerates every verb from the built binary's own `--help` tree and fails naming each
  one absent from `website/docs/reference/cli.md`.
- The step is in `task check`, not only in the Website workflow. `Taskfile.yml:136-142` records the
  two releases lost to a check that only CI ran.
- The seven verbs above have entries, so the step is green at the commit that introduces it.
- A verb added with no reference entry turns the gate red, proven by a test that adds one.
