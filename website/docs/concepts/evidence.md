---
title: Evidence and completion
sidebar_position: 4
description: Evidence records, the two times they carry, producers and independence, horizons and decay to Unknown, three-valued evaluation, ordering facts, and the join between AEP and ESS.
---

# Evidence and completion

Completion in AEP is not a declaration. It is a predicate over recorded facts, and this page
explains where those facts come from and how they are evaluated.

## Evidence records

Evidence is a typed record submitted to the engine: a test result, a static-analysis run, a
deployment result, an approval, a diff, a review, a verification, a conformance run. Each record
carries:

* a **kind** — one of the kinds the protocol declares; a kind the protocol does not declare is
  refused at submission, with the declared kinds listed;
* an **`observed_at`** date — when somebody looked. Required, supplied by the caller, and the
  subject of the next section;
* a **producer** — who made the observation: `Producer::Agent { id }` or
  `Producer::Verifier { verifier }` are different variants, and requirements read which one it was;
* optionally a **subject**, written `about:` on the wire — what the observation is of;
* **provenance** — command, tool, revision, workspace, environment, digest, inputs — so the record
  can be re-derived later by someone who does not trust the submitter;
* a **sequence number** stamped by the engine at submission.

The engine projects facts from records: a `test_result` becomes `tests.unit.failed`,
`tests.unit.total` and so on; an `approval` becomes `approval.<id>.granted`. Predicates read those
facts. The full projection table is in the [vocabulary reference](../reference/vocabulary.md).

## The two times on a record

A record carries two dates and they answer different questions.

| Field | Whose | What it says |
|---|---|---|
| `observed_at` | the caller's | when somebody looked at the thing |
| `produced_at` | the engine's | when the record entered the log |

**`observed_at` is the identity of the fact.** A green suite from three weeks ago, filed today, is
three weeks old — the filing tells you nothing about the code. So the engine will not infer the
observation date from the moment of submission, and it does not accept a record without one.

An observation time in the future is **refused outright**, with the code
`observation_in_future`. The reason is narrow and worth stating: a scheduled-but-never-performed
check, stored as an observation, reads as the freshest record in the log, and once one is in there
the store can no longer answer whether anybody has ever actually looked.

### A day is not an instant, and the refusal knows which you wrote

`observed_at` accepts two spellings, and they are two different claims:

```yaml
observed_at: 2026-08-30        # a day
observed_at: 1788134400000     # an instant — that day's midnight UTC
```

A **day** is only a day somewhere. `2026-08-30` written at UTC+2 at 00:30 local is
`2026-08-29T22:30Z`, so a store east of Greenwich writes tomorrow's UTC date for the last hours of
every UTC day, and the date is correct. A day is therefore refused only once it has begun in **no**
timezone — the boundary is its midnight in the most-ahead zone in use, UTC+14, fourteen hours before
its midnight UTC.

An **instant** is an instant everywhere and keeps the exact comparison, because a caller who wrote
one meant one. The two records above can get opposite answers from one clock, and that is the point:
the spelling a document was written in survives to the comparison.

Nothing is clamped. The engine does not move a caller's date to its own clock — that would be the
engine deciding when the observation happened, which is the one thing it may not do. A date that has
begun nowhere stays refused, and the refusal is not a warning.

**A future record refuses itself, not the file.** `aep govern evaluate --evidence` submits every other
record in the document, prints its evaluation, and exits `1` with one line per refused record naming
the file, the position in it, and the date as the writer wrote it:

```text
verify.yaml: record 12: the observation time 2026-08-30 has not happened yet; the clock reads 2026-08-28T22:27:33Z (1787956053626ms)
```

A document whose every record is future-dated still fails.

`aep observe evidence inspect` reads a record file and reports the ages without evaluating anything. It
puts every record to the same comparison, so the two verbs answer identically about one file:

```shell-session
$ aep observe evidence inspect examples/development-passkeys/evidence/01-red-test.yaml
test_result              2023-11-12 1013d old  -  verifier test-runner
1 record(s), aged at 2026-08-21
```

