---
format: aep.planning-md/3
id: story:resume-submits-evidence-to-a-stopped-run
kind: story
status: implemented
title: A stopped run takes evidence on resume
summary: aep drive resume <run> --evidence <file> submits the file's records to the run's own snapshot before the owed operator step is settled; the pause text names that command; aep govern evaluate --state stays read-only and says so.
owner: project
tags:
- driver
relations:
- decomposes: epic:reference-driver
- serves: vision:O3
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/drive/aep-driver/src/run.rs
- confidence: cited
  path: crates/drive/aep-driver/tests/attested.rs
- confidence: cited
  path: crates/edge/aep-cli/src/app.rs
- confidence: cited
  path: crates/edge/aep-cli/src/drive.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/drive_cli.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/evidence_producers.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T19:34:29Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-23T19:34:29Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-23T20:10:04Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: A stopped run takes evidence on resume

## Outcome

A run stopped at an `operator` step under `--approver agent:<name>` can be answered. `aep drive
resume <run> --evidence <file>` reads the evidence list in `<file>` (the format `aep govern
evaluate --evidence` reads), submits every record to the run's own restored execution before the
owed step is settled, and the settle reads the approval the file carried: the run walks on, the
cursor's `answers` names the approver, and the committed generation holds the record. The pause
text an `operator` step prints names that command. `aep govern evaluate --state` stays what it is,
an evaluation that writes nothing, and says so when it is given evidence against a snapshot.

## Context

In 0.77.0 (`23294974`) the pause text (`crates/edge/aep-cli/src/drive.rs`, `run_operator`) tells
the operator to "record the approval against this run's snapshot with `aep govern evaluate
--evidence <file> --state <run>/snapshot.json`". That command (`crates/edge/aep-cli/src/app.rs`,
`evaluate`) restores the snapshot, submits the records, prints the evaluation and exits; it never
writes the snapshot back. It also names `snapshot.json`, which since the generations layout is a
compatibility projection the resume never reads when `state.json` points at a generation. The
resume (`crates/drive/aep-driver/src/run.rs`, `settle`) then finds nothing after
`owed.evidence_before` and stops again with "nothing was recorded while the run was stopped",
naming who would be admissible. Under `--approver` that is every resume, so the run has no way
out; without `--approver` the run walks on with nobody's answer recorded. The approval route the
0.76 story `story:attested-approver` tested (`crates/drive/aep-driver/tests/attested.rs`,
`approve`) writes through the engine and `RunDirectory::persist` in-process, which no CLI verb
does.

Seen by ess-ui-templates on 2026-10-23 in run `ess-ui-ess-068-crates/1` under
`drivers/development/checks.yaml`, reported by metaharness and routed here by the orchestrator.

## Acceptance

1. `aep_driver::run::resume` with `DriverOptions::evidence` holding one granted approval by the
   named approver, on a run paused at an `operator` step, walks on: the step after the pause runs,
   `cursor.answers` names the approver, nothing is owed, and the cursor on disk carries the answer
   (`crates/drive/aep-driver/tests/attested.rs`).
2. A submission the engine refuses stops the resume before anything is persisted: the run's
   cursor and generation pointer are byte-identical afterwards (*Refusals change nothing*).
3. `aep drive resume <run> --evidence <file>` reads the file with the same reader as `aep govern
   evaluate --evidence` and hands the records to the driver; an unreadable file is a refusal that
   writes nothing (`crates/edge/aep-cli/tests/drive_cli.rs`).
4. The `operator` pause text names `aep drive resume <run> --evidence <file>` and no longer names
   `aep govern evaluate` or `snapshot.json`.
5. `aep govern evaluate --state <snapshot> --evidence <file>` leaves the snapshot file unchanged
   and prints one line saying the evaluation wrote nothing and naming `aep drive resume
   --evidence` for a run.
6. `website/docs/reference/cli.md` lists `--evidence` on `aep drive resume`, and `CHANGELOG.md`
   carries the fix under Unreleased.

## Scope

- `crates/drive/aep-driver/src/run.rs`: `DriverOptions::evidence`, submitted once on the first
  iteration of `Session::run` after the map check and before `settle`.
- `crates/drive/aep-driver/tests/attested.rs`: acceptance 1 and 2.
- `crates/edge/aep-cli/src/drive.rs`: `ResumeArgs::evidence`, the pause text, the `read_record`
  refusal that names the approval route.
- `crates/edge/aep-cli/src/app.rs`: the note under `--state` with `--evidence`; `read_evidence`
  shared with `drive.rs`.
- `crates/edge/aep-cli/tests/drive_cli.rs`, `crates/edge/aep-cli/tests/evidence_producers.rs`.
- `website/docs/reference/cli.md`, `CHANGELOG.md`.
