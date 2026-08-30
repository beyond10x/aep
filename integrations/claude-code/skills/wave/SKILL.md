---
name: wave
description: Run a wave — pick the next set of stories that can be implemented at once, propose it for approval, then dispatch one implementor per story into its own worktree, send each result to the adversary, and merge what goes green into one integration branch. Use when the operator asks to pick the next wave, to start a wave, to implement several stories in parallel, or to fan out work across sub-agents. Proposes first and stops; it never starts a wave nobody approved.
---

**Skill version 0.2.0** — the version in `.claude-plugin/plugin.json`; the stage-1 proposal quotes it.

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

**Report by exception; the shape is the operator's, not this skill's.** How anything reaches a human
is governed by the operator's own output rules, wherever their harness keeps them — this file does
not restate them and must not grow a copy. What is specific to a wave is only *what counts as news*:
an implementor going green, an adversary approving, a merge applying cleanly and a store move the
facts already settled are the **expected outcome**, and an expected outcome is not news. Report a
deviation — a unit that leaves the wave, a gate you cannot get green, a decision you genuinely
cannot take, an incident. If nothing has deviated, say nothing and keep going.

This overrides the Agent tool's standing "relay what matters" for the duration of a wave. That
instruction bounds neither how often you relay nor how much, and a wave produces a completion
notification every few minutes: followed literally it converts the loop into a status feed. *What
matters* is the load-bearing half. Almost none of a healthy wave matters.

A sub-agent's report is **input to you, never output to the operator.** Its register is not yours to
pass on: take the findings, drop the voice.

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

**A wave of one is a wave.** If exactly one candidate passes those three properties, propose it
alone and run it. The loop is the same at N=1 — propose, dispatch, attack, gate once, merge, record
— and the loop is the part that has to keep running. Holding a ready story back until a second one
arrives stops the loop and buys nothing; padding the set with a candidate that failed a property is
worse than that. Say in the proposal that N is one, and why the others were left out.

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

**Approval removes the stop. It never removes the page.** An operator who pre-approves — *pick a
wave and run it* — has waived the stop and nothing else. A wave that read the two as one step wrote
no page at all, and that session compacted twice: the integration branch, both worktree paths, both
build directory paths, which unit was at which stage and why a third story was excluded existed only
in a context window that was summarised and discarded. Write the page before you dispatch, whether
or not you are going to stop.

**The page is a file stage 2 keeps current, not a document written once.** Per unit it carries the
branch, its head commit, the worktree path, the build directory path, the scratch root, and the
stage the unit has reached. It also carries the line naming the commits the wave will make. Update
it when a unit changes stage. It is then the input the cleanup step reads and the point a
coordinator recovers from after a compaction, instead of both of those living in a context window
that may not last the wave.

Report: the units with the objective each serves, the overlap risk per pair, what you deliberately
left out and why, the pre-flight numbers from below so the operator approves a wave whose cost is
known, and **one line naming the commits approval authorises** — N unit commits, the merges, the
closing store commit, the merge to the base branch, and nothing else. If the operator's standing
rule is *never commit unasked*, that line is the whole of the exception they are granting.

Two items in that report are fixed text rather than judgement.

**Print the skill version line from the top of this file.** It says which copy of these rules the
session is running, in the transcript, where a mismatch is visible. A wave once halted at the merge
boundary with five commits already made, because the copy loaded into the coordinator predated the
commit-authorisation section; the reload command it ran reported "no changes" and re-injected
nothing.

**Name the `subagent_type` each dispatch will use, in full, with its plugin prefix.** A built-in
agent used where a plugin agent exists is a deviation you report, not a substitution you make. One
session ran 23 of 24 dispatches as `general-purpose` because the plugin's agents were missing from
the copy it had loaded, and described that as having run the implementor.

**Fit it in whatever report budget the operator has set**, and treat that budget as a hard ceiling
rather than a target. This is a proposal, not the plan: the plan is the page you just wrote, and one
line linking to it carries the rest. The store requirements you satisfied, the lifecycle you read
and the scopers you ran are how you did the work, not what the operator decides on.

### After approval, do not stop again until the release

