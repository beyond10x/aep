<!-- Rendered from `adp/default/1` by `protocol workflow instruct`. Do not edit: change the workflow document or the principles timed against its phases, and render again. -->

# Standard development workflow

`adp/default/1` · 9 states · 9 transitions · 20 principles bind it.

Specification, decomposition, verifier setup, implementation, verification, adversarial
verification and review, with an explicit route back to implementation when verification fails.

## How to read this

Work moves through the states below and through nothing else. It starts in `receive`. You may
not enter a state until a transition into it opens, and a transition opens only when its guard
holds and everything listed under it has been met. An unobserved fact does not open one: where a
guard reads something nobody has recorded, the move stays shut, because not knowing is not the
same as knowing it is fine.

## The states

### 1. `receive` — Receive

Take in the request and record what is being asked for, unedited.

It belongs to phase `intake`.

From here you may move to:

* **`specify`** — nothing gates this move.
  Intake is done when there is something to specify; nothing else gates it.

### 2. `specify` — Specify

State the required behaviour. Anything left only in the prompt cannot be checked later, so this
is where an objective becomes something a verifier can disagree with.

It belongs to phase `specification`.

From here you may move to:

* **`decompose`** — only while `artifact.specification.exists`.
  Work is only broken up once there is a specification to break up.

### 3. `decompose` — Decompose

Break the specification into units that can each be implemented and verified on their own.

It belongs to phases `decomposition` and `planning`.

From here you may move to:

* **`establish_verifiers`** — nothing gates this move.
  With units identified, each one's acceptance check can be written.

### 4. `establish_verifiers` — Establish verifiers

Write the tests, contracts and checks that will decide whether the work is done — before there
is an implementation for them to be shaped around.

It belongs to phase `verification-setup`.

From here you may move to:

* **`implement`** — only while `test.exists`.
  A test must exist before implementation begins. This is the mechanical form of test-driven
  work: the guard, not the agent's good intentions, is what enforces the order.

### 5. `implement` — Implement

Make the smallest change that satisfies the decomposed unit and its verifiers.

It belongs to phase `implementation`.

These obligations fall due before you may enter it:

* `clean-room/before-implementation` — Clean-room reimplementation
* `design-by-contract/before-implementation` — Design by contract
* `incremental-decomposition/before-implementation` — Incremental decomposition
* `spec-driven/before-implementation` — Specification before implementation
* `test-driven/before-implementation` — Test-driven development

From here you may move to:

* **`verify`** — only while `diff.exists`.
  There is nothing to verify until the working tree has actually changed.

### 6. `verify` — Verify

Run the verifiers established earlier against the change.

It belongs to phase `verification`.

These obligations hold while you are here, and are checked as you leave:

* `adversarial-verification/during-verification` — Adversarial verification

From here you may move to:

* **`adversarial_verify`** — only while `(tests.unit.failed == 0 and tests.contract.failed == 0 and static_analysis.errors == 0)`.
  Unit and contract suites green and static analysis clean. Contracts are in the guard because a
  change that passes its own tests can still break a consumer.
* **`implement`** — only while `(tests.unit.failed > 0 or tests.contract.failed > 0 or static_analysis.errors > 0)`.
  Verification failed, so the work goes back to implementation. Any failing check sends it back,
  not just the unit suite — a static-analysis error is a defect that has not been symptomatic
  yet.

### 7. `adversarial_verify` — Adversarial verify

Try to break what the previous state just declared working: mutants, edge cases, property
violations, contract drift.

It belongs to phase `adversarial-verification`.

You may not enter it until all of this holds:

* evidence test_result from test-runner (independent)

From here you may move to:

* **`review`** — only while `evidence.missing == 0`.
  No required evidence outstanding. A reviewer reading a change with gaps in its evidence is
  being asked to guess, and will guess in favour of the author.

### 8. `review` — Review

Judge the change as a whole — intent, fit and risk, not just green checks.

It belongs to phase `review`.

These obligations fall due before you may enter it:

* `contract-testing/before-review` — Contract testing

From here you may move to:

* **`complete`** — only while `review.approved`.
  Review concluded in approval.

### 9. `complete` — Complete

The change is verified, reviewed and accounted for.

It belongs to phase `completion`.

These obligations fall due before you may enter it:

* `approval-gates/before-completion` — Approval gates
* `blast-radius-limitation/before-completion` — Blast-radius limitation
* `clean-room/before-completion` — Clean-room reimplementation
* `contract-testing/before-completion` — Contract testing
* `design-by-contract/before-completion` — Design by contract
* `differential-testing/before-completion` — Differential testing
* `ess-conformance/before-completion` — Conformance to the specification
* `invariant-checking/before-completion` — Invariant checking
* `mutation-testing/before-completion` — Mutation testing
* `property-based-testing/before-completion` — Property-based testing
* `provenance-tracking/before-completion` — Provenance tracking
* `reversible-changes/before-completion` — Reversible changes
* `spec-driven/before-completion` — Specification before implementation
* `static-analysis/before-completion` — Static analysis
* `test-driven/before-completion` — Test-driven development
* `verify-after-action/before-completion` — Verify after action

