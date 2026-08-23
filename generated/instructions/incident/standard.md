<!-- Rendered from `incident/standard/1` by `protocol workflow instruct`. Do not edit: change the workflow document or the principles timed against its phases, and render again. -->

# Standard incident response

`incident/standard/1` · 7 states · 6 transitions · 21 principles bind it.

Detect, triage, diagnose, mitigate, recover, verify and learn — with telemetry and health facts,
not an agent's confidence, deciding when each stage is finished.

## How to read this

Work moves through the states below and through nothing else. It starts in `detect`. You may not
enter a state until a transition into it opens, and a transition opens only when its guard holds
and everything listed under it has been met. An unobserved fact does not open one: where a guard
reads something nobody has recorded, the move stays shut, because not knowing is not the same as
knowing it is fine.

## The states

### 1. `detect` — Detect

Something is wrong and there is an observation that says so.

It belongs to phase `detection`.

From here you may move to:

* **`triage`** — only while `evidence.count.metric_observation >= 1`.
  Triage starts from a recorded observation. Requiring telemetry here is what stops a rumour in
  a channel from consuming the response.

### 2. `triage` — Triage

Establish scope and severity: what is affected, how badly, and how far it can spread.

It belongs to phase `triage`.

From here you may move to:

* **`diagnose`** — only while `evidence.count.health_observation >= 1`.
  Service health has been observed, so the blast radius is known rather than assumed.

### 3. `diagnose` — Diagnose

Form a hypothesis and test it. An untested hypothesis acted on under pressure is how an incident
acquires a second cause.

It belongs to phase `diagnosis`.

From here you may move to:

* **`mitigate`** — only while `verification.hypothesis.passed`.
  A hypothesis has been tested and held. Acting on an untested theory is the common path to
  making an incident worse, so the guard is the test result, not the theory.
  It also requires: evidence verification (independent).

### 4. `mitigate` — Mitigate

Stop the bleeding. Restoring full service is the next state's job, not this one's.

It belongs to phase `mitigation`.

These obligations fall due before you may enter it:

* `hypothesis-driven-diagnosis/before-mitigation` — Hypothesis-driven diagnosis
* `preserve-evidence/before-mitigation` — Preserve evidence

If a requirement here is not met: roll back (requires deployment.previous_revision.exists).

From here you may move to:

* **`recover`** — only while `service.health >= degraded`.
  The service is no longer down. `degraded` is deliberately enough to leave this state: holding
  out for full health here would keep an incident open while a working workaround serves users.

### 5. `recover` — Recover

Return the service to normal operation, including whatever the mitigation degraded.

It belongs to phase `recovery`.

From here you may move to:

* **`verify`** — only while `service.health == healthy`.
  Health is back to normal; now prove it with something other than an eyeball.

### 6. `verify` — Verify

Confirm recovery against telemetry rather than against the absence of new alerts — an alert that
stopped firing is not the same fact as a service that is serving.

It belongs to phase `verification`.

These obligations hold while you are here, and are checked as you leave:

* `adversarial-verification/during-verification` — Adversarial verification

From here you may move to:

* **`learn`** — only while `(recovery_verified and error_rate < service.slo.error_threshold)`.
  Recovery is verified and the error rate is back under its objective. Both, because a service
  can report healthy while still failing a fraction of requests.

### 7. `learn` — Learn

Record what happened and what changes because of it. This is a state and not an aspiration
because the postmortem is the only output of an incident that prevents the next one.

It belongs to phases `completion` and `learning`.

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

**Before entering `learn`, the completion phase:**

* if defined(deployment.production.status) then: evidence approval from human-approval (independent); approval production-change (by a person)

You may use `production.write` and `deployment.create:production` only with a recorded approval.

If one of its requirements is not met: block.

### `blast-radius-limitation/1` — Blast-radius limitation

A change acts through the narrowest mechanism that can fix the problem, and how far it reaches
is measured before it is made. Without it a fault in one service is answered with a fleet-wide
restart, and the outage ends up bigger than the bug.

Applies when `task.kind in [incident, release, migration]`.

**Before entering `learn`, the completion phase:**

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

**Before entering `learn`, the completion phase:**

* verification.clean-room.passed

Evidence it requires, which must exist before the work is finished:

* evidence verification (independent)

You may not use `network.read`. This is withdrawn, not gated: there is no approval that returns
it.

If one of its requirements is not met: block.

It also obliges `before phase implementation`, a timing no state of this workflow declares, so
nothing here comes due for it. It is named so that a reader knows this document is a view of the
principle and not the whole of it.

### `contract-testing/1` — Contract testing

Published interfaces stay compatible: contract checks pass and report no breaking change before
a task can complete. Without it, a compatible-looking refactor ships as someone else's outage.

Applies when `(task.kind in [feature, bugfix, refactor] and change.code)`.

**Before entering `learn`, the completion phase:**

* contracts.checked > 0
* contracts.failed == 0
* contracts.breaking_changes == 0

Evidence it requires, which must exist before the work is finished:

* evidence contract_result from contract-runner (independent)

Verifiers that must have spoken:

* contract-runner must run — before entering `learn`, the completion phase

If one of its requirements is not met: block.

It also obliges `before phase review`, a timing no state of this workflow declares, so nothing
here comes due for it. It is named so that a reader knows this document is a view of the
principle and not the whole of it.

### `design-by-contract/1` — Design by contract

Preconditions, postconditions and invariants are written down in an approved design and then
checked by a machine. Unchecked, a contract is a comment, and the caller who violates it finds
out in production.

Applies when `task.kind in [feature, bugfix, refactor, migration]`.

**Before entering `learn`, the completion phase:**