**Approval is what makes the commits legitimate, and it is bounded.** An operator's standing rule
may well be *never commit unasked* — this one's is. Approving a wave grants exactly the commits the
wave needs: one per unit, the merges into the integration branch, the closing store commit, and the
merge into the base branch once the gate is green. Nothing else. **Not** a push, **not** a tag,
**not** a release, **not** work that was not part of the wave, **not** the next wave. When the wave
closes the grant closes with it, and the operator's ordinary rule is back in force.

State that boundary in the stage 1 proposal, so approving the wave is visibly approving those
commits. An operator who did not know they were granting commit rights did not grant them.

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
| working tree clean, on the base branch | anything uncommitted, or the main checkout is not on `main` — you are about to branch from it, and a checkout sitting on somebody else's branch is a second session's workspace |
| `git worktree list` | a previous wave's trees are still there — that wave skipped *Take the worktrees down*, and this check refuses rather than repairs. Also refuse when a branch you are about to use is already checked out in another worktree: `git` will refuse the checkout later, and by then agents are in flight |
| the model budget the wave will spend | you have not asked the operator what is left of it. N agents run at once, so the limit arrives mid-flight and kills them; it does not queue them |
| build directories from previous waves | one is still on disk. `git` does not know about it, so nothing else will find it. **Print its size** |
| free disk against a floor | below the floor. **Print the number**; "insufficient disk" is not actionable |
| build cache wired up | a compiler cache is available but unset, and you are about to pay for N cold builds |
| one measured build | you have never measured what one worktree costs. Measure it, record it, then choose N |
| **the repository's own agent file** | you have not read it. `AGENTS.md` (or its equivalent) is where an adopting repository states the build, test and layout rules a wave has to obey — where build directories go, which gates are package-scoped, what a test fixture may assume. **Read it before you create the first tree.** This skill's defaults lose to it every time |

**N is bounded by what you measured, not by what you were asked for.** A wave of five that fills
the disk at agent four has produced nothing and destroyed four agents' work.

**N is also bounded by the model budget the operator states, and it defaults to 4.** Ask what is
left of the budget and size the wave to it. Six agents on the largest model returned HTTP 429, four
of them were killed, and the wave stalled 47 minutes while each was resumed by hand. If the operator
gives no number, run 4.

**Record every worktree path and build directory path in the wave page as you plan them**, not only
in this session. One wave had five worktrees and five build directories deleted under it while five
agents were working in them, and nothing could say who did it, because nothing outside the
coordinator's context knew they existed.

**The dirty-tree refusal has exactly one sanctioned override, and it is not yours to take.** The
operator says to proceed; you paste `git status --porcelain` into the wave page first, name which
of those paths any unit's surface touches, and leave the changes where they are. Never stash them,
never commit them, never branch them away. They may belong to another session.

**Gate your own opening commit before the first worktree exists.** The pre-flight above runs before
you have written anything; the commit that creates the wave page and moves the stories is the first
change on the integration branch, and nothing has checked it. One wave's opening commit put an
absolute path from the machine's home directory into a plan page, which the repository's own check
rejects — and it judges the index as well as the tree, so the integration branch was red from its
first commit. Three separate implementors found it, each spent report space on it, and none could
fix it, because it was not their file. Run the cheap gate steps — everything but the compiler and
the test suite — on the integration branch after your opening commit and before you create a tree.
It costs seconds.

### Arrange the branches

One integration branch for the wave; one branch and one worktree per unit, each forked from the
integration branch.

**Each worktree builds into its own build directory.** Never point two at one. A shared build
directory serves one tree's binaries to another and lets one tree's code generation rewrite
another's committed output — a failure that reads as a mysterious test failure and costs several
gate runs per agent before anybody suspects it.

**A unit's record is a triple — worktree path, build directory path, scratch root — written into
the wave page as you create each one, and all three named after the unit.** That list is the only
input the cleanup step has. A build directory placed outside the worktree is invisible to `git
worktree list`, so an unrecorded one is not found again by anything; it is found by the disk filling
up, months later.

