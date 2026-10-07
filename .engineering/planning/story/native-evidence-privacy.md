---
format: aep.planning-md/3
id: story:native-evidence-privacy
kind: story
status: rejected
title: Withhold a private evidence reference without losing the record
refs:
- provider: github
  reference: beyond10x/aep#90
relations:
- serves: vision:O2
- informed_by: design:native-evidence-privacy
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning/conformance_count.rs
- confidence: cited
  path: crates/edge/aep-cli/src/planning/git_record.rs
- confidence: cited
  path: crates/edge/aep-cli/src/store_command/migrate_git.rs
- confidence: cited
  path: crates/govern/aep-domain/src/project.rs
- confidence: cited
  path: crates/plan/aep-backend-markdown/src/journal.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:07:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "rejected", at: "2026-10-07T08:09:46Z", actor: "human:timo", revision: 5}
---
## Outcome

An adopter whose committed or migrated evidence records carry a private absolute path in
`change.reference` can withhold that reference through a governed `aep` operation. The exact
original is kept privately, the public evidence occurrence keeps its slot and is counted once, and
no artifact, transition or review body changes.

## Acceptance

1. An explicit, opt-in next project format supports a versioned evidence representation.
   `aep.project/5` behaviour and `aep.planning-md/3` stay unchanged. Old-reader routes still in use
   refuse the prepared store before mutation (explicit-store, alias and linked-worktree paths), or
   are upgraded or excluded before any opt-in.
2. One operation withholds only a complete, present, nonempty `change.reference` of one evidence
   occurrence: it retains the exact original bytes privately, then atomically replaces the same
   public slot with its representation and receipt. No new evidence observation, status move,
   artifact revision or completion claim is made.
3. A complete, valid, uncommitted `aep.project/1` → `/5` migration is supported. Selector
   preparation and each record operation have explicit crash and retry semantics and stale-input
   checks. Private archive containment, permission and symlink checks run before any write.
4. Every history, count, validation and mutation reader recognises the representation. An invalid
   representation is refused, never skipped. Identity, multiplicity, order, every non-reference
   field and lifecycle evidence eligibility stay unchanged.
5. Raw edits of immutable records stay refused. Tests cover altered protected fields, malformed,
   unknown and duplicate fields, old-reader refusal, retry and collision, archive corruption,
   incomplete writes, and diagnostics that print no private value. Review immutability is intact.
6. A synthetic adopter store with two private-path references passes the unchanged privacy gate
   after correction, with logical history and counts preserved. No hook bypass, hand edit, history
   rewrite or publication of a private original.
7. The failing regressions come first; the repository gate passes after independent review.

## Origin

https://github.com/beyond10x/aep/issues/90: an `aep.project/5` integration could not be committed
because immutable review-result bodies and evidence references held personal absolute paths, and
no governed correction exists. Review prose correction is out of scope here.

## Design

`design:native-evidence-privacy` holds the 2026-10-03 contract (one-slot envelope
`aep.evidence-representation/1`, rule `withhold-evidence-reference/1`, opt-in `aep.project/6`).
`review-result:native-evidence-privacy-design-1` holds its independent review: one documentation
warning, no contract blocker.

## Blocked on

`decision-blocker:evidence-file-in-place-representation`. The design adds one authorised
transition to the rule that a committed evidence file never changes (§ 7 of the design;
`crates/edge/aep-cli/src/planning/git_record.rs` refuses any change today). That is a change to the
store's immutability contract and needs an architecture decision recorded in Atlas before this
story is built. The design also introduces a noun (the evidence representation and its receipt), a
configuration (`aep.project/6`) and four commands; this repository has no ESS specification, so
where that model lives is part of the same decision.

## Scope

Not assessed for a wave: no implementation is scheduled until the blocker is cleared. The design's
§ 1 lists the source surfaces at 0.68.0.

## Decision

Decided 2026-10-07: option C. Committed evidence never changes, in any store version; no
in-place representation is built. Adopters keep private paths out of new references. A repository
whose historical evidence carries a personal path handles it with a Gates baseline move.
`story:native-evidence-privacy` is rejected; `design:native-evidence-privacy` stays as written, a
record of the option that was not taken. https://github.com/beyond10x/aep/issues/90 is closed as
not planned.
