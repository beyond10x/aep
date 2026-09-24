---
format: aep.planning-md/2
id: review-result:validate-v2-review-1
kind: review-result
status: active
title: 'Independent verification pass 1: validate answers a migrated Eventlog plan'
owner: aep
relations:
- reviews: task:validate-v2-projection-awareness
revision: 1
---
unit: task:validate-v2-projection-awareness — commit af2af7e74c0230dbb5f8a202e68b719dbfbed7a4 (base effef5b1a30019f4c9c2e143abe614cc9d5db89b), worktree home-path:sha256:a23a1d1e1aacbefe3795051159bb659f123cea241d785ff98042170c2d4ebe99 at that commit plus 208 test-only lines
verdict: red
cases: executed 580→584, red 2
origin: introduced 1, pre-existing 2, undecided 0
wrote-outside-worktree: 5 paths (listed in §6)
needs-coordinator: yes — the brief's own `TMPDIR=<scratch>` cannot be used as written (§6); confirm the substitute or shorten the assigned scratch path for the second pass

## 1. `git --no-pager diff --stat`

```
 crates/edge/aep-cli/tests/store_writer_control.rs | 208 ++++++++++++++++++++++
 1 file changed, 208 insertions(+)
```

One path, a test file. No implementation file was edited, not even briefly; no probe needed a
scratch copy of one.

Two untracked zero-byte files exist in the worktree that I created and that are not test files:
`.engineering/.aep-planning-writer.lock` and
`.engineering/.aep-planning-writer-592c1a5f2f6a234df6b3d2f75f2becd249485f5360340db21fb738506502c526.lock`
(both 22:09:12, written by the single read-only `aep plan artifact show --store <planning store>`
the brief invites — the CLI takes a planning writer lock in the discovered project even for a read
against another store). They are outside `git diff` because they are untracked. I left them rather
than delete a lock file, and name them here so the coordinator is not surprised by them.

## 2. Cases added — `crates/edge/aep-cli/tests/store_writer_control.rs`

| line | case | asserts | now |
| --- | --- | --- | --- |
| 1478 | `validate_reports_a_foreign_document_planted_in_an_eventlog_projection` | a planning document written into the projection of a migrated v2 plan is reported by `validate` | **red** |
| 1511 | `validate_reports_a_projected_document_deleted_outside_a_command_on_an_eventlog_plan` | a projected document deleted outside a command is one authority-decided drift finding | green |
| 1538 | `validate_reports_an_eventlog_projection_left_at_an_earlier_published_state` | a projection restored to the state the *previous* command published, while the authority holds a later move, is reported | green |
| 1602 | `strict_validate_reports_a_pre_migration_review_without_an_outcome_on_an_eventlog_plan` | a `review-result` created before migration, never answered, 262 days old, is reported under `--strict --outcome-within 7` | **red** |

All four were written before anything was run. Each was then run alone. Outputs verbatim (the two
red ones re-captured after `cargo fmt -p aep-cli` reflowed my own block and shifted line numbers;
the pre-fmt runs were identical but reported `:1501` and `:1654`).

### 2a. Red — planted document (run alone)

```
running 1 test
test validate_reports_a_foreign_document_planted_in_an_eventlog_projection ... FAILED

thread 'validate_reports_a_foreign_document_planted_in_an_eventlog_projection' (549844) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:1501:5:
a planning document planted in the projection is a change nothing decided, and validate reported the plan clean: {"artifacts":1,"files_read":1,"pre_provider":0,"problems":[],"store":"the Eventlog store home-path:sha256:818ceb56d68d84bfea336f4ade412489b9595297b5d5278865095bf9df68304c (control-sql/control-sql-tenant/01a0c06c-e36d-7033-a038-621b9400f198)"} — verify said {"format":"aep.planning-verification/1","outcome":{"kind":"verified","value":{ … "projection":{"authority_snapshot":"sha256:f016aef9…","drift":"current","inventory_digest":"sha256:52d7dc99…", … }}}} —

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 13.68s
```

