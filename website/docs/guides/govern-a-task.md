---
title: Govern a task
sidebar_position: 7
description: Put a task of your own under a profile — the task file, the artifact manifest, choosing a profile, and keeping the documents honest in CI.
---

# Govern a task

This guide takes a team that already has rules — in a wiki, a `CONTRIBUTING.md`, or one senior
engineer's head — and puts a real task under the protocol. The commands run from a checkout of the
AEP repository, where the worked example lives, with `aep` installed as in the
[quickstart](../getting-started.md):

```bash
git clone https://github.com/beyond10x/aep && cd aep
```

## What you write

Three things, and only the first two are files of yours:

| You provide | What it is | Smallest useful form |
|---|---|---|
| `task.yaml` | what is being done, and which profile governs it | 5 lines |
| `artifacts.yaml` | where the specs, designs, ADRs and reviews live — references, not copies | 1 entry |
| a profile name | which bundle of rules applies | `development.fast` / `.standard` / `.critical` / `.driven`, or your own |

Everything else — workflow, principles in force, capabilities, completion condition — is derived
from the profile. Restating any of it in the task is how it drifts.

A task that resolves:

```yaml
id: BILL-88
kind: feature
objective: move-invoices-to-partitioned-table
protocol: adp/1
profile: development.standard
derived_from:
  - story:BILL-87
manifest: artifacts.yaml
constraints:
  facts:
    # Declared here because nothing can observe it. A principle reads it to
    # decide whether it applies.
    change.database_schema: true
    change.public_contract: false
```

And the manifest it points at:

```yaml
version: aep.artifacts/1

artifacts:
  - id: story:BILL-87
    kind: story
    status: active
    location:
      provider: linear
      reference: BILL-87

  - id: spec:invoice-partitioning
    kind: specification
    status: approved
    location:
      path: docs/specs/invoice-partitioning.md
    relations:
      - specifies: story:BILL-87

  - id: plan:invoice-partitioning
    kind: migration-plan
    status: approved
    location:
      path: docs/migrations/0007-partition-invoices.md
    relations:
      - derived_from: spec:invoice-partitioning
```

`manifest:` is resolved relative to the task file, so the two sit beside each other. The manifest
holds no copies: the design stays in `docs/`, the PRD stays in the planning tool, and neither moves
for the protocol to reason about them.

## Where the documents live

The loader walks six directories under one root, recursively, skipping anything dot-prefixed
(`crates/edge/aep-project/src/load.rs`). Vendor the upstream documents (submodule, subtree or copy —
the loader cares about content, not provenance) and put yours beside them:

```text
your-repo/
  protocols/                        vendored, unchanged — the declared vocabulary
  workflows/                        vendored, unchanged — unless you write your own state machine
  principles/
    upstream/                       vendored
    migration-has-a-way-back.yaml   yours
  profiles/
    upstream/                       vendored
    acme-service.yaml               yours — usually `extends:` an upstream profile
  artifacts/lifecycles/             vendored
  drivers/                          step maps — what a harness *does* in each state
```

