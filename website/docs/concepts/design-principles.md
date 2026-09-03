---
title: Design principles
sidebar_position: 2
description: See one engineering task fail without explicit rules, then replay it through the AEP boundaries that enforce them.
---

# Design principles

A principle is useful only if two independent implementations can disagree about whether it was
followed. AEP turns that disagreement into typed input, a deterministic decision, and an explained
refusal. This page follows one change through both paths so the principles are concrete before they
become a reference list.

The complete example is committed under `examples/development-passkeys/` and replayed by the CLI
and engine integration tests. Commands below run from the AEP repository root against the current
public syntax.

## One change, two paths

Task `AUTH-142` adds passkey authentication. A credential must belong to exactly one user, failed
authentication must never create a session, password authentication must keep working, and the
public authentication API must remain compatible. The design in the example is version 7; the only
human review approved version 3.

### Without AEP: a predictable failure

Imagine giving an agent the requirements and a broad tool set. Its final report says:

```text
Implemented passkeys. Tests pass. The existing API is compatible and the design was approved.
Enabled the feature in production.
```

That report can be sincere and still leave every important question unanswered:

| Shortcut | Concrete consequence |
|---|---|
| “Tests pass” is accepted as completion evidence. | Nobody knows which command ran, which revision it observed, whether regression and contract suites ran, or whether the agent merely restated an expectation. |
| Missing results and failing results are both treated as “not green” — or both as “probably fine.” | The next actor either edits code nobody tested or advances work with no observation at all. |
| A later prompt or tool configuration grants broad production access. | An earlier denial silently disappears, and the agent can deploy before an approval exists. |
| The approval records “design approved” but not which design version Ada reviewed. | After the design changes from version 3 to version 7, Ada's old approval is still treated as current. |
| A provider updates the ADR, then discovers that the ADR it should supersede does not exist. | The caller receives an error after entity state or relations have already changed. Retrying makes the state harder to reconstruct. |
| Model, plugin, time, report, and wire-format assumptions remain implicit. | Nobody can replay which rules ran, which external system produced a claim, or whether today's parser gives yesterday's bytes the same meaning. |

This is not mainly a prompting problem. The loop has no machine-readable distinction between
unobserved, contradicted, and satisfied facts; no monotonic policy merge; no revision identity; and
no atomic mutation contract. A better sentence in the prompt cannot create those boundaries.

### With AEP: replay the same change

The AEP path uses the same task, but every boundary has one job. The following steps show the input,
the decision, and who is responsible for making that decision true.

## 1. Parse syntax, then construct a valid task

The task states only what cannot be derived. Its profile selects the workflow, principles,
completion predicate, and capability policy:

```yaml
id: AUTH-142
kind: feature
objective: add-passkey-support

protocol: adp/1
profile: development.standard
manifest: examples/development-passkeys/artifacts.yaml

constraints:
  facts:
    change.public_contract: true
    change.architectural: false
  notes:
    - Existing password authentication must keep working for the whole rollout.
```

`aep-schema` first parses YAML into raw input types. Domain construction then checks semantic
invariants and accumulates independent validation problems: references must resolve, predicates may
read only declared facts, workflow phases must exist, and a task cannot require a capability the
resolved policy denies. Correct-looking YAML does not bypass those checks.

Build the CLI and resolve the committed task:

```shell-session
$ cargo build --locked -p aep-cli
$ AEP=target/debug/aep
$ "$AEP" resolve --root . --task examples/development-passkeys/task.yaml
task        AUTH-142 (feature)
objective   add-passkey-support
protocol    adp/1
profile     development.standard
workflow    adp/default (initial: receive)
principles  spec-driven, test-driven, static-analysis, least-privilege, provenance-tracking,
            contract-testing, property-based-testing, approval-gates, reversible-changes
...
```

**Responsible boundary:** raw document and schema in `aep-schema`; semantic types and validators in
`aep-domain`; project-wide reference validation in `aep-project`.

## 2. Ask before using a capability

Capability policy is layered with fixed precedence: explicit deny, then approval required, then
allow, then not granted. A narrower document can restrict a grant but cannot silently grant back a
denial. The base protocol also has an approval floor for production-sensitive capabilities.

Ask whether the agent may enable passkeys in production:

```shell-session
$ "$AEP" explain --root . \
    --task examples/development-passkeys/task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --action production.write
production.write denied
  operation: change production state
  reason:    principle approval-gates rule production-write-requires-approval
  missing:   approval for capability production.write
  state:     receive
```

The command exits `1`. “Denied” here means the requested action cannot proceed now; the underlying
policy decision is `RequiresApproval`, and the explanation says exactly what would change it.
`secret.read`, by contrast, is explicitly `Denied` by `least-privilege` and cannot be granted back.

**Responsible boundary:** `CapabilityPolicy` in `aep-domain` resolves the layers; the engine's
`authorize` call records the request and decision and returns the attributable refusal. The harness
must ask before acting and expose only tools covered by the returned policy.

## 3. Submit an observation, not a sentence

The first test is a typed record. This is the complete committed record, not pseudocode:

```yaml
- kind: test_result
  observed_at: 2023-11-12
  suite: unit
  passed: 0
  failed: 1
  producer:
    producer: verifier
    verifier: test-runner
  about: task:AUTH-142
  provenance:
    command: cargo test -p auth passkey_credential_is_scoped_to_one_user
```

The kind projects facts such as `tests.unit.failed`; the engine stamps submission order, so
red-before-diff is checkable rather than aspirational. The producer identity also lets a principle
require an independent verifier instead of accepting the implementing agent's own claim.

**Responsible boundary:** evidence types live in `aep-domain`; the engine validates and projects
submitted records. The external producer is still responsible for running the named command,
reporting the real observation time and identity, and supplying honest provenance.

## 4. Keep `Unknown`, contradiction, and satisfaction separate

Before any evidence is submitted, the CLI reports an unknown test fact:

```shell-session
$ "$AEP" evaluate --root . \
    --task examples/development-passkeys/task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml
state       receive (Receive)
...
  ? tests.unit.failed == 0
      unobserved: tests.unit.failed
```

Submit the red test and advance as far as the evidence permits:

```shell-session
$ "$AEP" evaluate --root . \
    --task examples/development-passkeys/task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/01-red-test.yaml \
    --advance
state       implement (Implement)
...
  ✗ tests.unit.failed == 0
      tests.unit.failed = 1
  ✓ evidence test_result from test-runner (independent)
```

The distinction changes the next action:

| Engine result | Meaning | Correct next action |
|---|---|---|
| `?` / `Unknown` | No applicable observation exists. | Run the verifier that can produce the fact. Do not edit code to satisfy an absent result. |
| `✗` / `False` | An observation contradicts the predicate. | Fix the implementation, then observe it again. |
| `✓` / `True` | An observation exists and satisfies the predicate. | Advance only if every other guard and obligation is also `True`. |

Only `Truth::True` satisfies a guard. With the diff, green verification, review, and provenance
records submitted, the same command reaches the terminal state:

```shell-session
$ "$AEP" evaluate --root . \
    --task examples/development-passkeys/task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/01-red-test.yaml \
    --evidence examples/development-passkeys/evidence/02-implementation.yaml \
    --evidence examples/development-passkeys/evidence/03-verification.yaml \
    --evidence examples/development-passkeys/evidence/04-review.yaml \
    --evidence examples/development-passkeys/evidence/05-provenance.yaml \
    --advance
state       complete (Complete)
...
Task complete in `complete`:
  ✓ (tests.unit.failed == 0 and static_analysis.errors == 0 and evidence.missing == 0)
```

**Responsible boundary:** the `Truth` domain type preserves all three outcomes; the engine evaluates
predicates and advances only on `True`. The workflow document defines the legal transitions rather
than leaving the driver to invent them.

## 5. Bind approval to exactly what was reviewed

The artifact manifest declares `design:passkeys-auth` at version 7. The review evidence says
`reviewed_version: "3"`. `development.standard` does not require a fresh design review, so the
ordinary example can finish; switch the same task to `development.critical` and freshness becomes
an obligation:

```shell-session
$ critical_dir="$(mktemp -d)"
$ sed 's/development.standard/development.critical/' \
    examples/development-passkeys/task.yaml > "$critical_dir/task.yaml"
$ "$AEP" evaluate --root . \
    --task "$critical_dir/task.yaml" \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/04-review.yaml
...
  ✗ review of a design is approved (by a person)
      the approved review of design:passkeys-auth was given against a different version
```

