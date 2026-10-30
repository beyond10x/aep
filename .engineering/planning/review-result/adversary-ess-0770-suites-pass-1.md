---
format: aep.planning-md/3
id: review-result:adversary-ess-0770-suites-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: the ESS pin moves to 0.77.0'
relations:
- reviews: story:evidence-admits-ess-0770-suites
revision: 1
---
# Adversary pass 1: the ESS pin moves to 0.77.0 (unit B)

Recorded by the wave coordinator from the adversary's report on commit `33e3f2236` (6a8da7158 plus
the adversary's test file and fixtures). Scratch paths in the report are elided.

```
unit: B
verdict: red
cases: executed 130→133, red 1
origin: introduced 5, pre-existing 2, undecided 1
wrote-outside-worktree: assigned scratch directory only
needs-coordinator: no
```

Cases added in `crates/observe/aep-ess-evidence/tests/ess_0770_adversary_pass1.rs`, fixtures under
`tests/fixtures/ess-0770-adversary/`:

- Red on the unit branch: `the_changelog_records_the_suite_majors_ess_0_77_0_adds_and_the_pin`.
  Acceptance 6 asks for an Unreleased entry; the unit's diff does not touch `CHANGELOG.md`.
- Green: `a_seed_suite_58_ess_0_77_0_wrote_itself_is_admitted_with_its_report` and
  `a_seed_suite_59_ess_0_77_0_wrote_itself_is_admitted_with_its_report`: ESS's own seed fixture with a
  `Decimal` in a complete-subject check, written by the 0.77.0 binary at /58 and /59 (13 scenarios,
  2 seed applications, 13 passed); AEP admits both with matching counts, digest and seeded ids.

Full crate run after the cases existed: 133 tests, 1 failed (the changelog case).

Attacked and held: regenerating /58 and /59 with 0.77.0 is byte-identical; the pin's SHA256 equals the
x86_64-linux line of the release's `SHA256SUMS`; `generated/ess/` is a fixed point of 0.77.0; no new
refusal codes, scenario-id grammar, root member, provenance member or report fields between 0.69.2 and
0.77.0; seed admission 42–59 matches `SEED_MAJORS`; ordinary/coverage parity; /60 and /61 refused.

Notes: `ess verify conform select` over a /59 suite writes a selected suite whose lineage carries the
`events` catalog, which AEP refuses by name (`coverage_suite.rs:1032`), a decision recorded in 0.78.0.
From /28 ESS gives a direct-response `expected` payload its own nesting budget while AEP applies one
flat limit of 128 levels (`count_json.rs:59`); not exercised, no producer shown.

```findings
[
  {"file": "CHANGELOG.md", "line": 9, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Acceptance 6 is unmet: the unit adds no Unreleased entry, and CHANGELOG.md names neither ESS 0.77.0 nor ess-conformance/58 and /59. Test: crates/observe/aep-ess-evidence/tests/ess_0770_adversary_pass1.rs::the_changelog_records_the_suite_majors_ess_0_77_0_adds_and_the_pin (red)."},
  {"file": "crates/observe/aep-ess-evidence/src/coverage_suite.rs", "line": 182, "category": "coverage", "severity": "note", "origin": "introduced", "message": "Holds: seed suites that ess 0.77.0 writes itself at /58 (ordinary) and /59 (coverage) are admitted with their reports' counts, not only the relabelled /42 and /43 suites. Tests: ess_0770_adversary_pass1.rs::a_seed_suite_58_ess_0_77_0_wrote_itself_is_admitted_with_its_report and ::a_seed_suite_59_ess_0_77_0_wrote_itself_is_admitted_with_its_report (green)."},
  {"file": "crates/observe/aep-ess-evidence/tests/fixtures/ess-0770/README.md", "category": "provenance", "severity": "note", "origin": "introduced", "message": "Holds: re-running the README commands with the verified 0.77.0 binary reproduces suite-58, suite-59, report-58 and report-59 byte for byte. Test: none (manual regeneration in scratch regen/)."},
  {"file": ".github/workflows/ci.yml", "line": 93, "category": "provenance", "severity": "note", "origin": "introduced", "message": "Holds: ESS_SHA256 and PINNED_LINUX_SHA256 equal the x86_64-unknown-linux-gnu line of the 0.77.0 release SHA256SUMS. Test: none (read-only gh release download)."},
  {"file": "generated/ess/schema/.ess-output/state.json", "category": "provenance", "severity": "note", "origin": "introduced", "message": "Holds: the schema and Rust-types projections are a fixed point of ESS 0.77.0 over ess/, checked on a scratch copy. Test: none (ess generate --check in scratch gen/)."},
  {"file": "crates/observe/aep-ess-evidence/src/coverage_suite.rs", "line": 1032, "category": "edge-case", "severity": "note", "origin": "pre-existing", "message": "Note: ess 0.77.0 select over a /59 suite writes a selected suite whose lineage carries the events catalog, which AEP refuses by name as UnsupportedVocabulary; this was a documented decision in 0.78.0. Test: none."},
  {"file": "crates/observe/aep-ess-evidence/src/coverage_suite.rs", "line": 467, "category": "edge-case", "severity": "note", "origin": "pre-existing", "message": "Holds: AEP accepts a seed application outside a selection only at /43, which matches ESS 0.77.0 because ESS first checks the suite without the coverage inventory. Test: none."},
  {"file": "crates/observe/aep-ess-evidence/src/count_json.rs", "line": 59, "category": "edge-case", "severity": "note", "origin": "undecided", "message": "Note: from /28 on, ESS gives a direct-response expected payload its own nesting budget while AEP applies one flat limit of 128 levels; not exercised, and no producer of such a suite has been shown. Test: none."}
]
```

## Coordinator routing

- F1 (introduced): the CHANGELOG is the coordinator's file in this wave; its Unreleased entry was
  committed on the integration branch before this pass, and the case passes there after the merge
  `18b1229b6` (`cargo test -p aep-ess-evidence --test ess_0770_adversary_pass1`: 3 passed). Outcome fixed.
- F6 (pre-existing, NOTE): the 0.78.0 decision stands; no story. F8 (undecided): no producer shown;
  decided as no-op for this unit, not filed.
