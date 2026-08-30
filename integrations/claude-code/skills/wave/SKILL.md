---
name: wave
description: Run a wave — pick the next set of stories that can be implemented at once, propose it for approval, then dispatch one implementor per story into its own worktree, send each result to the adversary, and merge what goes green into one integration branch. Use when the operator asks to pick the next wave, to start a wave, to implement several stories in parallel, or to fan out work across sub-agents. Proposes first and stops; it never starts a wave nobody approved.
---

# Running a wave

A **wave** is N stories implemented at once, each on its own branch, merged into one integration
branch, and closed on one gate run. This skill makes *your* session the coordinator.

It is deliberately in two stages with a stop between them. Stage 1 proposes and halts; the operator
approves; stage 2 runs. A sub-agent cannot ask the operator anything, so the approval has to live
here, in the session that can.

## The cycle a wave is one turn of

```
   replan ──▶ dispatch ──▶ integrate ──▶ release ──▶ (replan)
      │           │            │            │
   approve     N agents    gate once     STOPS FOR
   the wave    in parallel  on the       A PERSON
   ▲           adversary    merge, then
   │           per unit     merge to base
   │
   STOPS FOR A PERSON (once; may be granted standing)
   └── the loop
```

**Two stops, and they are the only two.** Everything between them runs without checking in.
`integrate` includes merging the integration branch into the base branch: the gate decided, and a
coordinator that stops there has left finished work stranded and called it caution.

**`release` is not a state a loop may pass through on its own.** A release here is a written
procedure that nothing mechanical enforces; it has already slipped once, in the way these things
always slip — a gate piped into `tail`, which reports `tail`'s exit status, so two runs that aborted
at the first step read as green and two commits were pushed claiming a gate that never ran. A loop
that cut its own releases would industrialise that. So the cycle has **one mandatory human stop**,
and the wave's own approval is a second one that the operator may choose to grant standing.

**Between turns, replan rather than continue.** The store after a wave is not the store before it:
stories closed, findings filed, surfaces moved. A loop that selected its next wave from the
selection it made last time is a loop running on stale facts, and the second turn is where that
starts to cost something.

**What each turn owes the next.** A wave that ran and recorded nothing has produced code and no
knowledge. Each turn writes back: the scope it learned (below), the findings it filed, and the
numbers its pre-flight measured — so the next replan starts from a better store than this one did.

## What you are, and what you are not

You sequence and dispatch. **You never decide that work is done.**

| You do | You never |
|---|---|
| select candidates and propose them | decide a story is finished — the gate's exit code decides |
| arrange worktrees and branches | evaluate a gate, or restate one's verdict as your own |
| dispatch, route red and green, merge | let an implementor write to the planning store |
| own **every** `protocol artifact` call | share one build directory between two worktrees |
| record the evidence and move the stories | report a process killed without having watched it die |
| take the worktrees and their build directories down | remove a tree whose records nobody has read, or force one that is dirty |
| merge the wave into the base branch when the gate is green | ask the operator to do a step this table gives you |

**You are running a loop, not holding a conversation.** A wave stops for a person exactly twice:
at the end of stage 1, and at a release. Everything between those is yours to carry without
checking in — merging a green unit, routing a red one, taking a decision an implementor handed
back, writing a shared file an implementor was held off, merging the integration branch into the
base once the gate is green. Asking about one of those does not make the work safer; it stops a
loop that was running.

**The operator does not need to hear the machine working.** An implementor going green and its
adversary approving is the *expected outcome*, and an expected outcome is not news. Neither is a
merge that applied cleanly, a finding that was found and then fixed, or a store move the facts
already settled. Report a **deviation**: a unit that leaves the wave, a gate you cannot get green,
a decision you genuinely cannot take, an incident, or a fact that changes what the operator would
do next. If nothing has deviated, say nothing and keep going.

This overrides the Agent tool's standing "relay what matters" for the duration of a wave. That
instruction bounds neither how often you relay nor how much, and a wave produces a completion
notification every few minutes: followed literally it converts the loop into a status feed. *What
matters* is the load-bearing half. Almost none of a healthy wave matters.

**Why you own every store write.** The planning store's journal is append-only and committed, and
nothing merges it. Two branches that each move their own story both append to the tail, and the
textual merge produces a document whose revision no event supports — which the store's own
validator reports as forgery. Implementors touching only source files makes that impossible. It is
also the division that works: one agent, one surface; the shared files are yours.

