---
format: aep.planning-md/3
id: story:review-result-requires-findings
kind: story
status: active
title: A review-result without a findings block is refused, unless it says why
summary: new refuses a prose-only review without --prose-only; validate counts one as a problem once the store sets findings_required_since.
relations:
- decomposes: epic:review-facts
- serves: vision:O2
- informed_by: story:structured-findings-on-review-result
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T22:40:13Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T22:40:13Z", actor: "human:timo", revision: 4}
---
# Story: A review-result without a findings block is refused, unless it says why

## Outcome

A store that opted in cannot gain a review whose findings only prose holds. `new review-result`
refuses a body with no `findings` block unless the author records a reason, and `validate` counts a
prose-only review as a problem from the opt-in date on. Stores that have not opted in behave as
today.

## Context

Read against `main` at efd82bb78 (CLI 0.69.1):

- `crates/plan/aep-backend-markdown/src/findings.rs:606-608`: `parse` folds "no block" into an
  empty list through `recorded(..).map(Option::unwrap_or_default)`; `recorded` (`:618-633`) keeps
  the two apart.
- `crates/edge/aep-cli/src/planning.rs:2212-2229`: `new` calls `parse`, so it checks a block only
  when one is present; a body with none is admitted.
- `planning.rs:5713-5731`: `validate` lists a review with no block under `without_findings`
  whatever its status or `supersedes` edges; `:8232-8239` records why it is not a problem
  (retroactivity on an immutable kind); `:6229-6248`: only `--strict` refuses it.
- `artifacts/lifecycles/review-result.yaml`: `active -> archived` only; a correction is a second
  review. `supersedes` is a plain edge (`planning.rs:7865`), so a second review can name the first.
- `story:structured-findings-on-review-result` chose "reports (without failing)" and left
  retrofitting out of scope; this story takes the next step for stores that opt in.

The writers in `agentplugins` (critic rubric, adversary) change in the same release window.

## Acceptance

1. In a store whose `project.yaml` sets `findings_required_since`, `aep plan artifact new review-result <slug> --from <body>` with no `findings` block in the body
   and no `--findings` exits 1, writes nothing, and names the two ways forward: a block (`[]` for a
   review that found nothing) or `--prose-only <reason>`.
2. `--prose-only <reason>` admits such a body and records the reason in the artifact's front
   matter; an empty or whitespace-only reason is refused; the flag on any other kind, or beside a
   body that carries a block, is refused.
3. `project.yaml` takes an optional `findings_required_since` (a UTC date). With it set,
   `aep plan artifact validate` counts as a problem (exit 1) a `review-result` with no block, except
   one created before that date (read from its first transition), one that an artifact carrying a
   block `supersedes`, and one recorded with `--prose-only`; the exempt ones stay listed with the
   reason each is exempt.
4. A store without `findings_required_since` validates exactly as on 0.69.1: a missing block is
   reported, not counted; `--strict` still refuses it. This store's own `validate` stays green.
5. Migration needs no edit to an existing review: a new `review-result` with a block and
   `--relate supersedes:<old review>` takes the old one out of the problem list; outcomes go through
   `evidence --kind review_outcome` as today.
6. Spec first: the flag, the project key and the validate outcomes are declared in `ess/` (ESS
   0.56.0 or newer), projections regenerated, `task ess-gate` green.

## Out of Scope

- A verb that drafts the findings JSON from prose for a person to check (optional in the request);
  a later story if asked.
- Changing the writers in `agentplugins`.
- Editing or re-recording existing reviews in this store.

## Ambiguities

- Decided: `new` refuses a body with no block only in a store that set `findings_required_since`;
  a store without the key behaves as today, since an unconditional refusal would break every existing caller. `--prose-only` works in every store.
- `inferable`: "created before" compares the first transition's instant with the start of the
  `findings_required_since` date in UTC.

## Open Questions

None.