**You assign the scratch root, under the wave's own directory. The agent does not choose it.**
Scratch is the part `git` cannot see at all, and an unassigned one lands wherever the agent thought
best: a 579 MB rollback probe found only because its agent reported the size, a 95 MB and a 17 MB
directory beside it, a rendered bundle tree written outside the wave's directory entirely, and 15
files written to the system temporary directory against a standing rule. Assigning it costs one line
in the brief and turns cleanup into a loop over the triples instead of a reading exercise over
prose.

**When no disjoint pair exists, split one file by function rather than dropping the wave.** This is
the fallback, not the shape to reach for, and it holds only with all three of these:

1. Declare the split in each brief by symbol and line range, not as "do not collide".
2. Require each agent to paste its diff hunk headers, which are checkable against the ranges it was
   given.
3. Run `git merge-tree --write-tree --merge-base=<base> <a> <b>` as a dry run **before the first unit
   merges**, and read the tree it writes.

Step 3 is the one that pays. A textual merge can apply cleanly and still leave one agent's caller
pointing at a symbol the other renamed, which no conflict marker shows you. It is one command, it
needs no worktree and no build, and running it at integration time instead is running it after both
agents' work is already spent.

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

**Write each unit's brief to a file and pass the path; do not retype it into a prompt.**
[references/unit-brief.md](references/unit-brief.md) gives the shape: the repository's invariants
once, then what is specific to this unit, including the triple you assigned it. Six dispatches in
one wave re-declared roughly forty lines of invariants each, by hand, and nothing detects an
omission — an agent that was not told about a generated file simply does not know. A correction
round then carries only what changed. The file also survives a compaction, so a coordinator can
read what it told an agent.

**A charter or skill you edited during this session is not what this session runs.** The loaded copy
is the one from the last plugin load. Run `/reload-plugins` before the next dispatch and name the
version you reloaded to, in the report. One session dispatched an agent type that had been written
minutes earlier and got `not found`; another ran a whole wave against a copy of this file that
predated the section it was being held to.

**Require a fixed header as the first six lines of every report, before any prose.** Reports in one
wave ran to thousands of tokens each with the verdict scattered through them, and reading them is
the coordinator's whole job for minutes at a time — the fan-out serialises on the one agent that
cannot be parallelised. The header is exactly this:

```
unit: <unit id>
verdict: green | red | blocked
cases: executed <before>→<after>, red <n>
origin: introduced <n>, pre-existing <n>, undecided <n>
wrote-outside-worktree: <paths> | none
needs-coordinator: yes | no
```

`origin` is the adversary's line; an implementor writes `n/a`. `cases` is the executed count, not
the count that exists: a suite that selects none of an agent's new cases exits 0, and a green exit
with an unchanged count is the failure. One wave had a lane whose filter silently dropped two cases,
and only the number would have shown it.

**Route on the header. Read the prose only when the header says to** — `needs-coordinator: yes`, a
verdict that is not green, a case count that did not move, or a path outside the worktree. A green
header with a moved count and no outside paths takes the next step in *Route the result* and no
reading.

### Route the result

| Outcome | Do |
|---|---|
| green | merge the unit's branch into the integration branch |
| **red, and no case has failed twice** | send it back to **the same implementor**, which still holds its context, with the findings |
| **red, and a case failed again after being fixed** | a **fresh** implementor, handed the findings, the previous diff, and what has already been tried |
| **red after two full attacks** | stop attacking. It goes to a person |

**Route each finding on its `origin` column before you route the unit.** The adversary's findings
table says whether a defect was `introduced` by this unit, is `pre-existing`, or is `undecided`.

| `origin` | Do |
|---|---|
| `introduced` | back to the implementor, with the finding |
| `pre-existing` | file a story. It does not block the unit |
| `undecided` | you decide, and you record why in the artifact you write |

Before that column existed, both adversaries of one wave supplied the distinction in prose anyway —
"CONFIRMED (out of scope of this change)", "pre-existing rather than introduced" — and routing cost
a judgement per finding, read out of paragraphs. The column makes the common case mechanical. The
`undecided` row is the one you may not push back onto an agent.

