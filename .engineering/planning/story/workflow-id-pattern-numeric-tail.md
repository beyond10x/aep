---
format: aep.planning-md/1
id: story:workflow-id-pattern-numeric-tail
kind: story
status: implemented
title: A published pattern that still calls three step maps valid
relations:
- decomposes: epic:reference-driver
- informed_by: story:driver-spec-crate
- serves: vision:O3
revision: 5
---
# Story: A published pattern that still calls three step maps valid

## Outcome

An author whose editor accepts a `workflow:` pin finds the loader accepts it too — for every
spelling, not for most of them.

## Context

`story:driver-spec-crate` closed the large half of this on 2026-08-30: `PinnedWorkflowRef::PATTERN`
is now *composed from* `WorkflowId::PATTERN` (`crates/aep-domain/src/ids.rs:191`) rather than
hand-written beside it, so the two cannot drift again and the schema is regenerated from the
composition. That removed a second rule that had already drifted once.

A residue survives, and it is upstream of that fix. **`WorkflowId::PATTERN` is itself looser than
`WorkflowId::new`.** The constructor refuses an id whose last `.`- or `/`-separated component is a
bare integer; the pattern does not say so. So `adp/2/1`, `adp.2/1` and `adp/22/1` still pass the
published schema and still fail to load — exactly the thing
`story:driver-spec-crate`'s third acceptance line forbids.

This was measured, not reasoned: the fix proposed by the wave coordinator would not have closed it
either, and the implementor said so rather than shipping a pattern that looked stricter.

## Acceptance

- `WorkflowId::PATTERN` refuses a trailing bare-integer component, so the published schema and
  `WorkflowId::new` agree on every input.
- `ProfileId` is fixed in the same change if it shares the rule — the defect is that one identifier
  has two definitions, and fixing one of two leaves the defect.
- `crates/aep-driver-spec/tests/pin_pattern_agrees_with_the_loader.rs`'s residue test,
  `the_only_pins_the_schema_still_calls_valid_that_the_loader_refuses_are_the_numeric_tail`, is the
  guard that must go red when this lands, and it is deleted or inverted in the same change.
- `schemas/generated/driver-steps.schema.json` is regenerated, and `cargo xtask schema --check`
  is green.

## Out of Scope

The `u32` major-version ceiling. `adp/default/4294967296` also matches the pattern and does not load
(`crates/aep-domain/src/version.rs:43`), and a JSON Schema `pattern` cannot express an integer
ceiling. It is recorded in the residue test's doc comment so nobody rediscovers it as a bug.

## Why This Is Not Where It Was Found

`crates/aep-domain` was not on the wave unit's surface, and copying a stricter body into `pin.rs`
would have restored the duplicated-rule-that-drifts the same change had just removed. The residue is
pinned by a test instead, so closing it here fails there and regenerates the schema with it.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/aep-domain` — cited, the story and the guard test both say the rule belongs there (*"the fix is one line in `aep-domain`"*, `crates/aep-driver-spec/src/pin.rs:69`)
- **Files:** `crates/aep-domain/src/ids.rs:227` (`ProfileId` pattern), `:235` (`WorkflowId` pattern) — cited. **The story's citation has drifted**: it names `ids.rs:191`, which is inside the macro's `schema_name()`; the shared `PATTERN` const is `:138`
- **Files:** `crates/aep-driver-spec/tests/published_pattern_evaluated.rs:473` (the residue guard) and `:536` (`the_numeric_tail_rule_is_expressible_as_a_pattern`, which already carries the replacement body, corpus-checked) — cited
- **Files:** `crates/aep-driver-spec/src/pin.rs:65-73` — cited. `PinnedWorkflowRef::PATTERN` is a hand-written literal at `:73`, **not** composed, so it does not follow `aep-domain` automatically
- **Files:** `schemas/generated/workflow.schema.json`, `profile.schema.json`, `event.schema.json`, `driver-steps.schema.json` — cited, all four carry the `WorkflowId`/`ProfileId` definitions the change edits. The acceptance names only `driver-steps`
- **Symbols:** `WorkflowId::PATTERN`, `ProfileId::PATTERN`, `validate` (`ids.rs:97-108`, the numeric-tail rule the pattern must mirror), `PinnedWorkflowRef::PATTERN` — cited
- **Also likely:** `crates/aep-driver-spec/src/map.rs:81` — inferred; `StepMapId` is a fourth copy of the same body with its own numeric-tail check at `:113-122`, and the acceptance's *one identifier, two definitions* argument reaches it
- **Also likely:** `crates/aep-domain/src/version.rs:296`, `:305` — inferred; `WorkflowRef` and `ProfileVersionedRef` publish looser hyphen-in-class paraphrases with no numeric-tail rule
- **Also likely:** `crates/aep-domain/src/ids.rs:287` (`ToolRef`) — inferred, same `Charset::Dotted` and a byte-identical pattern, so the same gap
- **Gate:** `cargo xtask schema --check` — cited (`xtask/src/main.rs:1655`)
- **Documents:** none — the whole acceptance is code plus generated schema
- **Confidence:** **high** — the story, `pin.rs`'s doc comment and the guard test all name the defect site, and the replacement pattern body is already written and corpus-checked at `published_pattern_evaluated.rs:537`
- **Would collide with:** any unit touching `crates/aep-domain/src/ids.rs` (every identifier pattern lives in one macro table), `crates/aep-driver-spec/src/pin.rs`, `crates/aep-driver-spec/tests/published_pattern_evaluated.rs`, or **regenerating anything under `schemas/generated/`** — the last is a whole-directory collision, since `xtask schema` rewrites the set together

**Not established.** The story's *Context* says `PinnedWorkflowRef::PATTERN` *"is now composed from `WorkflowId::PATTERN`"*. At `3d86d5b` it is a plain literal (`pin.rs:72-73`), so *one line in `aep-domain`* understates the change and `pin.rs` is on the surface. The test file the acceptance cites, `crates/aep-driver-spec/tests/pin_pattern_agrees_with_the_loader.rs`, was deleted; a comment at `published_pattern_evaluated.rs:433` says its one unique statement moved there. The test name the acceptance cites does not exist either — the tree has `the_only_pins_the_published_pattern_calls_valid_that_the_loader_refuses_are_the_numeric_tail` at `:473`. And the story names `WorkflowId` and `ProfileId` only, while `StepMapId`, `ToolRef`, `WorkflowRef` and `ProfileVersionedRef` share the gap; whether they are in scope is unresolved by the body.
