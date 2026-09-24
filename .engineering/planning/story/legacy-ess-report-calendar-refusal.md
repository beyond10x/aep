---
format: aep.planning-md/2
id: story:legacy-ess-report-calendar-refusal
kind: story
status: draft
title: Legacy ESS report dates refuse before the planning backend panics
tags:
- measured
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests
revision: 3
---
## Outcome

A caller importing a legacy ESS report receives a checked calendar-range refusal before any planning-store mutation when its otherwise valid timestamp cannot be represented by the event backend.

## Measured defect

Coordinator executed actual ESS Rust report1 output at source a46bd7ff46ec8553bef4f48d4021514c8f175e82 through AEP source30aeef2c9985613f5283764bdff3eec12160e43d (current100fc25 changes only internal plan prose). The unchanged legacy library adapter preserved completed_at9007199254740993 and9223372036854775807. The independent planning command `aep plan artifact evidence story:producer-compat --from legacy-report.json --store <scratch-store>` panicked at crates/plan/aep-backend-entity/src/lib.rs:224: `recording.seal(...).expect(...)`, after the runtime rejected the out-of-calendar recorded_at. The scratch store remained byte-identical. This is a reachable CLI import of valid legacy Rust output, not a fabricated backend state.

Raw commands, output and exits: target/ess-conformance-v2-counts/producer-compat/actual-a46bd7ff46ec/checks/rust/clock-9007199254740993/legacy-planning.log and the corresponding clock-9223372036854775807 log; command records are in the same run's commands.json. Original suite/report files and hashes are retained beside the checks. This source file is byte-identical between inspected pre-reader00c742e4179593738a2e8aa69e2ecc07d3c89402 and published reader30aeef2; the report1 import path is unchanged. Origin is pre-existing, established by those source reads plus actual reproduction on the published source; no old checkout was executed.

The new report2 path already returns PlanningTimestampUnsupported before opening or mutating the store for these same actual producer timestamps. Its typed adapter/replay preserves the full u64 values. This story does not hold the count-writer migration: widening or changing legacy behavior was explicitly excluded there.

## Acceptance

- Reproduce actual legacy report1 import at the calendar boundary and the two observed upper-i64 values; return a located error rather than panic and retain exact original source time.
- Refuse before opening an absent store or mutating an existing one; exercise the actual CLI path.
- Preserve valid in-range legacy evidence, zero-total refusal, and existing signed/unsigned transport semantics. Do not relax the runtime's calendar contract or narrow the library's valid u64 input domain.
- Preserve the separately checked report2 route and exact typed replay.

## Scope

- crates/edge/aep-cli/src/planning.rs, recorded_from_report report1 branch — cited; apply an explicit checked calendar adaptation at this edge.
- crates/edge/aep-cli/tests — inferred; focused actual legacy import and no-store-mutation controls.
- crates/plan/aep-backend-entity/src/lib.rs:224 — cited defect endpoint, read-only unless a separately justified backend policy change is selected.
- Confidence: high for the measured CLI defect; implementation choice remains unselected.

## Out of Scope

ESS writer changes, default/version movement, rewriting stored evidence, changing report1's aggregate category meaning, or implementing a general backend panic policy.
