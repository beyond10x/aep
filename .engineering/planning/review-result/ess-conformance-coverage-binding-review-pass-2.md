---
format: aep.planning-md/2
id: review-result:ess-conformance-coverage-binding-review-pass-2
kind: review-result
status: active
title: ESS complete-selection binding correction review
relations:
- reviews: story:admit-ess-conformance-coverage
revision: 1
---
unit: suite/5 binding correction review 2; exact snapshots in binding-review-2/manifest.json
verdict: NEEDS-CHANGE
cases: not executed; document correction review only, no implementation-test result
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: clarify one added sentence about decimal Text truthiness; both prior findings are resolved
git --no-pager diff --stat
```text
 .engineering/planning/journal.jsonl                | 11 +++++
 .../planning/story/review-conformance-coverage.md  | 49 +++++++++++++++++-----
 docs/design/review-conformance-coverage.md         | 23 ++++++++--
 3 files changed, 69 insertions(+), 14 deletions(-)
```

This is the coordinator-owned working diff present at the start of this review, not reviewer changes. Only this report was written, in the assigned scratch directory. The assignment is correction-only and explicitly excludes implementation testing; no suite, compiler, formatter, browser, compatibility harness or executable counterexample ran. This is the second and final planned document review, separate from the closed count-writer implementation wave.

1. Exact covered inputs

Manifest SHA256: `0ccaba137782e7800c9d0520d7fad5c5cab63fd81a5daf84d84a17ac381e2b9d`.
Every snapshot verified against its listed hash. Prior immutable report still hashes to `035d8b8fd665457e387a73edfd47fec9b40fbdc85bc6c93a774b48f3f4f8ada3`.

| Owner | Original source | Bytes | SHA256 |
|---|---|---:|---|
| ess | home-path:sha256:d610381eae78b88c2af2b0b4fc8270a031d804bff428556ebea28f900ba4c4d4 | 73976 | `b8443b858bd35fb3236b3cdf2619f398648abfce800c83f5133a2b4d0abac512` |
| ess | home-path:sha256:268398418f457389d9cb820036107d2c5edf1de22a92c6ce6eb5af67c0cc6929 | 13288 | `6b0e55e00f7d4d35979c3184949d993d15bd3016b510c4eddf05f6394d0073bf` |
| ess | home-path:sha256:2e9f66e127badfb2cbde89b0f6ccfa68d9941de86e56b04b411028af6e138b72 | 8349 | `c0c16675bd98c13ac39fbc1cb86e73728fa11e3f975264a5bb4861127c0a9d04` |
| aep | home-path:sha256:8656238c8f4ea05710738a84edb0b47003b6a7af797f3dbc004568d40bf2a0a0 | 19827 | `15d208bd723841f2343d6ea5c52bf543c1882e03e1be051b119ec7728d3479d2` |
| aep | home-path:sha256:8dc0c4f9624ec19685e372cf7c2676ee790b751435aad15a18f42a403da82610 | 10291 | `52c93f3d55001f05ba47268e4c7c87857e0f4367f7042874b03cad663e7641f6` |
| atlas | home-path:sha256:7df9269199d348858672ca86bb21e078e238d752856987bca7b5d8e92c024e1e | 7123 | `d4f5ee85817c1b6518c2a25ad636d95fca2ce149fb02b2bd53e3413506e3e16c` |

Snapshot paths are specified in manifest.json beneath this report's ess/, aep/ and atlas/ directories. Baseline heads remain ESS `be0eefd7ec125d46bb3b664c4b95b8d638a2b1fe`, AEP `62ef3a73112143453319215b9aa31a6c9626ed5f`, and Atlas `34fa907ff3ffcbdbb6bbf37d4dd674c05904bb20`. File/line references below identify the frozen correction snapshots and the named repository owner.

Read-only `git diff --no-index` comparisons covered all six pairs. ESS accepted design changed by20 insertions/3 deletions; ESS story by3/3; AEP proposal by50/5; AEP story by2/2. The transport proposal and Atlas ADR are byte-identical, with comparison exit0. Changed comparisons exit1 as expected for a document diff; these statuses are not test results.

2. Previous findings and correction disposition

