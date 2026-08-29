# engineering-protocols

> A strongly typed, portable and machine-executable specification for how autonomous engineering
> work is performed and proven correct.

Coding and operations agents are usually governed by prose:

> *Follow TDD, don't break existing APIs, verify your work, and ask before deploying.*

That reads well and enforces nothing. It leaves every operative question open: what counts as
following TDD, what evidence proves a test failed *before* the implementation existed, which
operations need approval, what "verify your work" means, when the task is actually finished, and what
happens when verification fails.

`engineering-protocols` moves those rules out of prompts and into typed, executable protocol
definitions. The model still reasons. The protocol decides what the resulting facts permit. The
agent may be probabilistic; the protocol semantics are not.

## Two halves, one seam

| | Governs | Answers |
|---|---|---|
| **AEP** — Agentic Engineering Protocol | how engineering work is performed | *Was this built properly?* |
| **ESS** — Executable System Specification | what software must exist | *Is this the thing we meant to build?* |

They are not layers: AEP does not know what an invoice is, and ESS does not know what a code review
is. They meet at exactly one point — evidence. ESS defines the target, work proceeds under AEP, ESS
conformance checks the result, and that verdict is a fact AEP's completion predicate reads. The loop
closes because the specification that *generated* the contracts is the one that *tests* the
implementation: an agent cannot pass by weakening a test it did not write, and cannot declare itself
done, because completion is a predicate over facts it does not control.
[`docs/VISION.md`](docs/VISION.md) is the full argument.

## What that looks like

```console
$ protocol explain --task examples/development-passkeys/task.yaml --action production.write
production.write denied
  operation: change production state
  reason:    principle approval-gates rule production-write-requires-approval
  missing:   approval for capability production.write
  state:     receive
```

The refusal has an address: it names a principle someone can go and read, and says what would unlock
the operation. Nobody wrote that denial into the task — the task names a profile and an objective,
and nine principles, a workflow and twelve capability decisions are derived from the document tree.

* **Red-before-green is a fact, not an instruction:**
  `evidence.first_seq.test_result < evidence.first_seq.diff`.
* **An agent cannot verify itself.** An evidence requirement marked `independent: true` is never
  satisfied by the agent's own report of a green suite.
* **An approval names the revision it approved.** Approving design version 3 stops satisfying the
  requirement at version 7 — otherwise a reviewer's name ends up attached to a decision they never saw.
* **Unknown is not false.** `✗` is a fact that is wrong — fix the code; `?` is a fact nobody
  observed — go run the tests. Only `true` permits a transition.
* **A fact knows when somebody looked, and stops counting when nobody has.** Every evidence record
  states `observed_at`; a requirement may declare a horizon, and past it the observation reads `?`
  again — never `✗`, because a green suite from three weeks ago is not a wrong answer, it is an old
  one. Nothing extends a horizon: the only refresh is to observe again and write a new date.
* **Capabilities default to deny**, and `deny` cannot be granted back by a later document.
* **Nothing is deleted.** Archive and supersede are the vocabulary; every mutation crosses one
  boundary carrying actor, executor, correlation, causation and an idempotency key.
* **`protocol drive` runs the workflow instead of suggesting it.** The tools an agent holds in a
  state are the ones the protocol grants there, a refused action is refused by a program rather than
  by a paragraph, and every transition is the engine's. A step that calls a model has no field to put
  evidence in — the type, not a rule.
* **`protocol workflow render` draws the run.** A workflow, and where a run has been, what it
  produced and why it stopped, as SVG, HTML, PNG or a live terminal frame. The reasons are the
  engine's own sentences, verbatim.
* **`protocol workflow flow` projects the workflow into the document the b10x loop walks natively**,
  with `--map` carrying each state's step into its node. An honest projection and not an
  equivalence: retreats become repeating groups, terminal states are dropped, and no guard travels,
  so the governor stays a program the loop asks at every section boundary — and every state is a
  section, so that boundary is every state's. `protocol workflow instruct` writes the same
  workflow out in words.
