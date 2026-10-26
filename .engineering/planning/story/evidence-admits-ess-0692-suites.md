---
format: aep.planning-md/3
id: story:evidence-admits-ess-0692-suites
kind: story
status: implemented
title: Evidence admits the suite majors ESS 0.69.2 writes
summary: aep 0.77.1 refuses an ess-conformance/54 suite from ess 0.69.2 as UnsupportedSuiteVersion; the admitted majors end at /44 and /45 (ESS 0.55.0).
owner: project
tags:
- evidence
relations:
- serves: vision:O2
- informed_by: epic:evidence-gated-completion
- informed_by: story:evidence-admits-ordinary-ess-suites
- informed_by: story:current-ess-coverage-suites
scope:
- confidence: cited
  path: .github/workflows/ci.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-cli/tests/ess_conformance_coverage.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/ess_ordinary_suites.rs
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
- confidence: cited
  path: crates/observe/aep-ess-evidence/src/ordinary_suite.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/current_suites.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/ess_0692_adversary_pass1.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/ess_0692_suites.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/fixtures/ess-0692
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/ordinary_suites.rs
- confidence: cited
  path: crates/observe/aep-ess-evidence/tests/suite_route.rs
- confidence: cited
  path: ess/domains/evidence.yaml
- confidence: cited
  path: ess/ess-inputs.yaml
- confidence: cited
  path: generated/ess/rust/.ess-output/state.json
- confidence: cited
  path: generated/ess/rust/Cargo.toml
- confidence: cited
  path: generated/ess/rust/types-report.json
- confidence: cited
  path: generated/ess/rust/types.rs
- confidence: cited
  path: xtask/tests/ess_gate.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T17:31:56Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-26T17:31:56Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-26T18:39:04Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: Evidence admits the suite majors ESS 0.69.2 writes

## Outcome

`aep plan artifact evidence <id> --from <report/2> --suite <suite>` records a report/2 against an
ordinary or coverage suite of every major ESS 0.69.2 writes and reads (`ess-conformance/46` through
`/57`), instead of refusing it as `UnsupportedSuiteVersion`. A major newer than the ones this build
lists is still refused by name.

## Context

Reported 2026-10-26 by the agency repository: aep 0.77.1 refuses an `ess-conformance/54` suite
produced by ess 0.69.2; agency recorded its evidence as `test_result` plus `verification` with a
source and a ref instead.

Reproduced on aep 0.77.1 with ess 0.69.2, 2026-10-26: `ess verify conform synthesize --target ir`
over the agency specification writes `ess-conformance/54` (and `/55` with `--suite-format 5`);
`ess verify conform run --target interpreted --report-format 2` writes a report/2 per suite; on a
scratch store `aep plan artifact evidence story:repro --from report54.json --suite suite54.json`
exits 1 with `UnsupportedSuiteVersion at $suite.provenance.suite_version: ess-conformance/54 is
not a suite version this build admits`, and the same for `/55`.

Cause: `ORDINARY_SUITE_MAJORS` (`crates/govern/aep-domain/src/ess_conformance_v2.rs`) and
`COVERAGE_SUITE_MAJORS` (`crates/govern/aep-domain/src/ess_conformance_coverage/values.rs`) were
transcribed from ESS 0.55.0 and end at /44 and /45. ESS added the pairs 46/47 (0.57.0), 48/49
(0.58.0), 50/51 (0.62.0), 52/53 (0.63.0), 54/55 (0.64.0) and 56/57 (0.67.0); ESS 0.69.2's own
readers admit majors 1–57 (`admission.rs` `validate_suite`). The suite readers also gate, by
major, what ESS added outside scenario bodies that AEP reads: provenance members, scenario-id
forms, coverage refusal codes, and from /52 the suite-level `events` and `one_time_response`
catalogs.

## Acceptance

Conformance scenarios of `aep.evidence.RecordFromReport` (`ess/domains/evidence.yaml`), each held
by a named Rust test:

1. `aep.evidence.RecordFromReport/outcome/recorded` for an ordinary suite of a major added after
   /45: the committed ess 0.69.2 pair (an `ess-conformance/54` suite with its report/2) is recorded
   by the CLI, exit 0, one record with the report's counts.
2. The coverage route for a coverage major added after /45: the committed `ess-conformance/55`
   pair is admitted by the coverage reader.
3. `aep.evidence.RecordFromReport/outcome/unsupported-suite-version` for a major newer than the
   build lists (`ess-conformance/58`, `/59`): refused by name, never as a missing or unknown field.
4. The per-major gates hold for each vocabulary ESS added after /45 that AEP reads (provenance
   members, id forms, refusal codes, suite-level catalogs): present under a lower major, refused as
   `UnsupportedVocabulary`; present at or above its major, admitted.
5. The fixtures are written by a released ess (0.69.2), with a note naming the commands.
   A `--suite-input` lineage whose suites carry a suite-level catalog is refused as
   `UnsupportedVocabulary` naming the catalog, never as a changed survivor.
6. The ESS pin (`ess/ess-inputs.yaml`, `xtask/tests/ess_gate.rs`, CI) is 0.69.2, and the lists
   name the ESS release they were transcribed from; `ess-gate` fails when the pin moves past it,
   so the next ESS major is a reviewed edit rather than a downstream refusal.

## Scope

- `crates/govern/aep-domain/src/ess_conformance_v2.rs` (`ORDINARY_SUITE_MAJORS`, `ScenarioId::new`)
- `crates/govern/aep-domain/src/ess_conformance_coverage/values.rs` (`COVERAGE_SUITE_MAJORS`)
- `crates/observe/aep-ess-evidence/src/coverage_suite.rs`, `ordinary_suite.rs`,
  `coverage_definition.rs`, `coverage_wire.rs` (per-major gates)
- tests and fixtures under `crates/observe/aep-ess-evidence/tests/`,
  `crates/govern/aep-domain/tests/`, `crates/edge/aep-cli/tests/`
- `ess/domains/evidence.yaml` (comments naming the admitted range), `ess/ess-inputs.yaml`,
  `xtask/tests/ess_gate.rs`, `.github/workflows/ci.yml`, `generated/ess/`
- `CHANGELOG.md`

## Decision

Admitted range: the explicit lists, extended to every major ESS 0.69.2 reads (ordinary 6–56 even,
coverage 5–57 odd). No forward-compatible rule (admitting any later major by parity): ESS's own
readers refuse a later major ("a later format may mean something different by the same words",
`scenario.rs` `a_suite_format_from_a_later_build_is_refused_rather_than_guessed`), 52/53 already
added root members a closed reader cannot have known, and 56/57 changed the meaning of an existing
id spelling. Admitting by number would trade a named `UnsupportedSuiteVersion` for an unknown-field
refusal or a silent misreading. The recurring cost is moved instead: the lists carry the ESS
release they were transcribed from, and the ESS gate refuses a pin past it.

Selected lineage from /52 on: ESS 0.69.2's `select` drops catalog entries a child's survivors no
longer need and re-spells their `absent_events` (measured on the fixture: a survivor's `except`
went from `[IncidentOpened]` to `[]` beside a catalog without it), so a survivor's bytes differ from
its parent's while its expansion does not. AEP compares survivors byte for byte and does not
transcribe the expansion; such a lineage is refused by name. Transcribing the expansion is a later
story if a consumer records selected suites.

## Out of Scope

Reading scenario bodies; admitting majors ESS has not released.
