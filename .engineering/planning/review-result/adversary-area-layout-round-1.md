---
format: aep.planning-md/1
id: review-result:adversary-area-layout-round-1
kind: review-result
status: active
title: 'Adversary, round 1: crates under area directories'
relations:
- reviews: story:crates-under-area-directories
revision: 1
---
# Adversary, round 1 — story:crates-under-area-directories

Verdict: NEEDS-CHANGE. Cases executed 2119 → 2121, red 2. Origin: introduced 3 / pre-existing 0.
Agent: `adp:adversary` (opus). Cases added: `xtask/tests/pre_move_crate_paths.rs` (283 lines, untracked).

Measured reach of finding 1: `story:profile-and-cli-crates-named-after-aep` (scopes
`crates/edge/protocol-cli`, `crates/profile/aop-domain`) and `story:recurrence-key` (scopes
`crates/protocol-cli`, `crates/aop-domain`) share wave 7 with no collision reported; correcting
`recurrence-key`'s paths on a scratch copy of the store produces two collisions and moves it out of
the wave (total collisions 414 → 407).

Attacked, did not break: every `CARGO_MANIFEST_DIR`/`../..` join in `crates/**` and `xtask`;
`[package] name` of all 22 crates byte-identical to `9607ea6`; no `build.rs`, `publish`, `include`,
`exclude`, `default-members`; the AGENTS.md § Areas dependency rule holds for every compiled edge;
`crates/**` write scopes and `*/src/*` transcript selectors at the new depth; `cargo xtask guards`
(1820 bodies, 0 duplicated; reverting the `nth(2)` change reddens `guard-check`); `xtask deps /
version / claims / status --check / schema --check / notes --self-test` exit 0; `task install`,
workflows, `.cargo/config.toml`, scripts; the two recorded-transcript narrations and the blog quote
are correct as left. Off-gate `.engineering/checks` units red on pre-existing causes, not raised.

```findings
- file: .engineering/planning/story
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: >-
    50 live planning artifacts still scope 112 paths at the pre-move crate directory, and
    `aep artifact waves` compares scope strings without normalising, so this wave's own rename
    story shares wave 7 with a story that rewrites the same two directories.
- file: docs/plan/gap-register.md
  line: 85
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >-
    the story's acceptance predicate is false — 288 pre-move paths survive in 35 files, 121 of
    126 of which resolved at the base commit, including the open-gap register's own file:line
    citations to current code.
- file: xtask/src/main.rs
  line: 2405
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: >-
    layout_tests read only the immediate children of each area directory and only the top of
    `crates/`, so a manifest at `crates/<area>/<group>/<crate>` is a member of nothing and no test
    reports it; the state was not constructed because doing so means writing under `crates/`.
- file: .engineering/planning/journal.jsonl
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: >-
    the acceptance command uses `rg` without `--hidden`, so it cannot read `.engineering/` at all —
    1674 pre-move citations exist there against the 472 the command sees, and the machine-read
    scope drift is entirely outside what a green acceptance certifies.
```
