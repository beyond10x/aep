---
format: aep.planning-md/1
id: story:validate-strict-refuses-what-it-reports
kind: story
status: implemented
title: '`validate --strict` refuses what `validate` only reports, and `move` refuses what `validate` would'
summary: validate prints assertion-closed stories and then valid; move --to active succeeds where validate then reports empty_declaration; body --from - on empty stdin writes an empty body.
tags:
- cli
- store
relations:
- decomposes: epic:evidence-gated-completion
- serves: vision:O2
- informed_by: story:completion-needs-evidence
revision: 4
---
# Story: `validate --strict` refuses what `validate` only reports, and `move` refuses what `validate` would

## Outcome

A gate can make the evidence gate bite. `validate` keeps reporting assertions without failing — that
is `story:completion-needs-evidence`'s recorded position — and `--strict` turns each reported class into
an exit 1 for the caller that wants it. A move that would leave the store invalid is refused at the move.

## Context

- `ed007513#1300`: four stories reached `implemented` on an assertion (a swallowed `--reference` typo);
  `validate` printed the four bullets and then `valid`.
- `9da4f51c#464` vs `#3286`: 37 runs, always `valid`, while 19 stories contradicted their own bodies.
- `e70b8018 s1#1866`: 78 runs, 78 × `valid`; a `plan-reviewer` pass found 8 findings.
- `114c2340#92`, `4d4c15a4#149`: `move --to active` succeeds, `validate` then reports
  `[empty_declaration] … is active and serves no objective` — `validate_grounding`
  (`crates/aep-domain/src/artifact.rs:2318`) runs only in `validate`.
- `11727595#10819`: `body --from -` on empty stdin wrote an empty body and bumped the revision.
- `ed007513#2767`: `new --relate` then `body --from` lands a new artifact at revision 3.

## Acceptance

- `protocol artifact validate --strict` exits 1 when any artifact is closed on an assertion, when any
  document predates the event log, or when drift is reported. Without the flag, output and exit are unchanged.
- `move` runs the graph rules on the would-be store and refuses a move that would add a finding, printing
  the finding's own text and hint (the `serves` case first).
- `body --from -` with an empty body is refused, naming the flag; `body --from <file>` on an empty file too.
- `plan-check` in `Taskfile.yml` stays non-strict; the story that flips it is `story:completion-needs-evidence`.

## Out of Scope

Verifying that a cited test run happened.
