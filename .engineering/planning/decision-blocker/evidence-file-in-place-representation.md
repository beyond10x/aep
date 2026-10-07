---
format: aep.planning-md/3
id: decision-blocker:evidence-file-in-place-representation
kind: decision-blocker
status: cleared
title: Nobody has decided whether a /6 store may replace a committed evidence file in place
relations:
- blocks: story:native-evidence-privacy
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-07T08:09:45Z", actor: "human:timo", revision: 3}
---
## Question

May an opt-in `aep.project/6` Git store replace a committed evidence file in place with a
versioned representation that withholds its `change.reference`, keeping the exact original in
private custody?

## Why it is a decision

- The accepted Git-native store design says each evidence record is one immutable file, and
  `crates/edge/aep-cli/src/planning/git_record.rs` refuses any change to a committed one.
  `design:native-evidence-privacy` § 7 adds exactly one authorised transition (original →
  envelope bound to the original's digest). That changes the store's immutability contract.
- The design introduces a noun (the evidence representation and its receipt), a configuration
  (`aep.project/6`) and four commands. This repository has no ESS specification and records no
  opt-out, so where that model lives must be decided with it.
- The coordinating Atlas ADR has no number: ADR 0066 was allocated to another decision.

## Options

- A: accept the in-place representation; draft an ESS specification for the planning store's
  evidence first; build `story:native-evidence-privacy` in a later wave.
- B: accept the in-place representation; record in this repository that Rust types stay the
  model's source of truth for store formats; build in a later wave.
- C: reject it; evidence files stay immutable and adopters keep private paths out of new
  references only.

## Clears when

The decision is recorded, with the Atlas ADR number if A or B.

## Decision

Decided 2026-10-07: option C. Committed evidence never changes, in any store version; no
in-place representation is built. Adopters keep private paths out of new references. A repository
whose historical evidence carries a personal path handles it with a Gates baseline move.
`story:native-evidence-privacy` is rejected; `design:native-evidence-privacy` stays as written, a
record of the option that was not taken. https://github.com/beyond10x/aep/issues/90 is closed as
not planned.