---

## Stage 1 — propose, then stop

### Read the store before you propose anything

```console
$ protocol artifact list --kind story --status draft --format json
$ protocol artifact list --kind story --status proposed --format json
$ protocol artifact graph --format json
$ protocol artifact blocked
```

A candidate is **ready** when its status is `draft` or `proposed`, its `blocked_by` is empty, and
every `depends_on` it carries points at something terminal. Nothing computes this for you — compose
it from `graph` and `list`.

Expect readiness to prune almost nothing. In a real backlog most drafts depend on nothing, so
`depends_on` is a tiebreaker and not a filter. **The selection is a judgement, which is exactly why
it is proposed rather than performed.**

### Scope the candidates before you choose between them

**You cannot select on non-overlap using a store that does not record what its stories touch.** In
a real backlog most bodies cite no path at all, so the disjointness a wave rests on is an assertion
unless somebody establishes it.

Fan out `story-scoper`, one agent per candidate, and run them at once. They are read-only by
charter — which is what makes running many safe: the planning journal is append-only and one file,
so N agents writing it would race. **They return `## Scope` sections; you write them**, one at a
time, through `protocol artifact body` with the complete body.

Each section says where the work lands and marks every line `cited` or `inferred`. Read the
confidence line before you trust the surface: a wave whose disjointness rests on a `low` scope is a
wave that finds out at merge time, with N agents' work already spent.

**A story whose scope cannot be established is not thereby safe.** It is unassessed. Say so, and
either scope it properly or leave it out — those are the two honest options and *assume it is
fine* is not among them.

### Choose on three properties, in this order

1. **Implementable in this tree.** No credential, no paid run, no second harness, no third party.
   A story that cannot be finished tonight is not a candidate however good it is.
2. **Surfaces that do not overlap.** This is the one that decides whether a wave is possible at
   all. Two agents editing one file is a merge conflict whichever order they finish in, and no
   amount of disk or parallelism helps.
3. **Blast radius of one package.** The first wave's point is the loop, not the difficulty.

### Name the overlap risk honestly, per pair

Read each candidate's body for the paths it cites. Then say, for every pair in the proposed set,
whether they touch the same package.

**Where a story cites no path, write *blast radius unknown* — do not guess.** A story that names no
file is not thereby safe; it is unassessed, and saying so is the finding. A proposal that quietly
assumes disjointness is the one that produces a conflict at merge time, with N agents' work already
spent.

### Check what the store will demand on the way out

Before a story can leave its initial status, the store may require edges it does not yet have — in
this repository, that every non-draft story `serves` a declared objective. Find out rather than
assume:

```console
$ protocol artifact lifecycle story
$ protocol artifact validate
```

Whatever the store requires, add it **as part of the proposal**, so the operator sees which
objective each story is being claimed to serve and can disagree before anything runs.

### Write the wave page, then stop

Write the proposal where this repository's plans live, in the shape its existing plan pages use.
Then **stop and report**. A plan is not a work order, and a wave nobody approved is a wave that
spends real time and money on a guess.

Report: the units with the objective each serves, the overlap risk per pair, what you deliberately
left out and why, and the pre-flight numbers from below so the operator approves a wave whose cost
is known.

**Fit it in whatever report budget the operator has set**, and treat that budget as a hard ceiling
rather than a target. This is a proposal, not the plan: the plan is the page you just wrote, and one
line linking to it carries the rest. The store requirements you satisfied, the lifecycle you read
and the scopers you ran are how you did the work, not what the operator decides on.

### After approval, do not stop again until the release

The next thing the operator hears from you is a deviation or the closing report. Not the first
implementor returning, not the first merge, not each adversary's verdict. If you are composing a
message and cannot name what the operator would *do differently* for having read it, do not send
it — go and do the next step instead.

---

## Stage 2 — run

### Pre-flight, before anything is spawned

Every launch-time fact is free to check and expensive to discover late. Refuse the wave and print
the number rather than starting one that cannot finish.

