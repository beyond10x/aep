# The three-arm evaluation program — plan to completion

Status: **proposed** (operator direction, 2026-08-23). Supersedes the standalone HC-wave proposal
of the same day by absorbing it (HC-1/2/4 → phase R2, HC-3 → phase R1). Builds on
`epic:self-evaluation` and its stories rather than rivalling them; where this plan and that epic
disagree, the disagreement is named in § Decisions and the epic is amended deliberately or wins.

## The question the program ends by answering

How well do **Claude Code** and **Codex** follow the workflows this repository defines, under three
treatments:

| arm | treatment | what it represents |
|---|---|---|
| **a** | raw rendered instructions in the prompt, nothing else | "text + hope" — what everyone does today |
| **b** | the shipped plugin (`integrations/claude-code/`, `integrations/codex/`) installed | this repository's product for ordinary users |
| **c** | driven: metaharness spawns and enforces, EP's engine decides every tool call over the hook wire | the ceiling — enforcement instead of persuasion |
| **d** | native: the b10x loop, where the published toolset **is** the policy and nothing adjudicates a call | a surface instead of a refusal — a tool outside the published set is not denied, it does not exist |

**A fourth arm was added after this page was written** — `dce6db5` put `Arm::Native` on
`Harness::B10x` — so the title and every *three* below read as *every arm*. It is not a fourth
flavour of `c`: `b` injects a policy into a vendor's loop and `c` decides one from outside per
call at a seam, while `d` has no vendor loop to treat, which is why it runs on a third harness
rather than on Claude Code or Codex and why `protocol eval run` refuses to launch it.

Terminal artifact: a **matrix of per-expectation outcomes** (held / violated / unobservable) per
harness × arm × workflow, judged by the same trace expectations from committed transcripts, plus
cost/tokens/wall-time columns. Not a scalar score — see Decision 1.

The program is **complete** when that matrix has been produced from real runs **and** one
improvement-loop iteration has been demonstrated: worst plugin finding → plugin fix → arm-b re-run
→ observed delta.

## Design constants (the science)

1. **The instrument is constant across arms; only the treatment varies.** Every run — all three
   arms — is spawned by metaharness into a hermetic scratch home with the recording hook installed.
   Arms a and b run the hook in a new *observe* mode (allow everything, record everything); arm c
   runs it enforcing, with EP's engine answering. This is what makes the arms comparable, and it is
   why we do **not** build `metaharness audit` over foreign transcripts (it stays refused by name).
2. **The judge is constant and ignorant of the arm.** `trace-spec` expectations score every
   transcript identically; the checker never learns which treatment produced the stream. A judge
   that knew the arm could not produce a comparison anyone should trust.
3. **No model-judged verdicts.** `epic:self-evaluation`'s refusal holds: every expectation is a
   bound over observed values, or it is not an expectation.
4. **The boundary holds.** No crate dependency in either direction, ever. The `metaharness` binary
   on `PATH` is a *tool* dependency of the eval runner, like `git`; absent binary = skip by name,
   never a red gate. Everything that crosses, crosses as bytes with goldens pinned on both sides.

## Phase R1 — aep round by itself (free, public, stranger-runnable)

| story | content | acceptance |
|---|---|---|
| R1.1 instruction render | deterministic prose rendering of a workflow + its timed principles — arm a's treatment is a **committed artifact**, not an ad-hoc prompt (`aep-render` today draws pictures; this is a text backend beside it) | same workflow → byte-identical instructions; rendered files committed under `generated/` |
| R1.2 coverage map | every workflow in `workflows/` mapped to plugin coverage in both plugins, or a named gap; the map is checked, not prose | a workflow added without plugin coverage or a named gap turns the check red |
| R1.3 eval-case corpus | per workflow, K small tasks with trace expectations (extends `conformance/trace/expectations.*`; folds in `specification:agent-charter-eval-cases`) | each case: task + expectations + fixture transcript replay green in `task check` |
| R1.4 matrix assembler | assemble `trace_conformance` records + a run manifest (arm, harness, workflow, plugin digest, model, pins) into the outcome matrix; refuse a record whose manifest is incomplete | committed fixture records → the exact matrix, reproducibly |
| R1.5 contract gating (= HC-3) | `contract_result` wired into a protocol requirement so `breaking_changes > 0` blocks a transition; `--record -` stdin | mutation test: a breaking record blocks, a clean one passes |

