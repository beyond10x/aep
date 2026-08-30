---
format: aep.planning-md/1
id: decision-blocker:store-lock-scope
kind: decision-blocker
status: open
title: Is the store lock per store, or per project?
relations:
- blocks: story:protocol-drive-verb
revision: 2
---
# Blocker: is the store lock per store, or per project?

## What Is Blocked

`story:protocol-drive-verb`'s **first acceptance line**, and with it the story's terminal move:

> The lock lives at one fixed path per **store**, taken with `create_new` before a run id is
> allocated — two invocations racing cannot both succeed.

Everything else in the story is now closed or in correction. This one line cannot be closed by an
implementor because the two ways to make it true are a behaviour change and a contract change, and
choosing between them is not an engineering detail.

## What Would Clear It

The driver owner picking one of two, and it being recorded:

**A. The lock follows the store.** `runs_directory` is derived from the store rather than from the
project, so `--store` moves `lock.json` with it. The acceptance line becomes true as written. This
is a behaviour change to a shipped verb: a run started before the change and one started after do
not see each other's lock, so the migration is *finish every in-flight run first*.

**B. The story stops saying "store" and says "project".** No code moves. The hole the adversary
demonstrated stays open and becomes documented behaviour: **two projects pointed at one store take
two different locks and both runs proceed.** The refusal that exists is then a per-project
courtesy, not a mutual-exclusion guarantee, and anything relying on it for correctness is relying
on a coincidence of directory layout.

Recommended: **A.** The concrete failure under B is two `protocol drive` runs walking one set of
documents at the same time — the adversary's case observed a transition read out of the *holder's*
store after the second run's own had been deleted. A lock whose scope is not the thing it protects
is not a lock.

## Who Can Clear It

The driver owner. It is a one-line decision plus a recorded rationale; the build behind either
answer is small.

## What We Are Doing Meanwhile

`story:protocol-drive-verb` stays `active` and the other four findings from its adversarial pass are
in correction round 1 — the `Drop` on `HeldLock` (F2), the two-engines test that never reaches
`allocate_run` (F4), the vacuous host substring (F5) and the test that pre-excuses its own red (F6).
The unit does not merge until this is answered, because merging it would close a story on an
acceptance line the tree contradicts.

## Evidence

- `crates/protocol-cli/src/drive.rs:683` takes the lock at `runs_directory(&inputs.project)`;
  `:546` selects the store from `--store`. The two are independent.
- **Two** doc comments assert the option nobody has chosen: `crates/aep-driver/src/lock.rs:9` and
  `crates/protocol-cli/src/drive.rs:92` both say *"one fixed path per store"*. Whichever way this
  decision goes, **both** are part of it — a resolution that amends the story and leaves these
  standing puts the losing option back into the tree as documentation.
- Red case: `adversary_a_second_drive_over_one_store_from_another_project_is_refused_and_writes_nothing`,
  `crates/protocol-cli/tests/drive_cli.rs:2315`, failing at `:2341`:
  *"a second run over a store another live run holds allocated a run directory anyway, so two runs
  are now walking one set of documents: moved specify -> decompose — read from the holder's store;
  its own was deleted."*
- `story:protocol-drive-verb`'s own 2026-08-28 re-scope table flagged the mismatch as a note and
  nobody resolved it; the coordinator of the 2026-08-30 wave kept it back as prose. It is not prose.
