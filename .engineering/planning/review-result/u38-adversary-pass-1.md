---
format: aep.planning-md/2
id: review-result:u38-adversary-pass-1
kind: review-result
status: active
title: u38 adversary pass 1 (findings prose and JSON)
relations:
- reviews: story:review-findings-accept-prose-and-json
revision: 1
---
Adversary pass 1 on wave unit u38 (`story:review-findings-accept-prose-and-json`, #38). Tree
`wave-20260925-u38` on `b3087b3bb9` plus the implementor's uncommitted diff.

Header as returned: verdict NEEDS-CHANGE; cases executed 847→857, red 6; origin introduced 4,
pre-existing 1, undecided 0. Added `crates/plan/aep-backend-markdown/tests/findings_adversary.rs`
and one case in `planning_cli.rs`.

Coordinator routing: findings 1–4 back to the implementor; finding 5 (pre-existing) fixed in the
same unit because it lives in the same functions as finding 4.

```findings
[{"file":"crates/plan/aep-backend-markdown/src/findings.rs","line":594,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"render_block writes U+0085, U+007F and C1 controls raw, so parse folds NEL to a space and refuses DEL/C1 that parse_json accepted"},{"file":"crates/plan/aep-backend-markdown/src/findings.rs","line":441,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"without_positions leaves libyaml's 'at position N' in the refusal and the line points at the array opener, not the offending line"},{"file":"crates/plan/aep-backend-markdown/src/findings.rs","line":582,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a vocabulary value written with JSON escapes is not located and the refusal reports line 1 quoting '['"},{"file":"crates/edge/aep-cli/src/planning.rs","line":4067,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"prose quoting an example findings block inside a longer fence is refused as ambiguous with --findings"},{"file":"crates/plan/aep-backend-markdown/src/findings.rs","line":481,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"block() reads a findings fence quoted inside a longer fence as the review's findings, ignoring the real block"}]
```
