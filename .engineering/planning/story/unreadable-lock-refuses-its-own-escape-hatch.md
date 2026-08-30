---
format: aep.planning-md/1
id: story:unreadable-lock-refuses-its-own-escape-hatch
kind: story
status: active
title: A lock file nobody can read refuses the command that exists to remove it
relations:
- decomposes: epic:reference-driver
- informed_by: story:protocol-drive-verb
- serves: vision:O3
revision: 5
---
# Story: A lock file nobody can read refuses the command that exists to remove it

## Outcome

An operator who finds `protocol drive` refused by a damaged lock file runs the command the refusal
names and gets their repository back — instead of being handed the same parse error by every verb
they try, including the one documented as the way out.

## Context

Found by the adversarial pass on `story:protocol-drive-verb`, 2026-08-30, by writing an empty
`lock.json` and running three verbs. All three answer identically:

```
error: reading .../.engineering/runs/lock.json: EOF while parsing a value at line 1 column 0
```

`run`, `run --take-lock` **and** `status`. `read_lock` (`crates/protocol-cli/src/drive.rs:1418`)
propagates the serde error with `?`, and `take_lock` (`:1444-1452`) calls it **before** it consults
`force` — so `--take-lock`, the route the refusal itself advertises, is refused by the thing it
exists to override. There is no verb that clears the state and nothing tells the operator to delete
the file.

**This repository has already solved this exact problem once, for the neighbouring file.**
`a_holder_cursor_that_will_not_parse_is_a_refusal_and_never_a_crash` holds it for `cursor.json`. The
lock is the file *every* verb reads, and it got the weaker treatment.

**How the unreadable file gets there without anybody doing anything wrong.** `take_lock` calls
`create_new` and then `write_all(body)?`. A failure between the two leaves a zero-byte `lock.json`
and **no `HeldLock` value in existence**, so the `Drop` guard added under `story:protocol-drive-verb`
cannot fire. The verb writes the file that wedges it. That half is fixed under that story; this one
is the half that makes the residue unrecoverable.

## Acceptance

- An unreadable `lock.json` — empty, truncated, or not JSON — produces a **refusal that names the
  file and says how to clear it**, not a serde diagnostic. Asserted for `run`, `run --take-lock` and
  `status` separately, because all three read it and all three are wedged today.
- `--take-lock` consults `force` **before** parsing, so a damaged lock cannot refuse the flag whose
  purpose is to override a lock.
- `status` reports a damaged lock as damaged and still prints everything it can read, rather than
  failing whole.
- The red case that exists is
  `adversary_a_lock_file_that_will_not_parse_is_a_refusal_and_never_a_parse_error`
  (`crates/protocol-cli/tests/drive_cli.rs:2621`), delivered red on purpose by the wave of
  2026-08-30 and merged in that state.

## Out of Scope

The lock's *scope* — whether it is per store or per project — which is `decision-blocker:store-lock-scope`.
A damaged lock is wedging whatever scope it turns out to have.

## Why It Is Not In `story:protocol-drive-verb`

It was found in that unit's final correction round and touches three call sites the story does not
otherwise open. Folding it in would have closed a story on a surface nobody planned to review.

## Scope

