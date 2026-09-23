---
format: aep.planning-md/1
id: review-result:evidence-on-hand-review-2
kind: review-result
status: active
title: 'Independent verification pass 2: history questions on a migrated Eventlog plan read the authority'
owner: aep
relations:
- reviews: task:evidence-on-hand-v2-journal
revision: 1
---
unit: task:evidence-on-hand-v2-journal — commit f3a06a0cac43917408da629f2d6544dc8cb7db25 (base af2af7e74), worktree ~/.local/state/worktree/trees/b10x/aep/ess-evolution-evidence-on-hand-review-2-20260920 detached there plus 354 test-only lines
verdict: red
cases: executed 600→603, red 3
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/ess-wave-v2/u2r2/{tmp/, build.log, alone-outcome-order.log, alone-unrelated-outcome.log, alone-same-second.log, alone-rerun.log, clippy.log, clippy2.log, fmt.log, fmt2.log, fmt3.log, base-planning.rs, suite.log} and this report (§6)
needs-coordinator: yes — findings 1 and 2 route back to the implementor or become their own story; finding 3 is the pass-1 ordering defect narrowed rather than removed and the CHANGELOG discloses the narrowing, so whether it holds the unit is not mine to say (§4)

## 1. `git --no-pager diff --stat`

```
 crates/edge/aep-cli/tests/store_writer_control.rs | 354 ++++++++++++++++++++++
 1 file changed, 354 insertions(+)
```

One path, a test file. No implementation file was edited, not even briefly; no probe needed a scratch
copy of one. Two untracked `.engineering/.aep-planning-writer*.lock` files were left in the worktree
by the suite run; per the repository invariant they are named here and left as found.

## 2. Cases — `crates/edge/aep-cli/tests/store_writer_control.rs`

All three were written before anything was run, and each was run **alone** before the suite
(`cargo test -p aep-cli --test store_writer_control -- --exact --test-threads=1 <name>`,
`TMPDIR=~/.cache/ess-wave-v2/u2r2/tmp`). Every case carries its own **Markdown control**:
the same store, the same records, asserted before the migration and again after it, so the case
cannot be read as a disagreement about what the right answer is.

| line | case | asserts | now |
| --- | --- | --- | --- |
| 3272 | `the_outcomes_of_a_review_of_two_subjects_keep_their_order_across_a_migration` | one review of `story:one` and `story:zulu`, answered on `zulu` first; `show` lists the outcomes oldest first before the migration **and after it** | **red** |
| 3328 | `an_outcome_on_an_artifact_a_review_no_longer_reviews_survives_a_migration` | an outcome recorded on `story:zulu`, then `unrelate review-result:pair reviews story:zulu`; `show` still lists it and `validate --strict` does not call the review unanswered, before the migration **and after it** | **red** |
| 3460 | `two_rounds_recorded_in_one_second_are_compared_in_the_order_they_happened` | two rounds carrying the same recorded second; `findings story:one` compares `from: zulu → to: alpha` before the migration **and after it** | **red** |

One cosmetic change was made after the first red runs and before the suite: clippy `-D warnings`
refused `both_rounds_recorded_in_one_second(root: &PathBuf)` on `ptr_arg`, so the helper takes
`&std::path::Path`. All three cases were re-run alone afterwards and are red with the same
assertions and the same messages (`…/u2r2/alone-rerun.log`).

### 2a. Red — :3299 (run alone, verbatim, first execution)

```
running 1 test
test the_outcomes_of_a_review_of_two_subjects_keep_their_order_across_a_migration ... FAILED

---- the_outcomes_of_a_review_of_two_subjects_keep_their_order_across_a_migration stdout ----

thread 'the_outcomes_of_a_review_of_two_subjects_keep_their_order_across_a_migration' (2017919) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:3295:5:
assertion `left == right` failed: `story:zulu` was answered before `story:one` and the Markdown plan said so; the Eventlog plan reads the authority one subject at a time in id order and concatenates, so the same two untouched records come back newest first, and `outcomes_of` is documented oldest first: [Object {"at": String("2026-09-20T22:32:00Z"), "outcome": String("no-op"), "reviewed": String("story:one"), "source": String("human:second")}, Object {"at": String("2026-09-20T22:32:00Z"), "outcome": String("fixed"), "reviewed": String("story:zulu"), "source": String("human:first")}]
  left: String("story:one")
 right: "story:zulu"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 30 filtered out; finished in 6.03s
EXIT=101
```