The subject ID identifies the design; `reviewed_version` identifies the version Ada saw. Because
3 does not equal 7, the fresh-review requirement remains unsatisfied until version 7 is reviewed.

**Responsible boundary:** revision-bound artifact and `ReviewResult` types in `aep-domain`; the
engine's requirement matcher. The external review provider must preserve the reviewed revision when
it creates the record.

## 6. Refuse illegal and partial state changes

Workflow movement and entity mutation use related but separate boundaries:

- The engine returns `Blocked` when no transition guard is `True`; it does not move and then report
  that the destination was invalid.
- Every persisted entity mutation crosses `CommandService::execute` in a `CommandEnvelope`. The
  envelope carries an idempotency key and an expected revision, so a retry is recognizable and a
  stale write is a conflict rather than an implicit merge.
- A conforming backend/provider evaluates a command against candidate state and publishes that state
  atomically.

The memory-backend regression test exercises the failure that the unguided path could not prevent:
accept an ADR while asking it to supersede an entity that does not exist. This excerpt is
**illustrative, not standalone**; its complete compiled form is
`crates/plan/aep-backend-memory/tests/failure_atomicity.rs`.

```rust
let error = block_on(backend.execute(envelope(
    "accept-adr",
    "key-accept-adr",
    2_000,
    Command::AcceptAdr(AcceptAdr {
        adr: adr.clone(),
        supersedes: Some(missing),
    }),
)))
.expect_err("the superseded ADR does not exist");

assert_eq!(error.code(), "not_found");
assert_eq!(after.entity, before.entity);
assert_eq!(after.relations, before.relations);
assert_eq!(after.history, before.history);
assert_eq!(after.events, before.events);
```

The runnable test checks more than this shortened excerpt: the entity set and revision are also
unchanged, and exactly one `CommandRejected` audit record is appended. The refusal is observable;
the rejected candidate state is not.

**Responsible boundary:** command vocabulary in `aep-domain`; command contract in `aep-contract`;
atomic implementation in each backend/provider. A custom provider must meet that contract. AEP
cannot roll back an external side effect performed outside it.

## 7. Keep decisions deterministic and effects at named edges

For the same validated documents, evidence sequence, and supplied time, the engine returns the same
ordered decision. Tests inject a fixed clock explicitly; this is an excerpt from the compiled
end-to-end test, not a standalone program:

```rust
fn engine() -> Engine<FixedClock> {
    Engine::with_clock(registry(), FixedClock::new(1_700_000_000_000))
}
```

Filesystem loading, clocks, commands, networks, stores, models, and people remain named inputs.
The reference driver consumes a version-pinned step map such as
`drivers/development/default.yaml`:

```yaml
format: aep.driver-steps/1
id: development/default
workflow: adp/default/2

states:
  establish_verifiers:
    steps:
      - kind: command
        run: [cargo, test, --workspace]
        evidence:
          kind: test_result
          suite: unit
          verifier: test-runner
```

The driver executes declared `command`, `llm`, and `operator` steps and submits observations. It
does **not** evaluate transition or completion gates; it asks the engine. A model, plugin directory,
credentials, budget, and harness are supplied at that execution boundary. AEP does not discover a
marketplace, fetch a plugin, or infer ambient credentials.

**Responsible boundary:** deterministic decisions in `aep-engine`; I/O projection in `aep-project`;
step-map types in `aep-driver-spec`; effects and evidence capture in the reference driver or another
external harness.

## 8. Join separate systems through a strict report

ESS and agent plugins are not hidden AEP subsystems. ESS may use documentation as its executable
specification, run conformance independently, and publish a standalone report. The optional
`aep-ess-evidence` adapter converts that report into an AEP `ess_conformance` record. It rejects
unknown fields, inconsistent counts, and a passing verdict with failed scenarios; the evidence must
also carry the digest of the specification revision it exercised.

```yaml
- kind: ess_conformance
  specification: billing/v3
  spec_digest: 13577b3ce695932e980d418d5863bcde07f4c362516d53147870d31eaf2ed861
  implementation: billing-reference 0.39.2
  status: passed
  scenarios_total: 29
  scenarios_failed: 0
  observed_at: 1699920000000
  producer:
    producer: verifier
    verifier: conformance-runner
```