| Previous signature | Disposition in these snapshots | Correction inspected |
|---|---|---|
| AEP docs/design/ess-conformance-coverage-evidence.md:146 / NEEDS-CHANGE / introduced / blocker | Resolved | Lines146–175 now enumerate every fact suffix, type, value and presence rule; lines177–192 define raw/admitted/replaced/deserialized projection, aliases and knowledge/completeness/nonempty distinctions; lines265–267 require exact table-driven assertions including absence and extra aliases. |
| ESS .engineering/planning/story/review-conformance-coverage.md:84 / NEEDS-CHANGE / pre-existing / warning | Resolved | Both lines47 and84 now require strict non-success for incomplete/unknown/empty coverage, allow-incomplete diagnostic execution and pre-execution refusal of the conflicting flag pair. |

The complete fact table now settles the original implementation-critical choice. Counts/times are exact Text; required zero/false facts remain present; component and parent facts have explicit conditional absence; raw/replaced/deserialized sources project no coverage namespace until admission; failed re-admission clears the reading; global qualification and dynamic collection aliases are excluded. The corrected known/complete/nonempty distinction matches complete-selection/1, including empty complete inventory and repeated in-scope refusals. The small wording issue below does not carry forward the previous exhaustive-table blocker.

The accepted ESS design adds a current-source clarification, updates M33/M58 and the operative impact paragraph to ess-impact/3 with ess-diff/2, and retains S9 as a historical observation. This matches the baseline IMPACT_FORMAT at impact.rs:97 and the already reviewed transport's in-memory context/persisted-envelope boundary. Unknown/incomplete v5 coverage and missing exact input/lineage explicitly refuse narrowing. No persisted field or WholeAnswer spelling is introduced by the correction.

The AEP story delta is only draft→proposed and revision23→24. Its acceptance text did not change. The transport and Atlas ADR had no correction delta and were not reopened for a broader review.

3. One new wording warning

| Owner / file:line | Category | Severity | Verdict | Origin | Finding |
|---|---|---|---|---|---|
| AEP / docs/design/ess-conformance-coverage-evidence.md:190 | contract-drift | warning | NEEDS-CHANGE | introduced | The correction says decimal Text does not support truthiness, but the existing AEP predicate semantics treat every nonempty decimal string, including "0", as truthy. |

What was inspected: new lines190–191 say “Numeric-looking Text supports quoted equality, not numeric ordering or truthiness.” At AEP62ef3a7, `crates/govern/aep-domain/src/facts.rs:205–209` defines Text truthiness as nonempty and unequal to the literal string false. `predicate.rs:399–401` resolves a bare path and directly invokes that rule. Therefore a newly specified `counts.total` value Text("0") remains truthy when used as a bare predicate; an absent raw-source fact remains Unknown. This is a direct static source comparison, not an executed test result.

What reaches it: the new table intentionally publishes ordinary FactValue::Text values into existing predicate/FactStore consumers and introduces no namespace-specific predicate override. Someone interpreting the sentence as refusal or false behavior for `ess_conformance_coverage_v1.counts.total` would expect semantics the existing typed owner does not provide. The shared qualifier remains independent and cannot be bypassed by that fact; no coverage qualification defect is claimed.

Precise resolution: replace the sentence with an explicit distinction between generic Text semantics and numeric meaning, for example: “Decimal Text preserves exact digits and supports quoted equality without numeric coercion. Existing Text truthiness remains unchanged: even \"0\" is truthy when present, so use nonempty and failed_zero for the specified Boolean checks. Text ordering follows existing protocol scales and is not integer ordering.” Do not add a new primitive/namespace override or alter the frozen count behavior to make the earlier wording literal.

Origin: introduced by this correction's new paragraph; the first snapshot had no such assertion. Severity is warning because the fact-table contract and safe Boolean alternatives are already explicit and the intended repair is one explanatory sentence, not a new carrier or qualifier design.

4. Limits and relinquishment

Review was restricted to the correction diffs, the two previous signatures and the existing Text predicate owner needed to assess the new wording. No third full document or implementation attack was launched. The earlier review's parent/source/refusal/browser/impact/qualification/replay coverage is retained without being presented as a newly executed check. Browser pairing still makes no F15 fidelity repair claim.

All six correction snapshots and the previous immutable report remain unchanged. No source, test, manifest, lockfile, planning artifact/journal, Git index/ref/object, external path, shared cache or worktree lifecycle was written. This report is the sole authored output. It is immutable on return, and all scratch writes are relinquished. No approval, source correctness, implementation execution or deployment readiness is claimed.

```findings
- file: docs/design/ess-conformance-coverage-evidence.md
  line: 190
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'The correction says decimal Text does not support truthiness, but the existing AEP predicate semantics treat every nonempty decimal string, including "0", as truthy.'
```