(The line moved from 3295 to 3299 when `cargo fmt` reflowed an earlier `assert_eq!` in the same
case; the re-run after formatting reports 3299 and is otherwise identical.)

### 2b. Red — :3362 (run alone, verbatim, first execution)

```
running 1 test
test an_outcome_on_an_artifact_a_review_no_longer_reviews_survives_a_migration ... FAILED

---- an_outcome_on_an_artifact_a_review_no_longer_reviews_survives_a_migration stdout ----

thread 'an_outcome_on_an_artifact_a_review_no_longer_reviews_survives_a_migration' (2021321) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:3355:5:
assertion `left == right` failed: one `review_outcome` naming `review-result:pair` is in the authority and the Markdown plan listed it; the Eventlog plan reads only the artifacts the review still `reviews`, so the record is held by the store and read by nobody: []
  left: 0
 right: 1

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 30 filtered out; finished in 5.01s
EXIT=101
```

### 2c. Red — :3498 (run alone, verbatim, first execution)

```
running 1 test
test two_rounds_recorded_in_one_second_are_compared_in_the_order_they_happened ... FAILED

---- two_rounds_recorded_in_one_second_are_compared_in_the_order_they_happened stdout ----

thread 'two_rounds_recorded_in_one_second_are_compared_in_the_order_they_happened' (2022929) panicked at crates/edge/aep-cli/tests/store_writer_control.rs:3484:5:
assertion `left == right` failed: `review-result:zulu` was recorded first and the Markdown plan said so; both rounds carry the same recorded instant, the authority keeps instants to the second, and the id tie-break puts `alpha` first — so the migration turns the ledger back to front and every finding the second round resolved is reported as new: {"artifact":"story:one","carried":[],"from":"review-result:alpha","from_reviewer":"agent:alpha","new":[],"resolved":[],"reviews":2,"to":"review-result:zulu","to_reviewer":"agent:zulu"}
  left: String("review-result:alpha")
 right: "review-result:zulu"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 31 filtered out; finished in 4.86s
EXIT=101
```

## 3. Suite

Run after the three cases above existed, nothing deselected:

```console
$ TMPDIR=~/.cache/ess-wave-v2/u2r2/tmp cargo test -p aep-cli --no-fail-fast
```

```
37 lanes, summed from the runner's `test result:` lines: 600 passed; 3 failed; 0 ignored   (executed 603)
     Running tests/store_writer_control.rs (target/debug/deps/store_writer_control-49c55abfd705e745)
failures:
    an_outcome_on_an_artifact_a_review_no_longer_reviews_survives_a_migration
    the_outcomes_of_a_review_of_two_subjects_keep_their_order_across_a_migration
    two_rounds_recorded_in_one_second_are_compared_in_the_order_they_happened
test result: FAILED. 29 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 155.40s
tests/drift.rs:           test result: ok. 6 passed; 0 failed; 0 ignored
tests/planning_cli.rs:    test result: ok. 86 passed; 0 failed; 0 ignored
tests/wave_derivation.rs: test result: ok. 13 passed; 0 failed; 0 ignored
EXIT=101
```

`<before>` = 600 is the correction's own `cases:` line; the count moved by exactly the three cases
added, and the three failures are exactly those three — the lane's 29 pre-existing cases all passed,
so nothing I wrote broke anything that was standing. `cargo fmt -p aep-cli -- --check` exit 0 and
`cargo clippy -p aep-cli --all-targets -- -D warnings` exit 0 after my additions (clippy was red once,
on `ptr_arg`, fixed in the test file). Postgres lanes not exercised (`ENTITY_POSTGRES_URL` unset).
Full log: `~/.cache/ess-wave-v2/u2r2/suite.log`.

## 4. Findings (commit f3a06a0ca, worktree above)

**1 — a migration reorders a review's outcomes: the authority is read one subject at a time, in id
order, where the journal it replaces was one file in write order.**

* **file:line** `crates/edge/aep-cli/src/planning.rs:907` — `Opened::history_of` collects the ids
  into a `BTreeSet` and `flat_map`s each artifact's history in turn, so the result is *grouped by
  id* and carries no order across artifacts. `outcomes_of` (:6770, "Every recorded outcome of one
  review, **oldest first**") returns it unsorted.
* **what was measured** `store_writer_control.rs:3299`, exit 101. One review of two subjects,
  answered on `story:zulu` first and `story:one` second. The Markdown plan lists
  `[zulu, one]` — the control assertion at :3286 passed. The same two records, untouched, come back
  `[one, zulu]` from the Eventlog plan the migration produced. Both records carry the same `at`
  (`2026-09-20T22:32:00Z`), so the authority cannot recover the order even if `outcomes_of` sorted
  by it: the instants are written to the second (`crates/plan/aep-backend-entity/src/lib.rs:72`).
* **what reaches it** `aep plan artifact show <review-result> --format json` on any migrated plan,
  no flags, for a review that declares two `reviews` edges and has an outcome recorded on each —
  `aep plan artifact new review-result … --relate reviews:a --relate reviews:b` is the ordinary
  spelling and nothing restricts the relation's cardinality (`aep-domain/src/artifact.rs:2671`
  requires at least one, not at most one). `review-value` is unaffected (it counts, it does not
  order).
