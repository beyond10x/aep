---
title: Facts, predicates and vocabulary
sidebar_position: 3
description: The declared capabilities, evidence kinds and verifiers, the fact spellings the engine actually projects, and the predicate syntax.
---

# Facts, predicates and vocabulary

The vocabulary is declared by the protocol documents (`protocols/aep/1.yaml` and its extensions);
a document that uses anything outside it fails validation. This page mirrors the repository's
authoring brief.

## Capabilities

Nothing else may appear in any `capabilities:` block:

```text
repository.read  repository.write  tests.execute  command.execute
network.read[:audience]  network.write  telemetry.read
production.read  production.write
deployment.create[:environment]  deployment.rollback[:environment]
secret.read  artifact.read  artifact.write  planning.read  planning.write
review.request  approval.request
```

Environments: `development`, `test`, `staging`, `production`, or omitted (every environment).

Audiences: `public`, `private`, or omitted (every audience). `network.read:private` covers a read of
correspondence addressed to a bounded audience — a direct message, a group DM, a private channel, a
mailbox, a ticket's internal comment; `network.read:public` covers material published to an
unbounded one. **Membership is irrelevant**: a token that *can* read a direct message is exactly the
case a denial is for. A harness that cannot tell which audience a read will reach must refuse it
rather than guess, and it does not have to remember to — a read that does not say its audience asks
for the unscoped wildcard, and `network.read:public` does not cover that.

**Approval floor:** `production.write`, `deployment.create:production` and `network.read:private` may
never appear in a profile's `allow`; resolution refuses otherwise. A broad grant counts as granting
what it covers, so `allow: [network.read]` is refused on its own and accepted beside
`deny: [network.read:private]` — which is how a profile says *read the public channels, never a
DM*.

## Evidence kinds

```text
test_result  static_analysis  contract_result  property_test_result  deployment_result
metric_observation  health_observation  approval  diff  artifact  review  verification
specification
```

`adp/1` adds two more, each minted by the verb that ran the check rather than by the party under
review:

```text
ess_conformance     an implementation against a specification — protocol ess conform evidence
trace_conformance   an agent run against a trace-spec/1 document — protocol trace evidence
```

See [Verify an implementation](../guides/verify-conformance.md) and
[Check a transcript](../guides/check-a-transcript.md).

## Verifiers

```text
compiler  test-runner  contract-runner  static-analyzer  property-tester  model-checker
telemetry-query  policy-engine  human-approval  human-review  artifact-validator
```

`adp/1` adds `conformance-runner` and `trace-checker`, the producers of the two evidence kinds it
declares.

`aep_engine::engine::kinds_for_verifier` maps each verifier class to the evidence kinds it can
produce.

## Artifact kinds and statuses

Kinds: `vision product-requirements initiative epic story task specification acceptance-criteria
design feature-design component-design architecture-design api-design data-design
architecture-decision-record test-plan evaluation-plan verification-report review-result
approval-record release-plan migration-plan runbook incident-report postmortem`, plus
`executable-system-specification` under `adp/1`. Requiring `design` is satisfied by any design
subkind.

Statuses: `draft proposed in_review approved accepted rejected active implemented superseded
archived`. Requiring `approved` is satisfied by `accepted`, `active` or `implemented` too.

**Both vocabularies are open to authors.** Since `0.13.0` a kind and a status may be any name *some
ladder declares* — the two lists above are the built-ins, not the boundary. Adding a kind of work
this repository never anticipated is a `<kind>.yaml` in your own document tree, with no change to
any crate; see [Lifecycles, decided as data](../concepts/lifecycles.md).

Open to authors is not open to anything. A status is accepted because a ladder declares it, never
because it parses, so `drafted` is refused where `draft` was meant — a typo one letter from a
built-in is not a new rung.