Exit: `task check` green; nothing in R1 needs metaharness, credentials, or a network.

## Phase R2 — metaharness round by itself (private; 2 paid runs ≤ $1)

| story | content | acceptance |
|---|---|---|
| R2.1 codex launch face (= HC-1) | c1 launch vectors close the CT-4 named `Obligation::Gap`; `checked` 10→N moves deliberately; contract goldens regenerated, EP's committed fixtures refreshed — the cross-repo regeneration loop exercised once for real | CT-4 symmetry test shows no launch gap; both repos' goldens agree |
| R2.2 codex loopback door (= HC-2, LP-4) | shaped by V-LP6; stub-verified free, paid confirm listed as a to-try | per V-LP6's answer, no silent fallback between adapter classes |
| R2.3 re-pin claude → 2.1.240 (= HC-4) | installed binary is 2.1.240, every run warns; one capture re-record | golden-version-pair green without the warning |
| R2.4 codex allow path | the `allow` half of Codex's decision wire, driven once for real (today only deny is proven — status.mdx names it refused) | one paid run: an allowed call executes, the rollout says so; refusal lifted by name |
| R2.5 observe mode | a `DecisionMode` that allows all and records all, named in `capabilities`, attested in `session.started` — arms a/b's spawn mode | conformance vector: observe-mode stream carries every `tool.decided` with the mode named |
| R2.6 plugin injection | `--plugin-dir` copies a plugin into the scratch home; the installed plugin's digest lands in the launch attestation — crossing #4 | plan is a value: copy list + digest readable before spawn; mutation: edited plugin → different digest |

Exit: `task check` green; R2.4's paid run recorded like every other capture.

## Phase R3 — interplay hardening (the seam grows one crossing and a runner)

**Built, 2026-08-23, `story:eval-runner` and `story:eval-dry-run`.** The heading's promise is now
half wrong and worth correcting rather than quietly leaving: the seam grew **no crossing and no
event field** — see R3.2 — and what R3 actually added is a runner and one attestation this side
learned to read.

