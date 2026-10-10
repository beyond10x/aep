---
format: aep.planning-md/3
id: review-result:adversary-generated-runner-profile-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: evidence records reports from ESS''s generated runners'
relations:
- reviews: story:evidence-admits-generated-runner-profile
revision: 1
---
```
unit: story:evidence-admits-generated-runner-profile
verdict: confirmed (one contract-drift warning in documentation; no failing case)
cases: none added
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: whether the two design documents must be updated before the wave ships
```

Covers commit `8b1f70bb1` on `wave/go-profile`.

**Attacked and not broken**

- The `go-scenario-status/2` status rule in both readers (`profile_status` in
  `crates/govern/aep-domain/src/ess_conformance_v2.rs`, `execution_status` in
  `crates/govern/aep-domain/src/ess_conformance_coverage.rs`) matches ESS 0.57.0's own
  `execution()` in `ess-conformance/src/counts.rs`.
- Conformance status: count-stage gives failed when execution failed, else inconclusive; coverage
  passes only when nonempty and complete; both match ESS's `qualification()`.
- Mutants caught by the existing tests: dropping `unsupported` from the failed branch, dropping
  `skipped` from the inconclusive branch, checking inconclusive before failed; a stated status that
  disagrees with the counts is refused (`ExecutionStatusMismatch`) in both readers.
- Counts against outcome lists are shared and unchanged.
- `go-scenario-status/1` still refuses error and unsupported counts; `/3`, `/0`, a bare name, a
  trailing space, a `;runner=` suffix and case variants are refused.
- The TypeScript runner writes the same closed key set the readers accept.
- The `### Fixed` entry names only behaviour present at 0.71.1.

**Finding**

The count-stage and coverage design documents still listed only the earlier profiles and lacked
the `go-scenario-status/2` execution rule.

```findings
[
  {
    "file": "docs/design/ess-conformance-v2-evidence.md",
    "line": 60,
    "category": "contract-drift",
    "severity": "warning",
    "verdict": "CONFIRMED",
    "origin": "introduced",
    "message": "the count-stage and coverage design documents (also ess-conformance-coverage-evidence.md:197 and the rules at v2 lines 78-79) list only rust/1, go/1 and external profiles and lack the go-scenario-status/2 execution rule both readers now admit"
  }
]
```
