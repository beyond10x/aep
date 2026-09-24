---
format: aep.planning-md/1
id: review-result:validate-v2-review-2
kind: review-result
status: active
title: 'Independent verification pass 2: validate answers a migrated Eventlog plan'
owner: aep
relations:
- reviews: task:validate-v2-projection-awareness
revision: 1
---
unit: task:validate-v2-projection-awareness
verdict: red
cases: executed 587→596, red 4
origin: introduced 4, pre-existing 1, undecided 0
wrote-outside-worktree: home-path:sha256:38e07107bd2f9bb9559849c037c28606dc203d3cb75fd510dee48f6f29faf090 (11 files), home-path:sha256:86706d42c5fbd252474d2153714afef1ece58aaaf3c6d6818798ab9c3f4801cf (assigned TMPDIR, +60 entries incl. 16 fixture roots), home-path:sha256:93f48363b69afcad4f43778135828bb8ba79a76b4e8812f2826a39101cd31a79 (harness copy of the suite output), and this file
needs-coordinator: no

Findings cover worktree `home-path:sha256:c501c8dd6de126ec8f3d7563e968c446a131a89596c556ba765b9bacb89718ba`,
detached at `11cc13e10074eb409e213f2c91e1d1f95cd5231a` (af2af7e74 over base effef5b1a, plus unit 2's correction), plus
677 test-only lines. Second and last pass; pass 1 is `review-1-report.md` beside this file.

## 1. `git --no-pager diff --stat`

```
 crates/edge/aep-cli/tests/store_writer_control.rs | 677 ++++++++++++++++++++++
 1 file changed, 677 insertions(+)
```

One path, a test file. No implementation file was edited, not even briefly; no probe needed a scratch copy.
Untracked and outside `git diff`: `.engineering/.aep-planning-writer.lock` and
`.engineering/.aep-planning-writer-b506bb234de6e4a30d83bb3b633415014fb70d22efa0033b9ce0b42a1ec42ad4.lock`,
both left by the killed first attempt's store read (resumption note); untouched, named per the brief.

## 2. Findings

Every row: **what was measured** (`file:line`, exit) and **what reaches it** are separate lines. Line numbers are
post-`rustfmt` (test file only; see §4).

### F1 — `closed_on_an_assertion` is permanently empty on every Eventlog plan

* **file:line** `crates/edge/aep-cli/src/planning.rs:6131` — the `asserted` list is built from `opened.journal()`,
  which is `None` for `Plan::Eventlog`.
* **what was measured** `tests/store_writer_control.rs:2129` and `:2206`, exit 101 each. A `move --evidence
  test_result=1` made on the Markdown plan is listed by the Markdown plan (`closed_on_an_assertion: ["story:one
  reached proposed on an assertion …"]`) and by nothing after migration, although the legacy line survives in the
  authority. The same move made **after** migration (`--command-identity`) is known to `explain` as
  `asserted` (the case asserts that first) and listed by nothing; `--strict` exits 0 on both.
* **what reaches it** `aep plan artifact validate --strict` on any migrated plan. Promised unconditionally by
  `website/docs/reference/cli.md:101` and by the acceptance of `story:validate-strict-refuses-what-it-reports`
  (`.engineering/planning/story/validate-strict-refuses-what-it-reports.md:39`: "exits 1 when any artifact is closed
  on an assertion"). `plan-check` (`Taskfile.yml:133`) is non-strict, so no gate goes green on this today — that is
  why it is a warning and not a blocker. The author's report §5 recorded the switch as "decided, not deferred";
  11cc13e10 then added `Opened::history_entries` (planning.rs:892) — the authority-derived reading — and applied it
  to the *other* `--strict` class (`without_an_outcome`), leaving this one on `journal()`. `CHANGELOG.md:17` says
  the class answers "as for SQLite and Postgres": a SQLite plan never held these records; a migrated plan holds
  them in its authority, and both cases show they are readable.
* **base** `git show effef5b1a:crates/edge/aep-cli/src/planning.rs` builds the list from `opened.files.as_ref()`
  (≈6036) and the base `open_plan` Eventlog arm sets `files: Some(store)` (≈1058), so at base the frozen journal
  was read and the pre-migration assertion **was** reported; the post-migration one never was. Determined by
  reading, not by running the base.
* **verdict** NEEDS-CHANGE · **origin** introduced · **severity** warning
* **named correction (not applied)** at planning.rs:6130 read `opened.history_entries()` instead of
  `opened.journal()…` — the one-line switch 11cc13e10 made for `reviews_without_an_outcome`; or, if the decision
  stands, say so at cli.md:101, in the story's acceptance and at CHANGELOG.md:17.

Red output, case alone, verbatim (`logs/case-1-asserted-pre.log`):

```
running 1 test
test strict_validate_reports_a_pre_migration_status_closed_on_an_assertion_on_an_eventlog_plan ... FAILED

thread 'strict_validate_reports_a_pre_migration_status_closed_on_an_assertion_on_an_eventlog_plan' (1675659) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:2129:5:
a status the Markdown plan reported as closed on an assertion is reported by nothing once the plan is Eventlog: before {"artifacts":1,"closed_on_an_assertion":["story:one reached proposed on an assertion rather than a record — the evidence was claimed, not held"],"files_read":1,"pre_provider":0,"problems":[],"store":"home-path:sha256:addaa2b201abbb72ed089b764e195e1a980841feeaffb3db1c599a957aa0ddaa"}, after {"artifacts":1,"files_read":1,"pre_provider":0,"problems":[],"store":"the Eventlog store home-path:sha256:03f23959952593d81a8e69b079795d8d394ac073dedce58e33c0188fa18ddafa (control-sql/control-sql-tenant/01a0c0b6-4a9a-77cd-8e51-7be187afec47)"} 

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in 2.24s
EXIT=101
```

(`logs/case-2-asserted-post.log`):

```
running 1 test
test strict_validate_reports_a_post_migration_status_closed_on_an_assertion_on_an_eventlog_plan ... FAILED

thread 'strict_validate_reports_a_post_migration_status_closed_on_an_assertion_on_an_eventlog_plan' (1675775) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:2206:5:
a governed move made on asserted evidence after the migration is reported by no class: {"artifacts":1,"files_read":1,"pre_provider":0,"problems":[],"store":"the Eventlog store home-path:sha256:6ea41a17ecacb3dbd1895b9a069cbcf2012e312c7b311f7363a10c1b6947697e (control-sql/control-sql-tenant/01a0c0b6-567b-763f-b55c-1629dd56e92e)"} 

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in 14.51s
EXIT=101
```

### F2 — `pre_provider` is `0` by construction on every Eventlog plan, and a pre-journal review falls out of both classes

* **file:line** `crates/edge/aep-cli/src/planning.rs:6097` — the `None` arm of `match opened.journal()` returns
  `pre_provider = 0`; nothing on v2 ever counts a document with no history.
* **what was measured** `tests/store_writer_control.rs:2513`, exit 101: a story with no journal line and no event
  is `pre_provider: 1` on the Markdown plan and `pre_provider: 0` after migration, while `plan artifact history
  story:two --format json` on the migrated plan answers `[]` — the authority has no readable history of it either,
  which is the class's own definition ("documents with no events at all"). `:2590`, exit 101: the same for a
  `review-result` nobody answered — Markdown `pre_provider: 1`; v2 `pre_provider: 0`, no `without_an_outcome`
  (its creation cannot be dated, F3), only `without_findings` names it, which is a class about prose. The doc
  comment at planning.rs:5727-5731 leaves such a review out of `without_an_outcome` *because* `pre_provider`
  reports it; on v2 neither does.
* **what reaches it** every Markdown plan whose documents predate its journal — the state `tests/drift.rs:165`
  models as legal — once migrated. Promised by cli.md:101 ("a document predating the event log") and the story's
  acceptance (line 39: "when any document predates the event log"). Contract item 4 (post-migration artifacts
  are not counted) holds; what the brief asked — is the pre-migration one still reported — does not. Non-strict
  `validate` prints the count only, so no gate is green on it today.
* **base** the base `match &opened.files` was `Some` on v2 → `log_findings(frozen journal)` → `drift::detect`
  counted the document (no events in the frozen journal), so at base it was reported, alongside the false
  positives this unit removed. Reading.
* **verdict** NEEDS-CHANGE · **origin** introduced · **severity** warning
* **named correction (not applied)** on v2 count the documents whose `entries_from_the_contract` is empty (case 6
  shows that is exactly the imported pre-journal set) — or declare the class closed on v2 at cli.md:101, the
  story and CHANGELOG.md:17, stating the consequence for a pre-journal review.

(`logs/case-6-pre-provider.log`):

```
running 1 test
test strict_validate_reports_a_document_predating_the_journal_on_an_eventlog_plan ... FAILED

thread 'strict_validate_reports_a_document_predating_the_journal_on_an_eventlog_plan' (1676282) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:2513:5:
assertion `left == right` failed: a document the Markdown plan reported as predating the event log is reported by nothing once the plan is Eventlog — the authority's history of it reads [] : before {"artifacts":2,"files_read":2,"pre_provider":1,"problems":[],"store":"home-path:sha256:faf46e5045918ddd72202479d8e1f010123032528b2aaa611434f4e945b2a69c"}, after {"artifacts":2,"files_read":2,"pre_provider":0,"problems":[],"store":"the Eventlog store home-path:sha256:1fb94f4b468d496c19e6e085ffa68e72ffa203cd14f1dfcd7e5d53245c9480fe (control-sql/control-sql-tenant/01a0c0b6-8f6f-7118-82df-2a012761711a)"} 
  left: Number(0)
 right: 1

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in 2.68s
EXIT=101
```

(`logs/case-7-pre-journal-review.log`):

```
running 1 test
test strict_validate_names_a_pre_journal_review_without_an_outcome_in_some_class_on_an_eventlog_plan ... FAILED

thread 'strict_validate_names_a_pre_journal_review_without_an_outcome_in_some_class_on_an_eventlog_plan' (1676367) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:2590:5:
a review the journal never recorded and nobody answered is named by no class once the plan is Eventlog — the authority's history of it reads [] : before {"artifacts":2,"files_read":2,"pre_provider":1,"problems":[],"store":"home-path:sha256:56dbef0dd6f7ce3349f885672d59ec62f5905d33b1fc568d4c7ad4165233e351","without_findings":["review-result:old states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere"]}, after {"artifacts":2,"files_read":2,"pre_provider":0,"problems":[],"store":"the Eventlog store home-path:sha256:d0e44ed507416ddc0b4d273e3cae132c7b1ed5e369669b18b7d4bd7429b94cab (control-sql/control-sql-tenant/01a0c0b6-9a1a-73f8-901c-571f5e8f10b8)","without_findings":["review-result:old states its findings as prose only — nothing can enumerate what it found, so                  the next review starts from nowhere"]} 

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in 2.93s
EXIT=101
```

### F3 — an imported document the legacy journal never recorded has an empty readable history on the Eventlog plan

* **file:line** `crates/edge/aep-cli/src/planning.rs:599` (`PlanBackend::entries_of`).
* **what was measured** in the two panics above: `plan artifact history <id> --format json` is `[]` for both
  `story:two` and `review-result:old` after migration. So `Opened::history_entries` (planning.rs:892) yields
  nothing for them, `reviews_without_an_outcome` cannot date them, and a fix for F2 routed through
  `history_entries` would have to define the class as "history empty" rather than find a creation instant.
* **what reaches it** the same migrated pre-journal documents as F2; `history` and `explain` on them answer
  nothing recorded.
* **base** `entries_of`, `entry_from_event` and `events_of` have 0 hunks in `git diff effef5b1a 11cc13e10`;
  reading.
* **verdict** CONFIRMED · **origin** pre-existing · **severity** note — not this unit's to fix; it bounds what a
  fix for F2 can do.

### F4 — the projection drift finding never names the file (judgement)

* **file:line** `crates/edge/aep-cli/src/planning.rs:6114`.
* **what was measured** `tests/store_writer_control.rs:2304` (`--nocapture`, green): one owned file edited and
  another deleted at once → exactly one finding in `drift` and one in `problems`, text `the projection <root>
  drifted from its authority: projection ownership is absent or disagrees — …`. Cases at `:2374`, `:2611`,
  `:2677` (marker removed, projection directory gone, marker edited to disown a file) produce the same sentence.
  Four faults, one prose, no path; the referral `plan store verify` answers a refusal code only.
* **what reaches it** every operator who receives the finding. Contract item 2 promises "one finding", not a
  path, so this is judgement and not a contract break.
* **verdict** CONFIRMED · **origin** introduced · **severity** note

### F5 — `CHANGELOG.md:17` and `:27` give two accounts of `without_an_outcome` on an Eventlog plan

* **file** `CHANGELOG.md:17`: the advisories "(`closed_on_an_assertion`, `without_an_outcome`, `pre_provider`)
  answer for an Eventlog plan as they do for SQLite and Postgres"; `CHANGELOG.md:27` (same commit 11cc13e10):
  the reviews-without-an-outcome class "now come[s] from the authority". The first entry was not amended when
  the second landed; for `closed_on_an_assertion` and `pre_provider` the first entry is also the only place the
  loss F1/F2 measure is written down, and it does not say a record that exists is no longer read.
* **verdict** CONFIRMED · **origin** introduced · **severity** note

### Pass-1 findings — status

| pass-1 | status | evidence |
| --- | --- | --- |
| 1 `planning.rs:5694` reviews-without-an-outcome dead on v2 | **resolved** in 11cc13e10 via `Opened::history_entries` | adopted case `:1977` green in the suite; the post-migration half (`:2228`, `--outcome-within 0`) green. Residue — a review the journal never recorded is dated by nothing — is a narrower state on different lines, filed as F2/F3, not a re-open |
| 2 `store_command.rs:1584` planted document reported by nothing | **carried** | filed as `story:unowned-document-in-eventlog-projection-is-reported`; recorded by `a_foreign_document_planted_in_an_eventlog_projection_is_reported_by_nothing_today` (`:1835`); CHANGELOG.md:20-23 corrected. Not re-raised |
| 3 `tests/store_writer_control.rs:323` TMPDIR `SUN_LEN` / symlink `receipt_conflict` | **carried**, pre-existing | not reproduced here because the brief assigned a short real directory (40 bytes): 25 lanes of the fixture ran under it, no `receipt_conflict`, no `SUN_LEN` |

## 3. Cases — `crates/edge/aep-cli/tests/store_writer_control.rs`

| line | case | asserts | now |
| --- | --- | --- | --- |
| 2119 | `strict_validate_reports_a_pre_migration_status_closed_on_an_assertion_on_an_eventlog_plan` | an assertion the Markdown plan listed is listed after migration; `--strict` exits 1 | **red** |
| 2151 | `strict_validate_reports_a_post_migration_status_closed_on_an_assertion_on_an_eventlog_plan` | a `move --evidence` after migration (which `explain` calls asserted) is listed; `--strict` exits 1 | **red** |
| 2228 | `strict_validate_reports_a_post_migration_review_without_an_outcome_on_an_eventlog_plan` | a review `new`-ed after migration, unanswered, is listed at `--outcome-within 0` | green |
| 2304 | `validate_reports_an_eventlog_projection_with_one_file_edited_and_another_deleted` | not clean; exactly one finding in `drift` and in `problems` | green |
| 2374 | `validate_reports_an_eventlog_projection_whose_ownership_marker_was_removed` | not clean; problems non-empty | green |
| 2480 | `strict_validate_reports_a_document_predating_the_journal_on_an_eventlog_plan` | Markdown `pre_provider == 1` (self-check), v2 `pre_provider == 1`, `--strict` exits 1 | **red** |
| 2536 | `strict_validate_names_a_pre_journal_review_without_an_outcome_in_some_class_on_an_eventlog_plan` | v2 names the review in `pre_provider` or `without_an_outcome` | **red** |
| 2611 | `validate_reports_an_eventlog_plan_whose_projection_directory_is_missing` | `verify` exits 1; `validate` exits 1 with a JSON answer, one drift finding, one problem | green |
| 2677 | `validate_reports_an_eventlog_projection_whose_ownership_marker_was_edited` | marker edited to disown `story/one.md` → one drift finding, exit 1 | green |

Helpers added: `closed_on_an_assertion` `:2042`, `migrated_eventlog_plan_whose_legacy_move_rested_on_an_assertion`
`:2060` (both the killed attempt's), `run_expecting_any` `:2410`, `story_one_with_one_recorded_event` `:2428`,
`a_second_legacy_story_the_journal_never_heard_of` `:2459` (mine).

Provenance and order, as it happened: the first five cases were written by the killed attempt and never run; I
read them as input and kept them — the only edit is `:2304` gaining the exact-count assertions and an
`eprintln!`. The last four are mine. All nine existed before anything was run; each was then run **alone**
(`--exact`, the compiled binary, `TMPDIR` set to the assigned scratch). Disclosed: `:2480` and `:2536` first went
red on a fixture defect of mine — the Markdown plan answered `pre_provider: 2`, because the compact journal line
the fixtures write is a journal `Entry` and not an event, and `drift::detect` counts events. That red was a typo,
not a finding; I gave `story:one` an event through `plan artifact evidence` (as the unit's own fixture does),
recompiled, and reran the two alone: red for the reason they name. `rustfmt --edition 2021` on the test file
alone then reflowed lines ≥ 2506 (`cargo fmt -p aep-cli -- --check` had flagged only that file); the four reds
were re-captured (`:2510→2513`, `:2584→2590`; `:2129`, `:2206` unchanged) and are the outputs quoted in §2.
Solo-run logs: `logs/case-1-…` to `logs/case-9-…`.

## 4. Suite — after the cases existed

```console
$ TMPDIR=home-path:sha256:854acd7f81c75efc4298f4ca5f600dd65a97d56ac643bcbc67a2d2006388e583 cargo test -p aep-cli --no-fail-fast
```

```
37 lanes; 592 passed; 4 failed; 0 ignored          (executed 596)
     Running tests/drift.rs (target/debug/deps/drift-f8e53ddaaf8836d2)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
     Running tests/store_writer_control.rs (target/debug/deps/store_writer_control-49c55abfd705e745)
running 25 tests
…
failures:
    strict_validate_names_a_pre_journal_review_without_an_outcome_in_some_class_on_an_eventlog_plan
    strict_validate_reports_a_document_predating_the_journal_on_an_eventlog_plan
    strict_validate_reports_a_post_migration_status_closed_on_an_assertion_on_an_eventlog_plan
    strict_validate_reports_a_pre_migration_status_closed_on_an_assertion_on_an_eventlog_plan

test result: FAILED. 21 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 169.00s

error: test failed, to rerun pass `-p aep-cli --test store_writer_control`
EXIT=101
```

`<before>` = 587 is the implementor's number at 11cc13e10, as the brief states; I did not run the suite before my
cases existed. `<after>` = 596 = 587 + 9, so every added case was selected. `tests/drift.rs` 6/6, unedited
(contract 3, Markdown). `cargo fmt -p aep-cli -- --check` exit 0; `cargo clippy -p aep-cli --all-targets --
-D warnings` exit 0 with 0 warnings (recheck forced by touching the test file). Postgres lanes not exercised
(`ENTITY_POSTGRES_URL` unset), so "Postgres byte-identical" rests on the author's `task check`. Full log:
`logs/suite-nofailfast.log`; clippy: `logs/clippy.log`.

## 5. Attacked and not broken

* Contract 1 — a governed move on a migrated plan reports neither drift nor a forged revision; every fixture here
  passes `validate` clean before its fault is introduced.
* Contract 2 above one fault — two owned files wrong at once: one authority-decided finding in each list, exit 1
  (`:2304`); the second fault does not mask the first.
* Ownership marker removed, or edited to disown a file: reported as drift, exit 1 (`:2374`, `:2677`); the marker's
  digests are checked against the files and the (snapshot, inventory) pair against the authority's watermarks.
* Projection directory removed whole: `validate` still loads the plan from the authority and answers one drift
  finding, exit 1; `verify` exits 1 too — the two verbs agree (`:2611`).
* Finding 1's correction — a review recorded after migration, unanswered, is listed (`:2228`): `history_entries`
  dates a post-migration `new` from the authority's `recorded_at`; the pre-migration review keeps its legacy
  instant (adopted `:1977`, 7 days). `--outcome-within` uses the right instant when the store holds one (F3 is
  the case where it holds none).
* Contract 3 for Hybrid, SQLite, Postgres — `journal()` returns `self.files` for those arms by exhaustive match
  and `history_entries` returns the journal for them or `Vec::new()` for SQLite/Postgres, so `findings` on a
  Hybrid plan reads what base read. Reasoned; not run on a Hybrid fixture (`tests/drift.rs` has none).
* Contract 5 — `verify` and `inspect` untouched; `projection_current` propagates `projection_inventory`'s error
  with `?`, and every green fault case above is red-on-swallow by construction.
* Marker-forged rollback to an older watermark — not written as a case: `before_projection_watermark`
  (`aep-planning-migration/src/durable.rs:863`) removes only the named watermark record from the *current*
  snapshot and `authority_snapshot_identity` (`:1128`) digests every record, so an older snapshot id cannot equal
  the current-minus-one identity; pass 1 §2d measured the same for a whole-directory rollback.

## 6. Paths written outside the worktree

1. `home-path:sha256:38e07107bd2f9bb9559849c037c28606dc203d3cb75fd510dee48f6f29faf090` — 11 files: `case-1-asserted-pre.log`, `case-2-asserted-post.log`,
   `case-3-review-post.log`, `case-4-two-faults.log`, `case-5-marker-removed.log`, `case-6-pre-provider.log`,
   `case-7-pre-journal-review.log`, `case-8-projection-gone.log`, `case-9-marker-edited.log`,
   `suite-nofailfast.log`, `clippy.log`.
2. `home-path:sha256:86706d42c5fbd252474d2153714afef1ece58aaaf3c6d6818798ab9c3f4801cf` — the assigned `TMPDIR`: 295 → 355 entries, 23M in total, of which
   16 fixture roots are this pass's (red cases panic before their own cleanup):
   `aep-public-writer-control-{1663903,1664042,1666492,1666500,1672460,1672574,1675658,1675774,1676281,1676366}-0`
   and `aep-public-writer-control-1691522-{4,7,9,10,11,13}`. The 8 older roots (pids 1307625–1418061) predate this
   pass.
3. `home-path:sha256:93f48363b69afcad4f43778135828bb8ba79a76b4e8812f2826a39101cd31a79`
   — the harness's copy of the background suite output.
4. This report, at
   `home-path:sha256:2483e126ee770c034933292c1a1061d732452b18ec893be4bd847c7b0bc43df7`.

Inside the worktree and outside git's view: `target/` (3.9G, left in place per the brief) and the two
`.engineering/.aep-planning-writer*.lock` files named in §1. Nothing under `/tmp`. No planning-store write, no
`aep plan artifact` verb of any kind, no git write command; worktree commands were the lease hooks only
(`session-start`, heartbeats, `session-end`) for session `review-2-validate-v2-20260920-fable`. Disk 80G free
and MemAvailable 46 GiB at start; 80G at end.

```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 6131
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the closed_on_an_assertion list is built from Opened::journal(), None on every Eventlog plan, so a status closed on an assertion before or after migration is listed by nothing and --strict exits 0 while cli.md:101 and story:validate-strict-refuses-what-it-reports promise the class unconditionally, although Opened::history_entries already reads exactly these records from the authority.
- file: crates/edge/aep-cli/src/planning.rs
  line: 6097
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: pre_provider is 0 by construction on every Eventlog plan, so a document the Markdown plan reported as predating the event log is counted by nothing after migration, and a never-answered review-result the journal never recorded falls out of both pre_provider and without_an_outcome at once.
- file: crates/edge/aep-cli/src/planning.rs
  line: 599
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: an imported document the legacy journal never recorded has an empty readable history on the Eventlog plan (history --format json answers []), so no history_entries-based class can date it and a fix for pre_provider cannot find a creation instant through that path.
- file: crates/edge/aep-cli/src/planning.rs
  line: 6114
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the projection drift finding is the same sentence for an edited file, a deleted file, a removed or edited marker and a missing projection directory, and never names the path, while the verify it refers the reader to answers a refusal code only.
- file: CHANGELOG.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: line 17 says without_an_outcome answers on an Eventlog plan as it does for SQLite and Postgres while line 27 of the same commit says it now comes from the authority, and line 17 is also the only record that closed_on_an_assertion and pre_provider stopped reading records the store still holds.
```
