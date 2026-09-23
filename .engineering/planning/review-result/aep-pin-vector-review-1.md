---
format: aep.planning-md/1
id: review-result:aep-pin-vector-review-1
kind: review-result
status: active
title: 'Independent verification pass 1: one URL one rev across both repositories, and what the CHANGELOG claims'
relations:
- reviews: task:aep-pin-verify-once-vector
revision: 2
---
# Independent verification pass 1 — task:aep-pin-verify-once-vector

```
unit: task:aep-pin-verify-once-vector — AEP 8970d5baeb0f0299556d828ad9553bc1b23abcc5 and Entity Runtime 97d6edfb5cccdadbe142399dc31da7d5d851a529, in worktree ~/.local/state/worktree/trees/b10x/aep/ess-evolution-aep-pin-review-1-20260921 (detached, no tracked file modified)
verdict: red
cases: executed 2083→2091, red 1
origin: introduced 1, pre-existing 1, undecided 1
wrote-outside-worktree: 4 roots — this report, ~/.cache/ess-wave-v2/u5r1/tmp/, ~/.cache/ess-wave-v2/u5r1/base/, ~/.cache/ess-wave-v2/u5r1/er/; full list in §7
needs-coordinator: yes
```

**The pin itself is clean, and it is clean by measurement rather than by reading.** Ten `rev` sites,
ten lockfile `source` lines, one commit per git URL in both repositories, nothing else moved in
either lock, `cargo metadata --locked --offline` exit 0 under three toolchains, and the three Entity
Runtime feature builds a split pin breaks all exit 0 at the pinned commit. The four refusals the
CHANGELOG promises a caller still happen **through the AEP edge**, measured against a real
provisioned authority with an AEP backend handle held open across the damage.

Three findings, none of them in the diff's mechanics. One is the gate's blindness to the second git
URL AEP compiles; one is a sentence of the CHANGELOG this unit wrote; one is the unit's own
acceptance statement naming an Entity Runtime commit that would reproduce the split pin it forbids.

---

## 1. The bound — `git --no-pager diff --stat` and `git status --porcelain`

```console
$ cd ~/.local/state/worktree/trees/b10x/aep/ess-evolution-aep-pin-review-1-20260921
$ git --no-pager diff --stat
$ git status --porcelain
?? .engineering/.aep-planning-writer-0384d94fbc8610ffe413df718e965d5c46a558d8c739f7b003489e94033b0beb.lock
?? .engineering/.aep-planning-writer.lock
?? crates/edge/aep-cli/tests/pin_review_aep_edge_refusals.rs
?? xtask/tests/pin_review_one_git_url_one_rev.rs
```

`diff --stat` is empty because both files I wrote are new and I ran no `git add`. `git status
--porcelain` is the proof: two paths I authored, both under a `tests/` directory.

**Two non-test paths are in that listing and I did not author them, so I name them myself rather
than leave them to be noticed.** `.engineering/.aep-planning-writer*.lock` are two zero-byte files
created by `crates/edge/aep-cli/src/planning_writer_fence.rs:16,104` when the repository's own suite
runs a planning command inside its own checkout. They appeared during my `cargo test --workspace`
runs. The same two files exist, untracked, in the implementor's integration tree
(`…/aep/ess-evolution-aep-migration-20260915/.engineering/`, dated 16 and 19 September), so this is
an ordinary side effect of running the gate in a checkout and predates this unit by days. I neither
created them deliberately nor removed them.

**No implementation file, manifest, lockfile, document or planning artifact was touched, in either
repository.** The Entity Runtime integration tree is byte-clean: `git status --porcelain` there is
empty. No `git` write command (`commit`, `add`, `stash`, `switch`, `checkout`, `branch`,
`worktree`) was run at any point, and no `aep plan artifact` verb other than `show`.

---

## 2. The cases I added, each run alone before any suite

Eight cases in two files, all written before anything was run. `TMPDIR` =
`~/.cache/ess-wave-v2/u5r1/tmp` (38 bytes, inside the 45-byte writer-control socket
budget), `CARGO_NET_OFFLINE=true`, build directory `<worktree>/target`, `CARGO_TARGET_DIR` never
set.

