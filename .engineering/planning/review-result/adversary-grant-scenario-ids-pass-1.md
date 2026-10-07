---
format: aep.planning-md/3
id: review-result:adversary-grant-scenario-ids-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: evidence admits the scenario ids ESS 0.55.0 synthesizes'
relations:
- reviews: story:ess-grant-scenario-ids
revision: 1
---
```
unit: story:ess-grant-scenario-ids
verdict: red
cases: executed 1032→1040, red 2
origin: introduced 2, pre-existing 1, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes
```

Covers the uncommitted tree `<worktree>` (aep-u88) on base `65937c3c1`. The implementation files have not changed since 11:09, and I did not touch them.

**1. What I touched.** `git diff --stat` is unchanged from the implementor's: 8 files, +1057/−52. Everything I added is untracked and is test or fixture only:
- `<worktree>/crates/observe/aep-ess-evidence/tests/grant_ids_adversary_pass1.rs` (6 cases)
- `<worktree>/crates/govern/aep-domain/tests/scenario_id_adversary_pass1.rs` (1 case)
- `<worktree>/crates/edge/aep-cli/tests/grant_ids_adversary_pass1.rs` (1 case)
- `<worktree>/crates/observe/aep-ess-evidence/tests/fixtures/adversary-88/` (19 files, about 204K, all written by `ess 0.55.0`; the README there gives each command)

**2. Cases, each run alone before the suite**

| Case | Asserts | Now |
|---|---|---|
| `a_seed_record_ess_0_55_refuses_is_not_admitted` | 6 seed-record edits that ESS 0.55.0 refuses (I measured each with `ess verify conform report`) are refused by AEP | **red** |
| `a_later_form_in_the_transcribed_suite_5_is_refused_naming_the_form_and_its_major` | at suite/5, 7 later forms are refused as `UnsupportedVocabulary` naming the form and major | **red** |
| 4 cases with ESS-written suites | admitted: disclosure cells sent by actors (external and ESS-run reports), a selected child of those cells, a component-scoped grant suite, authored refusals 038 and 039 beside grant scenarios | green |
| `scenario_ids_are_admitted_exactly_where_ess_0_55_parses_them` | `ScenarioId::new` agrees with ESS on 109 ids, each one checked through `ess verify conform select` | green, 0 disagreements |
| CLI case | `aep plan artifact evidence --from … --suite/--suite-input` records 5 reports (/35 grant, /35 read-grant, /43 seeds, /35 authored, /35 disclosure child) | green |

Red output, verbatim:
```
admitted seed records ESS 0.55.0 refuses: [
    "InvalidSynthesisSeeds: invalid source digest",
    "InvalidSynthesisSeeds: an application names an authored scenario",
    "InvalidSynthesisSeeds: an application names an unknown selection",
    "InvalidSynthesisSeeds: a selection names an unknown source",
    "InvalidSynthesisSeeds: selections must be sorted and distinct",
    "InvalidSuite: invalid instance name identifier \"Not Kebab\"",
]
suite/5 refusals: [ "demo.core.Reads/aggregate: MalformedScenarioId at $suite.scenarios.demo.core.Reads/aggregate: …",
  … the same for binding/final-failure, grant/denied, grant/admitted, grant/read/denied, disclosure, binding/refusal ]
```

**3. Suite runs.** All use `cargo test -p <crate> --no-fail-fast` under the build lock.

| Package | Result | Exit |
|---|---|---|
| aep-ess-evidence | 84 existing pass; my file `4 passed; 2 failed` | 101 |
| aep-domain | 343 passed | 0 |
| aep-cli | 607 passed, 0 failed | 0 |

- **Count:** the "before" number (1032) is this same run minus my three test binaries (6 + 1 + 1 cases).
- **Clippy:** `--all-targets -D warnings` exits 0 on all three crates (aep-cli after I split my test, which was over the 100-line limit).
- **Format:** `rustfmt --check` exits 0 on my files.
- **Disk:** 18G free at the last cargo command, so close to the 15G stop line.