* **base** at af2af7e74 `outcomes_of` read `opened.files` (`base-planning.rs:6653`), which is
  `Some` for an Eventlog plan (`:1091`), so it read the frozen journal in write order and answered
  `[zulu, one]`. This case passes at the base; the diff creates it. Read with `git show`, not run.
* **verdict** CONFIRMED · **origin** introduced · **severity** warning
* **named correction (not applied)** sort the entries `history_of` returns on an Eventlog plan
  before handing them back, or sort in `outcomes_of` — and, because the authority's instants are
  to the second, keep the id (or the per-entity ordinal) as a documented tie-break rather than
  leaving the order to `BTreeSet` iteration. Alternatively drop "oldest first" from the doc
  comment and from what `show` implies, and say the list is by subject.

**2 — a `review_outcome` on an artifact the review no longer `reviews` is held by the store and
read by nobody, and `validate --strict` then reports the answered review as unanswered.**

* **file:line** `crates/edge/aep-cli/src/planning.rs:6783` (`outcomes_of` takes its ids from
  `subjects_of(review)`, the edges the document declares **now**) and `:5561` (`review_history`
  does the same for `reviews_without_an_outcome` and `review_records`). The edge is checked when
  the record is written (`review_outcome_of`, `:6761`) and never again; `unrelate` (`:3576`) has no
  guard for an edge a `review_outcome` rests on.
* **what was measured** `store_writer_control.rs:3362`, exit 101. Outcome recorded on `story:zulu`,
  then the `reviews story:zulu` edge taken back. The Markdown plan still lists the outcome — the
  control assertion at :3349 passed. The Eventlog plan lists `[]`.
* **what reaches it** `aep plan artifact unrelate <review-result> reviews <subject>` after an
  outcome was recorded — a first-class documented verb, not a fixture hack. The review must declare
  a second `reviews` edge, because a review with none fails the graph rule
  (`aep-domain/src/artifact.rs:2671`) and `unrelate` refuses the write. **I found no workflow in
  this repository that removes a `reviews` edge**, so the path is reachable by hand and not by
  anything automated. The correction report (§7) discloses the gap; `CHANGELOG.md` and
  `website/docs/reference/cli.md:99` state the bound ("the artifacts they `reviews`") but not its
  consequence — that a `--strict` gate can report a review the store holds an answer for.
* **base** at af2af7e74 `outcomes_of` read the frozen journal, which holds the outcome line whatever
  the edges say, and `reviews_without_an_outcome` returned `Vec::new()` on an Eventlog plan
  (`base-planning.rs`, the `journal()`-is-`None` early return). Both assertions pass at the base.
  Read with `git show`, not run.
* **verdict** CONFIRMED · **origin** introduced · **severity** warning
* **named correction (not applied)** either refuse `unrelate` of a `reviews` edge that a
  `review_outcome` record names — the symmetric half of the check `evidence` already makes — or
  record the outcome against the review as well as the subject so the review's own history answers
  it, which also removes the second read per subject. Documenting it is the third option and the
  cheapest: say in `cli.md` that an outcome on an artifact a review no longer reviews is not found.

**3 — the pass-1 ordering defect is narrowed, not removed: two rounds recorded in one second are
still compared by id, and the migration reverses a pair the Markdown plan had right.**

* **file:line** `crates/edge/aep-cli/src/planning.rs:5631` — `Ordinal::At(at.epoch_millis())` from
  an instant the store writes to the second, with `stored.document.frontmatter.id` as the tie-break
  in the sort key at `:5636`.
* **what was measured** `store_writer_control.rs:3498`, exit 101. `review-result:zulu` recorded
  first, `review-result:alpha` second, both carrying `2026-02-02T02:02:02Z` in the plan's journal
  before the migration freezes it. The Markdown plan answers `from: zulu → to: alpha` — the control
  assertion at :3477 passed, because its order is the journal's line order. The Eventlog plan
  answers `from: alpha → to: zulu`: the ledger the correction's own doc comment describes as
  "calling every finding the second round resolved new".
