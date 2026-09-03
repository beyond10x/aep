---
format: aep.planning-md/1
id: story:confined-driven-workspace
kind: story
status: draft
title: A driven run on the native harness can write, so the two arms are comparable
relations:
- decomposes: epic:cross-harness-portability
scope:
- confidence: inferred
  path: crates/drive/aep-driver/src/run.rs
- confidence: inferred
  path: crates/edge/aep-cli/src/drive.rs
- confidence: cited
  path: scripts/drive-score
revision: 7
---
# Story: A driven run on the native harness can write, so the two arms are comparable

## Outcome

The same task, driven on Claude Code and on the b10x loop, produces two run records that can be put
beside each other — because both arms did the work, rather than one of them being unable to.

## Context

**`protocol drive` can now spawn the b10x loop, and a b10x run cannot yet do a task's work.** The
executor landed on 2026-08-29: a step map's `harness:` field selects it, `metaharness run b10x` is
the invocation, the shared `tool_config` renders into that loop's own catalogue rather than Claude
Code's, and a pre-flight refuses a map naming it before a run id or a lock exists. Six guards, each
verified by breaking it.

What it cannot do is write. Writes and execution in that loop require `--substrate*` **and** a
workspace whose directory name carries the `ws_` prefix (`SUBSTRATE_WORKSPACE_PREFIX`); a governed
tree is an ordinary checkout, so an unconfined launch publishes no `file_write`, no `file_edit` and
no `run`.

**Why that makes the comparison dishonest rather than merely limited.** A driven Claude run of
`W4-3/1` wasted 17.2% of its tool calls — 37 of 215 — and 24 of those 37 were shell-shaped: a
composed command, or a program outside the surface. On a read-only b10x arm those cannot happen
**because the shell is not there**. The arm would score beautifully for the same reason a run that
does nothing scores beautifully, which is the defect the eval corpus already had to grow a row for
(`the-implementation-was-changed`, added after one arm ended cleanly at 19 turns having written no
file at all).

So the number this repository wants — *does the native harness waste less* — cannot be answered
until a b10x run can do a task's work. Until then the two arms differ in what they were **able to
attempt**, and any table putting them side by side is comparing instruments.

## Acceptance

- A driven run under `harness: b10x` reaches a state that writes, writes there, and its record shows
  the write — over a workspace the loop will confine, arranged by the driver rather than by the
  operator remembering a prefix.
- The **same task and the same map** run on both harnesses, and `drive-score` reports both. What is
  compared is waste **per unit of work completed**, not raw refusal counts: an arm that attempted
  less is not an arm that wasted less.
- Where a waste class is *structurally impossible* on one arm, the comparison says so rather than
  counting it as a win. The four classes and their honest verdicts are already written down by the
  executor's own author and are the starting point.
- The workspace arrangement is the driver's and is recorded: a reader can tell a confined run from
  an unconfined one from the run directory alone.

## Out of Scope

- Making a governed checkout itself confinable. The workspace is arranged **for the run**; a
  repository is not renamed to suit a harness.
- The `--decisions` modes. The b10x adapter observes and adjudicates nothing by design, and giving
  it a seam would put the driven arm's treatment on top of the observed arm and make the two differ
  in name only — its own header argues this and it stands.
- Any change to the Claude Code arm.

## Open Questions

**Does the driver create the confined workspace, or refuse until one exists?** Decides: driver
owner. Default if nobody answers: **create it** — an operator who has to know about a `ws_` prefix
is an operator who will forget, which is the class of defect three of today's five fixes were.

**Is a run in a confined workspace still a run of *this* repository?** Decides: protocol owner.
Default: **yes, and the record names the workspace** — the store it governs is the one it was
pointed at, and where the files sat while it worked is provenance rather than identity.
