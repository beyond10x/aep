---
format: aep.planning-md/1
id: story:attested-approver
kind: story
status: draft
title: An operator step can be answered by an independent agent, not only by a person
relations:
- decomposes: epic:reference-driver
revision: 3
---
# Story: An operator step can be answered by an independent agent, not only by a person

## Outcome

A governed run that has done everything correctly does not stop for the night because the only actor
allowed to say *yes* is asleep. The approval still comes from somebody who did not do the work, and
the record says who — which is the property the rule was always about.

## Context

**The driver stops at an `operator` step because it cannot attest an approver, not because approval
needs a human.** The refusal says so in its own words: *"there is no flag that answers one: nothing
below the driver checks who granted an approval, so the refusal has to be the driver's."* Read that
again — the reason is an **absence of attestation**, and the fix chosen for it was to defer to a
person, because a person at a keyboard is an attestation of a kind.

Run `W4-3/1` (2026-08-29) reached `establish_verifiers` and stopped exactly there, `awaiting-operator`,
exit 0, lock released. It had written a specification of 17 single-claim requirements, run its own
suite red, and recorded the red result. Everything it was asked to do, it had done.

What happened next is the evidence for this story. The specification was reviewed — its four cited
`file:line` references checked against the tree, all four exact — and approved by the **orchestrating
agent**, which is not the agent that wrote it. That is independence in substance: a different actor,
different context, no stake in the artefact. `protocol artifact move` accepted it because it accepts
any caller, which is the same gap from the other side.

So the current position is incoherent in one direction: the store will accept an approval from the
**author** without complaint, and the driver will refuse to continue without a **human**, and neither
check is the one that matters.

**What matters is the producer.** The evidence model already has the vocabulary:
`Evidence::Approval` carries a `Producer`, `independent: true` means *not the implementing agent*,
and `Producer::Human` is one producer among several rather than the only legitimate one. A driven
run's approval could carry the approver's identity the same way a `test_result` carries the verifier
that produced it.

## Acceptance

- An `operator` step can be satisfied by a **named non-human approver**, and the run's record says
  which one. A run that continues past an approval carries, in its snapshot, who granted it.
- The approver is **refused when it is the actor that produced the thing being approved**. This is
  the rule the step is for; a run that approved its own specification would satisfy a principle by
  writing to the document the principle is about.
- `--pause-on-approval` keeps its present meaning: stop and wait. The new route is opt-in and named
  on the command line, so a run that stops for a person is still the default.
- The refusal, when there is no admissible approver, says what would be one — it currently names a
  person because that was the only answer, and it must name the others once they exist.
- A person approving is unchanged, and no existing run's record is reinterpreted.

## Out of Scope

- **Attestation by signature.** Gap-register **D-3** stays proposed and nothing here assumes it. This
  story records *who* approved as a claim in the run's own record, which is exactly as strong as the
  rest of the evidence model and no stronger.
- Approving anything other than what an `operator` step asks for.
- Removing the human route.

## Open Questions

**Is an agent's approval `Producer::Agent`, or does it need a class of its own?** Decides: protocol
owner. Default if nobody answers: **a class of its own** — `Producer::Agent` is the implementing
agent, and the whole claim here is that the approver is not that. Reusing it would make the
independence check unable to see the difference it exists to see.

**Does the independence check compare identities, or roles?** Decides: protocol owner. Default:
**identities** — two sessions of the same model are two actors, and a rule that said otherwise would
refuse the one arrangement that makes this useful.

## Implementation notes (2026-08-29)

**The symptom the story was written for was wrong, and the cause was right.** Read back before
implementing: the driver does not stop for the night. `--pause-on-approval` is a one-shot stop —
the pause advances the cursor past the `operator` step (`crates/aep-driver/src/run.rs`, *the pause
is this step's completion*) and a resume walks on. `NATIVE-1/1` moved
`establish_verifiers -> implement` holding **zero** `approval` records; the guard it met,
`spec-driven.before_implementation`, is an artifact requirement (`kind: specification`,
`status: approved`, no relation) and its worktree holds two approved specifications of other
stories. So the defect is *a governed run proceeds past an approval with nothing recorded, and says
nothing about it* — and the story's own Context still names the cause: an absence of attestation.

**What shipped.** `protocol drive run --pause-on-approval --approver agent:<name>` (and the same
on `resume`, remembered from the launch). The flag answers nothing: the run stops as before, the
named actor records a granted `approval` against the run's snapshot while it is stopped, and the
resume reads what arrived (`DriverCursor::owed` remembers how much record existed at the pause).
A person's approval is admissible unnamed; the named agent's is admissible unless it is the run's
own actor — the task, the execution, the harness its `llm` steps run under — and an agent nobody
named is refused naming the flag. The cursor records who answered (`DriverCursor::answers`;
`protocol drive status` prints `answered …`). Rule and refusals: `crates/aep-driver/src/attest.rs`.
Tests: `crates/aep-driver/tests/attested.rs`.

**The asymmetry, stated.** With an approver named, a resume that finds no admissible approval stops
again and says who would be admissible. With none named, a resume that finds nothing walks on as it
always did — a person who moved the artifact the prompt names and resumed is that route, and the
guard on the way out is what decides — and the report now carries one line saying the record holds
nobody's answer. That line is the contrapositive the correction asked for: a run that continued past
an approval with no admissible approver is visible as such. An approval that arrived and is
**not** admissible — the run's own actor, an unnamed agent, a denial — stops the run again on both
routes, because walking past a self-approval is the case the step exists to prevent.

**Decisions taken, and their defaults.**

| question | taken | default in the story |
|---|---|---|
| identities or roles | **identities**: `agent:orchestrator` and `agent:T-1.1` are two actors | identities — taken |
| is an agent's approval `Producer::Agent`, or a class of its own | **left open — `Producer::Agent` reused for now.** The check compares identities, the driver stamps no `Producer::Agent` of its own today, so reuse loses nothing the check can see. A class of its own is a `Producer` variant: a published-schema change (`cargo xtask schema`), a `CHANGELOG` line, adopter-visible. Decides: protocol owner | a class of its own — **not** taken, named here instead |
| what is *the run's own actor* | the task id, the execution id (`<task>.<n>`, refused as a family at launch), and each `llm` step's harness name; `Producer::Harness` always | — |

**What this does not do, named.** The approver is whatever `producer` the record carries: a
declared identity, as strong as the rest of the evidence model (D-3 stays proposed). And the
store's own provenance cannot yet tell an agent's `protocol artifact move` from a person's:
`command_actor()` (`crates/protocol-cli/src/planning.rs`) stamps `human:<USER>` on every move,
which is the *accepts any caller* gap this story's Context names from the other side. Closing it —
an actor the driver hands each session, read by the CLI over `USER` — is a story of its own.
