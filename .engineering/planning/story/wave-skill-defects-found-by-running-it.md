---
format: aep.planning-md/1
id: story:wave-skill-defects-found-by-running-it
kind: story
status: draft
title: Seven defects the wave skill found by being run once
summary: 'Every one found by running it, not reading it: a charter that forked a store, a routing that skipped the re-attack the workflow requires, and a rule that counts the wrong thing.'
owner: plugin
tags:
- plugin
- wave
relations:
- informed_by: story:wave-as-a-surface
- decomposes: epic:self-evaluation
revision: 7
---
# Story: Seven defects the wave skill found by being run once

## Outcome

The `wave` skill's charters and routing say what the first real wave proved they
must, so the second wave does not re-discover the same seven things.

## Context

The wave of 2026-08-30 — `story:board-columns-come-from-the-ladders` and
`story:usage-series-assertions` — closed green on `124cac7`, 192 suites, 2,917 tests.
Every defect below was found by running the skill, not by reading it.

**1. The adversary charter contradicts the wave rule, and it forked a store.**
`adversary.md` says record judgement findings as a `review-result`; the wave skill says
only the coordinator writes the store. One adversary declined and said why; the other
complied and wrote into its worktree's planning store. Measured: worktree journal 564
lines against the main tree's 568 — forked, and a merge would have produced the
forged-revision failure the rule exists to prevent. **Fix: the adversary returns
findings; the coordinator records them.**

**2. The skill routes green straight to merge; `adp/default` does not.** The workflow
runs `implement -> verify -> adversarial_verify`, so a correction re-enters attack. The
skill's table says `green -> merge`. Both units were sent for a second pass against the
spec rather than the skill, and **both went red again** — 3 findings and 5 findings on
code that had just passed its first adversarial pass. Shipping on the skill as written
would have shipped eight defects. **Fix: the skill's routing follows the workflow's.**

**3. The correction rule counts red rounds, not repeated failures.** It says red-twice
means a fresh implementor. Both units fixed every round-1 case correctly and then failed
on *new* ground found by a *new* attack — which is not the anchoring the rule was written
to catch. It fired twice for the wrong reason and was defensible both times, which is
worse than firing wrongly. **Fix: count a case that fails again after being fixed.**

**4. Nothing says when attacking stops.** The skill caps corrections at three and is
silent on attacks, so the loop runs until someone pays for it to stop. The workflow's own
answer is `adversarial_verify -> review`, a person. **Fix: say so.**

**5. The implementor charter has no `cargo fmt --check`.** It gates on `cargo test -p`
and `cargo clippy -p`. Twenty lines of unformatted source reached the integration branch
and the full gate was the first thing to see it. **Fix: one line in the charter.**

**6. A `## Scope` section can be confidently wrong, and it costs a round.**
`story:usage-series-assertions`'s scope named `event_stream.rs` a blind reader. It is
not. The implementor built on that and every series verdict on every driven run cited
nothing — `events=[]` on `ok` and `gap` alike, against `report.rs`'s *"never empty for a
fact read off an event"*. Only the adversary caught it. The scoper's `cited`/`inferred`
marking did its job — the line was marked inferred — but nothing downstream treats an
inferred line as a thing to check first. **Fix: an implementor verifies the inferred
lines of its scope before building on them.**

