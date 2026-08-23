---
format: aep.planning-md/1
id: story:contract-result-gates
kind: story
status: implemented
title: The contract runner's record decides a transition, and arrives on a pipe
summary: A contract_result reporting a breaking change stops a change entering review, a record reporting a red run does not, and a count nobody wrote down is refused rather than read as zero.
owner: protocol
tags:
- evidence
- harness
relations:
- decomposes: epic:metaharness-migration
- depends_on: story:contract-result-ingestion
revision: 1
---
# Story: The contract runner's record decides a transition, and arrives on a pipe

## Outcome

The number an outside contract runner measured changes what a run may do. A `contract_result`
reporting a breaking change holds the work out of review; the same run reporting a failure that is
the runner's own machinery does not; and a record that never states a count is refused where it
enters instead of reading as a clean one.

## Context

`story:contract-result-ingestion` built the road and said so in its own *Out of Scope*:

> **Anything gating on the record.** No workflow, profile or step map requires a `contract_result`
> about a metaharness adapter; the verb makes the fact available and nothing yet asks for it.

That is what this story closes, and it is R1.5 of `docs/plan/eval-program-three-arms.md` (= HC-3).
Its second half — reading the record from standard input — was the other named remainder of the same
story, and is what makes the loop a pipe rather than a redirect.

## The gate, and why it reads `breaking_changes` and not `failed`

`principles/development/contract-testing.yaml` gains one timed obligation:

```yaml
requires:
  before_review:
    predicates:
      - contracts.breaking_changes == 0
```

`before_<phase>` is the mechanism the principle vocabulary already has — the same one
`test-driven`'s `before_implementation` uses — so the engine blocks entry to every state declaring
that phase without a line of new code. In `adp/default` that is `adversarial_verify -> review`.

Three decisions are embedded in that one line.

**It reads a different number from the completion obligation, not a stricter copy of it.** `failed`
says *the contract run is red*; `breaking_changes` says *and somebody who already calls this
interface was told something that is no longer true*. A review is exactly the right place for the
first — a person can read a failure, decide it is the runner's own machinery, and say so — and no
place at all for the second, because the people affected are not in the room.

**It is the first guard in that workflow only a contract runner can answer.** The workflow's
`verify -> adversarial_verify` guard reads `tests.contract.failed`, which is an *alias*: any test
runner satisfies it with a suite it happened to name `contract`, and `drivers/development/checks.yaml`
fills it from `protocol validate`. `contracts.breaking_changes` has exactly one producer.

**It is scoped by the principle and not by the workflow.** Put in `workflows/development/default.yaml`
the rule would apply to every ADP profile and every task, and run `W4-2/1` already measured what that
costs: a documentation story walked six states and could not leave `adversarial_verify` because it
owed a record no verifier could produce for it. Inside the principle, `applies_when` still holds — a
task declaring `change.code: false` owes nothing here, and a task declaring nothing owes it in full.

## Acceptance

- A breaking record blocks the transition a clean one permits, with a control that isolates the
  number. **Met**: `a_breaking_record_does_not_reach_review_and_a_red_one_does`
  (`crates/aep-engine/tests/contract_gate.rs`) walks the worked example to `adversarial_verify` and
  reads `adversarial_verify -> review` against three records — `20/0/0`, `20/1/0`, `20/1/1`. Rows two
  and three agree that the run went red and disagree on one line; the test asserts that nothing about
  `contracts.failed` appears in the refusal, so the obligation cannot be rewritten over the other
  count and still pass. Verified by mutation, twice: removing the obligation and swapping it to
  `contracts.failed == 0` both fail the file.
- Missing and null fail closed. **Met**, in two places. In the engine,
  `a_run_that_never_heard_from_a_contract_runner_does_not_enter_review_by_saying_nothing` puts a
  `test_result` named `contract` where the record would be — which is what a driven run actually
  submits — so the walk arrives identically and the count is simply unobserved: the requirement reads
  `unobserved`, not a measured zero, and the transition is refused (invariant 5). At the boundary,
  `a_count_the_record_never_states_is_refused_rather_than_defaulted_to_zero` and
  `a_count_stated_as_null_is_refused_by_the_name_of_the_count` refuse a document that omits `checked`,
  `failed` or `breaking_changes`, because `ContractResult` defaults each to zero and zero on
  `breaking_changes` is the claim the gate above reads as a pass.
- `--record -` reads the record from standard input, end to end. **Met**:
  `the_record_can_arrive_on_a_pipe_and_the_loop_still_closes`
  (`crates/protocol-cli/tests/metaharness_contract_result.rs`) pipes the committed bytes through the
  binary, evaluates the document the engine gets back, and compares it with the same bytes minted
  from the file: identical but for the two lines that name where they came from.
- The existing boundary refusals still hold. **Met**: `checked: 0` and `breaking_changes > failed`
  keep their tests beside the rule and through the binary, unchanged, and
  `a_record_whose_failures_are_all_breaking_is_accepted` still holds the `>=` boundary.

## What the pipe costs, stated rather than implied

`--record <file>` stays the form to reach for. The predecessor story's reason was not that a pipe is
hard but that the bytes the provenance digest names should exist somewhere a later reader can go and
check, and a pipe has nowhere. So the record says which was used — `inputs: [standard input]` against
`inputs: [claude.json]` — and the two are told apart by reading it rather than by trusting the caller
to remember.

## Out of Scope

- **A second gate for the release workflow.** `workflows/releases/progressive.yaml`'s
  `qualify -> stage` already reads `contracts.failed == 0`, and since a breaking change is one of the
  failures, adding `contracts.breaking_changes == 0` beside it would decide nothing. A rule that
  cannot change an outcome is decoration, and this repository does not ship decoration.
- **A driven run minting the adapter's record.** No step of either shipped map runs
  `protocol contract evidence`; the record still arrives because somebody piped it in. That remains
  `story:contract-result-ingestion`'s open question, and it is answered by a step map in the other
  repository if it is answered at all.
- **Attestation.** Unchanged: the digest is over bytes this process was handed, not bytes it watched
  being produced.

## Open Questions

- Whether `contracts.checked > 0` belongs at the review gate too. It would mean a run cannot reach a
  reviewer without a contract runner having actually checked something, which is a stronger claim than
  this story makes and one that would bite every task the principle applies to. Decides: operator.