| Check | Refuse when |
|---|---|
| working tree clean, on the base branch | anything uncommitted — you are about to branch from it |
| `git worktree list` | a previous wave's trees are still there — that wave skipped *Take the worktrees down*, and this check refuses rather than repairs |
| build directories from previous waves | one is still on disk. `git` does not know about it, so nothing else will find it. **Print its size** |
| free disk against a floor | below the floor. **Print the number**; "insufficient disk" is not actionable |
| build cache wired up | a compiler cache is available but unset, and you are about to pay for N cold builds |
| one measured build | you have never measured what one worktree costs. Measure it, record it, then choose N |

**N is bounded by what you measured, not by what you were asked for.** A wave of five that fills
the disk at agent four has produced nothing and destroyed four agents' work.

### Arrange the branches

One integration branch for the wave; one branch and one worktree per unit, each forked from the
integration branch.

**Each worktree builds into its own build directory.** Never point two at one. A shared build
directory serves one tree's binaries to another and lets one tree's code generation rewrite
another's committed output — a failure that reads as a mysterious test failure and costs several
gate runs per agent before anybody suspects it.

**Write down the pair — worktree path and build directory path — as you create each one, and name
both after the unit.** That list is the only input the cleanup step has. A build directory placed
outside the worktree is invisible to `git worktree list`, so an unrecorded one is not found again
by anything; it is found by the disk filling up, months later.

Move each story out of `draft` yourself, in the main tree, after adding whatever edge the store
requires. The implementors never touch it.

### Dispatch

Up to N implementors in parallel, one per unit, each pointed at its own worktree and told:

* the story id and its acceptance statement;
* that it writes source and tests only, and nothing under the planning store;
* that it gates **package-scoped** — the whole gate runs once, later, on the integration branch;
* the test-first ordering, and that the red run goes in its report.

When one returns, dispatch the adversary against that worktree. The adversary may add failing cases
and may not edit the implementation.

### Route the result

| Outcome | Do |
|---|---|
| green | merge the unit's branch into the integration branch |
| **red, first time** | send it back to **the same implementor**, which still holds its context, with the adversary's findings |
| **red, second time** | a **fresh** implementor, handed the findings, the previous diff, and what has already been tried |
| **red, third time** | the unit **leaves the wave**. Record why |

The switch at the second failure is deliberate: an agent that has failed the same case twice has a
wrong model of the problem, and a third round in the same context produces the same wrong fix in
new words.

A unit that leaves the wave is a **result**, not a failure to hide. A wave that reports only its
successes has measured nothing.

**Only `green` merges, and there is no fifth row.** The one you will be tempted to invent is *red on
purpose* — an adversary's case that is correct, for a defect this unit is not going to fix, and
deleting it feels like hiding the defect. It is not a row. It is one of these three:

| the case is | do |
|---|---|
| **correct, and the defect is open** | rewrite it to assert the **current** state with the number or behaviour it has today, and a message naming the story and saying what to do when it changes. Green, and it goes red the moment the gap widens |
| **correct, and you will not assert today's state** | the unit **leaves the wave**. Its branch keeps the case, the story keeps the defect, and the base branch stays green |
| **wrong now** — a decision changed under it | rewrite it to assert what was decided, so narrowing it later is a change somebody makes on purpose |

Never `#[ignore]`: a case that pre-excuses its own red is one nobody reads again.

The reason this is a rule and not a judgement call: a suite is read by its **exit status**, and one
red case takes it for everything. `cargo test` without `--no-fail-fast` stops at the first failing
target, so a deliberate red does not add one known failure to the report — it deletes every result
after it. A wave that merges three of those has not documented three defects; it has blinded the
gate and called it honesty.

### Before you promote a finding, check the scenario is one somebody reaches

An adversary is rewarded for constructing a break, and a fixture can break anything. **A red case
proves the code does what the case says under the conditions the case builds. It does not prove
anybody ever builds them.** Whether they do is the coordinator's to establish, and it is the step
that is easy to skip because the failing test is right there and looks like the whole argument.

So before a finding becomes a blocker, a story or a held unit, separate two things and write both:

| | |
|---|---|
| **what was measured** | the assertion, the file and line, the exit status |
| **what reaches it** | the caller, the flag, the default, the documented workflow — or *nothing found* |

*Nothing found* is a real answer and often the right one. It does not make the finding worthless:
a contract that says one thing while the code does another is still wrong, and saying so costs a
doc line. It makes it a **different, smaller** finding, and the difference decides whether the unit
is held or shipped.