`examples/evidence-horizons-corpus/writers-day.yaml` is the corpus case: five records at one
reference date, two of which name the same instant and get opposite answers.

## Horizons: a fact that gets old reads `Unknown`, never `False`

An evidence requirement may declare a **`horizon`** — `horizon: 3d`. Past it, the requirement stops
being satisfied by that record, and what it reads is `Unknown`: a lapsed check has not failed,
**nobody has run it**. The reason names the horizon, the observation date and the day it lapsed, so
the report says *go re-run this* rather than *this is broken*.

The decay reaches the facts as well as the requirement. A lapsed record's facts are withheld from
the fact store under the strictest horizon the resolved plan declares for that kind, and an absent
fact is `Unknown` — so a guard that reads `tests.unit.failed == 0` directly, rather than through a
requirement, decays the same way instead of quietly running on a stale number. `evidence.lapsed`
sits beside `evidence.missing` so that a gate nobody has looked at recently is distinguishable from
a gate nobody has ever met.

**The horizon lives on the requirement and nowhere else.** A record has no horizon field, so there
is nothing on a submitted fact to extend; there is no operation anywhere that mutates one; and a
source scan over the five crates a horizon can be reached from — `aep-domain`, `aep-engine`,
`aep-backend-markdown`, `aep-driver`, `aep-cli` — refuses both `.horizon =` and any `fn` taking
`&mut self` with `horizon` in its name. The reason is behavioural rather than architectural: if
`extend` is as easy to call as `re-check`, `extend` is the one that gets called at six on a Friday.

Re-submitting the same record restores nothing. Only a new observation time does, which is the whole
point — the thing that clears a lapse is somebody looking again.

**What this does not do yet.** A requirement that declares a horizon and no `subject` is revived by
any fresh record of its kind, including one about something else entirely; the matcher checks the
subject only when one is given. That is an open row on the repository's gap register, inherited from
the matcher rather than introduced by horizons — see [Limitations](../status/limitations.md).

### Reading horizons in prose documents

Not every dated claim lives in an evidence file. `aep observe evidence scan` reads human-written
markdown for the one-line annotation convention a dated claim is written in, and reports coverage
beside the classification — how many annotation-shaped occurrences it saw against how many records
it parsed, per file, because an annotation that is present, correct and legible to a human but
invisible to the gate is the failure worth catching:

```shell-session
$ aep observe evidence scan examples/evidence-horizons-corpus/corpus --at 2026-09-01 --warn-days 2
...
43 occurrence(s), 43 record(s), 0 unparsed — 16 ok, 17 expiring, 10 expired, 8 malformed (at 2026-09-01)
```

That corpus is ground truth contributed by an outside adopter, who maintains a dated claim register
by hand for exactly this reason. `--strict` fails on a coverage gap and `--fail-on-expired` fails on
a stale claim, and they are separate flags because they answer separate questions: *is the gate
blind?* and *is the claim old?*

## Independence

An evidence requirement can be marked `independent: true`. The agent's own submission never
satisfies it — only a record whose producer is a verifier does. This is how "an agent cannot verify
itself" stops being a principle and becomes a type: the requirement reads the `Producer` variant.

The limit is worth stating as precisely as the mechanism. Independence is **structural, not
attested**: the engine checks who the record *says* produced it. A harness that lies about the
producer defeats the check, and nothing cryptographic prevents that today — see
[Limitations](../status/limitations.md), which treats this as the project's central trust
assumption.

## Three-valued evaluation: `Unknown` is not `False`

`tests.unit.failed == 0` is *false* when a suite ran and failed, and *unknown* when nothing ran.
Those are different situations demanding different responses, so predicate evaluation is
three-valued (Kleene logic), and **only `True` permits a transition**:

| Truth | What happened | What a harness should do |
|---|---|---|
| `True` | observed, and it holds | nothing |
| `False` | observed, and it is wrong | fix the code |
| `Unknown` | nothing observed it | run the verifier that would |

