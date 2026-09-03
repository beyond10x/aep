---
format: aep.planning-md/1
id: story:per-record-horizons
kind: story
status: draft
title: Twelve horizons in one corpus, and one number the engine will hold
summary: 'A horizon is a property of the claim, not of the evidence kind: an adopter with 215 records across twelve horizons must declare the longest and run its own expiry gate beside the protocol''s, and the two can disagree.'
owner: protocol
tags:
- adoption
- evidence
- protocol
relations:
- decomposes: epic:adopter-feedback-round-1
- informed_by: story:evidence-horizons
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/edge/protocol-cli
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/govern/aep-engine
- confidence: cited
  path: examples/evidence-horizons-corpus/distribution.json
revision: 10
---
# Story: Twelve horizons in one corpus, and one number the engine will hold

## Outcome

An adopter whose claims rot at twelve different speeds writes each speed down once, beside the claim
it belongs to, and the protocol's answer to *has this expired?* is the same answer their own gate
gives. Today they cannot: they declare the slowest horizon their corpus contains, run a second
expiry gate of their own beside the protocol's, and the two can disagree about the same record.

## Context

Same adopter as `story:evidence-horizons`, third round, **2026-08-29**, and this story is that
story's own Open Question coming back with a corpus behind it. It closed on its stated default:

> Whether the horizon is declared on the requirement, on the evidence record, or on both with the
> stricter winning. Decides: protocol owner. Default if nobody answers: **on the requirement**,
> because the requirement is the thing the protocol controls, and a record that could set its own
> expiry is a record that can extend itself — the exact move the adopter's second trap forbids.

The default was taken and hardened into invariant 17 (`AGENTS.md:341`). Nothing below argues against
its reason. What is new is a measurement of what the default costs the corpus it was defended for.

### What the engine does today, re-verified against this checkout

* A horizon is a field on an **evidence requirement** and nowhere else —
  `crates/govern/aep-domain/src/requirement.rs:279`, with the reason in the doc comment above it at
  `:264-277`.
* Lapse is decided against that one number: `crates/govern/aep-domain/src/requirement.rs:454-471` reads
  `self.horizon`, compares it to the record's `observed_at`, and returns a reason naming the
  horizon, the observation date and the day it lapsed.
* For fact withholding the plan collapses every requirement to **one horizon per evidence kind,
  plan-wide, strictest wins** — `crates/govern/aep-engine/src/execution.rs:386-397`
  (`strictest_horizon`), read by `:442-448` (`has_lapsed`).
* The semantics past the horizon are correct and are not in question: the observation reads `?`,
  never `✗`, and the reason names the lapse date. That is invariant 5 and it stays.

So the granularity of *how fast does this rot?* is the **evidence kind**, and there is one answer per
kind for the whole plan.

### The corpus that does not fit through it

The adopter's store carries **215 `Verify:` annotations across 70 files with twelve distinct
horizons**, measured 2026-08-28 in their own tree:

| horizon | 1d | 2d | 3d | 5d | 7d | 8d | 10d | 14d | 21d | 30d | 60d | 90d |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| records | 1 | 8 | 44 | 17 | 61 | 3 | 2 | 48 | 3 | 25 | 1 | 2 |

Every one of the 215 is the same evidence kind. A live-state claim about a Kubernetes image is
re-checked every **3** days; a merged-MR claim every **14**; a chart-repo file every **90**. The
number is a property of **the claim** — how fast that particular fact rots — not of the kind of
evidence that establishes it, and not of the question being asked, because it is the same question
215 times.

Their exporter (`bin/aep-evidence.py --verify`, 215 records into one evidence document) therefore
declares the **longest** horizon, 90d, on the `claim-freshness` principle. The reasoning is sound and
is the reason this story exists: a floor the engine can never use to contradict the adopter's own
gate. The consequence is that **the engine cannot be the thing that decides "expired" for that
corpus**. Their `bin/check-verify.py` stays the real gate; its verdict enters the protocol as an
ordinary verification claim, `verification.check-verify.passed`.

That is the whole reason they could not make their gate a wrapper over `protocol evaluate`. Their own
write-up records two blocking rows, and this is the first: *"the engine holds one horizon per kind
per plan. The store holds twelve. A wrapper would have to declare twelve evidence kinds, or twelve
plans, to say what one file says today."* They also note, correctly, that this is **a disagreement
about where the number belongs, not an oversight**, and cite this repository's own corpus README
(`examples/evidence-horizons-corpus/README.md:108`) saying so.

### Why invariant 17's hazard is not what this proposes

Invariant 17 defends against one move: **extension**. *"If `extend` is as easy to call as
`re-check`, it is the one that gets called — every time, under pressure, by whoever is trying to get
a gate green"* (`crates/govern/aep-domain/src/requirement.rs:268`,
`examples/evidence-horizons-corpus/README.md:108`). A record that sets its own expiry can set it
later. That is right, and it is why *shipping a `horizon` field on a record* is not what is asked
for here.

What the corpus asks for is narrower and moves in the opposite direction. Measured: **all twelve of
the adopter's horizons are at or below the 90d they declare on the requirement.** A per-record
horizon that may only ever be **stricter** than the requirement's — the requirement stays the
ceiling, the record may bring the date forward and never push it back — covers the entire corpus, and
cannot extend anything by construction. The record can shorten its own life; it cannot lengthen it.
The Friday-evening move invariant 17 exists to make unavailable stays unavailable, because raising a
record's horizon past the requirement's is a refusal, not a slower gate.

