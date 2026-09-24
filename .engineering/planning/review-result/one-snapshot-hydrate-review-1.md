---
format: aep.planning-md/1
id: review-result:one-snapshot-hydrate-review-1
kind: review-result
status: active
title: 'Independent verification pass 1: what the retained snapshot answers, and what pins it'
relations:
- reviews: story:eventlog-store-hydrates-from-one-snapshot
revision: 2
---
# Unit 7 — independent verification, pass 1

```
unit: story:eventlog-store-hydrates-from-one-snapshot — worktree ess-evolution-one-snapshot-hydrate-review-1-20260921 at 2d4a93126e3a7c052b31cbf014a9c47834fb1555, base 1a5ceda4de5d045cde7935e89756ca69d4aaa9c0
verdict: red
cases: executed 2091→2096, red 3
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 10 paths under home-path:sha256:6a958c039ef955defbc791b3f102be98edfcdd070133c09fccf69bd95206a602 plus this report (§6)
needs-coordinator: yes
```

`needs-coordinator`: the workspace-wide case count was **not** re-run here. `<before>` 2091 is the
implementing state's own reported figure; `+5` is measured on the lane I changed
(`aep-backend-eventlog`: executed 8 → 13, §3). A full `cargo test --workspace` in this worktree is a
cold build of every AEP crate while a second reviewer builds on the same disk (31G free at the end,
1.7G of it consumed by my two scratch trees), so I ran the package lane and derived the workspace
figure rather than start it. If the sub-operator wants the measured number, it is one workspace run.

## 1. `git --no-pager diff --stat` — what I touched

```
 .../plan/aep-backend-eventlog/src/retained_snapshot_tests.rs | 166 ++++++++++++++++++++-
 1 file changed, 163 insertions(+), 3 deletions(-)
```

plus one untracked path, from `git status --porcelain`:

```
 M crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs
?? crates/plan/aep-backend-eventlog/tests/
```

Both paths are test files. No implementation file, manifest, lockfile or document in the worktree
was changed, and no `git` write command was run.

The three removed lines are not a case, a case body or an assertion. Verbatim, from
`git --no-pager diff -U0`:

```
-    AppendOutcome, BatchKey, CompleteStoreSnapshot, HistoryOrigin, StoreCoverage, StoredBatch,
-                    history: genesis(subject),
-        Ok(genesis(subject))
```

They are the fixture's `use` line and the two places where `CountingProvider` hard-coded an empty
genesis history; each is replaced by a lookup into a new, **empty-by-default** map, so every
existing case sees exactly the history it saw before. That is the change area (b) needed: while
`complete_snapshot` gave every `SubjectSnapshot` an empty genesis, no case in the module could
compare a history value served out of the capture (finding 4).

## 2. The cases I added, and their output when written

Written before any suite run, each run **alone** first. `TMPDIR=home-path:sha256:3e4a9b72d160295d8f30ed16640f0ed308bb7961d3c21971022e2653fda89024`
(38 bytes), `CARGO_NET_OFFLINE=true`, `CARGO_TARGET_DIR` never set.

### 2.1 `crates/plan/aep-backend-eventlog/tests/retained_capture_freshness.rs` — new file, 2 cases

These run against the **real** file Eventlog provider through the crate's public surface
(`prepare_file` / `provision_file` / `open` / `write_file_invocation` / `EntityBackend::with_store`),
not against the unit's own counting stub. The stub cannot answer these questions: its
`complete_snapshot` and its `history` are built from the same two lines, so they cannot disagree,
and it has no notion of a second handle.

**Case 1 — `a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened` (:114). RED now.**

```console
$ cargo test -p aep-backend-eventlog --test retained_capture_freshness -- --exact a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened
    Finished `test` profile [unoptimized + debuginfo] target(s) in 16.08s
     Running tests/retained_capture_freshness.rs (target/debug/deps/retained_capture_freshness-22acabf63fcc5827)

running 1 test
test a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened ... FAILED

failures:

---- a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened stdout ----

thread 'a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened' (3199027) panicked at crates/plan/aep-backend-eventlog/tests/retained_capture_freshness.rs:114:5:
assertion `left == right` failed: a read on a handle that has written nothing must describe the authority as it is, and answered ["inv-before-open"]
  left: ["inv-before-open"]
 right: ["inv-after-open", "inv-before-open"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.43s

error: test failed, to rerun pass `-p aep-backend-eventlog --test retained_capture_freshness`
EXIT=101
```

