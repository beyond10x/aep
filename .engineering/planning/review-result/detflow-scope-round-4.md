---
format: aep.planning-md/3
id: review-result:detflow-scope-round-4
kind: review-result
status: active
title: 'Plan critic (scope), round 4: deterministic delivery workflows'
relations:
- reviews: epic:deterministic-delivery-workflows
- reviews: story:concurrent-command-steps
- reviews: story:bugfix-workflow
- reviews: story:release-step-map
- reviews: story:spend-against-baseline
revision: 1
---
Plan-critic round 4, scope lens, on the four revised draft stories of
`epic:deterministic-delivery-workflows`, read against `~/.local/state/worktree/trees/b10x/aep/aep-detflow3`
(HEAD 56100faa6). I extracted 13 promises from the epic's Outcome, Scope list and Done When and traced
all 13 to an item, with one exception that bears on a Done When bullet (below). All seven round 3
findings are fixed. The two ESS-scope blockers are gone: `concurrent-command-steps` and
`spend-against-baseline` now put `ess/domains/*.yaml`, `generated/ess/` and `ess_gate.rs`
`SYNTHESIZED` in Scope, specified first, with ESS 0.60.0 named. The `project.rs` citation is corrected.
`bugfix-workflow` now owns `concurrent: true` on diagnose and lists `generated/instructions/bugfix/`.
`release-step-map` no longer lists `workflows/releases/source.yaml` as changed. Step `id`/`after` is
dropped from the concurrent story and no reach beyond the epic remains there. The shared-output
refusal and the barrier and batch rules trace to "Concurrent command steps, opted into per state" and
to the diagnose row. No two stories claim one outcome.

The one defect is a gap. The four `verification` claims have no writer, and the epic needs a story
for it. Done When bullet 1 requires a release cut by `aep drive run`. The release workflow's
transitions are guarded by `verification.release-on-default-branch`, `verification.release-inputs`,
`verification.release-tag-pushed` and `verification.source-release`. `verification` is not in
`EvidenceMapping::MINTABLE`, so a command step can only read a record that some command wrote through
`record:` (the precedent is `aep govern validate --evidence` in `drivers/development/default.yaml`,
for the claim `document-tree-valid`). `story:release-cutting-commands` is implemented and covers
`prepare`, `tag`, `pin` and `record` only, so nothing in the set or the store writes these records.
The map in `release-step-map` is therefore a map no real release can pass, and its fixture passes only
because a stub writes the claims. The Open Question hands the choice to the operator, which is
honest, but the epic's table says "verify release" is deterministic "yes, every step" and its Done
When names no escape. Either add a story to the epic's Scope (for example, release checks that write
the four `verification` records, depended on by `story:release-step-map`) or amend Done When bullet 1
to say what it does not need. I recommend the story; the epic has no other owner of this outcome.

Out of lane, not set as findings: a possible mismatch between the Done When's "per task" and
`spend-against-baseline`, which prints a median per task kind and never a single task's standing (the
epic's "The measures" paragraph says the read verb reports "each task kind", so I did not treat it as
a narrowing). The same story reports `total_tokens` alone, not the five split token figures.
`release-step-map`'s `await_approval` row also names a human approval with no writer outside the test
(`tests/attested.rs`); that is a design and acceptance question.

What I read: 1 epic, 4 stories, review-result:detflow-scope-round-3, `workflows/releases/source.yaml`,
`story:release-cutting-commands`, `story:evidence-records-task-spend`, `drivers/development/default.yaml`
and `EvidenceMapping::MINTABLE`, via `aep plan artifact show`/`graph`/`list`. Promises extracted: 13;
traced to an item: 12 fully, 1 (Done When bullet 1) only conditionally.
Not established: whether any other repository command already writes a `verification` record for these
four claims. I grepped `xtask/src` and `crates` and found none.

```findings
[
 {"file":".engineering/planning/epic/deterministic-delivery-workflows.md","line":122,"category":"scope","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"Done When 'A release of this repository is cut by `aep drive run` over a release step map that has no `kind: llm` step' is promised, but no story claims a writer for the four `verification` claims (release-on-default-branch, release-inputs, release-tag-pushed, source-release) that guard the release/source transitions, and `verification` is not mintable by a command step; add a story to Scope (release-step-map would depend on it) or narrow Done When"}
]
```
