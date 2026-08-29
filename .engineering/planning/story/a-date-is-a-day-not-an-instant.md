---
format: aep.planning-md/1
id: story:a-date-is-a-day-not-an-instant
kind: story
status: active
title: A date written at UTC+2 is not a claim about the future
summary: A bare observation date means midnight UTC, so 20 of an adopter's 215 records read as future-dated for the last two hours of every UTC day — and one future record refuses the whole evidence document.
owner: protocol
tags:
- adoption
- evidence
- protocol
relations:
- decomposes: epic:adopter-feedback-round-1
- informed_by: story:evidence-horizons
- informed_by: story:per-record-horizons
- serves: vision:O2
revision: 4
---
# Story: A date written at UTC+2 is not a claim about the future

## Outcome

Someone who writes today's date in an evidence document gets it accepted today, wherever they are.
Today, for the last two hours of every UTC day, an adopter east of Greenwich writes the correct
date, and the engine refuses **the whole document** — every other record in it included.

## Context

Same adopter as `story:evidence-horizons`, third round, **2026-08-29**, and this is the smaller of
the two findings that round produced. It is not an argument against the future-date refusal — that
refusal is right, it is invariant 7, and this story keeps it. It is an argument about **granularity**
in two places: the unit the comparison is made in, and the unit the refusal is reported in.

### A bare date is midnight, and midnight is not local

`observed_at` accepts the calendar date a person writes in a document, and that date means
**midnight UTC** — stated in the type's own doc comment,
`crates/aep-domain/src/time.rs:519-522`:

```yaml
observed_at: 2026-08-30        # midnight UTC on that day
observed_at: 1788134400000     # the same instant, exactly
```

A submission whose `observed_at` is after the engine's clock is refused —
`crates/aep-domain/src/time.rs:545-550` (`ObservedAt::is_after`), called at
`crates/aep-engine/src/engine.rs:361-366`, raising
`ProtocolError::ObservationInFuture` (`crates/aep-engine/src/error.rs:94-99`).

The adopter's store sits at **UTC+2** and writes local calendar dates. So between 22:00 and 24:00
local — the last two hours of every UTC day — the date a writer has just written parses to an instant
in the future, and the record is refused. Measured in their tree at **22:27 UTC on 2026-08-28: 20 of
215 records.** Their exporter clamps exactly those 20 to the export instant in epoch milliseconds,
deterministically, with the reason in a comment beside each; the other 195 stay readable ISO dates.
Their note on it: *"a store west of Greenwich would never have found this."*

### The refusal is document-wide, and it need not be

`protocol evaluate` submits an evidence file record by record and propagates the first failure with
`?` — `crates/protocol-cli/src/main.rs:3988-3994`. One record two hours ahead therefore discards the
other 214, and the run produces no evaluation at all rather than an evaluation missing one fact.
What the adopter saw:

```console
error: submitting evidence from …/verify.yaml: the observation time 1787961600000ms is in the
       future; it is 1787956053626ms
```

The error names an epoch-millisecond pair and no file position, so the record that caused it is not
identified either.

### This repository has already decided it the other way, once

`protocol evidence inspect` reads **the same document** and does both halves differently, on purpose:

* It compares at **civil-date granularity**, and says why —
  `crates/protocol-cli/src/main.rs:4843-4848`: *"a record stamped 14:07 today is not 'in the future'
  of today … comparing millis against it would refuse every record the day it is written, which is
  the verb's primary use."*
* It collects future observations **per record**, prints the whole table anyway, and carries the
  verdict in the exit code — `crates/protocol-cli/src/main.rs:4835`, `:4848-4851`, `:4894-4897`.

Its own help says the two verbs apply *"the same refusal to a future observation time"*. They do not:
one refuses an instant and the file, the other reports a day and the record. Two verbs over one
document with two answers is the shape that produces a bug report from whoever meets the second one.

## Acceptance

- A bare calendar date in `observed_at` is not refused for being *today somewhere*. The comparison
  runs against the end of that calendar day in the most-ahead timezone in use (UTC+14), so a date is
  refused only when it is unambiguously in the future for every writer on earth. A full instant —
  the epoch-millisecond spelling — keeps the exact comparison it has today, because a caller who
  wrote an instant meant one.
- The two spellings are distinguishable where the comparison is made. Today they are not: both parse
  to a `Timestamp` and the information that one was written as a day is gone by the time
  `is_after` runs. Whatever carries that distinction is the design question this story turns on.
- A future observation refuses **that record**, naming it, and not the document. The other records in
  the file are submitted; the run reports the refusal as a finding a person can act on. A document
  where every record is future-dated still fails — nothing is downgraded to a warning.
- The refusal names the record's file position and the date as written, not only an epoch pair. The
  message an adopter pasted (`1787961600000ms` versus `1787956053626ms`) does not say which of 215
  records it is about.
- `protocol evaluate` and `protocol evidence inspect` answer identically about the same file. Where
  they still differ, `inspect`'s help stops claiming they are the same.
- Invariant 7 is unchanged and is asserted to be: a caller still cannot back-date by omission,
  `observed_at` is still required, and a scheduled-but-never-performed check is still unwritable. The
  test that reaches the state where this is load-bearing is a record dated **tomorrow** in every
  timezone, which is refused, beside one dated **today at UTC+14**, which is not.
- The corpus records the case. `examples/evidence-horizons-corpus/` is where the annotation forms
  that broke a parser live, and *the writer's day is not the engine's day* is the same class of
  finding as the seven parse positions already in it.

## Out of Scope

Timezone-aware observation times, offsets in the document, or a `timezone:` key anywhere. The
adopter did not ask for one and it would make every evidence document carry a field that is right
99.9% of the time and unreviewable when it is wrong. A date is a day; the question is only which
instants count as that day.

Any relaxation of the refusal itself. A record genuinely in the future stays refused, for the reason
`crates/aep-engine/src/error.rs:87-93` gives.

Clamping. The adopter clamps in their own exporter and that is the right place for a workaround; the
protocol should not clamp a caller's date, because a clamp is the engine deciding when the
observation happened, which invariant 7 forbids.

## Open Questions

**Which of the two halves is the fix?** Decides: protocol owner. Default if nobody answers: **both,
in that order** — the day-granularity comparison first, because it removes the whole class for every
adopter east of Greenwich and needs no new error path, and the per-record refusal second, because it
is the one that changes what `protocol evaluate` does with a partly-bad file and deserves its own
review.

**Where does *this was written as a day* live?** Decides: protocol owner. Default if nobody answers:
**on `ObservedAt`**, as the parsed granularity beside the instant — that type already exists to give
the future comparison and the age computation one home
(`crates/aep-domain/src/time.rs:500-505`), and putting the granularity anywhere else means a second
place that has to agree with it.

**Does a per-record refusal change what an execution's audit holds?** Decides: protocol owner.
Default if nobody answers: **the refusal is recorded, the record is not** — invariant 15's rule, that
a refused command changes nothing and is still recorded, applied unchanged here.