**Case 2 — `a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened`
(:147). RED now.**

```console
$ cargo test -p aep-backend-eventlog --test retained_capture_freshness -- --exact a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running tests/retained_capture_freshness.rs (target/debug/deps/retained_capture_freshness-22acabf63fcc5827)

running 1 test
test a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened ... FAILED

failures:

---- a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened stdout ----

thread 'a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened' (3199238) panicked at crates/plan/aep-backend-eventlog/tests/retained_capture_freshness.rs:147:5:
assertion `left == right` failed: the subject was created by one record and the handle answered 0 records
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.43s

error: test failed, to rerun pass `-p aep-backend-eventlog --test retained_capture_freshness`
EXIT=101
```

**Origin run for both, at `1a5ceda4`**, from a `git archive` extraction in scratch — this worktree
was never moved:

```console
$ cd home-path:sha256:bcdcdd3a1c6640cec0011d583f8c8409a08c579d7dc0b4ade0b631a3965c4cf7   # git archive 1a5ceda4 | tar -x, plus this one test file
$ cargo test -p aep-backend-eventlog --test retained_capture_freshness
    Finished `test` profile [unoptimized + debuginfo] target(s) in 15.93s
     Running tests/retained_capture_freshness.rs (target/debug/deps/retained_capture_freshness-ebd7bf677924db49)

running 2 tests
test a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened ... ok
test a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s

EXIT=0
```

Green at base, red at `2d4a9312`: **introduced**.

### 2.2 `crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs` — 3 cases appended

**Case 3 — `every_provider_read_the_trait_declares_is_counted` (:531, assertion at :540). RED now.**

```console
$ cargo test -p aep-backend-eventlog --lib -- --exact retained_snapshot_tests::every_provider_read_the_trait_declares_is_counted
   Compiling aep-backend-eventlog v0.55.0 (…/crates/plan/aep-backend-eventlog)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.08s
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)

running 1 test
test retained_snapshot_tests::every_provider_read_the_trait_declares_is_counted ... FAILED

failures:

---- retained_snapshot_tests::every_provider_read_the_trait_declares_is_counted stdout ----

thread 'retained_snapshot_tests::every_provider_read_the_trait_declares_is_counted' (3199819) panicked at crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs:518:5:
assertion `left == right` failed: `lookup_batch` is one capture on the real bridge and the counter did not see it: Calls { snapshots: 0, loads: 0, histories: 0, batches: 0, observations: 0 }
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    retained_snapshot_tests::every_provider_read_the_trait_declares_is_counted

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-backend-eventlog --lib`
EXIT=101
```

(The `:518` in that capture is the line before the fixture change of §1 shifted it; the assertion is
`:540` in the tree as it now stands, and the later runs in §3 print `:540`.)

**Case 4 — `a_capture_answers_each_subject_with_its_own_row` (:554). GREEN now, by design.** It is
the kill-case for probe R2 (§4, finding 5): it is the assertion the unit's suite does not make.

```console
$ cargo test -p aep-backend-eventlog --lib -- --exact retained_snapshot_tests::a_capture_answers_each_subject_with_its_own_row
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)

running 1 test
test retained_snapshot_tests::a_capture_answers_each_subject_with_its_own_row ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s

EXIT=0
```

**Case 5 — `a_capture_answers_a_subject_with_the_history_it_captured` (:607). GREEN now, by design.**
Kill-case for probe R1 (§4, finding 4).

```console
$ cargo test -p aep-backend-eventlog --lib -- --exact retained_snapshot_tests::a_capture_answers_a_subject_with_the_history_it_captured
   Compiling aep-backend-eventlog v0.55.0 (…/crates/plan/aep-backend-eventlog)
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)

