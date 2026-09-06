---
format: aep.planning-md/1
id: review-result:ess-conformance-v2-binding-review-pass-1
kind: review-result
status: active
title: ESS count-stage binding review
relations:
- reviews: story:admit-ess-conformance-v2-counts
revision: 1
---
unit: story:admit-ess-conformance-v2-counts — bounded binding-draft review at AEP 00c742e4179593738a2e8aa69e2ecc07d3c89402
verdict: NEEDS-CHANGE
cases: executable cases not applicable (draft/source review); no builds or tests run
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: bind legacy numeric ObservedAt compatibility before source implementation

1. Actual checkout state

```text
$ git --no-pager diff --stat
 .engineering/planning/journal.jsonl | 25 +++++++++++++++++++++++++
 1 file changed, 25 insertions(+)
exit: 0

$ git status --short
 M .engineering/planning/journal.jsonl
?? .engineering/planning/story/admit-ess-conformance-v2-counts.md
exit: 0
```

This is the observed shared checkout state, not a clean-diff claim. This reviewer made no production, planning/store or ref write; those surfaces remain coordinator-owned. The only reviewer write is this assigned ignored report.

2. Frozen reviewed bytes

All three finalized drafts were read completely. Their hashes were checked again after inspection and remained unchanged.

| Draft under target/ess-conformance-v2-reader/binding-drafts/ | SHA-256 |
|---|---|
| ess-conformance-v2-evidence.md | 40977e13640646f8a51fad5a76809a8c6f427f73314d66ae7698f4136514d972 |
| atlas-adr-proposal.md | 37b037df8326d8c8eb6a16a648f491af53eb4888527bcf296d40d5d5e2ae4895 |
| atlas-coordination-story-body.md | 397d085c2771c21d35d8f6cddc3d611e993e15979454c2dd1ec626a66e1a1d2e |

The accepted ESS design in the root ESS checkout hashes to 4d3c0db04da9e63eab3e581fbd5f9f3214311f4df72a9cb3c2e7d4da43244db3, matching the frozen reference in the drafts. Reviewed its count-stage contract, unknown-coverage restriction, exact-byte/timestamp rules and staged rollout. Source inspection used AEP HEAD 00c742e4179593738a2e8aa69e2ecc07d3c89402.

3. Finding

| Location | Finding | Verdict | Origin | Severity |
|---|---|---|---|---|
| target/ess-conformance-v2-reader/binding-drafts/ess-conformance-v2-evidence.md:215 | The shared ObservedAt rewrite leaves historically accepted integral floating/exponent inputs unbound despite the draft's promise to preserve valid legacy behavior. | NEEDS-CHANGE | introduced | warning |

The draft freezes valid historical behavior at line 19, then specifies a direct exact unsigned/date visitor at line 215. Its explicit retained controls cover calendar dates, granularity and future/day behavior, but do not settle the legacy numeric spellings that an unsigned-only visitor would refuse.

At the exact AEP source, ObservedAt::deserialize calls Node::deserialize and then Number::to_string().parse::<u64>() (crates/govern/aep-domain/src/time.rs:647–659). Node converts JSON numbers to f64 (node.rs:33–51), and Number::Display prints integral finite small values as an i64 decimal (facts.rs:109–118, with the predicate at 63–65). Thus the existing source path accepts numerical 1.0 and 1e0 as the instant 1, and negative floating zero as 0. This is a source-traced compatibility observation, not an executed test result.

The path is exposed through the shared EvidenceInput.observed_at (crates/edge/aep-schema/src/parse.rs:324–345), including legacy evidence kinds. Replacing only the shared visitor with unsigned/date cases therefore affects existing non-v2 input, even though new report/2 count/completed_at tokens must be strict. The accepted ESS design explicitly keeps new scalar restrictions separate from legacy domains (review-conformance-coverage.md:174–180).

Bind the distinction explicitly: preserve the established non-v2 numeric admission behavior while making v2 original scalars and envelope precision strict, and add concrete compatibility controls for these existing spellings. If narrowing legacy transport is intended, it needs an explicit compatibility decision instead of the blanket preservation claim. This does not require a generic Number/Node redesign. The defect here is the introduced ambiguity in the implementation contract; the existing conversion path is not counted as a new source finding.

4. Bounded checks with no further finding

- Raw trust: immutable original strings, a nonserialized reading, raw Deserialize with no facts/model claim, and forced admission at submission/direct-record/restore close the stated document/cache bypasses. A custom Rust reader remains an explicitly trusted extension.
- Mutation and restore: preparation before links, observation refresh, records, facts or events addresses the current mutation order at engine.rs:393–425. Whole-candidate restore rejects without returning a partial execution; IDs, observation/production times, order and arrival states remain historical inputs.
- Exact count/time representation: checked u64 parsing/arithmetic and decimal Text facts avoid the legacy usize/binary64/i64 narrowing paths. The planning branch explicitly permits full-range wire values and requires a pre-mutation refusal if a concrete narrower adaptation cannot represent one.
- Independent qualification: expectations come from resolved task constraints, not projected record facts. Both requirement evaluation and evidence.missing use one per-record decision; wrong-subject or split records cannot supply each other's missing conditions.
- Count-stage coverage: suite/1–4 pairing remains diagnostic and UnknownCoverage; no dormant complete-inventory branch or false positive complete-conformance fixture is prescribed.
- Opt-in compatibility: the chosen new policy leaves preserve defaults, both real report readers are owned, detailed run/2 is refused by standalone dispatch, and the ADR/story distinguish source publication from installations and later coverage/default migration.

No production implementation, executable test, model validation, full gate, deployed consumer review or external consumer re-inventory was performed. This bounded review does not establish implementation readiness beyond the draft findings above.

5. Writes and handoff

Only target/ess-conformance-v2-reader/binding-review-pass-1.md was written. No earlier report or draft was edited. Writes are relinquished; the coordinator owns the binding decision and subsequent implementation.

```findings
- file: target/ess-conformance-v2-reader/binding-drafts/ess-conformance-v2-evidence.md
  line: 215
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The shared ObservedAt rewrite leaves historically accepted integral floating/exponent inputs unbound despite the draft's promise to preserve valid legacy behavior.
```

