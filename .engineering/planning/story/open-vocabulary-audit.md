---
format: aep.planning-md/1
id: story:open-vocabulary-audit
kind: story
status: implemented
title: Every adopter-facing declaration, checked for whether it is actually open
summary: 'The audit the meta-defect asks for: for each thing the docs invite an adopter to declare, is the vocabulary open, and is the closure deliberate and stated.'
owner: protocol
tags:
- adoption
- protocol
relations:
- decomposes: epic:adopter-feedback-round-1
- informed_by: story:outbound-claims-and-status-vocabulary
- informed_by: story:adopter-bugs
- informed_by: story:entity-runtime-mapping
revision: 8
---
# Story: Every adopter-facing declaration, checked for whether it is actually open

## Outcome

For each thing the documentation invites an adopter to declare, the answer to *"can I put my own value
here?"* is written down — and where the answer is no, the closure is deliberate, stated, and has a
reason a reader can argue with.

## Context

An early adopter's review, round 1 — **item B**, the meta-defect: **things the
docs invite an adopter to declare keep turning out to be fixed in the engine.** Three instances in one
afternoon, all found by *writing* a tree rather than by reading the guide — `ArtifactStatus` closed
(B1, now `story:outbound-claims-and-status-vocabulary`), `PROJECT_DIRECTORY` a compile-time constant
(B2, riding `story:adopter-bugs`), and A2's kind ladder defined over built-in variants only. Their
closing line is the request this story answers: *for every adopter-facing declaration, check the
vocabulary is actually open.*

The report also says what constrains the audit, and it is the more useful half: phases, verifiers,
artifact kinds, capabilities and observables **were** open, and a domain with no compiler and no
deployment slotted in without touching the engine. `evidence_kinds` being closed is **correct** — it
is the seam whose semantics are guaranteed, and their knowledge work mapped onto the existing kinds
honestly. So the audit's output is not "open everything". It is a table with three columns: the
declaration, whether it is open, and — where closed — the guaranteed semantics that closure buys.

## Acceptance

- Every adopter-facing declaration surface in the published guides appears in one table with an
  open/closed verdict and, for the closed ones, the guarantee the closure buys.
- Each closed entry names where its reason is written down for adopters, not only in this table.
- Every closed entry found to have **no** guarantee behind it gets a story or a recorded decision that
  it stays closed; a closed vocabulary with no stated reason does not survive the audit unremarked.
- The audit is repeatable: it says how it was produced, so the next round is a diff and not a rewrite.

## Second round, and the suite that decides it — 2026-08-28

The audit table existed from the W4-2 driven run; this round re-ran it against the tree and settled
the two rows the `ova-*` stories name. **18 rows before and after — 14 open, 4 closed — and the two
closed vocabularies that had no stated reason now have one.**

| what the round found | |
|---|---|
| stale citations | **7** — five into `vocabulary.md` had drifted 14 lines, two into `artifact.rs` 12; one was pointing at the word `Clone,` |
| corpus | grew by `website/docs/concepts/lifecycles.md`, 33 → 34 files |
| relation names | closed, deliberately — reason and the 13 kinds now in `docs/guide/adopting.md#relation-names`, with how to ask for a new one |
| predicate operators | closed, same class of reason — the full set `eq ne lt lte gt gte any_of none_of exists truthy` in `docs/guide/adopting.md#predicate-operators` |

**The acceptance is a suite, and it is green.** `bash .engineering/checks/run.sh` → **119 pass, 0
fail, 13 of 13 units**, each unit a `task:ova-*` in this store, all now `implemented` on that run.
Two of its rows had been red for reasons that were not about the audit at all, and both were fixed
rather than excused:

- **H2 was reading the store with whatever `protocol` was on `PATH`** — here `0.28.0` against a
  `0.31.0` store — and reported five stories as drifted from their journal that had not drifted. It
  now uses this tree's own build and refuses when the versions disagree, naming both.
- **H4 was asserting a file that left the repository** on 2026-08-22 with
  `epic:metaharness-migration`. It now asserts what is still assertable here: that this suite has
  not grown its own copy of the migrated model.

**Two candidate rows were considered and refused**, with the reason in the audit's own method
section: *fact spellings* (closed and already argued, but four types project facts, so there is no
single declaration to attach a verdict to) and *evidence producer* (what an adopter writes there is
a verifier class, which the open `verifiers:` row already covers).

**B2 — the project directory name — gets no row**, and the reason is recorded rather than left as a
gap: it is already fixed in `main` (`AEP_PROJECT_DIR`, `crates/aep-engine/tests/project_directory_env.rs:39`),
and the table's own checks make a row mechanically impossible for it — an open row's `crates/` line
must be a `Variant(String)`, which a `const` is not.

Still advisory, and named here rather than discovered later: the `source`/`target` pairings in
`artifacts/relations/relations.yaml` are read by nothing in `crates/`.

## Out of Scope

Opening any specific vocabulary. Each one that should open is its own story with its own migration
question; this story produces the verdict and the list.

## Open Questions

Whether the table lives in the guide or in `docs/reviews/`. Decides: protocol owner. Default if nobody
answers: **the guide**, because the audience is an adopter deciding what they may declare, and a
review page is where this repository talks to itself.
