---
format: aep.planning-md/1
id: story:corpus-observables
kind: story
status: draft
title: Freshness is what an ingestion pipeline exists to provide, and it cannot appear in a completion condition
summary: 'ingest.** and corpus.** as observable families, plus the documentation fix: the numeric half is already reachable through metric_observation and the adopter did not find it.'
owner: protocol
tags:
- adoption
- protocol
relations:
- decomposes: epic:ingestion-vocabulary
revision: 4
---
# Story: Freshness is what an ingestion pipeline exists to provide, and it cannot appear in a completion condition

## Outcome

Somebody whose workflow ends in *"the corpus is no more than an hour stale and every claim carries a
source"* writes that as a completion condition and it is decided by records — and an adopter reading
the published protocols finds the families rather than concluding there is no way to say it.

## Context

Adopter register row **`D-I5`**, 2026-08-21. Their words:

> Completion conditions for ingestion are staleness and coverage statements: `ingest.lag_seconds < N`,
> `buckets.pending == 0`, `citations.missing == 0`, `identifiers.retained == 0`. `observables` has
> `task.**`, `tests.**`, `deployment.**`, `service.**` and the rest — nothing for a corpus. Freshness is
> the single property an ingestion pipeline exists to provide, and it cannot appear in a completion
> condition.

Their closing condition: *"`ingest.**` and `corpus.**` observable families."*

**Measured 2026-08-28 UTC, `target/debug/protocol` 0.32.1.** A scratch extension of `aep/1` declaring
`observables: ['ingest.**', 'corpus.**']` validates:

```console
$ protocol validate --root <scratch>
2 file(s): 1 protocol(s), 0 principle(s), 0 workflow(s), 0 profile(s), 0 lifecycle(s), 0 step map(s)
valid
```

`docs/guide/open-vocabulary.md:159` already records *A protocol document's `observables:` block —
open*, and `Protocol::merge` unions a base's families into an extension's
(`crates/aep-domain/src/protocol.rs:154-158`). So the row's conclusion is wrong on the mechanism and
right on the experience: an adopter with only `aep/1` and `aop/1` in front of them sees a fixed list
that has `service.**` and nothing for a corpus, and there is no sentence anywhere in their path
telling them a third protocol document is theirs to write.

**And the facts have a producer, which is the part nobody would guess.** All four conditions the row
names are numbers, and `metric_observation` projects a number at a path the **caller** chooses:
`EvidenceKind::facts` pushes both `metric.<path>` and the bare `<path>` itself
(`crates/aep-domain/src/evidence.rs:1699-1709`), over `MetricObservation { metric: FactPath, value:
Number }` (`evidence.rs:400-404`). Declare the family, submit a `metric_observation` whose metric is
`ingest.lag_seconds`, and `ingest.lag_seconds < 3600` is decided. It is the only arm of `facts()`
that writes to a caller-named path, and no guide says so — every worked example writes
`metric.<something>`.

So this row closes by **decision plus documentation**, not by code, and the decision is the same one
`story:acquisition-phase-set` carries: publish the families upstream, or say that a fact family is an
own-a-tree matter and make the route findable. The argument for publishing is weaker here than for
phases — no upstream principle times an obligation against an observable — and the argument for
documenting is stronger: a producer that exists and is invisible is worse than one that does not
exist, because the adopter spends the effort concluding it is impossible.

**One residual is real and is code, and it belongs to `story:provenance-scale`**: this only works for
numbers. A text-valued fact at a caller-named path has no producer at all, which is why that story
depends on this one and why *"every claim carries a source"* is expressible as a **count**
(`citations.missing == 0`) and not as a class.

## Acceptance

- A decision is recorded, either way: `ingest.**` and `corpus.**` are published upstream, or they are
  not and the reason is that a fact family is an own-a-tree matter.
- `docs/guide/adopting.md` says that `metric_observation` projects its metric at the path the record
  names, not only under `metric.`, with a worked two-line example that a completion condition over an
  adopter's own family reads. Today the guide's own unobservable-fact diagnostic (`adopting.md:449-451`)
  prints the declared families as a wall of names, which reads as a fixed list.
- A document tree under `examples/` declares a family of its own by extension, a workflow's
  completion condition reads a fact in it, and a `metric_observation` decides it — so the route is
  proved by `protocol validate` and `protocol evaluate` on every run rather than asserted in a guide.
- If published: each family's members are named and what produces each one is named beside it. A
  family whose facts nothing can produce is a completion condition that reads `?` for ever — which is
  what this adopter reports from the other direction, every condition in their tree printing `?`
  because nothing feeds it.
- `docs/guide/open-vocabulary.md`'s observables row is unchanged — it was already `open` and correct.

## Out of Scope

Any staleness **policy**. The adopter's register is explicit that they do not want one — *"A stale
corpus should block the work that reads it, not warn"*, and `default_failure_policy: block` is already
right. Nothing here special-cases failure handling for a corpus.

A redaction evidence kind. Their profile marks `identifiers.retained == 0` unrepresentable on two
counts, the observable and *"no `redaction_result` evidence kind, so the invariant that no data-subject
identifier is retained cannot be evidenced, only asserted"*. The observable half is this story; the
evidence half is not, and it is not `story:source-record-evidence` either. If it is wanted, it is a row
this epic did not take.

Making `protocol evaluate` produce these numbers. This repository reaches no network and runs no
pipeline; the numbers arrive as submitted evidence.

## Open Questions

**Two families or one?** `ingest.**` is about the run, `corpus.**` is about the store — lag versus
coverage — and the adopter names both. Decides: protocol owner. Default if nobody answers: **do not
publish either**, record the decision, and ship the `adopting.md` fix alone, on the same reasoning as
`story:acquisition-phase-set`: the documentation half helps every adopter, and an unpublished family
costs nothing to publish later.

**Which members?** The row names four conditions across the two families and the register does not
enumerate the families' members. Unclear — ask the adopter.
