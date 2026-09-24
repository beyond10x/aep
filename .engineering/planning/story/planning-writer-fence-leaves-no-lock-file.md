---
format: aep.planning-md/1
id: story:planning-writer-fence-leaves-no-lock-file
kind: story
status: draft
title: A released planning writer fence leaves no lock file behind, or says why it must
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning_writer_fence.rs
revision: 2
---
## Outcome

After a clean release of the planning writer fence, the store directory holds no
`.aep-planning-writer*.lock` file, or the module states in its own doc comment why the file must
persist and every tool that reports "lock file left behind" stops treating it as debris.

## Why

Observed 2026-09-21 in rehearsal run 3 (wave-validate-v2-20260920, copy `home-path:sha256:33a89503aef363fe2509aefb7752c7e6ad48489f1b2a1c123b1c93dbad486964`)
and on the real Eventlog planning store after the M4a cutover: two zero-byte files,
`.engineering/.aep-planning-writer.lock` and `.engineering/.aep-planning-writer-<digest>.lock`,
absent from the pre-cutover archive, present after the first writer-shaped command.

Mechanism, read from `crates/edge/aep-cli/src/planning_writer_fence.rs` at 763d195be:

- `lock()` (`:110-125`) opens with `.create(true)` and takes an exclusive `flock`.
- `Drop for PlanningWriterFence` (`:128-134`) calls `fs2::FileExt::unlock` and nothing else. The
  file is never unlinked; the designed release leaves it by construction.
- Two files because `acquire_store` (`:78`) takes the project-scoped lock (`LOCK_FILE`, `:16`) and
  a digest-named one (`:104`).
- Callers: `store_command.rs:169`, `:196`; `serve/api.rs:256`; `planning.rs:2019`, `:9010`.

Pre-existing instance: the AEP integration tree carries the same pair, zero bytes, dated 2026-09-17
08:36, mtime unmoved through every gate and commit of the wave.

Which command creates them is not pinned: in run 3, `list` and `history` (12:53:25) did not; five
commands share the 12:53:26 second (`validate`, `inspect`, `dry-run`, two `move` probes). One
command at a time on a fresh copy settles it.

Cost: every `git status` in a planning repository shows two untracked files no command removes; in
this wave they were explained three times (pre-flight, staged file lists, run 3's closing line). A
reader cannot tell a left-behind fence from a live one.

## Acceptance

- Red first: a case that runs one fenced command to completion and asserts the store directory is
  as it was.
- A lock left by a process that died stays distinguishable from a live one; removal on release must
  not open a window in which two writers both believe they hold the fence. The mechanism is named
  in the source.
- If the file must persist for the `flock` protocol on a supported platform, the module's doc
  comment says so, and `evidence/c-rehearsal.sh`'s closing check stops reporting it.
