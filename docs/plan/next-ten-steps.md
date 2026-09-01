# The next ten steps — after the store waves

> **Status: delivered 2026-08-28, except step 5** — released as `0.32.0` (`bfd391d`) and `0.32.1`
> (`6868ccc`). Nine of the ten steps shipped in one wave; **step 5, the driven run, is prepared and
> not run**, because it spends money and stops at a person (D1's default). Its hand-off is
> [`w4-3-to-try.md`](w4-3-to-try.md).
>
> **Two things this page says that the tree contradicts, corrected here rather than quietly.** The
> *two releases* below did not happen that way: steps 6–10 depended on nothing in 1–5, so everything
> except the driven run shipped in **0.32.0** and there is no 0.33.0. And **D6's candidate changed**
> — step 1 found `story:retry-budgets` already shipped and closed it, so the run drives
> `story:operator-resume-ux`. D4 was also decided against its own default, and its row already says
> so.
>
> The Goal below still reads as a promise. One clause of it — *one real story is driven end to end*
> — is the one thing outstanding, and `story:governed-dogfood-run` is still `draft`.
>
> Originally **proposed 2026-08-28**, written against `aep` 0.31.0 (`1419f1c`) and
> `entity-runtime` 0.13.0 (`ddee747`), the day waves F–H closed (`store-waves-f-g-h.md`). Every fact
> in § 1 was read from the two trees on that date; nothing below it is a fact until a story ships.
> Ten steps rather than a wave, because they cross four epics and two repositories and the point
> is the order, not a theme.

**Goal: the plan tells the truth about the driver, a story becomes `implemented` on evidence the
store can show, one real story is driven end to end on this repository's own backlog, and the
adopter-facing defects that need no decision are gone — before any new capability is started.**

## 1. Where this stands — verified 2026-08-28

| fact | evidence |
|---|---|
| `epic:planning-store-as-backend` is `implemented`: 16/17 stories, one archived as superseded | `.engineering/planning/epic/planning-store-as-backend.md`; `protocol artifact validate` → `valid` |
| `epic:reference-driver` is `draft` at 1/12, yet the driver exists | `crates/aep-driver`, `crates/aep-driver-spec`, `crates/protocol-cli/src/drive.rs` (90 functions), `drivers/development/`; `story:driver-spec-crate`, `story:protocol-drive-verb`, `story:default-step-map`, `story:own-engineering-store` sit at `active`/`proposed` |
| `epic:evidence-gated-completion` is 4/7; the three open stories name what the store waves just enabled | `story:completion-audit-join` (join through the journal → the event log exists in every store since G3/H1), `story:completion-needs-evidence` (a verdict, its two mechanism halves marked **Shipped** in its own body), `story:evidence-producers-for-the-driven-map` |
| `story:governed-dogfood-run` (W4.1) was run once and stopped short | `harness-wave-4-governed-dogfood.md:3` — *"W4.1 has been run once — W4-1/1, 2026-08-21, and it stopped short"* |
| Every store now counts evidence from the entity's events; a move over SQLite is decided on what was recorded | `e550a6c`; `store_selection.rs` 6/6 with exit codes asserted |
| A release restamps ~120 files whose content did not change | `33d3636` (117 files), `e7ca3bc` (121 files); `story:generator-version-stamp` names it |
| `entity-runtime` has asked for a verdict and carries a false "blocking fact" about this repository | `story:entity-runtime-mapping` here; `task:roadmap-page-is-current` there |
| `epic:adopter-feedback-round-1` is 1/14; five of its defects are recorded as needing no decision | `story:adopter-bugs` — *"A1, A2, A3 plus B2's compile-time directory name and G2's untyped failure policy"* |

## 2. The ten steps

| # | step | story | epic | after | size |
|---|---|---|---|---|---|
| 1 | The plan tells the truth about the driver | eleven stories of `epic:reference-driver` moved or re-scoped on evidence | reference-driver | — | hours |
| 2 | *What made this done*, answerable from the store | `story:completion-audit-join` | evidence-gated-completion | — | 1 day |
| 3 | The W4.3 verdict, recorded | `story:completion-needs-evidence` | evidence-gated-completion | 2 | ½ day |
| 4 | The default map produces its own evidence | `story:evidence-producers-for-the-driven-map` | evidence-gated-completion | 1 | 2 days |
| 5 | One real story, driven end to end | `story:governed-dogfood-run` | reference-driver | 1, 3, 4 | 1 day + a paid run |
| 6 | Five adopter defects that need no decision | `story:adopter-bugs` | adopter-feedback-round-1 | — | 1 day |
| 7 | Is it actually open? The vocabulary audit | `story:open-vocabulary-audit`, `story:ova-relation-vocabulary`, `story:ova-predicate-operator-vocabulary` | adopter-feedback-round-1 | — | 1–2 days |
| 8 | The verdict `entity-runtime` asked for, and their false fact | `story:entity-runtime-mapping`; ER `task:roadmap-page-is-current` | — | — | ½ day, two repos |
| 9 | Harness-neutrality as a gate step | `story:shell-echo-harness` | cross-harness-portability | 5 | 2 days |
| 10 | A release that rewrites nothing it did not change | `story:generator-version-stamp` | — | — | 1 day |

~~Two releases: **0.32.0** after step 5 (the plan is true, completion is evidence-gated, one story was
driven), **0.33.0** after step 10.~~ **Not what happened, 2026-08-28.** Steps 6–10 turned out to
depend on nothing in 1–5, and three of them (1, 6, 9) were audits that closed on work already
shipped — so waiting for step 5 would have held nine finished steps behind one paid run. Everything
except the driven run shipped in **0.32.0**, patched by **0.32.1**; there is no 0.33.0, and the
release after step 5 is whatever number is next when it runs.

The rest of the paragraph held: nothing in 6–10 changed a printed output that 1–5 pin.

## 3. Each step, in one paragraph

**1 — The plan tells the truth about the driver.** `epic:reference-driver` reads as barely started
and is mostly shipped: the router (`aep-driver`), the spec crate, `protocol drive` with its lock,
run directory and resume, the default step map, the plugin hooks. Each of the eleven open stories is
read against the code and moved through `protocol artifact move` on a `test_result` naming the
tests that hold it — or re-scoped in its body to what is genuinely left (`story:retry-budgets`,
`story:reusable-workflow-nodes`, `story:operator-resume-ux` look real) — or archived as superseded.
No code. Acceptance: every status in the epic is one a named test or gate stands behind, and
`validate` reports no status reached on an assertion for it.

**2 — What made this done.** Since G3 every store's history is an event log, and since H1 the
evidence record is an event on the artifact at the revision it was recorded against. The join the
story asks for therefore exists as data; what is missing is the verb. `protocol artifact explain
<id>` (or `protocol explain` over a planning store — the story says `protocol explain`) answers, per
status the artifact reached: the move, the evidence it rested on, each record's source, reference,
instant and the **revision the artifact was at** — so an edit made afterwards cannot make an old
record look like it was about the new text. Reads through the contract, so it answers alike over
markdown, SQLite, Postgres and the hybrid; `store_selection.rs` gains the verb. Acceptance: the
story's four lines, the third read as the new verb.

**3 — The W4.3 verdict.** `story-completion-evidence-design-v0.1.md` is proposed-not-accepted; the
story's own body already marks two of its three lines **Shipped**. The step is the decision, in the
design document's own status block: accepted, accepted in part, or refused, with the reason — and the
separable `delivers` row for `artifacts/relations/relations.yaml`. Gap-register row *"a story's
`implemented` is a claim nothing checks"* closes either way, as it says it will. Default in § 4.

**4 — The default map produces its own evidence.** `protocol drive run --map
drivers/development/default.yaml` on a `kind: feature` task starts today only with
`--allow-evidence-gap`, because no step mints `test_result`, `static_analysis`, `diff` or
`specification` records with `producer: verifier`. The story's acceptance is specific down to the
`regression_suite` record the coverage scan cannot see. This is the prerequisite step 5 wedged on.

**5 — One real story, driven end to end.** W4.1 again, on a story from this backlog that step 1
leaves genuinely open — `story:retry-budgets` is the natural candidate: a real gate, a real diff,
an acceptance line somebody argued about. Every transition through the engine, every status move
through `protocol artifact move` (the write guard enforces, `validate` audits), every `llm` step's
transcript checked and submitted as `trace_conformance`. A wedge is a **recorded result**. This
step needs money and a person at the keyboard — see § 4.

**6 — Five adopter defects.** `story:adopter-bugs` is scoped as *"five defects that need no
decision"*: A1–A3, B2's compile-time directory name, G2's untyped failure policy. One commit per
defect, each with the regression test the adopter's report implies. Independent of everything else
and the cheapest adopter-visible improvement on the board.

**7 — The vocabulary audit.** For each thing the documents invite an adopter to declare, is the
vocabulary open, and if closed, is the closure stated with its reason? The two `ova-*` stories are
the two places already known to be closed without a stated reason: relation names
(`RelationKind`, a Rust enum — this repository's own `store_selection.rs` and `journal::Change`
lean on it being one) and predicate operators in mapping form. Acceptance: a table in
`docs/guide/open-vocabulary.md`, and for each closed vocabulary either an opening or a sentence a
reader can disagree with.

**8 — Across the arrow, both ways.** `story:entity-runtime-mapping`: `entity-runtime` expressed our
eight ladders as definitions under `examples/aep/` and asked whether the verbs mean what ours mean;
the answer is an equivalence test here over the pinned tag, and a verdict in the story. Their
`task:roadmap-page-is-current`: `docs/roadmap.md` § 1 still says this repository has never heard of
theirs, and it now takes five of their crates at one tag. Two small commits, one each side; the
arrow (`adr/0002`) stays one-way in every manifest.

**9 — Harness-neutrality as a gate step.** `story:shell-echo-harness`: a shell-echo
`LlmStepExecutor` and a reader for its own transcript dialect, so one `llm` step runs inside `task
check` with no model and no network and is decided by `protocol trace check` against the same
specification a real harness is. The `partial` Codex tier already landed (gap register row 38);
this is the tier that makes portability a gate rather than a claim. The live Codex `full` tier
stays where it is: it costs money and needs a keyboard.

**10 — A release that rewrites nothing it did not change.** Two of today's four release commits
touched 238 files whose only change was a version string, because `compiler_version` and
`generator_version` are stamped into every projection and every conformance evidence file. The
story's answer is a stamp derived from what the artefact was made **from** — the specification's
digest, the generator's own source digest — so a version bump that changes neither rewrites
nothing, and a change to either rewrites exactly what it should. Acceptance: `cargo xtask
generate` after a version bump reports `0 changed`; AGENTS.md § Releases loses five lines.
**Shipped**, and D4 came out narrower than its default: the two digests already there were the
whole answer, and no generator-source digest was added. `synthesizer_version` and the synthesised
trees' `planner_version` went with them, because a release that still rewrites twelve files has not
stopped rewriting files. The proof is `a_version_bump_rewrites_no_generated_file` in `xtask`.

## 4. Decisions this plan takes

| # | decision | default if nobody answers |
|---|---|---|
| D1 | Step 5 needs a paid model run and the operator at the keyboard for the approval step | everything up to the run is prepared and gated; the run itself waits for a short numbered *to try* block and a yes |
| D2 | Step 3's verdict on `story-completion-evidence-design-v0.1.md` | **accepted in part**: the two shipped halves accepted; the producer-independence judgement refused as engine work and kept under `story:evidence-producers-for-the-driven-map` |
| D3 | Step 7: `RelationKind` stays a closed enum or opens | stays closed, with the reason written: a relation name is a graph semantic the engine interprets (`decomposes` builds the tree `board` prints), and an open one would be a name nothing can act on |
| D4 | Step 10: what replaces the build version in a stamp | **taken, and narrower than the default: nothing replaces it.** The specification's content is already `source_digest` and the slice's is `contract_digest`; a digest of the generator's own source answers *by which build* no more actionably than the tag did and moves on every commit to the generator rather than once per release. Recorded as deviation 13 in `docs/design/reconciliation-v0.2.md` §5 |
| D5 | Two releases or one | ~~two — 0.32.0 after step 5, 0.33.0 after step 10~~ → **one, 0.32.0 (+0.32.1)**: steps 6–10 depended on nothing in 1–5, so holding them behind a paid run bought nothing |
| D6 | Which story step 5 drives | ~~`story:retry-budgets`~~ → **`story:operator-resume-ux`**. Step 1 did find `retry-budgets` shipped and closed it. The fallback named here was `reusable-workflow-nodes`; `operator-resume-ux` was taken instead because it fits `story:governed-dogfood-run`'s own default better — one real defect, one crate, mechanical acceptance |
| D7 | Step 2's verb: `protocol artifact explain` or `protocol explain` | `protocol artifact explain <id>` — it is a plan question; `protocol explain` already means a policy evaluation |

## 5. Not in these ten, and where each lives

| item | why not here | where |
|---|---|---|
| the four lifecycle concepts (`decision-with-default`, `time-based-transitions`, `blocker-relation`, `outbound-claims-and-status-vocabulary`) | ladder semantics — a wave of their own, with a design first | `epic:adopter-feedback-round-1` |
| the agent-eval tasks and `story:driven-eval-acceptance` | paid runs, and step 5 is the run that decides whether the driver is ready to be evaluated | `epic:self-evaluation` |
| `epic:checker-vocabulary-depth` | four expectation kinds nobody is blocked on today | that epic |
| the live Codex `full` tier | money and a keyboard; step 9 makes the seam a gate first | `story:codex-adapter` |
| the 58 GB of other sessions' temporary trees under `~/.cache/claude-tmp` | not this repository's, and not mine to delete unasked | the operator |

## 6. How each step is accepted

The same way the store waves were: the story's own acceptance lines, a `test_result` recorded
through `protocol artifact evidence` naming the gate run, and the move to `implemented` refused
until it is there. A step that turns out to be a decision (3) or a re-scoping (1) is accepted on the
written verdict, and `validate` must not report it as a status reached on an assertion.