(the `verify` payload is elided only at ` … `; it is `"drift":"current"`, verbatim, and the full
line is in `scratch/suite-nofailfast.log`.)

### 2b. Red — pre-migration review without an outcome (run alone)

```
running 1 test
test strict_validate_reports_a_pre_migration_review_without_an_outcome_on_an_eventlog_plan ... FAILED

thread 'strict_validate_reports_a_pre_migration_review_without_an_outcome_on_an_eventlog_plan' (644350) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:1656:5:
a review recorded before the migration and never answered is reported by no class once the plan is Eventlog: {"artifacts":2,"files_read":2,"pre_provider":0,"problems":[],"store":"the Eventlog store home-path:sha256:1dfe7dff9759383a2dab915fcd82ade55ce7c834bf0244311d089b3fbdb73389 (control-sql/control-sql-tenant/01a0c078-b44f-7326-899e-f1f478456f00)","without_findings":["review-result:old states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere"]}

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 2.64s
```

The case proves the plan holds the review (`artifacts: 2`) and that `validate` sees it
(`without_findings` names it) before it asserts the class is empty, so the red is the missing
class and not a missing fixture.

### 2c. Green — deleted projected document (run alone)

```
test validate_reports_a_projected_document_deleted_outside_a_command_on_an_eventlog_plan ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 14.73s
```

### 2d. Green — projection left at an earlier published state (run alone)

```
test validate_reports_an_eventlog_projection_left_at_an_earlier_published_state ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 42.78s
```

This was written as an attack and it failed: I expected the watermark search in
`projection_inventory` (which accepts any watermark whose inventory digest matches) to accept a
rolled-back projection. It does not — a projection restored to the first move's published bytes,
with the authority at the second move, is reported. The invariant holds and the case now guards it.

## 3. Suite

First run, `cargo test -p aep-cli`, stopped at the failing lane (34 of 37 lanes, exit 101). The
number in the header comes from the second run, with nothing deselected:

```console
$ TMPDIR=… cargo test -p aep-cli --no-fail-fast
```

```
37 lanes
582 passed; 2 failed; 0 ignored          (executed 584)
test result: FAILED. 11 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 101.22s   (tests/store_writer_control.rs)
failures:
    strict_validate_reports_a_pre_migration_review_without_an_outcome_on_an_eventlog_plan
    validate_reports_a_foreign_document_planted_in_an_eventlog_projection
EXIT=101
```

`<before>` = 580 is the implementing state's own `cases:` line (report.md §4, `05-green-aep-cli.log`);
I did not run the suite before my cases existed. `tests/drift.rs` ran 6 passed, 0 failed, unchanged
and unedited — contract item 3 holds for Markdown and Hybrid. `cargo fmt -p aep-cli -- --check`
exit 0 and `cargo clippy -p aep-cli --all-targets -- -D warnings` exit 0 after my additions.
Postgres lanes were not exercised (`ENTITY_POSTGRES_URL` left unset, per the brief), so
"Postgres behaves byte-identically" is unmeasured by me and rests on the author's `task check`.

Full logs: `scratch/suite.log`, `scratch/suite-nofailfast.log`.

## 4. Findings

**1 — `--strict`'s reviews-without-an-outcome class is dead on every Eventlog plan.**

* **file:line** `crates/edge/aep-cli/src/planning.rs:5694`
* **what was measured** `store_writer_control.rs:1656`, exit 101. On a plan migrated through the
  public path, a `review-result` whose creation the frozen journal records (2026-01-01), which no
  `review_outcome` names and which `validate` demonstrably sees (it is named in
  `without_findings`), produces an empty `without_an_outcome`. The diff changed that function's
  first line from `opened.files.as_ref()` to `opened.journal()`, and `journal()` is `None` for
  `Plan::Eventlog`, so the class can never be non-empty on v2 — not for a pre-migration review,
  not for one recorded after.
