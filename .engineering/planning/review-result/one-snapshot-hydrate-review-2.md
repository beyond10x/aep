---
format: aep.planning-md/2
id: review-result:one-snapshot-hydrate-review-2
kind: review-result
status: active
title: 'Independent verification pass 2: which added lines can be deleted while the suite stays green'
relations:
- reviews: story:eventlog-store-hydrates-from-one-snapshot
revision: 2
---
unit: story:eventlog-store-hydrates-from-one-snapshot — commit 2d4a93126e3a7c052b31cbf014a9c47834fb1555, worktree home-path:sha256:c2c8654daef6e67dc34a2b50295b370090297dbdf1ff4dd9c67f946d5fc68130 (HEAD detached, no tracked non-test file modified)
verdict: red
cases: executed 8→14, red 3
origin: introduced 6, pre-existing 0, undecided 0
wrote-outside-worktree: 2 roots — this report and home-path:sha256:dc344759da85542d1d50c0340723c702f7c58387eb5acb25681033a2852d7ff2; full list in §6
needs-coordinator: no

**Five of sixteen one-line mutations to the code this unit added leave all eight shipped cases
green, and all five are fail-open.** They are the two capture lookups (`lib.rs:571`, `:587`) and
the synthesized genesis behind the second one (`:589`). Under any of them the store answers a read
for one subject with **another subject's row** — a `aep.relation` read answered with the
`aep.entity` row of the same id, or every read answered with whatever the capture holds first —
and the shipped suite does not notice, because every read it makes while a capture is held is for
a subject the capture holds, under the one kind its fixture writes. Three cases I added kill all
five (§3).