The workflow ends here. Nothing leaves this state, and reaching it is what finishing means.

## What binds you here

These principles reach this workflow: each times an obligation against a phase one of its states
declares, withdraws a capability, or requires evidence that must exist before the work is
finished. A principle applies unless the condition under *Applies when* is observed to be false
— an unobserved condition does not switch a principle off.

### `adversarial-verification/1` — Adversarial verification

A finding counts only once something independent has tried to refute it and failed. The agent
that produced the work is the worst available judge of whether the work is right.

**While in `verify`, the verification phase:**

* evidence verification (independent)

If one of its requirements is not met: block.

### `approval-gates/1` — Approval gates

Production mutation and irreversible steps need an approval a person actually granted, recorded
where an auditor can find it. Without it, the record of who agreed to the change is whatever the
agent wrote in its own summary.

**Before entering `complete`, the completion phase:**

* if defined(deployment.production.status) then: evidence approval from human-approval (independent); approval production-change (by a person)

You may use `production.write` and `deployment.create:production` only with a recorded approval.

If one of its requirements is not met: block.

### `blast-radius-limitation/1` — Blast-radius limitation

A change acts through the narrowest mechanism that can fix the problem, and how far it reaches
is measured before it is made. Without it a fault in one service is answered with a fleet-wide
restart, and the outage ends up bigger than the bug.

Applies when `task.kind in [incident, release, migration]`.

**Before entering `complete`, the completion phase:**

* verification.blast-radius.passed
* (not (defined(evidence.first_seq.deployment_result)) or evidence.first_seq.verification < evidence.first_seq.deployment_result)
* evidence verification (independent)

You may not use `production.write`. This is withdrawn, not gated: there is no approval that
returns it.

You may use `deployment.create` only with a recorded approval.

If one of its requirements is not met: block.

### `clean-room/1` — Clean-room reimplementation

A reimplementation is built from the specification alone: the original is not fetched, and
something other than the agent has to state that it wasn't.

Applies when `change.clean_room == true`.

**Before entering `complete`, the completion phase:**

* verification.clean-room.passed

**Before entering `implement`, the implementation phase:**

* artifact specification (approved)

Evidence it requires, which must exist before the work is finished:

* evidence verification (independent)

You may not use `network.read`. This is withdrawn, not gated: there is no approval that returns
it.

If one of its requirements is not met: block.

### `contract-testing/1` — Contract testing

Published interfaces stay compatible: contract checks pass and report no breaking change before
a task can complete. Without it, a compatible-looking refactor ships as someone else's outage.

Applies when `(task.kind in [feature, bugfix, refactor] and change.code)`.

**Before entering `complete`, the completion phase:**

* contracts.checked > 0
* contracts.failed == 0
* contracts.breaking_changes == 0

**Before entering `review`, the review phase:**

* contracts.breaking_changes == 0

Evidence it requires, which must exist before the work is finished:

* evidence contract_result from contract-runner (independent)

Verifiers that must have spoken:

* contract-runner must run — before entering `complete`, the completion phase

If one of its requirements is not met: block.

### `design-by-contract/1` — Design by contract

Preconditions, postconditions and invariants are written down in an approved design and then
checked by a machine. Unchecked, a contract is a comment, and the caller who violates it finds
out in production.

Applies when `task.kind in [feature, bugfix, refactor, migration]`.

**Before entering `complete`, the completion phase:**

* (verification.precondition.passed or property_test.precondition.result == passed)
* (verification.postcondition.passed or property_test.postcondition.result == passed)
* (verification.invariant.passed or property_test.invariant.result == passed)

**Before entering `implement`, the implementation phase:**

* artifact.design.sections_present
* artifact design (approved)

If one of its requirements is not met: block.

### `differential-testing/1` — Differential testing

A change that claims to preserve behaviour must be shown to preserve it, by running the old and
the new implementation against the same inputs. Otherwise "pure refactor" is an assertion by the
party least able to check it.

Applies when `(task.kind == refactor or change.behaviour_preserving)`.

**Before entering `complete`, the completion phase:**

* tests.differential.failed == 0
* verification.differential.passed

Evidence it requires, which must exist before the work is finished:

* evidence test_result from test-runner (independent)
* evidence verification (independent)

If one of its requirements is not met: block.

### `ess-conformance/1` — Conformance to the specification

Where an executable system specification governs the work, the implementation must be checked
against that specification's own generated suite, by something other than the agent that wrote
the implementation.

**Before entering `complete`, the completion phase:**

* if artifact.executable-system-specification.exists then: ess_conformance.passed; ess_conformance.scenarios.failed == 0; evidence ess_conformance from conformance-runner (independent); artifact executable-system-specification

Verifiers that must have spoken:

* conformance-runner must run — before entering `complete`, the completion phase

If one of its requirements is not met: block.

### `incremental-decomposition/1` — Incremental decomposition

Work is broken into named pieces with stated acceptance criteria before it is built. Without it,
review happens once, at the end, on everything — which is the point at which nobody can say
whether a change was in scope.

Applies when `task.kind in [feature, bugfix, refactor, migration]`.

**Before entering `implement`, the implementation phase:**

