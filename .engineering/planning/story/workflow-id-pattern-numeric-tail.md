---
format: aep.planning-md/1
id: story:workflow-id-pattern-numeric-tail
kind: story
status: draft
title: A published pattern that still calls three step maps valid
relations:
- decomposes: epic:reference-driver
- informed_by: story:driver-spec-crate
revision: 1
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
