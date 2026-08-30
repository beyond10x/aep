# Wave — reading what is already recorded

> **Status: proposed 2026-08-30, not accepted.** The first wave selected by the plugin's `wave`
> skill rather than by hand, written against `a8b139b` with the working tree carrying the
> `implementor`/`adversary`/`wave` surfaces uncommitted. Nothing here is a work order until the
> operator says so.
>
> **This page is the skill's own dry run, and the dry run rewrote it.** The first pass selected
> three units by grepping bodies for `crates/` paths. Then four `story-scoper` agents ran
> concurrently over the candidates and **changed the answer**: one unit is not ready to dispatch,
> one collides through a file nobody would have guessed, and one candidate turns out to have largely
> shipped four days ago with nobody moving the artifact. § 2 is the revised selection; § 2a is what
> the first pass got wrong, kept because it is the argument for scoping before selecting.

**Goal: three stories that each make something the tree already records legible, without adding a
new observation — chosen for touching three surfaces that do not overlap, so they can be
implemented at once and merged onto one branch closed by one gate run.**

## 1. Where this stands — measured 2026-08-30 against this store

| fact | evidence |
|---|---|
| 40 draft stories, 36 dependency-ready | `protocol artifact graph --format json`, `depends_on` closure over `implemented`/`archived` |
| so `depends_on` prunes 4 of 40 | selection is judgement, not a query — which is why this page exists to be approved |
| 24 of 40 draft stories cite no source path | grep for `crates/…` over each body |
| of the 16 that do, 9 name `aep-domain`, 8 name `protocol-cli` | same scan — the two crates a naive wave would collide on |
| 1 of 40 draft stories carries a `serves:` edge | `graph --format json`; every story this wave moves needs one added first |
| free disk 66 G; the operator's `target/debug` 28 G | `df`, `du` |
| **a package-scoped build in a fresh worktree: 30 s wall, 1.6 G target** | measured 2026-08-30 — `cargo test -p protocol-cli --no-run` in a detached worktree at `a8b139b`, own target dir, 258 compile requests |
| **so N=5 costs ~8 G, not ~140 G** | 5 × 1.6 G. The fear that bounded N was the *workspace* target; package-scoped gating is what makes it wrong |
| `sccache` wired in for the measurement: **0.00 % Rust hit rate** | `sccache --show-stats` — this workspace has never been built through it, so it buys nothing on the first wave and pays from the second |

## 2. The units — after scoping

| # | story | serves | surface | confidence | verdict |
|---|---|---|---|---|---|
| 1 | `story:board-columns-come-from-the-ladders` | O2 | `crates/protocol-cli/src/planning.rs:2132-2177` | **high** — only column build in the tree | **in** |
| 2 | `story:usage-series-assertions` | O3 | `crates/trace-domain` + `crates/trace-spec`, one enum behind an exhaustive dispatch | **high** — two precedent commits show the exact file set | **in, with a caveat** |
| — | `story:recurrence-key` | — | `aop-domain` + `aep-domain/src/workflow.rs` + generated schema | **medium** | **out** |
| — | `story:evidence-subject-binding` | — | `crates/aep-domain/src/requirement.rs:374-382` | high on the remainder | **out — verify first** |

**A wave of two, not three.** Every scope section is now in the story it describes, so the next
replan starts from a better store than this one did.

**Unit 2's caveat, which no grep would have found.** The trace vocabulary's kind count is hard-coded
in **six** places, and two of them are `README.md:78` and `website/docs/status/where-this-stands.md:181`.
So this unit edits the README, and conflicts with any wave-mate that does. Unit 1 does not, so the
pair is safe — but the rule this establishes is general: *a unit's surface is not only its crate.*

**Why `recurrence-key` is out.** Not because it is hard. Because it is **under-specified in a way
that only reading the tree reveals**: its acceptance names "where the workflow's other outputs are
declared", and no workflow, profile or protocol document in this repository declares outputs —
`git grep outputs` returns one unrelated hit. It also needs a Rust change nobody anticipated
(`RawState` is `deny_unknown_fields`, so a new YAML key is a field plus a regenerated schema), and
there are no incident instances in this tree to demonstrate the rollup against. Dispatching it would
have produced an agent asking questions nobody was there to answer.

**Why `evidence-subject-binding` is out, and this is the most valuable thing the dry run produced.**
Its work appears to have **largely shipped on 2026-08-26** — `9fa3a0c` + `51f55f4`,
`gap-register.md:72` marks the axis Done, `CHANGELOG.md:1487-1510` ships it — and three of its four
acceptance lines are satisfied by that code. The artifact is still `draft`, one revision, no
evidence, and nothing in the store records the gap. Three website pages still state the limitation
as live, one of them edited *four days after* the guard landed. And both recorded citations for the
defect site are stale line numbers (`:311` and `:243`; it is at `:378`), so anyone sizing it off a
citation opens the wrong function. **This is store hygiene, not a wave unit.**

## 2a. What the first pass got wrong

Kept deliberately: it is the argument for § *Scope the candidates* existing at all.

| first pass, by grep | after scoping |
|---|---|
| 3 units, all "surface-disjoint" | 2 units; one dropped as not ready |
| `recurrence-key`: "workflows + maybe aop-domain", radius cited | needs `aep-domain/src/workflow.rs` and a regenerated schema; names a declaration site that does not exist |
| `usage-series-assertions`: "trace crates", radius inferred | correct — and it also edits `README.md`, which a crate-level view cannot see |
| `evidence-subject-binding`: excluded, radius unknown | correctly excluded, for a different and better reason: it may already be done |