running 1 test
test retained_snapshot_tests::a_capture_answers_a_subject_with_the_history_it_captured ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s

EXIT=0
```

Cases 3–5 exercise a trait and a type that do not exist at `1a5ceda4`, so their origin is
`introduced` by construction, not by a base run.

## 3. The suite run

```console
$ cargo test -p aep-backend-eventlog --no-fail-fast
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)
running 11 tests
test retained_snapshot_tests::a_commit_retires_the_retained_capture ... ok
test retained_snapshot_tests::a_planning_batch_retires_the_retained_capture ... ok
test retained_snapshot_tests::a_recorded_commit_retires_the_retained_capture ... ok
test retained_snapshot_tests::a_read_before_any_capture_falls_through_to_the_provider ... ok
test retained_snapshot_tests::a_capture_answers_a_subject_with_the_history_it_captured ... ok
test retained_snapshot_tests::a_second_read_after_a_capture_asks_the_provider_nothing ... ok
test retained_snapshot_tests::a_write_revalidates_the_authority_it_is_writing_to ... ok
test retained_snapshot_tests::every_provider_read_the_trait_declares_is_counted ... FAILED
test retained_snapshot_tests::an_observation_retires_the_retained_capture ... ok
test retained_snapshot_tests::a_capture_answers_each_subject_with_its_own_row ... ok
test retained_snapshot_tests::opening_a_populated_authority_costs_one_capture_not_one_per_record ... ok
test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/retained_capture_freshness.rs (target/debug/deps/retained_capture_freshness-22acabf63fcc5827)
running 2 tests
test a_read_only_handle_sees_a_row_another_handle_appended_after_it_opened ... FAILED
test a_read_only_handle_has_the_history_of_a_row_another_handle_appended_after_it_opened ... FAILED
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
   Doc-tests aep_backend_eventlog
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
LANE_EXIT=101
```

Lane `aep-backend-eventlog`: **executed 8 → 13, red 3, exit 101**. All eight of the unit's own cases
are still green; every red is one I added. The workspace figure in the header is 2091 + 5, derived
from this measured delta — see `needs-coordinator`.

Full log: `home-path:sha256:b50f5a242007581745c2519ebd76f2651d4954a8af19ffac3fd84208ef89be0c`.

## 4. The four areas the brief named

### (a) Staleness on a long-lived handle — the holder enumeration, in full

**How I found them.** `grep -rn 'EventlogPlanningStore|aep_backend_eventlog' crates xtask` for every
reference outside the crate, then read every production (non-`#[cfg(test)]`) construction site and
followed each held value to the end of its scope. Test-module boundaries checked with
`grep -n 'mod tests'` (`aep-planning-migration/src/projection.rs:944`, `mutation.rs:760`,
`aep-cli/src/store_command.rs:2196`).

| holder | where | lifetime | reads through the retained capture? | verdict |
|---|---|---|---|---|
| `Opened.backend` = `PlanBackend::Eventlog(EventlogBackend)` | `aep-cli/src/planning.rs:398`, built in `open_plan` `:1189`/`:1203` | **one CLI verb** | yes — `PlanBackend::entries_of` `:653`/`:672` call `with_store(legacy_journal_in_store_order)` and `with_store(events_in_store_order)`, both of which now read `subject_history()`/`snapshot()`; and `EntityBackend::commits`'s pre-command `durable.load` (`aep-backend-entity/src/lib.rs:1322`) | the longest-lived production holder there is |
| `DrivenPlan` (`aep_driver::PlanSource`) | `aep-cli/src/planning.rs:438-459` | opens a **fresh** backend inside each `load()` and drops it | n/a | not a holder |
| `Serving` (the `aep serve` HTTP surface) | `aep-cli/src/serve/api.rs:28-37` | holds a `StoreLocation`, **not** a backend; every handler re-enters `planning::*_of(&serving.location, …)` | n/a | not a holder — this was the most likely long-lived candidate and it is not one |
| `store writer control hold` foreground process | `aep-cli/src/store_command/writer_control.rs:331-560` | long-lived by design | **no** — it reads via `complete_file_snapshot` (`:428`), an independent short-lived handle, then serves a Unix socket holding no store at all | not a holder |
| `FileProjectionPublisher::stage` | `aep-planning-migration/src/projection.rs:88` | one function call; the second read at `:152` is `complete_file_snapshot`, a separate handle | within one call | not a holder |
| `EventlogMutationLedger` | `aep-planning-migration/src/mutation.rs:63-175` | long-lived across `reserve` → `record_committed`* → `finish`, and it **writes through independent handles** (`write_file_invocation`) | holds no `EventlogPlanningStore`, and **has no caller anywhere in this repository** (`grep -rn MutationLedger crates` matches only its own module) | not a holder |
| test constructions | `store_command.rs:2949`, `projection.rs:1016`/`:1051` | test-only | — | not production |

