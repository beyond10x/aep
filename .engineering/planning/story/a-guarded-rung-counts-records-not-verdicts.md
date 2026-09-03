---
format: aep.planning-md/1
id: story:a-guarded-rung-counts-records-not-verdicts
kind: story
status: draft
title: A guarded rung counts records, and cannot ask that one passed
relations:
- serves: vision:O2
revision: 3
---
## What was found

Found on 2026-09-03 by walking the ladder `0.50.0` shipped, against a real store.

`artifacts/lifecycles/executable-system-specification.yaml` declares:

```yaml
requires:
  conforming:
    - evidence: ess_conformance
      at_least: 1
```

`aep artifact move … --to conforming` was admitted on a report that says
`"status": "failed", "scenarios_total": 24, "scenarios_failed": 12`. The rung counts records. It
cannot ask that one of them *passed*.

The observation, in `sbf/acd`:

```console
$ aep artifact move executable-system-specification:acd-v3 --to conforming
executable-system-specification:acd-v3 moved validated -> conforming (revision 4)
```

## Why this is not covered by the principle

`principles/verification/ess-conformance.yaml` does judge the verdict —
`ess_conformance.passed` and `ess_conformance.scenarios.failed == 0`. It is the right rule and it
is written correctly. It gates a **task's completion**, not an **artifact's status**, so nothing in
it reaches the `move`.

The lifecycle's own comment says the opposite: *"Whether the record passed … is the
`ess-conformance` principle's judgement"*. That is true of the task and false of the rung.

## The fix is bigger than it looks — and the first sketch of it was wrong

This story first proposed `predicates:` beside `at_least:`, with the shell evaluating them against
the records before counting. **That cannot be built as described**, and the reason is worth writing
down before anybody starts:

**The store keeps no evidence facts.** `Change::Evidence`
(`crates/aep-backend-markdown/src/journal.rs:117`) carries `kind`, `source` and `reference`.
`source` is free text — `recorded_from_report` renders the report into a sentence
(`crates/protocol-cli/src/planning.rs:5752`):

```rust
let source = format!(
    "ess conform run: {implementation} against {specification} at {digest}, {failed} of \
     {total} scenario(s) failed"
);
```

`evidence_from_events` (`planning.rs:849`) then counts by kind and nothing else survives. There is
no `ess_conformance.passed` in the store to evaluate. `OnHand`
(`crates/aep-backend-entity/src/kernel.rs:65`) is `BTreeMap<EvidenceKind, usize>` and its own
comment explains why the kernel takes counts rather than fetching: what the outside world knows
enters as an argument. That design is right; the problem is that the outside world does not know
either.

**Where facts do live**: the engine. `Execution` holds a `FactStore`
(`crates/aep-engine/src/execution.rs:94`) and predicates are evaluated against it
(`crates/aep-engine/src/evaluate.rs:814`), populated by whatever drives the execution. So AEP has
two worlds — an artifact store that holds records without facts, and an engine that holds facts
without artifacts — and the `ess-conformance` principle works in one while the `conforming` rung
lives in the other.

## What would close it

In order, because the first is a prerequisite for the second:

1. **An evidence record carries the facts it was read from.** `Change::Evidence` gains a `facts`
   map; `recorded_from_report` fills it from the report rather than only rendering it into
   `source`; `evidence_from_events` returns them. This is the substantive change and it touches the
   journal format, so it needs a decision about records already written — a record with no facts is
   not a record whose facts are all false.
2. **`StatusRequirement` gains `predicates`**, in the vocabulary the principles already use. The
   shell evaluates them against the facts from (1) and presents two numbers to the kernel:
   `evidence` — records presented, so `Unobservable` still means *nobody looked* — and a new
   `qualifying`, which is what `at_least` is checked against. Both are needed: a rung with three
   failing records must report *3 held, 0 passing* and not *nobody presented any evidence*, which
   is the exact confusion `Verdict`'s three values exist to prevent.
3. `artifacts/lifecycles/executable-system-specification.yaml` gains the two predicates, and its
   comment at line 28 is corrected.

The cheaper alternative — leave the rung as a count and correct the comment to say the status is a
claim nothing checks — makes the same word mean two things in two places, and `conforming` is a
word that names a verdict.

## What the affected project did meanwhile

`sbf/acd` moved the artifact back to `validated` and recorded why in its body. It moves by hand,
and not while anything is skipped.
