---
format: aep.planning-md/1
id: task:ova-surface-hygiene
kind: task
status: draft
title: 'Inside the lines: the declared write surface, and checks that read no planning body'
summary: 'Acceptance 6 and 7 plus R18: protocol validate and protocol artifact validate exit 0, changed paths are only under docs/ and .engineering/, and no check reads the specification, the task or a planning artifact body.'
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
# Task: inside the declared lines, and checks that read no planning body

## What

Acceptance criteria 6 and 7, plus **R18** and the specification's Constraints, made into two checks:

- **Surface.** `protocol validate --root .` exits 0, `protocol artifact validate` exits 0, and
  `git status --porcelain` shows changed paths only under `docs/` and `.engineering/`.
- **Hygiene.** No check reads this specification, the task document or any planning artifact **body**.
  Checks read the audit, the corpus, the tree, and `protocol artifact list` output. Three programs
  only: `bash`, `git`, `protocol`. No `/tmp`. No network.

## Why

R18 is the one that would otherwise be violated by the most convenient implementation. A check that
greps the specification asserts that a sentence is still written there — which is a test of the
document that describes the work, not of the work. That is precisely the failure mode the story
exists to remove, reproduced inside the suite meant to remove it.

The surface half is the task's constraint restated as something a check decides rather than something
a reviewer remembers.

## Done When

| # | Acceptance |
|---|---|
| H1 | `protocol validate --root .` exits 0, and its output is relayed verbatim. |
| H2 | `protocol artifact validate` exits 0, and its output is relayed verbatim. |
| H3 | `git status --porcelain` lists changed paths only under `docs/` and `.engineering/`. Any path under `crates/`, `website/`, `integrations/`, `drivers/`, or the workspace `Cargo.toml` is red and named individually. |
| H4 | `integrations/claude-code/eval/checks/run-checks.sh` is unchanged — `git diff --quiet` on that path succeeds. It is read as a model, never edited. |
| H5 | No script under `.engineering/checks/` names a path under `.engineering/planning/`, or `.engineering/task-w4-2.yaml`, or the specification file. The only permitted route to store state is `protocol artifact list`. |
| H6 | The whole suite runs to exit 0 with `jq`, `yq`, `curl`, `wget`, `nc`, `python`, `python3`, `node` and `cargo` shadowed by stubs that exit 127. |
| H7 | No script under `.engineering/checks/` contains a literal `/tmp` path; every scratch path derives from `${TMPDIR:-$HOME/.cache/claude-tmp}`. |
| H8 | A run with `TMPDIR` set to an empty scratch directory leaves nothing behind in it after the suite exits, on both the passing and the failing path. |
| H9 | H5, H6 and H7 cover `run.sh` and every `check-*.sh`, including checks added later — the check enumerates the directory rather than a list. |

## Notes

- H9 is what keeps this from becoming a snapshot. A hygiene check with a hard-coded file list is
  green the day it is written and blind to the next file added.
- `protocol artifact list` is the sanctioned exception in H5, and it is a narrow one: the ids and
  statuses it prints are store *state*, not a planning artifact's prose. `task:ova-followups`
  depends on it.
- H3 runs inside this worktree. Paths under `.engineering/planning/` are expected to change — the
  follow-up artifacts land there — and that directory is inside the allowed surface.
- H6 shadows `cargo` deliberately: nothing in this suite compiles anything, and a check that quietly
  invoked the workspace build would make the run neither hermetic nor fast.

## Verifier

`.engineering/checks/check-surface.sh` (H1–H4) and `.engineering/checks/check-check-hygiene.sh`
(H5–H9).

Two scripts, one task: both answer "did this run stay inside the lines it declared", one about the
tree and one about the suite itself.
