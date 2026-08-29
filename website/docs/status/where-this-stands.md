---
title: Where this stands
sidebar_position: 1
description: What is implemented, per component, with the numbers the repository's own gate reports.
---

# Where this stands

{/* generated:currency:begin — do not edit; run `cargo xtask status` */}
Current as of the tag `0.33.0` (2026-08-30).

The repository's gate, `task check`, runs **21 steps** — `fmt-check`, `status-check`, `plan-check`, `audit-check`, `version-check`, `dep-check`, `guard-check`, `claim-check`, `clippy`, `test`, `docs-check`, `postgres-check`, `doc-check`, `schema-check`, `generate-check`, `suite-check`, `infra-check`, `synth-check`, `lab-check`, `msrv` and `website`.
{/* generated:currency:end */}

Behind those steps: formatting, clippy with warnings as errors, the test suite, rustdoc with
warnings as errors, the drift checks that regenerate every schema, projection, suite, synthesized
tree, infrastructure IR, the delivered-waves record and this page's own stamps, and compare bytes;
the plan's own store validated and its audit's `file:line` citations re-resolved; the CLI reference
held against the CLI itself; a build on the declared MSRV; and this documentation site's own build.

This page states no suite or test counts: four hand-written counts drifted apart within the
repository's first 48 hours, so the count now lives in exactly one place — the gate's own output.
Run `task check` for the measurement. The stamp above is derived from the repository's tags and its
Taskfile for the same reason, and `status-check` fails when it drifts.

CI runs the same twenty steps; nothing lands that does not pass all of them. The gate needs the Go
toolchain, the `wasm32-unknown-unknown` Rust target and Node beside Rust's own, and a step whose
toolchain is missing fails and names it rather than skipping — a skipped check reads exactly like a
passing one.

Two of those — the MSRV build and the website build — were added on 2026-08-26, and the reason is
the most useful thing on this page:
**CI had been red for eleven consecutive releases behind a green local gate**, because `task check`
was missing exactly the two jobs CI also ran. One failure arrived through the lockfile — a
transitive dependency raised *its own* `rust-version` and the declared MSRV stopped holding, with no
commit of ours touching a line of Rust. The other was a markdown link from a documentation page into
the repository tree, which this site's build resolves and CI's does too. A gate that covers less
than the gate it claims to be is worse than one that admits its scope.

## The headline: the protocol has now governed a real run, and the run stopped

For most of this project's life the honest answer to *has this governed anybody's work?* was **no**.
On 2026-08-21 it changed. `protocol drive` walked a story out of this repository's own planning
store under the `development.driven` profile, with four headless model sessions doing the work and
the Claude Code plugin's hooks as the enforcement arm.

**It blocked in `establish_verifiers`, four states short of the person it was meant to stop at**,
for two reasons the engine printed:

```text
blocked because:
  - establish_verifiers -> implement: ? artifact specification (approved) — declared: specification:agent-charter-eval-cases (draft) [principle spec-driven]
  - establish_verifiers -> implement: ✗ test.first_result == failed — test.first_result = passed [principle test-driven]
```

Both are correct refusals. The model wrote its failing checks as shell scripts — the idiom the
story's own acceptance is written in — and the map it was driving under, `development/default`, runs
`cargo` in every state that names a verifier, so the suite the driver ran came back green and
`test-driven` refused to advance. Nothing was changed to make the run go through; the run is the
finding, and both reasons became rows on the gap register. Both rows are now closed by code:
`development/default` says in its own header that its verifier is cargo, `development/checks` runs
the checks a story carries instead, and an `llm` step is handed the requirements on the way out of
its state as well as the ones in force inside it.

What the same run shows working, read out of its own records rather than from a summary:

| | |
|---|---|
| transitions | 3, every one the engine's, in the snapshot's audit trail |
| model sessions | 4, `is_error: false` in each |
| hook decisions | 80 — 69 allow, 11 deny |
| `permission_denials` in the transcripts | 11, one for one with the denies |
| planning store afterwards | 47 → 58 artifacts, `protocol artifact validate` exit 0 |
| tracked files modified | 0 |
| turns / session time / cost | 224 / 34 m 39 s / $15.42 |

The record, with the acceptance line that admits a failed run as an outcome, is
`docs/plan/harness-wave-4-governed-dogfood.md` § *W4.1 — The first run*.

## AEP — the protocol

The v0.2 scope is complete.

