---
format: aep.planning-md/3
id: story:legacy-ess-report-calendar-refusal
kind: story
status: active
title: Legacy ESS report dates refuse before the planning backend panics
tags:
- measured
relations:
- decomposes: epic:zero-open-issues-2026-11-09
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/ess_report_calendar.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:44Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:45Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

A caller importing a legacy ESS report receives a checked calendar-range refusal before any planning-store mutation when its otherwise valid timestamp cannot be represented by the event backend.

## Measured defect

Coordinator executed actual ESS Rust report1 output at source a46bd7ff46ec8553bef4f48d4021514c8f175e82 through AEP source30aeef2c9985613f5283764bdff3eec12160e43d (current100fc25 changes only internal plan prose). The unchanged legacy library adapter preserved completed_at9007199254740993 and9223372036854775807. The independent planning command `aep plan artifact evidence story:producer-compat --from legacy-report.json --store <scratch-store>` panicked at crates/plan/aep-backend-entity/src/lib.rs:224: `recording.seal(...).expect(...)`, after the runtime rejected the out-of-calendar recorded_at. The scratch store remained byte-identical. This is a reachable CLI import of valid legacy Rust output, not a fabricated backend state.

Raw commands, output and exits: target/ess-conformance-v2-counts/producer-compat/actual-a46bd7ff46ec/checks/rust/clock-9007199254740993/legacy-planning.log and the corresponding clock-9223372036854775807 log; command records are in the same run's commands.json. Original suite/report files and hashes are retained beside the checks. This source file is byte-identical between inspected pre-reader00c742e4179593738a2e8aa69e2ecc07d3c89402 and published reader30aeef2; the report1 import path is unchanged. Origin is pre-existing, established by those source reads plus actual reproduction on the published source; no old checkout was executed.

The new report2 path already returns PlanningTimestampUnsupported before opening or mutating the store for these same actual producer timestamps. Its typed adapter/replay preserves the full u64 values. This story does not hold the count-writer migration: widening or changing legacy behavior was explicitly excluded there.

## Acceptance

- `aep plan artifact evidence <id> --from <report>` on an `ess-conformance-report/1` whose
  `completed_at` is `9007199254740993` or `9223372036854775807` exits 1 (not 101), prints no panic
  and prints one line starting `PlanningTimestampUnsupported:` that names the report's
  `completed_at` as given and the timestamp it renders as — the same text the report/2 branch
  prints (`crates/edge/aep-cli/src/planning.rs:7533`). Before this story both values exit 101 with
  `EnvelopeError("recorded_at \"292278994-08-17T07:12:55Z\" is not a valid runtime timestamp")`.
- The refusal is decided by `entity_core::is_valid_timestamp`, the same check the report/2 branch
  uses, before the store is opened: on an existing store every file under `.engineering/` is
  byte-identical before and after, and with `--store` naming an absent directory that directory is
  not created.
- The same command on the same report with `completed_at` `1700000000000` still exits 0 and records
  one evidence file; a report/1 with zero scenarios is still refused as before; the report/2
  route's existing tests still pass unchanged.

## Scope

- crates/edge/aep-cli/src/planning.rs, recorded_from_report report1 branch — cited; apply an explicit checked calendar adaptation at this edge.
- crates/edge/aep-cli/tests — inferred; focused actual legacy import and no-store-mutation controls.
- crates/plan/aep-backend-entity/src/lib.rs:224 — cited defect endpoint, read-only unless a separately justified backend policy change is selected.
- Confidence: high for the measured CLI defect; implementation choice remains unselected.

## Out of Scope

ESS writer changes, default/version movement, rewriting stored evidence, changing report1's aggregate category meaning, or implementing a general backend panic policy.

## Triage 2026-11-09

Still present at `824869fec`, and reproduced on a Git-native store. The `ess-conformance-report/1` branch of `recorded_from_report` (`crates/edge/aep-cli/src/planning.rs:7626-7635`) takes `completed_at` with `as_i64` and has no `entity_core::is_valid_timestamp` check; the report/2 branch has it (`:7531-7534`) and so does `recorded_coverage` (`:7667-7670`). The panic is now at `crates/plan/aep-backend-entity/src/lib.rs:265-267`.

With aep 0.79.0, `aep plan artifact evidence story:probe --from r.json` with `completed_at` 9007199254740993 or 9223372036854775807 exits 101 on `EnvelopeError("recorded_at \"292278994-08-17T07:12:55Z\" is not a valid runtime timestamp")`; the store stays byte-identical; `1700000000000` records normally.
