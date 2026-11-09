---
format: aep.planning-md/3
id: story:published-pattern-residue
kind: story
status: active
title: Three published patterns still disagree with the constructors they are published for
relations:
- decomposes: epic:zero-open-issues-2026-11-09
- serves: vision:O2
scope:
- confidence: cited
  path: crates/drive/aep-driver-spec
- confidence: cited
  path: crates/drive/aep-driver-spec/tests/published_pattern_evaluated.rs
- confidence: cited
  path: crates/govern/aep-domain
- confidence: cited
  path: crates/govern/aep-domain/src/artifact.rs
- confidence: cited
  path: crates/govern/aep-domain/src/domain_event.rs
- confidence: cited
  path: crates/govern/aep-domain/src/entity.rs
- confidence: cited
  path: crates/govern/aep-domain/src/facts.rs
- confidence: cited
  path: schemas/generated/protocol.schema.json
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:07:43Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-09T22:07:43Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"review_outcome":1}}}
---
# Story: Three published patterns still disagree with the constructors they are published for

## Outcome

`FactPattern`, `EntityType`, `DomainEventType` and `ArtifactId` publish a JSON Schema `pattern` that
accepts or refuses strings their own constructor does not, so an editor and the loader answer the
same document differently.

## Context

Found by the second adversarial pass of the wave of 2026-08-30, against `4f2dd4e`. That wave closed
`story:workflow-id-pattern-numeric-tail`, which derived every published pattern in `aep-domain` and
`aep-driver-spec` from one body per charset — fourteen identifiers and references. **These four were
measured and deliberately not fixed**, because each is a second change hiding inside that one. Every
count below is pinned by an exact assertion in
`crates/drive/aep-driver-spec/tests/published_pattern_evaluated.rs`, so a case goes red the day one moves.

**1. `crates/govern/aep-domain/src/facts.rs:488` — `FactPattern::PATTERN` puts `*` inside the character
class.** The published pattern is `^([A-Za-z0-9_*-]+)(\.[A-Za-z0-9_*-]+)*$`. `FactPattern::new`
allows `*` only as a whole segment and `**` only last. Measured: **96 divergences over the shape
corpus, 287,967 over 955,206 strings, all schema-looser** — `a*`, `*a`, `***`, `**.a` and
`tests.**.failed` are valid to the schema and refused by the constructor.

*What reaches it:* `schemas/generated/protocol.schema.json:267`, referenced at `:113` for
`observables`, live in `protocols/aep/1.yaml:121`, `protocols/adp/1.yaml:40` and
`protocols/aop/1.yaml:24`. **All 46 shipped observables agree today**, so the exposure is the next
author, not the current tree. That is what makes this a story rather than an incident.

This is the exact shape `story:workflow-id-pattern-numeric-tail` fixed for `-` in four vocabularies:
a separator inside a character class where the constructor requires it between segments.

**2. `crates/govern/aep-domain/src/entity.rs:282` and `crates/govern/aep-domain/src/domain_event.rs:213` — same
class.** `[a-z0-9.-]*` and `[a-z0-9-]+` accept a segment that is bare `-`, an empty segment and a
trailing `-`. Measured: **66 and 49 divergences, all schema-looser** — `a.-/v1`, `a..a/v1`,
`a-b.-.-/v1`.

*What reaches it:* **nothing found.** Neither type appears in any file under `schemas/generated/`,
so today the wrong rule exists only in `json_schema()`. Recorded because it is the same defect and
will be published the day either type reaches a schema.

**3. `crates/govern/aep-domain/src/artifact.rs:102` — `ArtifactId::PATTERN` refuses a leading separator its
constructor takes.** The pattern anchors **both** halves on `[A-Za-z0-9]`; `ArtifactId::new`
(`:57`) allows `-_./` anywhere in either. Measured: **601 divergences over 56,594 + 4,687 strings,
all schema-stricter** — `_a:b`, `.a:b`, and in the second half `a:.`, `a:-`, `a:_`, `a:/`.

*Two adversarial passes disagreed about this one and the third measurement settled it.* Pass 1
reported `design:auth_flow` as pattern-false; it is pattern-**true** and constructor-true, because
`_` is inside `[A-Za-z0-9._/-]`. The class is a leading separator, not an underscore. `grep` finds 0
ids containing `_` under `.engineering/planning`, so nothing in this tree reaches it.

## Acceptance

- `FactPattern::PATTERN`, `EntityType`'s and `DomainEventType`'s published patterns are composed
  from the same segment definitions their constructors check, and
  `every_published_identifier_rule_accepts_exactly_what_its_constructor_accepts_in_its_own_shape`
  (`crates/drive/aep-driver-spec/tests/published_pattern_evaluated.rs`) passes with
  `OPEN_DIVERGENCES` empty. Before this story it pins three rows: EntityType 66, DomainEventType 49,
  FactPattern 96.
- The shape corpus gains `a*`, `*a` and `a**` (the hole under *Not established*), and with them
  loosening `FactPath::PATTERN` to include `*` in its class makes that test fail; restored, it
  passes.
- `ArtifactId` is measured first, with the same test's comparison over its own shape including the
  strings `_a:b`, `.a:b`, `a:.`, `a:-`, `a:_` and `a:/`. If pattern and constructor disagree on any of
  them, the pattern is changed to accept exactly what `ArtifactId::new` accepts and a case in that
  test asserts each of the six strings is accepted by both or refused by both. If they already agree,
  that case is still added and passes before and after, and the story records that `ArtifactId` was
  already consistent.
- `cargo xtask schema` leaves `schemas/generated/` unchanged or the changed patterns are the only
  difference, and `schema-check` passes.

## Out of Scope

- **The `u32` major-version ceiling and `adp/default/01`**, already out of scope by name in
  `story:workflow-id-pattern-numeric-tail`.
- **`SubjectRef`'s per-half length residue**, pinned there and not expressible as one `maxLength`.

## Open Questions

**Does `FactPattern` need a segment alternation or a shared definition?** Decides: protocol owner.
Default if nobody answers: an alternation of the shape
`^(\*\*|\*|[A-Za-z0-9_-]+)(\.(\*|[A-Za-z0-9_-]+))*(\.\*\*)?$`, composed from one definition the way
`identifier_pattern!` composes the charsets, so this cannot drift again.

## Not established

**The instrument has a hole of its own, found and not chased.** Loosening `FactPath::PATTERN`
(`facts.rs:375`) to include `*` in its class **survives the whole suite at exit 0**: neither corpus
contains an alphanumeric glued to `*`, because `charset_corpus`'s alphabet has no `*` and
`shape_corpus` only joins segments with `.`, `:` and `/`. Closing it means adding `a*`, `*a`, `a**`
to the shape corpus — which moves the pinned counts above. Whoever takes this story should expect to
re-pin them.

## Triage 2026-11-09

Still present at `824869fec`, for three of the four. `crates/govern/aep-domain/src/facts.rs:488` still has `*` inside the character class; `crates/govern/aep-domain/src/entity.rs:282` and `crates/govern/aep-domain/src/domain_event.rs:213` keep the loose classes; `crates/drive/aep-driver-spec/tests/published_pattern_evaluated.rs:1594` pins `OPEN_DIVERGENCES` to three rows: EntityType 66, DomainEventType 49, FactPattern 96.

`ArtifactId` is no longer in that list. Its `PATTERN` (`artifact.rs:135`) still anchors both halves on an alphanumeric while `new` takes a separator anywhere; whether the corpus still reaches it was not run at triage (inferred). The implementor measures it first.

The open question's default is taken: the alternation the story gives, composed from one definition.
