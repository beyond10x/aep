---
format: aep.planning-md/3
id: review-result:store-pack-adversary-closed-states-pass-1
kind: review-result
status: active
title: Adversary, pass 1, story:lifecycle-declares-closed-states
relations:
- reviews: story:lifecycle-declares-closed-states
revision: 1
---
unit: story:lifecycle-declares-closed-states, impl commit b8c84de21 plus adversary commit a8e7f0f19 in <trees>/wt-46e78419435d
verdict: CONFIRMED (3 red cases; 1 NEEDS-CHANGE judgement finding)
cases: executed 7→10, red 3
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 7 paths (part 6)
needs-coordinator: yes. Merging this makes the installed aep 0.79.0 refuse this repository's own store, because `protocols: ".."` points at this tree.

**1. What I changed** (taken before the commit, which is a8e7f0f19)
```
 .../govern/aep-engine/tests/lifecycle_fallback.rs  | 75 ++++++++++++++++++++++
 1 file changed, 75 insertions(+)
```
Only a test file. I touched no non-test path.

**2. Cases added** (all in `crates/govern/aep-engine/tests/lifecycle_fallback.rs`, all red now)

The first run of case 1 failed on a stale shared-target build (`no method named undeclared_closed`). That matches the coordinator's cargo.sh note, so I ran it again. The outputs below are from the clean runs, each case run alone.

- **`a_last_state_named_only_as_a_target_is_closed`**
  - Asserts: the file's own `LOG` ladder (`draft: [active]`, `active: [archived]`, no `archived:` row) has `closed_states() == {archived}` and `is_closed(archived)`.
  - Red output:
  ```
  panicked at crates/govern/aep-engine/tests/lifecycle_fallback.rs:392:5:
  assertion `left == right` failed: a status with nowhere to go is closed
    left: {}
   right: {Archived}
  ```
- **`every_undeclared_closed_state_is_named_in_the_refusal`**
  - Asserts: `closed: [implemented, rejected]` on a ladder that has neither state gives one `unknown_state` refusal naming both.
  - Red output:
  ```
  panicked at crates/govern/aep-engine/tests/lifecycle_fallback.rs:419:5:
  .../review-result.yaml: [unknown_state] lifecycle.closed: the `review-result` lifecycle lists `rejected` under `closed`, and its `transitions` do not declare that state (hint: ...)
  ```
- **`a_closed_state_named_only_as_initial_is_refused`**
  - Asserts: `initial: draft`, with `draft` absent from `transitions`, and `closed: [draft]` is refused.
  - Red output:
  ```
  panicked at crates/govern/aep-engine/tests/lifecycle_fallback.rs:439:5:
  assertion `left == right` failed: []
    left: 0
   right: 1
  ```

**3. Suite run** (after the cases existed)

Full binary: `cargo.sh test --locked -p aep-engine --test lifecycle_fallback`
```
test result: FAILED. 7 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
exit=101
```

Before count: the same binary with my three cases skipped (`--skip a_last_state_named_only_as_a_target_is_closed --skip every_undeclared_closed_state_is_named_in_the_refusal --skip a_closed_state_named_only_as_initial_is_refused`)
```
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
exit=0
```

**4. Findings**

All cover a8e7f0f19.

1. **`crates/govern/aep-domain/src/artifact.rs:2318`** (`closed_states`, and `is_closed` at :2331). CONFIRMED, introduced, warning.
   - What was measured: case 1, red. A state named only as a target has no successor, but it is not counted as terminal, so it is not closed. The story's Outcome says "a terminal state (no successor) is always closed".
   - What reaches it: every fixture ladder in `lifecycle_fallback.rs` (`LOG`, `DIGEST`, `FALLBACK`, `BRIEFING`, `INSIGHT`) is written this way, and the loader accepts it. For such a ladder, `aep plan artifact lifecycle log` prints `closed: none`. `at_the_end_of` (planning.rs:6851) already treats a missing row as the end, so the codebase now has two answers. The shipped ladders all write `[]` rows, so they are not affected.
   - Suggested fix: compute terminal over `statuses()` with `is_none_or(BTreeSet::is_empty)`, or refuse target-only states at load and say so in `lifecycle-file.md`. The reference row for `closed` currently defines terminal as "mapped to `[]`".