| | State |
|---|---|
| domain model, engine, documents, CLI | implemented — 20 top-level CLI verbs |
| interaction contract, identity, audit | implemented, with an in-memory reference backend |
| conformance suites for backends | implemented — 16 suites, 3 levels, 89 properties at `full`, checked against a deliberately broken backend |
| a durable planning store | implemented as markdown (`aep-backend-markdown`) — **not** an implementation of the storage contract; see [Limitations](./limitations.md) |
| a reference driver | implemented — `protocol drive run\|status\|resume`, with `command`, `llm` and `operator` steps, driving two harnesses: Claude Code through `metaharness run claude`, and the b10x loop through `metaharness run b10x` — the shipped `development/default` map says `harness: b10x` on all six of its `llm` steps |
| running on a real project | twice, from this repository's own backlog — `W4-1/1` (2026-08-21) blocked in `establish_verifiers` for two correct reasons, see above; `W4-3/1` (2026-08-29) walked two states with sessions that could not reach the `protocol` CLI and submitted nothing, and six driver defects fixed under `[Unreleased]` in the changelog carry its run id as their evidence |

The document tree is 49 files — 3 protocols, 22 principles, 4 workflows, 6 profiles, **12 artifact
lifecycles** and 2 step maps — validated against the protocol vocabulary in CI, plus 15 generated
JSON Schemas that CI fails on if they drift from the Rust types.

### The ladder became data