**4. Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `coverage_suite.rs:231` (`admit_seeds`) | INFEASIBLE | introduced | It only checks the JSON structure (ESS's `admit_json`). ESS's other seed checks are skipped (`admit_selections`, the id-only parts of `admit_applications`, the typed name grammars), and none of them reads a scenario body. | Only a hand-built suite plus a report naming its digest. ESS never writes one, and `ess … report` refuses it. |
| `count_suite.rs:70`, reached from `coverage_suite.rs:126` | INFEASIBLE | pre-existing | At the transcribed suite/5, `ScenarioId::frozen` refuses every later form as `MalformedScenarioId` before the major gate runs. The acceptance says these must be `UnsupportedVocabulary` naming the form and the major. At base, aggregate and final-failure fail the same way. | A suite/5 carrying a later key. ESS writes these forms only from /17. It is still refused, just under the wrong code. |
| `coverage_suite.rs:701` | INFEASIBLE | introduced | The unit test checks the suite/5 refusal by calling `admit_id_forms` directly, which the reader never does for these ids at /5. It passes while the reader does something else. | Nothing beyond the row above. |

Fixes, named but not applied:
- **Seeds (row 1):** add ESS's selection- and application-level checks to `admit_seeds`: the existing `original_digest` check on source digests, sorted and distinct entries, every source and selection known, no authored scenario named, at most 64 selections, and the instance/entity/state grammars.
- **suite/5 (rows 2 and 3):** in the suite/5 branch, read the keys with `ScenarioId::new` and `admit_id_forms` before `count_suite::admit_scenarios`.

**5. Attacked, could not break**
- **Grammar:** 109 measured ids, no disagreement with ESS. The 4 grant forms, binding refusal, the condition aspects and disclosure cells mirror ESS 0.55.0's `parse` and `Cell::parse`.
- **Major gates:** all seven match ESS's constants (aggregate 16; final-failure and grants 26; read grants and disclosure 34; binding refusal and conditions 36).
- **Wildcard matcher:** no admitted id spells another form.
- **Coverage majors:** odd /5–/45, matching ESS `is_coverage_version`.
- **Authored codes:** 1–41 except 36, matching ESS's catalogue; ESS's reader ties no authored code to a major.
- **`scenario_initial_state` and seeds by major:** the rules match ESS `validate_suite`.
- **Lineage:** a child whose seed decimal is respelled (0.1 vs 0.10000000000000001) is admitted by ESS 0.55.0 as well, so AEP agrees with ESS there.
- **Frozen grammar and count reader:** unchanged.
- **Fixture honesty:** 5 of the unit's fixtures and all 14 of mine reproduce byte-for-byte.

**6. Paths written outside the worktree:** none. Scratch is in `<worktree>/target/scratch/adversary`. The CLI case leaves three 52K stores in `<worktree>/target/ess-conformance-coverage/fixtures/adversary-88-*`. Lease released.

**Your decision:** whether rows 1 and 2 (both INFEASIBLE by reach) are fixed in this unit or moved to their own story, and whether my fixtures and the 6 green cases are kept.

```findings
- file: crates/observe/aep-ess-evidence/src/coverage_suite.rs
  line: 231
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: admit_seeds admits six seed records ESS 0.55.0 refuses (bad source digest, authored or unknown application target, unknown source, unsorted selections, malformed instance), none of which needs a scenario body
- file: crates/observe/aep-ess-evidence/src/count_suite.rs
  line: 70
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: at the transcribed suite/5 every later id form is refused as MalformedScenarioId by ScenarioId::frozen, not as UnsupportedVocabulary naming the form and major as the acceptance requires
- file: crates/observe/aep-ess-evidence/src/coverage_suite.rs
  line: 701
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the unit test checks the suite/5 refusal by calling admit_id_forms directly, a path the reader never reaches for later forms at /5, so it stays green while the reader returns MalformedScenarioId
```
