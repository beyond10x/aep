---
format: aep.planning-md/1
id: review-result:raw-capture-code-pass-2
kind: review-result
status: active
title: Pure raw capture final source examination
relations:
- reviews: task:planning-migration-raw-capture-values
revision: 1
---
unit: task:planning-migration-raw-capture-values at corrected submitted commit b3e29e4fab23a62d3d0e2de433f12322573548ce
verdict: NEEDS-CHANGE
cases: executed 54→55, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 directories / 23 files
needs-coordinator: route the introduced pending-batch refusal finding for final correction without a third source examination

1. `git --no-pager diff --stat`

```text
 .../tests/raw_capture_pending_batch_refusal.rs     | 60 ++++++++++++++++++++++
 1 file changed, 60 insertions(+)
```

The worktree diff is test-only. `git diff --check` exited zero. The retained patch is
`home-path:sha256:cf9e32bab74a73f4931c08f1b029756501dfa6d053302a161c497d5cbdeb3496`
with SHA-256 `147116a6ec40bafc14ceb66de3f0c564d49dc868f8e58cfb9ea452dd0206758b`.
The existing four-case adversarial file was not edited and retains the coordinator-supplied SHA-256
`45565b05cdc28324b6dc4c9aa7566de0279d0ec5a31c49eb0464edba71ef78b9`.

2. Case added

`crates/plan/aep-contract/tests/raw_capture_pending_batch_refusal.rs:10`
`retained_pending_batch_accepts_its_dedicated_refusal` constructs a refused Markdown first scan that
retains the exact root `.aep-batch.pending.json` bytes and names the contract's dedicated
`pending_batch_present` refusal at that exact path. It asserts that public validation admits this
internally consistent refused observation. The case is red now.

The first execution occurred before any suite and before the later two rustfmt-only expression-layout
changes. Exact command, complete raw output and exit are retained in
`0001-pending-batch-refusal.{command,log,exit}`. The failure portion is:

```text
running 1 test

thread 'retained_pending_batch_accepts_its_dedicated_refusal' (2588011) panicked at crates/plan/aep-contract/tests/raw_capture_pending_batch_refusal.rs:61:10:
a retained pending-batch marker has its own exact refusal code: CaptureValidationErrorsV1([CaptureValidationIssueV1 { code: IncompatibleVariant, path: "$.observation.phases[0].result.value.refusals" }])
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test retained_pending_batch_accepts_its_dedicated_refusal ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-contract --test raw_capture_pending_batch_refusal`
```

Exit status was 101. The case compiled and selected exactly one test, so the failure is the claimed
validation behavior rather than a construction or filter failure. Final test SHA-256 after scoped
rustfmt is `ebaf3dbe33822aa2ce3d55a570e81c8cc2833c42644962313bfcc4e137ee5c11`.

3. Affected suite run

The suite ran only after the new case existed. Exact command is in
`0002-affected-contract-schema.command`, complete verbatim output is in
`0002-affected-contract-schema.log`, and exit 101 is in `0002-affected-contract-schema.exit`.
It used Rust 1.98.1, locked/offline resolution, at most two jobs, tree-local `target/`, disabled debug
and incremental output, lld, empty compiler wrappers and the assigned TMPDIR. Verbatim result portions:

```text
running 48 tests
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

running 4 tests
test known_method_preflight_still_checks_present_source_compatibility ... ok
test complete_validation_accumulates_capture_defects_when_reconstruction_fails ... ok
test retained_foreign_markdown_nodes_require_their_exact_refusal ... ok
test unresolved_source_refusals_require_root_coordinates ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
test retained_pending_batch_accepts_its_dedicated_refusal ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 2 tests
test the_trait_extractor_reads_methods_and_not_prose ... ok
test the_contract_has_one_write_path_and_it_is_the_command_boundary ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 30 tests
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s

error: 1 target failed:
    `-p aep-contract --test raw_capture_pending_batch_refusal`
