---
format: aep.planning-md/1
id: task:ova-audit-corpus
kind: task
status: draft
title: The audit at docs/guide/open-vocabulary.md, and the corpus it names
summary: 'R0-R2: the audit exists at the guide path, docs/guide/README.md routes to it, and the corpus list in the audit equals the set the R1 globs produce, every path resolving.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
revision: 2
---
# Task: the audit document, and the corpus it declares

## What

**R0–R2.** Create `docs/guide/open-vocabulary.md` and add one row to the *Which guide* table in
`docs/guide/README.md` pointing at it. The audit opens by naming its corpus explicitly, as a list of
paths, and states the three globs that produce it:

- `docs/guide/*.md`
- `website/docs/**/*.md`
- `docs/plan/document-authoring-brief.md`

The check re-derives the set from those globs and compares it to the list in the audit. A guide added
later makes the audit red rather than silently out of date.

The audit's location resolves the story's open question at its stated default — the guide, because
the reader is an adopter deciding what they may declare.

## Why

Every other check in this suite reads this file, and every citation check resolves a path against
this corpus. A corpus that is a sentence rather than a list is one no check can decide, and the whole
audit degrades into a document read for plausibility.

## Done When

| # | Acceptance |
|---|---|
| C1 | `docs/guide/open-vocabulary.md` exists and is non-empty. |
| C2 | `docs/guide/README.md`'s *Which guide* table gains exactly one row whose link target is `open-vocabulary.md`, and that target resolves to an existing file. |
| C3 | The audit carries a corpus section listing paths, one per line. |
| C4 | That list equals the set the three R1 globs produce — set equality in both directions, with missing and extra entries named individually. |
| C5 | Every listed path exists. (R2.) |
| C6 | The audit prints the corpus file count, and that number equals both the length of the list and the count the globs produce. |
| C7 | The audit names the three globs verbatim, so the check re-derives rather than trusting the literal list. |
| C8 | Creating a file matching `docs/guide/*.md` on a scratch copy turns C4 red, naming the unlisted file. Shown, with the real tree unchanged. |
| C9 | Nothing outside `docs/` is written by this task. |

## Notes

- The counts in the specification (5 + 26 + 1 = 32) are the values at the commit the round was taken
  at. C6 compares against the glob at check time, not against those literals — a count typed into a
  check is a second source of truth and the one that goes stale.
- `website/docs/**/*.md` is in the corpus while `website/` remains outside the **write** surface:
  the audit reads the published site and does not edit it. That distinction is what keeps the
  constraint and the corpus from contradicting each other.
- The audit is a document, not a generated file. `scan-declarations.sh` never writes it.

## Verifier

`.engineering/checks/check-corpus.sh`. C1–C9 are its rows.

C4 is the load-bearing one: it expands the globs at check time and diffs both directions. C8 runs it
against a copy of the tree under `${TMPDIR:-$HOME/.cache/claude-tmp}` with one file added.
