---
title: The evidence file
sidebar_position: 4
description: The JSON file aep plan artifact evidence writes for each record in an aep.project/5 store — path, name, fields, and the immutability rule.
---

# The evidence file

In an `aep.project/5` store, each evidence record about an artifact is one JSON file. This is the
**planning-store** record that `move` counts. The typed records that `aep govern evaluate` reads are
a different format, described under
[evidence records](./documents.md#evidence-records).

## Where

```text
.engineering/evidence/<kind>/<name>/<instant>-<digest>.json
```

| Part | Value |
|---|---|
| `<kind>/<name>` | the artifact's id, split at the colon: `story:pay-by-card` → `story/pay-by-card` |
| `<instant>` | the record's `at`, with every character except letters and digits removed: `20260928T085501Z` |
| `<digest>` | the first 12 hex characters of the SHA-256 of the file's own bytes |

Identical copies of one record, which can come from a migrated journal, get `-1`, `-2`, … appended
to the stem, because each copy counts.

## Fields

```json
{
  "at": "2026-09-28T08:55:01Z",
  "actor": "human:alex",
  "artifact": "story:pay-by-card",
  "kind": "story",
  "revision": 4,
  "change": {
    "change": "evidence",
    "kind": "test_result",
    "source": "cargo test -p checkout",
    "reference": "https://ci.example.invalid/runs/1042"
  }
}
```

| Field | Meaning |
|---|---|
| `at` | when the observation was made: `evidence --at`, or now |
| `actor` | who recorded it, from `AEP_ACTOR` |
| `artifact`, `kind` | what it is about, and that artifact's kind |
| `revision` | the artifact's revision when the record was admitted; `explain` prints it |
| `change.change` | always `evidence` |
| `change.kind` | the evidence kind, one the protocol declares: `test_result`, `approval`, `review_outcome`, … |
| `change.source` | where it came from (`--source`); for a `review_outcome`, the review's id |
| `change.reference` | where to look (`--ref`), when given |
| `change.review`, `change.outcome` | on a `review_outcome` only: the `review-result` id and `fixed`, `escalated` or `no-op` |

The file is pretty-printed JSON with a trailing newline.

## Rules

- **Never rewritten.** Writing the same record again answers the same path and changes nothing. A
  different file at that path is refused.
- **Written atomically.** The file is written under a temporary name in the same directory and then
  linked into place, so a reader never sees half a record.
- **Counted per artifact.** `move` counts the records under the artifact's own directory, per
  `change.kind`, against the rung's `requires:`.
- **Refused kinds.** An evidence kind the protocol does not declare is refused, and the refusal
  lists the kinds that are declared.

## Written by

| Command | Record |
|---|---|
| `aep plan artifact evidence <id> --kind <k> --source <s> [--ref <r>] [--at <instant>]` | one record |
| `aep plan artifact evidence <id> --kind review_outcome --review <review-id> --outcome <o>` | a review outcome |
| `aep plan artifact evidence <id> --from <ess-report> [--suite … \| --suite-input …]` | a record read out of an ESS conformance report |
| `aep plan store migrate git` | one file per journalled evidence record of an `aep.project/1` store |
