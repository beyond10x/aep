---
format: aep.planning-md/1
id: story:three-arm-pilot-2
kind: story
status: draft
title: 'Pilot 2: every cell re-run on the frozen corpus, with the streams committed'
relations:
- decomposes: epic:self-evaluation
revision: 2
---
# Story: Pilot 2 — every cell re-run on the frozen corpus, with the streams committed

## Outcome

Somebody asking *how well do the harnesses behave under our protocol* gets one table whose numbers
can still be checked next month — because the streams behind it are in the repository, and the
corpus that scored them is the one in the repository.

## Context

**Pilot 1 (2026-08-23) can no longer be compared with anything, and it cannot be recomputed.** Two
independent reasons, both recorded on
[`2026-08-23-three-arm-pilot-1.md`](../../../docs/reviews/2026-08-23-three-arm-pilot-1.md):

- **its corpus is superseded.** Those cells were scored against a **10-row**
  `conformance/eval/development-honest/expectations.trace.yaml`. It has **12** rows now and changed
  three times since: `2f7498f` added `the-implementation-was-changed` — the row pilot 1's own
  finding argued for, because one arm ended cleanly having written nothing and outscored the two
  that did the work; `24e8d6a` made the write selectors vendor-neutral, which is what the codex
  cells' `unobservable` counts were about; `9a2853e` added `the-scope-was-actually-tested`. A
  held-count over 10 rows and one over 12 are different measurements.
- **its streams are gone.** They were written to a session scratchpad (`…/scratchpad/r4/`), which
  no longer exists — searched 2026-08-28. The only committed streams are **synthetic**
  (`crates/protocol-cli/fixtures/eval-run/`, stated in that README's first line), so the free
  re-ingest path `crates/protocol-cli/tests/eval_dry_run.rs` exists and has nothing live to point
  at.

**The arm nobody has ever measured is now runnable.** Pilot 1's arm `c` cost $0 because it was
refused before starting, on the four evidence kinds `drivers/development/default.yaml` could not
produce. `story:evidence-producers-for-the-driven-map` closed that on 2026-08-28: the pre-flight
names zero kinds and a `kind: feature` run starts with no `--allow-evidence-gap`. The programme's
central hypothesis — `H-arms`, that expectations-held orders `a ≤ b ≤ c` — has never had its third
term.

**This is the same defect the store waves were about, one layer out.** The measurement is a phase
table in `docs/plan/eval-program-three-arms.md` and was in no artifact of this store; the machinery
around it — `story:eval-runner`, `story:eval-matrix-assembler`, `story:eval-dry-run`,
`story:eval-case-corpus` — is all `implemented`, so the plan reads as though the measurement had
been done. This story is the thing that was missing.

## Acceptance

- **Every cell is re-run**, not only the new one: `adp/default` × {claude, codex} × {raw, plugin,
  driven}. Pilot 1's numbers are not carried forward for any cell, and the report says why in one
  line rather than referring the reader to this story.
- **The streams are committed** under `conformance/eval/` (or a sibling the corpus README names),
  with a manifest naming each run's digest — so `protocol eval matrix` can be re-run from the
  repository alone, by somebody who was not there, after the corpus changes again. A run whose
  stream is not committed does not enter the matrix.
- **The corpus is frozen before the first paid run**, per R4.2, and the freeze is a commit somebody
  can name. An expectation edited between two cells makes the two cells incomparable, which is what
  happened here by accident.
- **Arm `c` is decided rather than refused.** If a driven run blocks, the block is the result and
  the report says where; a `$0` cell reported as *refused before start* is only acceptable when the
  refusal is the finding, and it says which pre-flight refused.
- The report states, per cell, what the arm **could not witness** and why — the codex `apply_patch`
  limit and the driven arm's three known-different rows are already argued in the corpus's own
  comments and are cited rather than rediscovered.

## Out of Scope

- **The decline path.** `adp/default/2` has a terminal `declined`, and this case judges every run
  against the delivery path — which is how a run that correctly built nothing read as clean. The
  corpus names a sibling case as owed; it needs a committed transcript of a declining run and is its
  own story, not a widening of these rows.
- **The R4.3 sweep** (4 workflows × K=3, ~72 runs, $25). This is R4.1 repeated properly; the sweep
  is worth paying for only against a corpus that has stopped moving and a pilot whose cells all
  decided.
- **`pi` and `opencode`.** Operator sequencing, 2026-08-23: they join after this programme's first
  matrix.

## Open Questions

**How many runs per cell?** Decides: whoever holds the eval budget. Default if nobody answers:
**one**, as pilot 1 — six cells at roughly pilot 1's rate is about $6–10, and the value here is the
third arm existing at all rather than a variance estimate. A second run per cell is the sweep's job.

**Does a driven cell need the same case as the raw and plugin cells?** Decides: eval owner. Default
if nobody answers: **yes** — a different case per arm would measure the case. The corpus already
records the three rows that answer differently under `driven` and why, so the comparison survives
being honest about them.

**Where do the streams live?** Decides: eval owner. Default if nobody answers: beside the case, under
`conformance/eval/<case>/runs/<harness>-<arm>.jsonl`, because the thing that must not drift apart is
a stream and the expectations it was scored against.