Collapsing `Unknown` into `False` produces an agent that "fixes" code nobody has tested. Collapsing
it into `True` produces one that finishes tasks nothing verified. The `Truth` type in `aep-domain`
has three variants, Kleene `and`/`or`, and deliberately no `From<bool>` and no `as_bool` — there is
no boolean to collapse into. The CLI renders the distinction as `✗` (false) versus `?` (unknown).

## Ordering is a fact

The engine records submission order, and `evidence.first_seq.<kind>` exposes it. Red-before-green is
therefore a checkable predicate rather than an instruction:

```text
evidence.first_seq.test_result < evidence.first_seq.diff
```

A test result was recorded before any code change existed. This only works if the harness submits
evidence as it observes it — batching everything at the end destroys the ordering facts.

## The adapter: how standalone ESS evidence completes AEP tasks

AEP and ESS are independent repositories that meet at one declared report boundary. A task can carry an artifact of kind
`executable-system-specification`; the `ess-conformance` principle then requires, before completion,
evidence of kind `ess_conformance` that is `independent: true` and produced by a
`conformance-runner`:

```yaml
requires:
  before_completion:
    conditional:
      - when: artifact.executable-system-specification.exists
        require:
          predicates:
            - ess_conformance.passed
            - ess_conformance.scenarios.failed == 0
          evidence:
            - kind: ess_conformance
              independent: true
              verifier: conformance-runner
```

Three consequences:

* **Nobody reads a diff to judge conformance.** The specification judges it, and the protocol
  refuses to call the task done until something other than the author ran that judgement.
* **A task with no specification owes nothing here.** The condition reads the artifact graph at
  evaluation time; adding a specification turns the rule on without editing the rule.
* **The evidence is revision-bound and fails closed.** A conformance run carries the digest of the
  specification it ran against, and it must match the `model_digest` the specification artifact
  records. If the artifact records no digest, no run can be shown current and the requirement can
  never be satisfied — evidence that cannot demonstrate its revision is not assumed fresh.

ESS publishes a standalone closed conformance report. The optional `aep-ess-evidence` crate reads
that report and converts it into AEP evidence without core AEP depending on ESS modeling types. It
refuses unknown fields, inconsistent counts, and a passing verdict that reports failures. The
report's completion time becomes the evidence observation time.

## A second thing that can produce a record: the transcript checker

ESS conformance judges the software. `aep observe trace check` judges the *run* — it reads an agent
transcript and holds it against a typed specification of what that run was supposed to do, and
`aep observe trace evidence` turns the verdict into a record of kind `trace_conformance`, produced by
the verifier `trace-checker`.

The record is a summary and not the report: counts, ids and two digests cross the boundary, and the
citation rows — which quote the prompt, the model's reasoning and file contents it read — do not.
That is why the evidence verb has no `--redact` flag; there is nothing left in the record for one to
remove. See [Check a transcript](../guides/check-a-transcript.md).

**`provenance.command` is spelled with the canonical `protocol` name, whichever binary was
invoked.** `aep trace evidence` and `protocol trace evidence` write the same record, byte for byte,
including that line. The provenance says what was *asked for*, in the vocabulary of the verb, and
that vocabulary has one spelling for the tool; which of the two names is on the operator's `PATH` is
a fact about the machine and not about the observation. It is what makes a committed record
regenerable by a reader who installed the other name — a `command` that followed the invoked binary
would make one check produce two documents, and every record already in a repository would diff
against a rerun of the command that wrote it.

---

**Sources.** `crates/govern/aep-domain/src/evidence.rs`; `crates/govern/aep-domain/src/requirement.rs` (the
`horizon` field and why it lives there); `crates/govern/aep-engine/src/error.rs`
(`observation_in_future`); `crates/govern/aep-domain/tests/horizon_immutability.rs` (the five scanned
crates); `crates/govern/aep-domain/src/predicate.rs` (the `Truth` type);
`principles/verification/ess-conformance.yaml`; `examples/evidence-horizons-corpus/`;
`examples/billing-conformance/`; `docs/design/evidence-horizons-design-v0.1.md`;
`docs/guide/harness.md`; `AGENTS.md` § *Invariants* 5 and 7. Command output on this page was
produced by `target/debug/protocol` at `0.10.0-horizons-dogfood-lab`.