| case | file:line | asserts | now |
|---|---|---|---|
| A | `xtask/tests/pin_review_one_git_url_one_rev.rs:144` | every git URL resolves to exactly one commit, in the manifests **and** in `Cargo.lock`, and the two agree | green |
| B | `xtask/tests/pin_review_one_git_url_one_rev.rs:331` | every git-sourced package the lockfile compiles is one `cargo xtask deps` actually constrains | **red** |
| C | `xtask/tests/pin_review_one_git_url_one_rev.rs:241` | no workspace member hides a configuration behind a cargo feature or an optional dependency | green |
| D1 | `crates/edge/aep-cli/tests/pin_review_aep_edge_refusals.rs:203` | a committed frame damaged in place after the store was opened is refused through the open AEP handle, and the history is byte-identical afterwards | green |
| D2 | `…/pin_review_aep_edge_refusals.rs:248` | an object altered after open is refused through the open handle, history byte-identical | green |
| D3 | `…/pin_review_aep_edge_refusals.rs:290` | a consistent history that does **not extend** the one the handle observed is refused, history byte-identical, and a fresh handle accepts it | green |
| D4 | `…/pin_review_aep_edge_refusals.rs:347` | a document written into the derived Markdown projection is never read as planning authority, and `--store <projection>` is refused | green |
| D5 | `…/pin_review_aep_edge_refusals.rs:395` | the control: a healthy store is refused by none of the above under the same sequence | green |

Case A's **first** run failed, and it was my own parse bug rather than a finding: I read
`git+<url>?rev=<rev>#<commit>` without stripping the `#<commit>` fragment, so the lockfile's rev
compared unequal to the manifest's. Reported rather than hidden, verbatim:

```console
$ cargo test --locked --offline -p xtask --test pin_review_one_git_url_one_rev -- --exact every_git_url_in_the_workspace_resolves_to_exactly_one_commit
thread 'every_git_url_in_the_workspace_resolves_to_exactly_one_commit' (4040228) panicked at xtask/tests/pin_review_one_git_url_one_rev.rs:218:9:
assertion `left == right` failed: the lockfile and the manifests disagree about https://github.com/beyond10x/entity-runtime: the lock compiles 97d6edfb5cccdadbe142399dc31da7d5d851a529#97d6edfb5cccdadbe142399dc31da7d5d851a529 and the manifests ask for 97d6edfb5cccdadbe142399dc31da7d5d851a529
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.24s
```

I corrected the fragment handling in my own file and re-ran it alone, before any suite. Every case
below is the run after that correction, in the order run. Case B's log below reports its assertion
at `:319` because the third xtask case did not exist yet when it was run alone; in the final file
that assertion is at `:363`, which is where the suite run in §3 reports it.

```console
$ cargo test --locked --offline -p xtask --test pin_review_one_git_url_one_rev -- --exact every_git_url_in_the_workspace_resolves_to_exactly_one_commit
running 1 test
test every_git_url_in_the_workspace_resolves_to_exactly_one_commit ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
EXIT=0

$ cargo test --locked --offline -p xtask --test pin_review_one_git_url_one_rev -- --exact the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile
running 1 test
test the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile ... FAILED

---- the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile stdout ----

thread 'the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile' (4042021) panicked at xtask/tests/pin_review_one_git_url_one_rev.rs:319:5:
`cargo xtask deps` — the gate's `dep-check` — reads Cargo.lock for packages named ["entity-", "ess-"] and nothing else, so these git-sourced packages are compiled with no pin-uniformity rule over them and the lockfile could carry either URL at two commits with the step still exiting 0:
  https://github.com/beyond10x/eventlog → eventlog-core, eventlog-file
This is the defect the migration already hit: AEP resolved two `eventlog-core` crates after a `cargo update`, and it was caught by a person reading the lock.

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p xtask --test pin_review_one_git_url_one_rev`
EXIT=101

