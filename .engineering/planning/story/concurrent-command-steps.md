---
format: aep.planning-md/3
id: story:concurrent-command-steps
kind: story
status: implemented
title: Independent command steps run at once, where a state opts in
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:llm-step-declares-judgement
- depends_on: story:command-step-expects-failure
- serves: vision:O2
scope:
- confidence: cited
  path: crates/drive/aep-driver-spec/src/cursor.rs
- confidence: cited
  path: crates/drive/aep-driver-spec/src/map.rs
- confidence: cited
  path: crates/drive/aep-driver/src/executor.rs
- confidence: cited
  path: crates/drive/aep-driver/src/route.rs
- confidence: cited
  path: crates/drive/aep-driver/src/run.rs
- confidence: cited
  path: crates/drive/aep-driver/tests/concurrent.rs
- confidence: cited
  path: crates/edge/aep-cli/src/drive.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/drive_cli.rs
- confidence: cited
  path: ess/domains/drive.yaml
- confidence: cited
  path: website/docs/reference/documents.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T06:23:57Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":11}}}
- {from: "proposed", to: "active", at: "2026-10-15T06:23:57Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":11}}}
- {from: "active", to: "implemented", at: "2026-10-16T01:52:57Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":12}}}
---
# Story: Independent command steps run at once, where a state opts in

## Outcome

A state in a step map may declare `concurrent: true`. A state without it runs its steps one at a
time, as today. In a concurrent state:

- **Batch.** Each maximal run of consecutive `command` steps is a batch, and a batch is one cursor
  unit. `next_step` (route.rs) returns the batch, from `DriverCursor::step` to the batch's last
  index. The cursor keeps its single step pointer at the batch's first index, and the pointer moves
  past the batch only when every step in it has ended. The batch's results are then submitted in
  map order.
