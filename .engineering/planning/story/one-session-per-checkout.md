---
format: aep.planning-md/1
id: story:one-session-per-checkout
kind: story
status: implemented
title: One session per checkout, and a wave in its own worktree
summary: A peer's git add -A swept three files onto main; three sessions shared uncommitted skill edits; five worktrees vanished mid-wave.
tags:
- process
- wave
relations:
- decomposes: epic:self-evaluation
- serves: vision:O6
- informed_by: story:wave-as-a-surface
revision: 4
---
# Story: One session per checkout, and a wave in its own worktree

## Outcome

`AGENTS.md` states the rule three sessions paid for, and the wave skill's pre-flight checks it.

## Context

- `3130470e#123,#162,#242`: a peer session's `git add -A` swept three unclaimed files onto public `main`;
  `main` moved under the wave twice mid-integration.
- `11727595#17370-#17397`: three sessions shared one checkout with uncommitted edits to
  `skills/planning/SKILL.md`; a peer's unformatted hunks made `fmt-check` red for everyone.
- `4d4c15a4#274`: five worktrees and build dirs under `~/.cache/harness-wave` deleted mid-wave with five
  agents in flight, cause unattributed.

## Acceptance

- `AGENTS.md` § Branches and waves: a session that edits works in a worktree it created; the main checkout
  is for reading and merging; a wave records every worktree path in its wave page.
- `skills/wave/SKILL.md` pre-flight refuses when `git worktree list` shows a worktree whose branch is
  checked out elsewhere, or when the main checkout is not on `main`.

## Out of Scope

Detecting a second Claude session by process inspection.