**Nothing in this repository holds an `EventlogPlanningStore` across two commands.** The window that
does exist is *inside* one command: between `open()` — which already takes and retains a capture,
through `validate_legacy_boundaries()` — and any later read through that handle. Two `aep` processes
on one store at once is not exotic here: `aep-cli/src/planning_writer_fence.rs:1-6` exists precisely
because "two AEP processes that implement this protocol" may open the same store, and it fences
**writers only** — a reader takes no lock. Cases 1 and 2 build exactly that state with two bridge
handles, which is mechanically what two processes are.

What that means for severity is in findings 1 and 2: the behaviour change is confirmed and reproduced
against the real provider, and the *unbounded* version of it — a snapshot served stale forever —
needs a holder I could not show exists. The correction the brief names (a cheap validity check
before serving, or binding the capture to one `hydrate`) is still the right one and I have not
applied it.

One thing worth saying plainly because it cuts the other way: at `1a5ceda4` the *documents* every
verb answers from were **already** frozen at open, by `EntityBackend`'s in-memory hydration
(`aep-backend-entity/src/lib.rs:776-786`). What this unit newly freezes is the provider-level reads
that used to be fresh — `StateProvider::load`, `records`, `observations`, `events_in_store_order`,
`legacy_journal_in_store_order`. Against that, the capture makes the reads *mutually* consistent
where they were not, and it shortens the window by removing 221 of 222 captures. This is a real
trade, not a pure regression, and the sub-operator should weigh it as one.

### (b) Retirement on every write path — all eight mutations re-applied

Applied one at a time to a **scratch copy** of the tree (`git archive HEAD` into
`home-path:sha256:963a240b8cbf91751909642018093687cb1777ee2f8e209c94eb47e20c1f8635`), never to the worktree. Driver
`home-path:sha256:612dd5daaa4914e286864c628c4f1b0e41b231bea257cd9887290d1756842730` refuses any anchor that does not match exactly once
and byte-restores the file in a `finally`. Log
`home-path:sha256:3148823d8293359f9fa549c19a11b4aa66578fe349c441e9231fe0991aa9421b`.

| # | mutation | cases that went red | still killed? |
|---|---|---|---|
| M1 | `snapshot()` returns the capture without storing it | `opening_…`, `a_second_read_…` | yes |
| M2 | `terminal()` stops reading the retained capture | `opening_…`, `a_second_read_…` | yes |
| M3 | `subject_history()` stops reading the retained capture | `opening_…`, `a_second_read_…` | yes |
| M4 | `terminal()`'s `None => self.provider.load(subject)` → `Ok(None)` | `a_read_before_any_capture_…` | yes |
| M5 | `subject_history()`'s `None => self.provider.history(subject)` → synthesized genesis | `a_read_before_any_capture_…` | yes |
| M6 | `self.retire()` **before** `append_planning_batch` deleted | `a_write_revalidates_the_authority_it_is_writing_to` | yes |
| M7 | `self.retire()` **after** `append_planning_batch` deleted | `a_commit_…`, `a_recorded_commit_…`, `a_planning_batch_…` | yes |
| M8 | `self.retire()` after `provider.observe` deleted | `an_observation_retires_the_retained_capture` | yes |

