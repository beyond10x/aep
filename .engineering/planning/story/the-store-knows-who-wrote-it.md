---
format: aep.planning-md/1
id: story:the-store-knows-who-wrote-it
kind: story
status: draft
title: The planning store records who made a write, so a driven move is not the operator's
summary: AEP_ACTOR declares the actor a store write is journalled with; the driver sets it to agent:<execution id>, and the llm session half is blocked on metaharness admitting one variable into its constructed child environment.
owner: protocol
tags:
- driver
- provenance
relations:
- decomposes: epic:reference-driver
- informed_by: story:attested-approver
scope:
- confidence: cited
  path: crates/drive/aep-driver
- confidence: cited
  path: crates/edge/aep-cli
- confidence: inferred
  path: crates/plan/aep-backend-markdown
revision: 8
---
# Story: The planning store records who made a write, so a driven move is not the operator's

## Outcome

`protocol artifact history story:x` tells a reader which moves a person made and which a driven run
made. A run that recorded its own approval is visible as such in the store, not only in the run
directory — so the independence rule `story:attested-approver` enforces at an `operator` step is a
rule somebody can check afterwards, from the record, rather than a rule that held only while the
driver was watching.

## Context

**`command_actor()` stamped `human:<$USER>` on every write, whoever made it.** Every
`protocol artifact new`, `move`, `body`, `relate` and `evidence` went into the journal under the
name of whoever was logged in on the machine, because `$USER` was the only thing the CLI read. A
driven `llm` session that ran `protocol artifact move <spec> approved` from inside a run was
journalled as the operator's own move. That is the *accepts any caller* gap
`story:attested-approver` § Context names from the store's side, and its own implementation notes
end by naming it as a story of its own.

**What shipped, 2026-08-29.** `AEP_ACTOR` — named beside `AEP_DRIVE_PLUGIN_DIR` — declares the
actor, parsed with `ActorRef::parse`, so `human:`, `agent:`, `service:` and `system` are all
sayable. A value that does not parse is refused naming the variable and the value, never defaulted
to `$USER`: falling back there would attribute an agent's write to a person, which is the defect
rather than a recovery from it. Unset, nothing changes. The driver sets it to
`agent:<execution id>` on every process it starts for a step, and that value comes from
`aep_driver::attest::session_actor` — the same function `own_actors` uses — so the actor a run
writes under and the actor `admit` refuses an approval from are one string and not two.

**What it does not reach, measured rather than assumed.** An `llm` step's model session. Both arms
spawn through `metaharness run <harness>`, and metaharness constructs its child's environment
rather than inheriting one: `env_clear()` and a fixed allowlist —
`INHERITED_KEYS`, seven names, in the Claude adapter's launch, and `PATH` plus a credential in the
b10x adapter's — with no flag on `RunSpec` that admits another variable. So the driver's
declaration reaches metaharness and stops there, and a `protocol artifact move` the model itself
runs is still journalled as `human:$USER`. A `command` step is this process's own child and does
inherit it, which is why the mechanism is live rather than dead code today.

## Acceptance

- An `llm` step's session runs `protocol artifact move` and the journal entry names the run, not
  the operator. `protocol artifact history` shows the two apart on one artifact.
- The route the value travels is **declared, not inherited**: metaharness admits the variable by
  name on both its `claude` and its `b10x` adapters, the way it already admits `RUSTUP_HOME`, so
  nothing else in the driver's environment leaks into a hermetic child. That is a change in
  `metaharness` and this repository does not make it — the arrow between the two runs one way and
  a dependency never crosses it.
- A session that was launched without the variable writes as it does today rather than failing:
  the fallback stays `human:$USER`, because a driven run that cannot write to the store at all is
  worse than one whose writes are attributed to a person.
- The audit reads the same on both sides: an approval recorded in the store by `agent:<execution>`
  is refused by the driver as a self-approval and is *findable* as one afterwards by reading the
  journal alone.

## Out of Scope

- **Attestation by signature.** Gap register **D-3** stays proposed. This story records a
  *declaration* of who wrote something, exactly as strong as the rest of the provenance model and
  no stronger; a store that cannot verify an identity must not carry a field that looks verified.
- Any change to `metaharness` made from here. Vocabulary crosses to that repository and a
  dependency never does, so the flag is asked for there and consumed here.
- Attributing a write to a *step* rather than to a run. The execution is the identity the
  independence rule already compares, and a finer one would be a second vocabulary for the same
  question.

## Open Questions

**Does an unrecognised `AEP_ACTOR` refuse the command, or refuse only the write?** Decides:
protocol owner. Default if nobody answers: **refuse the command**, which is what shipped — a read
that succeeded under a malformed declaration would teach a caller that the variable is advisory.

**Should the driver refuse to start when the session cannot carry the actor?** Decides: driver
owner. Default: **no, and say so once** — a run that will not journal its own writes is still a run
worth having, and refusing here would make an unrelated harness limitation block every driven run.
