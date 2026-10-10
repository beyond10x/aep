---
format: aep.planning-md/3
id: story:evidence-admits-generated-runner-profile
kind: story
status: active
title: Evidence records a report from ESS's generated Go and TypeScript runners
relations:
- serves: vision:O2
- informed_by: story:evidence-admits-ordinary-ess-suites
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T03:23:33Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T03:23:33Z", actor: "human:timo", revision: 3}
---
# Story: Evidence records a report from ESS's generated Go and TypeScript runners

## Outcome

`aep plan artifact evidence <id> --from <report/2> --suite <suite>` records a report whose
`producer_profile` is `go-scenario-status/2`, the profile ESS 0.56.0 and later write from their
generated Go and TypeScript runners, instead of refusing it as `UnsupportedProducerProfile`.

## Context

A consumer on aep 0.71.1 ran the Go package ESS 0.56.0 generates with `ESS_REPORT_FORMAT=2 go test`:
format `ess-conformance-report/2`, `producer_profile: go-scenario-status/2`, policy
`complete-selection/1`, suite written by `ess verify conform synthesize --target ir` with a digest
equal to the report's `suite.digest`. The suite check passes; the report is refused
`UnsupportedProducerProfile at $.producer_profile: go-scenario-status/2`.

- Cause: `ProducerProfile::from_wire` in `crates/govern/aep-domain/src/ess_conformance_v2.rs`
  admits only `rust-scenario-status/1`, `go-scenario-status/1` and
  `external-scenario-status/1[;runner=<name>@<version>]`; `counts.rs` and `coverage.rs` in
  `aep-ess-evidence` refuse every other spelling.
- What ESS 0.56.0 and 0.57.0 write (their `website/docs/reference/formats.md`, "Generated Go and
  TypeScript count reports", and `docs/design/review-conformance-coverage.md`, "Generated runtime
  category parity"): Rust `rust-scenario-status/1`; generated Go and TypeScript
  `go-scenario-status/2`; supplied results `external-scenario-status/1`, optionally with
  `;runner=`. `go-scenario-status/1` stays readable for older reports. Unknown profiles are
  refused.
- `go-scenario-status/2` carries all five categories: passed, failed, error, unsupported, skipped.
  Execution is failed when any scenario failed or was unsupported; otherwise inconclusive when any
  errored or was skipped; otherwise passed. `go-scenario-status/1` keeps its restriction (no error,
  no unsupported).

## Acceptance

1. A report/2 with `producer_profile: go-scenario-status/2` and the exact suite it ran is recorded
   by `aep plan artifact evidence --from <report> --suite <suite>`: exit 0, one record carrying the
   report's counts: `ess_conformance_v2` for an ordinary or count-stage suite,
   `ess_conformance_coverage_v1` for a coverage suite (the kind each route already writes).
2. The execution status under `go-scenario-status/2` follows ESS's rule: failed or unsupported
   gives failed, else error or skipped gives inconclusive, else passed; each branch has a test.
3. `go-scenario-status/1` still refuses error and unsupported counts (`ProfileOutcomeMismatch`), and
   an unknown spelling such as `go-scenario-status/3` is still `UnsupportedProducerProfile`.
4. The test fixture follows the consumer's report shape quoted above (format, profile, policy,
   suite digest), with a note naming the ESS release and command it reproduces.
5. Spec first: the admitted producer profiles and their execution rule are declared in `ess/`,
   projections regenerated, `ess-gate` green; where ESS cannot express them, the gap is reported
   instead of a parallel model.

## Out of Scope

Profiles ESS has not released; reading scenario bodies; the report/1 path.
