---
format: aep.planning-md/1
id: story:gate-runs-in-a-worktree
kind: story
status: implemented
title: Every gate step runs in a fresh worktree, and a wrapper's exit is never read as the gate's
summary: synth-check fails in any worktree on go build VCS status; a background wrapper reports exit 0 for a gate that exited 201; protocol-cli has no lib target.
tags:
- gate
relations:
- decomposes: epic:declared-configuration-invariants
- serves: vision:O6
revision: 5
---
# Story: Every gate step runs in a fresh worktree, and a wrapper's exit is never read as the gate's

## Outcome

A wave's worktree can run the whole gate. What is left after `412982c` (`npm ci`), `b216ce7`
(`--no-fail-fast`) and per-step capture:

## Context

- `431986de` agent `a34b2fe9ebb1c2f66`: `synth-check` fails in any worktree — `go build` → `error obtaining
  VCS status: exit status 128`; `go build -buildvcs=false ./...` → exit 0. Blocked every fan-out agent's gate.
- `cc946bc3#633`, `8cffc110#267-#269`: the background-task wrapper reported "completed (exit code 0)" for a
  `task check` that exited 201; only `GATE_EXIT=$?` caught it (documented `AGENTS.md:475`).
- `11727595` six sub-agents + two main-session calls: `cargo test -p protocol-cli --lib` → "no library
  targets found", one wasted call each.

## Acceptance

- ~~`cargo xtask synth --check` passes `-buildvcs=false` to every `go build`/`go vet` it runs~~ —
  already landed as `fab1d73` (2026-08-28 22:08, `GOFLAGS=-buildvcs=false` in `go_tool`); the
  session that reported it ran an older tree. Recorded here so it is not re-filed.
- `AGENTS.md` § Gate: "run the gate as `task check; echo GATE_EXIT=$?` and read that line; a wrapper's
  exit code is the wrapper's".
- `AGENTS.md` § Conventions: `protocol-cli` has no lib target; `cargo test -p protocol-cli` runs its tests.

## Out of Scope

Making the gate fit a 43 GB disk — see `AGENTS.md` § Gate on target directories.
