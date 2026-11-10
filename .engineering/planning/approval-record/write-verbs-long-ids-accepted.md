---
format: aep.planning-md/3
id: approval-record:write-verbs-long-ids-accepted
kind: approval-record
status: draft
title: 'Accepted: the long-artifact-id defect, under the operator''s ess goal'
tags:
- non-interactive
relations:
- decides: story:write-verbs-refuse-long-artifact-ids
revision: 1
---
# Approval record: the long-artifact-id defect, accepted under the operator's active goal

## Which stop

Acceptance of `story:write-verbs-refuse-long-artifact-ids`, which moves it to the accepted status
that the story lifecycle names. Reason `non-interactive`: the coordinating session runs as a
sub-agent. The decision reached it through the orchestrating session on 2026-11-10, and no operator
turn exists inside this run.

## The operator's goal, as relayed by the orchestrating session (2026-11-10)

> zero remaining issues on github, all bugs fixed from planning store for repo ess

The defect blocks that goal. In the ess store, an archived review-result cannot be given its
`reviews` relation, because `aep plan artifact relate` refuses the pair of long ids ("invalid
idempotency key"). `aep plan artifact validate` therefore exits 1 there, and no verb in aep 0.79.0
or 0.79.1 repairs it without a hand edit.

The orchestrating session decided to fix it as its own unit in a separate pull request, not riding
the store-pack feature pull request. Nothing else is accepted by this record.