The report's table is accurate, including that M6 and M7 kill different cases — the two retirements
around `commit_planning_batch` really are independently held. (`every_provider_read_…` is red in
every row of that log because it is red at pristine; it is my case, not a kill.)

**Two further one-line probes, of my own, both survived the unit's eight.** Full log in
`mutations.log`; the re-run with my kill-cases present is `mutations-r-killcase.log`.

- **R1** — `subject_history()` discards the captured history and answers *every* subject with an
  empty genesis (`Some(_snapshot) => Ok(SubjectHistory { …, origin: Genesis, records: vec![] })`).
  Result: `9 passed; 1 failed` — **identical to the pristine tree**. All eight of the unit's cases
  green. Finding 4.
- **R2** — `terminal()` answers every subject with the *first* row the capture holds
  (`.find(|value| …)` → `.next()`). Result: `8 passed; 2 failed`, and the two failures were my own
  two red cases. All eight of the unit's cases green, including the acceptance case. Finding 5.

With cases 4 and 5 present, R1 and R2 are both killed (`mutations-r-killcase.log`).

### (c) Fall-through and the untouched arms

- `a_read_before_any_capture_falls_through_to_the_provider` is genuine: M4 and M5 both kill it, so
  both fall-throughs are guarded. Verified independently, not taken from the report.
- `git --no-pager diff --stat 1a5ceda4 -- crates/plan/aep-backend-markdown crates/plan/aep-backend-hybrid
  crates/plan/aep-backend-sqlite crates/plan/aep-backend-postgres` prints **nothing**. The claim
  holds.
- One gap in the fall-through, and it is finding 2: the fall-through exists for a **cold** handle
  and not for a **warm** one. A subject the retained capture does not name is answered `Ok(None)`
  (`lib.rs:559`) and with a synthesized empty genesis (`lib.rs:576-583`) rather than by asking the
  provider — so a subject that exists reads as one that was never written, which is the shape
  `AGENTS.md` invariant 5 names.

### (d) The claim is pinned by a count, not a timing

Confirmed. `grep -n 'Instant|elapsed|Duration|sleep'` over
`crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs` and `src/lib.rs` matches nothing.
Every assertion in the counting cases is against `Calls`, a `#[derive(Default, Clone, Copy)]` struct
of `usize` counters incremented inside `CountingProvider`. No wall clock anywhere in the lane.

The one thing the counter does **not** cover is `lookup_batch`, which on the real bridge is
`capture_model()` — a whole capture. That is finding 3.

### The provider claim, checked at the pin rather than taken on trust

The report's central argument is that `complete_snapshot` *is* the reads it replaces. I read
`entity-runtime@8569da24`'s `crates/entity-eventlog/src/adapter.rs` and it holds, tightly:

- `complete_snapshot` (`:651-679`) iterates `model.histories` and pairs each with
  `model.terminals.get(&subject)`, erroring if one is missing; `build_committed` (`:2277-2295`)
  inserts a terminal for **every** history, and `build_import` (`:2158-2166`) inserts both. So the
  snapshot's subject set is exactly `load`'s subject set — a row cannot be present to `load` and
  absent from the capture, which was the failure mode I went looking for first.
- `history()` (`:625-648`) answers an unknown subject with `HistoryOrigin::Genesis` and no records,
  which is what `subject_history()`'s synthesized fallback reproduces. The two paths agree about an
  absent subject *at one instant*, exactly as the doc comment claims.
- `CapturedModel.histories` is a `BTreeMap`, so the linear `.find()` in `terminal()` cannot be
  ambushed by a duplicate subject.

## 5. Judgement findings

Covering worktree `ess-evolution-one-snapshot-hydrate-review-1-20260921` at
`2d4a93126e3a7c052b31cbf014a9c47834fb1555`, base `1a5ceda4`.