The failure this prevents, in the shape it actually takes: an adversary points two projects at one
store with an explicit flag, both runs proceed, and the coordinator promotes it to *two runs walking
one document set* — severity nobody measured, on a configuration nobody was shown to use. The
verified defect underneath was two doc comments disagreeing with the code, which is a two-line fix
and no decision at all.

### What you write into the store carries its source

You demand `file:line` from every agent you dispatch, and you are the only one of you that writes to
the durable record. **Hold your own store writes to the rule you hold theirs to.** Every claim in an
artifact you author is a quotation from a command's output, a `path:line`, or an agent's report —
and anything you concluded rather than read is labelled as such, in the artifact, not only in the
report.

A wrong fact in a chat message is corrected by the next message. A wrong fact in the store is
committed, is read later by somebody who was not here, and is indistinguishable from a checked one.
Do not infer a directory is abandoned from its name, a scenario is common from a passing fixture, or
a defect is severe from how bad it would be if it happened.

### Close it

1. Run the **whole gate once**, on the integration branch.
2. **Read the gate's own exit status.** Not a pipeline's — a gate piped into anything reports the
   last command's status, and two commits have already been pushed here claiming a gate that never
   ran past its first step.
3. Record the evidence the store requires for each story against the merge commit, then move each
   one to its terminal status through `protocol artifact move`.
4. `protocol artifact validate`, and relay its output **verbatim**.
5. **Merge the integration branch into the base branch.** This is integration, not release, and it
   is yours — the same step as merging a unit, one level up. Do not stop here to ask; the gate
   already decided, and stopping strands finished work on a branch nobody asked you to leave it on.

**A release is not this step.** A release is a tag, a version bump and a push, and *that* is the
stop. Do not enlarge it to cover the merge, and do not shrink it to let a loop cut its own tag.

### Take the worktrees down

The wave is not over when the gate is green. It is over when the trees it made are gone — and
nothing else does it for you. `git worktree list` in the next wave's pre-flight is a **refusal**,
not a cleanup, so a wave that skips this step costs itself nothing and blocks the wave after it.

The order is not interchangeable:

1. **Read the untracked records out first.** A driven run's directory, a gate log, the red run an
   agent quoted — all per-worktree and gitignored, so nothing merged them and nothing will.
   Anything a reader would want goes into the closing commit, the wave page or a story *before* the
   tree goes. A worktree removed with its records unread takes the whole account of what happened
   with it.
2. **`git worktree remove <path>`, one per unit.** It takes the checkout and leaves the branch,
   which is the point: the evidence cites `impl/*` by name, so those branches stay and are not
   deleted.
3. **Remove each unit's build directory by name**, from the list you wrote when you made it. This
   is the step that gets missed, because it is the one `git` knows nothing about: a build directory
   outside the worktree survives `worktree remove` untouched. One wave here left a **16 GB** build
   directory standing on a disk already at 93%, every worktree it belonged to long since removed,
   and nothing found it until somebody went looking for free space.
4. **`git worktree prune`, then `git worktree list` — and read the list.** Report what the list
   says, not that you ran the removal. A removal you did not confirm is the same class of claim as
   a process you did not watch die.

**Never force a removal.** `git worktree remove` refuses a tree with uncommitted changes, and
`--force` answers that by discarding them — an agent's unpushed work, or a run's only record. A
dirty worktree at cleanup time is a **finding for the operator**: name the path and what is in it,
leave it standing, and let the next wave's pre-flight refuse on it.

**A unit that left the wave keeps its tree** until the operator says otherwise. It holds the only
copy of what was tried, which is the reason the third failure was recorded rather than hidden.

---

## Supervising what you started

Both of these are mistakes made here, not principles that sounded good.

**Every process you start is yours until you have watched it die.** Verify with `ps` or `kill -0`
and report the check, not the intent. A session once printed *"hogs killed"* on the strength of an
`echo`, and twelve spinners held four cores for thirty-four minutes until the operator found them.

**A wait condition must not match the waiter.** `until ! pgrep -f "the thing"` never exits, because
the waiting shell's own command line contains that string. Prefer the harness's completion
notification and do not poll at all; where you must, match on something the waiter cannot contain.

**Kill a run the moment you know it cannot succeed** — not after the next step, not after the
report. Diagnosing it and letting it continue is the worst of both.

## Reference

The branch and merge conventions this skill follows, and what a wave leaves behind, are in
[references/branch-and-merge.md](references/branch-and-merge.md).
