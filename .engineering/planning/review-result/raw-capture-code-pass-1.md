---
format: aep.planning-md/1
id: review-result:raw-capture-code-pass-1
kind: review-result
status: active
title: Pure capture submitted-code examination, pass 1
relations:
- reviews: task:planning-migration-raw-capture-values
revision: 1
---
unit: task:planning-migration-raw-capture-values at submitted commit a5123b894b42adfbb2757bf24d2d24491666f215
verdict: NEEDS-CHANGE
cases: executed 50→54, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 directories / 30 files
needs-coordinator: route four introduced findings for correction and the required second submitted-code pass

1. `git --no-pager diff --stat`

```text
 .../aep-contract/tests/raw_capture_adversarial.rs  | 194 +++++++++++++++++++++
 1 file changed, 194 insertions(+)
```

The diff is test-only. `git diff --check` exited zero. The retained patch is
`~/beyond10x/.ess-evolution/waves/0005-aep-migration/raw-capture-review-1/0009-test-only.diff`.

2. Cases added

| Case | Assertion | Current result | First execution |
| --- | --- | --- | --- |
| `crates/plan/aep-contract/tests/raw_capture_adversarial.rs:42` `known_method_preflight_still_checks_present_source_compatibility` | A present Markdown source cannot accompany the known SQLite method even when the result is a preflight refusal. | red, intended | `0001-known-method-preflight.{command,log,exit}`, exit 101 |
| `crates/plan/aep-contract/tests/raw_capture_adversarial.rs:72` `unresolved_source_refusals_require_root_coordinates` | `unresolved_source_coordinate` cannot name a fabricated captured Markdown path. | red, intended | `0002-root-coordinate-refusal.{command,log,exit}`, exit 101 |
| `crates/plan/aep-contract/tests/raw_capture_adversarial.rs:100` `retained_foreign_markdown_nodes_require_their_exact_refusal` | A retained foreign regular file must have a `foreign_markdown_node` refusal at its path; `read_failure` at the same path cannot admit it. | red, intended | `0003-foreign-node-refusal.{command,log,exit}`, exit 101 |
| `crates/plan/aep-contract/tests/raw_capture_adversarial.rs:155` `complete_validation_accumulates_capture_defects_when_reconstruction_fails` | A bad phase roster and the independent unsupported supplied capture are both reported. | red, intended | `0004-accumulated-capture-defects.{command,log,exit}`, exit 101 |

The first red outputs, verbatim failure portions (the complete raw logs above also retain compiler output):

```text
thread 'known_method_preflight_still_checks_present_source_compatibility' (1707803) panicked at crates/plan/aep-contract/tests/raw_capture_adversarial.rs:62:10:
a known SQLite method cannot describe a Markdown source: ()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test known_method_preflight_still_checks_present_source_compatibility ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-contract --test raw_capture_adversarial`
```

```text
thread 'unresolved_source_refusals_require_root_coordinates' (1714446) panicked at crates/plan/aep-contract/tests/raw_capture_adversarial.rs:90:10:
source resolution cannot invent a captured Markdown path: ()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test unresolved_source_refusals_require_root_coordinates ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-contract --test raw_capture_adversarial`
```

```text
thread 'retained_foreign_markdown_nodes_require_their_exact_refusal' (1718685) panicked at crates/plan/aep-contract/tests/raw_capture_adversarial.rs:145:10:
a foreign node must be named as foreign_markdown_node: ()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test retained_foreign_markdown_nodes_require_their_exact_refusal ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-contract --test raw_capture_adversarial`
```

```text
thread 'complete_validation_accumulates_capture_defects_when_reconstruction_fails' (1724468) panicked at crates/plan/aep-contract/tests/raw_capture_adversarial.rs:188:5:
failed reconstruction must not suppress validation of the supplied capture
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test complete_validation_accumulates_capture_defects_when_reconstruction_fails ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p aep-contract --test raw_capture_adversarial`
```

3. Suite run

Command, exactly, is retained in `0007-affected-suite-no-fail-fast.command`; verbatim output is in
`0007-affected-suite-no-fail-fast.log`; exit status `101` is in
`0007-affected-suite-no-fail-fast.exit`. It used Rust 1.98.1, locked/offline resolution, two jobs,
tree-local `target/`, debug and incremental disabled, lld, empty wrappers, and the assigned TMPDIR.

