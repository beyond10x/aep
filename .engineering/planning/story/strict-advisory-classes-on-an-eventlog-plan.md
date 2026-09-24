---
format: aep.planning-md/2
id: story:strict-advisory-classes-on-an-eventlog-plan
kind: story
status: draft
title: validate --strict reports its three advisory classes on an Eventlog plan from the authority
owner: aep
relations:
- informed_by: review-result:validate-v2-review-2
- informed_by: review-result:evidence-on-hand-review-1
- informed_by: story:eventlog-planning-authority-migration
revision: 1
---
## Outcome

On an `aep.project/2` Eventlog plan, `aep plan artifact validate --strict` reports the same three
advisory classes it reports on a Markdown plan, from the authority: `closed_on_an_assertion`,
`pre_provider` and the full `without_an_outcome` (including a review the legacy journal never
recorded). `website/docs/reference/cli.md` promises them unconditionally; until this lands the
Eventlog exception is documented there and in CHANGELOG.

## Findings this comes from

review-result:validate-v2-review-2 (2026-09-20, pass 2 over af2af7e74 as corrected by 11cc13e10),
and review-result:evidence-on-hand-review-1 finding 2:

- `crates/edge/aep-cli/src/planning.rs:6131` — `closed_on_an_assertion` is built from
  `Opened::journal()`, `None` on Eventlog, so a status closed on an assertion before or after
  migration is listed by nothing (cases in
  `.ess-evolution/waves/0005-aep-migration/wave-validate-v2-20260920/unit-1-validate-v2/review-2-cases.patch`
  at :2119 and :2151, exit 101).
- `planning.rs:6097` — `pre_provider` is 0 by construction on Eventlog; a document the Markdown plan
  reported as predating the event log is counted by nothing after migration, and a never-answered
  `review-result` the journal never recorded falls out of both classes (:2480, :2536).
- `planning.rs:599` (pre-existing, bounds the fix) — an imported document the legacy journal never
  recorded has an empty readable history on the Eventlog plan (`history --format json` is `[]`), so
  no history-based class can date it. `pre_provider` on v2 is therefore "documents whose contract
  history is empty", which is the imported pre-journal set exactly.
- Cost (evidence-on-hand-review-1 finding 2): each per-entity contract read costs about 0.8–1.1 s
  on the 2026-09-20 provider, on top of 5.0 s to open, so a faithful port that reads every
  artifact's history is 4–5 minutes on the 299-document AEP store. The provider fix
  (Eventlog: verify the log and blobs once per open rather than per transaction) must land first;
  after it, measure again before choosing between per-entity reads and a store-wide history read.

## Decision recorded

Timo, 2026-09-21 00:05 CEST: option B. Document the classes as closed on Eventlog plans now; port
them here after the provider is fast. The four red reviewer cases were rewritten in
task:evidence-on-hand-v2-journal to assert today's behaviour and name this story; invert them when
this lands.

## Acceptance

- The four cases above, inverted, pass; Markdown behaviour unchanged (`tests/drift.rs`).
- `validate` on a migrated fixture with 10 artifacts and 2 review-results completes in under 2 s
  with the provider fix in place (measured, three runs).
- cli.md and CHANGELOG no longer carry the Eventlog exception.

## Scope

cited: crates/edge/aep-cli/src/planning.rs, crates/edge/aep-cli/tests/store_writer_control.rs,
website/docs/reference/cli.md, CHANGELOG.md.