$ cargo test --locked --offline -p xtask --test pin_review_one_git_url_one_rev -- --exact no_workspace_member_hides_a_configuration_behind_a_feature_or_an_optional_dependency
running 1 test
test no_workspace_member_hides_a_configuration_behind_a_feature_or_an_optional_dependency ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
EXIT=0

$ cargo test --locked --offline -p aep-cli --test pin_review_aep_edge_refusals -- --exact a_committed_frame_damaged_after_the_store_was_opened_is_refused_through_the_open_handle
running 1 test
test a_committed_frame_damaged_after_the_store_was_opened_is_refused_through_the_open_handle ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 31.37s
EXIT=0

$ cargo test … -- --exact an_object_altered_after_the_store_was_opened_is_refused_through_the_open_handle
running 1 test
test an_object_altered_after_the_store_was_opened_is_refused_through_the_open_handle ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 30.97s
EXIT=0

$ cargo test … -- --exact a_history_that_does_not_extend_the_observed_manifest_is_refused_through_the_open_handle
running 1 test
test a_history_that_does_not_extend_the_observed_manifest_is_refused_through_the_open_handle has been running for over 60 seconds
test a_history_that_does_not_extend_the_observed_manifest_is_refused_through_the_open_handle ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 75.10s
EXIT=0

$ cargo test … -- --exact an_edited_eventlog_projection_is_never_read_as_planning_authority
running 1 test
test an_edited_eventlog_projection_is_never_read_as_planning_authority ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 8.35s
EXIT=0