* **what reaches it** any migrated plan plus `aep plan artifact validate --strict`; the class is
  promised unconditionally by `website/docs/reference/cli.md:101` ("a `review-result` at least
  `--outcome-within` days old (14 by default) that no `review_outcome` record names"), which this
  diff does not change. Gate step 3 (`task plan-check`, Taskfile.yml:133) calls `validate` without
  `--strict`, so no gate goes green on this today — that is why it is not a blocker.
* **base** `git show effef5b1a:crates/edge/aep-cli/src/planning.rs` line 5673 reads
  `opened.files.as_ref()`, and the base `open_plan` Eventlog arm (line 1058) sets
  `files: Some(store)`, so at base this review *was* reported. Determined by reading the base, not
  by running it.
* **verdict** NEEDS-CHANGE · **origin** introduced · **severity** warning
* **named correction (not applied)** either keep the class alive on v2 by deriving the review's
  creation instant from the authority — the `None` arm of `evidence_on_hand`/`entries_of` already
  reaches events — or, if the loss is accepted, say so in `website/docs/reference/cli.md:101` and
  in the CHANGELOG entry, which currently says only that the advisories "answer for an Eventlog
  plan as they do for SQLite and Postgres" and does not say that a record which exists is no
  longer read. The author's report §5 names the switch but not this consequence, and the brief's
  deliberately-open list (item 6) does not cover it.

**2 — a planning document written into a v2 projection is reported by nothing.**

* **file:line** `crates/edge/aep-cli/src/store_command.rs:1584`
* **what was measured** `store_writer_control.rs:1501`, exit 101. After a governed move,
  `.engineering/planning/story/two.md` was written by hand. `validate` answers
  `{"artifacts":1,"problems":[]}` — the document is not even loaded, because a v2 plan's documents
  come from the authority — and `plan store verify` on the same tree answers
  `"drift":"current"`, exit 0. `projection_current` inherits this from
  `projection_owned_inventory`: `read_owned_files`
  (`crates/plan/aep-planning-migration/src/projection.rs:693`) reads only the paths the ownership
  marker lists, so a path the marker does not name is not part of any digest.
* **what reaches it** writing a `.md` file into `.engineering/planning/` — the habit every
  Markdown-plan repository carries into migration, and on a Markdown plan `validate` does report
  it. Not an observed incident: the case constructs the file. The *act* is the same class the
  unit's own `validate_reports_a_hand_edited_eventlog_projection_as_drift_decided_by_the_authority`
  models, which is why I did not downgrade this to INFEASIBLE.
* **base** reproduces by reading: the diff does not touch `open_plan` or `log_findings`; at base a
  v2 plan's `report` also came from the authority and `journal::reconcile` only reports artifacts
  the journal names, so `story:two` was invisible at base too. `verify` is untouched by this diff
  and misses it as well.
* **verdict** CONFIRMED · **origin** pre-existing · **severity** warning
* **note on the unit's own words** the store Outcome and the CHANGELOG say "a projection edited by
  hand is reported" / "a projection edited outside a command is still a problem". That sentence is
  broader than the check: a *change to an owned file* is reported (proved by the unit's case and by
  my deletion case), an *addition of an unowned file* is not. Correcting the sentence costs a line;
  closing the gap is `verify`'s to close and belongs in its own story.

**3 — the two cases this unit ships cannot run under the scratch path the brief assigns.**

* **file:line** `crates/edge/aep-cli/tests/store_writer_control.rs:323`
* **what was measured** with `TMPDIR` set to the assigned scratch directory (113 characters),
  every cargo invocation fails before compiling: `sccache: error: path must be shorter than
  SUN_LEN`. With `TMPDIR` set to a short *symlink* to that same directory, `sccache` is happy and
  the migration fixture refuses instead: `apply` returns
  `{"outcome":{"kind":"refused","value":{"refusals":[{"code":"receipt_conflict"}]}}}`, and the
  author's own
  `validate_answers_a_migrated_eventlog_plan_from_its_authority_not_the_frozen_legacy_journal`
  fails at `apply_selected_source` for that reason. The same test passes unchanged with `TMPDIR`
  set to a short real directory. Runtime observation, three runs, one variable.
* **what reaches it** any project reached through a symlinked path — the receipt comparison appears
  to compare selector host paths as given rather than canonicalised, but I did not isolate the
  line, so treat the mechanism as unproven and the observation as the finding.
* **verdict** CONFIRMED · **origin** pre-existing (the helper and the receipt comparison predate
  this diff) · **severity** note

## 5. Reviewed and not faulted

* Contract 1 — after a governed move on a migrated plan, `validate` reports neither drift nor a
  forged revision: the unit's own case passes here, and my two extra fixtures reach the same state
  without a false positive.
* Contract 2 — the hand-edit *of an owned file* is reported exactly once, in both `drift` and
  `problems`, and my deletion case shows the same for a removed owned file.
* Contract 3 — `tests/drift.rs` 6/6, unedited; `journal()` is an exhaustive match returning
  `self.files` for Markdown, Hybrid, SQLite and Postgres, so those four cannot have changed.
* Contract 5 — `verify` and `inspect` are untouched by the diff; `projection_current` propagates
  `projection_inventory`'s error with `?` rather than swallowing it, which is what makes the
  deletion case red-on-mutation.
* The finding text's remediation command is real: `plan store rebuild --authority-snapshot <id>`
  exists (`store_command.rs:91-96`).
* The stale-projection attack in §2d failed — the watermark search does not accept a superseded
  published state.
* Contract 6's declared-open set (`evidence_on_hand`, `reviews_of`, `review_records`, `outcomes_of`,
  the `asserted` list) — read, no new consequence found beyond finding 1, which is a different
  function and not on that list.

