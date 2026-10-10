---
format: aep.planning-md/3
id: story:evidence-admits-ordinary-ess-suites
kind: story
status: active
title: Evidence records a report against an ordinary suite current ESS writes
relations:
- serves: vision:O2
- informed_by: epic:evidence-gated-completion
- informed_by: story:current-ess-coverage-suites
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T01:30:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T01:30:13Z", actor: "human:timo", revision: 3}
---
# Story: Evidence records a report against an ordinary suite current ESS writes

## Outcome

`aep plan artifact evidence <id> --from <report/2> --suite <suite>` records a report whose suite is
an ordinary (coverage-free) suite major that ESS 0.56.0 and later write by default, instead of
refusing it as a coverage suite with no `coverage` field.

## Context

Reproduced on aep 0.71.0 (the release `main` carries) with ess 0.57.0, 2026-10-10:

- `ess verify conform synthesize --path ess --target ir` over this repository's own specification
  writes `provenance.suite_version: ess-conformance/34` with top-level keys `provenance, scenarios`
  only; `ess verify conform run --target interpreted --report-format 2` writes a report/2 whose
  `suite.version` is `ess-conformance/34`, whose digest matches the suite, and whose
  `coverage` is `{knowledge: unknown}`.
- `aep plan artifact evidence epic:planning-on-entity-runtime --from report.json --suite suite.json`
  on a copy of this store exits 1: `MissingField at $suite.coverage: required field absent`.
- Cause: `coverage_input_from_raw_suite` in `crates/edge/aep-cli/src/planning.rs` sends every suite
  major from /5 on to the coverage reader (`major >= 5`). Only the odd majors in
  `COVERAGE_SUITE_MAJORS` (`crates/govern/aep-domain/src/ess_conformance_coverage/values.rs`) carry
  a `coverage` block; the even majors from /6 on are ordinary suites, and the count reader admits
  only /1–/4 (`SuiteReference::new` in `crates/govern/aep-domain/src/ess_conformance_v2.rs`). An
  ordinary suite from /6 on therefore has no reader.

## Acceptance

1. A report/2 and the exact ordinary suite it ran, both written by ess 0.56.0 or newer (an even
   major, no `coverage` block, report `coverage.knowledge: unknown`), are recorded by
   `aep plan artifact evidence --from <report> --suite <suite>`: exit 0, one `ess_conformance_v2` record (the kind a report/2 already records)
   with the report's counts, source and instant.
2. The suite stays bound by digest: a suite whose bytes differ from the report's `suite.digest` is
   refused, and so is a report whose `suite.version` differs from the suite's.
3. Routing is by what the suite is, not by its number alone: a coverage major still goes to the
   coverage reader (the existing `current_suites` tests stay green), an ordinary major the build
   does not know is refused by name as an unsupported suite version, never as a missing field.
4. A fixture pair written by a released ess (0.56.0 or newer) is committed with the tests, with a
   note naming the command that wrote it.
5. Spec first: the admission outcomes are declared in `ess/`, projections regenerated, `task
   ess-gate` green; where ESS cannot express them, the gap is reported instead of a parallel model.

## Out of Scope

Admitting suites from an ESS release newer than the fixtures; reading scenario bodies.
