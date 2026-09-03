---
format: aep.planning-md/1
id: story:design-principles-concrete-failure-and-code-example
kind: story
status: implemented
title: Show how vague principles doom an agent—and how AEP enforces them
summary: Turn the Design principles page into one concrete failure-and-recovery walkthrough backed by AEP types, commands, and evidence.
tags:
- adopter-experience
- design-principles
- documentation
- hands-on
refs:
- provider: public-docs
  reference: https://beyond10x.github.io/docs/aep/concepts/design-principles/
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: website/docs/concepts/design-principles.md
revision: 5
---
## Problem

The public [Design principles](https://beyond10x.github.io/docs/aep/concepts/design-principles/) page states ten correct rules, but mostly as compact assertions. A first-time adopter cannot see one realistic engineering task fail without those rules or trace each rule to the AEP types, commands, evidence, and refusal that enforce it. The result reads as philosophy rather than a protocol someone can use.

## Requested improvement

Add one coherent, hands-on “without AEP / with AEP” walkthrough. Start with a plausible agent change—for example, adding an authorization check to an API—where an unconstrained agent:

- calls prose such as “tests pass” completion evidence;
- collapses missing evidence into success or failure;
- silently regains a denied capability;
- applies an approval to a later revision;
- performs a partial write before reporting an error; and
- produces output whose meaning or bytes cannot be replayed.

Show why this is predictably doomed rather than merely a low-quality prompt.

Replay the same change through AEP with realistic code, YAML, and CLI/output excerpts. The example should connect each existing section to a concrete protocol mechanism: raw-to-validated domain construction, a principle or profile, revision-bound artifacts and approvals, typed evidence, three-valued predicate evaluation, layered capability resolution, commands and legal transitions, deterministic engine input, atomic provider behavior, reference-driver inputs, ESS/plugin report boundaries, and byte-level schema or compatibility checks.

Use current public AEP syntax and commands. Prefer snippets exercised by tests or adapted from repository fixtures; label any deliberately illustrative pseudo-code as such. Explain what AEP itself guarantees and what remains the responsibility of an external producer, driver, provider, or credential boundary.

## Acceptance criteria

- The page contains one end-to-end scenario with clearly paired “without AEP” and “with AEP” paths.
- The failure path demonstrates concrete consequences, not only claims that principles are important.
- The AEP path includes input plus evaluated output or refusal, so a reader can see what the engine decided and why.
- Every current principle section points to a specific step, line, or callout in the scenario.
- The example distinguishes `Unknown`, contradiction, and satisfaction and shows how each changes the next action.
- A stale approval, denied capability, and partial-write attempt each produce a technically accurate AEP outcome.
- The walkthrough names the responsible implementation boundary: domain type, validator, engine, command service, backend/provider, reference driver, or external report adapter.
- Runnable snippets are held by an existing or new test/fixture; non-runnable snippets are explicitly marked illustrative.
- The final explanation answers in plain language: why clear principles are necessary, how AEP makes them executable, and what AEP does not prove.
- Existing concise principle summaries remain useful as a reference after the walkthrough is added.

## Source

Operator documentation review on 2026-09-03. Public route:
`/docs/aep/concepts/design-principles/`. Repository source:
`website/docs/concepts/design-principles.md`.