Documents are indexed by the `id` declared inside each file, never by path, so the subdirectory
layout is yours to choose. `drivers/` is the newest of the six and the only one a harness needs: a
step map names the program, model or person that runs in each workflow state, and it is what
[the reference driver](#letting-the-driver-walk-it) walks. A tree with no step map still validates —
missing directories are not an error.

Loading this repository's own tree:

```shell-session
$ aep govern validate --root .
60 file(s): 5 protocol(s), 24 principle(s), 6 workflow(s), 10 profile(s), 13 lifecycle(s), 2 step map(s)
valid
```

## Naming the tree once, in a project file

Passing `--root`, `--task` and `--artifacts` on every invocation is fine for one command and tedious
for a session. `.engineering/project.yaml` names them once, and every verb that takes those three
flags discovers them from the working directory upwards:

```yaml
version: aep.project/5
protocol: adp/1
profile: development.standard
protocols: ..          # where the document tree is, relative to `.engineering/`
planning_scope: aep
store:
  git: {}
task: task.yaml        # the default; relative to `.engineering/`
```

With `.engineering/project.yaml` and `.engineering/task.yaml` in place, the flags become optional:

```shell-session
$ aep govern resolve
inputs      project …/aep
task        W4-1 (feature)
objective   agent-eval-cases
protocol    adp/1
profile     development.driven
workflow    adp/default (initial: receive)
principles  spec-driven, test-driven, static-analysis, least-privilege, provenance-tracking, contract-testing, property-based-testing, approval-gates, reversible-changes
obligations 11
```

The first line says which it used, and carries the project's absolute path — abbreviated here.
`inputs project` means it discovered one; `inputs . and <task>` means you passed the paths
yourself.

Every field of the file is in [`project.yaml`](../reference/project-file.md).

## Seeing the workflow the profile puts you on

The profile picks a workflow, and the workflow is where the guards live. `aep govern workflow render`
draws it without running anything:

```shell-session
$ aep govern workflow render --id adp/default --format tui
Standard development workflow
adp/default/2 · 10 states · 13 transitions

  · receive              Receive                  intake
  │
  · specify              Specify                  specification
  ├─▷ declined  specification.declines_the_request
  │  artifact.specification.exists
  · decompose            Decompose                decomposition, planning
  │
  · establish_verifiers  Establish verifiers      verification-setup
  │  test.exists
  · implement            Implement                implementation
  │  diff.exists
  · verify               Verify                   verification
  │  (tests.unit.failed == 0 and tests.contract.failed == 0 and static_analysis.errors == 0)
  ╰─◁ implement  (tests.unit.failed > 0 or tests.contract.failed > 0 or static_analysis.errors > 0)
  · adversarial_verify   Adversarial verify       adversarial-verification
  ╰─◁ implement  (tests.unit.failed > 0 or tests.contract.failed > 0 or static_analysis.errors > 0)
  │  evidence.missing == 0
  · review               Review                   review
  ├─▷ complete  review.approved
  ╰─◁ implement  review.changes_requested
  │  review.rejected
  · declined             Declined                 completion · terminal
  · complete             Complete                 completion · terminal
```

(Colour stripped for this page.) The guard sits on the arrow, which is where it acts: `verify` goes
back to `implement` when a test fails, and forward only when none does. `--format` also takes `svg`, `mermaid`,
`html` and `png`; `--run <RUN>` draws a driver run over the same picture.

## Choosing a profile

Measured on the repository's worked example — same task, same artifacts, only the profile changed:

| | `development.fast` | `development.standard` | `development.critical` | `development.driven` |
|---|---:|---:|---:|---:|
| principles in force | 5 | 9 | 15 | 9 |
| obligations | 6 | 11 | 18 | 11 |
| completion checks | 14 | 24 | 45 | 24 |
| distinct evidence kinds owed | 5 | 7 | 7 | 7 |

Reproduce any column: `aep govern resolve` prints the first two rows directly, and
`aep govern evaluate --format json` carries the third and fourth — `completion` is the array, and the
entries whose `flavour` is `evidence` are the kinds.

The evidence *kinds* barely grow between standard and critical — what grows is the number of runs
and who has to sign.

| Profile | Choose it when | What it costs |
|---|---|---|
| `development.fast` | blast radius contained, contract surface private: internal tooling, scripts, a spike | a spec, a failing test first, static analysis, provenance. It cannot request a review or an approval — a human is never in the loop |
| `development.standard` | anything with an external consumer, persisted data, or a customer-visible path — the default | adds contract tests and a property suite, and the ability (with it, the obligation) to ask a human |
| `development.critical` | a silent defect is worse than a late delivery: auth, money, migrations, crypto | adds a mutation run, a differential run against the implementation being replaced, an invariant check, design-by-contract, adversarial verification, specification conformance, an approved design, and a **fresh human review of that design** |
| `development.driven` | a model, not a person, is doing the typing | `development.standard` plus `command.execute`, and nothing else. It exists because the planning store has no tool surface other than the `aep` CLI, so a driven step under `.standard` cannot create an artifact at all — the run does not fail, it never moves |

`development.driven`'s grant is the one place in this directory where a profile widens what an agent
may reach, and the profile's own header says so rather than leaving it to be discovered. The
narrowing is a hook, not a capability:
the driver's own per-call policy (`decide_tool` in `crates/edge/aep-cli/src/drive.rs`,
answering the metaharness seam) denies any `Bash` call that is not one simple
invocation of `aep plan artifact …` or `aep observe trace …`. That constraint is pattern-based and
best-effort — granting `command.execute` grants a superset of the shell's reach, and a hook narrows
it rather than making it a function of the capability.

The critical row's freshness requirement is the one that bites. The worked example's manifest holds
the design at version 7 and the review approved version 3, and the evaluation says so in its own
words:

```shell-session
$ aep govern evaluate --root . --task task-critical.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/04-review.yaml
…
  ✓ artifact design (approved) which designs a specification      [completion]
  ✗ review of a design is approved (by a person)                  [completion]
      the approved review of design:passkeys-auth was given against a different version
…
```

(`task-critical.yaml` is the shipped `examples/development-passkeys/task.yaml` with
`profile: development.critical`.)

## Walking a task on its evidence

The worked example ships five evidence files. Submitting them in order and asking the engine to
advance shows the whole lifecycle. One file carries the task to `implement` and stops it there:

```shell-session
$ aep govern evaluate --task examples/development-passkeys/task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/01-red-test.yaml \
    --advance
inputs      . and examples/development-passkeys/task.yaml
state       implement (Implement)
transitions
  implement -> verify [blocked]
      guard: diff.exists
Task incomplete in `implement`:
  ✗ (tests.unit.failed == 0 and static_analysis.errors == 0 and evidence.missing == 0)  [completion]
      tests.unit.failed = 1; unobserved: static_analysis.errors; evidence.missing = 7
  ? (specification.satisfied and contracts.failed == 0)           [completion]
      unobserved: specification.satisfied; unobserved: contracts.failed
  ? specification.satisfied                                       [principle spec-driven]
      unobserved: specification.satisfied
  ✗ tests.unit.failed == 0                                        [principle test-driven]
      tests.unit.failed = 1
  …
  ✓ evidence test_result from test-runner (independent)           [principle test-driven]
  ? evidence diff                                                 [principle test-driven]
      0 of 1 required record(s) submitted
  ✓ test-runner must run                                          [principle test-driven]
  …
```

(Twenty-four requirement lines; seven shown, the `…` marking what was cut.) Each line names what is owed, which document asked for
it, and which of the three truth values it holds: `✓` observed and true, `✗` observed and false,
`?` nobody observed it. The blocked transition names its guard. Submit all five files on one command
line and the same invocation reaches `complete`:

```shell-session
$ aep govern evaluate --task examples/development-passkeys/task.yaml \
    --artifacts examples/development-passkeys/artifacts.yaml \
    --evidence examples/development-passkeys/evidence/01-red-test.yaml \
    --evidence examples/development-passkeys/evidence/02-implementation.yaml \
    --evidence examples/development-passkeys/evidence/03-verification.yaml \
    --evidence examples/development-passkeys/evidence/04-review.yaml \
    --evidence examples/development-passkeys/evidence/05-provenance.yaml \
    --advance
inputs      . and examples/development-passkeys/task.yaml
state       complete (Complete)
transitions
  (none: this state is terminal)
Task complete in `complete`:
  ✓ (tests.unit.failed == 0 and static_analysis.errors == 0 and evidence.missing == 0)  [completion]
  …
```

Every record in those files carries an `observed_at`. It has no default, and the omission is
deliberate: a caller who has to write down when they looked is a caller who cannot accidentally
claim they looked just now.

## Letting the driver walk it

Everything above is a person typing `evaluate`. `aep drive` is the loop that does it: the reference
driver. It makes the engine's calls in order, executes the steps a step map declares for each state,
and records what it did under `.engineering/runs/<run>/`. It evaluates no gate itself. A driver that
could evaluate a gate would be a second implementation of the protocol, with none of the conformance
suites behind it.

A step is one of three kinds: `command` (a program), `llm` (a model session) or `operator` (a
person). `aep drive` runs command and operator steps. A map with an `llm` step is refused before a
run is allocated:

```shell-session
$ aep drive run --map development/default
error: this map contains model-backed steps; use `metaharness aep drive run` or `metaharness aep drive resume`; AEP allocated no run and launched nothing
```

Both maps that ship, `development/default` and `development/checks`, contain model steps, so they
run under `metaharness aep drive`. That host uses the same governor and writes the same run records,
and `aep drive status` reads them:

```bash
aep drive status              # the last run, and who holds the lock
aep drive resume AUTH-142/3   # continue a stopped command/operator run
```

A map of your own that only has `command` and `operator` steps runs under `aep drive run` directly.
`--pause-on-approval` stops at the first thing a person owes and exits `0`. The resume then walks
on from the step after it.

[Integrate an agent harness](./integrate-a-harness.md) covers what the driver enforces, what it
delegates to hooks, and what to do instead if you are writing your own.

## Keeping the documents honest in CI

`validate` reads the tree on its own; `resolve` also needs a task, because some checks only mean
anything once a profile and a workflow are paired:

| Caught by | Refusal | Otherwise |
|---|---|---|
| `validate` | `unobservable_fact` — a predicate reading a fact the protocol does not declare | the rule can never be satisfied; a task hangs |
| `validate` | `unreachable_state` | the state is decoration |
| `validate` | `dead_end_state` — a non-terminal state with no way out | execution wedges there |
| `validate` | `incomplete_rollback_policy` | "rolled back" describes a wish |
| `resolve` | `unknown_phase` — an obligation timed against a phase no state declares | the rule is not strict, it is absent |
| `resolve` | `capability_conflict` — a task needing a capability the policy denies | the agent finds out mid-task |

Both exit 1, and both accumulate all problems in one run. Add them to CI as their own step, and keep
at least one representative task per profile you ship — `validate` alone cannot tell you that a
principle is timed against a phase your workflow does not have:

```yaml
- name: Documents
  run: |
    aep govern validate --root .
    aep govern resolve --root . --task examples/development-passkeys/task.yaml
```

If your repository keeps a planning store as well, add `aep plan artifact validate` to the same
step. See [Validate the plan in CI](./validate-in-ci.md).

## Next

* [Write a principle](./write-a-principle.md) — encode a rule of your own and put it in force.
* [Integrate an agent harness](./integrate-a-harness.md) — make these answers govern a real agent.