* (verification.precondition.passed or property_test.precondition.result == passed)
* (verification.postcondition.passed or property_test.postcondition.result == passed)
* (verification.invariant.passed or property_test.invariant.result == passed)

If one of its requirements is not met: block.

It also obliges `before phase implementation`, a timing no state of this workflow declares, so
nothing here comes due for it. It is named so that a reader knows this document is a view of the
principle and not the whole of it.

### `differential-testing/1` — Differential testing

A change that claims to preserve behaviour must be shown to preserve it, by running the old and
the new implementation against the same inputs. Otherwise "pure refactor" is an assertion by the
party least able to check it.

Applies when `(task.kind == refactor or change.behaviour_preserving)`.

**Before entering `learn`, the completion phase:**

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

**Before entering `learn`, the completion phase:**

* if artifact.executable-system-specification.exists then: ess_conformance.passed; ess_conformance.scenarios.failed == 0; evidence ess_conformance from conformance-runner (independent); artifact executable-system-specification

Verifiers that must have spoken:

* conformance-runner must run — before entering `learn`, the completion phase

If one of its requirements is not met: block.

### `hypothesis-driven-diagnosis/1` — Hypothesis-driven diagnosis

Nothing changes in production until someone has said what they think is wrong and pointed at the
telemetry that says so. Without it the first action taken is the one that felt plausible, and
the restart that "fixed" it also destroyed the evidence that would have contradicted it.

Applies when `task.kind == incident`.

**Before entering `mitigate`, the mitigation phase:**

* verification.hypothesis.passed
* evidence metric_observation from telemetry-query
* evidence verification (independent)

Verifiers that must have spoken:

* telemetry-query must run — before entering `mitigate`, the mitigation phase

If one of its requirements is not met: block.

### `invariant-checking/1` — Invariant checking

An invariant stated in a design document is a comment until something checks it. This obliges
the stated invariants to be machine-checked, and obliges them to have been stated in the first
place.

**Before entering `learn`, the completion phase:**

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

**Before entering `learn`, the completion phase:**

* tests.mutation.failed == 0
* verification.mutation.passed

Evidence it requires, which must exist before the work is finished:

* evidence test_result from test-runner (independent)
* evidence verification (independent)

If one of its requirements is not met: block.

### `preserve-evidence/1` — Preserve evidence

The state that explains the failure is destroyed by the thing that fixes it, so it gets captured
first. Without it the postmortem is written from memory, and the outage recurs because nobody
could say what caused the first one.

Applies when `task.kind == incident`.

**Before entering `mitigate`, the mitigation phase:**

* evidence metric_observation from telemetry-query
* evidence artifact
* artifact incident-report

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

**Before entering `learn`, the completion phase:**

* evidence.count.property_test_result >= 1

Evidence it requires, which must exist before the work is finished:

* evidence property_test_result from property-tester (independent)

If one of its requirements is not met: block.

### `provenance-tracking/1` — Provenance tracking

Every claim a finished run makes points at the thing that produced it. Without it, six months
later nobody can tell whether a test result came from a test runner or from an agent's summary
of one, and the only way to find out is to run everything again.

**Before entering `learn`, the completion phase:**

* evidence verification (independent)
* if task.kind not in [incident] then: evidence diff

If one of its requirements is not met: block.

### `reversible-changes/1` — Reversible changes

A change either has a known previous revision to go back to, or it is not finished. Without it
the recovery plan gets invented while the service is down, by whoever happens to be awake.

**Before entering `learn`, the completion phase:**

* if defined(deployment.revision) then: deployment.previous_revision.exists

If one of its requirements is not met: roll back (requires deployment.previous_revision.exists).

### `spec-driven/1` — Specification before implementation

An approved specification must exist before implementation starts, and every requirement it
states must be satisfied before the task completes. Without it, "done" means whatever the
implementer remembers wanting.

Applies when `task.kind in [feature, bugfix, refactor, migration]`.

**Before entering `learn`, the completion phase:**

* specification.satisfied

Evidence it requires, which must exist before the work is finished:

* evidence specification

If one of its requirements is not met: block.

It also obliges `before phase implementation`, a timing no state of this workflow declares, so
nothing here comes due for it. It is named so that a reader knows this document is a view of the
principle and not the whole of it.

### `static-analysis/1` — Static analysis

No task completes with an unresolved static-analysis error. Without it, a defect the compiler or
the linter already found ships anyway, because nobody was obliged to read the output.

**Before entering `learn`, the completion phase:**

* static_analysis.errors == 0

Evidence it requires, which must exist before the work is finished:

* evidence static_analysis (independent)

If one of its requirements is not met: block.

### `test-driven/1` — Test-driven development

A failing test must exist before the implementation that makes it pass, and the suite must be
green at completion. Without the ordering, a passing test only proves the code does what it
does.

Applies when `task.kind in [feature, bugfix]`.

**Before entering `learn`, the completion phase:**

* tests.unit.failed == 0
* regression_suite.result == passed
* evidence.first_seq.test_result < evidence.first_seq.diff

Evidence it requires, which must exist before the work is finished:

* evidence test_result from test-runner (independent)
* evidence diff

Verifiers that must have spoken:

* test-runner must run — before entering `learn`, the completion phase

If one of its requirements is not met: block.

It also obliges `before phase implementation`, a timing no state of this workflow declares, so
nothing here comes due for it. It is named so that a reader knows this document is a view of the
principle and not the whole of it.

### `verify-after-action/1` — Verify after action

A production change is not finished until something other than the agent that made it says the
service is healthy. Without it the run ends green on its own say-so, and the first party to
discover the service is still down is a customer.

**Before entering `learn`, the completion phase:**

* service.health == healthy
* verification.recovery.passed
* evidence health_observation (independent)
* evidence verification (independent)

If one of its requirements is not met: escalate to oncall.