**1 — a read on a handle answers from the capture taken at open, and only a write through that same
handle retires it.** `crates/plan/aep-backend-eventlog/src/lib.rs:494` (the `retained` field) —
*verdict* CONFIRMED, *origin* introduced, *severity* warning.
*What was measured:* `tests/retained_capture_freshness.rs:114`, exit 101 — `ids(INVOCATION_AS)` on an
open handle answers `["inv-before-open"]` after a second handle appended `inv-after-open`; the same
case exits 0 at `1a5ceda4`. Real file provider, public API.
*What reaches it:* `Opened.backend` in `aep-cli/src/planning.rs:1189`, read after open via
`PlanBackend::entries_of` (`planning.rs:653`, `:672`) and via `EntityBackend::commits`
(`aep-backend-entity/src/lib.rs:1322`). One CLI verb long; `planning_writer_fence.rs:1-6` documents
that two AEP processes may hold the same store and fences only writers. **No holder outliving one
command exists in this repository** (§4a table), so this is a within-command race, not the unbounded
staleness the brief feared.
*Correction, not applied:* compare the provider's observed head with the snapshot's before serving,
or bind the capture to one `hydrate` and drop it when `EntityBackend::over` returns.

**2 — a warm handle answers a subject the capture does not name as never-written, instead of asking
the provider.** `crates/plan/aep-backend-eventlog/src/lib.rs:576` — *verdict* CONFIRMED, *origin*
introduced, *severity* warning.
*What was measured:* `tests/retained_capture_freshness.rs:147`, exit 101 — `records()` for a subject
that exists answers 0 records, not 1; exits 0 at `1a5ceda4`.
*What reaches it:* every read behind `subject_history()` — `HistoryProvider::records`
(`lib.rs:640`), `observations` (`:655`), `events_in_store_order` (`:1026`), `revisions_before`
(`:894`) — reached from `aep plan artifact history` through `PlanBackend::entries_of`.
*Why it is its own row and not part of 1:* the cold-handle fall-through the story promises
(`None => self.provider.history(subject)`) is deliberately *not* taken on a warm handle. The
synthesized `Genesis` turns "this handle has not seen that subject" into "that subject has no
history", which is what `AGENTS.md` invariant 5 forbids. The cheap correction is to fall through to
the provider on a capture miss rather than synthesize, which also fixes finding 1's `load` half.

**3 — `Calls::reads()`, the figure the `5 + 2N → 1` result is stated in, cannot see one of the six
provider reads the new trait declares.** `crates/plan/aep-backend-eventlog/src/lib.rs:451` (the
`lookup_batch` declaration) — *verdict* CONFIRMED, *origin* introduced, *severity* warning.
*What was measured:* `src/retained_snapshot_tests.rs:540`, exit 101 — `recover_planning_receipt`
leaves `Calls` at all-zero. On the real bridge `lookup_batch` is
`EventlogRecordedStore::lookup_batch` → `capture_model()` (`adapter.rs:627-631`), one full capture of
the authority.
*What reaches it:* `recover_planning_receipt` is called from `EntityBackend::execute`'s **replay**
branch only (`aep-backend-entity/src/lib.rs:1301`), so no open makes one and the `11 → 1` headline is
unaffected. The defect is in the measuring apparatus, not the result: the trait's own doc comment
says "each read below is one `capture_tenant`" and "a test counts them through this", and one of the
six is not counted, so a read path that later grew a receipt recovery would still measure as one.
*Correction:* give `Calls` a `lookups` field, increment it in `CountingProvider::lookup_batch`, and
add it to `reads()`.

**4 — a one-line `subject_history()` that throws the captured history away leaves all eight of the
unit's cases green.** `crates/plan/aep-backend-eventlog/src/lib.rs:576` — *verdict* NEEDS-CHANGE,
*origin* introduced, *severity* warning.
*What was measured:* probe R1 on a scratch copy — `9 passed; 1 failed`, byte-identical to the
pristine tree's result, `logs/mutations.log`. The cause is in the fixture: `CountingProvider`'s
`complete_snapshot` gave every `SubjectSnapshot` an empty genesis history, so no case in the module
ever compared a history *value* served out of the capture — only call counts.
*What reaches it:* `records`, `observations`, `events_in_store_order` and `revisions_before` all read
through that line; under R1 `aep plan artifact history` on a migrated store answers nothing and a
write's revision expectation is computed from an empty history, with the suite green.
*Correction, written not applied:* `a_capture_answers_a_subject_with_the_history_it_captured`
(`src/retained_snapshot_tests.rs:607`) kills R1. `AGENTS.md` invariant 15 is the rule it closes.

