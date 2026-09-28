---
format: aep.planning-md/3
id: review-result:er-direct-return-reader-review
kind: review-result
status: active
title: ER direct-return reader adversary
relations:
- reviews: story:admit-er-direct-return-evidence
revision: 1
---
unit: AEP direct-return evidence reader working tree over b11db555f402aa32a1da9008bec12ad4e0f74b74
verdict: nothing found in the bounded review
cases: executed 69→74 reader cases, red 0; five reviewer cases passed without rerunning the full gate
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned $TASK_SCRATCH files only; exact local paths are in a separate scratch manifest
needs-coordinator: finish the running full gate, record review and publish the compatible reader pin

`git --no-pager diff --stat` showed implementor/coordinator-owned changes:

```text
 CHANGELOG.md                                      |  8 +++
 README.md                                         |  4 ++
 crates/edge/aep-cli/src/planning.rs                 |  7 +--
 crates/edge/aep-cli/tests/ess_conformance_coverage.rs | 72 ++++++++++++++++++++++
 crates/govern/aep-domain/src/ess_conformance_coverage/values.rs | 8 ++--
 crates/govern/aep-domain/src/ess_conformance_v2.rs  |  8 ++-
 crates/observe/aep-ess-evidence/src/count_json.rs   | 22 +++++++
 crates/observe/aep-ess-evidence/src/count_suite.rs  | 33 ++++++++--
 crates/observe/aep-ess-evidence/src/coverage.rs     |  4 +-
 crates/observe/aep-ess-evidence/src/coverage_definition.rs | 29 +++++++--
 crates/observe/aep-ess-evidence/src/coverage_suite.rs | 25 +++++---
 crates/observe/aep-ess-evidence/src/lib.rs          |  1 +
 schemas/generated/task.schema.json                |  4 +-
 13 files changed, 193 insertions(+), 32 deletions(-)
```

New implementation/tests/fixtures/design and the coordinator's planning artifact were untracked during review. Reviewer-attributable repository diff: empty. All probes were Rust tests in assigned scratch; no implementation, planning, repository test or Git file was changed, and no AEP planning command was invoked.

The review covered the new closed direct-response authority, decimal/exponent integer validation, exact parent-definition comparison, ordinary and coverage version references, CLI dispatch, documented support limits, and retained mutation/producer-fixture evidence. The implementor's prior reader result was 69 passing cases, with domain and CLI checks reported separately; these were not rerun before the reviewer cases. All five new cases were written before execution and linked to the actual built reader library.

The first probe accepted exact signed/unsigned range boundaries, shifted decimal forms, signed zero, and zero with extreme signed-64-bit exponents while preserving the original suite string. It rejected the next integers outside i64/u64, fractions which binary64 could round to integers, nonzero huge exponents and an exponent outside the declared range. The exact log is `$TASK_SCRATCH/integer-boundaries.log`.

Four further probes passed in `$TASK_SCRATCH/authority-probes.log`:

- A child retained the exact parent scalar `100e-2`; changing it to mathematically equal `1`, `1.0` or `1e0` was refused as `ParentScenarioMismatch`.
- A 128-alias path was admitted and 129 aliases refused as `ResponseResourceLimit`; unknown field authority was refused as `UnknownField`.
- A whitespace-only change to the suite bytes was refused as `SuiteDigestMismatch` against the original report.
- Optional text-list parent authority retained order and duplicate multiplicity; reordering or removing a duplicate was refused as `ParentScenarioMismatch`.

The documented profile matches the code: String, Boolean, Integer, Optional<String>, Optional<List<String>>, and transparent nominal aliases only. Other types, declarations and unrecognized fields are refused. The 1 MiB original-response-authority limit deliberately counts raw formatting/escaping, and the inherited JSON document-depth cap is stated. No general ESS support is claimed, and intermediate suites/6–27 remain refused rather than being reinterpreted. Suite/28 remains ordinary count evidence with unknown coverage. Suite/29 uses complete inventory and exact parent-chain checks. Existing suite/5 normalization stays in its legacy comparison branch.

I inspected the retained named digest-guard mutation failure and restored passing case, plus the recorded admission of the complete generated ER suite/29. The latter records shape/original-byte admission only and explicitly does not claim an execution report. The CLI fixture covers both raw-suite and input-carrier dispatch, while the existing persistence boundary continues to clear deserialized admission and require re-reading. No compiled ESS modeling dependency was added.

Reviewed source identities (SHA-256):

```text
ea703e22960d865ec8c96b47d91a3c3a5ae8ee5bb749fc1f37bc52e3ef537c66  crates/observe/aep-ess-evidence/src/direct_response.rs
c8487056fb11f9bdc0b72b826cf10f5d4bd799918837e562fa862287d439a79a  crates/observe/aep-ess-evidence/src/count_json.rs
2b89392d1865a50d8e47779caafcd6076e128b1f4ab7e09819efb2539599946d  crates/observe/aep-ess-evidence/src/count_suite.rs
a67b77f4d359c53cfa7e9b249aef2e04875cd3152d16969827bf0db9ec6bc473  crates/observe/aep-ess-evidence/src/coverage_definition.rs
8383c70bf403cd1d5c5a05e4750aa6b4256d2eb495267160fd9e11f44c7beaed  crates/observe/aep-ess-evidence/src/coverage_suite.rs
620e5c6861e78d5d1b492781bdc70ffc24aff27b1be3ceaf80121f3a4be03f75  crates/edge/aep-cli/src/planning.rs
```

Retained reviewer files are `$TASK_SCRATCH/probe.rs`, `probe`, `integer-boundaries.log`, `authority-probes.log`, and `report.md`. `$TASK_SCRATCH` denotes the coordinator-assigned AEP review directory; exact local paths and linked-library identity are in the separate local `scratch-manifest.md`. The reviewer released its own lease. This report is bounded agent review, not approval or independent verifier evidence.

```findings
[]
```