### What exists already and is one flag short

`protocol evidence inspect --horizon` (`crates/edge/protocol-cli/src/main.rs:4818-4894`) already renders a
per-record `ok`/`expired` column, and already reports a future observation per record rather than
refusing the file. But its horizon is a single value on the command line, applied to every record —
its own help says *"report only … a what-if applied to a printed table"*. So it answers *which of
these lapsed at 90d* and can never answer *which of these lapsed at its own*, which is the question
the adopter's gate answers 215 times a run.

## Acceptance

- An evidence **record** — or the submission envelope beside `observed_at`; see *Open Questions* —
  may carry an optional `horizon`. Absent, nothing changes: every evidence document written before
  this parses and evaluates byte-identically, which is asserted rather than assumed.
- Where a record carries one, it **overrides the requirement's kind-wide horizon for that record
  only**, and it may only be stricter. A record horizon longer than the requirement's earns a named
  refusal that prints both numbers. The record is a floor on the requirement's ceiling and there is
  no way to write it the other way round.
- `protocol evaluate` honours it. Lapse is decided per record; the reason names **that record's**
  horizon and **that record's** lapse date; and the lapsed record still reads `?` and never `✗`,
  because invariant 5 does not move for this.
- The plan's fact-withholding follows the same number. `strictest_horizon`
  (`crates/govern/aep-engine/src/execution.rs:386-397`) currently answers per kind; a record carrying its
  own horizon is withheld on its own clock, so `has_lapsed` and the requirement outcome cannot
  disagree about one record — which is the property `has_lapsed`'s doc comment already says it is
  public in order to hold.
- `protocol evidence inspect --horizon` takes each record's own horizon where it has one and the
  flag's value as the fallback, and the report says per row which of the two decided it. A column
  that silently mixes the two is worse than the column that exists today.
- **Invariant 17 is amended in `AGENTS.md:341-353`, not quietly breached.** Its first enforcement
  mechanism — *"an evidence record has no horizon field, so there is nothing on a record to
  mutate"* — stops being true, and the invariant must state what replaced it: no assignment, no
  `&mut self` mutator, and the horizon re-read from a parsed document on every resolve.
  `crates/govern/aep-domain/tests/horizon_immutability.rs` extends its scan to the record type. An
  invariant whose stated enforcement no longer exists is worse than no invariant.
- The horizon corpus gains the twelve-horizon fixture.
  `examples/evidence-horizons-corpus/distribution.json` records **ten** horizons over **167** tokens,
  measured 2026-08-21. The adopter's 2026-08-28 measurement is **twelve** over **215**. Keep both,
  dated: a distribution that moved in seven days is itself the evidence that the horizon count is a
  property of a corpus and not a constant an implementation may assume.
- A test that reaches the state where the rule is load-bearing: two records of one kind, under one
  requirement, at one clock, differing **only** in their horizon — one lapsed, one not. A fixture
  where both lapse, or neither, proves nothing about the override.
- The consequence of leaving this open is written where an adopter meets it, not only here: a store
  whose claims rot at different speeds must run its own expiry gate beside the protocol's, and the
  two can disagree about the same record on the same day.

## Out of Scope

Lengthening a horizon per record, by any spelling. That is the move invariant 17 refuses and this
story does not reopen it; if the answer turns out to be *the record's number wins outright*, that is
a different story with a different argument.

Per-record horizons on anything that is not an evidence record. A requirement, an obligation and a
principle keep exactly the horizon they have.

Deciding what any adopter's horizons should be. `3d` for a running image and `90d` for a chart file
are their measurements of how fast their own facts rot; the protocol carries the number and does not
own it.

The adopter's second blocker — the coverage claim, *occurrences seen versus records produced* — which
is about who owns the definition of a markdown annotation convention, not about horizons. It stays
where they recorded it.

## Open Questions

**On the record, or on the submission envelope?** Decides: protocol owner. Default if nobody
answers: **on the envelope, beside `observed_at`**. Both are the caller's statement about the
observation rather than about its content, invariant 7 already draws the line there — *the engine
does not decide when the observation happened* — and it keeps `Evidence`'s payload types, which the
source scan in `crates/govern/aep-engine/tests/evidence_scan.rs` enumerates, untouched.

**Shorten-only, or free?** Decides: protocol owner. Default if nobody answers: **shorten-only**,
refusing a record horizon longer than the requirement's by name. It covers the adopter's whole
corpus as measured, it keeps invariant 17's hazard closed by construction rather than by review, and
it is the recoverable direction — a rule that later turns out to be too strict can be relaxed with a
changelog line, where a horizon that grew under pressure is not detectable from a single reading.

**Does a requirement that declares no horizon at all let a record's horizon decide?** Decides:
protocol owner. Default if nobody answers: **no** — a requirement with no horizon does not decay
today, and a record that could introduce decay where the reviewed document asked for none is a
record changing the meaning of a gate nobody re-read. An adopter who wants per-record decay declares
a ceiling first, which is one line and is reviewable.