- **Attempts.** Starting a batch charges one attempt to each of its steps (`record_attempt` per
  index, so each step's log is `<state>-<index>-<attempt>.log`). `DriverCursor::in_flight` holds one
  `InFlightAttempt` for the whole batch: its first index, and the attempt number charged to that
  step.
- **Interruption.** An interruption before the batch ends submits nothing. `aep drive resume
  --retry-in-flight <id>` reruns every step of the batch with the attempt numbers already charged,
  as a serial retry keeps its number today (`Session::begin_attempt`). No counter advances.
- **Barrier.** An `llm` or `operator` step runs after every earlier command step of the state has
  ended. The command steps after it form the next batch.
- **Shared dependency.** Steps that name the same `depends_on` run one after another, in map order,
  inside their batch. The circuit breaker then reads the failure count a serial run would give it.
- **Retries and budget.** A step's `retries` are spent inside its batch, and each retry advances
  that step's attempt number. A failed step does not cancel its siblings. If a step spends its
  retries with no verdict, the batch still submits the verdicts its other steps produced, in map
  order. The run then stops `budget-exhausted`, as a serial run stops at that step
  (`Session::step`).

This serves the bug-fix loop's diagnose state, where collecting state (logs, traces, store reads)
is a set of commands that can run at once (the epic's table, "bug fix | diagnose").

## Context

- A command step's `depends_on` names an outside dependency that a circuit breaker keys on. It is
  not a step reference (`RawCommandStep::depends_on`, `CommandStep::circuit_breaker`; the breaker
  read in `Session::step`, run.rs).
- A step has no id: it is addressed by its index in `StepMap::steps_for(state)`. Step ordering
  inside a batch (an `id` and `after`) is not asked for by the epic and is not added.
- `next_step` and `steps_remaining` (route.rs) read `cursor.step` as one index, and `Session::step`
  (run.rs) runs one index per loop iteration.
- Concurrency stays in the `edge` host. `CommandStepExecutor` gains a batch method whose default
  runs the batch serially; the CLI host (`CliExecutors`, drive.rs) runs it on scoped threads. The
  deterministic core stays sequential over collected results, so `tests/determinism.rs` in
  `aep-driver` keeps its banned-token scan unchanged.
- Each step's evidence is minted with a wall-clock `observed_at` (`observed_now`, drive.rs) and
  carries an evidence id. Byte comparison across runs therefore excludes both.
- Two command steps of one concurrent state that write the same `output:` would race for one file
  in the run directory. Serially the later one wins, deterministically; concurrently it does not,
  so it is refused at load.

## Acceptance

Specified first. `ess/domains/drive.yaml` declares, before any code changes:

- `aep.drive.Step` gains `concurrent` (Boolean, its state's opt-in), set by `aep.drive.DeclareStep`.
- A new command, `aep.drive.DeclareConcurrentOutput` (wire `driver-steps-load-concurrent-output`).
  It is shaped like `aep.drive.DeclareOutput` and takes a command step's `path`, its `output`, and
  `shared_with: Optional<aep.drive.StepPath>`, the earlier step of the same concurrent state that
  names the same output. Outcomes:
  - `concurrent-output-shared` when `defined(shared_with)`, error `aep.drive.ConcurrentOutputShared`;
  - `kept` otherwise.
- An ESS-LIMIT note. ESS 0.60.0 has no predicate comparing a value across two instances, so the
  pairwise comparison that fills `shared_with` is the loader's (`StepMap::try_from`). It is pinned by
  the map.rs test for the shared-output refusal below.

`cargo xtask ess --check` validates under ESS 0.60.0, and `generated/ess/` matches a fresh
projection. `ess verify conform synthesize` yields scenarios for `concurrent-output-shared` and
`kept`, and `xtask/tests/ess_gate.rs` (`SYNTHESIZED`) records the new counts with the reason.

Then, through `aep drive run` and `aep drive resume` over command-only fixture maps (an `llm`
fixture through a stub host where noted):

- **Concurrency.** A `concurrent: true` state has three command steps that sleep 3, 1 and 2
  seconds, in that map order. Each has `test_result` evidence with suites `s3`, `s1` and `s2`. The
  run exits 0 within 4 seconds of wall time, and `snapshot.json` lists the three suites in map
  order (`s3`, `s1`, `s2`).
- **Determinism.** Over ten runs of that fixture, `snapshot.json` is byte-identical once every
  `observed_at` and evidence id is removed.
- **Interruption.** Step 0 sleeps 1 second. Step 1 runs `sh -c 'test -f killed || { touch killed;
  kill -9 $PPID; }'`, killing the driver mid-batch.
  - After the kill, `snapshot.json` holds no evidence from the batch, and `cursor.json`'s
    `in_flight` names step 0.
  - `aep drive resume --retry-in-flight <id>` exits 0. The run directory then holds
    `<state>-0-1.log` and `<state>-1-1.log` and no `-2.log`. Both steps' evidence is submitted once,
    in map order.
- **Retries in a batch.** Step 0 has `retries: 1` and runs `sh -c 'test -f once || { touch once;
  kill -9 $$; }'`; step 1 sleeps 1 second. The run directory holds `<state>-0-1.log`,
  `<state>-0-2.log` and `<state>-1-1.log`. Step 0's passing evidence precedes step 1's in
  `snapshot.json`.
- **Budget spent in a batch.** Step 0 has `retries: 0` and runs `sh -c 'kill -9 $$'`; step 1 exits
  0. `snapshot.json` holds step 1's evidence, and `cursor.json` holds `"status":
  "budget-exhausted"`. The call exits 1.
- **Shared dependency.** Steps 0 and 1 name `depends_on: store` with `circuit_breaker: 1` and
  `retries: 1`, and each runs `sh -c 'kill -9 $$'`. Step 2 names no dependency and sleeps 1 second.
  - The run directory holds exactly one log for step 0, `<state>-0-1.log`, and none for step 1.
  - The report notes that step 1 "was not attempted: `store` has failed 1 time(s)".
  - Step 2's evidence is submitted. This is the result a serial run gives.
- **Operator barrier.** A concurrent state holds command step A, an `operator` step, then command
  step B. `aep drive run --pause-on-approval` stops with `cursor.json` `"status":
  "awaiting-operator"`. A's `<state>-0-1.log` exists, and no `<state>-2-1.log` does.
- **llm barrier.** Command steps A and B, then an `llm` step, then command step C, driven through a
  stub host. The stub `llm` executor observes that A's and B's `output:` files exist when it is
  launched, and that C's does not.
- **Shared output refused.** A concurrent state where two command steps write `output: a.txt` is
  refused at load with code `unsupported_construct`. The message names both step paths. The same two
  steps in a state without `concurrent` load.
- **No cancellation.** In a batch where the first step exits 1 and the second exits 0, both are
  submitted, failing then passing, in map order.
- **Unchanged maps.** `drivers/development/default.yaml` and `checks.yaml` declare no
  `concurrent`, and their existing driver tests pass unchanged.

## Scope

Specified first:

- `ess/domains/drive.yaml` (`aep.drive.Step.concurrent`, `aep.drive.DeclareStep`,
  `aep.drive.DeclareConcurrentOutput`, `aep.drive.ConcurrentOutputShared`)
- `generated/ess/` (rewritten by `cargo xtask ess`)
- `xtask/tests/ess_gate.rs` (`SYNTHESIZED`)

Then, cited:

- `crates/drive/aep-driver-spec/src/map.rs` (`RawStateSteps`, `StateSteps`: `concurrent`; the
  shared-output refusal in `StepMap::try_from`; a batch accessor beside `StepMap::steps_for`)
- `crates/drive/aep-driver-spec/src/cursor.rs` (the doc of `DriverCursor::step`, `in_flight` and
  `InFlightAttempt` for a batch; no new field)
- `crates/drive/aep-driver/src/route.rs` (`NextStep`, `next_step` returning the batch,
  `steps_remaining`)
- `crates/drive/aep-driver/src/executor.rs` (the batch method on `CommandStepExecutor`)
- `crates/drive/aep-driver/src/run.rs` (`Session::step`, `Session::begin_attempt`)
- `crates/edge/aep-cli/src/drive.rs` (`CliExecutors`, the command host)
- `schemas/generated/driver-steps.schema.json` (rewritten by `cargo xtask schema`)
- `website/docs/reference/documents.md` (the step map's `concurrent`)
- `CHANGELOG.md` (Unreleased)
- tests: `crates/drive/aep-driver/tests/concurrent.rs` (new), `crates/edge/aep-cli/tests/drive_cli.rs`

## Depends on

- `story:llm-step-declares-judgement`
- `story:command-step-expects-failure`

Both edges are for the shared files `map.rs`, `drive.rs`, `ess/domains/drive.yaml` and the
driver-steps schema. No ESS release newer than 0.60.0 is needed.