$ cargo test … -- --exact a_healthy_store_is_not_refused_by_the_same_sequence_of_reads_and_writes
running 1 test
test a_healthy_store_is_not_refused_by_the_same_sequence_of_reads_and_writes has been running for over 60 seconds
test a_healthy_store_is_not_refused_by_the_same_sequence_of_reads_and_writes ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 77.69s
EXIT=0
```

### Case B corroborated by behaviour, not only by derivation

Case B derives the guard's coverage from `fn deps`'s own source. So that the finding does not rest
on a scrape, I also **measured** the guard. On a `git archive` extraction of the base in scratch —
`xtask/` is byte-identical at base and head (`git diff --stat 051a1934 8970d5ba -- xtask/` is
empty) — I added one test to `dep_tests` in the **scratch copy** of `xtask/src/main.rs`, never in
the worktree, handing `deps()` a lockfile that carries `eventlog-core` at two commits:

```console
$ cd ~/.cache/ess-wave-v2/u5r1/base
$ cargo test --locked --offline -p xtask --bin xtask -- --exact --nocapture dep_tests::pin_review_probe_a_lockfile_with_two_eventlog_revs_passes_dep_check
running 1 test
entity-runtime is pinned once: entity-core at 0.18.1 (git+https://github.com/beyond10x/entity-runtime?rev=97d6edf#97d6edf)
test dep_tests::pin_review_probe_a_lockfile_with_two_eventlog_revs_passes_dep_check ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s
EXIT=0
```

`deps` returns `Ok`, prints *"entity-runtime is pinned once"*, and the gate's `dep-check` step exits
0 over a lockfile compiling one git URL at two commits.

### Origin, settled by running at the base and never by moving the tree

`git archive 051a19346f5dda4096c292b5b7b738320eb63ad3 | tar -x -C ~/.cache/ess-wave-v2/u5r1/base`,
my xtask test file copied in, nothing else changed. No `checkout`, `switch`, `stash`, `branch` or
`worktree` command was run.

```console
$ cd ~/.cache/ess-wave-v2/u5r1/base && cargo test --locked --offline -p xtask --test pin_review_one_git_url_one_rev
running 3 tests
test the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile ... FAILED
test no_workspace_member_hides_a_configuration_behind_a_feature_or_an_optional_dependency ... ok
test every_git_url_in_the_workspace_resolves_to_exactly_one_commit ... ok
… uncovered: https://github.com/beyond10x/eventlog → eventlog-core, eventlog-file
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
EXIT=101
```

Case B is red at the base as well: **`pre-existing`**. Cases A and C are green at both revisions.
Cases D1–D5 are green at head; I did not run them at the base, because the base's provider is the
one units 3 and 4 already characterised and a full `aep-cli` build in scratch would have cost about
8 GiB on a disk that was at 26 GiB — stated as a limit rather than as a result.

---

## 3. The suite, after the cases existed

The gate's authoritative `test` step. `<before>` is the implementing state's own number — *"16 of 16
steps exit 0, 2,083 passed, 0 failed"* (commit message of 8970d5ba). I ran no suite before my cases
existed.

```console
$ cargo test --workspace --locked --offline --no-fail-fast
168 `test result:` lines
passed=2090 failed=1 ignored=0
test the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile ... FAILED
error: 1 target failed:
    `-p xtask --test pin_review_one_git_url_one_rev`
EXIT=101                                                      # 2083 → 2091, red 1
```

Every one of the unit's 2,083 existing cases passes. The single red lane is the one I added, and a
red suite here is the successful outcome. This run is the second of two: the first (same numbers,
same single failure) preceded two **lint-only** edits to my own files, so I re-ran the whole suite
after them rather than report a number that predates the files it describes.

```console
$ cargo clippy --workspace --all-targets --locked --offline -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s
EXIT=0
```

Both of my files are clippy-clean under the gate's own `-D warnings`. (They were not at first:
`assigning_clones`, `map_unwrap_or` and two `cast_possible_truncation` hits in my own code, fixed in
my own files and re-run.)

---

## 4. Invariant by invariant

### 1 — One git URL, one commit, in both repositories. **Holds.**

Counted, not read. AEP, at 8970d5ba:

```console
$ git grep -n 'rev = "' -- '*.toml' | sed 's/.*rev = "\([0-9a-f]*\)".*/\1/' | sort | uniq -c
      8 97d6edfb5cccdadbe142399dc31da7d5d851a529
      2 db608cd4152e6a2370e8ef0e3a562d7a829cf34a
$ git grep -cn 'rev = "' -- '*.toml'
Cargo.toml:10                              # ten sites, all in the root manifest, nowhere else
$ grep -c 'source = "git+' Cargo.lock ; grep -o 'source = "git+[^"]*"' Cargo.lock | sort | uniq -c
10
      8 …entity-runtime?rev=97d6edfb…#97d6edfb…
      2 …eventlog?rev=db608cd4…#db608cd4…
$ git grep -n -e f802eb8b -e 8b175736 -e c698923
(no output)
```

Entity Runtime, at 97d6edfb (read out of the integration tree's object store, never checked out):

```console
$ git grep -h 'rev = "' 97d6edfb -- '*.toml' | sed 's/.*rev = "\([0-9a-f]*\)".*/\1/' | sort | uniq -c
      8 db608cd4152e6a2370e8ef0e3a562d7a829cf34a
$ git show 97d6edfb:Cargo.lock | grep -o 'source = "git+[^"]*"' | sort | uniq -c
      4 …eventlog?rev=db608cd4…#db608cd4…
$ git grep -n -e f802eb8b -e 8b175736 -e c698923 97d6edfb
(no output)
```

Eight sites across four ER manifests (`entity-eventlog` 4, `entity-postgres` 2, `entity-sqlite` 1,
`entity-cli` 1), ten across AEP's one. Neither lock carries a second commit of either URL, and none
of the three superseded revs survives in any manifest or lock in either tree. Case A is the same
property, expressed so a program re-establishes it on any day.

The cross-repository half is the one that matters and it also holds: AEP's lock compiles
`eventlog-core` once, at `db608cd`, which is the rev ER's own four manifests name at the commit AEP
pins. Had AEP pinned ER at `e535aded` — the commit its acceptance statement names — ER would have
brought `c698923` and AEP's lock would carry two `eventlog-core` crates. That is finding N3.

### 2 — Nothing else moved in either lock. **Holds.**

```console
# AEP 051a1934 → 8970d5ba
$ git diff … -- Cargo.lock | grep '^[+-][^+-]' | sed 's/=.*//' | sort | uniq -c
     10 +source
     10 -source
$ git show 051a1934:Cargo.lock | grep -E '^name = |^version = |^checksum = |^dependencies|^ "' | md5sum
c51d251de29450f5de2342e5cec7919d
$ git show 8970d5ba:Cargo.lock | grep -E '^name = |^version = |^checksum = |^dependencies|^ "' | md5sum
c51d251de29450f5de2342e5cec7919d
$ git show 051a1934:Cargo.lock | grep -c '^\[\[package\]\]'   → 233
$ git show 8970d5ba:Cargo.lock | grep -c '^\[\[package\]\]'   → 233

# Entity Runtime 4ce78c11 → 97d6edfb (e535aded between them moves CHANGELOG only)
      4 +source / 4 -source ; inventory md5 130f8909313cdb8250002ab45a912754 on both sides
```

The digest covers every `name`, `version`, `checksum`, `dependencies` and dependency-edge line of
both lockfiles and is identical across each pair, so no package entry was added, removed, renamed or
re-versioned and **no dependency edge moved** — which is where the deliberately reverted
`tempfile` → `getrandom` rewrite would show. Twenty changed lines in AEP's lock; all twenty are
`source`, on exactly the ten packages.

```console
$ cargo          metadata --locked --offline --format-version 1   → EXIT 0   (AEP)
$ cargo +1.91.0  metadata --locked --offline --format-version 1   → EXIT 0   (AEP)
$ cargo +1.85.0  metadata --locked --offline --format-version 1   → EXIT 0   (AEP, the msrv floor)
$ cargo          metadata --locked --offline --format-version 1   → EXIT 0   (ER 97d6edfb, extracted)
$ cargo +1.91.0  metadata --locked --offline --format-version 1   → EXIT 0   (ER 97d6edfb, extracted)
```

### 3 — The configurations a split pin breaks. **Both halves hold.**

Entity Runtime, against a `git archive 97d6edfb` extraction in scratch (the integration tree is
read-only for me and was not written to):

```console
$ cargo check --locked --offline -p entity-cli      --features eventlog-providers   → EXIT 0
$ cargo check --locked --offline -p entity-sqlite   --features eventlog-facade      → EXIT 0
$ cargo check --locked --offline -p entity-postgres --features eventlog-facade      → EXIT 0
```

The question the brief says matters more — **is there any AEP configuration, gated or not, in which
the two eventlog crates and the eight entity crates compile together uncovered?** No, and the reason
is narrow enough to be worth pinning: AEP declares **no `[features]` table and no
`optional = true` dependency in any manifest in the tree**. Case C is that statement, derived from
the manifests. So `cargo clippy --workspace --all-targets` and `cargo test --workspace` compile every
configuration that exists, and the three crates that bind the providers — `aep-backend-eventlog`
(which depends on `entity-eventlog` **and** `eventlog-core`/`eventlog-file` directly, and is
therefore where a split pin's `E0308` would land), `aep-planning-migration` and `aep-cli` — are in
both. AEP is safe from Entity Runtime's failure mode by a property, not by a rule; case C is the
thing that will say so the day a member gains a feature.

### 4 — What the provider change means for AEP's own contract. **The AEP edge still refuses.**

Cases D1–D5, each against a real provisioned file authority under `.engineering/state`, written to
through the shipped `aep` binary, with an AEP planning backend (`aep_backend_eventlog::open`, the
same call `planning.rs:399` makes for every `plan` verb) **held open across the damage**. Every case
requires the first read to succeed, the read after the damage to refuse, and `events.jsonl` and
`manifest.json` to be byte-identical across the refusal.

| shape the brief names | case | what it measured |
|---|---|---|
| a committed frame damaged after open | D1 | refused; history byte-identical; control — a fresh backend cannot open the damaged store either, so the damage is real |
| an object altered after open | D2 | refused; history byte-identical |
| a history that does not extend the observed manifest | D3 | refused; history byte-identical; control — the rolled-back state is internally consistent and a **fresh** backend opens it and reads exactly the earlier head, so the refusal is about this handle's memory and not about corruption |
| a projection file edited under an open handle | D4 | a hand-written document in the derived Markdown projection is not read as authority, and `--store <projection>` is refused outright |
| the control | D5 | a healthy store is refused by none of it, two reads through one handle agree, and an ordinary CLI write extends both the committed bytes and the manifest |

D3 is the one that only this level can ask. The rolled-back store is bytes this authority actually
had, so nothing in the file system is wrong; only a handle that already observed the later head can
tell. That is exactly the property the once-per-open provider rests on, and it holds at the AEP
edge. The three findings units 3 and 4 opened against `eventlog-file` are not re-reported and are
not claimed fixed; what I measured is AEP's behaviour, not the provider's.

One note on shape rather than outcome: in AEP, "an open handle" is a `RecordedEventlogBridge` that
holds one `FileEventStore` on a worker thread for the life of the backend, and a CLI process opens
one per command. So D1–D3 exercise the long-lived handle a single `aep plan artifact list` uses
across its 218 transactions, which is the configuration the pin changes.

### 5 — Both CHANGELOG lines. **Entity Runtime's is honest; AEP's carries finding N2.**

Entity Runtime's entry (`CHANGELOG.md:77-88` at 97d6edfb) was corrected after unit-4's F2 and is now
explicit on both sides of the measurement: *"48 commits cost 29.97 s and 53.22 s against 74.86 s and
65.54 s before, while a read through an already-open handle is unchanged at about 240 ms … the
saving a caller sees here is on the write path."* It also now names the refusal the correction
restored. Every claim in it is one the wave measured. Nothing to report.

AEP's entry is `CHANGELOG.md:129-140`, one Unreleased `### Changed` item, correctly placed under
`## [Unreleased]` (line 10). It names both revs, states the provider's new behaviour, and closes
with *"a committed frame damaged in place after the store was opened still refuses without altering
the history"* — which cases D1, D2 and D3 measure green. One sentence is finding N2, at lines
137-138.

---

## 5. Findings

The commit is 8970d5baeb0f0299556d828ad9553bc1b23abcc5 (AEP), with Entity Runtime
97d6edfb5cccdadbe142399dc31da7d5d851a529 read as its input; the base compared against is
051a19346f5dda4096c292b5b7b738320eb63ad3.

### N1 — the gate's dependency guard reads one of the two git URLs AEP compiles

| | |
|---|---|
| **what was measured** | `xtask/tests/pin_review_one_git_url_one_rev.rs:363` — the assertion of `the_dependency_guard_constrains_every_git_sourced_package_in_the_lockfile`, defined at `:331` — exit 101, naming `eventlog-core` and `eventlog-file`; and behaviourally, `deps()` returning `Ok` over a lockfile carrying `eventlog-core` at two commits (`xtask/src/main.rs:727`, probe run in §2, exit 0) |
| **what reaches it** | `task check` step `dep-check` is `cargo xtask deps` (`Taskfile.yml:198`), and `AGENTS.md:79-80` advertises it as the enforcement: *"`cargo xtask deps` refuses more than one Entity Runtime version or pin."* The state was actually reached in this wave: AEP's `cargo update` resolved **two** `eventlog-core` crates, and the brief records that it was caught by a person reading the lock and by nothing else. `AGENTS.md:117` — *"A rule without a check is not an invariant"* — is the repository's own statement of why this matters |
| **verdict / origin** | CONFIRMED / `pre-existing` — red at 051a1934 too, and `xtask/` is byte-identical at the two commits |
| **relation to what is already filed** | `story:cross-repository-rev-uniformity-and-pin-order` asks for an **operator-side** check across a named set of checkouts; `story:one-git-url-one-rev-across-the-workspace` is filed in **entity-runtime**. Neither covers AEP's own repository-local guard, which is what this is. If the coordinator reads it as covered by those, the failing case is still the evidence for them |
| **the correction, named not applied** | in `deps`, replace the single `ENTITY_RUNTIME_PREFIX` selection with a pass over every `[[package]]` whose `source` begins `git+`, grouped by URL, refusing any URL that resolves to more than one commit. That is a strictly larger rule than the one there now, needs no new list, and turns my case green. The existing `entity-*` two-versions rule stays as it is |

### N2 — the CHANGELOG attaches the pre-pin measurement to the post-pin behaviour

| | |
|---|---|
| **what was measured** | `CHANGELOG.md:137-138` reads *"A command that makes one transaction per artifact **no longer pays** the whole store for each one — `plan artifact list` over a 64-artifact migrated plan **made** 218 transactions, 227,452 object reads and 2,863 MB of I/O for a 16 MB store."* The unit's own two other records attribute those three numbers to the **v1** provider: the commit message of 8970d5ba — *"The file provider verified the whole committed history and every bound object on every transaction … **so a command cost the whole store per artifact**: one `plan artifact list` … made 218 transactions and 227,452 object reads, and read 2,863 MB" — and `task:aep-pin-verify-once-vector` § *Why*, which adds *"and took 79.7 s"*. The em-dash clause in the CHANGELOG has no "before" marker, and the entry publishes no post-pin number at all |
| **what reaches it** | `CHANGELOG.md` is the published record of what a user sees, and AGENTS.md § *Change conventions* makes an Unreleased entry the deliverable for every user-visible change. A reader taking this pin for the performance reason reads 3,554 object reads per artifact as what they are getting, when it is what they are leaving |
| **verdict / origin** | CONFIRMED / `introduced` — the sentence is in this diff |
| **what I did not do** | I did not re-measure the post-pin cost of `plan artifact list` over a 64-artifact plan. The finding does not rest on that: it rests on the unit's own commit message and its own planning artifact stating the same three numbers as the v1 provider's cost |
| **the correction, named not applied** | mark the side, as the Entity Runtime entry for the same change does: *"… no longer pays the whole store for each one — before this pin, `plan artifact list` over a 64-artifact migrated plan made 218 transactions, 227,452 object reads and 2,863 MB of I/O for a 16 MB store."* Better, add the measured after-number beside it; if it was not measured, say the before-number and no more |

### N3 — the acceptance statement names an Entity Runtime commit that reproduces the split pin it forbids

| | |
|---|---|
| **what was measured** | `task:aep-pin-verify-once-vector` (revision 3) says, in `summary`, `## Done when` and `## Acceptance`: *"every `rev` naming `github.com/beyond10x/entity-runtime` reads `e535aded9970d41ba16349e9eccf110df29d08c3`"*. The code pins `97d6edfb`. Running it: `git grep -h 'rev = "' e535aded -- '*.toml'` in the ER tree returns **8 × `c698923038de4413e0bbba3cd91ae108607d6be9`**, while AEP's own two eventlog sites read `db608cd`. Taking the acceptance literally therefore produces two `eventlog-core` crates in AEP's lock — the exact state its own first bullet forbids (*"no other commit of either URL survives anywhere in the workspace or in `Cargo.lock`"*) and the exact state the wave already hit |
| **what reaches it** | the artifact is the unit's completion criterion and the input to the next agent who re-derives or re-checks this pin. `e535aded` was ER's head when the artifact was written; unit 4's follow-up commit `97d6edfb` moved it, and the artifact was not revised. The implementation is right and the acceptance is stale |
| **verdict / origin** | NEEDS-CHANGE / `undecided` — the planning store is a separate checkout outside the tree under review and I could not run it against a base revision. The disagreement itself is measured, not inferred |
| **the correction, named not applied** | one `aep plan artifact body` revision replacing `e535aded…` with `97d6edfb…` in `summary`, `## Done when` and `## Acceptance`, before the task is moved. I write nothing to the store |

---

## 6. Reviewed and could not fault

- The ten `rev` edits and the twenty `Cargo.lock` `source` lines: textually exact, and no package
  entry, version, checksum or dependency edge moved in either repository (§4.2, digests identical).
- Both lockfiles' consistency under every toolchain the gate uses: `cargo metadata --locked
  --offline` exits 0 under stable, 1.91.0 and 1.85.0 in AEP and under stable and 1.91.0 in ER.
- The cross-repository vector: AEP's `entity-*` pin and ER's `eventlog-*` pin name a consistent
  pair, so AEP's lock compiles a single `eventlog-core` and a single `entity-core` (§4.1).
- The three Entity Runtime feature builds no gate step enables: all three exit 0 at 97d6edfb.
- AEP's feature surface: no `[features]`, no `optional = true`, anywhere in the tree — so no
  configuration escapes `clippy --workspace --all-targets` or `test --workspace` (§4.3, case C).
- The AEP edge's refusals under an open handle, in all four shapes the brief names, plus a control
  that none of them fires on a healthy store (§4.4, cases D1–D5).
- The unit's 2,083 existing cases: unchanged and green, with no assertion weakened, skipped or
  rewritten anywhere in the diff — no source file is in it.
- The CHANGELOG entry's placement, count, both revs, the provider description and the refusal claim
  at `CHANGELOG.md:139-140`, which cases D1–D3 measure green. Entity Runtime's entry in full.
- The unit's scope discipline: `git status --porcelain` at 8970d5ba is clean and the commit touches
  three files — the root manifest, the lock and the CHANGELOG — exactly the artifact's `## Scope`.

## 7. Every path written outside the worktree

All under the assigned scratch `~/.cache/ess-wave-v2/u5r1/`; nothing in `/tmp`.

- `tmp/case-a-head.log`, `case-b-head.log`, `case-d-head.log` — the three xtask cases run alone
- `tmp/case-c1-head.log` … `case-c5-head.log` — the five AEP-edge cases run alone
- `tmp/cases-abd-base.log` — the same xtask cases at 051a1934
- `tmp/probe-dep-guard.log` — the `deps()` behavioural probe
- `tmp/build-aep-cli.log`, `tmp/suite-workspace.log`, `tmp/suite-workspace-final.log`,
  `tmp/clippy.log` — the builds and the two suite runs of §3
- `tmp/er-features.log`, `tmp/meta*.err`, `tmp/er-meta*.err`, `tmp/strace-probe.txt` — invariants 2
  and 3
- `base/` — `git archive 051a1934` plus my xtask test file, **one added test in its copy of
  `xtask/src/main.rs`** (the §2 probe; the worktree's copy was never touched), and its own `target/`
  (964 MiB). This is the origin evidence and the probe's evidence; reclaimable once recorded.
- `er/` — `git archive 97d6edfb` of Entity Runtime plus its own `target/`, for the three feature
  checks and the two `cargo metadata` runs. The ER integration tree itself was never written to.

**Outside scratch:** this report, at the brief's `report:` path.

Disk stayed between 26 and 41 GiB free throughout; the 20 GiB floor was never approached and
`MemAvailable` never fell near 16 GiB. `CARGO_TARGET_DIR` was never set. I removed no build
directory and no worktree, and touched no other session's lease.

Lease `ess-v2-u5r1-review` on the review worktree: `session-start` taken, heartbeated during the
builds, `session-end` released before returning.

**For the coordinator, beyond the findings:** cases D1–D5 cost about 4 minutes of wall time in the
`test` lane (each provisions a real authority and drives the shipped binary several times). They are
the only thing in the repository that measures the CHANGELOG's refusal sentence through AEP; keeping
them, trimming the fixture sizes, or dropping them is a call I do not get to make.

```findings
- file: xtask/src/main.rs
  line: 727
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the gate's dep-check reads Cargo.lock only for packages named `entity-*`, so the second git URL AEP compiles is under no pin-uniformity rule at all — `deps()` returns Ok over a lockfile carrying eventlog-core at two commits, which is the state this migration actually reached and which only a person reading the lock caught.
- file: CHANGELOG.md
  line: 137
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the entry says a command "no longer pays the whole store for each one" and then gives 218 transactions, 227,452 object reads and 2,863 MB with no before-marker, but the unit's own commit message and its own planning artifact both attribute those three numbers to the v1 provider; no post-pin number is published at all.
- file: ~/.local/state/worktree/trees/b10x/aep/ess-evolution-aep-migration-20260915/.engineering/planning/task/aep-pin-verify-once-vector.md
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: the acceptance statement requires every entity-runtime rev to read e535aded, but e535aded pins eventlog at c698923 while AEP pins db608cd, so satisfying the acceptance literally produces the two eventlog-core crates its own first bullet forbids; the code's 97d6edfb is right and the artifact is stale.
```