Derived 2026-08-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/protocol-cli` — cited, the story names `drive.rs` and `drive_cli.rs`, and every lock call site is in that crate
- **Files:** `crates/protocol-cli/src/drive.rs` — `read_lock` (`:1448`), `take_lock` (`:1462`, its `read_lock` at `:1510` and the `force` consult at `:1512`), `start` (`:688`, `take_lock` at `:754`), `resume` (`:820`, `take_lock` at `:889`), `status` (`:939`, `read_lock` at `:950`) — cited. **The story's own citations have drifted**: it names `:1418` and `:1444-1452`; at `3d86d5b` they are `:1448` and `:1462-1512`
- **Files:** `crates/protocol-cli/tests/drive_cli.rs` — cited, all three acceptance assertions (`run`, `run --take-lock`, `status`) are CLI cases and this is the only drive CLI suite
- **Symbols:** `read_lock`, `take_lock`, `Holder`, `LOCK_FILE` (`drive.rs:106`), `LockState::refusal`, `Liveness` — cited
- **Also likely:** `crates/aep-driver/src/lock.rs` — inferred. `LockState::refusal` (`:105`) is where every other lock refusal is worded, so a *damaged* arm could land there; but `LockState` requires `run`/`pid`/`host`, which a damaged file has none of, and `crates/aep-driver/tests/routing.rs:390` forbids the string `lock.json` anywhere in that crate's code lines — so a refusal that **names the file** cannot live there and probably stays in `drive.rs`
- **Also likely:** `crates/aep-driver/tests/routing.rs` — inferred, only if the refusal arm is added in `aep-driver`; the purity scan (`the_driver_crate_never_opens_the_lock_file_it_is_told_about`) is the test that would fail first
- **Documents:** `CHANGELOG.md` — inferred, `AGENTS.md:561` routes *what a user of the protocol sees change* there and a new refusal message is user-visible. `website/docs/reference/cli.md:195-198` only if a clearing verb or flag is added, which the acceptance does not require
- **Confidence:** **high** — the story names the defect site, the tree confirms `read_lock`'s `?` and `take_lock`'s parse-before-`force` ordering, and all four affected functions are in one file
- **Would collide with:** any unit touching `crates/protocol-cli/src/drive.rs` (its lock, `start`, `resume` or `status` paths) or `crates/protocol-cli/tests/drive_cli.rs`; secondarily anything editing `crates/aep-driver/src/lock.rs`

**Not established.** **The red case named in the acceptance is not on `main`.** `crates/protocol-cli/tests/drive_cli.rs:2621` at `3d86d5b` is inside `adversary_a_refused_second_driver_leaves_the_tree_byte_for_byte_as_it_found_it` (starts `:2627`); `adversary_a_lock_file_that_will_not_parse_is_a_refusal_and_never_a_parse_error` exists only on branch `impl/protocol-drive-verb`, at `:2695`. `b216ce7` removed it from `main` deliberately and its message names this story as the owner — so it is restored from that branch, not found in place. The story says three verbs are wedged; the tree shows **four** call sites — `resume` (`drive.rs:820`, `take_lock` at `:889`) reads the lock too and the acceptance does not name it. Where the refusal wording lands is genuinely open: the `aep-driver` purity test blocks naming the file there, but a file-less *damaged lock* arm on `LockState::refusal` is still possible. The sibling precedent is a solution, not a location — `a_holder_cursor_that_will_not_parse_is_a_refusal_and_never_a_crash` (`drive_cli.rs:1593`) is handled inside `Holder::holder_state` by `.ok()?`, a swallow rather than a refusal message.

## Left the wave of 2026-08-30 — the work is on a branch, unmerged

**Status: two full adversarial passes spent, still red on its own acceptance.** The wave's routing
budget is two attacks; after them a unit goes to a person rather than a third round. This one did.
Nothing here is lost — the branch holds every commit and every case.

| | |
|---|---|
| branch | `impl/unreadable-lock-refuses-its-own-escape-hatch` |
| commits | `ce2d1da` (first implementation), `97d7bcf` (correction after attack 1) |
| suite at `97d7bcf` | `cargo test -p protocol-cli --no-fail-fast` → 25 suites, 557 passed, exit 0 |
| suite with attack 2's cases | exit 101, 558 passed, 4 failed |
| adversarial cases on the branch | 9 — 5 from pass 1 (all now green), 4 from pass 2 (red) |

**Why it did not land: the acceptance is not met.** The story asks that an unreadable `lock.json`
stop refusing `--take-lock`, the one documented route past it. After the correction, a damaged lock
naming **another host** still refuses `--take-lock` — `Liveness::OtherHost` implies `!is_stale()`
(`drive.rs:1770`) — and the refusal it prints names only routes on a machine that is not here:
*"resume run `DRIVE-1/1` on a-machine-that-is-gone, or clear the lock there"*. There is no
lock-clearing verb: `protocol drive` is `run`, `status`, `resume` and two hidden hooks
(`website/docs/reference/cli.md:195`). The sibling arm's *"or delete the file"* is absent. So the
escape hatch is unusable for the case it most obviously exists for, and closing that is a design
decision about what `--take-lock` may do across hosts, not a defect fix.

**What the correction did land, and it is worth keeping.** `read_lock` now maps only
`ErrorKind::NotFound` to `Free`, so a file that will not read is damaged rather than free;
`LockRead::DamagedNaming(Holder)` is fed by a `salvage()` that reads the prefix `serde_json` got
through, so `--take-lock` puts the ordinary liveness question to a salvaged pid; and supersession
unlinks before `create_new` because `fs::write` on a mode-`000` file is `EACCES` even for the owner.

**Three findings attack 2 measured, each with what reaches it.**

1. **`drive.rs:1552` — `Salvage::holder` requires both `pid` and `host`, and returns `None` if
   either is missing.** Measured over all 95 byte offsets of a 94-byte `record_run` body: **18
   offsets (40–57) hold a complete, correct `pid` and no `host`**, and every one is answered
   `LockRead::Damaged` — the arm `--take-lock` replaces without asking. Verified end to end: a live
   pid, replaced, a second driver started. Reaches it: `HeldLock::record_run` (`drive.rs:1357`) is
   `fs::write`, truncate-then-write, so a crash mid-write produces exactly this. This inverts the
   crate's own rule — `aep-driver/src/lock.rs:41` says treating an unanswerable liveness question as
   dead is how two runs come to share a store.
2. **`drive.rs:1698` — `replace_damaged_lock`'s doc claims the unlink plus `create_new` is atomic.**
   It is two syscalls. Measured, 6 racers × 30 rounds: **7 rounds with 2–3 winners**, one leaving
   `DRIVE-1/1` and `DRIVE-1/2` walking the same store. The race predates the correction
   (`ce2d1da`'s `fs::write` raced the same way); the *claim* is new. Fix shape: create at a sibling
   path and `rename` over.
3. **`drive.rs:1774` — a salvaged supersession records no `stolen`.** Measured: a damaged lock
   naming a dead local pid is superseded with no `note: this run took the lock from pid …` and no
   `took_lock_from` in the cursor; the parseable control has both. Worse on `resume`:
   `aep-driver/src/run.rs:558` only overwrites on `Some`, so a resume that superseded a damaged lock
   leaves an *older* run's `took_lock_from` standing as the record of this one.

**Two findings with nothing reaching them**, recorded so they are not rediscovered as new: a
dangling symlink at `lock.json` reads `NotFound` and creates `AlreadyExists`, so `status` says
`free` and `--take-lock` says *"released while it was being read"* for ever (`drive.rs:1523`) — no
code in this tree creates one; and a salvaged empty `host` renders *"pid 1 on , another host"*
(`drive.rs:1810`) — `host()` never returns empty.

**What attack 2 could not break.** All 95 truncation offsets: `salvage` never names a partial pid.
`[]`, `123`, `"a lock"` and a nested object name nobody. pid `0`, `4294967295`, `-1`, `"12"` and `1`
all behave as documented. All 5 of pass 1's cases and the correction's own case still pass.

**Owed if it is picked up again:** a `CHANGELOG.md` line — `AGENTS.md:669-672` requires one in the
same commit for a rule that now refuses what it used to allow, and this change adds three
user-visible strings.