**Four concurrent read-only agents, roughly four minutes.** The cost of not running them is a wave
that dispatches an under-specified unit and a conflict on a file nobody listed.

## 3. The two units, one paragraph each

**1 — the board's columns come from the ladders.** A bug found while landing
`story:blocker-relation`: `board` iterates a list compiled into the binary, so an adopter-declared
status — a `blocker` at `open` — appears in **no column at all**. Its acceptance is mechanical,
which is what makes it the right first unit. One caveat the scoper found: if the acceptance's
"passkeys fixture plus one blocker" adds a document to `examples/planning-passkeys`, that store's
counts are asserted verbatim by fifteen sites; the existing blocker test builds a scratch store
instead, and the implementor should follow it.

**2 — usage-series assertions.** A vocabulary for sequences over data the trace IR already keeps.
`TraceIr::requests` exists and **nothing reads it today**, so this is its first consumer. The kind
lands in one enum behind a compiler-enforced exhaustive dispatch, which means a missing arm fails to
build rather than passing silently. Open before it starts: the wire spelling of the new kinds
(nothing like `usage.trend` exists, and `NAMES` is asserted sorted), and whether the coverage guard's
required negative case exists in the committed fixtures.

## 4. Decisions, with the default if nobody answers

| # | decision | default taken on silence |
|---|---|---|
| D1 | **N for this wave.** Two, or push for three | **two — and the constraint is scope, not disk.** The measurement above settles it: 1.6 G per worktree means N=5 costs 8 G of 66 G free. What bounds this wave at two is that only two candidates are ready, which no amount of disk fixes |
| D2 | **`story:evidence-subject-binding`** — implement the remainder, or first verify what shipped | **verify first.** A story whose acceptance is three-quarters satisfied by code nobody recorded is a store problem before it is a work item. The verification is a `plan-reviewer` pass, not an implementor |
| D3 | **`story:recurrence-key`** — leave it, or answer its open questions | **leave it in draft, and add the two questions to its body.** Where outputs are declared, and which store counts. Both are the protocol owner's |
| D4 | **Wire `sccache` into the repo, or export it per wave** | **per wave, in the pre-flight** — `RUSTC_WRAPPER` in `.cargo/config.toml` changes every build every adopter runs. Note it earns nothing on wave one: the measured hit rate was **0.00 %**, because this workspace has never been compiled through it. It is an investment that pays from wave two |
| D5 | **The three website pages that still state the subject limitation as live** | **fix them with D2's verification, not with the story.** A page that describes a shipped guard as missing is drift, and it is cheaper to fix than to explain |

## 5. What is deliberately not in this wave

- **`story:acquisition-phase-set`**, which is otherwise a strong candidate and reads as a clean
  single surface. Its first acceptance bullet is *"a decision is recorded, either way"* — an
  operator's decision, not an implementor's. A unit that stops at a person is not a fan-out
  candidate; it is a question, and it should be asked before it is scheduled.
- **Anything naming a credential, a paid run or a second harness.** 14 of the 36 ready drafts
  mention `metaharness`, `b10x`, `codex`, `substrate`, a token or a paid run. All excluded by the
  first selection property, which is also the property that makes a wave finishable in one sitting.
- **Filing the citation gap as work.** § 6 names it; whether it becomes a story is the operator's.

## 6. What this wave has to teach the specification

The reason to run it at all. `adp/default` governs **one** unit of work — there is no state, kind,
relation or field anywhere in `artifacts/`, `workflows/` or `protocols/` that says *these N run at
once*. A wave today is three untyped things: this page, an annotated tag, and a branch convention
that until today lived only in `git log`. Four questions this run has to answer in its own words,
each of which is a YAML decision afterwards:

| question | what it becomes |
|---|---|
| What makes two units safely concurrent? | today, prose — *surfaces that do not overlap*. A typed answer is a declared scope per unit and a disjointness check, and `scope:` already exists on an `llm` step in `aep.driver-steps/1` |
| How do N per-unit verdicts become one wave verdict? | `story:fanout-promote` already models this shape for release promotion — *a set of targets, each with its own guard*, and an aggregate that must not round to one answer |
| What is the integration step, in protocol terms? | `adp/default` has no merge state, because it governs one unit. Either a wave workflow has one, or integration is outside the workflow and the documents say so |
| Who may declare a wave done, and on what? | today, one `test_result` against the merge commit closes every story in it (`c203308`). A real rule, written nowhere until `AGENTS.md` § *Branches and waves* |
| Where does a wave sit in the cycle, and which of its states may a loop pass through alone? | the cycle is `replan → dispatch → integrate → release → replan`. **`release` may not be automated**: it is a written procedure nothing enforces, and it has already slipped once — a gate piped into `tail` reports `tail`'s status, so two aborted runs read as green and two commits were pushed claiming a gate that never ran. A loop that cut its own releases would industrialise that. So the typed cycle carries at least one mandatory human stop, and saying which is a protocol decision, not a harness one |

**And one the dry run answered before the wave ran.** *Can a coordinator establish blast radius from
the store?* **No — not today.** 24 of 40 draft stories cite no source path, and two of this wave's
own three units had their radius inferred rather than read. Either a story's body carries where it
lands, or a wave's disjointness claim is an assertion. That is a finding about the artifact
templates, not about these three stories, and it is the most useful thing this page produced.