* **what reaches it** two `review-result` records whose creation the store recorded in the same
  second, on a plan that is later migrated. I pinned the tie by rewriting the pre-migration
  journal's `at` values, so **I built this state**. It is not exotic — the two evidence records in
  finding 1's fixture landed in one second by themselves, and two consecutive CLI writes routinely
  do — but **I could not show any workflow that records two rounds of one artifact one second
  apart**, and two review rounds are normally hours or days apart. `findings --from/--to` given
  explicitly is unaffected: `review_named` (`:5646`) takes the pair as written and never consults
  the order. So: the mechanism is confirmed, the state is constructed.
* **base** at af2af7e74 `reviews_of` took its order from `opened.files`' journal line index, which
  for two pre-migration rounds is their write order, so this case passes at the base. Read with
  `git show`, not run.
* **verdict** INFEASIBLE — it holds, and the state it holds in is one I built and cannot show
  anybody reaches · **origin** introduced · **severity** note
* **named correction (not applied)** a total order rather than a second-granularity one: the
  authority's per-entity ordinal beside the instant, or a store-wide event sequence if the provider
  offers one. Failing that the present behaviour is what the CHANGELOG already says, and the honest
  move is to leave it and say in `cli.md:99` that two rounds recorded in one second are compared by
  id.

### The pass-1 findings

| pass-1 finding | status | why |
| --- | --- | --- |
| 1 — `reviews_of` takes no order from any record on an Eventlog plan; `findings` compares rounds by id | **changed** | fixed for the case pass 1 measured: `reviews_of` now reads the `Created` instant from the authority and the adopted case at :2282 is green in this suite run. The id fallback survives wherever the two instants are equal, and the authority's instants are to the second — finding 3 above. |
| 2 — `history_entries` reads every artifact's history on every `validate` | **resolved** | `history_entries` no longer exists (`grep` = 0). `Opened::history_of(ids)` reads the named ids only; a plan with no `review-result` reads nothing (`:903`). The bound is also what produces findings 1 and 2 above: the per-subject reads lose the journal's order, and the current-edges bound loses records. |

### Authority reads on the fixture (brief item)

Counted by reading, on the correction's 14-document fixture
(`~/.cache/ess-wave-v2/u2r1/tmp/aep-public-writer-control-1298446-0`: `story:one`,
`story:probe-01..11`, `review-result:zulu` and `:alpha`, both declaring `reviews: story:one` and
nothing else):

| command | authority history reads | how |
| --- | --- | --- |
| `plan artifact validate` (`--strict` or not) | **3** | the only per-artifact read inside `validate` is `reviews_without_an_outcome` → `review_history` = the two `review-result` documents ∪ their subjects = `{zulu, alpha, story:one}`, deduplicated by the `BTreeSet` at `:908` |
| `plan artifact findings story:one` | **2** | `reviews_of` → `history_of` over the reviews of the subject = `{zulu, alpha}`; `findings_ledger` makes no other per-artifact read |

Agrees with the correction's measured 14 → 3 for `validate`, and with its `findings` timing
(108.1 s against 101.1 s for a `list` that reads none). This diff adds no transaction: every change
to a read path removes reads or narrows them.

## 5. Reviewed and not faulted

* **Contract 1** — `grep -n "opened\.files\|self\.files" planning.rs` returns three lines: `missing`
  (:819), `path_of` (:827) and `journal()`'s own arm (:874). Every history reader goes through
  `journal()` or `history_of`.
* **`history_of` deduplication** — the `BTreeSet<&ArtifactId>` at `:908` means two reviews of one
  subject read that subject once, and a subject that is itself a `review-result` is read once, not
  once as a review and once as a subject. No entry is double-counted.
* **`or_insert` and the earliest `Created`** — `PlanBackend::entries_of` (:599) returns "retained
  journal entries followed by the provider's actual recorded event suffix", i.e. chronological per
  entity, and `history_of` concatenates whole per-artifact blocks, so `review_records`' and
  `reviews_without_an_outcome`'s `created.entry(..).or_insert(..)` still takes that artifact's
  *earliest* `Created`. For a migrated artifact that is the retained legacy line, which is right.
* **Pre-migration and post-migration instants are comparable** — both are wall-clock RFC-3339 to the
  second: the retained line's `at` is what `now_at_the_edge()` wrote, the event's is the seal's
  `recorded_at`. Nothing mixes `Ordinal::Line` with `Ordinal::At` either: `journal()` is `Some` or
  `None` for the whole plan.