2. **`crates/govern/aep-engine/src/registry.rs:98`** (`undeclared_closed().first()`). CONFIRMED, introduced, note.
   - What was measured: case 2, red. Only one undeclared state is named. It is the first in `ArtifactStatus` order (`rejected`), not document order. INVARIANTS.md says validation accumulates.
   - What reaches it: copying the story's `closed: [implemented, rejected]` onto a ladder that has neither state.
3. **`crates/govern/aep-domain/src/artifact.rs:2337`** (`undeclared_closed` checks against `statuses()`, which includes `initial`). INFEASIBLE, introduced, note.
   - What was measured: case 3, red. The reference row and the refusal text both say "a status `transitions` declares". A state named only as `initial` is accepted.
   - What reaches it: nothing found. It needs a ladder whose `initial` has no row and is never a target. Either the check or the doc wording should change.
4. **`CHANGELOG.md`**: no `### Added` line for `closed:`. NEEDS-CHANGE, introduced, warning.
   - The story's Shared surfaces names this entry as the story's own.
   - This is also the only place adopters would learn that aep 0.79.x refuses a tree carrying `closed:`, because `ArtifactLifecycle` is `deny_unknown_fields`.
   - This repository's `.engineering/project.yaml` has `protocols: ".."`. So on main, the installed 0.79.0 refuses this store until a new binary is installed.
5. **`crates/plan/aep-contract/src/registry.rs:26`** (`LifecycleDescriptor`). CONFIRMED, introduced, note.
   - The contract descriptor a harness reads carries no closed set, while `aep plan artifact lifecycle` now prints one.
   - The acceptance says the closed set holds "everywhere the tooling ... publishes it". aep-contract is not in the story's scope.

**5. Attacked and not broken**
- `is_closed` and `closed_states` agree for every input; I reasoned this through and did not write a case for it.
- `permissive()` has no closed states, because every status has successors.
- The load-time check runs before the fallback branch, so a fallback lifecycle is checked too. No test covers that path.
- `insert_lifecycle` is the only load path (`aep-project/src/load.rs:400`), so the CLI cannot bypass the check.
- Text, yaml and json output all come from one view. The ordering is the `BTreeSet` order, which is deterministic.
- The kernel's `definition_for` builds from fields, so `closed` does not leak into the entity definition.
- A repository override without `closed:` drops the extra closed states and keeps terminal ones. The design says exactly that ("packs terminal states only").
- Design and specification can go `implemented` (closed) → `superseded` (not closed). This does no harm, because writes to a packed artifact are refused (design, *Writes, and unpacking*).
- Duplicates in `closed:` are quietly merged although the schema says `uniqueItems`. Transition targets already work this way; I did not raise it.
- Case and whitespace variants of a state name come out as unknown states and are refused.

**6. Paths written outside the worktree**

All are under `<scratch>/pack/units/lifecycle-declares-closed-states/scratch/`:
- `adv-case1.log`
- `adv-case1b.log`
- `adv-case2.log`
- `adv-case3.log`
- `adv-suite.log`
- `adv-suite-before.log`
- `adv-commit.txt`

Test fixtures also went to the shared target's `target/tmp/lifecycle-fallback-{closed-target-only,undeclared-closed-two,closed-initial-only}`. Cases 1 and 3 delete theirs only when they pass, and case 2 deletes its own only when it passes; all three are red, so all three directories remain.

**7. Findings block**
```findings
- file: crates/govern/aep-domain/src/artifact.rs
  line: 2318
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a state named only as a transition target has no successor but is not counted terminal, so closed_states() omits it, contrary to the story's "a terminal state (no successor) is always closed"
- file: crates/govern/aep-engine/src/registry.rs
  line: 98
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a closed list with several undeclared states is refused naming only the first in status order, not every one
- file: crates/govern/aep-domain/src/artifact.rs
  line: 2337
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: undeclared_closed accepts a state named only as initial although the reference page and the refusal say each entry must be a status transitions declares
- file: CHANGELOG.md
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the story's own changelog line for closed is missing, and it is the only place the 0.79.x refusal of trees carrying closed would be stated
- file: crates/plan/aep-contract/src/registry.rs
  line: 26
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the published LifecycleDescriptor carries no closed set while the lifecycle verb now prints one
```
