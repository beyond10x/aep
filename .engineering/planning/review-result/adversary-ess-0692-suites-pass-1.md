---
format: aep.planning-md/3
id: review-result:adversary-ess-0692-suites-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: evidence admits the ESS 0.69.2 suite majors'
relations:
- reviews: story:evidence-admits-ess-0692-suites
revision: 1
---
```
unit: story:evidence-admits-ess-0692-suites, uncommitted tree on base 4c324a31d
verdict: NEEDS-CHANGE (F1), resolved in the unit before commit
cases: 4 written, run by the coordinator once disk allowed; the CI case moved to xtask/tests/ess_gate.rs
origin: introduced 4 / pre-existing 0 / undecided 0
```

**Cases** (`crates/observe/aep-ess-evidence/tests/ess_0692_adversary_pass1.rs`, plus one in
`xtask/tests/ess_gate.rs`)

| case | asserts |
|---|---|
| `the_pin_is_the_release_ci_installs` (extended) | `ESS_SHA256` in `.github/workflows/ci.yml` is the `x86_64-unknown-linux-gnu` line of ess 0.69.2's `SHA256SUMS` |
| `a_newer_coverage_suite_with_a_member_this_build_does_not_know_is_refused_by_version` | a /59 suite with an unknown root or provenance member is `UnsupportedSuiteVersion`, never `UnknownField` |
| `the_refusal_codes_ess_added_from_018_to_022_carry_the_effects_ess_assigns` | at /57, 018 and 019 are `candidate_not_emitted`, 020 to 022 `check_not_emitted`; a flipped effect is `RefusalEffectMismatch` |
| `the_new_id_arms_admit_and_refuse_exactly_what_ess_parse_does` | 6 admitted and 19 refused ids across the held, invariant-wrapped and view-reference forms |

**Findings**

| # | where | finding | verdict | resolution |
|---|---|---|---|---|
| F1 | `.github/workflows/ci.yml` | `ESS_VERSION` moved to 0.69.2 with 0.68.0's `ESS_SHA256`, so the install step's `grep -Fx` fails every CI run | NEEDS-CHANGE | checksum set to ess 0.69.2's; the ESS gate test now holds it beside the pin |
| F2 | `coverage_suite.rs` `admit_known_version` | no test failed with the guard deleted | CONFIRMED (warning) | pinned by the /59 unknown-member case |
| F3 | `values.rs` refusal effects | effect mapping of 018 to 022 untested | CONFIRMED (note) | pinned by the effects case |
| F4 | `ess_conformance_v2.rs` new id arms | no malformed id of the three new forms was refused by a test | CONFIRMED (note) | pinned by the id-arms case |

**Checked against ESS 0.69.2 and held**: the three id arms match ESS's parser predicates; floors
50, 52, 54; seeds 42 to 57; root catalogs from /52; 023 and 024 from /53; authored codes 1 to 46
without 036; report/2 unchanged since 0.55.0; the ordinary reader refuses an unknown version
before reading members; ESS itself refuses a seeded selected child from /47.

```findings
- file: .github/workflows/ci.yml
  line: 93
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ESS_VERSION moved to 0.69.2 with 0.68.0's ESS_SHA256 left in place, so the install step's grep -Fx fails and CI cannot run
- file: crates/observe/aep-ess-evidence/src/coverage_suite.rs
  line: 207
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: no test can fail if admit_known_version is deleted, because the /59 case carries only members this build allows
- file: crates/govern/aep-domain/src/ess_conformance_coverage/values.rs
  line: 485
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the effect mapping for ESS-SYNTH-018 to 022 is untested, so dropping 20|21|22 or adding 18/19 to the check set survives
- file: crates/govern/aep-domain/src/ess_conformance_v2.rs
  line: 150
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no test refuses a malformed held, wrapped-invariant or view-reference id, so any condition of the new arms can be dropped unnoticed
```
