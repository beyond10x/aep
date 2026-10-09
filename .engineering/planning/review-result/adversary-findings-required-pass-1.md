---
format: aep.planning-md/3
id: review-result:adversary-findings-required-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: a review-result without a findings block is refused, unless it says why'
relations:
- reviews: story:review-result-requires-findings
revision: 1
---
# Adversary pass 1: a review-result without a findings block is refused, unless it says why

unit: U1 of wave findings-required, commit d28f7eb00 on unit/findings-required, plus one new untracked test file
verdict: NEEDS-CHANGE
cases: executed 685→688, red 3
origin: introduced 5 / pre-existing 1 / undecided 0

## Cases added

`crates/edge/aep-cli/tests/findings_required_adversary.rs`, all three red:

- `a_shallow_clone_dates_a_review_by_the_commit_that_added_it_not_by_the_clone_boundary` (:185): a project opted in on 2026-06-01, a prose review committed 2026-01-05, an unrelated commit on 2026-10-05. Full history: `exempt_before_opt_in`, exit 0. A `git clone --depth 1` of the same commit: `(1, Some("missing"))` instead of `(0, Some("exempt_before_opt_in"))`; `created_at` 2026-10-05T00:00:00Z.
- `moving_the_project_directory_does_not_redate_a_review_recorded_before_the_opt_in` (:208): `.engineering` moved away and back with `git mv`: `(1, Some("missing"))` instead of `(0, Some("exempt_before_opt_in"))`.
- `show_returns_the_prose_only_reason_of_a_review_recorded_prose_only` (:276): `show --format json` carries no `prose_only` (`left: None`, `right: Some("the reviewing tool writes no block yet")`).

Suite: `CARGO_INCREMENTAL=0 cargo test -p aep-cli --no-fail-fast`: passed 685, failed 3 (`findings_required_adversary`), EXIT=101.

## Findings

- F1 (NEEDS-CHANGE, blocker, introduced) `git_record.rs:225`: `first_committed` runs `git log --no-renames --diff-filter=A` without checking for a shallow clone; at depth 1 every file is added by the tip commit, so every pre-opt-in prose review is dated by the clone boundary and counted `missing`. Reached by `website/docs/guides/validate-in-ci.md:40` (`actions/checkout@v4`, default depth 1). The dating mechanism is pre-existing; until now only the uncounted outcome reminder used it.
- F2 (INFEASIBLE, note, introduced) `git_record.rs:225`: `--no-renames` dates a review moved with `git mv` by the move; no aep verb moves review files.
- F3 (CONFIRMED, note, introduced) `planning.rs:5463-5467`: a store outside Git (a `git archive` export) dates every review now: exit 0 in the repository, exit 1 in the export. Follows the spec's `created_at`, but guesses where *Unknown differs from false* would say unknown.
- F4 (CONFIRMED, warning, introduced): `show` (text and JSON) omits `prose_only` while returning `findings: []` for a block-less review; an SQLite store's reader cannot tell found-nothing from prose-only on purpose.
- F5 (NEEDS-CHANGE, note, introduced) `CHANGELOG.md:11`, `website/docs/concepts/reviews.md`: they say `new` refuses from the opt-in date; code and spec refuse whenever the key is set (measured: `findings_required_since: 2099-01-01`, `new` exits 1).
- F6 (CONFIRMED, note, introduced) `project.rs:120`: an empty, `~` or `null` `findings_required_since` is read as no opt-in without a message.
- F7 (INFEASIBLE, note, pre-existing) `time.rs:155`: `CivilDate::parse` accepts `+202-10-01` as year 202.

## Attacked and not broken

The refusal at `new` (no block, template, `review` alias, a block nested in a four-backtick fence); an unclosed block; `--prose-only` beside `--findings`, on another kind, blank including U+3000; odd reasons quoted and read back; `prose_only` through `relate`, `unrelate`, `evidence`, `move --to archived` in Git and SQLite stores; dates 2028-02-29 accepted, 2026-02-29, a timestamp, a trailing space refused naming the key; exemption precedence; supersedes counted only from a review carrying a block; `--strict` with and without the key; `doctor` agreeing with `validate`.

```findings
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 225
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: in a depth-1 clone, which the CI guide's default checkout makes, every pre-opt-in prose review is dated by the clone boundary and counted missing, so the before-opt-in exemption fails in CI while passing locally
- file: crates/edge/aep-cli/src/planning/git_record.rs
  line: 225
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: --no-renames dates a review whose directory was moved with git mv by the move, turning an exempt review missing; no aep verb is shown moving one
- file: crates/edge/aep-cli/src/planning.rs
  line: 5463
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a store outside Git, such as a git-archive export, dates every review now, so identical files validate 0 in the repository and 1 in the export
- file: crates/edge/aep-cli/src/planning.rs
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: show (text and JSON) omits prose_only while returning findings [] for a block-less review, so an SQLite store's reader cannot tell found-nothing from prose-only on purpose
- file: CHANGELOG.md
  line: 11
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: CHANGELOG and reviews.md say new refuses from the opt-in date, but code and spec refuse whenever the key is set, including a future date
- file: crates/govern/aep-domain/src/project.rs
  line: 120
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an empty, ~ or null findings_required_since is silently read as no opt-in rather than refused naming the key
- file: crates/govern/aep-domain/src/time.rs
  line: 155
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: CivilDate::parse accepts a sign-prefixed year such as +202-10-01 as year 202
```
