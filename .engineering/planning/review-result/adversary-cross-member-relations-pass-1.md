---
format: aep.planning-md/3
id: review-result:adversary-cross-member-relations-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: cross-member relations are writable'
relations:
- reviews: story:cross-member-relations-are-writable
revision: 1
---
# Adversary pass 1: cross-member relations are writable

Attacked commit 1128b524f. Verdict NEEDS-CHANGE; 10 cases added, 3 red.

- blocker — a crossing that closes a new loop is admitted when another loop of the same relation
  kind already exists. `Assembly::cycles` keeps only the first loop per kind (`assembly.rs:267`), so
  the before/after comparison sees nothing new. — `a_crossing_closing_a_new_cycle_is_refused_while_an_older_cross_member_cycle_stands`
  — `left: Some(0) right: Some(1)`, stdout `story:c depends_on beta/story:d (revision 2)`.
- warning — a local `relate` that closes a loop through another member is admitted
  (alpha c→a→beta/b→c). Only crossings go through the workspace check. —
  `a_local_edge_that_closes_a_cycle_through_another_member_is_refused` — `left: Some(0) right: Some(1)`.
- warning — in a SQLite store a crossing moves the revision and a local edge does not. Only the
  Markdown projection skips the bump for crossing-only updates (`projection.rs:304`). —
  `in_sqlite_a_crossing_moves_the_revision_exactly_as_a_local_edge_does` — `left: Number(2) right: Number(1)`.

Probes that held: mixed relate/unrelate across 3 members and two relation kinds to one target; a
pinned `@1` crossing; own-member long spelling normalised; every refusal leaves the document
byte-identical (Markdown); relating the same crossing twice gives one edge; `set` after
relate/unrelate neither drops nor resurrects a crossing; SQLite `new` with one local edge and two
crossings holds each once across two reopens.

```findings
- file: crates/plan/aep-backend-markdown/src/assembly.rs
  line: 267
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: cycles() reports one cycle per kind, so a crossing closing a second cycle is admitted while an older cross-member cycle stands
- file: crates/edge/aep-cli/src/planning.rs
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: a local relate closing a cycle through another member skips the workspace cycle check and is admitted
- file: crates/plan/aep-backend-markdown/src/projection.rs
  line: 304
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: in a SQLite store a crossing bumps the document revision while a local edge does not, contrary to the unit's own Markdown parity claim
```
