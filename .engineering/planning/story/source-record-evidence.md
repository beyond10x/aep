---
format: aep.planning-md/1
id: story:source-record-evidence
kind: story
status: draft
title: Where this sentence came from is not an evidence kind
summary: A corpus's atomic evidence is a citation — this claim, this permalink, this instant. evidence_kinds has no source_record, and an externally named verifier can establish nothing.
owner: protocol
tags:
- adoption
- evidence
- protocol
relations:
- decomposes: epic:ingestion-vocabulary
revision: 3
---
# Story: Where this sentence came from is not an evidence kind

## Outcome

A corpus can **require** that every claim carry its source, and the requirement is decided by a
record rather than by an agent's word. Six months later, *"was this measured or inferred, and from
what"* is answerable without re-running anything.

## Context

Adopter register row **`D-I3`**, 2026-08-21. Their words:

> `provenance-tracking` requires `kind: verification, independent: true`. That answers "did a verifier
> establish this" but not "where did this sentence come from". A corpus's atomic evidence is a
> citation: this claim came from this permalink at this timestamp. `evidence_kinds` has no
> `source_record`; `verifiers` has no `source-reader`. And because a required evidence kind with no
> verifier is a validation error, citation evidence cannot even be *required* today.

Their closing condition, with the part that matters for the review of it:

> `source_record` in `evidence_kinds`, `source-reader` in `verifiers`. Note this weakens nothing: a
> source-reader attests retrieval, not truth.

**Measured 2026-08-28 UTC, `target/debug/protocol` 0.32.1**, two probes against a scratch extension of
`aep/1`. The two halves of the row do not behave the same way.

```console
$ protocol validate --root <scratch>     # evidence_kinds: [source_record]
1 problem(s):
  - protocol document (…/aip/1.yaml): evidence_kinds: invalid evidence kind identifier
    "source_record": expected one of test_result, static_analysis, contract_result,
    property_test_result, deployment_result, metric_observation, health_observation, approval, diff,
    artifact, review, verification, specification, ess_conformance, trace_conformance
    at line 5 column 17
```

```console
$ protocol validate --root <scratch>     # verifiers: [source-reader]
2 file(s): 1 protocol(s), …
valid
```

**The verifier half is already open, and it is inert.** `source-reader` parses as
`Verifier::ExternalTool(ToolRef)` (`crates/aep-domain/src/verification.rs:67`), which is why it
validates. But `Protocol::can_establish` asks `EvidenceKind::default_verifiers()`
(`crates/aep-domain/src/evidence.rs:1325-1342`), and that table returns a **named** verifier for every
kind and never an external tool. A declared `source-reader` can therefore establish nothing, for any
kind, today. The row's sentence *"a required evidence kind with no verifier is a validation error"* is
`ValidationCode::NoVerifierForEvidence`, and it is the wall behind the wall: even with the kind added,
the pair only works if `SourceRecord`'s `default_verifiers()` row names something a document can
declare.

This is the second of the two deliberately closed rows in
[`docs/guide/open-vocabulary.md:168`](../../../docs/guide/open-vocabulary.md) — *Evidence kind names
the engine accepts*, closed at `crates/aep-domain/src/evidence.rs:1189`, guarantee: *an evidence kind
carries fixed semantics and a fixed set of verifiers that may establish it, so a requirement for one
cannot be satisfied by a record that means something else.* That page and `AGENTS.md` both name
`evidence_kinds` as closed **correctly**, and this story does not argue with that. It argues that the
set is short one kind, and the adopter's own note is the strongest argument for adding it rather than
loosening anything: **a source-reader attests retrieval, not truth.** A citation is the one fact a
corpus is built to carry and the one this vocabulary cannot name.

Note what the row is careful *not* to ask for, and keep it that way: their register's *"What is
deliberately not proposed"* section refuses weakening `provenance-tracking`'s `independent: true`,
*"because in ingestion the agent that read the source is usually the one reporting it. Resist: that is
precisely the property that makes a corpus trustworthy, and D-I3's `source-reader` is the honest way
to satisfy it."*

## Acceptance

- `source_record` is an `EvidenceKind`, with a documented shape that carries at minimum the subject
  it is about, the retrieval locator, and the instant it was retrieved. What it does **not** carry is
  a truth claim, and the doc comment says so in those words — the kind attests that a sentence was
  read at a place at a time.
- `default_verifiers()` names a verifier a document can declare for it, and the choice is argued:
  either an external tool is admissible here (which changes `can_establish` for every kind and needs
  saying), or a new named `Verifier` variant exists. A kind whose only verifier is unnameable is the
  defect this story found, not one to reproduce.
- A requirement for `source_record` can be **declared and refused**: a profile requiring it, with no
  record presented, refuses by name; with a record presented, passes. The fixture reaches the state
  where the rule is load-bearing — a corpus claim with a citation and one without, in the same run.
- The facts it projects are named and observable, so a completion condition can read
  *"every new claim carries a source"* rather than asserting it. This is the seam with
  `story:corpus-observables`; whichever lands second states the join rather than re-deciding it.
- `evidence_kinds` is still closed and `docs/guide/open-vocabulary.md` still says so, with the same
  guarantee and the count updated. Widening a closed set without touching the page that calls it
  closed is how that page stops being true.
- `CHANGELOG.md` carries a line written for the person hitting it: what a `source_record` record
  establishes and what it does not.

## Out of Scope

Reading anybody's source. This repository holds no credential and reaches no network; the
source-reader is a class of verifier, and the thing that fetches a permalink lives where
`infra-scout` lives.

Judging the source. `source_record` says *this came from there*, never *therefore it is true*, and no
rule in this story reads it as the second. Independence stays where `story:evidence-producers-for-the-driven-map`
and gap-register **D-S4** left it.

A retention or expiry model for a citation. That is the artifact-kind row (`D-I7`) and this epic does
not take it.

## Open Questions

**Does `source_record` satisfy `provenance-tracking`, or sit beside it?** The principle requires
`verification` with `independent: true`. A citation is a different question from an independent
verification, and satisfying the principle with one would quietly weaken it — the exact thing the
adopter refused. Decides: protocol owner. Default if nobody answers: **beside it, never instead of
it** — `provenance-tracking` is unchanged, and a corpus profile requires both.

**Is the locator a URI, or free text?** A permalink is a URI in every case the adopter names, and a
typed locator is checkable where free text is not. Decides: protocol owner. Default: **free text with
a documented convention**, because a corpus whose sources include a database row and a log line has
locators no URI scheme covers, and a field that refuses the real cases gets filled with a fiction.
