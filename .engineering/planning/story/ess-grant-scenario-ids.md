---
format: aep.planning-md/3
id: story:ess-grant-scenario-ids
kind: story
status: active
title: Evidence admits the scenario ids ESS 0.55.0 synthesizes, including grant forms
refs:
- provider: github
  reference: beyond10x/aep#88
relations:
- serves: vision:O2
- informed_by: epic:evidence-gated-completion
scope:
- confidence: cited
  path: crates/edge/aep-cli/tests/ess_conformance_coverage.rs
- confidence: cited
  path: crates/govern/aep-domain/src/ess_conformance_coverage/values.rs
- confidence: cited
  path: crates/govern/aep-domain/src/ess_conformance_v2.rs
- confidence: cited
  path: crates/govern/aep-domain/tests/ess_coverage_values.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/src/coverage_definition.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/src/coverage_suite.rs
- confidence: inferred
  path: crates/observe/aep-ess-evidence/tests
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/current_suites.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/fixtures/current-suites
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:06:11Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-07T08:06:11Z", actor: "human:timo", revision: 5}
---
## Outcome

`aep plan artifact evidence --from <report> --suite <suite>` records a conformance report whose suite
was synthesized by ESS 0.49.0 or later, including suites that carry command-grant and view-grant
scenarios. The scenario-id grammar the evidence adapter admits is the grammar ESS 0.55.0 reads, so a
newer ESS suite form is not refused as `MalformedScenarioId`.

## Acceptance

- A coverage suite whose scenario keys include `<command>/grant/denied` and
  `<command>/grant/admitted/<actor>` is admitted and its report recorded, at a suite major that
  carries those forms; the same keys under a major below the one that introduced them are refused as
  `UnsupportedVocabulary` naming the form and the major, as `aggregate` and `binding/final-failure`
  are today.
- The same holds for every other form ESS 0.55.0 `ScenarioId::parse` accepts and AEP refuses today:
  `<view>/grant/read/denied`, `<view>/grant/read/admitted/<actor>`,
  `<binding>/binding/refusal/<outcome>`, the `disclosure` cells and binding aspects added after
  `final-failure`. Each form has a test that admits it and one that refuses it below its major.
- A malformed id (an empty segment, a malformed qualified name, an unknown keyword) is still refused
  as `MalformedScenarioId`, with a test per case.
- The count reader's frozen suite/1–4 grammar (`ScenarioId::frozen`) is unchanged.
- No AEP manifest gains an `ess-*` dependency (`dep-check`).

## Origin

https://github.com/beyond10x/aep/issues/88, reported downstream on aep 0.68.0: a report/2 of 911 of
911 passed scenarios could not be recorded because its suite carried `<command>/grant/denied` keys
(an ESS suite for served components since ESS 0.49.0, https://github.com/beyond10x/ess/issues/265).

## Source

- `crates/govern/aep-domain/src/ess_conformance_v2.rs:86` `ScenarioId::new` admits only the frozen
  forms plus `<view>/aggregate` and `<binding>/binding/final-failure`.
- `crates/observe/aep-ess-evidence/src/coverage_suite.rs:159` `LATER_ID_FORMS` gates those two forms
  by suite major.
- ESS 0.55.0 grammar: `crates/verify/ess-conformance/src/scenario.rs` `ScenarioId::parse` (ESS tag
  0.55.0); grant majors in `grant.rs` and `view_grant.rs`, disclosure in `one_time_response.rs`.

## Specification

No ESS specification exists in this repository. This fix changes no noun, command, outcome or
configuration: it widens the admitted grammar of an existing value (`ScenarioId`) to match the
external ESS format AEP already reads. Rust types stay the source of truth (`schemas/generated/` by
`cargo xtask schema`).

## Scope

- cited: `crates/govern/aep-domain/src/ess_conformance_v2.rs`
- cited: `crates/govern/aep-domain/tests/ess_coverage_values.rs`
- cited: `crates/observe/aep-ess-evidence/src/coverage_suite.rs`
- inferred: `crates/observe/aep-ess-evidence/tests/`

## Found during the fix

Recorded 2026-10-07 from the implementation and two adversary passes; not fixed in this story.

- Seed-record refusals that are about references (unknown source or selection, a scenario the
  suite does not hold, an authored target, a duplicate identity, ordering, counts) are reported as
  `InvalidShape`, which elsewhere in the coverage reader means a JSON type mismatch; the detail
  text tells them apart. ESS names them `InvalidSynthesisSeeds`. A separate code is a new outcome
  for consumers and was left out of a defect fix.
- Steps that establish and address a seed row read scenario bodies, which AEP leaves to ESS; a
  hand-built suite whose steps disagree with its seed record is admitted.
- Scenario ids named in `coverage.refused[].scenario` are not gated by major; ESS's reader gates
  only scenario keys, which is what AEP mirrors.

- The transcribed suite/5 lineage compares payloads as `aep_domain::Node`, whose numbers are
  `f64`, so a selected child that changed an integer above 2^53 to its neighbour compares equal to
  its parent and is admitted (pre-existing). Fixing it changes `aep_domain::Number`.
