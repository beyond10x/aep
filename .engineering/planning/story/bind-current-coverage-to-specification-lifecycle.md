---
format: aep.planning-md/3
id: story:bind-current-coverage-to-specification-lifecycle
kind: story
status: implemented
title: Earn specification conformance from admitted current coverage
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: artifacts/lifecycles/executable-system-specification.yaml
- confidence: cited
  path: crates/edge/aep-cli
- confidence: cited
  path: crates/observe/aep-ess-evidence
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/journal.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/tests/journal.rs
- confidence: cited
  path: docs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T08:51:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-28T08:51:26Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-09-28T09:30:57Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Acceptance and result

Exact admitted suite29/report2 coverage now earns specification conformance through a narrow typed compatibility bridge. The original coverage record remains ess_conformance_coverage_v1; decided_on records a distinct ess_conformance_from_coverage eligibility basis rather than inventing a legacy evidence event. ER's final exact pair was re-imported with retained originals and its real specification moved validated to conforming on 2026-09-28. The source closure, suite digest and original completion milliseconds are preserved.

## Contract and guards

The bridge re-admits original report and suite-input strings from a closed source envelope and validates descriptive fields against them. It requires the current model digest, complete inventory for the entire system with generated and authored origins and an all filter, nonempty execution, zero outside/synthesis refusal/nonpass counts, a matching planning timestamp projection, and nonfuture completion. The newest exact completion time for the current model controls eligibility; every tied candidate must qualify. A newer failed or partial run vetoes older success. Summary-only or malformed sources cannot qualify; another model remains a separate candidate set. Legacy evidence and task-engine principle behavior stay unchanged.

## Named verification and review

Permanent checks include current_complete_coverage_earns_specification_conformance_without_inventing_a_record, a_newer_or_equally_timed_failed_coverage_run_prevents_old_success_earning_conformance, and planning_source_retains_originals_and_binds_model_and_full_completion_time. All affected package tests, strict Clippy and formatting passed. Removing the model-digest guard made its named test fail; restored source passed. Seven independent adversarial CLI probes passed, including the actual ER pair after deleting its input files and released AEP0.63.1 history readback and validation. No store-format or package-version migration is needed; older CLI builds cannot grant this new route.

The complete task check passed with source and planning store held unchanged. An earlier full-gate attempt failed because root added the review record while the repository's SQLite snapshot-comparison test was running; its failure is retained as an orchestration error, with no product change made for it. Retained evidence is under docs/evidence/coverage-lifecycle/. Composition with current main, final integration checks and publication remain the next delivery steps.