**The fall-through the story's acceptance names cannot happen.** `open` (`lib.rs:439-440`)
constructs the store and then calls `validate_legacy_boundaries()`, which retains a capture,
**before** the caller is given anything. So the `None` arms of `terminal` (`:573`) and
`subject_history` (`:596`) are unreachable in any process, and three documents say otherwise: the
story's acceptance bullet, the doc comment at `lib.rs:561-564`, and — user-visibly — the CHANGELOG
entry this unit added (*"A handle that has captured nothing still makes the single-subject read for
a single-subject question"*). The implementation report §2 justifies keeping the arms by naming
"the writer-control and invocation paths"; those paths do not use `EventlogPlanningStore` at all,
they go through `control_bridge` (`lib.rs:364`). Red case in §2.

**The staleness the brief sent me for is real and reaches nobody.** A handle that only reads
serves its opening instant forever; red through the **real bridge** over a real file authority,
green at `1a5ceda4`, both run (§2, §4 F3). I enumerated every construction site of an
`EventlogPlanningStore` in the workspace and every in-process second writer; no holder outlives
one CLI command, and the one second writer that does run while a handle is alive
(`write_file_invocation` inside `eventlog_invocation`, `planning.rs:1057`) writes only
`aep.planning-invocation` subjects, which no read path of this store asks about. **INFEASIBLE, not
CONFIRMED** — the mechanism holds, the exposure is latent.

**The implementation report's eight mutations were re-applied and all eight are still red, on the
cases it names** (§3). Area (c) and area (d) are clean and measured (§3).

---

## 1. `git --no-pager diff --stat` and `git status --porcelain` — proof of the bound

```
$ cd home-path:sha256:c2c8654daef6e67dc34a2b50295b370090297dbdf1ff4dd9c67f946d5fc68130
$ git --no-pager diff --stat
 .../src/retained_snapshot_tests.rs                 | 407 +++++++++++++++++++++
 1 file changed, 407 insertions(+)

$ git status --porcelain --untracked-files=all
 M crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs
?? crates/plan/aep-backend-eventlog/tests/retained_snapshot_review_two.rs
```

**Two paths, both test files, and the tracked one is `407 insertions(+)` with zero deletions** —
every line I wrote is appended below a marker comment, and no line the unit wrote was changed,
deleted, renamed or weakened. `crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs` is
`#[cfg(test)] mod` only (`lib.rs:12-13`); I used it rather than a second integration file because
`EventlogPlanningStore::new` is **private**, so the counting seam this unit built is reachable only
from inside the crate. No implementation file, manifest, lockfile, document or `.engineering/` path
was touched, and no `git` write command (`commit`, `add`, `stash`, `switch`, `checkout`, `branch`,
`worktree`) was run at any point. Every mutation in §3 was applied to a `git archive` extraction
under `home-path:sha256:dc344759da85542d1d50c0340723c702f7c58387eb5acb25681033a2852d7ff2`, never to this worktree.

`cargo fmt -p aep-backend-eventlog -- --check` exits **0** and
`cargo clippy -p aep-backend-eventlog --all-targets -- -D warnings` exits **0** with both of my
files present, so nothing I added moves the gate.

---

## 2. The cases I added, and the red ones verbatim, each run alone before the suite

`crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs` (appended, five cases) and
`crates/plan/aep-backend-eventlog/tests/retained_snapshot_review_two.rs` (new, one case).

| case | asserts | now | at `1a5ceda4` |
| --- | --- | --- | --- |
| `what_the_retained_capture_answers_is_what_the_provider_would_have_answered` (`:706`) | for all four kinds `hydrate` reads, for an id that exists under **two** kinds, and for three ids nothing wrote: `load`, `records`, `observations` and `events` answered from the retained capture equal the same reads answered by the provider | green | n/a (seam is new) |
| `a_capture_lookup_answers_the_kind_that_was_asked_for` (`:766`) | a `aep.relation` read is answered with the relation row, not the `aep.entity` row of the same id | green | n/a |
| `a_subject_the_capture_does_not_name_answers_as_an_unknown_subject` (`:805`) | with a capture held, an id nothing wrote answers `None` / empty, identically to the provider | green | n/a |
| **`a_store_built_the_way_open_builds_one_still_falls_through_to_the_provider`** (`:869`) | a store built the way `open` builds one still has the acceptance's single-subject fall-through | **RED** | n/a (arms are new) |
| **`a_read_only_handle_notices_a_write_made_by_another_writer`** (`:898`) | a handle that has made no write of its own answers about the authority as it is | **RED** | n/a |
| **`a_backend_that_only_reads_answers_about_the_authority_as_it_is_now`** (`tests/…:135`) | the same, through the **real** file bridge and the crate's public API only | **RED** | **green, run** |

### Red case 1, alone, before the suite

```console
$ cargo test -p aep-backend-eventlog --lib -- --exact retained_snapshot_tests::a_store_built_the_way_open_builds_one_still_falls_through_to_the_provider
   Compiling aep-backend-eventlog v0.55.0 (…/crates/plan/aep-backend-eventlog)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.57s
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)

running 1 test
test retained_snapshot_tests::a_store_built_the_way_open_builds_one_still_falls_through_to_the_provider ... FAILED

failures:

---- retained_snapshot_tests::a_store_built_the_way_open_builds_one_still_falls_through_to_the_provider stdout ----

thread 'retained_snapshot_tests::a_store_built_the_way_open_builds_one_still_falls_through_to_the_provider' (3240124) panicked at crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs:869:5:
assertion `left == right` failed: the single-subject fall-through is unreachable from `open`: validating the legacy boundaries retains a capture before the caller sees the store, so the `None` arms of `terminal` and `subject_history` cannot run in any process. before Calls { snapshots: 1, loads: 0, histories: 0, batches: 0, observations: 0 } after Calls { snapshots: 1, loads: 0, histories: 0, batches: 0, observations: 0 }
  left: 0
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    retained_snapshot_tests::a_store_built_the_way_open_builds_one_still_falls_through_to_the_provider

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-backend-eventlog --lib`
EXIT=101
```

The case does exactly what `open` does — `EventlogPlanningStore::new(provider, authority)` then
`validate_legacy_boundaries()` (`lib.rs:439-440`) — and then makes a single-subject `load`. The
counter says `loads: 0`: the read was answered from the capture the boundary validation left
behind. There is no other constructor: `new` is private and `open` is the only public path to one.

**What was measured.** `retained_snapshot_tests.rs:869`, exit 101. **What reaches it.** Nothing —
that is the finding. Three documents describe the mode this proves unreachable: the story's
`## Acceptance` third bullet, the doc comment at `lib.rs:561-564` (*"which a caller reading one row
never needs"*), and `CHANGELOG.md` (*"A handle that has captured nothing still makes the
single-subject read for a single-subject question, which costs the authority the same one capture
and materializes nothing else"*). The implementation report §2 names "the writer-control and
invocation paths" as the callers that justify the arms; `read_file_control` / `write_file_control`
(`lib.rs:260`, `:315`) build a `RecordedEventlogBridge` through `control_bridge` (`lib.rs:364`) and
never construct an `EventlogPlanningStore`, so they are not callers of this code at all.

### Red case 2, alone

```console
$ cargo test -p aep-backend-eventlog --lib -- --exact retained_snapshot_tests::a_read_only_handle_notices_a_write_made_by_another_writer
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)

running 1 test
test retained_snapshot_tests::a_read_only_handle_notices_a_write_made_by_another_writer ... FAILED

failures:

---- retained_snapshot_tests::a_read_only_handle_notices_a_write_made_by_another_writer stdout ----

thread 'retained_snapshot_tests::a_read_only_handle_notices_a_write_made_by_another_writer' (3240137) panicked at crates/plan/aep-backend-eventlog/src/retained_snapshot_tests.rs:898:5:
assertion `left == right` failed: a handle that has made no write of its own answered from the capture it took when it opened, and the authority has moved since
  left: ["01JCOUNT000000000000000001"]
 right: ["01JCOUNT000000000000000001", "01JCOUNT000000000000000002"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    retained_snapshot_tests::a_read_only_handle_notices_a_write_made_by_another_writer

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-backend-eventlog --lib`
EXIT=101
```

### Red case 3, alone — the same thing through the real bridge, and the one that settles origin

Written against the crate's **public** surface only (`prepare_file`, `provision_file`,
`write_file_control`, `open`, `EntityBackend::with_store`), so it compiles and runs unchanged at
`1a5ceda4`. `write_file_control` opens a bridge of its own, so the write it makes is a second
writer on the same file authority, exactly as a second process is. The case carries its own
control: a handle opened *after* the write sees it, so a red on the last assertion is staleness and
not a write that never landed.

```console
$ cargo test -p aep-backend-eventlog --test retained_snapshot_review_two
   Compiling aep-backend-eventlog v0.55.0 (…/crates/plan/aep-backend-eventlog)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.29s
     Running tests/retained_snapshot_review_two.rs (target/debug/deps/retained_snapshot_review_two-9bdd227e7e3dcbef)

running 1 test
test a_backend_that_only_reads_answers_about_the_authority_as_it_is_now ... FAILED

failures:

---- a_backend_that_only_reads_answers_about_the_authority_as_it_is_now stdout ----

thread 'a_backend_that_only_reads_answers_about_the_authority_as_it_is_now' (3240277) panicked at crates/plan/aep-backend-eventlog/tests/retained_snapshot_review_two.rs:135:5:
the handle answered from the capture it took when it opened; another writer has committed to this authority since and the handle has made no write of its own, so it will answer this way for as long as it lives
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    a_backend_that_only_reads_answers_about_the_authority_as_it_is_now

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s

error: test failed, to rerun pass `-p aep-backend-eventlog --test retained_snapshot_review_two`
EXIT=101
```

**Origin, run rather than read.** The same file, copied into a `git archive 1a5ceda4` extraction at
`home-path:sha256:da4c8661a20dd8a1b0abae186518bab70d4eae8ae530083a666af8c83c4a439d` (this worktree never moved):

```console
$ cd home-path:sha256:da4c8661a20dd8a1b0abae186518bab70d4eae8ae530083a666af8c83c4a439d
$ cargo test -p aep-backend-eventlog --test retained_snapshot_review_two
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
     Running tests/retained_snapshot_review_two.rs (target/debug/deps/retained_snapshot_review_two-544b80df5abfcfb8)

running 1 test
test a_backend_that_only_reads_answers_about_the_authority_as_it_is_now ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
```

Green at `1a5ceda4`, where every `load` was one fresh `bridge.load`. **Introduced**, and the same
run settles origin for red case 2, which asserts the same property against the counting seam.

---

## 3. (a)–(d), and the question this pass was sent for

### The question: which added lines can be deleted while the suite stays green

Sixteen one-line changes to the code this unit added, applied to a `git archive 2d4a931` extraction
under `home-path:sha256:c4e997dc8668da361797bf8dcd082a50eb1877471a53467f3a3ea63b2dd49ad4` — never to this worktree — each run against the
**shipped** eight-case lane with my files absent, then the survivors run again with my appended
file present and my two red-unmutated cases deselected. The tree is rewritten from a pristine copy
before every row; the `restore` row is the control.

| # | the one line changed | shipped 8 | my cases | fail-open? |
| --- | --- | --- | --- | --- |
| — | restored (control) | **exit 0**, 8 passed | exit 0 | — |
| A1 | `snapshot():543` the held-capture fast path deleted | 101 ✓ | — | — |
| A2 | `snapshot():550` the capture is not stored (report M1) | 101 ✓ | — | — |
| A3 | `retire():557` the whole retirement, every write path at once | 101 ✓ | — | — |
| A4 | `terminal():569-573` never reads the capture (report M2) | 101 ✓ | — | — |
| A5 | `terminal():573` `None => Ok(None)` (report M4) | 101 ✓ | — | — |
| **A6** | `terminal():571` the lookup matches on the **id** and ignores the kind | **exit 0** | **red** ×2 | **yes** |
| **A7** | `terminal():571` the lookup matches the **first** capture entry | **exit 0** | **red** ×3 | **yes** |
| A8 | `subject_history():585-596` never reads the capture (report M3) | 101 ✓ | — | — |
| **A9** | `subject_history():587` lookup matches on the id, ignores the kind | **exit 0** | **red** | **yes** |
| **A10** | `subject_history():587` lookup matches the first capture entry | **exit 0** | **red** ×2 | **yes** |
| **A11** | `subject_history():589-593` the synthesized genesis carries the first entry's records | **exit 0** | **red** ×2 | **yes** |
| A12 | `commit_planning_batch():740` leading `retire()` deleted (report M6) | 101 ✓ | — | — |
| A13 | `commit_planning_batch():742` trailing `retire()` deleted (report M7) | 101 ✓ | — | — |
| A14 | `Store::observe():720` `retire()` deleted (report M8) | 101 ✓ | — | — |
| A16 | `subject_history():596` fall-through → a locally synthesized genesis (report M5) | 101 ✓ | — | — |
| A15 | control: capture never consulted, i.e. `1a5ceda4`'s cost | 101 ✓ | — | — |

```console
$ bash run-mutations.sh <mutate-tree> mutations-shipped.log -- restore A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14 A16 A15
restore EXIT=0   A1 EXIT=101  A2 EXIT=101  A3 EXIT=101  A4 EXIT=101  A5 EXIT=101
A6 EXIT=0        A7 EXIT=0    A8 EXIT=101  A9 EXIT=0    A10 EXIT=0   A11 EXIT=0
A12 EXIT=101     A13 EXIT=101 A14 EXIT=101 A16 EXIT=101 A15 EXIT=101

$ bash run-mine.sh <mutate-tree> mutations-withmine.log restore A6 A7 A9 A10 A11
restore EXIT=0  A6 EXIT=101  A7 EXIT=101  A9 EXIT=101  A10 EXIT=101  A11 EXIT=101
```

**Why the shipped suite misses all five.** Both survivors' families are the *lookup into the
capture*, and the shipped fixture cannot see a wrong lookup: `CountingProvider` writes subjects
under one kind only (`STORED_AS`), so a lookup that ignores the kind finds the right row anyway;
and `a_second_read_after_a_capture_asks_the_provider_nothing` holds exactly **one** subject, so a
lookup that returns the first entry returns the right one. `opening_a_populated_authority…` holds
three subjects but asserts only *call counts* — `hydrate` would happily insert the same artifact
three times and the counter would still read 1. Nothing shipped ever reads, while a capture is
held, a subject the capture does **not** hold, which is why A11 survives too.

Each survivor is fail-open in the same direction: **the store answers about a subject other than
the one asked for, and returns it as that subject's state.** A6/A9 need an id present under two
kinds; A7/A10/A11 need nothing at all.

The three cases I added kill all five, and which one kills which is in
`home-path:sha256:acd04a6bd88fe5b1a742d173800fd62945720c56d57ed85953e5e3db9b19c111`.

### (a) Staleness on a long-lived handle — every holder, and how I found them

Found by `rg` over `crates/` for `EventlogPlanningStore::new`, `aep_backend_eventlog::open(`,
`EventlogBackend` and `PlanBackend`, then reading each site's enclosing function for its lifetime;
then `rg` for every in-process second writer (`write_file_control`, `write_file_invocation`,
`import_file_anchors`, `provision_file`, `rebuild_file_indexes`) to see whether any runs while a
handle is alive.

| holder | where | lifetime | can it serve a stale capture? |
| --- | --- | --- | --- |
| `Opened.backend: Option<PlanBackend>` | `aep-cli/src/planning.rs:846`, built at `:1203` from `Plan::open_backend` `:399` | **the only struct field that holds one**; one CLI verb, dropped at its end | no holder observed — see below |
| `DrivenPlan::load` | `aep-cli/src/planning.rs:450` | a **fresh** `open` per `PlanSource::load` call, dropped inside it; the driver calling `load` repeatedly gets a new capture each time | no |
| `Resolved::inventory` | `aep-cli/src/store_command.rs:1745` | local to one call | no |
| `FileProjectionPublisher::stage` | `aep-planning-migration/src/projection.rs:88` | local to `stage()`; `commit()` re-reads with `read_file_control`, a fresh bridge | no |
| writer-control custody (`serve`) | `aep-cli/src/store_command/writer_control.rs` | **outlives one command by design** ("keep this foreground command running") | **holds no `EventlogPlanningStore`** — it re-reads with `complete_file_snapshot` (`:428`), a fresh capture each time |
| `projection.rs:1016,1051`; `store_command.rs:2949,3223,3358,3377` | — | `#[cfg(test)]` | n/a |

There is no service or daemon crate in this repository (`crates/observe/` is the only match for
`serv`), and nothing under `crates/*/src` keeps the backend in a struct except `Opened`.

**The one place a second writer runs while a handle is alive** is `eventlog_invocation`
(`aep-cli/src/planning.rs:1057`): `opened.backend` is held across
`aep_planning_migration::execute_file_invocation`, which reserves and closes the invocation control
subject through `write_file_invocation`'s **own** bridge (`mutation.rs:95`, `:167`). That write
cannot be observed stale: it writes `aep.planning-invocation` subjects, and no read path of
`EventlogPlanningStore` asks about that kind — `hydrate` reads `aep.entity`, `aep.relation`,
`aep.audit`, `aep.applied` (`aep-backend-entity/src/lib.rs:944-1005`) and
`validate_legacy_boundary_snapshot` reads `aep.migration.LegacyRecordCoordinate` (`lib.rs:1162`).
The actual planning write in the same closure goes through `opened.backend`, which retires.

So: the mechanism is proved (§2 red case 3, green at base), and **no holder reaches it**. Verdict
`INFEASIBLE`, and the remedy if a holder ever appears is the one the brief names — a validity check
before serving, or binding the capture to one `hydrate` instead of to the handle's lifetime.

One thing worth the coordinator's attention even so: the doc comment at `lib.rs:521-528` reads as a
safety statement (*"It is never held across a write through this handle"*) and the invariant it
states is only about **this** handle. Nothing in the type, the comment or the CHANGELOG tells a
future caller that keeping one of these alive across a foreign write is unsafe, and
`EventlogBackend` is a public type.

### (b) Retirement on every write path — all eight of the report's mutations re-applied

Re-applied and each still red on the case the implementation report names: M1 = A2, M2 = A4,
M3 = A8, M4 = A5, M5 = A16, M6 = A12, M7 = A13, M8 = A14 — eight for eight, with the killed case
names matching the report's table exactly (`mutations-shipped.log`). M6 and M7 do kill different
cases, so the two sides of `commit_planning_batch`'s retirement are independently held, as claimed.
A3 (the whole of `retire()`) additionally confirms that no write path is retirement-free: deleting
it kills five cases at once.

The write surface is complete. `entity_store::Store` declares exactly `commit`, `commit_recorded`
and `observe` (`entity-store/src/lib.rs:233-277`); `PlanningStore` adds `commit_planning_batch` and
the read-only `recover_planning_receipt`. `commit` and `commit_recorded` both route into
`commit_planning_batch`, so the four writes the report names are all of them.

### (c) The fall-through and the untouched arms

`git --no-pager diff --stat 1a5ceda4 | grep -Ei "markdown|hybrid|sqlite|postgres"` → **nothing**.
Four files plus the new test module, all under `aep-backend-eventlog`, `CHANGELOG.md` and
`Cargo.lock`. The fall-through half of (c) is §2 red case 1: it answers correctly *when it runs*
(A5 and A16 are both red), and it cannot run.

### (d) The claim is pinned by a count, not a timing

`grep -nE "Instant|Duration|elapsed|SystemTime|now\(\)"` over `lib.rs` and
`retained_snapshot_tests.rs` → **nothing**. `opening_a_populated_authority_costs_one_capture_not_one_per_record`
asserts on `Calls { snapshots, loads, histories, … }`, counters the fixture increments itself; the
only number the case compares is `1`. No wall clock anywhere, and the `143.59 s → 74.17 s` figure
in the implementation report §4 is correctly presented as an observation and asserted by nothing.

---

## 4. The suite, after the cases in §2 exist

```console
$ cargo test -p aep-backend-eventlog --no-fail-fast
     Running unittests src/lib.rs (target/debug/deps/aep_backend_eventlog-f6609bd950aa75de)
running 13 tests
test result: FAILED. 11 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/retained_snapshot_review_two.rs (target/debug/deps/retained_snapshot_review_two-9bdd227e7e3dcbef)
running 1 test
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
   Doc-tests aep_backend_eventlog
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
EXIT=101
```

Full output `home-path:sha256:ab371025f5f91c0ab26682c7d84ff7aa4abe25adeda9d9404e98a8ed7cc00ee0`.

**executed 8 → 14.** `<before>` is 8, read from the `restore` row of §3 — this commit's sources with
my files absent, in the `git archive 2d4a931` extraction, run *after* my cases existed; it agrees
with the `aep_backend_eventlog (unittests src/lib.rs): executed 0 → 8` the implementing state
reported. `<after>` is 14: 13 in the lib lane and one integration case. **Red 3**, all three §2's.

Only `aep-backend-eventlog` was run. My additions are `#[cfg(test)]` inside one crate plus one
integration file in the same crate, so no other lane can see them; the workspace `2083 → 2091` is
the implementor's §6 and I did not re-run it, to keep the disk for the two extraction trees.
`cargo clippy -p aep-backend-eventlog --all-targets -- -D warnings` and
`cargo fmt -p aep-backend-eventlog -- --check` both exit 0 with my files present.

---

## 5. Findings

Covering commit `2d4a93126e3a7c052b31cbf014a9c47834fb1555` against base
`1a5ceda4de5d045cde7935e89756ca69d4aaa9c0`.

| # | where | verdict | origin | severity | what was measured / what reaches it |
| --- | --- | --- | --- | --- | --- |
| F1 | `crates/plan/aep-backend-eventlog/src/lib.rs:571` (`terminal`'s capture lookup) | CONFIRMED | introduced | warning | A6 and A7: matching the capture on the id alone, or on nothing, leaves **all 8 shipped cases green**. The store would answer a `aep.relation` read with the `aep.entity` row of the same id, or every read with the capture's first row. Reached by `hydrate`'s four `load(kind, id)` passes on any authority (`aep-backend-entity/src/lib.rs:944-1005`) — the guard is live, the coverage is not. Two cases added, green; red under both. |
| F2 | `crates/plan/aep-backend-eventlog/src/lib.rs:587` (`subject_history`'s capture lookup) | CONFIRMED | introduced | warning | A9 and A10, same shape, same result: 8/8 green. Reached by `records`, `observations`, `events_in_store_order` and `revisions_before`. One case added, green; red under both. |
| F3 | `crates/plan/aep-backend-eventlog/src/lib.rs:589` (the synthesized genesis for a subject the capture does not name) | CONFIRMED | introduced | warning | A11: making the absent-subject arm carry the first capture entry's records leaves 8/8 green — nothing shipped reads an absent subject while a capture is held. Reached by `aep plan artifact history <id nobody wrote>`. One case added, green; red under A11. |
| F4 | `crates/plan/aep-backend-eventlog/src/lib.rs:573` (and `:596`), against `CHANGELOG.md`, the story's `## Acceptance` and the doc comment at `:561-564` | NEEDS-CHANGE | introduced | warning | red case at `retained_snapshot_tests.rs:869`, exit 101: `open` (`:439-440`) retains a capture before the caller sees the store, so the single-subject fall-through cannot run in any process. The CHANGELOG entry this unit added states it as user-visible behaviour; the implementation report §2 justifies it by naming the writer-control and invocation paths, which use `control_bridge` (`:364`) and never construct this type. **What reaches the arms: nothing.** |
| F5 | `crates/plan/aep-backend-eventlog/src/lib.rs:529` (the `retained` field and its doc comment at `:521-528`) | INFEASIBLE | introduced | warning | red case at `tests/retained_snapshot_review_two.rs:135`, exit 101, through the real file bridge with a control proving the foreign write landed; **green at `1a5ceda4`, run**. A handle that only reads serves its opening instant forever. **What reaches it: nothing found** — full enumeration in §3(a); no holder outlives one CLI command, and the one in-process second writer that runs while a handle is alive writes only `aep.planning-invocation`. Latent, not live. |
| F6 | `crates/plan/aep-backend-eventlog/Cargo.toml:25-26` | CONFIRMED | introduced | note | the `[dev-dependencies] aep-domain` entry carries **no comment**. `AGENTS.md` § *Change conventions* requires "Explain every necessary dependency beside its manifest entry", and the implementation report §8 states the entry has "a comment saying why". Read from `git --no-pager diff 1a5ceda4 -- crates/plan/aep-backend-eventlog/Cargo.toml`, which is three added lines and no comment. |

**The fixes, none of which I applied.** F1–F3: no production change is needed; take the three green
cases in `retained_snapshot_tests.rs` (or equivalents) so the lookups stop being unguarded. F4: one
of two — delete the two `None` arms and the claims that describe them, or make the state reachable
(construct the store without validating the boundaries first), whichever the unit meant; the
CHANGELOG sentence must go either way. F5: a line in the doc comment saying the capture is bound to
the handle's lifetime and that a holder outliving one command must reopen — or the validity check
the brief names, if a holder is ever added. F6: one comment line.

---

## 6. Reviewed and could not fault

- **What the capture answers, against what the provider answers, for every kind and for ids that do
  not exist.** `what_the_retained_capture_answers_is_what_the_provider_would_have_answered` asks
  both paths for `load`, `records`, `observations` and `events` over `aep.entity`, `aep.relation`,
  `aep.audit` and `aep.applied`, for an id present under two kinds and for three ids nothing wrote.
  They agree on all seven subjects. The shipped `CountingProvider` cannot show this: it answers
  every `history` with genesis and builds every capture entry's history with genesis too, so both
  sides are empty by construction; mine models `adapter.rs:613-679` instead.
- **Why the two paths agree at the provider, not just in a fixture.** `capture_model` inserts a
  subject into `histories` and `terminals` together on both paths that create one
  (`adapter.rs:2161-2168` for an imported anchor, `:2295` for the terminal fold), and
  `complete_snapshot` (`:651-679`) errors rather than dropping a history whose terminal is missing.
  So the capture's key set **is** `load`'s key set; an absent subject is absent in both.
- **The read path can no longer tear between `ids` and `load`.** Before this change `ids(kind)` came
  from one capture and each `load(kind, id)` from a separate one, so a concurrent write between
  them could make `hydrate` refuse with *"listed, and `load` answers absent"*
  (`aep-backend-entity/src/lib.rs:947`). One capture removes that race. The unit did not claim it
  and no case asserts it; it is a real improvement.
- **Every write path retires, and the report's eight mutations are all still red** (§3(b)),
  including A3, which deletes the whole of `retire()` and kills five cases at once.
- **`Store::observe` reads nothing before it appends**, so retiring only behind it is correct: the
  AEP side does `Subject::new` and `context_from_parts` and nothing else (`lib.rs:707-726`), and the
  provider resolves the observed revision from its own fresh capture (`adapter.rs:1776`, `:1896`),
  never from AEP's retained one. I could not construct a read that happens first.
- **No concurrency hazard inside the handle.** All three writes take `&mut self` and every holder
  reaches the store through `EntityBackend`'s `durable: Mutex<S>`
  (`aep-backend-entity/src/lib.rs:641`), so `snapshot()`'s check-then-insert cannot interleave with
  a `retire()`. `get_or_insert` discarding a fresher capture is unreachable for the same reason.
- **`recover_planning_receipt` bypassing the capture is correct, not an oversight.**
  `CompleteStoreSnapshot` carries no `batches`, so the capture could not answer a `lookup_batch`;
  going to the provider is the only correct thing. It does cost one full capture per call, which
  the story did not claim to fix.
- **The counting fixture models the real path.** The `5 + 2N` formula reproduces the 222
  `events.jsonl` opens the profile measured on the real store, which is the strongest available
  check that the three-artifact fixture is the same shape as the migrated authority. I did not
  re-measure it.
- **Area (c) and (d)**, both measured in §3: nothing under Markdown, Hybrid, SQLite or Postgres, and
  no clock anywhere in the changed code or its cases.
- **The story's fifth acceptance bullet — CLI medians in the CHANGELOG — is still unmet**, and the
  brief lists it as settled (the sub-operator measures it on the real-store copy before the merge),
  so it is recorded here and not as a finding.

---

## 7. Every path written outside the worktree

- `home-path:sha256:8f7df02632a8581f52f7d32702d7ebb5284cc18fd0db5e321cfdb1859146ca66` — this file, as the dispatch directs
- `home-path:sha256:f36c7141068f10466e0758b000a386bfdf9e772bfa1275b7582aa5ade4520531` — the `TMPDIR` every cargo command in this session used (38 bytes, under the brief's 45)
- `home-path:sha256:dc344759da85542d1d50c0340723c702f7c58387eb5acb25681033a2852d7ff2` — `append-cases.rs`, `mutate.py`, `run-mutations.sh`,
  `run-mine.sh`, `lib.rs.pristine`, `retained_snapshot_tests.rs.orig`, `warm-build.log`,
  `new-cases-alone.log`, `case-1-alone.log`, `case-2-alone.log`, `case-3-alone.log`, `suite.log`,
  `lane-after.log`, `mutations-shipped.log`, `mutations-withmine.log`, `fmt.log`, `clippy.log`
- `home-path:sha256:112783d3824af821b7309c3e4128065c2223f5b3538b2ca219689347ddd5b62c` — a `git archive 2d4a931` extraction with its own
  `target/`, used for every mutation row in §3
- `home-path:sha256:6185b03002f892b4a9ebf5fc7cad29736690abad5686d21ceb64189fbf06d1da` — a `git archive 1a5ceda4` extraction with its own
  `target/`, used for the origin run in §2
- fixture authorities created by the integration case under `$TMPDIR`
  (`aep-retained-snapshot-review-two-<pid>-<n>`), removed by the case on success and left behind on
  the red assertion; they are inside the scratch root above

Nothing under `/tmp`. `CARGO_TARGET_DIR` never set; every build wrote to its own tree's `target/`.
No `git` write command, no `.engineering/` write, no `aep plan artifact` verb of any kind. Disk 32 G
free and MemAvailable 47 GiB throughout; no full-workspace build was run. The two scratch
extractions carry build directories the coordinator may delete; this worktree's `target/` is the
coordinator's.

---

```findings
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 571
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the retained capture's terminal lookup is pinned by nothing — matching on the id alone, or on the first entry, leaves all eight shipped cases green and lets a read for one kind be answered with another kind's row, because the shipped fixture writes one kind and its multi-subject case asserts only call counts.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 587
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the retained capture's history lookup is pinned by nothing in the same two ways, so records, observations, events_in_store_order and revisions_before could all answer about another subject with all eight shipped cases green.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 589
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the synthesized genesis for a subject the capture does not name is pinned by nothing, because no shipped case reads an absent subject while a capture is held, so an unknown artifact's history could be served another subject's records and the suite would stay green.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 573
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: open validates the legacy boundaries before the caller sees the store, which retains a capture, so the single-subject fall-through the story's acceptance, the doc comment and the user-visible CHANGELOG entry all describe cannot run in any process, and the writer-control and invocation paths the implementation report names as its callers use control_bridge and never construct this type.
- file: crates/plan/aep-backend-eventlog/src/lib.rs
  line: 529
  category: concurrency
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a handle that makes no write of its own serves the capture it took at open for as long as it lives, red through the real file bridge and green at 1a5ceda4, but no holder in the workspace outlives one CLI command and the one in-process second writer that runs while a handle is alive writes only aep.planning-invocation subjects, which no read path of this store asks about.
- file: crates/plan/aep-backend-eventlog/Cargo.toml
  line: 25
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the new aep-domain dev-dependency carries no comment, which AGENTS.md requires beside every manifest entry, and the implementation report states that it has one.
```