**"The same implementor" is sometimes not there any more.** A sub-agent is gone once its session has
been compacted, and a killed one cannot be resumed. When the agent the second row names cannot be
reached, use a fresh implementor handed the brief file, the unit's diff, the findings, and what has
already been tried — the third row's handover, used for a different reason. Say which of the two you
did. The rule exists because context is worth keeping, and a wave that quietly starts fresh every
round has given that up without anybody deciding to.

**Re-read free disk when each unit returns, and report the number only if it crossed the floor.**
One wave measured 84 G at pre-flight and 62 G later in the same wave — a compiler cache growing to
its cap, plus one 6.2 G build directory. The pre-flight cannot see this, because it runs before the
builds that cause it. One command converts a wave-ending failure at unit N into a one-line refusal.

**Count a case that fails again after being fixed — not red rounds.** These are different, and the
difference decides whether an implementor keeps its context or loses it. An agent that fixed
everything it was given and then failed on *new* ground found by a *new* attack has a working model
of the problem and the freshest possible context; taking it off is a pure loss. An agent whose fix
did not hold is the one with the wrong model, and that is what the switch is for. The first wave to
run this skill fired the rule twice on red rounds, was defensible both times, and was wrong both
times — which is worse than firing wrongly, because nobody notices.

**Attacking has a budget, and it is two passes.** A correction re-enters `adversarial_verify` —
`adp/default` runs `implement → verify → adversarial_verify`, so green does not route to merge and
a second pass is the rule, not diligence. But the second pass is not the last one that would find
something: on the wave of 2026-08-30 two units came back with 4 then 3 findings, and 4 then 5. A
third would likely find more, and that is the argument for a person deciding rather than a number.
After two attacks, hand it over — do not open a third.

**The hand-over carries the trend, not just the verdict.** State findings per pass, how many were
regressions of an earlier pass, and whether the count rose or fell. Two passes finding 5 then 1 and
two passes finding 5 then 6 are different situations, and they currently reach a person looking
identical. Four waves' worth of observed pairs: 4 then 9, 3 then 4, 5 then 6, 6 then 6 — none with a
regression of the earlier pass, all diverging or flat. That is the number the decision turns on and
nothing asks anyone to write it down.

**You verify the correction that answers the second pass.** By construction it is code no adversary
has seen, and the budget otherwise ends in an unverified state. Read the diff yourself and check two
things: that no assertion was dropped, and that a re-pinned test was re-pinned and not relaxed. Then
the unit merges or it goes to a person. Do not open a third attack to cover this.

**Two adversarial cases can be mutually unsatisfiable, and no rule fixes that.** One wave produced a
pair where satisfying either broke the other, differing only in a count the story's *Out of Scope*
refused. The implementor was right to stop rather than satisfy both. A suite of adversarial cases is
a specification, and two of them can disagree; when they do, that goes to a person and never to
another correction round.

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

   **Capture one exit status per step, not one for the run.** A whole-gate command is long enough to
   be killed — by a harness timeout, by memory pressure, by a signal nobody sent on purpose — and a
   gate that dies at step 10 of 21 tells you nothing about steps 11 to 21. `exit status 143` is a
   signal, not a verdict, and reporting it as *the gate failed* is the same error as reading a
   pipeline's status. Run the steps individually, record each one's own code, and you keep every
   result the run got to before it died. On the wave of 2026-08-30 five of eight whole-gate attempts
   were killed mid-flight; per-step capture is what turned that from nothing into a full result.

   Two more things a step's exit code does not tell you, both seen on that wave:
   **a step that skips itself still exits 0** — `postgres-check: skipped, ENTITY_POSTGRES_URL unset`
   is indistinguishable in the status from a step that ran — and **a fresh worktree is not the
   environment the gate expects**: the integration branch's tree has no `node_modules`, no
   downloaded fixtures and none of whatever else was installed by hand in the main checkout, so a
   step can fail `127 command not found` on a tree nobody has broken. Read what each step *printed*,
   not only what it returned, and say which steps were skipped rather than folding them into a
   count of green ones.
3. **Rewrite each unit's `## Scope` section from its implementor's confirmation table**, through
   `protocol artifact body`, with the corrections visible rather than deleted. The implementor
   checks every `inferred` line before building on it; one returned five rows and found two of them
   wrong, and both wrong lines are still in the store today. The next wave selects on overlap by
   reading that section, so a scope the wave learned and did not write back is a wave that taught
   the store nothing and left it misleading. It is two commands.
