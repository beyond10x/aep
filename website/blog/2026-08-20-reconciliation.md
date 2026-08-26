---
title: "0.3.3 — twenty gates, closed before the oracle was allowed to start"
description: >
  Not a feature wave. A mutation review ran twenty single-edit changes against the load-bearing
  rules; seven survived, two survived the whole gate. What a user notices is mostly things this
  repository used to accept and now refuses.
slug: reconciliation
tags: [release, ess, conformance]
---

Wave 4 was going to make the specification judge generated code. Before that could start, four
independent reviews opened **twenty gates**, and this release closes them.

It is not a feature wave. What a user notices is mostly **things this repository used to accept and
now refuses**.

{/* truncate */}

## The mutation review

Twenty single-edit changes were run against the load-bearing rules, to see which edits the test
suite would fail to notice. **Seven survived. Two survived the whole gate.**

The two are worth stating plainly, because both are the kind of hole you cannot find by reading:

* **A reviewer who read a change, refused it, and recorded that refusal was thereby granting the
  production write.** The refusal path granted what the approval path was supposed to.
* **Adding `Deserialize` to a validated type** — the single thing the invariants forbid, because it
  lets a value skip its own constructor — compiled and passed everything.

Both are guarded now. So is the class that hid them, which is the real finding: in every case **the
rule was correct, its doc comment said so, and no fixture anywhere reached the state where the rule
was load-bearing.** `ApprovalDecision::Denied` appeared in zero tests.

A rule with no test that reaches it is a comment.

## Six refusals a user will meet

A saturating numeric version. A duplicated YAML key in any document. A number that cannot
round-trip. A type nested past 32 levels. A lifecycle transition no command causes. An escalation
that says nothing observable.

## Two model gaps, closed because a generated suite would have had nothing to assert

An outcome now declares the entity it acts on, and `on_failure: escalate` names the event it emits.
Both were writable before and unassertable — which only became visible once something was going to
generate tests from them.

Conformance evidence is now **bound to the specification revision it attests, failing closed**.
Unproven is not proven.

`ess-conformance` arrives as the crate that decides whether a candidate input satisfies a guard —
and **refuses rather than guesses** when it cannot be decided.

---

*Written retrospectively from this release's `CHANGELOG.md` section and its annotated tag; no
re-run command output.*