## 6. Paths written outside the worktree

1. `home-path:sha256:29dd2d006d31be7c24a84439ee8fd23ee692dd5cc2fe979b2cea4a340e835754`
2. `home-path:sha256:4cfa2ccccb7676056979914ce87b85069692358b000dc946fc525da785b9c719`
3. `home-path:sha256:4335e720018a95765df2e1bff71b67be50627609c16a521781110a4ade9dd661` — 311 fixture directories and writer-lock files left by the package suite's own temp usage, moved here from (5)
4. `home-path:sha256:621133e389b2f5aa5a29b63994c0f61712640a8d39602af15d086f95e950bb5b` and `aep-writer-control-*` — four fixture roots left behind by the two red cases, which panic before their own cleanup
5. `home-path:sha256:431dfd714f9bbf603b79e567127b2231b9c61ee03c4f360bbdd33eba913df38b` — **the substitute `TMPDIR`**, created because the assigned
   scratch path cannot serve as one (finding 3). Everything it held was moved into (3) and the
   directory was removed; it no longer exists. This is a deviation from the brief's triple and it
   is the one thing I could not settle without the coordinator.

Also written: this report, at
`home-path:sha256:39d4fe3cd372d6b4c9f9de4c2939819c4794280888ec2c1ed45155da6935b22b`.
Nothing under `/tmp`. No planning-store write, no `aep plan artifact` write verb, no git write
command, no worktree command other than the three lease hooks for session
`review-1-validate-v2-20260920`.

```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 5694
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: routing reviews_without_an_outcome through Opened::journal() makes the --strict reviews-without-an-outcome class permanently empty on every Eventlog plan, including a pre-migration review whose creation instant the frozen journal still holds and which the base reported, while website/docs/reference/cli.md:101 still promises the class unconditionally.
- file: crates/edge/aep-cli/src/store_command.rs
  line: 1584
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: projection_current digests only the files the ownership marker lists, so a planning document written by hand into a v2 projection is reported by neither validate nor plan store verify, which is narrower than the unit's claim that a projection edited by hand is reported as drift.
- file: crates/edge/aep-cli/tests/store_writer_control.rs
  line: 323
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the migration fixture refuses with receipt_conflict when the project root is reached through a symlinked TMPDIR and cargo refuses outright when TMPDIR is longer than SUN_LEN, so this unit's two new cases cannot be reproduced under the scratch path the brief assigns.
```
