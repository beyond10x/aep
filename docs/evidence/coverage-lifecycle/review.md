unit: bind-current-coverage-to-specification-lifecycle; working tree over 068478785661ed41b05146ab77858d1b94794388
verdict: nothing found
cases: executed 0→7 reviewer cases, red 0 product cases
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned scratch only; complete local manifest retained separately
needs-coordinator: none

Reviewer source diff: empty. No implementation, test, planning-store or status file in the assigned worktree was changed by this review. The implementation diff already present during review is retained in `$TASK_SCRATCH/implementation-diff-stat.txt`; its production and documentation changes belong to the implementor. The review exercised CLI-created disposable stores only under assigned scratch.

The reviewed CLI SHA-256 was `b3bba1b539468df5724424e36bcc30f76d7caad6a6bd20e409e9ee58f31ead58`. Reviewed source SHA-256 values were:

| File | SHA-256 |
| --- | --- |
| `crates/edge/aep-cli/src/planning.rs` | `de12a76955dbaf26c2c1262205721f054b53138462e193067ca681697f69c218` |
| `crates/observe/aep-ess-evidence/src/planning_coverage.rs` | `1ae8019533baaee04cedd9b82134f2565e532407f28564a23ef6047002fa7291` |
| `crates/plan/aep-backend-markdown/src/journal.rs` | `378c7503f13d438bb7a82acb7d16aa7b183c27ce11f44b7088d2b35bd703071c` |

The reviewer wrote `$TASK_SCRATCH/probe.rs` before executing its cases. The Rust test binary drives the real CLI against isolated Git-native stores; expected move results, counts, reasons, milliseconds and evidence kinds are literal assertions. It does not modify or substitute the implementation.

| Reviewer case | Observation | Result |
| --- | --- | --- |
| `reviewer_equal_time_failure_vetoes_later_arriving_pass_and_replays_one_basis` | A pass arriving after an equally timed failure cannot earn conformance. A strictly newer pass does; repeating its exact import remains idempotent. Three distinct coverage records yield one separate eligibility unit and no manufactured legacy record. Store validation passes. | passed |
| `reviewer_same_second_order_uses_original_milliseconds_not_iso_projection` | A 1002 ms failure vetoes an arriving 1001 ms pass despite their identical one-second planning projection; a 1003 ms pass restores eligibility. All three original times remain exact. | passed |
| `reviewer_newest_incomplete_or_future_run_does_not_fall_back_to_old_success` | A newer generated-only selection and a newer future observation each block the older passing candidate. | passed |
| `reviewer_other_model_newer_failure_is_separate_and_current_model_change_revokes` | A newer failure for another model does not veto current-model success; changing the specification to that other digest then refuses conformance. | passed |
| `reviewer_original_integrity_and_closed_summary_fields_refuse_forged_authority` | Summary-only source, unknown admission claim, changed original suite bytes, forged descriptive counts and mismatched observation projection each fail to earn eligibility. The real move refuses with `no ess_conformance record`. | passed |
| `reviewer_released_reader_accepts_new_transition_and_preserved_original_kind` | Installed AEP 0.63.1 reads the successful new transition and validates its Git-native store. Its history retains the actual coverage kind and contains no fabricated legacy evidence event. | passed |
| `reviewer_actual_er_suite29_and_report2_earn_one_retained_current_basis` | ER's exact final release suite/report imports and earns conformance in a disposable Git-native store after input files are removed. The stored originals are byte-equal, passing count is 414, time is exactly 1790584881523 ms, derived eligibility is one, and validation passes. | passed |

The first selected equal-time case passed alone, exit 0, before the complete six-case reviewer run. Four earlier probe attempts had reviewer fixture/expectation errors: an invalid project discriminator, a forbidden absolute project locator, an incorrect expectation that an exact Git import duplicates a record, and an incorrect ordering expectation for history entries with the same projected instant. Their logs are retained as `fixture-*-error.log`; they are not product findings. The fixtures were corrected without altering their lifecycle acceptance assertions.

The complete first six-case run used:

```console
REVIEW_RUN=suite2 REVIEW_OLD_AEP="$(command -v aep)" "$TASK_SCRATCH/probe" --nocapture --test-threads=1
```

```text
running 6 tests
test reviewer_equal_time_failure_vetoes_later_arriving_pass_and_replays_one_basis ... ok
test reviewer_newest_incomplete_or_future_run_does_not_fall_back_to_old_success ... ok
test reviewer_original_integrity_and_closed_summary_fields_refuse_forged_authority ... ok
test reviewer_other_model_newer_failure_is_separate_and_current_model_change_revokes ... ok
test reviewer_released_reader_accepts_new_transition_and_preserved_original_kind ... ok
test reviewer_same_second_order_uses_original_milliseconds_not_iso_projection ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.99s
exit: 0
```

The subsequently added exact ER integration case ran alone:

```console
REVIEW_RUN=actual REVIEW_OLD_AEP="$(command -v aep)" "$TASK_SCRATCH/probe" --exact reviewer_actual_er_suite29_and_report2_earn_one_retained_current_basis --nocapture
```

```text
running 1 test
test reviewer_actual_er_suite29_and_report2_earn_one_retained_current_basis ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 12.98s
exit: 0
```

No judgement findings. Review scope was the planning eligibility bridge, retained-source re-admission, latest/tied evidence selection, original millisecond versus planning timestamp behavior, provenance basis, relevant reader/journal/CLI tests and stated compatibility. Malformed descriptive text cannot become an admitted candidate; another model remains a separate candidate set. Existing explicit legacy evidence behavior was not broadened or redesigned. Root owns the full repository gate, governance and publication; this review makes no release-completion claim.

All writes are under `$TASK_SCRATCH`: the Rust probe and binary, selected/full logs, fixture-error logs, implementation diff-stat, this report, and disposable stores with their per-command captured stdout/stderr/exit status. `$TASK_SCRATCH/scratch-manifest.md` maps the scratch identifier to its exact local path and enumerates every written file; it remains local. The reviewer acquired and released only its own worktree lease.

```findings
[]
```
