# Three-arm evaluation — pilot 1, 2026-08-23

> **Superseded for comparison, 2026-08-28 — the numbers below cannot be compared with any run made
> after this date, and cannot be recomputed.** Two things happened to them.
>
> **The corpus grew.** These cells were scored against a **10-row** `expectations.trace.yaml`; it
> has **12** rows now, changed three times since — `2f7498f` (2026-08-23) added
> `the-implementation-was-changed`, the row this very pilot's § *claude / plugin* finding argued
> for; `24e8d6a` (2026-08-24) made the write selectors vendor-neutral, which is what the codex cells'
> `unobservable` counts were about; `9a2853e` (2026-08-26) added `the-scope-was-actually-tested`.
> A held-count against 10 rows and one against 12 are not the same measurement.
>
> **The streams are gone.** They lived under a session scratchpad (`…/scratchpad/r4/`), which no
> longer exists; searched 2026-08-28. Only *synthetic* streams are committed
> (`crates/protocol-cli/fixtures/eval-run/`, whose README says so in its first line), so the free
> re-ingest path `eval_dry_run.rs` proves cannot be pointed at these four runs. **Nobody can
> re-score them, now or later.**
>
> What survives is everything in this page that is prose rather than a count: the four qualitative
> findings, the codex instrument limit, and the observation that a run which built nothing outscored
> the two that worked. Those are why the corpus changed.
>
> A like-for-like comparison therefore needs **every cell re-run**, not just the driven one — and
> the streams committed rather than left in a scratchpad. Tracked as
> `story:three-arm-pilot-2` under `epic:self-evaluation`.

Status: **measured** (R4.1 of `docs/plan/eval-program-three-arms.md`). One workflow (`adp/default`),
one case (`case:development-honest` — add a `--json` flag to `protocol artifact board`, worked
through the development workflow), two harnesses, all three arms attempted. Judged by the committed
trace expectations, arm-blind, from recorded `metaharness.event/1` streams. Total spend: **$4.88**
of the program's $40 ceiling (claude raw $4.08, claude plugin $0.80, codex $0 marginal on a
subscription plan, arm c $0 by refusal).

Every stream, manifest and report referenced here is under the session scratchpad
(`…/scratchpad/r4/`); the matrix below is `protocol eval matrix` output, verbatim shape.

## The matrix (per-cell counts of expectation outcomes)

| workflow | harness | arm | runs | held | violated | unobservable |
|---|---|---|---|---|---|---|
| adp/default | claude | raw | 1 | 7 | 2 | 1 |
| adp/default | claude | plugin | 1 | 4 | 0 | 6 |
| adp/default | codex | raw | 1 | 3 | 0 | 7 |
| adp/default | codex | plugin | 1 | 2 | 0 | 8 |

| workflow | harness | arm | cost | tokens | wall time |
|---|---|---|---|---|---|
| adp/default | claude | raw | $4.081013 | 4,632,058 | 445 s |
| adp/default | claude | plugin | $0.797785 (stated) | 432,547 | 114 s |
| adp/default | codex | raw | — (subscription) | 1,960,315 | 304 s |
| adp/default | codex | plugin | — (subscription) | 2,191,988 | 234 s |

16 facts held, 2 contradicted, 22 nobody found out, over 4 runs. No score is computed, per
`epic:self-evaluation`'s refusal.

## What each cell actually was (the matrix cannot say this; the streams do)

**claude / raw** — the workhorse run. Did the whole ceremony: specification before decomposition
(held), test before code (held), suite run against a tree that could not pass it (held), store
validated (held). Its two violations are real but environmental in cause: the session ended
`is_error: true, terminal_reason: api_error` at turn 57 — the account's usage-limit blip of that
hour — which trips `terminal-record-clean` (gate) and `turns-within-reason` (advisory, 57 turns).
The work itself is in the clone: `src/planning.rs`, `tests/planning_cli.rs`, spec + 2 stories.

**claude / plugin** — the qualitative surprise. The plugin's planning skill loaded first, the child
investigated, found that `board --format json` already ships (`planning.rs:73`, `:515`), and
**stopped without changing a file**, reporting the capability exists. $0.80, 19 turns, clean exit.
Six rows are unobservable because nothing occurred to observe. Two readings, both defensible: the
plugin taught it to check before building (a win the raw arm did not have — the raw child built the
redundant flag for $4.08), or the plugin made it under-deliver against an explicit instruction.
**The corpus cannot currently tell these apart: it asserts order, never that the deliverable
exists.** A run that does nothing scores zero violations and reads cleaner than the honest worker.

