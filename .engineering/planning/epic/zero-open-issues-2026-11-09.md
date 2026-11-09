---
format: aep.planning-md/3
id: epic:zero-open-issues-2026-11-09
kind: epic
status: active
title: Zero open issues and zero open defects, 2026-11-09
summary: Every open GitHub issue and every defect still open in this store, delivered as one pull request.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:25Z", actor: "human:timo", revision: 4}
---
# Epic: Zero open issues and zero open defects, 2026-11-09

## Outcome

Every GitHub issue open on `beyond10x/aep` on 2026-11-09 is closed by a merged change or by recorded
proof, and every non-terminal story or task in this store that describes a defect present in
today's code is implemented, or is closed with the proof that its defect is fixed or its surface is
gone. One pull request carries all of it.

## Why

Operator goal, 2026-11-09, verbatim: "zero remaining issues on github, all bugs fixed from planning
store". Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but in the end
provide 1 single PR to gh. make sure all are below 1 big epic for integration via bottoken from
./atlas".

## What is in it

Open issues (`gh issue list -R beyond10x/aep`, 2026-11-09): #152 and #153. Each has one story here
carrying the issue as an external ref.

Defects in the store were found by reading every draft story and task (94) against the source at
`824869fec`; three read-only triage passes returned a verdict with `file:line` for each
defect-like one. The stories under this epic are the ones whose defect is still present. The
stories whose defect is fixed on main or whose surface was deleted are related to this epic too and
are archived with that proof in their body, not implemented.

## Not in it

- A release: no tag, no version bump.
- Feature requests and decisions among the drafts; they stay in the backlog.
- `story:a-snapshot-parent-its-group-can-write-is-not-trusted-silently`: still present, and its own
  acceptance leaves open which parent directories are checked and how a per-user group is told from
  a shared one. That is a trust decision; `decision-blocker:which-parent-directories-a-snapshot-trusts`
  records it.
- `story:checks-map-trace-spec-path`: which file the checks map reads as its trace specification is
  undecided; `decision-blocker:how-the-checks-map-finds-its-trace-specification` records it.
- `story:two-adapters-two-paths`, `story:a-dropped-stream-does-not-end-a-run`,
  `story:cold-start-outlives-the-deadline` and `story:wave-skill-defects-found-by-running-it`: their
  code lives in metaharness and agentplugins, not here;
  `dependency-blocker:defect-stories-whose-code-lives-in-sibling-repositories` records it.
- `story:journal-actor-from-configuration` (a configuration feature with a precedence design) and
  `story:workspace-declaration-has-one-reader-tests-included` (a test refactor): not defects in
  behaviour; they stay in the backlog.
- `dependency-blocker:metaharness-source-receipt`: clearing it is an Atlas reconciliation, not code
  in this repository.

## Acceptance

`gh issue list -R beyond10x/aep --state open` prints nothing, every story that `decomposes` this
epic is `implemented` or `archived`, and `aep plan artifact validate` prints `valid`.
