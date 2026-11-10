---
format: aep.planning-md/3
id: review-result:store-pack-adversary-closed-states-pass-2
kind: review-result
status: active
title: Adversary, pass 2, story:lifecycle-declares-closed-states
relations:
- reviews: story:lifecycle-declares-closed-states
revision: 1
---
```
unit: story:lifecycle-declares-closed-states, fix 0a564e508 plus adversary commit 882464803 in <trees>/wt-46e78419435d
verdict: nothing found that holds the unit (1 CONFIRMED note, 1 INFEASIBLE note; 0 red cases)
cases: executed 105→111, red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 20 scratch files plus shared-target build output (part 6)
needs-coordinator: yes. The pass-1 CHANGELOG finding is still open: 0a564e508 adds no `### Added` line for `closed:`.
```

I could not break the fix. All six new cases pass, and they now cover lines of 0a564e508 that no test reached before. The pass-1 cases are unchanged: `git diff a8e7f0f19 HEAD -- crates/govern/aep-engine/tests/` is empty.

**1. What I changed** (taken before the commit, then the staged set)
```
 crates/edge/aep-cli/tests/describe_type.rs         | 63 ++++++++++++++++
 .../govern/aep-engine/tests/lifecycle_fallback.rs  | 84 ++++++++++++++++++++++
 .../tests/descriptor_closed.rs                     | 55 ++++++++++++++   (aep-profile-development, new)
 .../tests/descriptor_closed.rs                     | 27 +++++++       (aep-profile-operations, new)
 4 files changed, 229 insertions(+)
