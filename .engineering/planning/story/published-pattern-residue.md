---
format: aep.planning-md/1
id: story:published-pattern-residue
kind: story
status: draft
title: Three published patterns still disagree with the constructors they are published for
revision: 2
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
`crates/aep-driver-spec/tests/published_pattern_evaluated.rs`, so a case goes red the day one moves.

**1. `crates/aep-domain/src/facts.rs:488` — `FactPattern::PATTERN` puts `*` inside the character
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

**2. `crates/aep-domain/src/entity.rs:282` and `crates/aep-domain/src/domain_event.rs:213` — same
class.** `[a-z0-9.-]*` and `[a-z0-9-]+` accept a segment that is bare `-`, an empty segment and a
trailing `-`. Measured: **66 and 49 divergences, all schema-looser** — `a.-/v1`, `a..a/v1`,
`a-b.-.-/v1`.

*What reaches it:* **nothing found.** Neither type appears in any file under `schemas/generated/`,
so today the wrong rule exists only in `json_schema()`. Recorded because it is the same defect and
will be published the day either type reaches a schema.

**3. `crates/aep-domain/src/artifact.rs:102` — `ArtifactId::PATTERN` refuses a leading separator its
constructor takes.** The pattern anchors **both** halves on `[A-Za-z0-9]`; `ArtifactId::new`
(`:57`) allows `-_./` anywhere in either. Measured: **601 divergences over 56,594 + 4,687 strings,
all schema-stricter** — `_a:b`, `.a:b`, and in the second half `a:.`, `a:-`, `a:_`, `a:/`.

*Two adversarial passes disagreed about this one and the third measurement settled it.* Pass 1
reported `design:auth_flow` as pattern-false; it is pattern-**true** and constructor-true, because
`_` is inside `[A-Za-z0-9._/-]`. The class is a leading separator, not an underscore. `grep` finds 0
ids containing `_` under `.engineering/planning`, so nothing in this tree reaches it.

## Acceptance

Each of the four publishes a pattern derived from its own validator rather than paraphrasing it, and
`every_published_identifier_rule_accepts_exactly_what_its_constructor_accepts_in_its_own_shape`
holds with an empty `OPEN_DIVERGENCES`.

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
