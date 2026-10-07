---
format: aep.planning-md/3
id: review-result:adversary-grant-scenario-ids-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: evidence admits the scenario ids ESS 0.55.0 synthesizes'
relations:
- reviews: story:ess-grant-scenario-ids
revision: 1
---
```
unit: story:ess-grant-scenario-ids
verdict: red
cases: executed 435→437, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes
```

Pass 2 on commit `d12860e0a`. Two red cases are open in the seed checks: one rejects a suite that ESS 0.55.0 writes, one accepts a seed record ESS refuses. The suite/5 path and the moved test hold. The three pass-1 findings are fixed: those 6 cases now pass.

**What I touched** (all untracked except the README; no implementation file):
- `<worktree>/crates/observe/aep-ess-evidence/tests/grant_ids_adversary_pass2.rs` (2 cases)
- `<worktree>/crates/observe/aep-ess-evidence/tests/fixtures/adversary-88/integer-identities/` (model, seed files and 4 files ESS wrote; two independent runs were byte-identical)
- `<worktree>/crates/observe/aep-ess-evidence/tests/fixtures/adversary-88/README.md` (+12 lines saying how those files were written)

**Red cases, each run alone first**

| Case | Red output |
|---|---|
| `a_seeded_suite_ess_0_55_wrote_with_distinct_integer_identities_above_2_53_is_admitted` | `external: InvalidShape at $suite.provenance.synthesis_seeds: two selections share one qualified identity` |
| `seed_applications_out_of_scenario_order_are_refused_as_ess_0_55_refuses_them` | `applications out of ESS's order must be refused` (AEP admitted it) |

**Suite run:** `cargo test -p aep-domain -p aep-ess-evidence --no-fail-fast` exits 101. 435 pass; my two cases fail.
- **Count:** 435 is the coordinator's 458 minus the two aep-cli test binaries, which I did not re-run and did not change.
- **Gates on my file:** `cargo clippy -p aep-ess-evidence --all-targets -- -D warnings` and `rustfmt --check` both exit 0.
- **Disk:** 19G free at every cargo command (stop line 17G).

**Findings**

| file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|
| `coverage_suite.rs:380` | NEEDS-CHANGE | introduced | Seed identities are compared as `aep_domain::Node`, whose numbers are `f64`. ESS wrote two rows with identities `9007199254740992.0` and `9007199254740993`; they round to the same `f64`, so AEP refuses them as duplicates. ESS's own reader and interpreter admitted that suite and passed 8 of 8. | An ESS-written suite: any `Integer` identity at or above 2^53 where two seeded rows differ by less than the `f64` step (e.g. ids of the snowflake kind). |
| `coverage_suite.rs:308` and `:409` | INFEASIBLE | introduced | The comment says ESS orders applications "by its scenario-id type, not by spelling". ESS's `impl Ord for ScenarioId` (`scenario.rs:1045`) orders by the rendered name. So AEP admits applications sorted within a scenario but out of order across scenarios; ESS refuses that with "applications must be sorted and distinct". | Only a hand-built suite; ESS writes them sorted. |
| `coverage_suite.rs:354` and `:404` | CONFIRMED | introduced | `InvalidShape` covers 9 seed refusals that are about references, not shape (unknown source or selection, scenario not held, authored target, duplicate identity, ordering, counts). Elsewhere in the reader it means "expected an object/array/string/boolean" (`count_json.rs:116-167`). The detail text tells the two apart. | Any consumer that branches on the code. |

`MalformedName`, `MalformedSourceDigest`, `MalformedSourceIdentity` and `InvalidDocument` match how the reader already uses them.

Fixes, named but not applied:
- **Row 1:** compare identities the way ESS does, by their rendered JSON (`identity.raw`, or `serde_json::Value`, which keeps integers exact), not as `Node`.
- **Row 2:** check `windows(2)` on `(scenario spelling, establish_step)`, and correct the comment.

**Attacked, could not break**
- **Other seed checks:** source and identity grammars, digest, state, instance and entity names, nesting depth 120, the 64-selection limit, every source used, and the held-or-filtered rule (filtered only at /43) all match ESS's reader.
- **Steps that address a seed row:** these read scenario bodies. ESS-written suites are always consistent, so this gap only matters for hand-built suites.
- **Suite/5 path:** `ScenarioId::new` accepts everything `frozen` accepts, and those ids are classed as frozen, so every key admitted at /5 at base still reaches `count_suite::admit_scenarios` and gets the same result. The only change is that later forms are now `UnsupportedVocabulary`. Where a suite has several defects, which one is reported first can differ.
- **Moved test** (`current_suites.rs:1033`): it goes through `admit_raw`, which is the reader path the evidence command uses.

**Your decision:** whether row 1 is fixed in this unit. It is reachable from suites ESS writes, the same class of failure as issue #88. Rows 2 and 3 can follow the precedent you set for pass 1.

Paths written outside the worktree: none. Scratch is in `<worktree>/target/scratch/adversary-2`. Lease released.

```findings
- file: crates/observe/aep-ess-evidence/src/coverage_suite.rs
  line: 380
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: seed identities are compared as f64 Node values, so a suite ESS 0.55.0 wrote and admitted with integer identities 9007199254740992.0 and 9007199254740993 is refused as two selections sharing one identity
- file: crates/observe/aep-ess-evidence/src/coverage_suite.rs
  line: 308
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: ESS orders seed applications by the rendered scenario id, not by type as the comment says, so AEP admits applications out of order across scenarios that ESS refuses as not sorted
- file: crates/observe/aep-ess-evidence/src/coverage_suite.rs
  line: 354
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: InvalidShape, which elsewhere in the reader means a JSON type mismatch, now also names nine relational seed refusals, so the code alone no longer says what was wrong
```
