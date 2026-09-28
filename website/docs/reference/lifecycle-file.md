---
title: Lifecycle files
sidebar_position: 5
description: The format of an artifact lifecycle document — initial status, transitions, evidence-priced rungs, date-guarded rungs and descriptions — and how a kind finds its lifecycle.
---

# Lifecycle files

A lifecycle document declares the statuses of one artifact kind and the moves between them. They
live in the document tree under `artifacts/lifecycles/`, one file per kind. Any file name works,
because the `kind:` inside decides.

```yaml
kind: data-migration              # the kind this governs; omit for the tree's fallback
initial: draft                    # where `new` starts an artifact (required)
transitions:                      # status -> the statuses that may follow it
  draft: [rehearsed, abandoned]
  rehearsed: [applied, abandoned]
  applied: []                     # terminal
  abandoned: []
requires:                         # status -> what reaching it costs
  rehearsed:
    - evidence: test_result       # an evidence kind the protocol declares
  applied:
    - evidence: approval
      at_least: 2                 # default 1
when:                             # status -> a date the artifact itself records
  abandoned:
    before: decide_by             # reachable only while now is before the artifact's `decide_by`
descriptions:                     # status -> one line, shown by `board --format markdown`
  draft: Written down, not yet run anywhere.
```

## Keys

| Key | Required | Shape | Meaning |
|---|---|---|---|
| `kind` | no | kebab-case kind | the kind governed; absent means this is the fallback for kinds with no nearer lifecycle (at most one per tree) |
| `initial` | yes | status | the status an artifact is created at |
| `transitions` | no | map of status → list of statuses | the legal moves; a status mapped to `[]` is terminal |
| `requires` | no | map of status → list of `{evidence, at_least}` | evidence records of that kind, held for the artifact, needed to reach the status |
| `when` | no | map of status → `{after, before}` | each names a **front-matter key** of the artifact holding a date; `after: due` means *not until the instant is past this artifact's `due`* |
| `descriptions` | no | map of status → string | what being at the status means |

Unknown keys are refused. `when` compares against the instant `move --at` gives, which defaults to
now. An empty guard constrains nothing.

`requires` is deliberately smaller than a principle's evidence requirement. It counts records of a
kind and does not judge who produced them or whether they are independent. Those judgements belong
to the [engine](../concepts/governance.md).

## How a kind finds its lifecycle

1. the lifecycle whose `kind` is exactly the artifact's kind;
2. a lifecycle for a kind it specialises, nearest first. A kind's parent is named by its last hyphen
   segment, so `weekly-digest` → `digest` and `credential-blocker` → `blocker`;
3. the tree's fallback lifecycle (the document with no `kind:`).

## Checking one

```bash
aep govern validate --root <tree>          # the whole tree, lifecycles included
aep plan artifact lifecycle <kind>         # the ladder as the store will apply it
```

The JSON Schema is
[`artifact-lifecycle.schema.json`](https://github.com/beyond10x/aep/blob/main/schemas/generated/artifact-lifecycle.schema.json).
The shipped lifecycles are in
[`artifacts/lifecycles/`](https://github.com/beyond10x/aep/tree/main/artifacts/lifecycles).