Between `0.13.0` and `0.23.2` the status ladder every plan item climbs stopped being a hand-written
Rust lookup and became a YAML file, decided by
[`entity-core`](https://github.com/beyond10x/entity-runtime) — a separate, IO-free kernel taken as a
pinned dependency. The consequence for a person: **modelling a kind of work this repository never
anticipated no longer needs a pull request here and a release.**

The proof is `outbound-claim`, added in `0.23.0` as **a YAML file and no Rust change at all** — a
ladder for a statement that left the boundary, where `sent` has no path back to `draft` and a wrong
claim is fixed by making a second claim.

| shipped | what it means for a plan |
|---|---|
| the ladder is data | a kind's transitions live in `artifacts/lifecycles/<kind>.yaml`, in your tree or ours |
| the vocabulary is open to authors | `ArtifactKind` and `ArtifactStatus` accept a rung *some ladder declares* — open to authors, closed to typos |
| a rung can cost evidence | `requires:` names the kind and the count; the refusal says which kind of `no` it is |
| a rung can open on a date | the clock is read at the edge and passed in, never read inside the kernel |
| the store has a journal | `protocol artifact history` answers what happened to one artifact, out of append-only JSONL; a corrupt line is skipped **and counted** |
| a move records its provenance | evidence that was *recorded* is separated from evidence that was *asserted*, and a move leaning on an assertion says so as it happens |
| three ladders that needed no Rust | `blocker`, `obligation` and `outbound-claim` |

The dependency is reversible on purpose:
`crates/aep-backend-markdown/tests/kernel_equivalence.rs` holds the kernel's verdict identical to
the lookup it replaced, so the lookup is still standing behind it. See
[Lifecycles, decided as data](../concepts/lifecycles.md).

**What `entity-core` is not**: a store. It performs no IO of any kind — a source scan in its own
test suite refuses the tokens for filesystem, network, clock and randomness, and its whole
dependency list is `serde` and `serde_json`. Every byte the planning store writes is written by
`aep-backend-markdown` here. What it decides is the transition, and nothing else.

### The engine and the driver gained four things

| shipped | what it means for a run |
|---|---|
| evidence names its subject | a record declaring a subject that does not match the task it is submitted against is refused **before** the record is constructed, so a fact about one story cannot discharge another story's gate |
| an advisory enforcement tier | a requirement may be declared advisory with an owner and an exit criterion; it is evaluated and counted and **gates nothing**, so a rule can be measured before it is enforced |
| a dependency that keeps failing stops being called | a step map may declare `depends_on` and a circuit breaker; a repeatedly failing dependency opens the breaker instead of being retried |
| the driver writes what a step was **not** allowed to do | refusals are emitted as their own `trace-spec/1` document, so the absence of a tool call is a checkable fact rather than a silence |

### Adoption, from the other end

`protocol reverse` reads a repository that already exists and was not written with any of this in
mind. Three of its four verbs write nothing — `scan` reports what a repository already says about
itself, `history` reports what its git history says, `openapi` drafts an `ess/1` domain from an
OpenAPI document — and only `init` writes, producing the `project.yaml` that makes a repository an
adopting project.

### Evidence gained a time

An evidence record now carries two dates. `observed_at` is when somebody looked, is required, is the
caller's, and is the identity of the fact; `produced_at` remains the engine's. A date in the future
is refused (`observation_in_future`) rather than stored.

A requirement may declare a `horizon`. Past it the requirement reads `Unknown` — never `False`,
because a lapsed check has not failed, nobody has run it — and `evidence.lapsed` sits beside
`evidence.missing` so a stale gate is distinguishable from an empty one. The horizon is on the
requirement and nowhere else: a record has no horizon field, and a source scan over five crates
refuses any code path that would mutate one.

`protocol evidence scan` reads the same dated-claim convention out of human-written markdown and
reports its own coverage. Against the ground-truth corpus at `examples/evidence-horizons-corpus/`,
contributed by an outside adopter: **43 occurrences, 43 records, 0 unparsed**.

### The harness side

| | State |
|---|---|
| the markdown planning store (`aep.planning-md/1`) | implemented — this repository's own plan is 101 artifacts under `.engineering/planning/`, with an append-only journal behind `protocol artifact history` |
| `protocol artifact new\|move\|relate\|body\|list\|board\|graph\|history\|evidence\|validate\|kinds\|relations\|lifecycle` | implemented — a status move is decided against the kind's lifecycle by [`entity-core`](../concepts/lifecycles.md), and may be refused for want of recorded evidence |
| the Claude Code plugin | implemented — one `planning` skill, two agents (`decomposer` writes, `plan-reviewer` only reads), two `PreToolUse` hooks (`store-integrity` over edits, `driven-surface` over the shell) and an eval that spends real money on a real headless session and judges it two ways |
| `protocol workflow render` | implemented — `svg`, `html`, `png`, `tui`, and `--watch` to redraw a live run |
| `protocol workflow instruct` | implemented — the same documents as words, for a reader with no canvas |
| `protocol workflow flow` | implemented — a workflow projected into the document `b10x-harness workflow run` walks; with `--map`, each node carries its state's step; no guard travels, by design |
| the evaluation, four arms | `protocol eval run --arm raw\|plugin\|driven\|native` and `eval matrix`; a `native` cell has a reading rule rather than a column |

### Transcript conformance

A typed specification over what an agent run actually did, built as the third observation domain in
the repository after a specification and a cluster, and taking the same shape: an authored
`trace-spec/1` document, a content-addressed `trace-ir/1`, and `ok`/`gap`/`unk` verdicts where the
third value means the adapter did not understand the event.

`protocol trace check|inspect|evidence`, **51 expectation kinds**, and two severities: an
expectation gates by default, `Advisory` is evaluated and reported and gates nothing. `trace
evidence` mints an AEP record of kind `trace_conformance` — a summary, never the citations.

## ESS — executable system specifications

Seven waves delivered. A specification is the source of its own documentation, contracts, tests and
the structural part of its own implementation in three targets; when it changes, the change is a
typed, queryable object.

| | State | Since |
|---|---|---|
| the typed model (`ess-domain`) | implemented | `0.3.0-ess-wave-1` |
| resolution, IR, diagnostics (`ess-compiler`) | implemented — an unresolved reference is unrepresentable | `0.3.1-ess-wave-2` |
| four projections: docs, JSON Schema, OpenAPI 3.1, AsyncAPI 3.0 (`ess-gen`) | implemented, committed output drift-checked | `0.3.2-ess-wave-3` |
| the specification as oracle: generated suites, runner, evidence (`ess-conformance`) | implemented — 29 + 31 + 12 scenarios across three example specifications, 13 deliberately wrong implementations all caught by their named scenario | `0.4.0-ess-wave-4` |
| semantic diff and impact closure (`ess-diff`) | implemented — ten construct families; impact narrows down to scenario and generated artifact, with the path attached | `0.5.0-ess-wave-5`, extended by wave 7 |
| structural synthesis (`ess-synth`) | implemented — a language-neutral plan (billing: 45 capabilities = 33 generated / 8 obligations / 4 refused) and three emitters: Rust, Go, browser | `0.6.0-ess-wave-6` through `0.7.0-ess-wave-7` |
| the claims made mechanical — invariant scans, full SHA-256 digests, property tests | implemented | `0.6.1-ess-wave-6.5` |
| one specification, two running applications | implemented — `examples/gatepass/` synthesized to Rust and Go, both started in every gate run and compared on records, seven exchanges and published documents | `0.7.0-ess-wave-7` |

One result worth knowing when weighing the approach: building the oracle changed the specification
language four times. All four gaps had been rendered without complaint by all four projections —
a document does not have to run — and two were found only by the synthesizer refusing to generate a
scenario. The pattern repeated at each layer: the generated suite caught a real defect in the code
generator before any human did, and the dual-target demonstration caught two of the six mutations
held against it by the two applications disagreeing.

## Infrastructure — observed, diagnosed, held to a desired state

Four waves delivered, scoped exactly as the vision allows: nothing in the workspace reaches a
cluster or holds a credential. An external scanner (`infra-scout`, its own repository) writes the
observation; this workspace reads the file, refuses it or compiles it.

| | State |
|---|---|
| observation model and content-addressed IR (IW1) | implemented — seventeen Kubernetes kinds, eleven `INFRA-*` refusal codes over the observation, secrets only as digests |
| typed dependency graph, diagnosis, candidates, directions (IW2) | implemented — twenty `INFRA-DIAG-*` rules, three `INFRA-PROP-*` candidate rules, an HTML view |
| desired state and simulation (IW3) | implemented — `infra-spec/1`, twelve expectation kinds, three-valued verdicts; a False without a gap or an Unknown without a reason is unrepresentable |
| gaps projected back as patches (IW4) | implemented — a reviewable patch tree: mechanical changes as patches, human decisions as obligations, contradictions as refusals; the round trip is asserted |
| the CLI | nine verbs: `protocol infra validate\|compile\|inspect\|graph\|diagnose\|view\|simulate\|diff\|project` |
| a real example | `examples/k3d-dev-cluster/` — 28 expectations: 11 hold, 12 gaps, 5 undecidable; the committed projection closes 9 gaps with generated changes, leaves 16 as obligations and refuses none, across 2 patch files and 3 new objects; all drift-checked |

## Not built

The compact list; [Limitations](./limitations.md) carries the consequences of each.

* `describe_type` reports no lifecycle, so a harness cannot ask what a kind's ladder is (**D-P5**).
* `protocol conformance` runs only against the in-memory backend. The markdown and SQLite backends
  pass the sixteen suites — in Rust tests, each with a faulty-backend guard — but not from the
  command line.
* No out-of-process conformance runner, on either side.
* No behavioural synthesis — every algorithm is an obligation, by design.
* No attested evidence — `independent: true` is structural, not cryptographic.
* Obligations are plan entries, not yet artifacts a task can own (deferred by decision).
* Nothing gates on `trace_conformance`, so a driven run's own transcript check is provenance in the
  audit trail rather than a gate that can stop it.
* A second **vendor** harness has never been driven. The driving path now has two harnesses under
  one step map — Claude Code and the b10x loop — so neutrality is answered for a loop this family
  owns, and not yet for a stranger's. `0.22.0`'s Codex rollout adapter is checked against committed
  **synthetic** fixtures, and the verified reader for Codex rollouts lives in `metaharness`, not
  here.

---

**Sources.** `git tag -l`; `target/debug/protocol validate`, `conformance --level full --format
json`, `evidence scan`, `--help` (verb counts), `artifact lifecycle`, `artifact history` at `0.32.1`;
`ls artifacts/lifecycles/` and `find .engineering/planning -name '*.md' | wc -l` (the ladder and
artifact counts); `Taskfile.yml` (the twenty gate steps); `drivers/development/default.yaml`
(`harness: b10x`, six steps); `CHANGELOG.md` § *Unreleased* (`W4-3/1`);
`crates/aep-backend-markdown/Cargo.toml` and `src/kernel.rs` (the `entity-core` pin and the seam);
`docs/plan/harness-wave-4-governed-dogfood.md` § *W4.1*;
`docs/plan/gap-register.md`; `examples/evidence-horizons-corpus/expected.json`;
`examples/k3d-dev-cluster/simulation.json` and `projection/SUMMARY.md`;
`generated/rust/billing/PLAN.md`; `suites/generated/*/suite.json`;
`crates/trace-domain/src/spec.rs` (the expectation kinds); `CHANGELOG.md` §§ *0.10.0*–*0.23.2*.
