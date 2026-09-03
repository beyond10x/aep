---
format: aep.planning-md/1
id: story:a-guarded-rung-counts-records-not-verdicts
kind: story
status: draft
title: A guarded rung counts records, and cannot ask that one passed
relations:
- serves: vision:O2
revision: 2
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
`ess-conformance` principle's judgement"*. That is true of the task and false of the rung, and a
reader taking it at face value would believe `conforming` means what it says.

## Why it matters more here than elsewhere

`story: implemented needs 1 test_result` has the same shape and reads as *somebody ran the tests*.
`conforming` is a stronger word: it names a verdict, and a `conforming` specification whose suite
is red is a claim nothing behind it supports.

## What would close it

`requires:` gains a way to say what the record must hold — the vocabulary
`principles/**/*.yaml` already uses for the same evidence:

```yaml
requires:
  conforming:
    - evidence: ess_conformance
      at_least: 1
      predicates:
        - ess_conformance.passed
        - ess_conformance.scenarios.failed == 0
```

The alternative — leaving the rung as a count and correcting the lifecycle's comment to say the
status is a claim nothing checks — is cheaper and worse: it makes the same word mean two things in
two places.

## What the affected project did meanwhile

`sbf/acd` moved the artifact back to `validated` and recorded why in its body. It moves by hand
when the suite is green.
