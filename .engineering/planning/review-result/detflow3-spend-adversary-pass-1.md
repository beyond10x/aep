---
format: aep.planning-md/3
id: review-result:detflow3-spend-adversary-pass-1
kind: review-result
status: active
title: detflow-3 spend adversary pass 1
relations:
- reviews: story:spend-against-baseline
revision: 1
---
Adversary pass 1 on wave detflow-3's unit `story:spend-against-baseline`, tree `aep-df3-spend`
on `13302200a`.

Header as returned: verdict NEEDS-CHANGE; eight findings, all introduced by the unit. The red
cases are kept as `crates/edge/aep-cli/tests/adversary_spend.rs`. F1-F5 and F8 were fixed in the
unit (`f3d93895d`): a calendar date is digits only, an empty `spend` or kind is refused naming it,
the spend bound's schema is closed and requires both values, a refusal is checked to write
nothing anywhere in the tree, a non-integer figure is refused, and `--baseline 0` is refused.
F6 and F7 stay open and are named on the wave's pull request: the spend refusals reuse the
generic codes `conflicting_declaration` and `type_mismatch`, and a SQLite or Postgres plan
prints "no task_spend records" because only the Git store holds evidence files.

```findings
[{"file":"crates/govern/aep-domain/src/time.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a calendar date with a signed month or day was read as a valid date (fixed)"},{"file":"crates/govern/aep-domain/src/project.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"spend or a kind written with no value was accepted instead of refused naming it (fixed)"},{"file":"schemas/generated/project.schema.json","line":1,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the spend bound schema admitted unknown figures and a missing baseline or target (fixed)"},{"file":"crates/edge/aep-cli/src/planning/spend.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a refused spend report was checked to write nothing only in the store, not in the whole tree (fixed)"},{"file":"crates/govern/aep-domain/src/project.rs","line":1,"category":"property","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a non-integer baseline or target had no refusal test (fixed)"},{"file":"crates/edge/aep-cli/src/planning/spend.rs","line":1,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"spend refusals reuse the generic codes conflicting_declaration and type_mismatch (open)"},{"file":"crates/edge/aep-cli/src/planning/spend.rs","line":1,"category":"acceptance","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a SQLite or Postgres plan prints no task_spend records instead of saying evidence files are Git-store only (open)"},{"file":"crates/edge/aep-cli/src/planning/spend.rs","line":1,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"--baseline 0 took a median over no artifacts instead of being refused (fixed)"}]
```
