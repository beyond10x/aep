---
format: aep.planning-md/3
id: story:admit-er-direct-return-evidence
kind: story
status: implemented
title: Admit exact direct-return ESS conformance evidence for ER
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/edge/aep-cli
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/observe/aep-ess-evidence
- confidence: cited
  path: docs
- confidence: cited
  path: schemas/generated
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T07:38:44Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-28T07:38:45Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-09-28T08:35:47Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Acceptance and result

ER's exact actual library conformance report and inventory suite are admitted as typed coverage evidence. `tests/er_library_report.rs` retains the original runner report and suite and checks their complete successful selection and unchanged source bytes. ER's evidence command also recorded this pair against executable-system-specification:er-library-contracts. The runtime's later release-manifest pin alignment receives its own run; this retained first admission is not rewritten.

## Closed implementation contract

Ordinary suite28 and inventory suite29 use ESS source17/scenario4 from ac6fc6fe2f39b43f016e4d3a9edecb3573f7d6a1 (beyond10x/ess#185). Released suite26/27 retain different meanings and are not reinterpreted. Supported response types are String, Boolean, Integer, Optional<String>, Optional<List<String>> and transparent nominal newtypes ending in those types. Integer decimal/exponent spellings are evaluated exactly, without binary64 conversion. Unsupported types and unknown fields remain explicit refusals.

Original suite SHA256 association, model identity, selected scenario counts, complete inventory and parent lineage remain required. Modern scalar lexemes are retained exactly; legacy suite5 normalization stays unchanged. No compiled ESS dependency or new planning-store schema is added. Released AEP0.63.1 reads the resulting descriptive planning history, while admission and replay require this extended reader.

## Evidence

The initial task check passed, including source checks, MSRV and website validation. Named negative tests cover malformed declarations and mismatched report/suite authority. Removing the suite-digest guard made its integrity test fail, and restoration passed. Five independent boundary probes passed; review-result:er-direct-return-reader-review retains that report. The complete actual ER pair is a permanent fixture separate from authored reader reports. The reader integration test passes with the exact suite digest sha256:a6f8ceb64b95f76d33f75c741fb3b67770ad6130243504b77761921be6832548. Logs, mutation patch and review are in docs/evidence/direct-return-reader; source/profile limits are in docs/design/ess-direct-return-evidence.md.

## Integration

The operator authorized this compatibility work to complete ER's mainline integration, current planning-store adoption and release. beyond10x/aep#61 carries the change; beyond10x/entity-runtime#47 retains the actual implementation execution and AEP admission. Repository gates and required CI must pass on the final candidate before mainline merge. Upstream publication does not assert that ER's later release has already completed.