* (artifact.story.exists or artifact.task.exists)
* artifact acceptance-criteria

If one of its requirements is not met: block.

### `invariant-checking/1` — Invariant checking

An invariant stated in a design document is a comment until something checks it. This obliges
the stated invariants to be machine-checked, and obliges them to have been stated in the first
place.

**Before entering `complete`, the completion phase:**

* verification.invariant.passed
* artifact design (approved)

Evidence it requires, which must exist before the work is finished:

* evidence verification (independent)

If one of its requirements is not met: block.

### `least-privilege/1` — Least privilege

An agent holds the capabilities its task needs and nothing else, so the damage a wrong
instruction can do is bounded by the task rather than by the token.

You may not use `secret.read`. This is withdrawn, not gated: there is no approval that returns
it.

You may use `network.write`, `production.write` and `deployment.create` only with a recorded
approval.

If one of its requirements is not met: block.

### `mutation-testing/1` — Mutation testing

Tests must be shown to detect defects, not merely to pass. A suite that stays green while the
code under it is broken on purpose is a suite that would not have caught the bug you are about
to ship.

**Before entering `complete`, the completion phase:**

* tests.mutation.failed == 0
* verification.mutation.passed

Evidence it requires, which must exist before the work is finished:

* evidence test_result from test-runner (independent)
* evidence verification (independent)

If one of its requirements is not met: block.

### `progressive-delivery/1` — Progressive delivery

Production is the last environment a release reaches, and each stage is watched before the next
one starts. Without it "we canaried it" means the change was in front of 1% of users for as long
as it took to click promote — every user gets the bad version, just in two batches.

Applies when `task.kind == release`.

You may use `deployment.create:production` only with a recorded approval.

If one of its requirements is not met: block.

It also obliges `before phase promotion` and `during phase observation`, timings no state of
this workflow declares, so nothing here comes due for them. They are named so that a reader
knows this document is a view of the principle and not the whole of it.

### `property-based-testing/1` — Property-based testing

Named properties must hold over generated inputs, not just over the examples someone thought of.
A suite of examples documents the cases its author imagined and says nothing about the rest.

Applies when `change.code`.

**Before entering `complete`, the completion phase:**

* evidence.count.property_test_result >= 1

Evidence it requires, which must exist before the work is finished:

* evidence property_test_result from property-tester (independent)

If one of its requirements is not met: block.

### `provenance-tracking/1` — Provenance tracking

Every claim a finished run makes points at the thing that produced it. Without it, six months
later nobody can tell whether a test result came from a test runner or from an agent's summary
of one, and the only way to find out is to run everything again.

**Before entering `complete`, the completion phase:**

* evidence verification (independent)
* if task.kind not in [incident] then: evidence diff

If one of its requirements is not met: block.

### `reversible-changes/1` — Reversible changes

A change either has a known previous revision to go back to, or it is not finished. Without it
the recovery plan gets invented while the service is down, by whoever happens to be awake.

**Before entering `complete`, the completion phase:**

* if defined(deployment.revision) then: deployment.previous_revision.exists

If one of its requirements is not met: roll back (requires deployment.previous_revision.exists).

### `spec-driven/1` — Specification before implementation

An approved specification must exist before implementation starts, and every requirement it
states must be satisfied before the task completes. Without it, "done" means whatever the
implementer remembers wanting.

Applies when `task.kind in [feature, bugfix, refactor, migration]`.

**Before entering `complete`, the completion phase:**

* specification.satisfied

**Before entering `implement`, the implementation phase:**

* artifact specification (approved)

Evidence it requires, which must exist before the work is finished:

* evidence specification

If one of its requirements is not met: block.

### `static-analysis/1` — Static analysis

No task completes with an unresolved static-analysis error. Without it, a defect the compiler or
the linter already found ships anyway, because nobody was obliged to read the output.

**Before entering `complete`, the completion phase:**

* static_analysis.errors == 0

Evidence it requires, which must exist before the work is finished:

* evidence static_analysis (independent)

If one of its requirements is not met: block.

### `test-driven/1` — Test-driven development

A failing test must exist before the implementation that makes it pass, and the suite must be
green at completion. Without the ordering, a passing test only proves the code does what it
does.

Applies when `task.kind in [feature, bugfix]`.

**Before entering `complete`, the completion phase:**

* tests.unit.failed == 0
* regression_suite.result == passed
* evidence.first_seq.test_result < evidence.first_seq.diff

**Before entering `implement`, the implementation phase:**

* test.exists
* test.first_result == failed

Evidence it requires, which must exist before the work is finished:

* evidence test_result from test-runner (independent)
* evidence diff

Verifiers that must have spoken:

* test-runner must run — before entering `complete`, the completion phase

If one of its requirements is not met: block.

### `verify-after-action/1` — Verify after action

A production change is not finished until something other than the agent that made it says the
service is healthy. Without it the run ends green on its own say-so, and the first party to
discover the service is still down is a customer.

**Before entering `complete`, the completion phase:**

* service.health == healthy
* verification.recovery.passed
* evidence health_observation (independent)
* evidence verification (independent)

If one of its requirements is not met: escalate to oncall.