```text
running 48 tests
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

running 4 tests
test unresolved_source_refusals_require_root_coordinates ... FAILED
test known_method_preflight_still_checks_present_source_compatibility ... FAILED
test complete_validation_accumulates_capture_defects_when_reconstruction_fails ... FAILED
test retained_foreign_markdown_nodes_require_their_exact_refusal ... FAILED
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 2 tests
test the_trait_extractor_reads_methods_and_not_prose ... ok
test the_contract_has_one_write_path_and_it_is_the_command_boundary ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p aep-contract --test raw_capture_adversarial`
```

4. Judgement findings

All findings cover submitted commit `a5123b894b42adfbb2757bf24d2d24491666f215`. Each state is reachable
through the exported `RawCaptureObservationV1::validate` boundary and through `from_json`, which calls it.
No repository acquisition/provider caller exists yet because the accepted unit deliberately defers IO.

| File:line | Verdict | Origin | Finding and reachability |
| --- | --- | --- | --- |
| `crates/plan/aep-contract/src/migration/capture.rs:1396` | NEEDS-CHANGE | introduced | A known-method preflight skips `validate_method_source`, so a caller can submit a Markdown source with `SqliteReadTransaction` and a complete SQLite NotAttempted roster; public validation returns `Ok(())`, contrary to the contract's Refused compatibility rule. Measured at adversarial test line 62, exit 101. |
| `crates/plan/aep-contract/src/migration/capture.rs:2658` | NEEDS-CHANGE | introduced | `validate_refusals` checks ordering only, so `unresolved_source_coordinate` can name a fabricated `MarkdownPath` instead of a root coordinate; public validation returns `Ok(())`, contrary to the exact coordinate rule. Measured at adversarial test line 90, exit 101. |
| `crates/plan/aep-contract/src/migration/capture.rs:1837` | NEEDS-CHANGE | introduced | Refusal coverage ignores captured Markdown nodes, allowing a retained foreign regular file to be paired with unrelated `read_failure` at the same path instead of the required `foreign_markdown_node`; public validation returns `Ok(())`. Measured at adversarial test line 145, exit 101. |
| `crates/plan/aep-contract/src/migration/capture.rs:1300` | CONFIRMED | introduced | `validate_capture` is gated on successful phase reconstruction, so a bad roster suppresses the independent unsupported-capture diagnostic even though validation promises accumulated issues; validation still refuses the document for `PhaseRoster`, making this a warning rather than an admission bypass. Measured at adversarial test line 188, exit 101. |

5. Attacked and not broken

- Closed adjacent enum envelopes and required fields match the generated schema structure.
- Host-unit preservation and lexical relative-path grammar matched the accepted Unix and Windows rules.
- Canonical framing, digest domains, transcript/snapshot inputs, hybrid reconstruction, changed-set derivation, phase rosters, SQL read prefixes, and canonical vector ordering had no additional confirmed discrepancy.
- The generated `HexBytesV1` and `DigestV1` patterns reject trailing line terminators just as their Rust readers do. The targeted hypothesis passed and was removed rather than retained as a finding; exact receipt is `0005-scalar-schema-line-terminators.{command,log,exit}`, exit 0.
- Frozen implementation fingerprints remained identical to the implementor's ten `0093` hashes. After execution, the coordinator requested a crate-level documentation comment to eliminate the observed missing-docs warning; assertions and behavior were unchanged and no rerun was required. The final review test hash is `9595c577999f8172508bc3dce0fe84cd4dc8fbb188cd9eb29f383d062a4086a0`.

6. Paths written outside the worktree

- `/var/tmp/ess-evolution-aep-raw-capture-review-1-20260915/` (assigned TMPDIR; empty after execution).
- `~/beyond10x/.ess-evolution/waves/0005-aep-migration/raw-capture-review-1/` containing exactly: `0000-pre-execution.command`, `0000-pre-execution.exit`; `0001-known-method-preflight.command`, `.log`, `.exit`; `0002-root-coordinate-refusal.command`, `.log`, `.exit`; `0003-foreign-node-refusal.command`, `.log`, `.exit`; `0004-accumulated-capture-defects.command`, `.log`, `.exit`; `0005-scalar-schema-line-terminators.command`, `.log`, `.exit`; `0006-affected-suite.command`, `.log`, `.exit`; `0007-affected-suite-no-fail-fast.command`, `.log`, `.exit`; `0008-final-state.command`, `.log`, `.exit`; `0009-test-only.diff.command`, `0009-test-only.diff`, `0009-test-only.diff.exit`; and `review-report.md`.

The only build output is the tree-local `target/` (535 MiB). All commands completed synchronously;
there is no live execution handle. Free space after the final suite was 11,507,838,976 bytes, above the
8 GiB stop boundary. The build token is yielded with this report; no cleanup was performed.

7. Findings

```findings
- file: crates/plan/aep-contract/src/migration/capture.rs
  line: 1396
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: A known-method preflight skips source compatibility, so public validation admits a Markdown source paired with SqliteReadTransaction.
- file: crates/plan/aep-contract/src/migration/capture.rs
  line: 2658
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Refusal validation checks ordering but not code-to-coordinate rules, so unresolved_source_coordinate can fabricate a captured Markdown path instead of naming a root.
- file: crates/plan/aep-contract/src/migration/capture.rs
  line: 1837
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Refusal coverage ignores captured foreign Markdown nodes and therefore accepts read_failure where the retained node requires foreign_markdown_node.
- file: crates/plan/aep-contract/src/migration/capture.rs
  line: 1300
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Failed phase reconstruction suppresses validation of the independently supplied capture, so validation does not accumulate both defects.
```
