---
format: aep.planning-md/3
id: approval-record:resume-evidence-route
kind: approval-record
status: approved
title: 'Approved: a stopped run takes evidence on resume (0.77.1)'
summary: 'Orchestrator routing of the metaharness defect, 2026-10-23: govern evaluate --evidence never writes the snapshot back while the pause text names it; file a story, failing test first, fix, release 0.77.1.'
owner: project
tags:
- driver
relations:
- decides: story:resume-submits-evidence-to-a-stopped-run
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-23T19:34:29Z", actor: "human:timo", revision: 2}
---
# Approval record: a stopped run takes evidence on resume (0.77.1)

## Decision

A defect reported by metaharness, routed to this repository by `b10x-chef-orchestrator` on
2026-10-23 in these words:

> fix this defect (from metaharness, routed to AEP): in 0.77.0 (23294974) `aep govern evaluate
> --evidence` (aep-cli/src/app.rs:1787) never writes the snapshot back, while the drive pause text
> (drive.rs:1995) tells the operator to use it; a run with --approver then stops at run.rs:1783
> with no way out (seen in ess-ui-templates story:ess-068-crates run /1). File a story, failing
> test first, fix, release 0.77.1 through your process, then tell metaharness and ess-ui-templates.
> Approved under the grant in […].

The ess-ui-templates session reported the same defect directly, with the repro run
`ess-ui-ess-068-crates/1` and the workaround "run without `--approver`".

The grant named is the workspace operator's standing approval for class-C items relayed by the
orchestrator. Decider: operator.

## What this record authorizes

- The story it decides, as one unit, and the source release 0.77.1 that ships it, each through
  this repository's own process (`AGENTS.md` § *Releases and commits*).
- Telling the metaharness and ess-ui-templates sessions once the release is verified.

## Decisions taken inside the item

- D1 The route is the resume, not the evaluation. `aep drive resume <run> --evidence <file>`
  submits the file's records to the run's restored execution before the owed step is settled,
  through the driver, so the record lands in the committed generation the next resume reads.
  `aep govern evaluate --state` keeps writing nothing: an evaluation that rewrote a run's
  snapshot would be a second write path beside the driver's, against a projection the driver
  does not read. Both verbs now say which is which.
- D2 The driver submits the records once, on the first iteration, after the map check and before
  `settle`, and counts them in the report's `evidence_submitted`. A refused submission is a
  `DriveError` before anything is persisted (*Refusals change nothing*).
- D3 The file is read with the reader `aep govern evaluate --evidence` uses, so one document
  format serves both verbs. The driver does not inspect what the records are; `settle` decides
  whether an approval answers the step and who may have given it, as before.