This record is from the separately tested `examples/billing-conformance/` fixture. If passkey
documentation becomes the ESS for `AUTH-142`, the boundary stays the same: ESS judges the system
against those docs; the adapter translates the report; AEP checks whether the revision-bound
evidence satisfies the task. Repository co-location grants no authority.

**Responsible boundary:** ESS owns specification execution and its standalone report;
`aep-ess-evidence` owns strict translation; the engine owns only the resulting obligation. The
plugin or harness boundary owns plugin selection, installation, credentials, and actual execution.

## 9. Treat public bytes as part of the behavior

Meaning can drift even when source code still compiles. AEP therefore derives published JSON
Schemas from the raw Rust input types, checks those schemas against every shipped protocol,
principle, workflow, profile, lifecycle, step map, and the passkey task and manifest, and tests
accepted aliases explicitly. The canonical `aep` binary and the `protocol` compatibility alias are
also tested for identical exit status, stdout, and stderr on successful and refused commands.

That does not freeze every format forever. It makes a schema, alias, digest rule, command name, or
rendered compatibility surface an explicit migration instead of an accidental reinterpretation.

**Responsible boundary:** raw wire types and schemas in `aep-schema`; semantic validation in
`aep-domain`; byte and command-compatibility tests at the CLI boundary.

## What AEP guarantees—and what it does not

For a caller that stays inside the declared boundaries, AEP provides:

- validated documents before an execution exists;
- deterministic policy, predicate, requirement, and transition decisions for the same inputs and
  supplied clock;
- explicit attribution for capability decisions, approvals, evidence, commands, and refusals;
- revision-aware matching and optimistic-concurrency checks;
- failure atomicity from conforming command-service providers; and
- generated and tested public document contracts.

AEP does **not** prove that a producer identity is cryptographically genuine, run a verifier merely
because a record names one, confine a tool that bypasses the harness, make an arbitrary remote side
effect atomic, discover plugins or credentials, or prove more about software than the submitted
evidence says. Those are producer, harness, provider, credential, and external conformance-system
responsibilities. The point of the protocol is that each responsibility has a visible boundary and
a refusal when its required input is absent.

## Ten rules, at a glance

Use this list as the concise reference after the walkthrough:

1. **Evidence, not assertion.** Completion reads typed observations with provenance, as in step 3,
   not “tests pass” prose.
2. **Unknown is not false.** Step 4 preserves unobserved, contradicted, and satisfied facts so each
   produces the right next action.
3. **Deny by default.** Step 2 grants only declared capabilities, and deny outranks every later
   grant.
4. **Approval binds to a revision.** Step 5 refuses a version-3 review for a version-7 design.
5. **Deterministic core, named edges.** Step 7 supplies time and effectful dependencies rather than
   reading ambient state inside the engine.
6. **Parse, then validate.** Step 1 separates accepted syntax from values that satisfy domain and
   cross-document invariants.
7. **Refusals are results.** Steps 2 and 6 explain the rule or invalid target, preserve state, and
   retain an audit record.
8. **The reference driver proves a boundary.** Step 7 shows a caller that obtains evidence and asks
   the engine to decide; it is not a second policy implementation.
9. **Separate domains meet through reports.** Step 8 joins ESS and plugins through explicit,
   revision-aware inputs rather than repository proximity.
10. **Public bytes are a contract.** Step 9 tests schemas, aliases, exit codes, and output bytes so
    consumers do not inherit a silent semantic change.

For the broader execution model, continue with [AEP: governing engineering work](./aep.md). For the
evidence vocabulary and trust model, read [Evidence and completion](./evidence.md); for API-level
harness integration, read [Integrate an agent harness](../guides/integrate-a-harness.md).

---

**Executable sources.** `examples/development-passkeys/`;
`crates/edge/aep-cli/tests/cli.rs`; `crates/govern/aep-engine/tests/end_to_end.rs`;
`crates/plan/aep-backend-memory/tests/failure_atomicity.rs`; `crates/edge/aep-schema/tests/published.rs`;
`crates/edge/aep-cli/tests/command_equivalence.rs`; `crates/observe/aep-ess-evidence/src/lib.rs`;
`drivers/development/default.yaml`.