```
Only test files. I touched no non-test path. Committed as 882464803.

**2. Cases added** (each run alone first; all green then and now)

In `crates/govern/aep-engine/tests/lifecycle_fallback.rs`:
- **`a_fallback_lifecycle_whose_closed_names_an_undeclared_state_is_refused`**
  - Asserts: a lifecycle with no `kind:` and `closed: [implemented]` gets one `unknown_state` refusal naming "fallback" and `implemented`, and no fallback is registered.
  - Mutant it catches: moving the check below the fallback branch in `registry.rs:98`.
- **`a_closed_state_named_only_as_a_target_is_accepted`**
  - Asserts: a `closed:` entry named only as a transition target loads, and `closed_states() == {rejected, archived}`.
  - Mutant it catches: dropping the target clause of `undeclared_closed`. The doc row promises "a key or a target".
- **`a_status_the_ladder_does_not_mention_is_not_closed`**
  - Asserts: `is_closed` is false for `implemented` and for `Other("correction-owed")` on the `LOG` ladder, and false for every status on `permissive()`.
  - Mutant it catches: dropping `statuses().contains` in `has_no_successor`. That mutant makes every status outside the ladder closed, including every non-standard status under `permissive`.

In `crates/edge/aep-cli/tests/describe_type.rs`:
- **`the_descriptor_closed_set_is_what_protocol_artifact_lifecycle_prints`**
  - Asserts: for every shipped kind, the kernel descriptor's `closed` equals the verb's JSON `closed` (same `Vec`, same order, not empty), and the story's set is {implemented, rejected, archived}.
  - Mutant it catches: `closed: Vec::new()` in `kernel.rs:375`. No existing test read the descriptor's `closed`.

In `crates/profile/aep-profile-development/tests/descriptor_closed.rs`:
- **`the_specification_descriptor_closes_what_its_ladder_file_closes`**
  - Asserts: the hand-written `closed` equals the `closed:` line of `artifacts/lifecycles/specification.yaml` plus the descriptor's empty rows, with no duplicates.

In `crates/profile/aep-profile-operations/tests/descriptor_closed.rs`:
- **`the_runbook_descriptor_closes_exactly_its_terminal_states`**
  - Asserts: closed == {archived}, which is the set of empty-row states.

The single runs, verbatim:
```
test a_fallback_lifecycle_whose_closed_names_an_undeclared_state_is_refused ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
test a_closed_state_named_only_as_a_target_is_accepted ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
test a_status_the_ladder_does_not_mention_is_not_closed ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
test the_specification_descriptor_closes_what_its_ladder_file_closes ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test the_runbook_descriptor_closes_exactly_its_terminal_states ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test the_descriptor_closed_set_is_what_protocol_artifact_lifecycle_prints ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 1.95s
```

**The mutants were reasoned, not executed.** Each one needs a rebuild of a mutated crate copy behind the shared build lock, which was heavily contended. After the single runs, rustfmt rewrapped one `assert!` in my describe case (whitespace only).

**3. Suite run** (after the cases existed; the before counts come from the same binaries with my cases skipped)
```
$ cargo.sh test --locked -p aep-engine --test lifecycle_fallback
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit=0
$ cargo.sh test --locked -p aep-cli --test describe_type
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
exit=0
$ cargo.sh test --locked -p aep-profile-development --lib --test descriptor_closed
test result: ok. 44 passed; ... (lib)
test result: ok. 1 passed; ... (descriptor_closed)
exit=0
$ cargo.sh test --locked -p aep-profile-operations --lib --test descriptor_closed
test result: ok. 49 passed; ... (lib)
test result: ok. 1 passed; ... (descriptor_closed)
exit=0
```
Before (same binaries, my cases skipped by name, new files not selected):
- lifecycle_fallback: 10 passed, 3 filtered out
- describe_type: 2 passed, 1 filtered out
- development lib: 44
- operations lib: 49

So 105→111.

**4. Findings** (cover 0a564e508)

1. **`crates/plan/aep-backend-entity/src/kernel.rs:375`, `crates/profile/aep-profile-development/src/descriptor.rs:64`, `crates/profile/aep-profile-operations/src/descriptor.rs:78`.** Before my cases, no test read any descriptor's `closed`. Mutant, CONFIRMED, introduced, note.
   - What was measured: nothing failed when the field was emptied or changed. The four cases above now cover it.
   - What reaches it: `QueryService::describe_type` on all three backends.
2. **The two profile descriptors against the lifecycle verb.** Judgement, INFEASIBLE, introduced, note.
   - Specification: the same set as the verb, in a different order. The literal is `[implemented, rejected, archived]`; the verb and kernel print `rejected, implemented, archived` (`ArtifactStatus` order).
   - Runbook: there is no planning ladder for it, so `aep plan artifact lifecycle runbook` prints `closed: none`, while `aop.runbook/v1` publishes `[archived]`.
   - What reaches it: nothing found. Nothing in the workspace calls either crate's `type_descriptors()`. `aep describe` (app.rs:2138) builds its backend without ladders, so it publishes no lifecycle at all; I ran it and got no `lifecycle` key for `aep.story/v1` or `aep.specification/v1`. The profiles' `statuses` lists already differ in order from the kernel's, so this matches existing practice.

**5. Attacked and not broken**
- `closed_states()` and `is_closed()` agree with each other and with the doc for no-row, empty-row and initial-only states.
- `permissive()` has no closed status, including for `Other` statuses.
- `undeclared_closed` covers keys and targets, excludes `initial`, and the refusal names every undeclared state.
- The kernel descriptor's `closed` and the verb print the same `Vec` for all 13 shipped kinds (checked by test and by hand).
- The closed set of every shipped ladder is sensible:
  - architecture-decision-record: rejected, superseded
  - blocker: cleared
  - design, epic, initiative, specification, story, task: rejected, implemented, archived
  - executable-system-specification: archived
  - obligation: met
  - outbound-claim: corrected
  - review-result: archived
  - vision: rejected, archived
- `LifecycleDescriptor.closed` uses serde default and skips when empty. `deny_unknown_fields` breaking older readers is the same pre-existing pattern as `or`.
- The pass-1 cases are kept exactly as written.

**6. Paths written outside the worktree**
- In `<scratch>/pack/units/lifecycle-declares-closed-states/scratch/`:
  - `adv2-build.log`, `adv2-commit.txt`, `adv2-engine-cases.rs`, `adv2-describe-case.rs`, `adv2-suite.sh`, `adv2-suite.done`
  - `adv2-dev.log`, `adv2-ops.log`, `adv2-describe.log`
  - `adv2-a_fallback_lifecycle_whose_closed_names_an_undeclared_state_is_refused.log`
  - `adv2-a_closed_state_named_only_as_a_target_is_accepted.log`
  - `adv2-a_status_the_ladder_does_not_mention_is_not_closed.log`
  - `adv2-suite-{engine,describe,dev,ops}.log`
  - `adv2-suite-{engine,describe,dev,ops}-before.log`
- Build output in the shared target `<scratch>/target/`: the debug `aep` binary and the test binaries. The three engine fixture directories (`target/tmp/lifecycle-fallback-{undeclared-closed-fallback,closed-target-declared,closed-unmentioned}`) are deleted by the tests when they pass, and all three passed.

**7. Findings block**
```findings
- file: crates/plan/aep-backend-entity/src/kernel.rs
  line: 375
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no test read a descriptor's closed set or the fallback, target-only and outside-the-ladder branches of the fix, so emptying or flipping them stayed green until the six adversary cases in 882464803
- file: crates/profile/aep-profile-development/src/descriptor.rs
  line: 64
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the profile descriptors list closed states in document order (specification) or for a type the lifecycle verb has no ladder for (runbook, verb prints none), unlike the kernel and verb's status order, and nothing in the workspace reads them
```
