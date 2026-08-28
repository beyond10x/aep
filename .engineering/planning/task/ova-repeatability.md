---
format: aep.planning-md/1
id: task:ova-repeatability
kind: task
status: implemented
title: How it was produced, so the next round is a diff
summary: 'R14 and R15: a section naming the corpus rule, the scan command by path, the commit the round was taken at and the reading pass, with the named script running and exiting 0.'
owner: protocol
tags:
- adoption
- checks
relations:
- decomposes: specification:open-vocabulary-audit
- derived_from: task:w4-2-open-vocabulary-audit
- depends_on: task:ova-checks-runner
- depends_on: task:ova-scan-declarations
- depends_on: task:ova-audit-corpus
revision: 7
---
# Task: how it was produced, so the next round is a diff

## What

**R14–R15.** A section of the audit stating how it was produced:

- the corpus rule (the three globs of R1);
- the command, `bash .engineering/checks/scan-declarations.sh`, named by path;
- the commit the round was taken at;
- the reading pass that produced the non-scanned rows, and how many rows it produced.

And the property that follows: re-running the scan and the checks tells the reader which rows moved,
which citations no longer resolve and which candidates are new. Nothing in the audit needs rewriting
to run it again.

## Why

The story's fourth acceptance bullet. An audit that does not say how it was produced can only be
redone, not re-run — and a round-2 that is a rewrite is a round-2 that does not happen. The commit is
part of it: without it, "the corpus was 32 files" is a claim nobody can reproduce.

## Done When

| # | Acceptance |
|---|---|
| Y1 | The audit carries a section stating how it was produced, found by its heading. |
| Y2 | That section names `.engineering/checks/scan-declarations.sh` by path, and the path exists. |
| Y3 | The command the section prints runs verbatim from the repository root and exits 0. |
| Y4 | It states the commit the round was taken at, and `git cat-file -e <commit>` succeeds. |
| Y5 | The corpus rule it states is the same three globs `check-corpus.sh` re-derives from — compared, not eyeballed. |
| Y6 | It states the count of rows produced by the reading pass, and that number equals the reading-backed count `check-provenance.sh` prints. |
| Y7 | Running the whole suite twice in a row leaves `git status --porcelain` identical between the two runs, and both exit 0 — the suite is re-runnable without editing the audit. |
| Y8 | The section describes no step the suite does not perform: every command it names is one a check actually runs. |

## Notes

- Y6 is what stops the section drifting into narrative. The reading pass is a number the provenance
  check already computes; stating it in the audit and comparing the two makes the prose falsifiable.
- Y8's direction is deliberate — the audit may say less about its method than the suite does, never
  more. A described step nobody runs is the same defect as a documented check nobody wrote.
- The commit in Y4 is the one the round was taken at, not `HEAD` at check time. It is expected to go
  stale relative to `HEAD`, and Y4 asserts only that it still resolves.

## Verifier

`.engineering/checks/check-repeatability.sh`. Y1–Y8 are its rows.

Y3 does not read the command and approve of it — it extracts the command from the section and runs
it, which is the only version of this row that is worth anything.