| story | content | what was decided or built |
|---|---|---|
| R3.1 crossing-#4 goldens | plugin-injection attestation bytes pinned on both sides, like frame/event/contract before it | **Built, EP side — and corrected once by a live run.** `crates/protocol-cli/fixtures/eval-run/claude-plugin-attested.jsonl` and its two siblings carry `session.started.hermetic.installed_plugins[].digest`; the manifest's `plugin_digest` is that string **verbatim** — never a hash of the directory on disk, which would attest bytes the session never saw. `the_plugin_digest_in_the_manifest_is_the_one_the_session_attested_byte_for_byte` reads the expected value out of the fixture so the two cannot drift. Two refusals are the load-bearing half, and both are about the experiment: arm `plugin` over a stream attesting **no** plugin (`EVAL-STREAM-006`, the treated arm without its treatment) and arm `raw` over a stream attesting one (`EVAL-STREAM-007`, the control arm with it). A plugin attested with **no digest** is refused too (`EVAL-STREAM-008`). Correspondence to metaharness's `c1-plugin-injection` vector is named in that directory's `README.md`, with the frame contract's caveat: **until MH replays these exact bytes, this is one implementation agreeing with a transcription of another**. **Corrected 2026-08-23**: the reader took the digest from the top-level `plugins` echo, which is the *vendor's* init list — Claude Code writes one, Codex writes `null`, because metaharness will not mint a vendor field it did not receive. `hermetic.installed_plugins` is the **instrument's** row and is written on every adapter, and it is what crossing #4 actually is. The first live pilot run refused on this and the refusal was correct; the field was not |
| R3.2 run manifest across the seam | ~~the manifest R1.4 requires is emitted by metaharness (event field)~~ | **Decided: no. The manifest is assembled runner-side and the seam gains nothing.** A manifest has two kinds of field. The ones that *describe the run* — `harness_version`, `plugin_digest`, `model` — are already in `session.started`, and `transcript_digest` is what the check this runner performs states about the bytes it judged. The ones only the runner knows — `arm`, `workflow`, `case`, `observed_at` — could not be in a stream at all: metaharness runs a *session*, and *this is arm b of case X* is a claim about an **experiment** it has no business knowing. Emitting an `eval.run-manifest/1` fragment would have put `raw`/`plugin`/`driven` into a repository that does not have those words, and made every manifest change a two-repository release. What keeps it honest is fail-closed reading: a stream whose `session.started` lacks a field the manifest needs is **refused by name** and no manifest is written (`EVAL-STREAM-003`…`-012`, mutation-tested). One narrowing to the plan's field list: `model` is read from the stream, not from what the runner asked for — a runner writing down the model it *requested* would record one the run may not have used. **Corrected 2026-08-23**: `model` is now `Written`, exactly as `plugin_digest` is — key required, explicit `null` legal, absent refused. Codex's wire names no model at `session.started` at all (a 62-event pilot run never states one), so *the harness did not say* had to become something a manifest can state; inventing `gpt-5-codex` because it is the likely answer would be writing the one document the matrix trusts. The matrix carries `"model": null` in JSON; its text rendering has no model column to spell, and the runner's own per-run line says `(unstated)` |
| R3.3 the eval runner | `protocol eval` (or an xtask): binary-on-PATH detection, skip-by-name, `METAHARNESS_LIVE=1` gate for paid arms, `--budget-usd` hard cap | **Built** as `protocol eval run` (`crates/protocol-cli/src/eval.rs`, beside `matrix`). `metaharness` is found on `PATH` or via `METAHARNESS_BIN`; absent, it refuses by name and exits **2** — its own code, so *install something* and *fix what you wrote* are distinguishable without parsing prose — and the suite proves both directions (refusal where absent, **skip by name** where present). `METAHARNESS_LIVE=1` and `--budget-usd` are both required before a spawn; the cap is checked **before** each launch against `--assume-usd-per-run` (default `$0.25`), because a cap enforced afterwards is a receipt. A `total_cost_usd` the wire writes as `null` is **unknown**: it counts against the budget at the assumed rate and the manifest states no cost at all. Arms: `raw` → the workflow's committed instruction document in front of the case's task, `--decisions observe`; `plugin` → the **task alone** plus `--plugin-dir integrations/<harness>`, because arm b's treatment *is* the plugin and prefixing the instructions too would measure a and b at once. **Arm `driven` is a named refusal** (`EVAL-RUN-004`): `protocol drive run` launches it, and a second launcher would be a second policy to forget — the runner **reads** a driven run instead, through `--stream` |
| R3.4 free dry run | the whole pipeline — spawn (scripted upstream), record, judge, assemble — green with zero spend, in EP's gate | **Built**: `crates/protocol-cli/tests/eval_dry_run.rs`. Four recorded streams — two harnesses, all three arms, one corpus transcript and three derived fixtures — go through the runner's ingest path and `protocol eval matrix`, and the matrix is asserted **byte for byte** against `fixtures/eval-run/dry-run.matrix.{json,txt}`: 34 facts held, 2 contradicted, 4 runs, no vendor binary, no credential, no network. `--stream FILE` is what makes it free — the whole runner minus the spawn — and it is not a test hook: it is how a driven run enters the matrix and how a paid run is re-ingested after the manifest's rules change |
| R3.5 the native arm's write scope | the eval's step map declares `scope:` — `.engineering/planning/**` `write: denied` and a `**` catch-all `write: allowed` — so a native cell's store-integrity row is a refusal the tool made on the path, rather than compliance nobody was in a position to refuse (`docs/design/native-arm-store-integrity-design-v0.1.md` § 6 O2) | **owed**, and owed **there**: the key goes in `metaharness/evals/aep/driven.steps.yaml`, and this side has nothing to do — `map.rs` validates it and `drive.rs` already renders it into the b10x argv. Until it lands the reading rule stands as written for the reader who meets the table (`website/docs/reference/cli.md` § *Evaluation surface*): a clean store-integrity row on a native cell says *the model did not do it*, never *the call was refused* |

