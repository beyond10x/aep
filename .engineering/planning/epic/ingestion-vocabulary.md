---
format: aep.planning-md/1
id: epic:ingestion-vocabulary
kind: epic
status: draft
title: A pipeline that reads a company and writes a corpus, and the six things it cannot say
summary: 'Round 2 of the same adopter''s defects: an ingestion profile and workflow that do not load. A private-message denial, a source citation, a publish capability, an acquisition phase set, corpus observables and a provenance scale — three close by decision, three by code.'
owner: protocol
tags:
- adoption
- protocol
relations:
- decomposes: initiative:the-repo-governs-itself
- informed_by: epic:adopter-feedback-round-1
revision: 4
---
# Epic: A pipeline that reads a company and writes a corpus, and the six things it cannot say

## Outcome

An adopter whose product is a **record** rather than running software can write a profile and a
workflow against this specification and have both load. Today neither does, and the six reasons are
in the plan with honest names — including the three where the vocabulary was already open and the
adopter had no way to find that out.

## Why Now

Same adopter as `epic:adopter-feedback-round-1`, same day — **2026-08-21** — different exercise.
Round 1 was a review of the lifecycle and evidence model. This is what happened when they specified
their company-data ingestion pipeline as a profile plus a workflow and ran it: **neither document
loads**. In their own words, *"Both are written against the published vocabulary and both would be
rejected by `RawProtocol → Protocol` validation. That is not a drafting error — the class of work is
outside the vocabulary."* They wrote six register rows, `D-I1` … `D-I6`, each naming what closes it.
A seventh, `D-I7`, is deliberately not taken up here (see *Out of Scope*).

The rows were opened against `protocols/aep/1.yaml` and `protocols/aop/1.yaml` and are the adopter's
reading. **Every one of them was re-measured against this tree on 2026-08-28 UTC** with the repository's
own build (`target/debug/protocol`, `0.32.1`), by writing a scratch protocol document that extends
`aep/1` and declares the value each row says is missing, and running `protocol validate` on it. The
measurement moved three rows:

| row | what the adopter reported | measured 2026-08-28 UTC | closes by |
|---|---|---|---|
| D-I1 | no phases for acquisition work | an extension declaring `acquisition normalisation classification deduplication routing publication` is **valid** | decision |
| D-I2 | `network.read` cannot deny a DM | `unknown capability "private_message.read"` | code |
| D-I3 | no `source_record` evidence kind, no verifier for it | `invalid evidence kind identifier "source_record"`; the **verifier** name `source-reader` is accepted and inert | code |
| D-I4 | no `communication.publish` in the approval floor | `unknown capability "communication.publish"` | code |
| D-I5 | no observable for freshness or coverage | `ingest.**` and `corpus.**` in an extension are **valid** | decision |
| D-I6 | `scales` cannot express provenance strength | `provenance: [...]` in an extension is **valid**; the fact it would order has no producer | code, and not the code the row names |

That three of six were already expressible is not a reason to close them quietly. It is
`epic:adopter-feedback-round-1`'s own meta-defect arriving a second time from the other side:
*things the docs invite an adopter to declare keep turning out to be fixed in the engine* had a
mirror image nobody had seen — **things the engine already allows keep reading as fixed**. The
adopter checked their documents against `aep/1` plus `aop/1` and read the lists there as the lists.
`protocols/` does hold two documents that extend the base — `adp/1` and `aop/1` — and
`protocols/README.md` says extension is additive; what no page in the adopter's path says is that a
*third* one, theirs, is a document they may write. The nearest sentence is
`docs/guide/adopting.md:186`, *"A state machine your work actually has — different states, different
phases | No. Own a tree"*, which reads as a refusal rather than as the route it is. Each of those
three stories therefore carries a decision with a default and a documentation fix, not a build.

## Scope

One story per row, `D-I1` … `D-I6`, each carrying its own evidence and its own measurement. Three
are protocol-document decisions (`story:acquisition-phase-set`, `story:corpus-observables`,
`story:provenance-scale`'s first half) and three are changes to a closed vocabulary in `crates/`
(`story:private-message-denial`, `story:source-record-evidence`,
`story:communication-publish-capability`).

The two closed vocabularies these rows land on both have a stated guarantee in
[`docs/guide/open-vocabulary.md`](../../../docs/guide/open-vocabulary.md), and no story here asks to
open either. **Capability value names** are closed so that *a capability name resolves to the same
authorisation decision in every harness*; **evidence kind names** are closed so that *a requirement
for one cannot be satisfied by a record that means something else*. Both guarantees are the reason
these arrive as stories in this store rather than as keys the adopter adds to their own tree — which
is exactly what that page tells an adopter to do, and it is the right route.

## Out of Scope

**`D-I7`, an artifact kind for a corpus entry.** Measured `valid` on 2026-08-28 UTC — `artifact_kinds:`
is open at both layers, and `knowledge-entry` plus a lifecycle under `artifacts/lifecycles/` is an
own-a-tree change that needs nothing from here. It is recorded in the adopter's register and there
is no story for it; if the shared kind turns out to be wanted, it is a row this epic did not take.

Adopting the adopter's own protocol, or vendoring any of their documents into this tree. Their tree
is evidence, not a dependency — the same boundary `epic:adopter-feedback-round-1` drew.

Anything about *how* an ingestion pipeline should be built. Six of these are vocabulary and the
seventh is a sentence in a guide. No workflow, principle or profile for ingestion is proposed here.

## Risks

**One adopter, again**, and this time the sample is narrower still: one pipeline, in one company, in
one domain. The mitigation is the same as round 1's and the counts carry it — 5,503 provenance-classed
records behind `D-I6`, a categorical DM rule behind `D-I2`, an irreversible outward act behind
`D-I4`. The three decision rows are the ones to be most suspicious of: *publish it upstream* is the
answer that costs this repository a vocabulary entry forever, and *own a tree* is already documented
and already works.

**Two of the three code rows widen a vocabulary that is closed on purpose.** Each such story owes a
statement of what the new name means in **every** harness, not only in a corpus pipeline, or the
guarantee the closure buys is spent on one adopter's shape.

## Done When

Each of the six rows has either landed as code, or has a recorded decision saying it will not — and
an ingestion profile and workflow of the shape the adopter wrote **load**, checked against a
document tree in this repository's own examples rather than against theirs.
