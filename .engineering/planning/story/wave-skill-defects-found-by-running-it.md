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
revision: 1
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

## Acceptance

`integrations/claude-code/skills/wave/SKILL.md` and `agents/{implementor,adversary}.md`
carry fixes 1-6, and the routing sends an unsatisfiable case set to review rather than to
a correction round.

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
