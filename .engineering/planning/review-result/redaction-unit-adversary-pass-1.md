---
format: aep.planning-md/3
id: review-result:redaction-unit-adversary-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: can a redaction hide an edit or leave the literal behind'
relations:
- reviews: story:immutable-record-text-is-redacted-by-record
revision: 1
---
unit: story:immutable-record-text-is-redacted-by-record — uncommitted tree on base 85e3322b4, committed as 0f18a5409
verdict: red
cases: executed 792→796, red 4
origin: introduced 7, pre-existing 0, undecided 0
wrote-outside-worktree: the wave's scratch directory only (probe scripts and logs)
needs-coordinator: yes — J1 and J2 are design decisions the story specified

Adversary pass on the `redact` verb and the `validate` replay. Four defects, each with a failing case
in `crates/edge/aep-cli/tests/record_redaction_adversary.rs`, and three judgement findings.

| id | severity | where | finding | disposition |
|---|---|---|---|---|
| F1 | blocker | `planning.rs`, the evidence write | `--reason` was written verbatim, so a reason naming the literal put it back under `.engineering/` | fixed: a reason holding the literal in any letter case is refused before any write |
| F2 | major | `planning.rs`, `redacted_text` | `summary` kept the literal, and `set` is refused on a review-result | fixed: summary is redacted, the count totals title, summary and body, the replay covers all three |
| F3 | minor | `record_redaction.rs`, the window walk | a hand-written width near `u64::MAX` made `validate` panic on overflow | fixed: the walk adds only widths the text holds; such a record matches nothing and is reported |
| F4 | minor | `record_redaction.rs`, the matcher | a literal whose lowercase is longer than itself (`İ`) left its lowercase spelling | fixed: windows are measured after lowercasing; a literal inside a character that lowercases to two is refused |
| J1 | major | `review_problems` | a redaction admits any substitution, so a verdict could be replaced | accepted: the record keeps the replacement and the reason; the story asks for free replacement |
| J2 | major | `folded_digest` | an unsalted SHA-256 lets anyone with a list of candidate names confirm the literal | accepted: the story specifies the digest, and the literal stays in Git history, which this verb does not rewrite |
| J3 | minor | `journal.rs`, replay order | records replay by instant then file name; clock skew between two writers could fail a legitimate store | accepted: infeasible within one writer's sequence-numbered file names |

Attacked without a break: a forged count, a record matching nothing, overlapping windows, a
replacement that recreates the literal, three chained redactions in one second, reordered records, a
redaction before the first commit, a YAML-special replacement in the title, the literal in command
output or error text, each refusal writing nothing, and the store write lock.

After the fix round the four cases pass: `cargo test -p aep-cli --locked` exit 0, 799 executed.


```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 3894
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: redact writes --reason verbatim into the evidence record and history output, so a reason naming the literal puts it back under .engineering/ while the verb reports success
- file: crates/edge/aep-cli/src/planning.rs
  line: 3999
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: redact replaces only title and body, so a review-result summary keeps the literal, set is refused on the kind, and the only route left is the hand edit the docs say redact replaces
- file: crates/edge/aep-cli/src/planning/record_redaction.rs
  line: 46
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a redaction record with literal_chars near u64::MAX makes validate panic with an add overflow instead of reporting a problem
- file: crates/edge/aep-cli/src/planning/record_redaction.rs
  line: 66
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a literal whose to_lowercase is longer than itself (U+0130) leaves its lowercase spelling unredacted because the window width is the literal's own char count
- file: crates/edge/aep-cli/src/planning.rs
  line: 6545
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a redaction admits any substitution, so a review verdict can be flipped and validate passes, with only a digest and the new text on record
- file: crates/edge/aep-cli/src/planning/record_redaction.rs
  line: 17
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the unsalted SHA-256 of a short folded name lets anyone with a list of candidate names confirm the redacted literal
- file: crates/plan/aep-backend-markdown/src/journal.rs
  line: 569
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: replay order is at-then-file-name rather than revision, so clock skew between writers can reverse two interacting redactions and fail a legitimate store
```