The one flag that is a decision rather than an option: **`--observed-at` is required**, and
deliberately not defaulted to *now* as `protocol trace evidence` is. An evidence record is minted by
the process that made the observation; a run manifest is a committed document that has to assemble
to the same bytes twice, and a clock in it would make every re-ingest a diff.

## Phase R4 — the measurement (paid, operator in the loop)

| step | content | spend cap |
|---|---|---|
| R4.1 pilot | 1 workflow (`workflows/development/default.yaml`) × 2 harnesses × 3 arms × 2 tasks = 12 runs | $10 |
| R4.1b **pilot 2** | **owed.** Pilot 1 ran 4 of the 12 cells, arm `c` refused before start, and its numbers cannot be compared with anything made after 2026-08-23: the corpus went 10 rows → 12, and the streams went to a scratchpad that no longer exists. Re-run every cell, freeze the corpus first, commit the streams — `story:three-arm-pilot-2` | ~$10 |
| R4.2 freeze | read the pilot, fix instrument faults only (never expectations and instrument in one change), freeze the corpus | — |
| R4.3 sweep | all workflows × 2 harnesses × 3 arms × K tasks (4 workflows, K=3 → 72 runs) | $25 |
| R4.4 the report | the matrix, from committed transcripts, delivered as the answer to "how well do they behave" | — |
| R4.5 loop demo | worst plugin finding → plugin fix → arm-b re-run of affected cases → delta observed | $5 |

## After completion

`pi` and `opencode` join as harness #3/#4 against the *same frozen corpus* — the program doubles as
the admission test every new adapter must pass before its plugin is believed. (Operator sequencing
2026-08-23: pi/opencode come after this program's first matrix.)

## Hypotheses, labelled

- **H-arms**: expectations-held ordering a ≤ b ≤ c. Plausible, unmeasured — the program exists to
  test it. Arm c may still lose on the cost and latency columns while winning on conformance.
- **H-plugin-asymmetry**: the codex plugin is thinner than the claude one, so the a→b delta is
  larger on Claude Code. Unmeasured.
- What is already **verified** (paid, 2026-08): arm c's enforcement floor on both vendors — a
  frame admitting no shell produced the vendor's own denial record. Nothing about arms a or b has
  ever been measured.

## Decisions (defaults if nobody objects)

1. **No scalar score.** `epic:self-evaluation` refuses "any score, percentage or leaderboard"; the
   deliverable is the per-expectation outcome matrix — counts of facts, no aggregation into a
   number. Default: the epic stands unamended.
2. **Uniform capture via observe mode**, not vendor-native runs plus an audit verb. Default: as
   designed above; `metaharness audit` over foreign transcripts stays refused.
3. **Budgets**: $10 pilot, $25 sweep, $5 loop demo, all behind `METAHARNESS_LIVE=1` and
   `--budget-usd`. Default: these numbers.
4. **`story:native-plugin-eval` stays draft** — it waits on `claude plugin eval` leaving early
   access, an external gate; the program uses R3.3's runner and the story swaps the runner later
   without moving any expectation.