* **`protocol trace check` reads the run back.** An agent's transcript is judged against a typed
  specification — fifty-one expectation kinds — so *the agent followed the rules* is a verdict a program
  reached from the record, not a claim the agent made about itself. Its exit codes distinguish
  contradicted from nobody-found-out.

## Is this for you

| Read | If |
|---|---|
| [`docs/guide/adopting.md`](docs/guide/adopting.md) | you have engineering rules you want enforced, and a repository to put them in |
| [`docs/guide/harness.md`](docs/guide/harness.md) | you are building an agent harness and want the protocol to decide what it may do |
| [`docs/guide/backend.md`](docs/guide/backend.md) | you are storing designs, reviews and approvals, and want them to survive an audit |
| [`docs/guide/specification.md`](docs/guide/specification.md) | you want a system's contracts, tests and documentation derived from one document instead of maintained beside it |

It is not a tool for making an agent ship features faster — and deliberately not an LLM orchestration
framework, a CI system or a deployment platform: nothing here calls a cloud API or holds a
credential. External systems do the work; this project decides what the results permit.

## Where it sits

Nothing here spawns a harness, holds a credential or reaches a cluster. Those jobs belong to the
repositories beside it.

| repo | relationship |
|---|---|
| [metaharness](https://github.com/beyond10x/metaharness) | drives a vendor harness and decides each tool call at a seam. The driver's enforcement policy answers that seam per call; the evals that judge this repository's own runs live there, under `evals/engineering-protocols/` |
| [entity-runtime](https://github.com/beyond10x/entity-runtime) | the only repository here takes a **dependency** on, and the arrow points one way only. Its `entity-core` is an IO-free kernel — no clock, no filesystem, no network — that takes an entity type as data and answers whether a status move is permitted. `crates/aep-backend-markdown/src/kernel.rs` asks it exactly that and nothing else; nothing of this repository's appears in a manifest of theirs, at any version |
| [infra-scout](https://github.com/beyond10x/infra-scout) | the actor that holds a kubeconfig and produces the `infra-observation/1` bundles the `infra-*` crates here check three-valued against a desired state |
| [atlas](https://github.com/beyond10x/atlas) | the map of the wider `beyond10x` estate this sits in |

The split with `infra-scout` is the boundary [`docs/VISION.md`](docs/VISION.md) states under "What
this is deliberately not", and it is why that repository exists separately.

## Status

**Shipped and tagged, adopted by nobody. Latest release `0.23.2` (2026-08-26).** Releases are cut
per delivered wave; `docs/status.md` derives its delivered-waves table from the annotated tags, and
`task status-check` fails the gate if the two disagree.

Working today, all gated by `task check`: the AEP document tree, resolution, evidence-guarded
workflows and the `protocol` CLI; evidence that carries an observation date and decays past a
declared horizon; ESS specifications compiling to documentation, OpenAPI, AsyncAPI,
JSON Schema, generated conformance suites, and structural skeletons in Rust, Go and a browser
realization — with one specification run as two applications and compared in every gate run; a
Kubernetes observation checked three-valued against a desired state, with gaps projected back as
reviewable patches; a durable markdown planning store, which is the store this repository plans
itself in; an agent transcript judged against a typed specification; a
[Claude Code plugin](integrations/claude-code/) that plans through it; and the **reference driver**
that walks a workflow rather than being steered along one, deciding each tool call in Rust
(`decide_tool` in `crates/protocol-cli/src/drive.rs`) rather than in a hook script.

Not working yet, stated plainly: `describe_type` reports no lifecycle, so a harness still cannot ask
what a kind's ladder is (**D-P5**); `protocol conformance` runs only against the in-memory backend,
so a durable store's conformance is shown by a Rust test rather than from the command line;
generated code is structural, never behavioural — every algorithm
remains a typed obligation; the conformance runner cannot reach an out-of-process implementation —
holding your own system to a specification means depending on `ess-conformance` from your own tests;
`independent: true` is self-declared, with no signature or attestation binding a verifier to its
evidence.

And nobody's work is governed by this yet. One outside tree has been written against the
specification and validates, which is the first evidence that it is adoptable. This repository has
driven exactly one real story of its own backlog end to end, and that run **blocked** four states
short of the person it was meant to stop at, for two reasons the engine printed —
[`docs/plan/harness-wave-4-governed-dogfood.md`](docs/plan/harness-wave-4-governed-dogfood.md) § W4.1
is the record, kept as it ran. Built is not adopted.

[`docs/status.md`](docs/status.md) is the full status report: the delivered waves (derived from the
tags, drift-checked in the gate), the component tables, and every limitation with its consequence.
The gate is the measurement — run `task check` rather than trusting a number written in prose.

## Where everything is

| | |
|---|---|
| [`docs/guide/`](docs/guide/) | the adopter's guide — start here |
| [`docs/VISION.md`](docs/VISION.md) | why this exists, and how the two halves compose |
| [`docs/status.md`](docs/status.md) | the status report: waves, components, limitations, and the full document index |
| [`AGENTS.md`](AGENTS.md) | the working agreement, including the invariant register — every rule names the check that enforces it |
| [`docs/design/`](docs/design/), [`docs/plan/`](docs/plan/) | designs — proposed until a plan page accepts them — and the plan pages that did |
| [`crates/`](crates/) | the workspace: the protocol, the ESS toolchain, the infrastructure and trace domains, the driver and the CLI |
| [`protocols/`](protocols/), [`principles/`](principles/), [`workflows/`](workflows/), [`profiles/`](profiles/), [`drivers/`](drivers/) | the document tree — the rules themselves, and the step maps that say how a harness obtains what a workflow asks for |
| [`integrations/claude-code/`](integrations/claude-code/) | the plugin: the planning skill and two agents. Its enforcement hooks and eval migrated to the metaharness repository (`epic:metaharness-migration`); enforcement is the driver's own per-call policy through the metaharness seam |
| [`integrations/codex/`](integrations/codex/) | the same instructions in the form Codex reads them — a skill, an `AGENTS.md` fragment and an instruction-surface check. The product for Codex users; driving Codex is metaharness's `metaharness-codex` adapter |
| `.engineering/` | this repository's own project: the planning store it plans itself in, the task under work, and the driver's run records |
| [`examples/`](examples/) | the worked example, the normative specification, the two synthesised applications, and the evidence-horizon corpus a first adopter contributed |
| [`artifacts/`](artifacts/) | the artifact graph as data: kinds, relations, lifecycles and templates. The authoritative model is Rust; these carry the parts that are data |
| [`conformance/`](conformance/) | language-neutral fixtures, scenarios and expected results, plus the shipped `trace-spec/1` documents and the replayed eval-case corpus |
| [`schemas/`](schemas/), [`suites/`](suites/), [`generated/`](generated/) | outputs, each with exactly one owning `xtask` and a drift check in the gate. Do not hand-edit |
| [`xtask/`](xtask/) | the tasks behind every `--check` step: generate, suite, synth, infra, schema, status |
| [`website/`](website/) | the public documentation site |
| [`CHANGELOG.md`](CHANGELOG.md) | what changed, per release |

## Build

Requires Rust 1.85 or newer, [go-task](https://taskfile.dev), the Go toolchain, the
`wasm32-unknown-unknown` target and Node. A check whose toolchain is missing fails and names it
rather than skipping — a check that quietly passes without its toolchain reads exactly like a check
that passed.

```console
task check     # the ten-step gate. Run this; it is the measurement.
```

`task check` runs, in order: `fmt-check`, `status-check`, `clippy`, `test`, `doc-check`,
`schema-check`, `generate-check`, `suite-check`, `infra-check`, `synth-check`. The six `*-check`
drift steps each assert that a committed tree still equals what its inputs produce — a generated
file edited by hand fails the gate rather than surviving in the repository.

The gate is **hermetic**: it calls no API and spends no money. The evals that do are separate tasks
and are never steps of `check` — `task codex-eval` checks the Codex instruction surface for free,
with no model call.

Published documentation: <https://beyond10x.github.io/engineering-protocols/>

## Licence

Apache-2.0. See [LICENSE](LICENSE).