**`evidence_kinds`, above, is deliberately closed**, and the reason is the general one for closing a
vocabulary here: an open evidence vocabulary would let a caller invent the kind of proof a gate is
asking for. Openness is decided per vocabulary and the reason is written down; no vocabulary here is
closed by accident.

### Blocker types

A blocker is typed by **what would clear it**, and the type is the *kind*: `credential-blocker`,
`decision-blocker`, `third-party-blocker`. Every one of them reaches the single `blocker` ladder by
its last hyphen segment, so a type costs a name in your own tree and nothing else — no document per
type, no enum, no release.

`artifacts/kinds/blocker.yaml` ships six as a **starting set, not a boundary**, each with what
clears it:

```text
decision  review  credential  third-party  capacity  deploy
```

The list is open, and nothing checks a type against it: `procurement-blocker` works with nothing
added anywhere. The six are the ones an adopter reported, written down so a team has a shared
spelling to converge on rather than six spellings of *waiting on IT*.

Why type at all: five items blocked on one decision is one meeting, five items blocked on five
different things is five conversations, and a plan that records neither cannot tell them apart.
`protocol artifact blocked` groups by the blocker and prints the type; `list` and `board` mark the
blocked artifact with it.

A blocker may also name the evidence it is withholding — `withholds: test_result` — which is the
join between a blocker and an evidence gate: the rung wants a `test_result`, the job that would
produce one cannot mint a token, and `protocol artifact explain` says so. That value comes from the
**closed** evidence vocabulary above, deliberately: the type of blocker is yours, and the kind of
proof it is stopping is the engine's.

## Phases

`aep/1`: `intake specification planning implementation verification review learning completion`.
`adp/1` adds `decomposition verification-setup adversarial-verification`.
`aop/1` adds `detection triage diagnosis mitigation recovery qualification staging canary
observation promotion preparation migration`.

## Observable fact families

A predicate may only read these:

```text
task.**  change.**  risk  severity
state.**  workflow.**  principle.**  evidence.**  required_evidence.**
tests.**  test.**  unit_tests.**  contract_tests.**  regression_suite.**
static_analysis.**  contracts.**  property_test.**  coverage.**
specification.**  diff.**  source_diff.**  artifact.**  review.**  verification.**
approval.**  approvals.**  deployment.**  metric.**  service.**

adp/1 adds: ess_conformance.**  trace_conformance.**
            mutation.**  differential.**  invariant.**  clean_room.**  build.**  types.**
aop/1 adds: incident.**  blast_radius.**  slo.**  release.**  rollout.**  runbook.**  migration.**
            error_rate  recovery_verified
```

Scales for `>=` on non-numeric values: `risk: low<medium<high<critical`,
`severity: info<low<medium<high<critical`, `health: unhealthy<degraded<healthy`.

## Facts the engine projects

A family being declared is necessary but not sufficient — use these spellings. A fact in a declared
family with a spelling nothing projects passes validation and then never becomes true.