**7. An adversary can write two mutually unsatisfiable cases, and nothing notices.**
On `usage-series`, one case demanded `[0 x7, total]` not be `ok` under `non_decreasing`
while another demanded `[3, 3, 35]` be `ok`. Both have identical pair shapes — no
wrong-way pair, one that moved, the rest still — and differ only in *how many* stood
still. No pairwise rule separates them; only a count does, which the story's `## Out of
Scope` refuses. It took an operator decision to resolve, and the implementor was right to
stop rather than satisfy both. **There is no fix that prevents this**, and that is the
finding: a suite of adversarial cases is a specification, and two of them can disagree.
The routing has to send that to a person rather than to a third correction round.

**And one that is not about the skill.** Round 1's case
`one_ladders_column_order_does_not_depend_on_another_kind_being_in_the_store` passes for
a correct *and* an incorrect implementation, because its two ladders' union happens to be
acyclic. A green test that discriminates nothing, found by the second adversary — which
is an argument for the re-attack edge on its own.

**8. Nothing takes the worktrees or their build directories down, and `git` cannot
find the build directories for you.** The skill ends at the closing commit. `git worktree
list` in the next wave's pre-flight is a **refusal**, not a cleanup, so a wave that leaves
its trees standing pays nothing and the wave *after* it pays everything. Worse, a build
directory placed outside the worktree — which the same skill requires, so two trees never
share one — survives `git worktree remove` untouched and is invisible to every `git`
command there is. Measured on this machine on 2026-08-30: **14 GB** still standing at
`~/.cache/claude-tmp/claude-1000/-home-operator-projects-engineering-protocols/ba00d8e0-.../scratchpad/eval/ws_eval`,
last written **2026-08-24**, keyed to `~/projects/engineering-protocols` — a path that still
exists and holds `.git/` and `.claude/` and no working tree, so nothing in the checkout
being worked on names it. Root filesystem at **91 %**, 75 G free of 848 G. **Fix: a closing step that reads
the untracked records out, removes each worktree, removes each build directory by the name
it was written down under, and reports what `git worktree list` says afterwards — never
`--force`, because a tree that refuses to go is holding uncommitted work and that is a
finding for the operator.**

**9. The skill puts the build directory outside the worktree; this repository's
`AGENTS.md` says inside, and the suite enforces it.** `references/branch-and-merge.md`
describes a build directory as *"one per worktree, usually outside it"*, and the coordinator
of the 2026-08-30 wave followed that — four trees, four `…/<unit>-target` directories beside
them. `AGENTS.md:499-502` says the opposite in as many words: *"`store_selection.rs` asserts
`CARGO_TARGET_TMPDIR` lies under the repository root and fails eleven tests whenever the
target is elsewhere. Each worktree builds into its own `target/`."* It does. Measured: `cargo
test -p protocol-cli` in `impl/protocol-drive-verb` exited **101** with **11 failures, all of
them `crates/edge/protocol-cli/tests/store_selection.rs:77` — `the scratch tree is under the
repository: StripPrefixError(())`** — and none of them touching the unit's change. Because
cargo stops at the first failing target, the two suites the unit actually owed
(`drive_cli`, `driving`) **never ran**, so the gate was not merely noisy, it was empty. One
implementor spotted the collision from its own tree and reported it as a note; the
coordinator had already made the mistake four times.

**Fix: the skill defers to the repository.** The invariant it is really carrying is *no two
worktrees share a build directory* — where the directory sits is the adopted repository's
call, and `AGENTS.md` is where that call is written down. The skill must say so and must not
name a default location. The wave's own pre-flight should read the repository's agent file for
a build-directory rule before it creates the first tree; the cost of not doing so is a gate
that reports a red nobody caused, on a suite that never ran.

**And the reason both halves of this matter at once.** Defect 8 says a build directory outside
the worktree survives `git worktree remove` and is found by the disk filling up. Defect 9 says
outside is where this repository's tests refuse to work. Inside the worktree answers both: it
is removed with the tree, and `CARGO_TARGET_TMPDIR` lands under the repository root. That is
the recommendation — **inside, always, unless an adopting repository says otherwise** — and it
reverses the skill's current wording rather than qualifying it.

## Landed — 2026-08-30

Fixes 1-6, 8 and 9 are in the plugin. Fix 7 has none by construction, and its routing answer is in
the skill.

| fix | where |
|---|---|
| 1. the adversary returns findings; the coordinator records them | `agents/adversary.md` § *Returning the judgement findings*, and every `review-result` instruction removed |
| 2. green does not route to merge | `skills/wave/SKILL.md` § *Route the result* — a correction re-enters `adversarial_verify` |
| 3. count a case that fails again after being fixed, not red rounds | same table, rewritten rows |
| 4. attacking stops after two passes | same section |
| 5. the implementor gates on the formatter and the linter | `agents/implementor.md` step 6 |
| 6. an implementor checks the inferred lines of its scope first | `agents/implementor.md` § *Read before you write*, item 3 |
| 7. two unsatisfiable cases | no fix exists; the routing sends the pair to a person |
| 8. a wave takes its worktrees and build directories down | `SKILL.md` § *Take the worktrees down* |
| 9. the build directory goes where the repository says | `SKILL.md` pre-flight row, and `references/branch-and-merge.md` |

**Four of these were overridden by hand eight times each before they landed.** The wave of
2026-08-30 dispatched eight adversaries, and every one carried a paragraph telling it to ignore its
own charter's instruction to write a `review-result`. The correction rule was departed from twice
with a written justification each time. The budget was applied from this story's Open Question
default rather than from any rule. That is the finding worth keeping: **a defect recorded in a story
and not carried into the plugin is a defect that gets re-litigated in every prompt**, and the cost
is paid per dispatch, silently, by whoever writes the next one.

## Three more the second wave found

**10. Only `green` merges, and a coordinator will invent a fifth row.** Two units finished their
final correction round red — correct adversarial cases, for real defects nobody was fixing that day
— and the coordinator merged them anyway rather than letting them leave the wave, reasoning that
deleting the cases would hide the defects. They were already filed as stories. The cost was not
three known failures: `cargo test` without `--no-fail-fast` stops at the first failing target, so
the deliberate red **deleted every result after it** and a gate run on `main` reported one failure
in `aep-driver-spec` and nothing at all about `protocol-cli`. Fixed: `SKILL.md` now gives the three
things a correct-but-unfixed case can be, and none of them is *merge it red*.

**11. A whole-gate run does not survive, and one exit status is not a result.** Five of eight
attempts to run `task check` were killed mid-flight by SIGTERM at different steps. `exit status 143`
is a signal, not a verdict. Per-step exit capture turned an unreadable run into a full result.
Recorded with two things a step's code cannot tell you: `postgres-check` **skipped itself and
exited 0**, and `website` failed `127 command not found` because a fresh worktree has no
`node_modules` — the gate cannot pass in the tree the skill tells you to run it in.

**12. A deviation reported is not reported again.** The same open decision was re-listed in fifteen
consecutive reports. The skill already said to report deviations and stay quiet otherwise; it did
not say that a deviation, once reported, has been reported. Repeating it does not raise its
priority — it teaches the reader that the reports are things they have already seen, which is how
the one new line in the fifteenth gets missed.

## Acceptance

`integrations/claude-code/skills/wave/SKILL.md` and `agents/{implementor,adversary}.md`
carry fixes 1-6, 8 and 9, and the routing sends an unsatisfiable case set to review rather
than to a correction round. Fix 8 lands in three places: a *Take the worktrees down* step
in the skill, the two rows it adds to `references/branch-and-merge.md`'s survives-the-wave
table, and the rule in both agent charters that a worktree and a build directory are never
theirs to remove — plus a sixth report part naming every path each agent wrote outside its
worktree, because the coordinator can only clean up what it was told about.

## Out of Scope

- **The eighth gap, already fixed in flight.** An agent file is not dispatchable until
  the plugin registry reloads, so the session that authors one cannot dispatch it without
  `/reload-plugins`. Recorded here because it is the reason `story:agent-eval-cases` tests
  charters against committed transcripts rather than live runs.
- Deleting the non-discriminating case. It belongs to whoever owns `board`.

## Open Questions

**Does the adversary get a budget?** Decides: protocol owner. Default if nobody answers:
**two passes, then review** — which is what this wave did by judgement rather than by
rule. Two passes found 4 then 3 findings on one unit and 4 then 5 on the other, so the
second pass was not diminishing; a third might not be either, and that is the argument
for a person deciding rather than a number.

## Five more the third wave found — 2026-08-30, `wave/what-the-last-wave-found`

Four units, two merged, two left the wave. Every defect below was found by running the skill.

**13. Every unit went red on attack 1, and three of four on attack 2. Plan for five agent runs
per unit, not two.** Measured: **19 agent runs against a planned 8** — 4 implementors, 4 first
attacks, 4 corrections, 4 second attacks, 2 final corrections, 1 rate-limit casualty. Wall clock
3 h 11 min from the opening `chore(store)` to the gate; **5 h 03 min of agent time** across the 15
runs with recorded durations, 2.46 M sub-agent tokens. The finding rate did not fall between passes
on any unit — attack 2 on the guard unit measured **188 of 214 list entries deletable with the whole
suite green**, on code that had already survived one attack and one correction. **The skill's
estimate of a wave's cost is the thing to fix, not the number of passes.**

**14. The routing table's "send it back to the same implementor" assumes an agent list that
outlives the implementor. It did not.** By the time attack 1 returned, all four implementor agents
were gone from the harness's agent list; only adversaries survived. Both first corrections went to
**fresh** implementors carrying the findings and the previous diff — which is the row the table
reserves for an agent whose fix did not hold. The cost is re-reading the tree, not re-deciding it,
so it is small; the rule as written is simply not always available. **Fix: say what to do when the
implementor is gone, rather than leaving the coordinator to notice.**

**15. A proof-of-finding case converted to a regression test can land on a tautology, and only
mutation finds it.** Two of pass 1's cases on `story:prose-that-the-tree-contradicts` asserted the
negation of a true property — their own doc comments said *"the finding is that they are equal"* and
*"Red as written."* The coordinator applied the *wrong now* row and authorised converting them to
assert what was decided. One conversion produced `assert_eq!(walk(moved), read)` over a fixture with
no `drivers/` documents: list-concatenation identity, which cannot fail. Attack 2 proved it by
reversing the loader's sort at `load.rs:146` and watching the case stay green. **Fix: the *wrong
now* row requires a mutation check on the rewritten case — break what it claims to protect and watch
it redden — before the unit is called green.**

**16. An implementor edited two adversary cases it had been told to leave, and was right to.** On
`story:workflow-id-pattern-numeric-tail`, two length-bound cases read `PATTERN` alone. No correct
implementation can satisfy them: a regular expression cannot bound length without counted
repetition, which the file's own interpreter refuses to evaluate, and the correct fix puts the bound
in `maxLength`, which the cases could not see. The implementor rewrote them to read the published
*rule* and reported it. The coordinator verified the rewrite asserted no less — `single.is_empty()`
for every non-`SubjectRef` rule, the two residues pinned by exact value, failing if the list grows
*or* shrinks — and accepted it. **The instruction was right and the outcome was right, which means
the instruction was incomplete: an implementor told to stop needs a way to say *this case is
unsatisfiable and here is the measurement*, and get an answer inside the same round.**

**17. An adversary wrote to the planning store, and the ban has to name the command, not the
directory.** The charter says the adversary never writes the store; the dispatch said *nothing under
`.engineering/planning/`*. The pass-2 adversary on the guard unit ran `protocol artifact new`
**without `--store`** while checking whether any verb changes a title — it defaulted to the
worktree's real store, wrote `story/title-check.md` and appended a `journal.jsonl` line. It
disclosed this itself and reverted both by hand. The coordinator verified rather than accepted:
**623 = 623 journal lines against `HEAD`, `git status` empty, no `title-check` string, `protocol
artifact validate` → `valid`.** No forgery reached the tree. **Fix: the charter and every dispatch
say *never run `protocol artifact` at all*, not *do not write under the planning store* — the agent
obeyed the second and broke the first.** Every later dispatch in this wave carried the stronger
wording.

**And the number that keeps growing.** Defect 8 measured 14 GB of orphaned build directories.
This wave's pre-flight measured 9.0 G of another repository's, and an adversary measured
`~/.cache/claude-tmp` at **64 GB across 444 `protocol-drive-*` fixture trees** left by past runs,
with the root filesystem at **96%, 38 G free**. None of it is this repository's and nothing prunes
it. The cleanup step this story added covers a wave's own trees; **fixture trees written to
`TMPDIR` by test harnesses are a second unswept surface nobody owns.**
