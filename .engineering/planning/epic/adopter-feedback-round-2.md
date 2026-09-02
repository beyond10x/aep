---
format: aep.planning-md/1
id: epic:adopter-feedback-round-2
kind: epic
status: implemented
title: The second adopter, and the install path that broke under them
summary: 'Round 2 of adopter defects: the 0.14.0 install instruction points at a repository with no marketplace, templates have no home for a classified ambiguity, and nothing says whether a checkout is ready to be planned in.'
relations:
- decomposes: initiative:the-repo-governs-itself
- informed_by: epic:adopter-feedback-round-1
revision: 4
---
# Epic: The second adopter, and the install path that broke under them

## Outcome

Somebody who adopts the protocol from a release announcement can install it, check their setup with
one command, and get artifact templates that hold an undecided question as a classified ambiguity
instead of prose. The parts of that which are Rust or templates live here; the plugin parts live in
the agent-plugins repository's store.

## Why Now

On 2026-09-02 a second adopter reported in the 0.14.0 announcement thread that they set the protocol
up, could not get a consistent plan for a feature introducing a new entity, and fell back to a
third-party plugin. The 0.14.0 install instruction names this repository's former name as a
marketplace source; since the split on 2026-09-01 this repository carries no marketplace file. The
adopter also said they were not sure they had used the tools properly — which a preflight would have
answered. An independent review of the third-party plugin against this stack ranked the fixes; the
stories here are the Rust and template rows of that ranking.

## Scope

`README.md` and the website's install text; the epic, story and specification templates under
`artifacts/templates/`; a new `aep doctor` verb in `protocol-cli`.

## Out of Scope

Anything in `agentplugins`; session hooks; wave derivation as a verb (recorded as an unscheduled
story).

## Risks

A `doctor` that reaches the network or the clock breaks invariant 14 and 4; it must read only the
tree, the binary's own version and `git` state. A template change is a change to bytes an adopter's
tree may verify — check whether any conformance fixture pins the template text.

## Done When

The three scheduled stories are implemented, `task check` passes, a release carries them, and the
agent-plugins golden path cites the `doctor` verb.