* **`findings --from/--to` explicit** — bypasses the order entirely (`review_named`, :5646); the
  named pair is used as written. It is the workaround for finding 3 and it works.
* **`review-value --since`** — the read is not filtered by `--since`: `review_records` gathers the
  review-results and their subjects unconditionally and the cutoff filters the *rows* afterwards
  (:5995). No review is skipped by the bound because of `--since`.
* **Contract 5, the four adopted cases against the CHANGELOG** — read side by side. The CHANGELOG
  says `closed_on_an_assertion` and `pre_provider` are not computed on an Eventlog plan and
  `without_an_outcome` is computed from the authority bounded to the reviews and their subjects;
  the four cases assert exactly that, each naming
  `story:strict-advisory-classes-on-an-eventlog-plan`, none carrying `#[ignore]`, each saying which
  assertion to invert. `a_pre_journal_review_nobody_answered_is_named_by_neither_journal_class_today`
  is consistent under the new reader too: a review the authority has no `Created` for is skipped at
  `:5849` rather than guessed at.
* **Contract 6** — `tests/drift.rs` 6, `tests/planning_cli.rs` 86, `tests/wave_derivation.rs` 13,
  all green in the suite run above. SQLite and Postgres reach `history_of`'s `!matches!(Eventlog)`
  return at `:905` and answer empty.
* **Hybrid** — `journal()` returns the files for a Hybrid plan, so `history_of` reads its
  `journal.jsonl` exactly as before; none of the three findings touches it.
* **The evidence verb's own guard** — `review_outcome_of` (:6761) does refuse a `review_outcome` on
  an artifact the review does not `reviews`, so `subjects_of`'s doc comment is true at write time.
  Finding 2 is that nothing keeps it true afterwards.
* **Cost** — no new transaction in this diff; the ~100 s to open a 14-document migrated plan is the
  provider's per-transaction re-verification, `story:file-eventlog-verifies-once-per-open`, and not
  this unit's.

## 6. Paths written outside the worktree

All under the assigned scratch parent except the last:

1. `~/.cache/ess-wave-v2/u2r2/tmp/` — the assigned `TMPDIR` (created by me), 19 MB of lane
   fixtures; the three cases of §2 remove their own fixtures, the rest is the ordinary lane debris.
2. `~/.cache/ess-wave-v2/u2r2/{build.log, alone-outcome-order.log,
   alone-unrelated-outcome.log, alone-same-second.log, alone-rerun.log, clippy.log, clippy2.log,
   fmt.log, fmt2.log, fmt3.log, suite.log}` and `base-planning.rs` (the base file read out with
   `git show af2af7e74:…`, 372 KB) — one level above the assigned `tmp/`, which is a small deviation
   from the triple and is named here.
3. This report, at the path the brief names.

Nothing under `/tmp`. No planning-store write, no `aep plan artifact` write verb, no git write
command, no worktree command other than the lease hooks for session `ess-u2r2-fable-20260921`
(session-start, two heartbeats, session-end). Two untracked
`.engineering/.aep-planning-writer*.lock` files inside the worktree, left by the suite, left as
found. The pass-1 orphan shell (PID 1311658) is gone.

```findings
- file: crates/edge/aep-cli/src/planning.rs
  line: 907
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Opened::history_of concatenates per-artifact histories in BTreeSet id order, so outcomes_of — documented "oldest first" and unsorted — lists a review's outcomes by subject id; the same two untouched records read [zulu, one] on the Markdown plan and [one, zulu] on the Eventlog plan the migration produced (store_writer_control.rs:3299), and the authority's second-granularity instants cannot recover the order either.
- file: crates/edge/aep-cli/src/planning.rs
  line: 6783
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: outcomes_of and review_history read only the artifacts a review currently `reviews`, and nothing guards the edge after the record is written, so an `unrelate` leaves a review_outcome in the authority that `show` does not list and `validate --strict` answers by reporting the answered review as one nobody acted on (store_writer_control.rs:3362); the Markdown plan the store migrated from still lists it.
- file: crates/edge/aep-cli/src/planning.rs
  line: 5631
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: reviews_of orders Eventlog rounds by an instant the store writes to the second with the id as tie-break, so two rounds recorded in one second are compared back to front after a migration while the Markdown plan had them right (store_writer_control.rs:3498) — the pass-1 defect narrowed rather than removed; I pinned the tie in the fixture and found no workflow that records two rounds of one artifact one second apart.
```
