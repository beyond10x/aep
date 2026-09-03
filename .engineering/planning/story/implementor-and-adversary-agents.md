---
format: aep.planning-md/1
id: story:implementor-and-adversary-agents
kind: story
status: draft
title: The plugin ships an implementor and an adversary, and the reasons it did not are answered
summary: Two of the four states the no-implementor-or-verifier-agent gap held are now covered. The gap's first argument was about driven runs only; its second is why the adversary is not a verifier and emits no evidence.
owner: plugin
tags:
- eval
- plugin
relations:
- decomposes: epic:self-evaluation
- informed_by: story:workflow-plugin-coverage
revision: 3
---
# Story: The plugin ships an implementor and an adversary, and the reasons it did not are answered

## Outcome

An operator working interactively has an agent that writes the code test-first, and a second agent
whose job is to break what the first one declared working — with the same kind of charter the three
shipped agents carry, where the bound is the grant and the report is the evidence.

Six of the ten states of this repository's own development workflow now have a surface. Four did.

## Context

`integrations/workflow-coverage.yaml` held one gap, `no-implementor-or-verifier-agent`, over
`[establish_verifiers, implement, verify, adversarial_verify]`, and
`story:workflow-plugin-coverage` deliberately left closing it as a decision: *"Whether an
implementor agent should ship is a decision, and the gap register already holds the argument that a
verifier agent cannot honestly ship until D-3 closes."*

The gap's `reason` carried two arguments. This story takes them one at a time, because only one of
them survived.

**Argument 1 — an implementor has nothing left to learn, because `protocol drive` puts the
capability policy in force per state.** True, and true of a *driven* run. An interactive session has
no drive loop and no per-state policy: the operator is the loop. `workflow-coverage.yaml` joins
workflow states to the **instruction** surface, and that surface was empty for the two states where
the ordering rule actually lives. What `agents/implementor.md` teaches is what the `test.exists`
guard mechanises when there is a guard — the case is written, run, and *watched fail*, before the
implementation exists. A test that was never seen red has not been shown to test anything, and no
capability policy catches that.

**Argument 2 — a verifier agent would emit evidence that looks independent and is not, because
`independent: true` is checked structurally and nothing signs a record.** Correct, unchanged, and
the reason `agents/adversary.md` is **not a verifier**. It emits no evidence:

- Its mechanical output is a failing test case. The record comes from the test runner, which is a
  `Producer::Verifier` — independent absolutely, today, with no D-3. The case is independent
  because a program ran it, not because the agent says it was impartial.
- Its judgement output is a `review-result` from an agent. `Reviewer::is_human()` is false for
  `Agent` (`crates/govern/aep-domain/src/review.rs`), so it satisfies no review requirement carrying
  `human: true` — which is the one `development.critical` imposes. Nothing gates on its opinion.
- An `llm` step has no `evidence` key and cannot be given one
  (`crates/drive/aep-driver-spec/src/map.rs`): *"a step kind that could mint evidence from a model's
  output would be the single change that unpicks the whole loop."* The interactive agent holds
  itself to the same rule.

`crates/govern/aep-domain/src/requirement.rs` is untouched. Gap register **D-3** stays proposed, and the
verifier agent it blocks stays unbuilt — that one really does wait for a signature.

**Why this is not the LLM judge the repository refuses twice by name**
(`epic:self-evaluation`, `specification:agent-charter-eval-cases`). Those refusals are about asking
a model whether an agent behaved reasonably: unreproducible and unfalsifiable at once. A red suite
is reproducible on any machine on any day, which is why the adversary's table of attacks is ordered
to push every finding it can into a program and leave only the residue as prose.

**The asymmetry that makes it adversarial rather than a second opinion** was already in the tree and
is not invented here. `adp/default` orders `adversarial_verify -> implement` before
`adversarial_verify -> review`, and transitions are tried in document order, so an adversary that
succeeds at its actual job sends the work back rather than forward. The agent's win condition is a
**red** suite; the implementor's is a green one.

## Acceptance

`cargo test -p aep-cli --test workflow_coverage` passes with `implement`,
`establish_verifiers` and `adversarial_verify` claimed by a shipped agent file rather than named in
a gap, and with `verify` still named in one — coverage is total, and a state claimed twice is an
error, so the test passing is the assertion that the gap edit is honest.

## Out of Scope

- **Any change to `independent: true`.** Making it relative — *not the actor under review*, rather
  than *not an agent at all* — is a published-schema change that would let an agent's judgement
  count as evidence. It is the interesting one and it is not this story.
- **An `agent:` field on an `llm` step.** `RawLlmStep` carries `skills`, `scope`, `context` and
  `harness`; naming a role there is the driven half of this work and needs a schema change.
- **Any agent that can produce `review.approved`.** `no-review-or-completion-surface` stays a gap
  and its argument is untouched: teaching an agent to satisfy that guard is teaching it to forge the
  one record that has to come from a person.
- **Eval cases for the two new agents.** A new `conformance/eval/` directory is a new case and costs
  three files and no code, but its transcript is hand-synthesized. `story:agent-eval-cases` is still
  draft for the two agents that shipped before these, and the same story is where these belong.

## Open Questions

**Should the adversary record a `review-result` at all, or only report?** Decides: protocol owner.
Default if nobody answers: **record it**. A finding explained in a message is gone when the session
ends, the kind became authorable only when `story:review-result-cannot-be-authored` shipped, and
zero `review-result` artifacts exist across the store's 159 — the kind has never once been used for
the thing it was added for. The cost is that the record is immutable, so a wrong finding is retired
by archiving it and writing a second one.