**5 — a one-line `terminal()` that answers every subject with the first row the capture holds leaves
all eight of the unit's cases green, including the acceptance case.**
`crates/plan/aep-backend-eventlog/src/lib.rs:559` — *verdict* NEEDS-CHANGE, *origin* introduced,
*severity* warning.
*What was measured:* probe R2 on a scratch copy — the unit's eight all `ok`, `logs/mutations.log`.
`opening_a_populated_authority_costs_one_capture_not_one_per_record` drops the backend without
asserting anything about it, and `a_second_read_after_a_capture_asks_the_provider_nothing` holds one
artifact, so neither can tell a correctly indexed capture from one that answers every subject with
`histories[0]`.
*What reaches it:* `StateProvider::load`, i.e. `hydrate` (`aep-backend-entity/src/lib.rs:945`) — under
R2 a three-artifact plan hydrates as one artifact and the acceptance case still passes.
*Correction, written not applied:* `a_capture_answers_each_subject_with_its_own_row`
(`src/retained_snapshot_tests.rs:554`) kills R2.

## 6. Reviewed and could not fault

- **The mechanism at the pin.** `complete_snapshot`'s subject set is provably identical to `load`'s,
  and its per-subject history identical to `history()`'s, at `8569da24` — read, not assumed (§4).
- **`Store::observe` retiring only behind the append.** The implementor deleted the leading retire
  as unguardable; `lib.rs:706-721` reads nothing through this handle before appending, so the
  deletion is right and M8 holds the remaining one.
- **Both retirements around `commit_planning_batch`.** M6 and M7 kill disjoint cases; the
  read-before-append in `append_planning_batch` (`validate_legacy_boundaries` → fresh capture, then
  `revisions_before` off that same fresh capture) really does see the current authority.
- **The double-checked `get_or_insert` in `snapshot()` (`lib.rs:531-540`).** It looks like a lost-update
  race — a capture taken before a concurrent write could be inserted after it. It is not reachable:
  every write takes `&mut self`, so no `&self` read can be in flight, and `EntityBackend` additionally
  serialises through `durable: Mutex<S>`.
- **The four commit paths.** `commit`, `commit_recorded`, `commit_planning_batch` and `observe` are
  the complete set of writes on this type; `recover_planning_receipt` is a read and correctly does
  not retire.
- **Retirement on the failure path.** `let outcome = …; self.retire(); outcome` retires whether the
  append succeeded or not.
- **Untouched arms.** Markdown, hybrid, SQLite and Postgres: empty diff (§4c).
- **No wall clock** anywhere in the lane (§4d).
- **Pins.** `Cargo.toml` still names `8569da24…` for every Entity Runtime crate and `76aad5e3…` for
  eventlog; no `rev` moved.
- **The public surface.** `EventlogPlanningStore` gains a defaulted type parameter and
  `EventlogBackend` is unchanged, so `aep-cli/src/planning.rs:594`, the only external reference,
  needs nothing. `EventlogPlanningStore::new` stays private, so the new `pub trait` widens the API
  without widening what an outside crate can construct.

## 7. Every path I wrote outside the worktree

