---
format: aep.planning-md/3
id: review-result:adversary-detflow-1-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: llm step judgement and the release subcommands'
relations:
- reviews: story:llm-step-declares-judgement
- reviews: story:release-cutting-commands
revision: 1
---
```
unit: story:llm-step-declares-judgement, story:release-cutting-commands
verdict: NEEDS-CHANGE (seven findings, each but one with a failing case)
cases: 7 added at 73a02eb76 on adv/detflow-1; 1 run red, 6 not yet run
origin: introduced 6, pre-existing 0, undecided 1
wrote-outside-worktree: none in the repository
needs-coordinator: run the cases; fix each finding in the wave
```

Covers `wave/detflow-1` at `71f13020c`.

**Attacked and not broken**

- The missing, blank and multi-defect judgement paths, their code and the step path index.
- A judgement on a command or operator step is refused by the closed format, as the
  specification says.
- A no-break-space judgement is blank after trimming and is refused.
- Every map loader goes through the validating conversion.
- A map with no llm step counts 0; a cursor from an earlier build prints the counts as unknown.
- `tag` refuses a tag at another commit and a dirty tree before writing anything; the `pin` resume
  path; numeric version ordering; pre-release tags are filtered out.

```findings
[
  {"file": "crates/drive/aep-driver-spec/src/map.rs", "line": 1228, "category": "boundary", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a blank llm prompt returns before the judgement check, so a missing judgement is not reported beside it"},
  {"file": "crates/drive/aep-driver/src/run.rs", "line": 1106, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a retry-in-flight model call is not counted in the llm steps run, although the specification and the changelog count every attempt"},
  {"file": "schemas/generated/driver-steps.schema.json", "line": 1, "category": "contract-drift", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the published schema accepts a whitespace-only judgement that the loader refuses"},
  {"file": "xtask/src/release_cut.rs", "line": 197, "category": "boundary", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the version parser admits leading zeros, so prepare writes a version Cargo refuses"},
  {"file": "xtask/src/main.rs", "line": 539, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "record treats any test_result naming the short tag commit as the release record and writes nothing"},
  {"file": "xtask/src/release_cut.rs", "line": 470, "category": "boundary", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "tag accepts a lightweight tag at HEAD as the annotated release tag"},
  {"file": "xtask/src/release_cut.rs", "line": 92, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "undecided", "message": "the Release run selection lives in the forge adapter and no test exercises it"}
]
```
