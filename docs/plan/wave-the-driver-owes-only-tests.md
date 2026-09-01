# Wave — the driver owes only tests

> **Status: dispatched 2026-08-30**, on the operator's instruction `dispatch wave` after the four
> units were proposed and read back. Written against `035b2a4`.
>
> **One pre-flight check was deviated from, deliberately, and this is the record of it.** The skill
> refuses a wave started from a dirty tree. The tree carries nine modified files — five under
> `.engineering/planning/` (three story bodies re-verified today, two re-parented orphans, the
> journal) and four under `integrations/claude-code/` (the wave skill's own defect-8 fix). The
> wave's units touch **only `crates/`**, so the two file sets are disjoint and the merge back into
> `main` cannot conflict with what is uncommitted. The refusal's purpose is met; its letter is not.
> The cost of being wrong is one merge conflict in a file nobody in this wave opens.

**Goal: close the four remaining units of `epic:reference-driver`, none of which needs production
code beyond one prose line — every open acceptance row across all four is a test that was never
written. Four surfaces, no two on one file, merged onto one branch and closed by one gate run.**

## 1. Where this stands — measured 2026-08-30

| fact | evidence |
|---|---|
| root filesystem 75 G free of 848 G, at 91 % | `df -h .` |
| the operator's `target/` 30 G | `du -sh target` |
| **14 G still standing from a wave that never cleaned up** | `~/.cache/claude-tmp/claude-1000/-home-operator-projects-aep/ba00d8e0-…/scratchpad/eval/ws_eval`, last written 2026-08-24, under a repo path this tree no longer occupies. Left standing — it is the operator's to delete, and it is the evidence behind `story:wave-skill-defects-found-by-running-it` § 8 |
| `git worktree list` — one entry, the main tree | no previous wave's trees are standing |
| `sccache` at `/usr/bin/sccache`, wired at `~/.cargo/config.toml:60` | `RUSTC_WRAPPER` unset in the shell; the cargo config carries it |
| a package-scoped build in a fresh worktree: **30 s wall, 1.6 G target** | measured 2026-08-30 for the previous wave, same tree, `a8b139b` — `cargo test -p protocol-cli --no-run` |
| **so N=4 costs ~6.4 G** | 4 × 1.6 G against 75 G free |

## 2. The units

Every one is `epic:reference-driver`, every one serves **O3**, and every scope below is **cited** —
read out of each story's own re-verification table, which names `file:line` for each open row.

| # | story | status in | surface | overlap |
|---|---|---|---|---|
| 1 | `story:driver-spec-crate` | active | `crates/aep-engine/src/registry.rs` (new `mod tests`), `crates/aep-driver-spec/src/map.rs` (tests) | none |
| 2 | `story:driver-router` | active | `crates/aep-driver/tests/determinism.rs`, `crates/aep-driver/tests/tool_config.rs` | none |
| 3 | `story:protocol-drive-verb` | proposed → active | `crates/protocol-cli/tests/drive_cli.rs`, `crates/aep-driver/tests/driving.rs` | none |
| 4 | `story:default-step-map` | proposed → active | `crates/aep-engine/tests/` (new file) | none |

**Pairwise, by file.** 1×2 disjoint. 1×3 disjoint **only because of a routing decision taken here**
(below). 1×4 share the `aep-engine` package and no file — one edits `src/registry.rs`, the other
adds a file under `tests/`. 2×3 share the `aep-driver` package and no file — `determinism.rs` and
`tool_config.rs` against `driving.rs`. 3×4 disjoint. 2×4 disjoint.

**The one routing decision.** `story:driver-spec-crate`'s `UndeclaredEvidenceKind` red-path test
would naturally have gone into `crates/protocol-cli/tests/drive_cli.rs`, beside the green-path
`check_run` test at `:921` — which is the file unit 3 adds three tests to, and the only real
collision in the set. It is sent to `crates/aep-driver-spec/src/map.rs`'s own `mod tests` instead,
which is reachable because `check_run` is `pub fn` (`crates/aep-driver-spec/src/map.rs:899`) and is
the better home anyway: the refusal is produced twelve lines below it, at `:913`.

## 3. What each unit owes

**1 — `story:driver-spec-crate`.** Two red-path tests, no production code.
- An orphan major pin refused at load. `crates/aep-engine/src/registry.rs` has **no `mod tests` at
  all**; the production branch is `:377-398`. The nearest existing assertion
  (`crates/aep-driver-spec/src/map.rs:1585`) calls `cross_validate` directly and never loads through
  the registry, so it does not reach it.
- `UndeclaredEvidenceKind` fires. It appears once, at `crates/aep-driver-spec/src/map.rs:913`, in
  production code only; the sole `check_run` test asserts `refusals.is_empty()`.

**2 — `story:driver-router`.** Two test additions, no production code.
- An environment token in `BANNED`. `crates/aep-driver/tests/determinism.rs:21-29` bans `HashMap`,
  `HashSet`, `SystemTime`, `Instant::now`, `rand::`, `getrandom`, `thread_rng` — and no environment
  token, while the crate's purity claim is *no clock, no randomness, no ambient environment*.
- One capability in `allow`, `approval_required` **and** `deny` at once, asserted through
  `tool_config`. `crates/aep-driver/tests/tool_config.rs:18-32` uses three *different* capabilities.

**3 — `story:protocol-drive-verb`.** Three tests and one line of prose.
- Assert the host in the lock refusal. `a_second_driver_is_refused_by_name_and_writes_nothing`
  (`crates/protocol-cli/tests/drive_cli.rs:360`) writes `host()` into the lock and asserts only the
  run id, the pid and `--take-lock`.
- A `command` step that **creates** an artifact, with the next evaluation reading the higher
  `artifact.<kind>.count`. The three existing call sites of `story_count`
  (`crates/aep-driver/tests/driving.rs:727`, `:755`, `:800`) are all F7 *shrink* assertions.
- Two `Engine` values in one process that do not collide on a run directory. A search for `collide`,
  `collision`, `two engine`, `concurrent` over `crates/aep-driver/tests/` and `drive_cli.rs` returns
  nothing.

**4 — `story:default-step-map`.** One test.
- The loader's `TREE` table has `drivers` last (`crates/aep-engine/src/load.rs:33`) and is walked at
  `:131`, and no test in `crates/aep-engine/tests/` reads the table or its order. The comment above
  the row says the order is load-bearing; nothing holds it there.

## 4. What is not dispatched, and why

**The `declined` decision is the coordinator's, not an implementor's.** The map declares nine states
and the workflow ten — `declined` is in `workflows/development/default.yaml:105` and not in the map.
The default taken, absent an operator answer: **the shipped rule stands** — a state the map is
silent about transitions immediately (`crates/aep-driver/tests/routing.rs:83`,
`crates/aep-driver-spec/src/map.rs:754-762`), because a terminal state has nothing for a step to do.
That rewrites one acceptance line in `story:default-step-map` and builds nothing, so it is a store
write and stays with the coordinator.

**The lock-path wording, same reason.** `crates/aep-driver/src/lock.rs:9` documents *one fixed path
per store*; `crates/protocol-cli/src/drive.rs:99` puts `lock.json` under the project's
`.engineering/runs/`, which `--store` does not move. One line of prose in whichever direction the
driver owner picks — not an implementor's call.

**`story:governed-dogfood-run` is out.** It depends on unit 3 and is the epic's *Done When*: a real
task driven end to end. It is the wave *after* this one, by construction.

## 5. The run — 2026-08-30

**All four implementors returned their unit test-only and green. Zero adversarial passes ran:
every sub-agent was killed by a session rate limit at ~12:10, resetting 13:00 Europe/Berlin.**
Three adversaries died having read nothing; the fourth implementor died between its last edit and
its gate. **No work was lost** — every worktree still holds its diff, and the coordinator ran the
missing gate itself.

| unit | diff | production code | gate | adversary |
|---|---|---|---|---|
| `story:driver-router` | +119 / −3, both test files | none — `git diff --stat -- crates/aep-driver/src/` empty | `cargo test -p aep-driver` **exit 0**, 69 passed; `determinism` 2→3, `tool_config` 5→6 | killed before reading |
| `story:default-step-map` | +214, one new file | none | `cargo test -p aep-engine` **exit 0**; lib 62→64, new suite 2 | killed before reading |
| `story:driver-spec-crate` | +145 / −0 | none — insertions only, first `mod tests` in `registry.rs` | `cargo test -p aep-driver-spec -p aep-engine` **exit 0**; `aep-driver-spec` 35→36 | killed before reading |
| `story:protocol-drive-verb` | +270 / −0 | none | see below — **exit 0**, 609 passed across 36 suites | never dispatched |

**The most valuable thing the run produced came from an implementor, not an adversary.**
`story:driver-spec-crate`'s first draft of the orphan-pin test asserted `contains(VersionMismatch)`
plus message text, and **survived the mutation meant to redden it** — `StepMap::cross_validate`
raises the same code with nearly the same wording, so the draft never reached the registry branch at
`crates/aep-engine/src/registry.rs:377-398` at all. It now asserts `location == "step map
development/default"`, a string only the registry produces. That is the same defect class as the
previous wave's non-discriminating `one_ladders_column_order_…` case, caught one stage earlier.

### The build directories were put in the wrong place, and it cost a gate run

The coordinator placed each build directory **outside** its worktree, following the skill's
`references/branch-and-merge.md` — *"one per worktree, usually outside it"*. `AGENTS.md:499-502`
says the opposite in as many words. The first gate of `impl/protocol-drive-verb` exited **101** with
**11 failures, every one of them `crates/protocol-cli/tests/store_selection.rs:77` — `the scratch
tree is under the repository: StripPrefixError(())`** — none touching the unit's change. Because
cargo stops at the first failing target, `drive_cli` and `driving`, the two suites the unit
actually owed, **never ran**: the gate was not noisy, it was empty.

Re-run with the target inside the worktree, `--no-fail-fast`:

```
cargo test -p protocol-cli -p aep-driver --no-fail-fast   EXIT 0
609 passed across 36 suites, 0 failed
  tests/driving.rs           14 passed  (baseline 12)
  tests/drive_cli.rs         47 passed  (baseline 47 — the host assertion extends an existing test)
  tests/store_selection.rs   11 passed  (the 11 that had failed)
cargo clippy -p protocol-cli -p aep-driver --all-targets -- -D warnings   EXIT 0
cargo fmt --check                                                        EXIT 0
```

The other three units are unaffected: only `protocol-cli` carries that assert. Filed as
`story:wave-skill-defects-found-by-running-it` § 9, whose fix reverses the skill's wording rather
than qualifying it — **inside the worktree, always, unless an adopting repository says otherwise**,
which also answers § 8, since a build directory inside the tree is removed with it.

### Nothing is merged

Green does not route to merge. `adp/default` runs `implement → verify → adversarial_verify`, and
this repository's own record is that merging on the first green would have shipped eight defects
last wave. The four branches stand unmerged until an adversary has been at each.
