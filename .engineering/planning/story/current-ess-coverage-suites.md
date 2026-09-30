---
format: aep.planning-md/3
id: story:current-ess-coverage-suites
kind: story
status: implemented
title: Evidence admits every coverage suite ESS writes
relations:
- serves: vision:O2
- informed_by: epic:evidence-gated-completion
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:08:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T12:08:58Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T12:27:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A conformance report ESS writes today is recorded as evidence: aep admits every coverage-bearing
suite version ESS writes, binds the suite by digest, and reads scenario ids and refusal codes
with the grammar ESS uses.

## Acceptance

- `aep plan artifact evidence --from <report> --suite <suite>` admits suites `ess-conformance/5`,
  `/7`, `/9` … `/33` with their report/2; even and newer versions are refused by name.
- Scenario bodies above `/5` are opaque; the suite is bound by its `sha256-json-bytes/1` digest.
- Scenario ids follow ESS's `ScenarioId::parse` (including `<view>/aggregate` and
  `<binding>/binding/final-failure`), with ESS's version floors; refusal codes follow ESS's
  inventory (`ESS-SYNTH-015`–`017`, `ESS-AUTHOR-037`).
- `/1`–`/4` count reports keep their frozen grammar.

## Origin

aep 0.66.0 refused a downstream `ess-conformance/27` suite with its ESS 0.48.0 report.