| Fact | From |
|---|---|
| `task.id`, `task.kind`, `task.objective`, `task.profile` | the task |
| `tests.<suite>.{passed,failed,skipped,total,result,exists}` | a `test_result`; suites: `unit integration contract property regression mutation fuzz differential e2e smoke` |
| `test.result`, `test.exists`, `test.first_result` | most recent / first test run |
| `unit_tests.failed`, `contract_tests.failed`, `regression_suite.result` | aliases kept for the design documents' examples; accepted on input, canonical forms are what the engine emits |
| `static_analysis.{errors,warnings,result,exists}` | a `static_analysis` |
| `contracts.{checked,failed,breaking_changes,result,exists}` | a `contract_result` |
| `property_test.<claim>.{result,passed,cases,seed,exists}` | a `property_test_result`; `seed` is what re-runs the search that found a counterexample |
| `deployment.{status,succeeded,environment,revision}`, `deployment.previous_revision.exists`, `deployment.<env>.status` | a `deployment_result` |
| `metric.<name>` and bare `<name>` | a `metric_observation` |
| `service.health`, `service.<service>.health` | a `health_observation` |
| `approval.<id>.{granted,decision,by_human}` | an `approval` |
| `diff.{exists,files_changed,lines_added,lines_removed}`, `source_diff.exists` | a `diff` |
| `artifact.<kind>.{exists,count,approved,approved.count,<status>.count}` | the artifact graph |
| `artifact.<kind>.{schema_valid,sections_present,reviewed,current,relationship_valid}` | an `artifact` observation |
| `review.<subject-kind>.{result,approved,blocking_findings,by_human}`, `review.{result,approved}` | a `review` |
| `verification.<claim>.{status,passed}` | a `verification` |
| `specification.satisfied`, `specification.requirements.{total,satisfied}`, `specification.unsatisfied.count` | a `specification` |
| `ess_conformance.{status,passed,spec_version,spec_digest}`, `ess_conformance.scenarios.{total,failed}` | an `ess_conformance` (`adp/1`) |
| `trace_conformance.{status,passed,specification,spec_digest,transcript_digest}`, `trace_conformance.expectations.{total,gapped,unknown}` | a `trace_conformance` (`adp/1`) |
| `state.current`, `state.<id>.entered`, `workflow.terminal` | the engine |
| `evidence.count.<kind>`, `evidence.first_seq.<kind>`, `evidence.last_seq.<kind>` | the engine |
| `evidence.missing`, `evidence.lapsed`, `required_evidence.missing`, `approvals.granted`, `principle.<id>.active` | the engine |

`evidence.first_seq.<kind>` is submission order — how ordering rules become checkable:
`evidence.first_seq.test_result < evidence.first_seq.diff` says a test ran before any code changed.

Both `passed` facts read pessimistically: a conformance record claiming a pass alongside failed
scenarios or gapped expectations is contradicting itself, and the fact does not take the optimistic
half of that. `spec_digest` sits beside the label so a rule can pin the specification a task is
governed by rather than trust a name two models can share; `trace_conformance.transcript_digest`
pins the *run*, where `spec_digest` pins only the document it was judged against.

Verification claim ids are singular and shared across documents: `precondition postcondition
invariant hypothesis recovery blast-radius clean-room differential mutation migration dry-run`.
Reuse one before inventing another — `invariant` and `invariants` are different claims, and evidence
for one does not satisfy a requirement for the other.

## Predicate syntax

Compact form, one predicate per list item:

```yaml
- tests.unit.failed == 0
- error_rate < service.slo.error_threshold     # dotted right-hand side = another fact
- test.result == failed                        # bare word = a literal
- release.version == "1.2.3"                   # quote a literal containing dots
- specification.satisfied                      # bare path: present and truthy
- defined(deployment.previous_revision)
- not change.architectural
```

Structured form:

```yaml
all: [ ... ]          # a bare list is also an implicit `all`
any: [ ... ]
not: <predicate>
task.kind: {any_of: [feature, bugfix]}
risk: {gte: medium}
change.architectural: true
```

Operators in mapping form: `eq ne lt lte gt gte any_of none_of exists truthy`.

Evaluation is three-valued — see [Evidence and completion](../concepts/evidence.md). A fact nobody
has observed reads Unknown; so does an evidence requirement whose most recent record is older than
the `horizon:` it declares. Unknown blocks a transition exactly as False does, but it means
something different and wants a different response — run something, rather than fix something.
Horizons are in the [document reference](./documents.md#requirement-sets).

A horizon reaches this table too. A lapsed record's facts are withheld from the fact store under the
strictest horizon the resolved plan declares for that kind, so a guard reading `tests.unit.failed`
directly rather than through a requirement decays the same way instead of running on a stale number.
`evidence.lapsed` counts those separately from `evidence.missing`, so a gate nobody has looked at
recently is distinguishable from a gate nobody has ever met.