```

The pre-attack count of 54 contract cases comes from the accepted correction receipt; the new suite
executed 55 contract cases and the same 38 schema cases. The four prior findings' regressions are all
green. The initial scoped rustfmt check exited 1 and requested only two expression-layout changes;
assertions were preserved. The final scoped check in `0004-scoped-rustfmt-restored` exited 0.

4. Judgement finding

The finding covers corrected submitted commit `b3e29e4fab23a62d3d0e2de433f12322573548ce`.

| File:line | Verdict | Origin | Finding and reachability |
| --- | --- | --- | --- |
| `crates/plan/aep-contract/src/migration/capture.rs:1842` | NEEDS-CHANGE | introduced | The corrected retained-node guard hard-codes `ForeignMarkdownNode` for every captured regular file outside the admitted document/journal set. That set excludes the contract's known `.aep-batch.pending.json` marker, so a refusal carrying its dedicated `PendingBatchPresent` code is rejected as `IncompatibleVariant`. The new test measures the exported `RawCaptureObservationV1::validate` boundary at test line 58, exit 101; the exported `from_json` reader calls the same validator. There is intentionally no repository acquisition/provider caller in this pure unit. The source entered in this unit and the hard-coded branch entered in the correction diff, so origin is introduced. A final fix must distinguish the exact pending marker from generic foreign regular files for both host path variants before applying foreign-node coverage. |

5. Attacked and not broken

- The complete base `4eb999e0ae3cc77d1c387152e23a85ad4eae86dc` to corrected submission diff was examined; its SHA-256 is `928edecb44af94c26bc66ac19b9b1c8cbee6e46c517e430a80fd4e735b6d4ff0`, and its 13-path stat/name roster is retained in `0007-test-only-patch.log`.
- The accepted design SHA-256 is `a4f5d1d3f8a1e75130eb5ebe8ee30d0ecc613c390eb061d8015a3b9a0230105a`; prior review and correction report hashes match the brief at `589c069a...` and `13e87655...` respectively.
- The corrected production capture source retains SHA-256 `21af3eb10d881ee4053c57b397cf26efd8bf1ea66e666eefea34e7e22891bc10`; no implementation, schema, design, manifest, fixture, existing assertion or lockfile was changed by this pass.
- Known-method preflight source compatibility, root coordinates for unresolved/unreachable source refusals, exact generic foreign-node refusal coverage and supplied-capture diagnostics after reconstruction failure all remain green in the affected suite.
- Closed DTO/schema envelopes, phase and inner SQL read prefixes, coordinate/catalog validation, reconstruction, canonical ordering/framing/digests/diffs, hybrid policy combinations, path grammar and the generated schema surface yielded no additional surviving discrepancy. The disproved scalar trailing-newline hypothesis was not repeated.
- Reachable in-repository consumers remain the schema publication entry and tests; no IO acquisition, live provider, writer fencing, semantic import or migration command is present or claimed by this unit.

6. Paths written outside the worktree

- `/var/tmp/ess-evolution-aep-raw-capture-review-2-20260915/` is the assigned TMPDIR and is empty after execution.
- `home-path:sha256:1b12b290c88a95dbc41ae7b073cc7d63691a55897501b81899881c5a3fc23c2c` contains exactly `0001-pending-batch-refusal.command`, `.log`, `.exit`; `0002-affected-contract-schema.command`, `.log`, `.exit`; `0003-scoped-rustfmt.command`, `.log`, `.exit`; `0004-scoped-rustfmt-restored.command`, `.log`, `.exit`; `0005-source-examination.command`, `.log`, `.exit`; `0006-final-state.command`, `.log`, `.exit`; `0007-test-only-patch.command`, `.log`, `.exit`; `0007-test-only.diff`; and `review-report.md`.

The only build output is tree-local `target/` at 593,415,368 bytes. Final available space was
20,317,724,672 bytes, above the 8 GiB stop threshold. All commands completed synchronously and no
execution handle remains live. The sole heavy-build token is yielded with this report. No cleanup,
planning-store mutation, integration, publication, SQL review/provider work or third pass was performed.

7. Findings

```findings
- file: crates/plan/aep-contract/src/migration/capture.rs
  line: 1842
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The corrected retained-node guard classifies the known .aep-batch.pending.json marker as a generic foreign regular file, so validation rejects its dedicated pending_batch_present refusal.
```
