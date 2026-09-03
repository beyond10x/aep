---
format: aep.planning-md/1
id: story:provenance-scale
kind: story
status: draft
title: 5,503 records ordered by how they were known, and a predicate that can only ever read Unknown
summary: 'A provenance scale is declarable today and worth nothing: no evidence kind projects a text-valued fact at a path the adopter names, so claim.provenance >= measured is permanently unobservable.'
owner: protocol
tags:
- adoption
- evidence
- protocol
relations:
- decomposes: epic:ingestion-vocabulary
- depends_on: story:corpus-observables
- depends_on: story:source-record-evidence
scope:
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: docs/guide/open-vocabulary.md
- confidence: inferred
  path: protocols/aep/1.yaml
revision: 9
---
# Story: 5,503 records ordered by how they were known, and a predicate that can only ever read Unknown

## Outcome

*"Do not brief an inferred claim as measured"* is a rule an engine decides, not a habit a corpus's
maintainers keep. A predicate reading `claim.provenance >= measured` orders the classes correctly
**and** has something that can produce the fact.

## Context

Adopter register row **`D-I6`**, 2026-08-21, and the row with the largest corpus behind it. Their
words:

> `scales` declares `risk`, `severity`, `health`. A corpus needs an ordered provenance scale so that
> `claim.provenance >= measured` has a defined meaning. This is not speculative: the store being
> specified here already classifies **5,503 records** across exactly such a scale — observed 3,080 ·
> measured 882 · reported 753 · inferred 384 · refuted 270 · hypothesis 134 — and the ordering is what
> makes "do not brief an inferred claim as measured" checkable. The protocol has the right mechanism
> and not this instance of it.

Their closing condition: *"a `provenance` scale. One line, and it makes an existing invariant
expressible."*

**Measured 2026-08-28 UTC, `target/debug/protocol` 0.32.1.** The one line already works. A scratch
extension of `aep/1` declaring `scales: { provenance: [hypothesis, inferred, reported, measured,
observed] }` validates:

```console
$ protocol validate --root <scratch>
2 file(s): 1 protocol(s), 0 principle(s), 0 workflow(s), 0 profile(s), 0 lifecycle(s), 0 step map(s)
valid
```

`docs/guide/open-vocabulary.md:160` records *A protocol document's `scales:` block — open*, and
`Protocol::merge` unions a base's scales into an extension's
(`crates/govern/aep-domain/src/protocol.rs:159`). So the scale is one line and the adopter may write it
today, in their own tree, without asking anybody.

**And it would buy them nothing, which is the story.** A scale orders a fact; something has to
produce the fact. Reading every arm of `EvidenceKind::facts`
(`crates/govern/aep-domain/src/evidence.rs:1573-1908`): every **text**-valued fact is projected at a path the
**engine** fixes — `tests.<suite>.result`, `static_analysis.result`, `service.health`,
`approval.<id>.decision`, `verification.<claim>.status` — and the one arm that writes to a path the
**caller** names is `MetricObservation` (`evidence.rs:1699-1709`), whose value is
`FactValue::Number` by type (`MetricObservation { metric: FactPath, value: Number }`,
`evidence.rs:400-404`). There is no way for an adopter to establish a non-numeric fact at a path of
their own.

So `claim.provenance >= measured` parses, declares legally, orders correctly under the new scale, and
evaluates **Unknown for ever** — which under three-valued evaluation is `?`, never `✗`, so it never
refuses anything and never reads as failing either. A rule that cannot fire is worse than a missing
one: it looks like a control in the document and is not one anywhere else.

That is a sharper gap than the row states, it is **code**, and it is not the code the row names. The
row asked for a scale, which is free. What is owed is a producer for the class the scale orders.

**Why it should not be solved by encoding the class as a number.** `claim.provenance == 4` is
decidable today and is the workaround anybody hitting this will reach for. It defeats the whole point
of `scales:` — the reason ordering lives in a declared scale rather than in string comparison is that
`gte` on `risk: medium` must mean the same thing to every reader, and a magic number means nothing to
any of them. If the answer is *numbers are enough*, that should be a recorded decision, not something
adopters discover and each encode differently.

## Acceptance

- A `provenance` scale exists, or a recorded decision says a scale is an own-a-tree matter — the same
  decision `story:acquisition-phase-set` and `story:corpus-observables` carry, and it should be
  answered once for all three.
- **The load-bearing half:** an adopter can establish a text-valued fact at a path they name, and
  `<their.fact> >= <scale member>` is decided rather than Unknown. The natural home is
  `story:source-record-evidence` — a `source_record` says where a claim came from, and *how it was
  known* is the same record's business — which is why this story depends on it. If that is the answer,
  this story is the scale plus the fact the record projects; if it is not, this story owes the
  alternative producer.
- The refusal a wrong value earns names the scale and its members, the way an undeclared status names
  its ladder. A provenance class not in the scale is a typo, and typos are what a declared scale is
  for.
- A test reaches the state where the ordering is load-bearing: two records differing only in their
  class, one satisfying `>= measured` and one not, decided against the same predicate. A fixture where
  both pass proves nothing about the ordering.
- `docs/guide/open-vocabulary.md`'s scales row is unchanged, and — if the producer half lands — the
  page gains the row it currently cannot have: *what may produce a fact at a path an adopter names*.
  That is a value space fixed in the engine with no document key to find, which is exactly the class
  the page's own § *What the derivation cannot find* says a script never discovers.

## Out of Scope

Deciding what any provenance class means. `observed`, `measured`, `reported`, `inferred`, `refuted`,
`hypothesis` are the adopter's taxonomy; the protocol orders what a document declares and does not own
the words.

Any rule about briefing, publishing or reporting. *"Do not brief an inferred claim as measured"* is a
principle somebody writes over this fact, and it belongs in their tree.

Widening `FactValue`. Text-valued facts already exist throughout; what is missing is a **producer**
that writes one at a caller-named path, and that may well be one field on one evidence kind.

## Open Questions

**What is the ordering?** The row gives a census, not a ladder — *"observed 3,080 · measured 882 ·
reported 753 · inferred 384 · refuted 270 · hypothesis 134"* is descending by count, and `refuted`
sits between `reported` and `inferred` in it. `refuted` is plainly not a rung *below* `inferred` and
*above* `hypothesis` on a strength scale; it is a different axis — a claim that was checked and found
false is not weak evidence, it is a negative result. A scale that puts it in the middle would make
`>= measured` quietly wrong. Unclear — ask the adopter.

**Does `refuted` belong in this scale at all?** Decides: protocol owner, on the adopter's answer.
Default if nobody answers: **five rungs, `hypothesis < inferred < reported < measured < observed`,
and `refuted` left out** — an ordered scale with a non-ordinal member in it is a scale whose `gte`
answers are wrong in a way nobody will notice, and leaving a class out is recoverable where a bad
ordering is not.