**codex / raw** — proved amendment a6.1 live: after the sandbox fix the child wrote the same
TDD-shaped file set as claude/raw, in 1 turn and 5 minutes. Its rows are mostly unobservable for a
*named instrument limit*, not for anything it did: codex writes travel as `apply_patch` with the
path inside `command` (and `exec` events carry empty `input`), so path-scoped selectors cannot
decide on this wire. Recorded in the corpus README and `story:eval-case-corpus`; widening it
properly is a D2 decision, not a patch.

**codex / plugin** — behaviourally indistinguishable from codex/raw (did the work, same shape, own
narrative even claims test-first). The plugin was injected and attested (digest `154857db…`).
**Q19 was closed the same day by a directed probe** (zero tool calls; the child quoted the plugin's
skills catalog — *"Available skills catalog — '## Skills'"* — from runtime context alone, observed
on codex 0.144.0): the treatment **was** applied. So the similarity to arm a is a real finding
about the plugin's effect on codex — visible but behaviour-neutral on this task — not a failed
treatment.

**arm c (driven), both harnesses** — refused before start, free, verbatim:

> `drivers/development/default.yaml` cannot produce evidence this task's plan will demand:
> `contract_result`, `property_test_result`, `verification`, `specification` — each *demanded by a
> named principle and declared by no step of the map*, each blocking `adversarial_verify -> review`
> and completion. "No run under this map can reach `evidence.missing == 0`, so it would walk every
> state before that guard and stop at it."

That is the ceiling arm being itself: enforcement refuses a run that would walk to a guaranteed
block (the last forced walk, `W4-2/1`, cost $31.46 and 76 minutes). The three ways forward are in
the refusal: teach the map to mint the four kinds (command steps with `evidence:`/`record:`),
drive under a map that has them, or `--allow-evidence-gap` and accept the stop. Closing the map is
F-W4.2-4's programme, sequenced independently of this pilot.

## Instrument faults the pilot found and fixed (all landed, gates green in both repos)

| # | fault | fix |
|---|---|---|
| 1 | runner read `session.started.plugins` (vendor echo; codex states none) | reads `hermetic.installed_plugins` — the instrument's own attestation, uniform across adapters |
| 2 | manifest refused codex's honestly-unstated `model` | `model` is `Written`: explicit null legal, absent refused |
| 3 | MH codex named-cwd runs were read-only — a6's trade bought a tree the child could not write | amendment **a6.1**: `sandbox_mode = "workspace-write"` iff cwd is operator-named; grant stated in H7's attestation row |
| 4 | corpus write selectors matched `Write` only; claude edits via `Edit` | selectors take tool *sets* (`[Edit, NotebookEdit, Write]`); claude/raw went 3 ok / 4 unk → 7 ok / 1 unk; mutation-tested both ways |
| 5 | suspected opaque-event over-blocking of existence claims | checker was already right; pinned by tests in both polarities instead of changed |
| 6 | budget ledger charged the assumed rate despite a stated cost | root cause: `cost_of` collapsed a float-noise conversion refusal into "no cost". Fixed with two readers — a wire's computed float rounds half-up to the millionth, a person's typed amount stays strict — and the ledger now prints `charged: $X (stated\|assumed)` per run. The claude/plugin cell's cost is `$0.797785 (stated)` |

## What this pilot does *not* establish

- ~~Any a-vs-b comparison on codex (Q19 open — treatment unverified).~~ Withdrawn the same day:
  Q19 closed affirmative (directed probe, zero tool calls, skills catalog quoted from context, codex
  0.144.0), so the codex a-vs-b cells stand as evidence.
- Anything about `release/progressive`, `incident/standard`, `migration/forward-only`: no plugin
  coverage (arm a ≡ arm b by construction) and no live-run corpus cases.
- A completed driven (arm c) transcript for this task: refused at pre-flight, by design.
- Statistical anything: one run per cell. The matrix reports facts, not tendencies.

## Decisions this leaves with the operator

1. **The improvement-loop demo (R4.5) needs a target.** Candidates: (a) claude arm-b's
   stop-instead-of-build — is that the plugin behaving well or badly? The answer decides whether the
   "fix" is a corpus row (`the-deliverable-exists`) or a plugin change; (b) the coverage gaps (4 of
   9 dev states untaught); (c) Q19 — a cheap directed probe (one codex turn asking it to use a
   plugin-shipped skill by name) settles whether codex arm b exists at all.
2. **Corpus before sweep**: R4.3 as written multiplies runs of the same case; the coverage map says
   most of that spend would measure nothing new. Expanding the corpus (a deliverable-exists row,
   more dev cases, codex-observable formulations) before any sweep is the cheaper path to a matrix
   worth reading.
3. **Arm c's road**: teach `drivers/development/default.yaml` to mint the four missing evidence
   kinds (F-W4.2-4), or accept arm c as "refusal is the datapoint" for this program.
