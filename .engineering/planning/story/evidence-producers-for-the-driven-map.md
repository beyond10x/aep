---
format: aep.planning-md/1
id: story:evidence-producers-for-the-driven-map
kind: story
status: draft
title: Every kind the plan demands has a producer the map can name
summary: A driven run under development/default reaches review without --allow-evidence-gap, because a verifier exists for each of the four kinds the coverage pre-flight names — and none of the four is minted from an exit status it cannot honestly come from.
owner: protocol
tags:
- evidence
- driver
relations:
- decomposes: epic:evidence-gated-completion
revision: 1
---
# Story: Every kind the plan demands has a producer the map can name

## Outcome

`protocol drive run --map drivers/development/default.yaml` on a `kind: feature` task passes the
coverage pre-flight with no flag, and a run that walks to `adversarial_verify -> review` is held
there by the guards that read the records — not by records nobody could produce. The refusal that
today names four kinds (`contract_result`, `property_test_result`, `verification`,
`specification` — arm c of pilot 1, `docs/reviews/2026-08-23-three-arm-pilot-1.md`) names zero.

## Context

`crates/aep-driver/src/coverage.rs` (F-W4.2-4) compares the plan's demanded kinds against the
map's declared kinds at launch. For `development/default` under any development profile the gap is
exactly the four kinds above, one per principle: `contract-testing`, `property-based-testing`,
`provenance-tracking`, `spec-driven`. `EvidenceMapping::MINTABLE`
(`crates/aep-driver-spec/src/map.rs`) admits none of the last three from an exit status —
correctly, because their records carry names and counts an exit status does not hold. So each
needs what `trace_conformance` already has: a verifier that writes its own record, read back
through `record:` (`crates/protocol-cli/src/drive.rs`, `read_record`).

The three-arm pilot measured arm c under `--allow-evidence-gap`; the run stops at the guard by
design. This story is the other road the pilot's decision 3 names: after it, arm c runs unflagged.

## The four producers

| kind | producer | status |
|---|---|---|
| `contract_result` | `protocol contract evidence` over the committed consumer record (`crates/protocol-cli/fixtures/metaharness-contract-result-*.json`) | verb ships; the map step is the missing half. `--observed-at` must come from the record's own run date, not a literal in a committed map — that likely means teaching the verb to read the date out of the record it is handed |
| `verification` | a `--evidence <out>` mode on an existing verifier verb (`protocol validate` is the candidate: claim `document-tree-valid`, verifier `protocol`, status from what it found) | new; smallest of the three |
| `property_test_result` | a producer that runs the workspace's proptest suites in-process and writes the record — property name, case count, seed — the way `protocol trace evidence` writes what the checker measured. An `xtask` or a dedicated test target; parsing `cargo test` stdout is not it | new; design needed on where it lives |
| `specification` | a checker that reads the specification artifact's requirements and the run's admitted evidence and writes the requirement-by-requirement verdict (`SpecificationRecord.unsatisfied` names what is not met). What counts as "requirement" in a markdown artifact is the design decision this story must make before any code | new; the expensive one |

## Acceptance

- `protocol drive run --map drivers/development/default.yaml` on a `kind: feature` task prints no
  coverage gap and starts without `--allow-evidence-gap`.
- Each of the four records enters the run via `record:` (or, for `contract_result`, an honestly
  mintable exit-status step if the record path is rejected in design) with `producer: verifier`,
  never `producer: agent`.
- A deleted or malformed record is D5 `Unknown` — the step submits nothing and says why; no kind is
  minted with invented counts (the `MINTABLE` set does not grow beyond what an exit status can
  honestly state).
- The map also submits a `test_result` with `suite: regression`, because
  `regression_suite.result == passed` (`test-driven`) is a completion predicate the coverage scan
  deliberately cannot see (§ "What this cannot see", `coverage.rs`) and the run would otherwise
  walk to completion and block there.
- An operator step asks for the specification's approval before `implement`, the way
  `development/checks` already does — `spec-driven.before_implementation` is a person's act and the
  cargo map today has no step to ask at.

## Out of scope

- Any change to the guards, the principles or the profiles: the demands stay exactly as written;
  this story adds producers, not exemptions.
- The eval corpus and the three-arm matrix: re-running arm c unflagged is the eval programme's
  move, made after this story ships.