```
home-path:sha256:2f9d191c04ce531102cd74f4fa8e6dec5a3a34f1545f30de2276bc69991ce6bb                            TMPDIR, 38 bytes, assigned
home-path:sha256:6bf2d39d321d0f6f421f3f6545dcc9f8447606eb8750602c512b7e3e433e98f6                           git archive 1a5ceda4 + one test file + its target/ (origin run)
home-path:sha256:997a6c6e4579aa78046b2790d8723e47b37f651664b3a6257083330cc90fffc2                         git archive HEAD + test files + lib.rs.pristine + its target/ (mutation probes)
home-path:sha256:612dd5daaa4914e286864c628c4f1b0e41b231bea257cd9887290d1756842730                       the mutation driver
home-path:sha256:5cc9f1b0d68e28fb37925292a3265ea4bbd2df240366a297c79c901470a23433                the fixture extension applied to the test file in §1
home-path:sha256:5de8680bcaa3bd6212aab8311d351941b7a15899efd47ea6f8e810cad4fa76a3                 case 1 alone, when written
home-path:sha256:b511c8ce72f65e52ed9a1dacbbb99ef28f342aff8bd2a12a4143054de91d4098                 case 2 alone, when written
home-path:sha256:9ceae7734ae1b51982fb83f7f856dcca14c389e9ee7126a94846b380174473fa                 case 3 alone, when written
home-path:sha256:a5aa635d1a3b9d9149c590ea09a6055668105e19552b654d1bba145b112d1edd                 case 4 alone, when written
home-path:sha256:2e0cb5742f05cf82f5e89736e8bad880d4b3dfbd011ac435f87d3303ae270269                 case 5 alone, when written
home-path:sha256:3c0b022fd44463a3b5e7aeadd0a62da561ba490109112e4595e69df8367295a0            cases 1 and 2 at 1a5ceda4
home-path:sha256:3148823d8293359f9fa549c19a11b4aa66578fe349c441e9231fe0991aa9421b              M1–M8, R1, R2
home-path:sha256:d5cd908db6fabf01c87ffc0198b08588743222a7ccd50d2779283706b632ef47   R1 and R2 with the kill-cases present
home-path:sha256:1d5430bf3230c2018a92e439f6e76ce82b8b0ead516a83f0f0da1b7b93e532c4             the lane, fail-fast
home-path:sha256:b50f5a242007581745c2519ebd76f2651d4954a8af19ffac3fd84208ef89be0c         the lane, --no-fail-fast (§3)
.ess-evolution/waves/0005-aep-migration/wave-validate-v2-20260920/unit-7-one-snapshot-hydrate/review-1-report.md   (this file)
```

Nothing under `/tmp`. `CARGO_TARGET_DIR` never set — each tree built into its own `target/`, 1.7G for
both scratch trees together. `/` had 31G free at the end. No `git` write command, no `.engineering/`
write, no `aep plan artifact` command of any kind. The worktree lease was taken with
`worktree hook session-start --session u7-review-1-20260921`, heartbeat twice, and released with
`worktree hook session-end` when this report was returned.

```findings
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 494
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a read on an opened handle answers from the capture taken at open and only a write through that same handle retires it, so a row appended by another handle is invisible for the handle's life; reproduced against the real file provider at tests/retained_capture_freshness.rs:114, green at 1a5ceda4, and no holder outliving one CLI command exists in this repository so the reachable form is a within-command race.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 576
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a warm handle answers a subject the capture does not name with a synthesized empty genesis history and Ok(None) rather than falling through to the provider, so an existing subject reads as never-written instead of unknown, which is the rewrite AGENTS.md invariant 5 forbids; tests/retained_capture_freshness.rs:147, green at 1a5ceda4.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 451
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: RecordedPlanningProvider's doc comment says each of its six calls is one capture_tenant and that a test counts them through the trait, but Calls has no field for lookup_batch, which is a whole capture_model on the real bridge, so Calls::reads() under-reports the cost of any read path that recovers a receipt.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 576
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a one-line subject_history that discards the captured history and answers every subject with an empty genesis leaves all eight of the unit's cases green because the fixture gave every SubjectSnapshot an empty genesis history, so no case compares a history value served from the capture; the kill-case is at src/retained_snapshot_tests.rs:607.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 559
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a one-line terminal that answers every subject with the first row the capture holds leaves all eight of the unit's cases green, including the acceptance case, which drops the hydrated backend without asserting anything about it; the kill-case is at src/retained_snapshot_tests.rs:554.
```