4. Record the evidence the store requires for each story against the merge commit, then move each
   one to its terminal status through `protocol artifact move`.
5. `protocol artifact validate`, and relay its output **verbatim**.
6. **Merge the integration branch into the base branch.** This is integration, not release, and it
   is yours — the same step as merging a unit, one level up. Do not stop here to ask; the gate
   already decided, and stopping strands finished work on a branch nobody asked you to leave it on.

**Record what the wave cost, per agent: tokens, tool uses, wall duration.** The harness reports all
three when an agent completes; copy them into the closing report and the wave page. Nothing else
asks for them, so the pre-flight sizes the next wave from disk and one build while the thing that
actually ran out was the model budget. One wave spent 2,025,848 sub-agent tokens across nine runs
and merged neither unit — a number no proposal could have estimated, because no previous wave wrote
one down. Put the executed-case counts and the per-step exit statuses beside it.

**A release is not this step, and a release is not part of a wave.** A release is a tag, a version
bump and a push. It is the checklist in the repository's own agent file — `AGENTS.md` § Releases —
and following it is a separate piece of work that starts after the wave has closed. Do not enlarge
this step to cover it, and do not shrink it to let a loop cut its own tag.

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
2. **`git worktree remove <path>`, one per unit.** It takes the checkout and leaves the branch.
3. **Remove each unit's build directory and scratch root by name**, from the triple you wrote in
   the wave page when you made them. This is the step that gets missed, because it is the one `git`
   knows nothing about: neither survives in `git worktree list`, and both survive `worktree remove`
   untouched. One wave here left a **16 GB** build directory standing on a disk already at 93%,
   every worktree it belonged to long since removed, and nothing found it until somebody went
   looking for free space. Scratch is the same failure one size down — 579 MB, 95 MB, 17 MB in
   another wave, in three directories nobody had a list of.
4. **Delete every unit branch that is merged**, which after a green close is all of them. Use the
   branch prefixes [references/branch-and-merge.md](references/branch-and-merge.md) names for this
   repository, not a prefix from another one:

   ```console
   $ for b in $(git branch --list 'impl/*' --format='%(refname:short)'); do
   >   git merge-base --is-ancestor "$b" main && git branch -d "$b" || echo "NOT MERGED: $b"
   > done
   ```

   `git branch -d` refuses a branch that is not merged, which is the safety here — do not reach for
   `-D`. A merged branch holds no commit the base does not, so deleting it loses nothing, and the
   evidence in the store cites **commits**, which survive the branch by being on `main`. Leaving
   them is how a repository accumulates a list of unit branches nobody can read: after two waves
   this one had ten, every one merged, and no way to tell at a glance which wave any came from.

   A branch that is **not** merged is a unit that left the wave or a tree whose work never landed.
   Keep it, and name it in the closing report.

5. **`git worktree prune`, then `git worktree list` and `git branch --list 'impl/*'` — read both.**
   Report what they say, not that you ran the removal. A removal you did not confirm is the same
   class of claim as a process you did not watch die.

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

**A rate limit kills agents. It does not undo the commits they already made.** When HTTP 429 takes
one or more agents mid-flight, read each unit's branch head and the stage it had reached, write both
into the wave page, and resume that unit with a brief that says what is already on its branch.
**Never re-dispatch a unit whose branch has commits.** One session had 9 of 32 sub-agents killed by
a model limit and relaunched about eight identical tasks, paying twice for work that was already
committed. Resume rather than restart, and wait for the limit to clear before you do: another wave
stalled 47 minutes recovering four agents by hand, and one session idled three hours on a single
killed agent that nobody noticed was gone.

## Reference

The branch and merge conventions this skill follows, and what a wave leaves behind, are in
[references/branch-and-merge.md](references/branch-and-merge.md).

The shape of the brief each unit is dispatched with, and the repository invariants it states once so
no dispatch retypes them, are in [references/unit-brief.md](references/unit-brief.md).
