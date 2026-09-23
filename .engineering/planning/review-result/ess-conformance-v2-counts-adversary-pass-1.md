---
format: aep.planning-md/1
id: review-result:ess-conformance-v2-counts-adversary-pass-1
kind: review-result
status: active
title: Count reader first independent implementation review
relations:
- reviews: story:admit-ess-conformance-v2-counts
revision: 1
---
unit: AEP ESS count-stage reader first correctness pass; fc58d0fb365f04f2c92e0a7cc7a278f85b55ec8e plus the three listed new test paths; base 46c3d8ba1f05cc79e35999223fdc60d3781c9fdc
verdict: nothing found
cases: executed 1313→1320, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 pre-existing Cargo infrastructure metadata path; no authored external files
needs-coordinator: none
git --no-pager diff --stat

The command produced no output: all additions are new untracked test files. Supplementary `git --no-pager diff --no-index --stat /dev/null <path>` output for each addition follows; each no-index command exits 1 to indicate the added file.

```text
 .../edge/aep-cli/tests/count_reader_pass1.rs       | 229 +++++++++++++++++++++
 1 file changed, 229 insertions(+)
 .../aep-ess-evidence/tests/count_reader_pass1.rs   | 170 +++++++++++++++++++++
 1 file changed, 170 insertions(+)
 .../tests/support/count_reader_pass1.rs            | 33 ++++++++++++++++++++++
 1 file changed, 33 insertions(+)
```

Complete `git ls-files --others --exclude-standard` inventory:

```text
crates/edge/aep-cli/tests/count_reader_pass1.rs
crates/observe/aep-ess-evidence/tests/count_reader_pass1.rs
crates/observe/aep-ess-evidence/tests/support/count_reader_pass1.rs
```

2. Focused cases

The seven added cases were written before the first focused invocation. Each final case was selected alone with `--exact`; each ran exactly one case and now passes. The complete package suite has not run at the time this section is assembled.

| New case | Contract assertion | Final focused result |
|---|---|---|
| Adapter `decoded_duplicate_keys_refuse_in_report_and_arbitrary_nested_payload` | Decoded ASCII and Unicode duplicates refuse with the structured object path, including arbitrary payload maps; an escaped unique key admits. | 1 passed |
| Adapter `count_tokens_remain_strict_while_legacy_payload_numbers_keep_their_grammar` | All six counts reject negative zero, fractions, exponent and string tokens and u64 overflow; finite legacy payload numbers retain their grammar. | 1 passed |
| Adapter `structural_and_textual_predicate_depth_share_the_frozen_limit` | Mixed mapping and string-prefix depth admits exactly 32 and refuses 33 with InvalidPredicate. | 1 passed |
| Adapter `closed_scenario_values_and_shapes_do_not_close_literal_payload_keys` | Literal payload keys and arbitrary shape field names admit while unknown keys in the typed ScenarioValue and LeafShape records refuse. | 1 passed |
| CLI-package `typed_batch_preserves_crlf_and_unicode_bytes_and_refuses_either_bad_position` | JSON and YAML transport preserve leading/trailing CRLF, Unicode escape bytes and u64::MAX; either invalid position rejects the complete batch and an absent reader refuses. | 1 passed |
| CLI-package `exact_high_time_horizon_restore_and_changed_source_refusals_preserve_execution` | At u64::MAX, an exact one-day horizon remains unknown coverage and one extra millisecond is stale; changed suite bytes refuse direct mutation without changing nonempty state, facts or time; restore re-admits and refuses a one-millisecond future observation. | 1 passed |
| CLI-package `actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output` | Actual aep/protocol inspect aliases preserve exact completion digits, identify/refuse future observations, and print no admitted prefix for a malformed second input. | 1 passed |

Three preliminary CLI invocations did not execute cases because this review's new file omitted the RequirementContext trait import. The compiler also identified an unused ProtocolEngine import. These test-construction errors were corrected only in the new test file; they are not semantic findings. All three original compilation outputs remain below.

The first executing CLI inspect attempt failed this review's assertion that stderr literally contains `future`. The actual supported diagnostic at crates/edge/aep-cli/src/app.rs:2480 says `has not happened yet`, names record 1 and preserves the exact epoch. The assertion was corrected to check that established wording, exact epoch and ordinal; refusal status, alias agreement, complete output and malformed-batch requirements were retained. The original assertion failure and corrected focused run remain below. This is an incorrect new test oracle, not a reader defect.

All commands ran from the assigned AEP worktree. CARGO_TARGET_DIR, RUSTC_WRAPPER and SCCACHE_SERVER_UDS were unset; the coordinator allowed ordinary local compilation because the inherited sccache socket was no longer listening. TMPDIR was the assigned adversary-pass-1 scratch. CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_CACHE_RUSTC_INFO=0, CARGO_BUILD_JOBS=4 and CARGO_NET_OFFLINE=true. No daemon was started. Logs combine stdout and stderr in original order.

Focused invocations, in execution order:

```text
cargo test --locked --offline -p aep-ess-evidence --test count_reader_pass1 decoded_duplicate_keys_refuse_in_report_and_arbitrary_nested_payload -- --exact --nocapture
   Compiling aep-domain v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/govern/aep-domain)
   Compiling aep-ess-evidence v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/observe/aep-ess-evidence)
    Finished `test` profile [unoptimized] target(s) in 10.04s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-3709f17a590149b8)

running 1 test
test decoded_duplicate_keys_refuse_in_report_and_arbitrary_nested_payload ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

exit: 0
```

```text
cargo test --locked --offline -p aep-ess-evidence --test count_reader_pass1 count_tokens_remain_strict_while_legacy_payload_numbers_keep_their_grammar -- --exact --nocapture
    Finished `test` profile [unoptimized] target(s) in 0.12s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-3709f17a590149b8)

running 1 test
test count_tokens_remain_strict_while_legacy_payload_numbers_keep_their_grammar ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.01s

exit: 0
```

```text
cargo test --locked --offline -p aep-ess-evidence --test count_reader_pass1 structural_and_textual_predicate_depth_share_the_frozen_limit -- --exact --nocapture
    Finished `test` profile [unoptimized] target(s) in 0.14s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-3709f17a590149b8)

running 1 test
test structural_and_textual_predicate_depth_share_the_frozen_limit ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

exit: 0
```

```text
cargo test --locked --offline -p aep-ess-evidence --test count_reader_pass1 closed_scenario_values_and_shapes_do_not_close_literal_payload_keys -- --exact --nocapture
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-3709f17a590149b8)

running 1 test
test closed_scenario_values_and_shapes_do_not_close_literal_payload_keys ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

exit: 0
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 typed_batch_preserves_crlf_and_unicode_bytes_and_refuses_either_bad_position -- --exact --nocapture
   Compiling aep-domain v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/govern/aep-domain)
   Compiling aep-contract v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-contract)
   Compiling aep-driver-spec v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/drive/aep-driver-spec)
   Compiling aep-render v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/drive/aep-render)
   Compiling aep-ess-evidence v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/observe/aep-ess-evidence)
   Compiling trace-spec v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/observe/trace-spec)
   Compiling aep-backend-memory v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-backend-memory)
   Compiling aep-engine v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/govern/aep-engine)
   Compiling aep-conformance v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-conformance)
   Compiling aep-backend-entity v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-backend-entity)
   Compiling aep-backend-markdown v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-backend-markdown)
   Compiling aep-backend-postgres v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-backend-postgres)
   Compiling aep-backend-sqlite v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-backend-sqlite)
   Compiling aep-schema v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-schema)
   Compiling aep-backend-hybrid v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/plan/aep-backend-hybrid)
   Compiling aep-driver v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/drive/aep-driver)
   Compiling aep-project v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-project)
   Compiling aep-cli v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-cli)
warning: unused import: `aep_engine::engine::ProtocolEngine`
  --> crates/edge/aep-cli/tests/count_reader_pass1.rs:12:5
   |
12 | use aep_engine::engine::ProtocolEngine;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

error[E0599]: no method named `evidence` found for struct `Execution` in the current scope
   --> crates/edge/aep-cli/tests/count_reader_pass1.rs:165:53
    |
165 |         reason(requirement.qualify_record(&restored.evidence()[0], &restored)),
    |                                                     ^^^^^^^^ private field, not a method
    |
   ::: crates/govern/aep-domain/src/requirement.rs:82:8
    |
 82 |     fn evidence(&self) -> &[EvidenceRecord];
    |        -------- the method is available for `Execution` here
    |
    = help: items from traits can only be used if the trait is in scope
help: there is a method `link_evidence` with a similar name, but with different arguments
   --> crates/govern/aep-engine/src/execution.rs:209:5
    |
209 |     pub fn link_evidence(&mut self, evidence: EvidenceId, entity: EntityRef) {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: trait `RequirementContext` which provides `evidence` is implemented but not in scope; perhaps you want to import it
    |
  2 + use aep_domain::RequirementContext;
    |

For more information about this error, try `rustc --explain E0599`.
warning: `aep-cli` (test "count_reader_pass1") generated 1 warning
error: could not compile `aep-cli` (test "count_reader_pass1") due to 1 previous error; 1 warning emitted
warning: build failed, waiting for other jobs to finish...
exit: 101
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 exact_high_time_horizon_restore_and_changed_source_refusals_preserve_execution -- --exact --nocapture
   Compiling aep-cli v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-cli)
warning: unused import: `aep_engine::engine::ProtocolEngine`
  --> crates/edge/aep-cli/tests/count_reader_pass1.rs:12:5
   |
12 | use aep_engine::engine::ProtocolEngine;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

error[E0599]: no method named `evidence` found for struct `Execution` in the current scope
   --> crates/edge/aep-cli/tests/count_reader_pass1.rs:165:53
    |
165 |         reason(requirement.qualify_record(&restored.evidence()[0], &restored)),
    |                                                     ^^^^^^^^ private field, not a method
    |
   ::: crates/govern/aep-domain/src/requirement.rs:82:8
    |
 82 |     fn evidence(&self) -> &[EvidenceRecord];
    |        -------- the method is available for `Execution` here
    |
    = help: items from traits can only be used if the trait is in scope
help: there is a method `link_evidence` with a similar name, but with different arguments
   --> crates/govern/aep-engine/src/execution.rs:209:5
    |
209 |     pub fn link_evidence(&mut self, evidence: EvidenceId, entity: EntityRef) {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: trait `RequirementContext` which provides `evidence` is implemented but not in scope; perhaps you want to import it
    |
  2 + use aep_domain::RequirementContext;
    |

For more information about this error, try `rustc --explain E0599`.
warning: `aep-cli` (test "count_reader_pass1") generated 1 warning
error: could not compile `aep-cli` (test "count_reader_pass1") due to 1 previous error; 1 warning emitted
exit: 101
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output -- --exact --nocapture
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling aep-cli v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-cli)
warning: unused import: `aep_engine::engine::ProtocolEngine`
  --> crates/edge/aep-cli/tests/count_reader_pass1.rs:12:5
   |
12 | use aep_engine::engine::ProtocolEngine;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

error[E0599]: no method named `evidence` found for struct `Execution` in the current scope
   --> crates/edge/aep-cli/tests/count_reader_pass1.rs:165:53
    |
165 |         reason(requirement.qualify_record(&restored.evidence()[0], &restored)),
    |                                                     ^^^^^^^^ private field, not a method
    |
   ::: crates/govern/aep-domain/src/requirement.rs:82:8
    |
 82 |     fn evidence(&self) -> &[EvidenceRecord];
    |        -------- the method is available for `Execution` here
    |
    = help: items from traits can only be used if the trait is in scope
help: there is a method `link_evidence` with a similar name, but with different arguments
   --> crates/govern/aep-engine/src/execution.rs:209:5
    |
209 |     pub fn link_evidence(&mut self, evidence: EvidenceId, entity: EntityRef) {
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: trait `RequirementContext` which provides `evidence` is implemented but not in scope; perhaps you want to import it
    |
  2 + use aep_domain::RequirementContext;
    |

For more information about this error, try `rustc --explain E0599`.
warning: `aep-cli` (test "count_reader_pass1") generated 1 warning
error: could not compile `aep-cli` (test "count_reader_pass1") due to 1 previous error; 1 warning emitted
exit: 101
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 typed_batch_preserves_crlf_and_unicode_bytes_and_refuses_either_bad_position -- --exact --nocapture
   Compiling aep-cli v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-cli)
    Finished `test` profile [unoptimized] target(s) in 0.80s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-4d19c18382a6d073)

running 1 test
test typed_batch_preserves_crlf_and_unicode_bytes_and_refuses_either_bad_position ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.01s

exit: 0
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 exact_high_time_horizon_restore_and_changed_source_refusals_preserve_execution -- --exact --nocapture
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-4d19c18382a6d073)

running 1 test
test exact_high_time_horizon_restore_and_changed_source_refusals_preserve_execution ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.09s

exit: 0
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output -- --exact --nocapture
    Finished `test` profile [unoptimized] target(s) in 0.14s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-4d19c18382a6d073)

running 1 test

thread 'actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output' (3761805) panicked at crates/edge/aep-cli/tests/count_reader_pass1.rs:213:5:
assertion failed: String::from_utf8_lossy(&output.stderr).contains("future")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output ... FAILED

failures:

failures:
    actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p aep-cli --test count_reader_pass1`
exit: 101
```

```text
cargo test --locked --offline -p aep-cli --test count_reader_pass1 actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output -- --exact --nocapture
   Compiling aep-cli v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-cli)
    Finished `test` profile [unoptimized] target(s) in 0.72s
     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-4d19c18382a6d073)

running 1 test
test actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.02s

exit: 0
```

3. Required package suite

Baseline 1,313 passed, zero failed/ignored across 59 runner summaries was supplied by the coordinator from the final implementation handoff (SHA256 edaa883c03ccf116fba536b00e9600e096f4d7723f02c16084b7d9a1d15a2571); no unchanged baseline suite was rerun. The complete five-package suite followed all focused outputs above. It executed 1,320 passed, zero failed/ignored across 61 runner summaries, exit 0. The seven-case increase matches the seven new cases. Header red 0 describes the final package suite; preliminary new-test errors are preserved separately above. New-file rustfmt check also exited 0.

Command and combined stdout/stderr, verbatim:

```text
cargo test --locked --offline --no-fail-fast -p aep-ess-evidence -p aep-domain -p aep-engine -p aep-schema -p aep-cli
   Compiling aep-cli v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/edge/aep-cli)
   Compiling aep-ess-evidence v0.54.0 (~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/crates/observe/aep-ess-evidence)
    Finished `test` profile [unoptimized] target(s) in 0.80s
     Running unittests src/aep.rs (target/debug/deps/aep-1b91394d0d0670f1)

running 218 tests
test cli_reference::a_verb_the_reference_does_not_spell_is_reported_by_name ... ok
test contract::tests::a_count_stated_as_null_is_refused_by_the_name_of_the_count ... ok
test contract::tests::a_count_the_record_never_states_is_refused_rather_than_defaulted_to_zero ... ok
test contract::tests::a_lone_hyphen_is_the_pipe_and_everything_else_is_a_file ... ok
test contract::tests::a_record_of_another_kind_is_refused_by_the_kind_it_states ... ok
test contract::tests::a_failing_contract_run_is_read_rather_than_refused ... ok
test contract::tests::a_record_that_checked_nothing_is_refused_with_the_reason_named ... ok
test contract::tests::a_record_claiming_more_breaking_changes_than_failures_is_refused ... ok
test contract::tests::a_record_whose_failures_are_all_breaking_is_accepted ... ok
test contract::tests::the_provenance_digest_describes_the_bytes_the_runner_printed ... ok
test contract::tests::the_providers_own_bytes_are_the_payload_this_repository_defines ... ok
test doctor::tests::a_slugged_or_prefixed_tag_is_not_a_bare_version_and_cannot_win ... ok
test doctor::tests::a_plugin_directory_without_a_manifest_fails_naming_both_manifests_it_looked_for ... ok
test doctor::tests::a_snapshot_path_is_addressed_by_repository_and_revision_together ... ok
test drive::tests::a_claude_step_can_be_pointed_at_the_same_gateway_as_the_native_loop ... ok
test drive::tests::a_command_step_naming_this_cli_is_resolved_to_the_binary_this_process_is ... ok
test drive::tests::a_failing_command_mints_a_record_that_says_so_and_a_failed_diff_mints_nothing ... ok
test drive::tests::a_native_step_is_told_which_programs_it_may_start ... ok
test drive::tests::a_metacharacter_inside_quotes_is_an_argument_and_outside_them_it_composes ... ok
test drive::tests::a_b10x_step_is_told_the_b10x_catalogues_names_and_never_claude_codes ... ok
test drive::tests::a_session_path_matches_what_metaharness_constructs ... ok
test drive::tests::a_reading_state_may_read_at_scale_and_still_cannot_write_by_any_route ... ok
test drive::tests::a_map_with_an_llm_step_is_refused_at_launch_when_the_seams_binary_is_missing ... ok
test drive::tests::a_person_the_system_a_service_and_the_run_itself_are_refused_as_approvers_before_the_run ... ok
test drive::tests::a_driver_that_cannot_name_itself_refuses_a_map_whose_commands_say_protocol ... ok
test drive::tests::a_confined_workspace_gets_the_flags_that_let_the_arm_write_and_an_ordinary_one_does_not ... ok
test drive::tests::a_record_that_is_missing_or_does_not_read_submits_nothing_and_says_why ... ok
test drive::tests::a_record_a_verifier_wrote_is_submitted_as_that_verifiers_and_never_minted_here ... ok
test command_tree::every_verb_an_area_groups_is_also_a_top_level_alias_for_the_same_subtree ... ok
test drive::tests::a_call_the_engine_refuses_is_denied_and_the_refusal_is_in_the_executions_event_record ... ok
test drive::tests::a_skill_load_is_admitted_without_the_engine_being_asked_to_invent_an_action ... ok
test drive::tests::a_subagent_spawner_is_never_rendered_whatever_is_admitted ... ok
test drive::tests::a_subscription_source_and_a_dialect_reach_metaharness_as_flags_and_the_token_does_not ... ok
test drive::tests::a_state_that_admits_everything_writes_no_specification ... ok
test drive::tests::admitted_and_refused_operations_partition_the_vocabulary ... ok
test drive::tests::a_steps_skills_are_asked_for_in_the_prompt ... ok
test drive::tests::a_resume_inherits_or_narrows_its_cap_and_never_widens_it ... ok
test drive::tests::the_b10x_argv_carries_the_scope_and_never_the_frame_that_loop_would_refuse ... ok
test drive::tests::a_tool_outside_the_states_surface_is_denied_with_the_surface_named ... ok
test drive::tests::an_observed_session_is_counted_and_never_reported_as_a_clean_adjudication ... ok
test drive::tests::an_llm_sessions_launch_declares_the_run_as_its_actor_and_that_actor_cannot_approve_the_run ... ok
test drive::tests::an_approval_read_out_of_a_file_is_refused_however_well_formed_it_is ... ok
test drive::tests::the_committed_golden_is_the_document_the_driver_would_write ... ok
test drive::tests::an_unmet_outgoing_guard_is_named_in_the_prompt_under_the_reaching_heading ... ok
test drive::tests::paid_maps_require_exact_explicit_terms_and_command_maps_do_not ... ok
test drive::tests::each_offered_tool_renders_as_the_action_it_is_and_two_render_as_none ... ok
test drive::tests::an_approver_is_parsed_as_an_actor_and_needs_a_run_that_can_stop ... ok
test drive::tests::the_metaharness_argv_drives_the_seam_with_the_declared_directory_and_frame ... ok
test drive::tests::the_metaharness_operations_mirror_the_allowed_tools_decisions ... ok
test drive::tests::the_frame_carries_the_engines_lines_and_the_steps_coordinates ... ok
test drive::tests::the_prompt_names_the_task_the_run_drives_before_the_maps_own_words ... ok
test drive::tests::the_planning_stores_frontmatter_is_the_clis ... ok
test drive::tests::the_prompt_names_the_tools_the_state_admits_and_the_policy_agrees ... ok
test drive::tests::the_prompt_states_the_shell_rules_the_policy_will_refuse_on ... ok
test drive::tests::the_frame_compiles_the_steps_write_scope_into_ordered_subject_rules ... ok
test drive::tests::the_frame_document_is_sealed_by_the_rule_metaharness_verifies ... ok
test drive::tests::the_rendering_offers_a_shell_only_when_the_capability_is_admitted ... ok
test drive::tests::the_next_assumed_charge_is_persisted_only_when_it_fits ... ok
test drive::tests::a_call_the_policy_refuses_is_attributed_to_the_policy_and_never_reaches_the_engine ... ok
test drive::tests::the_shared_tool_decision_renders_into_two_vocabularies_and_is_taken_once ... ok
test drive::tests::the_tool_audit_reads_the_list_each_harness_answers_in ... ok
test drive::tests::the_write_scope_words_are_the_ones_the_step_map_is_written_in ... ok
test drive::tests::a_flag_a_refusal_names_is_a_flag_a_drive_verb_parses ... ok
test eval::native_arm_tests::the_arms_still_sort_in_the_order_the_experiment_runs_them ... ok
test eval::native_arm_tests::every_arm_has_a_code_of_its_own_and_none_is_reused ... ok
test drive::tests::the_shell_surface_admits_the_grouped_spelling_of_the_same_two_verbs ... ok
test drive::tests::the_task_placeholder_is_the_document_this_run_was_started_from ... ok
test eval::tests::a_cost_a_harness_computed_in_floating_point_is_read_and_not_refused ... ok
test eval::tests::a_cells_resource_total_says_how_many_runs_it_covers ... ok
test drive::tests::the_shell_surface_is_one_simple_protocol_invocation ... ok
test eval::native_arm_tests::the_fourth_arm_is_a_word_the_manifest_reads_and_writes ... ok
test drive::tests::the_refusal_specification_is_a_specification_the_checker_reads ... ok
test eval::tests::a_case_whose_subject_names_the_ess_skill_says_it_needs_ess ... ok
test eval::tests::a_manifest_that_describes_another_run_than_its_record_is_refused ... ok
test eval::tests::a_digest_that_is_not_a_digest_is_refused ... ok
test eval::tests::a_document_that_does_not_claim_the_format_is_refused_before_its_fields_are_believed ... ok
test eval::tests::a_marketplace_plugin_is_a_treatment_and_a_run_with_one_needs_no_directory_digest ... ok
test eval::tests::a_person_typing_an_amount_is_still_held_to_an_exact_one ... ok
test eval::tests::a_missing_field_is_refused_by_its_own_name_and_every_other_refusal_is_reported_beside_it ... ok
test eval::tests::a_model_that_is_written_and_empty_is_refused_rather_than_read_as_unstated ... ok
test eval::tests::a_marketplace_plugin_whose_digest_is_not_one_is_refused_by_the_field_that_is_wrong ... ok
test eval::tests::a_null_plugin_digest_on_arm_plugin_is_refused_because_the_plugin_is_the_subject ... ok
test eval::tests::a_pinned_plugin_reaches_the_argv_with_the_bytes_the_operator_wrote ... ok
test eval::tests::a_record_of_another_shape_is_refused_by_the_format_it_states ... ok
test eval::tests::a_marketplace_plugin_on_arm_raw_is_refused_for_the_reason_a_digest_is ... ok
test eval::tests::a_row_whose_verdict_is_null_is_unobservable_and_never_held ... ok
test eval::tests::a_plugin_digest_on_arm_raw_is_refused_because_arm_raw_is_the_arm_without_one ... ok
test eval::tests::a_stated_cost_this_reader_cannot_convert_is_refused_rather_than_read_as_no_cost ... ok
test drive::tests::two_mints_of_the_same_step_are_the_same_document ... ok
test eval::tests::a_run_that_states_no_cost_writes_no_cost_key_rather_than_a_zero ... ok
test eval::tests::a_usage_key_written_null_contributes_nothing_and_does_not_erase_the_total ... ok
test eval::tests::a_table_holding_a_native_cell_says_how_to_read_it_and_one_without_stays_silent ... ok
test eval::tests::an_amount_this_reader_cannot_convert_exactly_is_refused_rather_than_rounded ... ok
test eval::tests::a_verdict_word_this_build_cannot_read_is_refused_rather_than_bucketed ... ok
test eval::tests::an_amount_becomes_millionths_by_integer_arithmetic_and_never_by_a_float ... ok
test eval::tests::an_unpinned_plugin_is_refused_in_metaharness_own_words_and_never_defaulted ... ok
test eval::tests::an_arm_this_evaluation_does_not_have_is_refused_by_name ... ok
test eval::tests::arm_plugin_that_names_neither_mechanism_is_still_refused ... ok
test eval::tests::an_omitted_plugin_digest_is_refused_and_an_explicit_null_is_not ... ok
test eval::tests::an_omitted_model_is_refused_and_an_explicit_null_is_not ... ok
test drive::tests::a_map_naming_the_b10x_harness_is_refused_when_the_run_cannot_say_where_to_point_it ... ok
test command_tree::the_first_level_is_the_four_areas_and_doctor ... ok
test eval::tests::arm_driven_may_answer_either_way_because_the_enforcer_is_not_the_plugin ... ok
test eval::native_arm_tests::a_native_run_is_refused_a_spawn_and_told_what_does_launch_it ... ok
test command_tree::every_flat_spelling_the_last_release_answered_still_resolves_and_is_hidden ... ok
test eval::tests::micro_dollars_render_as_the_plain_decimal_a_vendor_flag_takes ... ok
test eval::tests::one_specification_at_two_digests_is_refused_because_the_rows_share_a_name_only ... ok
test eval::tests::every_arm_is_spawned_by_one_instrument_and_only_the_treatment_varies ... ok
test eval::tests::the_arms_sort_in_the_order_the_experiment_runs_them ... ok
test eval::tests::one_transcript_cannot_arrive_twice_because_one_run_would_be_counted_twice ... ok
test eval::tests::the_arms_the_refusal_lists_are_every_arm_the_type_has ... ok
test eval::tests::each_required_record_key_is_refused_when_omitted ... ok
test eval::tests::no_rendering_of_a_matrix_contains_a_score ... ok
test eval::tests::the_childs_path_is_the_one_metaharness_constructs ... ok
test eval::tests::the_terminal_events_cost_is_read_from_its_own_decimal_text ... ok
test eval::tests::the_model_is_forwarded_verbatim_and_before_the_treatment ... ok
test eval::tests::the_version_is_the_first_token_that_starts_with_a_digit ... ok
test eval::tests::what_is_left_of_the_cap_travels_to_a_claude_run_and_to_no_other ... ok
test eval::tests::the_three_verdicts_map_onto_the_three_columns ... ok
test eval::tests::the_manifest_the_runner_assembles_is_one_the_matrixs_own_reader_reads ... ok
test eval::tests::the_honest_manifest_is_read_so_every_mutation_below_reaches_its_rule ... ok
test eval::tests::required_record_keys_distinguish_null_empty_and_malformed ... ok
test evidence_doc::tests::the_producer_is_a_verifier_and_no_argument_can_change_it ... ok
test evidence_doc::tests::text_renders_the_document_rather_than_a_summary_of_it ... ok
test evidence_doc::tests::what_is_written_is_what_the_evidence_reader_parses ... ok
test eval::tests::one_session_that_wrote_two_terminal_records_is_charged_once ... ok
test cli_reference::every_verb_the_cli_answers_has_an_entry_in_the_reference ... ok
test eval::tests::a_transcript_of_several_sessions_totals_them_rather_than_reporting_the_last_one ... ok
test flow::tests::the_document_says_what_it_dropped_rather_than_dropping_it_quietly ... ok
test flow::tests::a_bound_the_caller_names_is_the_bound_that_is_written ... ok
test flow::tests::a_map_pinned_to_another_version_is_refused_in_the_words_the_driver_refuses_it_in ... ok
test flow::tests::a_projection_without_a_map_is_the_document_it_always_was ... ok
test flow::tests::what_this_verb_emits_is_a_document_the_harness_plans ... ok
test drive::tests::a_whole_file_store_rewrite_is_refused_by_the_committed_maps_declaration ... ok
test flow::tests::the_three_retreats_become_one_repeating_group_because_they_span_one_stretch ... ok
test money::tests::typed_dollar_amounts_are_exact_or_refused ... ok
test planning::tests::an_edge_argument_splits_at_the_first_colon_only ... ok
test planning::tests::a_declared_actor_is_who_the_write_is_from_and_an_undeclared_one_is_the_logged_in_person ... ok
test planning::tests::a_malformed_declared_actor_is_refused_naming_the_variable_and_never_defaulted ... ok
test planning::waves::tests::a_collision_with_one_inferred_side_is_marked_inferred ... ok
test planning::waves::tests::a_collision_moves_a_story_to_the_first_wave_with_room_and_not_to_the_end ... ok
test planning::waves::tests::a_cycle_is_reported_once_from_its_lowest_id_and_nothing_is_placed ... ok
test planning::waves::tests::a_dependency_on_something_outside_the_selection_does_not_hold_a_story_back ... ok
test property::tests::the_run_is_exhaustive_over_the_whole_space_rather_than_a_sample_of_it ... ok
test planning::waves::tests::a_dependency_through_an_unassessed_story_still_orders_the_two_that_are_placed ... ok
test property::tests::a_law_that_does_not_hold_is_named_with_the_assignment_that_broke_it ... ok
test redaction::tests::a_git_author_is_removed_where_the_user_name_would_not_have_been ... ok
test redaction::tests::a_user_name_is_replaced_as_a_word_and_never_inside_another_one ... ok
test redaction::tests::an_operator_named_user_does_not_make_the_placeholder_match_itself ... ok
test redaction::tests::redaction_is_idempotent_so_a_stream_can_be_cleaned_twice ... ok
test render::tests::a_watch_stops_at_a_completed_run_and_keeps_looking_at_a_blocked_one ... ok
test render::tests::every_driver_status_has_a_rendering_status_and_none_of_them_is_unknown ... ok
test redaction::tests::redaction_removes_both_spellings_of_the_home_and_then_the_name ... ok
test property::tests::the_record_names_the_property_the_case_count_and_a_verifier_that_is_not_the_agent ... ok
test drive::tests::the_committed_step_map_compiles_into_the_exact_argv_a_native_run_is_launched_with ... ok
test eval::tests::both_spellings_of_the_ess_plugin_trip_the_preflight ... ok
test reverse::relations::tests::a_composition_that_states_one_shape_is_read_as_that_shape ... ok
test reverse::relations::tests::a_camel_case_id_field_is_read_the_same_way ... ok
test flow::tests::a_state_the_map_is_silent_about_keeps_the_payload_it_always_had ... ok
test reverse::tests::a_call_shaped_marker_is_bounded_only_at_its_front ... ok
test reverse::relations::tests::a_composition_that_states_two_shapes_leaves_cardinality_unmapped ... ok
test reverse::relations::tests::a_ref_to_a_schema_that_becomes_an_entity_references_one_of_it ... ok
test reverse::tests::a_timestamp_becomes_the_day_it_names ... ok
test reverse::tests::a_tracker_key_may_carry_digits_but_a_standard_is_not_a_ticket ... ok
test reverse::tests::a_wire_name_is_kebab_case ... ok
test reverse::tests::every_listed_extension_is_findable ... ok
test reverse::tests::the_language_table_is_sorted ... ok
test reverse::relations::tests::an_id_field_matching_an_entitys_identity_leaves_ownership_unmapped ... ok
test reverse::tests::truncation_cuts_on_a_character_boundary ... ok
test serve::api::tests::a_host_this_server_does_not_answer_to_is_refused ... ok
test serve::api::tests::a_path_this_server_does_not_answer_is_a_not_found_rather_than_a_guess ... ok
test reverse::relations::tests::no_signal_is_no_relation ... ok
test serve::api::tests::a_request_without_the_run_token_is_refused_even_from_localhost ... ok
test serve::api::tests::a_transition_is_refused_by_name_when_the_server_is_read_only ... ok
test serve::api::tests::a_write_from_another_origin_is_refused ... ok
test serve::http::tests::a_body_is_read_to_its_stated_length_and_no_further ... ok
test serve::api::tests::the_page_is_served_at_the_root_and_carries_no_remote_reference ... ok
test serve::http::tests::a_body_longer_than_the_cap_is_refused_before_it_is_allocated ... ok
test serve::http::tests::a_chunked_body_is_refused_with_the_header_that_would_have_worked ... ok
test serve::http::tests::a_header_name_is_matched_however_the_client_spelled_it ... ok
test serve::http::tests::a_method_this_server_does_not_answer_is_refused_naming_the_two_it_does ... ok
test serve::http::tests::a_query_string_is_parsed_and_percent_decoded ... ok
test serve::http::tests::a_request_line_without_a_version_is_refused_rather_than_guessed ... ok
test serve::http::tests::a_version_this_server_does_not_speak_is_refused_by_name ... ok
test serve::tests::a_run_token_is_long_enough_to_be_worth_having_and_differs_between_runs ... ok
test specification::tests::a_quoted_filename_does_not_outrank_the_comparison_beside_it ... ok
test specification::tests::a_requirement_is_a_list_item_under_a_requirements_or_acceptance_heading ... ok
test specification::tests::a_requirement_whose_spans_are_all_prose_names_what_was_tried ... ok
test specification::tests::a_requirement_is_unmet_unless_its_own_predicate_is_observed_true ... ok
test specification::tests::a_requirement_wrapped_over_two_lines_is_one_requirement ... ok
test specification::tests::a_run_that_has_observed_nothing_satisfies_no_requirement ... ok
test specification::tests::a_specification_that_states_no_requirement_does_not_satisfy_the_principle ... ok
test redaction::tests::an_operator_with_nothing_to_remove_leaves_the_stream_byte_identical ... ok
test flow::tests::without_a_map_a_node_says_which_state_it_is_and_nothing_more ... ok
test specification::tests::only_a_specification_in_force_is_one_a_task_may_be_implemented_against ... ok
test specification::tests::a_task_that_declares_no_story_is_matched_by_a_specification_of_the_task_itself ... ok
test specification::tests::an_artifact_named_on_the_command_line_may_be_a_draft_of_this_tasks_work ... ok
test specification::tests::the_predicate_is_the_span_that_parses_rather_than_the_first_one ... ok
test redaction::tests::a_common_one_word_git_name_is_left_alone_rather_than_corrupting_the_stream ... ok
test specification::tests::an_artifact_named_on_the_command_line_still_has_to_specify_this_task ... ok
test specification::tests::another_storys_approved_specification_is_not_this_tasks ... ok
test reverse::relations::tests::a_schema_whose_properties_are_all_scalars_has_no_relations ... ok
test specification::tests::with_no_task_in_reach_the_stores_one_in_force_specification_is_still_decided ... ok
test specification::tests::the_specification_decided_is_the_one_that_specifies_this_tasks_work ... ok
test specification::tests::two_specifications_of_this_tasks_work_are_refused_and_both_are_named ... ok
test specification::tests::the_rule_this_verb_selects_by_is_the_one_the_shipped_principles_declare ... ok
test flow::tests::the_header_names_the_map_and_the_workflow_it_is_pinned_to ... ok
test flow::tests::every_state_that_runs_lands_somewhere_and_the_terminals_do_not ... ok
test reverse::tests::a_marker_has_to_stand_on_its_own ... ok
test reverse::relations::tests::an_array_of_that_ref_references_many_of_it ... ok
test flow::tests::a_state_with_one_llm_step_carries_the_prompt_the_scope_and_the_harness ... ok
test flow::tests::a_state_with_several_steps_becomes_a_group_chained_in_the_order_the_map_wrote_them ... ok
test flow::tests::a_command_step_carries_its_argv_and_the_evidence_running_it_establishes ... ok
test flow::tests::the_scope_of_a_step_is_written_in_the_order_the_map_wrote_it ... ok
test flow::tests::an_operator_step_carries_what_it_asks_and_a_state_of_one_step_is_a_section_of_one ... ok
test flow::tests::every_state_is_a_section_so_a_governor_asked_at_every_section_is_asked_at_every_state ... ok
test flow::tests::a_projection_with_a_map_is_still_a_document_the_harness_plans ... ok
test command_tree::every_leaf_is_reachable_by_its_grouped_path_and_by_its_flat_spelling ... ok

test result: ok. 218 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running unittests src/protocol.rs (target/debug/deps/protocol-0bce6b5ccd5ed681)

running 218 tests
test cli_reference::a_verb_the_reference_does_not_spell_is_reported_by_name ... ok
test contract::tests::a_count_stated_as_null_is_refused_by_the_name_of_the_count ... ok
test contract::tests::a_count_the_record_never_states_is_refused_rather_than_defaulted_to_zero ... ok
test contract::tests::a_failing_contract_run_is_read_rather_than_refused ... ok
test contract::tests::a_lone_hyphen_is_the_pipe_and_everything_else_is_a_file ... ok
test contract::tests::a_record_of_another_kind_is_refused_by_the_kind_it_states ... ok
test contract::tests::a_record_that_checked_nothing_is_refused_with_the_reason_named ... ok
test contract::tests::a_record_claiming_more_breaking_changes_than_failures_is_refused ... ok
test contract::tests::a_record_whose_failures_are_all_breaking_is_accepted ... ok
test contract::tests::the_provenance_digest_describes_the_bytes_the_runner_printed ... ok
test contract::tests::the_providers_own_bytes_are_the_payload_this_repository_defines ... ok
test doctor::tests::a_slugged_or_prefixed_tag_is_not_a_bare_version_and_cannot_win ... ok
test doctor::tests::a_plugin_directory_without_a_manifest_fails_naming_both_manifests_it_looked_for ... ok
test doctor::tests::a_snapshot_path_is_addressed_by_repository_and_revision_together ... ok
test drive::tests::a_claude_step_can_be_pointed_at_the_same_gateway_as_the_native_loop ... ok
test drive::tests::a_command_step_naming_this_cli_is_resolved_to_the_binary_this_process_is ... ok
test drive::tests::a_b10x_step_is_told_the_b10x_catalogues_names_and_never_claude_codes ... ok
test drive::tests::a_person_the_system_a_service_and_the_run_itself_are_refused_as_approvers_before_the_run ... ok
test drive::tests::a_metacharacter_inside_quotes_is_an_argument_and_outside_them_it_composes ... ok
test drive::tests::a_failing_command_mints_a_record_that_says_so_and_a_failed_diff_mints_nothing ... ok
test drive::tests::a_native_step_is_told_which_programs_it_may_start ... ok
test drive::tests::a_session_path_matches_what_metaharness_constructs ... ok
test drive::tests::a_map_with_an_llm_step_is_refused_at_launch_when_the_seams_binary_is_missing ... ok
test drive::tests::a_reading_state_may_read_at_scale_and_still_cannot_write_by_any_route ... ok
test drive::tests::a_record_that_is_missing_or_does_not_read_submits_nothing_and_says_why ... ok
test drive::tests::a_resume_inherits_or_narrows_its_cap_and_never_widens_it ... ok
test drive::tests::a_steps_skills_are_asked_for_in_the_prompt ... ok
test drive::tests::a_subscription_source_and_a_dialect_reach_metaharness_as_flags_and_the_token_does_not ... ok
test drive::tests::a_driver_that_cannot_name_itself_refuses_a_map_whose_commands_say_protocol ... ok
test drive::tests::a_confined_workspace_gets_the_flags_that_let_the_arm_write_and_an_ordinary_one_does_not ... ok
test drive::tests::a_tool_outside_the_states_surface_is_denied_with_the_surface_named ... ok
test drive::tests::admitted_and_refused_operations_partition_the_vocabulary ... ok
test drive::tests::a_record_a_verifier_wrote_is_submitted_as_that_verifiers_and_never_minted_here ... ok
test drive::tests::a_call_the_policy_refuses_is_attributed_to_the_policy_and_never_reaches_the_engine ... ok
test drive::tests::a_call_the_engine_refuses_is_denied_and_the_refusal_is_in_the_executions_event_record ... ok
test drive::tests::an_approval_read_out_of_a_file_is_refused_however_well_formed_it_is ... ok
test drive::tests::an_llm_sessions_launch_declares_the_run_as_its_actor_and_that_actor_cannot_approve_the_run ... ok
test drive::tests::each_offered_tool_renders_as_the_action_it_is_and_two_render_as_none ... ok
test drive::tests::an_observed_session_is_counted_and_never_reported_as_a_clean_adjudication ... ok
test drive::tests::an_unmet_outgoing_guard_is_named_in_the_prompt_under_the_reaching_heading ... ok
test drive::tests::an_approver_is_parsed_as_an_actor_and_needs_a_run_that_can_stop ... ok
test drive::tests::a_subagent_spawner_is_never_rendered_whatever_is_admitted ... ok
test drive::tests::the_b10x_argv_carries_the_scope_and_never_the_frame_that_loop_would_refuse ... ok
test drive::tests::a_state_that_admits_everything_writes_no_specification ... ok
test drive::tests::the_metaharness_argv_drives_the_seam_with_the_declared_directory_and_frame ... ok
test drive::tests::the_metaharness_operations_mirror_the_allowed_tools_decisions ... ok
test drive::tests::the_frame_carries_the_engines_lines_and_the_steps_coordinates ... ok
test drive::tests::the_committed_golden_is_the_document_the_driver_would_write ... ok
test drive::tests::the_frame_compiles_the_steps_write_scope_into_ordered_subject_rules ... ok
test drive::tests::the_prompt_names_the_task_the_run_drives_before_the_maps_own_words ... ok
test drive::tests::the_frame_document_is_sealed_by_the_rule_metaharness_verifies ... ok
test drive::tests::paid_maps_require_exact_explicit_terms_and_command_maps_do_not ... ok
test drive::tests::the_next_assumed_charge_is_persisted_only_when_it_fits ... ok
test drive::tests::the_planning_stores_frontmatter_is_the_clis ... ok
test drive::tests::a_skill_load_is_admitted_without_the_engine_being_asked_to_invent_an_action ... ok
test command_tree::every_flat_spelling_the_last_release_answered_still_resolves_and_is_hidden ... ok
test drive::tests::a_map_naming_the_b10x_harness_is_refused_when_the_run_cannot_say_where_to_point_it ... ok
test command_tree::the_first_level_is_the_four_areas_and_doctor ... ok
test command_tree::every_verb_an_area_groups_is_also_a_top_level_alias_for_the_same_subtree ... ok
test drive::tests::a_flag_a_refusal_names_is_a_flag_a_drive_verb_parses ... ok
test cli_reference::every_verb_the_cli_answers_has_an_entry_in_the_reference ... ok
test drive::tests::the_rendering_offers_a_shell_only_when_the_capability_is_admitted ... ok
test drive::tests::the_tool_audit_reads_the_list_each_harness_answers_in ... ok
test drive::tests::the_shared_tool_decision_renders_into_two_vocabularies_and_is_taken_once ... ok
test drive::tests::the_prompt_names_the_tools_the_state_admits_and_the_policy_agrees ... ok
test drive::tests::the_shell_surface_admits_the_grouped_spelling_of_the_same_two_verbs ... ok
test drive::tests::the_committed_step_map_compiles_into_the_exact_argv_a_native_run_is_launched_with ... ok
test drive::tests::the_task_placeholder_is_the_document_this_run_was_started_from ... ok
test drive::tests::the_prompt_states_the_shell_rules_the_policy_will_refuse_on ... ok
test drive::tests::the_shell_surface_is_one_simple_protocol_invocation ... ok
test drive::tests::the_write_scope_words_are_the_ones_the_step_map_is_written_in ... ok
test eval::native_arm_tests::a_native_run_is_refused_a_spawn_and_told_what_does_launch_it ... ok
test eval::native_arm_tests::every_arm_has_a_code_of_its_own_and_none_is_reused ... ok
test eval::native_arm_tests::the_arms_still_sort_in_the_order_the_experiment_runs_them ... ok
test drive::tests::the_refusal_specification_is_a_specification_the_checker_reads ... ok
test eval::native_arm_tests::the_fourth_arm_is_a_word_the_manifest_reads_and_writes ... ok
test eval::tests::a_cost_a_harness_computed_in_floating_point_is_read_and_not_refused ... ok
test eval::tests::a_cells_resource_total_says_how_many_runs_it_covers ... ok
test eval::tests::a_manifest_that_describes_another_run_than_its_record_is_refused ... ok
test eval::tests::a_digest_that_is_not_a_digest_is_refused ... ok
test drive::tests::two_mints_of_the_same_step_are_the_same_document ... ok
test eval::tests::a_document_that_does_not_claim_the_format_is_refused_before_its_fields_are_believed ... ok
test eval::tests::a_marketplace_plugin_is_a_treatment_and_a_run_with_one_needs_no_directory_digest ... ok
test eval::tests::a_model_that_is_written_and_empty_is_refused_rather_than_read_as_unstated ... ok
test eval::tests::a_person_typing_an_amount_is_still_held_to_an_exact_one ... ok
test eval::tests::a_pinned_plugin_reaches_the_argv_with_the_bytes_the_operator_wrote ... ok
test eval::tests::a_marketplace_plugin_whose_digest_is_not_one_is_refused_by_the_field_that_is_wrong ... ok
test eval::tests::a_marketplace_plugin_on_arm_raw_is_refused_for_the_reason_a_digest_is ... ok
test eval::tests::a_missing_field_is_refused_by_its_own_name_and_every_other_refusal_is_reported_beside_it ... ok
test eval::tests::a_null_plugin_digest_on_arm_plugin_is_refused_because_the_plugin_is_the_subject ... ok
test eval::tests::a_case_whose_subject_names_the_ess_skill_says_it_needs_ess ... ok
test eval::tests::a_row_whose_verdict_is_null_is_unobservable_and_never_held ... ok
test eval::tests::a_plugin_digest_on_arm_raw_is_refused_because_arm_raw_is_the_arm_without_one ... ok
test eval::tests::a_run_that_states_no_cost_writes_no_cost_key_rather_than_a_zero ... ok
test eval::tests::a_usage_key_written_null_contributes_nothing_and_does_not_erase_the_total ... ok
test eval::tests::a_record_of_another_shape_is_refused_by_the_format_it_states ... ok
test eval::tests::a_stated_cost_this_reader_cannot_convert_is_refused_rather_than_read_as_no_cost ... ok
test eval::tests::an_amount_becomes_millionths_by_integer_arithmetic_and_never_by_a_float ... ok
test eval::tests::a_verdict_word_this_build_cannot_read_is_refused_rather_than_bucketed ... ok
test eval::tests::a_table_holding_a_native_cell_says_how_to_read_it_and_one_without_stays_silent ... ok
test eval::tests::an_amount_this_reader_cannot_convert_exactly_is_refused_rather_than_rounded ... ok
test eval::tests::an_unpinned_plugin_is_refused_in_metaharness_own_words_and_never_defaulted ... ok
test eval::tests::an_arm_this_evaluation_does_not_have_is_refused_by_name ... ok
test eval::tests::an_omitted_plugin_digest_is_refused_and_an_explicit_null_is_not ... ok
test eval::tests::every_arm_is_spawned_by_one_instrument_and_only_the_treatment_varies ... ok
test eval::tests::micro_dollars_render_as_the_plain_decimal_a_vendor_flag_takes ... ok
test eval::tests::arm_plugin_that_names_neither_mechanism_is_still_refused ... ok
test eval::tests::an_omitted_model_is_refused_and_an_explicit_null_is_not ... ok
test eval::tests::one_specification_at_two_digests_is_refused_because_the_rows_share_a_name_only ... ok
test eval::tests::one_transcript_cannot_arrive_twice_because_one_run_would_be_counted_twice ... ok
test eval::tests::the_arms_sort_in_the_order_the_experiment_runs_them ... ok
test eval::tests::arm_driven_may_answer_either_way_because_the_enforcer_is_not_the_plugin ... ok
test eval::tests::the_arms_the_refusal_lists_are_every_arm_the_type_has ... ok
test eval::tests::the_childs_path_is_the_one_metaharness_constructs ... ok
test eval::tests::no_rendering_of_a_matrix_contains_a_score ... ok
test eval::tests::each_required_record_key_is_refused_when_omitted ... ok
test eval::tests::the_model_is_forwarded_verbatim_and_before_the_treatment ... ok
test eval::tests::the_honest_manifest_is_read_so_every_mutation_below_reaches_its_rule ... ok
test eval::tests::the_terminal_events_cost_is_read_from_its_own_decimal_text ... ok
test eval::tests::both_spellings_of_the_ess_plugin_trip_the_preflight ... ok
test eval::tests::the_version_is_the_first_token_that_starts_with_a_digit ... ok
test eval::tests::the_manifest_the_runner_assembles_is_one_the_matrixs_own_reader_reads ... ok
test eval::tests::what_is_left_of_the_cap_travels_to_a_claude_run_and_to_no_other ... ok
test eval::tests::the_three_verdicts_map_onto_the_three_columns ... ok
test evidence_doc::tests::the_producer_is_a_verifier_and_no_argument_can_change_it ... ok
test eval::tests::required_record_keys_distinguish_null_empty_and_malformed ... ok
test evidence_doc::tests::text_renders_the_document_rather_than_a_summary_of_it ... ok
test evidence_doc::tests::what_is_written_is_what_the_evidence_reader_parses ... ok
test eval::tests::one_session_that_wrote_two_terminal_records_is_charged_once ... ok
test eval::tests::a_transcript_of_several_sessions_totals_them_rather_than_reporting_the_last_one ... ok
test flow::tests::a_projection_without_a_map_is_the_document_it_always_was ... ok
test flow::tests::a_bound_the_caller_names_is_the_bound_that_is_written ... ok
test flow::tests::a_map_pinned_to_another_version_is_refused_in_the_words_the_driver_refuses_it_in ... ok
test flow::tests::every_state_that_runs_lands_somewhere_and_the_terminals_do_not ... ok
test drive::tests::a_whole_file_store_rewrite_is_refused_by_the_committed_maps_declaration ... ok
test flow::tests::what_this_verb_emits_is_a_document_the_harness_plans ... ok
test flow::tests::a_state_the_map_is_silent_about_keeps_the_payload_it_always_had ... ok
test flow::tests::the_document_says_what_it_dropped_rather_than_dropping_it_quietly ... ok
test planning::tests::a_declared_actor_is_who_the_write_is_from_and_an_undeclared_one_is_the_logged_in_person ... ok
test money::tests::typed_dollar_amounts_are_exact_or_refused ... ok
test planning::tests::a_malformed_declared_actor_is_refused_naming_the_variable_and_never_defaulted ... ok
test planning::tests::an_edge_argument_splits_at_the_first_colon_only ... ok
test planning::waves::tests::a_collision_with_one_inferred_side_is_marked_inferred ... ok
test planning::waves::tests::a_collision_moves_a_story_to_the_first_wave_with_room_and_not_to_the_end ... ok
test planning::waves::tests::a_dependency_on_something_outside_the_selection_does_not_hold_a_story_back ... ok
test property::tests::a_law_that_does_not_hold_is_named_with_the_assignment_that_broke_it ... ok
test planning::waves::tests::a_cycle_is_reported_once_from_its_lowest_id_and_nothing_is_placed ... ok
test planning::waves::tests::a_dependency_through_an_unassessed_story_still_orders_the_two_that_are_placed ... ok
test property::tests::the_run_is_exhaustive_over_the_whole_space_rather_than_a_sample_of_it ... ok
test redaction::tests::a_user_name_is_replaced_as_a_word_and_never_inside_another_one ... ok
test redaction::tests::a_git_author_is_removed_where_the_user_name_would_not_have_been ... ok
test redaction::tests::an_operator_named_user_does_not_make_the_placeholder_match_itself ... ok
test redaction::tests::redaction_is_idempotent_so_a_stream_can_be_cleaned_twice ... ok
test redaction::tests::redaction_removes_both_spellings_of_the_home_and_then_the_name ... ok
test render::tests::every_driver_status_has_a_rendering_status_and_none_of_them_is_unknown ... ok
test property::tests::the_record_names_the_property_the_case_count_and_a_verifier_that_is_not_the_agent ... ok
test flow::tests::without_a_map_a_node_says_which_state_it_is_and_nothing_more ... ok
test render::tests::a_watch_stops_at_a_completed_run_and_keeps_looking_at_a_blocked_one ... ok
test reverse::relations::tests::a_composition_that_states_one_shape_is_read_as_that_shape ... ok
test reverse::relations::tests::a_camel_case_id_field_is_read_the_same_way ... ok
test reverse::relations::tests::a_composition_that_states_two_shapes_leaves_cardinality_unmapped ... ok
test reverse::relations::tests::a_ref_to_a_schema_that_becomes_an_entity_references_one_of_it ... ok
test redaction::tests::a_common_one_word_git_name_is_left_alone_rather_than_corrupting_the_stream ... ok
test reverse::relations::tests::a_schema_whose_properties_are_all_scalars_has_no_relations ... ok
test reverse::tests::a_call_shaped_marker_is_bounded_only_at_its_front ... ok
test reverse::tests::a_marker_has_to_stand_on_its_own ... ok
test reverse::tests::a_timestamp_becomes_the_day_it_names ... ok
test reverse::tests::a_wire_name_is_kebab_case ... ok
test redaction::tests::an_operator_with_nothing_to_remove_leaves_the_stream_byte_identical ... ok
test reverse::tests::every_listed_extension_is_findable ... ok
test reverse::tests::the_language_table_is_sorted ... ok
test reverse::tests::truncation_cuts_on_a_character_boundary ... ok
test serve::api::tests::a_request_without_the_run_token_is_refused_even_from_localhost ... ok
test serve::api::tests::a_path_this_server_does_not_answer_is_a_not_found_rather_than_a_guess ... ok
test serve::api::tests::a_host_this_server_does_not_answer_to_is_refused ... ok
test serve::api::tests::a_transition_is_refused_by_name_when_the_server_is_read_only ... ok
test serve::api::tests::a_write_from_another_origin_is_refused ... ok
test serve::http::tests::a_body_is_read_to_its_stated_length_and_no_further ... ok
test serve::http::tests::a_body_longer_than_the_cap_is_refused_before_it_is_allocated ... ok
test serve::http::tests::a_chunked_body_is_refused_with_the_header_that_would_have_worked ... ok
test serve::http::tests::a_header_name_is_matched_however_the_client_spelled_it ... ok
test serve::http::tests::a_method_this_server_does_not_answer_is_refused_naming_the_two_it_does ... ok
test serve::http::tests::a_query_string_is_parsed_and_percent_decoded ... ok
test serve::http::tests::a_request_line_without_a_version_is_refused_rather_than_guessed ... ok
test serve::http::tests::a_version_this_server_does_not_speak_is_refused_by_name ... ok
test serve::tests::a_run_token_is_long_enough_to_be_worth_having_and_differs_between_runs ... ok
test specification::tests::a_quoted_filename_does_not_outrank_the_comparison_beside_it ... ok
test specification::tests::a_requirement_is_a_list_item_under_a_requirements_or_acceptance_heading ... ok
test specification::tests::a_requirement_whose_spans_are_all_prose_names_what_was_tried ... ok
test specification::tests::a_requirement_is_unmet_unless_its_own_predicate_is_observed_true ... ok
test specification::tests::a_requirement_wrapped_over_two_lines_is_one_requirement ... ok
test specification::tests::a_specification_that_states_no_requirement_does_not_satisfy_the_principle ... ok
test specification::tests::a_run_that_has_observed_nothing_satisfies_no_requirement ... ok
test reverse::tests::a_tracker_key_may_carry_digits_but_a_standard_is_not_a_ticket ... ok
test serve::api::tests::the_page_is_served_at_the_root_and_carries_no_remote_reference ... ok
test specification::tests::only_a_specification_in_force_is_one_a_task_may_be_implemented_against ... ok
test specification::tests::a_task_that_declares_no_story_is_matched_by_a_specification_of_the_task_itself ... ok
test specification::tests::the_predicate_is_the_span_that_parses_rather_than_the_first_one ... ok
test specification::tests::an_artifact_named_on_the_command_line_may_be_a_draft_of_this_tasks_work ... ok
test specification::tests::another_storys_approved_specification_is_not_this_tasks ... ok
test specification::tests::with_no_task_in_reach_the_stores_one_in_force_specification_is_still_decided ... ok
test specification::tests::the_specification_decided_is_the_one_that_specifies_this_tasks_work ... ok
test specification::tests::an_artifact_named_on_the_command_line_still_has_to_specify_this_task ... ok
test specification::tests::two_specifications_of_this_tasks_work_are_refused_and_both_are_named ... ok
test reverse::relations::tests::an_array_of_that_ref_references_many_of_it ... ok
test reverse::relations::tests::no_signal_is_no_relation ... ok
test reverse::relations::tests::an_id_field_matching_an_entitys_identity_leaves_ownership_unmapped ... ok
test flow::tests::the_three_retreats_become_one_repeating_group_because_they_span_one_stretch ... ok
test specification::tests::the_rule_this_verb_selects_by_is_the_one_the_shipped_principles_declare ... ok
test flow::tests::a_state_with_several_steps_becomes_a_group_chained_in_the_order_the_map_wrote_them ... ok
test flow::tests::every_state_is_a_section_so_a_governor_asked_at_every_section_is_asked_at_every_state ... ok
test flow::tests::the_header_names_the_map_and_the_workflow_it_is_pinned_to ... ok
test flow::tests::a_command_step_carries_its_argv_and_the_evidence_running_it_establishes ... ok
test flow::tests::a_state_with_one_llm_step_carries_the_prompt_the_scope_and_the_harness ... ok
test flow::tests::an_operator_step_carries_what_it_asks_and_a_state_of_one_step_is_a_section_of_one ... ok
test flow::tests::the_scope_of_a_step_is_written_in_the_order_the_map_wrote_it ... ok
test flow::tests::a_projection_with_a_map_is_still_a_document_the_harness_plans ... ok
test command_tree::every_leaf_is_reachable_by_its_grouped_path_and_by_its_flat_spelling ... ok

test result: ok. 218 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/cli.rs (target/debug/deps/cli-6ea07b6dbf4892a2)

running 38 tests
test conformance_fails_when_a_property_is_deliberately_broken ... ok
test a_repository_with_no_workspace_file_is_not_an_error ... ok
test a_scan_that_is_blind_to_an_annotation_fails_strict_and_says_which_file ... ok
test a_workspace_member_nobody_checked_out_is_reported_rather_than_fatal ... ok
test inspect_accepts_a_record_observed_on_the_reference_date_itself ... ok
test a_workspace_list_names_the_member_every_artifact_came_from ... ok
test conformance_refuses_a_store_for_the_backend_that_keeps_nothing ... ok
test inspect_refuses_an_observation_that_has_not_happened_yet ... ok
test an_expired_claim_fails_only_the_flag_that_exists_to_judge_it ... ok
test inspect_reports_when_each_submitted_record_was_observed ... ok
test a_reference_two_members_both_hold_is_refused_with_both_spellings ... ok
test conformance_rejects_an_unknown_level_or_fault ... ok
test outside_a_project_the_missing_task_is_explained ... ok
test conformance_runs_the_suites_against_the_reference_backend ... ok
test inspect_admits_a_date_that_is_today_at_utc_plus_fourteen_and_refuses_one_nowhere_yet ... ok
test the_corpus_case_admits_two_spellings_and_refuses_the_two_it_names ... ok
test the_scan_finds_every_annotation_the_corpus_holds_and_says_so_in_one_line ... ok
test validate_reports_a_broken_document_with_its_path_and_fails ... ok
test output_survives_a_reader_that_stops_reading ... ok
test an_evidence_document_without_an_observation_time_is_refused_by_name ... ok
test schema_lists_and_prints_generated_schemas ... ok
test evaluate_reports_the_state_and_why_a_transition_is_blocked ... ok
test explain_refuses_a_production_change_and_names_the_rule ... ok
test explain_allows_what_the_profile_grants ... ok
test a_document_whose_every_record_is_future_dated_still_fails ... ok
test evaluate_advances_with_the_examples_evidence ... ok
test json_output_is_machine_readable ... ok
test evaluate_and_inspect_answer_identically_about_one_file ... ok
test resolve_fails_when_the_task_names_a_profile_that_does_not_exist ... ok
test evaluate_reads_every_evidence_file_in_the_example ... ok
test resolve_prints_the_plan ... ok
test validate_accepts_the_repositorys_own_documents ... ok
test one_future_record_refuses_itself_by_position_and_the_document_is_still_evaluated ... ok
test validate_checks_an_artifact_manifest_against_the_lifecycles ... ok
test a_project_is_discovered_so_no_arguments_are_needed ... ok
test inspect_lists_documents_and_shows_one ... ok
test workflow_flow_makes_every_state_a_section ... ok
test conformance_runs_against_the_backend_the_caller_names_and_the_report_says_which ... ok

test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

     Running tests/command_equivalence.rs (target/debug/deps/command_equivalence-fcd648ffc55f9973)

running 11 tests
test the_canonical_command_and_alias_match_on_a_usage_error_inside_an_area ... ok
test the_canonical_command_and_alias_match_on_the_first_level_help ... ok
test the_canonical_command_and_alias_match_on_an_accepted_operation ... ok
test the_canonical_command_and_alias_match_on_a_usage_error_under_a_flat_spelling ... ok
test the_canonical_command_and_alias_match_on_a_usage_error ... ok
test the_canonical_command_and_alias_match_on_a_preflight_that_reports_failures ... ok
test the_canonical_command_and_alias_match_on_a_preflight_rendered_as_json ... ok
test the_canonical_command_and_alias_match_on_an_accepted_operation_at_both_spellings ... ok
test the_canonical_command_and_alias_match_on_a_domain_refusal_at_the_grouped_spelling ... ok
test the_canonical_command_and_alias_match_on_a_domain_refusal ... ok
test the_canonical_command_and_alias_match_on_unrelate_at_both_spellings ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.68s

     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-f5c450cc3b902dce)

running 3 tests
test typed_batch_preserves_crlf_and_unicode_bytes_and_refuses_either_bad_position ... ok
test actual_inspect_aliases_keep_full_u64_and_refuse_malformed_batch_without_partial_output ... ok
test exact_high_time_horizon_restore_and_changed_source_refusals_preserve_execution ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/describe_type.rs (target/debug/deps/describe_type-45cace407fd8d0b7)

running 2 tests
test every_backend_reports_the_same_ladder_for_every_planning_kind ... ok
test the_descriptor_edges_are_what_protocol_artifact_lifecycle_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s

     Running tests/doctor_cli.rs (target/debug/deps/doctor_cli-b258579ea7a3fd72)

running 11 tests
test a_root_that_is_not_a_git_checkout_warns_rather_than_failing ... ok
test the_binary_version_line_agrees_with_what_the_version_flag_prints ... ok
test a_pinned_protocol_source_warns_until_its_snapshot_is_cached_and_never_fetches_it ... ok
test two_runs_over_one_tree_print_identical_bytes ... ok
test a_protocol_source_path_that_is_not_there_fails_naming_where_it_resolved_to ... ok
test the_json_rendering_carries_the_same_codes_and_verdicts_as_the_text_one ... ok
test the_environment_supplies_a_plugin_directory_only_when_the_command_line_named_none ... ok
test a_project_file_that_is_absent_or_unparseable_fails_naming_the_file_and_the_defect ... ok
test a_plugin_directory_without_a_manifest_fails_and_no_directory_at_all_only_warns ... ok
test a_newest_release_tag_that_disagrees_with_the_binary_is_named_and_does_not_fail ... ok
test a_planning_store_that_is_absent_or_invalid_fails_with_the_finding_artifact_validate_reports ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/drift.rs (target/debug/deps/drift-8cbdaed78c3078e9)

running 6 tests
test a_forged_revision_on_a_document_with_no_events_is_still_only_a_document_predating_the_log ... ok
test a_document_removed_with_rm_is_reported_as_deleted ... ok
test a_document_that_matches_its_log_is_not_drift_and_a_plan_before_the_log_is_not_either ... ok
test a_revision_no_write_produced_is_reported_as_forged_and_not_as_an_edit ... ok
test a_document_with_more_events_than_its_revision_is_not_forged ... ok
test a_document_edited_in_an_editor_is_drift_naming_the_field_and_the_event ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

     Running tests/drive_cli.rs (target/debug/deps/drive_cli-2fcb32a1fcda699a)

running 55 tests
test a_holder_cursor_that_will_not_parse_is_a_refusal_and_never_a_crash ... ok
test a_command_step_that_says_protocol_runs_the_build_that_is_driving_it ... ok
test a_pause_is_the_lock_released_and_the_pointer_kept ... ok
test a_lock_naming_another_host_is_refused_at_the_binary_and_take_lock_does_not_pass_it ... ok
test a_run_advances_on_command_step_evidence_and_ends_with_the_engine_speaking ... ok
test a_relation_the_workspace_declares_does_not_stop_a_run_before_it_starts ... ok
test a_refusal_names_what_the_holding_run_is_doing ... ok
test a_run_and_a_resume_that_stole_nothing_write_no_took_lock_from_key ... ok
test a_blocked_run_prints_the_engine_reasons_without_rewording_them ... ok
test a_resume_that_stole_nothing_leaves_a_theft_already_in_the_record ... ok
test a_command_step_naming_another_program_is_resolved_as_written ... ok
test a_headless_start_refuses_what_only_a_person_can_answer_and_the_flag_is_the_route_through ... ok
test a_resume_that_supersedes_a_lock_records_that_theft_over_an_earlier_one ... ok
test a_second_driver_is_refused_by_name_and_writes_nothing ... ok
test a_holder_whose_cursor_cannot_be_read_is_refused_with_state_unknown ... ok
test a_map_that_is_both_uncoverable_and_unspawnable_reports_the_defect_that_travels ... ok
test a_resume_gets_the_iterations_it_was_given_rather_than_what_the_run_has_left ... ok
test a_run_that_took_nobodys_lock_writes_no_took_lock_from_key ... ok
test a_resume_against_another_live_runs_lock_is_refused_and_writes_nothing ... ok
test a_resume_expands_the_task_document_the_run_was_started_from ... ok
test a_run_that_supersedes_a_lock_and_stops_without_a_step_still_records_the_theft ... ok
test a_stolen_lock_is_in_the_taking_runs_cursor_and_status_prints_it ... ok
test the_hook_leaves_a_whole_file_write_to_the_declared_scope ... ok
test the_hook_reads_both_arms_spellings_of_the_same_edit ... ok
test the_hook_cannot_answer_an_unreadable_document ... ok
test transition_answers_only_the_transition_point ... ok
test transition_cannot_answer_an_unreadable_document ... ok
test adversary_a_run_that_failed_after_taking_the_lock_does_not_leave_it_behind ... ok
test a_command_step_binds_the_specification_verb_to_the_task_the_run_was_started_from ... ok
test the_hook_refuses_an_edit_that_crosses_a_planning_documents_fence ... ok
test transition_leave_of_a_failed_section_proceeds ... ok
test every_verb_can_be_asked_for_help ... ok
test adversary_a_refused_second_driver_leaves_the_tree_byte_for_byte_as_it_found_it ... ok
test a_map_that_cannot_produce_demanded_evidence_is_refused_before_the_first_step ... ok
test transition_with_an_unknown_run_cannot_answer ... ok
test adversary_the_refusal_names_the_host_beside_the_pid_rather_than_anywhere_in_the_output ... ok
test the_cargo_map_starts_a_feature_run_without_the_evidence_gap_flag ... ok
test the_checks_map_plans_against_the_repositorys_own_task ... ok
test the_committed_step_map_loads_and_is_refused_when_a_state_is_renamed ... ok
test a_run_stopped_by_its_iteration_bound_resumes_where_it_stopped ... ok
test transition_leave_is_refused_by_the_engine_when_the_rung_is_not_earned ... ok
test transition_enter_without_a_run_proceeds_on_a_state_the_workflow_declares ... ok
test a_second_project_aimed_at_one_store_holds_its_own_lock_and_is_not_refused ... ok
test transition_refuses_a_path_that_names_no_state ... ok
test adversary_a_second_run_of_one_task_allocates_a_new_directory_and_leaves_the_firsts_record_alone ... ok
test two_maps_fit_the_workflow_so_the_driver_refuses_to_choose_and_names_both ... ok
test status_reports_the_run_and_whether_the_lock_is_free ... ok
test a_run_of_one_task_is_not_numbered_by_the_current_pointer_of_another ... ok
test transition_proceeds_at_the_root_which_is_a_container_and_not_a_state ... ok
test transition_reads_a_retreat_group_as_its_first_and_last_state ... ok
test adversary_a_run_paused_for_a_person_released_the_lock_and_resumes ... ok
test the_resume_line_the_driver_prints_works_with_nothing_else_on_it ... ok
test transition_leave_root_with_a_run_is_the_engines_answer ... ok
test adversary_a_run_directory_that_was_removed_is_not_handed_out_to_a_second_run ... ok
test transition_with_a_run_answers_from_the_runs_cursor ... ok

test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s

     Running tests/entity_cli.rs (target/debug/deps/entity_cli-a899952fbd56e50c)

running 11 tests
test entity_get_by_an_unknown_locator_refuses_rather_than_printing_nothing ... ok
test entity_list_narrows_to_one_type ... ok
test entity_history_shows_the_seeding_and_nothing_else ... ok
test audit_lists_the_commands_that_seeded_the_manifest ... ok
test json_output_is_machine_readable ... ok
test describe_says_a_design_accepts_an_approval ... ok
test audit_rejected_is_empty_when_nothing_was_refused ... ok
test entity_relations_incoming_answers_what_points_at_this ... ok
test entity_relations_shows_what_the_design_designs ... ok
test entity_list_shows_one_line_per_artifact_in_the_manifest ... ok
test entity_get_by_locator_prints_the_design_the_manifest_declares ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/ess_conformance_v2.rs (target/debug/deps/ess_conformance_v2-3a5f7406b0c48487)

running 14 tests
test horizon_boundary_and_independent_constraints_survive_typed_task_readback ... ok
test envelope_forgery_alias_duplicates_and_rounded_times_cannot_admit ... ok
test qualifier_checks_independent_subject_model_suite_selection_producer_and_time_reasons ... ok
test raw_wrong_kind_empty_and_terminal_records_keep_distinct_qualification_reasons ... ok
test real_driver_ingests_original_pair_and_re_admits_it_on_status_and_resume ... ok
test current_reader_rechecks_cached_sources_on_submission_direct_record_and_restore ... ok
test refused_submission_and_restore_leave_nonempty_execution_unchanged ... ok
test actual_policy_composes_and_valid_same_record_reaches_unknown_coverage ... ok
test cross_record_fact_overwrites_and_event_claims_never_repair_qualification ... ok
test core_without_reader_and_direct_record_without_time_refuse_even_cached_admission ... ok
test cli_inspect_evaluate_aliases_and_pair_flag_refusals_use_the_actual_reader ... ok
test unrepresentable_planning_dates_refuse_before_opening_or_mutating_the_store ... ok
test typed_json_yaml_full_u64_and_raw_snapshot_readback_re_admit_original_bytes ... ok
test real_planning_pair_preserves_full_u64_diagnostics_and_refusals_do_not_open_store ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s

     Running tests/eval_corpus.rs (target/debug/deps/eval_corpus-c15c2cc327645fc4)

running 5 tests
test a_case_may_declare_what_it_is_about_and_a_typo_inside_it_is_refused ... ok
test the_two_development_cases_are_judged_by_one_document ... ok
test every_case_is_three_files_and_says_what_it_is_about ... ok
test every_case_gates_on_something ... ok
test every_case_replays_to_the_verdict_it_declares ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/eval_dry_run.rs (target/debug/deps/eval_dry_run-3f6c474001aa001d)

running 4 tests
test the_manifests_the_runner_wrote_are_the_documents_the_matrix_refuses_to_guess_at ... ok
test the_digest_every_manifest_carries_comes_from_the_instruments_row ... ok
test the_dry_run_reaches_both_harnesses_all_three_arms_and_a_contradiction ... ok
test the_whole_pipeline_runs_on_committed_streams_and_assembles_the_matrix_byte_for_byte ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/eval_matrix.rs (target/debug/deps/eval_matrix-9348850020732f93)

running 12 tests
test a_record_with_no_manifest_beside_it_is_refused_rather_than_dropped ... ok
test a_manifest_with_no_record_beside_it_is_refused_rather_than_skipped ... ok
test a_directory_with_no_runs_in_it_is_refused_rather_than_rendered_as_a_clean_sheet ... ok
test a_plugin_digest_on_arm_raw_is_refused_where_the_manifest_enters ... ok
test an_arm_this_evaluation_does_not_have_is_refused_by_name ... ok
test an_incomplete_manifest_is_refused_by_name_and_no_matrix_is_written ... ok
test a_manifest_whose_transcript_is_not_the_records_transcript_is_refused ... ok
test a_row_the_record_does_not_mention_at_all_is_the_same_answer_as_a_null_one ... ok
test the_matrix_reports_every_arm_of_every_harness_and_all_three_answers ... ok
test the_matrix_is_a_report_and_not_a_gate ... ok
test a_null_verdict_is_counted_unobservable_and_never_held ... ok
test the_committed_pairs_assemble_into_the_matrix_byte_for_byte ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/eval_run.rs (target/debug/deps/eval_run-bef21d60278c6a22)

running 41 tests
test a_spawn_with_no_cap_on_what_it_may_spend_is_refused_by_name ... ok
test a_declared_plugin_the_attestation_does_not_list_is_refused_rather_than_written_down ... ok
test a_marketplace_plugin_is_refused_by_name_on_a_harness_that_has_none ... ok
test a_model_is_refused_by_name_on_the_harnesses_whose_adapters_take_none ... ok
test a_session_that_omits_the_model_key_altogether_is_still_refused ... ok
test a_session_that_states_no_harness_version_is_refused_and_no_manifest_is_written ... ok
test a_conformant_replay_still_exits_zero ... ok
test a_preflight_with_two_faults_names_both_rather_than_the_first ... ok
test a_spawn_without_the_live_flag_is_refused_by_name_and_nothing_is_started ... ok
test a_run_that_pins_no_model_writes_no_model_requested_key ... ok
test a_session_whose_hermetic_row_is_missing_is_refused_by_the_field_that_is_missing ... ok
test a_stale_aep_on_the_childs_path_is_refused_before_anything_is_spent ... ok
test a_cost_the_wire_writes_as_null_leaves_the_manifest_silent_and_never_says_zero ... ok
test a_stream_of_another_harness_than_the_run_claims_is_refused ... ok
test a_contradicted_replay_exits_with_the_code_its_own_verdict_line_prints ... ok
test a_stream_that_stops_before_the_session_ends_is_refused_rather_than_reported_as_a_whole_run ... ok
test arm_driven_is_not_launched_here_and_the_refusal_names_the_verb_that_does ... ok
test naming_neither_a_case_nor_a_workflow_is_refused_rather_than_running_the_whole_corpus ... ok
test a_stated_cost_this_reader_cannot_convert_stops_the_run_instead_of_becoming_an_estimate ... ok
test one_recorded_stream_is_one_run_and_naming_two_cases_is_refused ... ok
test arm_raw_over_a_stream_that_attests_a_plugin_is_refused_by_name ... ok
test without_the_binary_the_runner_refuses_by_name_and_exits_two ... ok
test arm_raw_with_a_marketplace_plugin_is_refused_before_the_run_and_not_after_it ... ok
test an_undecided_replay_exits_three_rather_than_reporting_a_replayed_transcript ... ok
test a_run_whose_stream_states_a_cost_is_charged_that_cost_and_never_the_assumption ... ok
test a_run_of_arm_driven_is_read_even_though_it_is_not_launched_here ... ok
test an_attested_plugin_with_no_digest_is_refused_because_the_manifest_cannot_say_which_bytes ... ok
test an_unpinned_plugin_is_refused_before_anything_is_spawned_in_metaharness_own_words ... ok
test a_wire_that_names_no_model_assembles_a_manifest_that_says_so ... ok
test arm_plugin_over_a_stream_that_attests_no_plugin_is_refused_by_name ... ok
test arm_plugin_may_be_a_marketplace_plugin_alone_and_the_manifest_says_so ... ok
test a_spawn_forwards_every_pinned_plugin_verbatim_and_the_manifest_keeps_them_apart ... ok
test without_redact_the_operators_own_stream_is_left_exactly_as_it_was ... ok
test the_plugin_digest_in_the_manifest_is_the_one_the_session_attested_byte_for_byte ... ok
test redact_takes_the_operators_home_and_name_out_of_the_stream_it_writes ... ok
test the_cap_stops_the_sweep_before_the_run_that_would_pass_it ... ok
test the_assumed_rate_is_charged_only_where_the_stream_priced_nothing ... ok
test the_digest_is_read_from_the_instruments_row_and_not_from_the_vendors_echo ... ok
test the_manifests_digest_is_over_the_redacted_bytes_so_the_written_stream_replays ... ok
test the_manifest_records_what_was_asked_for_beside_what_the_attestation_reported ... ok
test a_spawn_gives_arm_raw_the_committed_instructions_and_arm_plugin_the_plugin ... ok

test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/evidence_producers.rs (target/debug/deps/evidence_producers-5c161eadfcd510dc)

running 7 tests
test protocol_property_evidence_writes_a_property_record_with_the_case_count_it_measured ... ok
test protocol_specification_evidence_writes_the_requirement_by_requirement_verdict ... ok
test a_tree_that_does_not_validate_still_produces_a_record_and_it_says_failed ... ok
test a_store_holding_two_specifications_of_this_tasks_work_is_refused_rather_than_guessed_at ... ok
test every_evidence_producer_writes_the_same_record_through_either_binary_name ... ok
test protocol_validate_writes_a_verification_record_the_driver_can_submit ... ok
test no_producer_writes_a_record_a_person_is_recorded_as_having_produced ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/golden_plan.rs (target/debug/deps/golden_plan-12c239af7e73a45b)

running 2 tests
test the_write_verbs_leave_exactly_the_documents_recorded ... ok
test the_read_verbs_print_exactly_what_was_recorded ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s

     Running tests/grouped_and_flat_spellings.rs (target/debug/deps/grouped_and_flat_spellings-277bc599c1fb9585)

running 4 tests
test the_help_a_reader_sees_offers_the_four_areas_and_doctor_and_no_flat_spelling ... ok
test every_case_reaches_its_verb_rather_than_a_usage_error ... ok
test the_grouped_path_and_the_flat_spelling_answer_identically_under_aep ... ok
test the_grouped_path_and_the_flat_spelling_answer_identically_under_protocol ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 57.20s

     Running tests/guide_console_blocks_are_what_the_command_prints.rs (target/debug/deps/guide_console_blocks_are_what_the_command_prints-7dd000399c5a7687)

running 1 test
test every_quoted_trace_report_on_the_transcript_guide_is_what_the_command_prints ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/instructions.rs (target/debug/deps/instructions-08d5ee5b0807236b)

running 3 tests
test a_workflow_that_moved_renders_different_bytes_and_leaves_its_neighbours_alone ... ok
test the_committed_instruction_documents_are_what_the_verb_writes ... ok
test two_runs_of_the_verb_write_the_same_bytes ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/metaharness_contract_result.rs (target/debug/deps/metaharness_contract_result-5a5089fb5f207d4c)

running 6 tests
test a_record_whose_breaking_changes_exceed_its_failures_is_refused_before_a_document_exists ... ok
test the_observation_time_is_required_because_this_process_did_not_watch_the_run ... ok
test a_record_that_checked_nothing_is_refused_before_a_document_exists ... ok
test the_record_can_arrive_on_a_pipe_and_the_loop_still_closes ... ok
test a_breaking_change_is_the_number_the_evaluation_turns_on ... ok
test every_captured_adapter_record_becomes_evidence_the_engine_reads ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/metaharness_frame_contract.rs (target/debug/deps/metaharness_frame_contract-462f88106e259f2b)

running 11 tests
test bytes_that_are_not_a_json_object_are_refused_before_the_tag_is_looked_for ... ok
test the_minted_golden_holds_exactly_the_fields_the_frame_struct_has ... ok
test a_frame_document_without_a_tag_it_knows_is_refused_as_untagged ... ok
test the_minted_golden_carries_the_exact_tag_the_consumer_looks_for ... ok
test a_single_flipped_byte_in_a_minted_frame_breaks_the_digest ... ok
test the_minted_golden_lists_its_operations_in_wire_name_order ... ok
test a_frame_document_that_was_never_sealed_is_refused_by_the_same_check ... ok
test a_frame_document_that_is_not_a_frames_shape_is_refused_as_misshapen ... ok
test the_transcribed_reader_accepts_a_frame_the_golden_does_not_cover ... ok
test the_minted_golden_is_accepted_by_the_rules_that_would_refuse_it ... ok
test a_scoped_frame_survives_projection_and_its_scope_participates_in_the_digest ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/planning_cli.rs (target/debug/deps/planning_cli-92c5e4be3daeab29)

running 83 tests
test a_malformed_ladder_document_does_not_silently_empty_the_board ... ok
test a_report_of_no_scenarios_is_refused_because_it_asserts_nothing ... ok
test a_listing_says_no_relations_with_an_empty_list_rather_than_by_omission ... ok
test a_conformance_report_is_read_into_the_record_rather_than_typed_at_it ... ok
test a_body_that_is_empty_after_trimming_is_refused_naming_the_flag ... ok
test a_pinned_git_protocol_source_is_materialized_once_and_then_read_from_cache ... ok
test a_lifecycle_is_printed_from_the_documents_the_tree_declares ... ok
test a_column_takes_its_description_from_the_ladder_that_governs_it ... ok
test a_new_story_is_written_where_its_id_says_and_validates_clean ... ok
test a_body_handed_to_new_on_standard_input_is_the_body_the_store_holds ... ok
test a_store_that_cannot_be_read_whole_is_never_written_to ... ok
test a_body_replacement_preserves_machine_owned_frontmatter ... ok
test a_malformed_findings_block_is_refused_at_new_with_the_line_it_is_wrong_on ... ok
test a_walk_refused_at_its_last_rung_still_reports_the_hop_it_made ... ok
test a_reference_given_at_new_is_written_to_the_document ... ok
test a_legal_move_rewrites_the_document_and_bumps_the_revision ... ok
test a_refused_evidence_kind_names_the_nearest_two_that_exist ... ok
test a_walk_crosses_unguarded_rungs_and_stops_at_a_guarded_one ... ok
test a_rung_on_no_cycle_is_not_printed_before_the_rung_that_leads_to_it ... ok
test a_reference_reaches_the_file_and_a_query_finds_it ... ok
test a_title_and_a_summary_may_begin_with_a_dash ... ok
test an_edge_to_an_artifact_the_store_does_not_hold_is_refused ... ok
test a_review_results_findings_block_is_parsed_at_new_and_returned_by_show_as_an_array ... ok
test an_edge_written_as_one_word_is_the_edge_written_as_three ... ok
test no_planning_verb_writes_to_the_store_except_through_a_command ... ok
test a_move_that_would_leave_the_store_invalid_is_refused_with_the_finding ... ok
test every_verb_can_be_built_and_asked_for_help ... ok
test a_section_and_an_append_are_body_verbs_rather_than_a_heredoc ... ok
test outside_a_project_the_missing_store_says_what_to_pass ... ok
test reading_a_record_and_typing_one_are_not_combined ... ok
test a_new_epic_story_and_specification_are_seeded_with_a_classified_ambiguities_section ... ok
test planning_documents_follow_the_protocol_tree_named_by_the_project ... ok
test review_value_falls_back_to_a_reviewer_key_when_the_review_has_no_owner ... ok
test review_value_says_in_eval_matrixs_own_words_why_it_computes_no_score ... ok
test an_illegal_move_exits_one_and_names_every_legal_target ... ok
test a_ladders_column_order_survives_a_second_kind_that_shares_its_rung_names ... ok
test kinds_lists_the_ladders_a_store_declares_and_the_open_blocker_family ... ok
test an_edge_is_taken_back_by_the_words_that_made_it_and_leaves_the_others_alone ... ok
test show_body_only_prints_the_bytes_body_from_would_write_back ... ok
test listing_narrows_by_kind_and_by_status ... ok
test blocked_says_when_no_ladder_declares_a_blocker_at_all ... ok
test listing_the_fixture_as_json_is_byte_identical_across_two_runs ... ok
test creating_the_same_artifact_twice_is_refused_rather_than_overwriting_it ... ok
test the_command_only_scan_sees_a_write_it_should_refuse ... ok
test the_entity_surface_counts_the_fixtures_artifacts ... ok
test the_entity_surface_refuses_both_sources_and_neither ... ok
test the_board_groups_the_fixture_into_status_columns ... ok
test the_evidence_help_names_the_review_outcome_kind_and_what_it_needs ... ok
test every_artifact_the_board_lists_lands_in_a_column ... ok
test only_the_named_kind_goes_when_two_edges_point_at_one_artifact ... ok
test a_model_digest_is_written_on_a_specification_and_refused_by_name_on_a_story ... ok
test the_board_renders_as_a_markdown_page_a_site_can_publish_unedited ... ok
test the_graph_draws_every_artifact_and_every_edge ... ok
test the_graph_renders_as_mermaid_with_ids_a_diagram_can_carry ... ok
test replacing_a_body_with_identical_bytes_does_not_invent_a_revision ... ok
test the_fixture_store_validates_clean ... ok
test the_vocabulary_verbs_answer_without_a_store ... ok
test a_review_outcome_naming_a_review_of_something_else_is_refused ... ok
test the_store_defaults_to_the_planning_directory_of_the_project_it_is_run_in ... ok
test validate_answers_the_same_from_a_subdirectory_as_it_does_from_the_root ... ok
test validate_lists_every_problem_in_a_broken_store ... ok
test a_review_outcome_is_recorded_against_the_reviewed_artifact_and_printed_on_the_review ... ok
test unrelating_an_edge_that_is_not_declared_is_refused_naming_the_ones_that_are ... ok
test review_value_since_a_date_leaves_out_what_was_recorded_before_it ... ok
test a_review_result_is_authored_whole_retired_by_its_ladder_and_edited_never ... ok
test the_new_kinds_have_the_ladder_the_store_needs ... ok
test set_changes_a_frontmatter_field_and_refuses_the_four_it_does_not_own ... ok
test validate_holds_agreed_work_to_an_objective_once_the_store_declares_one ... ok
test the_findings_verb_classifies_a_finding_that_moved_two_lines_as_carried ... ok
test explain_ends_with_what_each_legal_next_rung_costs ... ok
test validate_reports_a_review_result_with_no_findings_block_without_failing ... ok
test the_compiled_order_still_separates_two_known_rungs_when_a_kind_declares_no_ladder ... ok
test the_findings_verb_refuses_a_review_that_does_not_review_the_artifact ... ok
test the_findings_verb_takes_the_two_reviews_it_is_told_and_exits_zero_with_one_review ... ok
test the_board_has_a_column_for_a_rung_only_a_lifecycle_document_names ... ok
test strict_validate_fails_on_what_plain_validate_only_reports ... ok
test review_value_counts_per_reviewer_and_says_unknown_where_no_manifest_named_a_cost ... ok
test withheld_evidence_that_blocks_nothing_is_reported_by_validate ... ok
test the_outcome_flags_and_the_review_outcome_kind_require_each_other ... ok
test a_blocker_is_typed_by_what_clears_it_and_says_so_in_every_listing ... ok
test validate_reports_a_review_with_no_outcome_without_failing_and_the_age_is_a_flag ... ok
test one_ladders_column_order_does_not_depend_on_another_kind_being_in_the_store ... ok
test the_board_prints_a_terminal_rung_after_the_rungs_that_lead_to_it ... ok

test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.59s

     Running tests/render_cli.rs (target/debug/deps/render_cli-ab27e47e98523592)

running 13 tests
test a_workflow_the_tree_does_not_declare_is_refused_by_name ... ok
test a_workflow_renders_to_a_standalone_svg_document_on_standard_output ... ok
test a_run_id_with_no_directory_behind_it_is_refused_by_path ... ok
test watch_without_a_run_is_refused_because_there_would_be_nothing_to_follow ... ok
test the_html_page_is_written_whole_and_fetches_nothing ... ok
test watch_is_refused_on_a_format_that_writes_a_document_once ... ok
test png_without_an_output_file_is_refused_and_names_the_flag_that_fixes_it ... ok
test a_frame_written_to_a_file_carries_no_control_characters ... ok
test png_without_the_rasteriser_names_the_program_and_what_to_install ... ok
test a_snapshot_on_its_own_draws_the_path_and_refuses_to_guess_a_status ... ok
test a_run_directory_paints_the_overlay_and_prints_its_reasons_verbatim ... ok
test the_same_workflow_renders_to_the_same_bytes_twice ... ok
test every_committed_workflow_renders ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/reverse_cli.rs (target/debug/deps/reverse_cli-1c8c4e3860c7cb95)

running 19 tests
test an_unpinned_git_source_is_refused_and_nothing_is_written ... ok
test a_document_that_is_not_openapi_is_refused ... ok
test a_draft_names_every_decision_it_could_not_take ... ok
test an_unsupported_source_scheme_is_refused ... ok
test an_absolute_protocol_source_is_refused_and_the_repository_is_left_as_it_was ... ok
test a_test_that_never_runs_is_told_apart_from_one_that_is_opted_into ... ok
test a_relative_source_resolves_through_a_directory_that_did_not_exist_yet ... ok
test build_output_and_machine_state_are_not_part_of_a_repository_plan ... ok
test every_citation_a_scan_emits_resolves_to_a_real_line ... ok
test a_scan_reads_what_the_fixture_says_and_not_what_it_does_not ... ok
test a_language_call_named_todo_is_not_unfinished_work ... ok
test a_directory_with_no_history_says_so_in_one_sentence ... ok
test every_verb_can_be_built_and_asked_for_help ... ok
test one_tree_scans_to_the_same_bytes_twice ... ok
test a_marked_line_is_dated_from_the_commit_that_wrote_it_and_not_from_today ... ok
test a_history_reports_what_the_commits_say ... ok
test a_history_of_one_tree_is_the_same_bytes_twice ... ok
test init_writes_a_project_the_next_command_can_read ... ok
test a_vision_cannot_be_implemented_and_a_story_can ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

     Running tests/reverse_openapi_relations.rs (target/debug/deps/reverse_openapi_relations-15940afda9cb2dde)

running 2 tests
test a_schema_with_no_relation_signal_carries_no_relations_block ... ok
test the_drafted_domain_is_the_recorded_bytes ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/serve_cli.rs (target/debug/deps/serve_cli-e5781878f91e1bfd)

running 6 tests
test response_framing_names_exactly_how_many_body_bytes_must_arrive ... ok
test a_read_only_server_answers_reads_and_refuses_every_move ... ok
test the_board_is_answered_over_a_socket_and_only_to_the_token_it_printed ... ok
test an_illegal_move_is_a_conflict_carrying_every_legal_target ... ok
test explain_answers_the_rungs_a_page_would_draw_and_what_each_costs ... ok
test a_legal_move_is_written_and_the_next_read_sees_it ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s

     Running tests/specification_task_binding.rs (target/debug/deps/specification_task_binding-60388edeeea39350)

running 3 tests
test a_task_no_specification_in_the_store_is_about_is_refused_and_nothing_is_written ... ok
test an_artifact_named_on_the_command_line_does_not_lift_the_binding ... ok
test one_store_decides_two_tasks_differently_and_each_record_names_its_own_specification ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/sqlite_plan.rs (target/debug/deps/sqlite_plan-9bfe1322b495faae)

running 1 test
test this_repositorys_plan_round_trips_through_sqlite_and_answers_the_markdown_backends_history ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.07s

     Running tests/store_selection.rs (target/debug/deps/store_selection-19ebf97f162aa0d9)

running 12 tests
test a_hybrid_missing_a_policy_word_is_refused_naming_the_word ... ok
test the_sqlite_plan_is_read_from_the_database_and_not_from_files ... ok
test conformance_against_the_project_holds_the_configured_kind_of_store_to_the_suites ... ok
test explaining_an_artifact_no_store_holds_is_refused_naming_it ... ok
test a_hybrid_records_a_write_its_replica_refused_and_the_next_process_catches_it_up ... ok
test evidence_without_at_is_recorded_at_the_instant_the_edge_read ... ok
test an_edge_is_taken_back_alike_in_every_store ... ok
test show_prints_one_artifact_with_its_body_verbatim_in_every_store ... ok
test a_joined_record_outlives_the_file_its_reference_names ... ok
test what_made_a_story_done_names_the_revision_each_record_was_admitted_at ... ok
test a_status_reached_without_a_record_says_which_kind_of_claim_it_rested_on ... ok
test every_verb_answers_alike_over_markdown_and_sqlite ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.23s

     Running tests/trace_cli.rs (target/debug/deps/trace_cli-6161c6d97f03b23f)

running 7 tests
test a_file_that_is_neither_wire_is_refused_with_the_format_it_was_read_as ... ok
test a_downgrade_the_specification_does_not_declare_is_refused_by_the_evidence_verb_too ... ok
test a_driven_event_stream_is_checked_with_the_same_arguments_as_a_recorded_transcript ... ok
test a_run_that_gapped_is_written_down_rather_than_exited_on ... ok
test the_provenance_command_is_the_canonical_spelling_whichever_binary_minted_the_record ... ok
test the_json_rendering_is_read_by_the_same_loader_as_the_yaml_one ... ok
test the_record_the_checker_writes_is_one_the_engine_accepts ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/wave_derivation.rs (target/debug/deps/wave_derivation-f1ddadabd43665e8)

running 13 tests
test a_depends_on_cycle_prints_its_ids_and_exits_two ... ok
test the_fixture_store_prints_the_recorded_answer_byte_for_byte ... ok
test scope_is_refused_on_a_kind_that_is_not_a_story ... ok
test scope_add_writes_a_typed_entry_and_bumps_the_revision_once ... ok
test show_prints_the_scope_and_json_carries_it_as_an_array ... ok
test a_story_with_no_scope_is_unassessed_and_never_placed ... ok
test scope_records_an_inferred_entry_apart_from_a_cited_one_and_remove_takes_it_out ... ok
test waves_leaves_every_byte_of_the_store_where_it_was ... ok
test an_inferred_entry_collides_and_is_marked_as_inferred ... ok
test waves_never_point_a_dependency_the_wrong_way ... ok
test waves_answers_about_the_status_it_was_asked_about ... ok
test waves_places_disjoint_stories_together_and_pushes_the_pair_that_collides ... ok
test validate_reports_a_non_draft_story_with_no_scope_and_still_exits_zero ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s

     Running unittests src/lib.rs (target/debug/deps/aep_domain-3949933cc8b2ee1a)

running 289 tests
test action::tests::every_action_maps_to_one_capability ... ok
test action::tests::network_intent_selects_the_capability ... ok
test artifact::tests::a_confidence_the_vocabulary_does_not_have_is_refused_by_name ... ok
test artifact::tests::a_custom_kinds_parent_is_the_kind_its_last_segment_names ... ok
test artifact::tests::a_blockers_type_is_the_part_of_its_kind_before_blocker ... ok
test artifact::tests::a_misspelled_member_is_dangling_even_inside_a_workspace ... ok
test artifact::tests::a_member_qualified_target_in_a_store_with_no_workspace_is_still_dangling ... ok
test artifact::tests::a_relation_naming_another_member_is_external_rather_than_dangling ... ok
test artifact::tests::a_model_digest_is_refused_on_a_kind_that_has_no_model ... ok
test artifact::tests::a_scope_entry_reads_the_mapping_and_the_bare_path_as_one_thing ... ok
test artifact::tests::a_scope_entry_refuses_an_empty_path_and_accepts_any_granularity ... ok
test artifact::tests::a_single_segment_custom_kind_is_the_top_of_its_own_family ... ok
test artifact::tests::an_unqualified_dangling_edge_is_still_refused ... ok
test artifact::tests::design_subkinds_satisfy_a_design_requirement ... ok
test artifact::tests::lineage_never_reads_an_alias_as_a_parent ... ok
test artifact::tests::lifecycle_rejects_a_status_the_kind_does_not_have ... ok
test artifact::tests::evidence_withheld_from_nothing_is_refused_by_name ... ok
test artifact::tests::one_lifecycle_on_a_custom_parent_kind_governs_the_whole_family ... ok
test artifact::tests::parses_kind_aliases_and_unknown_kinds ... ok
test artifact::tests::only_a_revision_bound_model_is_governing ... ok
test artifact::tests::parses_locations_in_every_documented_form ... ok
test artifact::tests::rejects_dangling_edges_and_cycles ... ok
test artifact::tests::the_end_of_a_ladder_is_the_rung_with_nowhere_to_go ... ok
test artifact::tests::superseded_artifacts_must_name_a_successor ... ok
test artifact::tests::the_fallback_is_consulted_last_and_only_when_nothing_nearer_answers ... ok
test artifact::tests::projects_facts_including_the_kind_hierarchy ... ok
test audit::tests::a_change_without_a_before_revision_is_a_creation ... ok
test audit::tests::a_change_that_names_neither_revision_is_rejected ... ok
test audit::tests::a_change_without_an_after_revision_is_a_removal ... ok
test audit::tests::a_protocol_decision_without_a_decision_is_rejected ... ok
test audit::tests::a_change_record_round_trips_with_both_revisions ... ok
test audit::tests::a_record_whose_decision_refused_the_action_may_not_carry_a_change ... ok
test audit::tests::a_redacted_change_that_kept_its_payload_is_rejected ... ok
test artifact::tests::walks_relations_forwards_and_backwards ... ok
test audit::tests::a_refusal_whose_decision_says_allowed_is_rejected ... ok
test audit::tests::a_refused_command_records_why_and_changes_nothing ... ok
test audit::tests::a_redacted_payload_keeps_the_actor_correlation_and_reason ... ok
test audit::tests::a_refused_command_that_carries_a_change_is_rejected ... ok
test audit::tests::an_entity_change_record_without_a_change_is_rejected ... ok
test audit::tests::correlation_is_mandatory_and_causation_is_optional ... ok
test audit::tests::causation_serialises_with_its_kind_tag ... ok
test audit::tests::every_audit_kind_round_trips_through_its_snake_case_name ... ok
test audit::tests::validation_reports_every_problem_at_once ... ok
test capability::tests::a_denied_capability_is_not_downgraded_to_requiring_an_approval ... ok
test capability::tests::a_denied_private_read_is_not_downgraded_to_requiring_an_approval ... ok
test capability::tests::a_denied_private_read_is_not_reopened_by_a_broad_network_grant ... ok
test audit::tests::the_actor_and_the_executor_stay_distinct ... ok
test capability::tests::a_read_that_will_not_say_its_audience_is_not_covered_by_a_public_grant ... ok
test capability::tests::a_wildcard_denial_is_not_reopened_by_an_approval_on_one_environment ... ok
test capability::tests::an_environment_wildcard_covers_named_environments ... ok
test capability::tests::deny_beats_approval_which_beats_allow ... ok
test capability::tests::identifies_production_mutations ... ok
test capability::tests::parses_and_renders_audience_scoped_network_reads ... ok
test capability::tests::parses_and_renders_capability_strings ... ok
test capability::tests::rejects_unknown_capabilities_and_misplaced_environments ... ok
test capability::tests::restrict_cannot_grant_but_grant_can ... ok
test capability::tests::the_unknown_capability_diagnostic_names_the_scoped_capabilities_too ... ok
test capability::tests::unmentioned_capabilities_are_not_granted ... ok
test command::tests::a_payload_field_the_protocol_does_not_define_is_rejected ... ok
test command::tests::a_relation_command_targets_the_entity_whose_edges_change ... ok
test command::tests::a_summary_names_what_the_command_touched ... ok
test command::tests::an_entity_cannot_hold_a_relation_to_itself ... ok
test command::tests::a_well_formed_command_has_nothing_to_report ... ok
test command::tests::an_entity_cannot_supersede_itself ... ok
test command::tests::an_unrecognised_command_type_is_rejected_and_the_vocabulary_named ... ok
test command::tests::an_update_that_changes_nothing_is_refused ... ok
test command::tests::approving_a_design_needs_artifact_write_not_approval_request ... ok
test command::tests::approving_a_stale_revision_is_a_different_command_from_approving_the_current_one ... ok
test command::tests::asking_for_a_review_is_the_only_command_that_does_not_write_an_artifact ... ok
test command::tests::creating_an_entity_names_no_target_because_nothing_exists_yet ... ok
test command::tests::every_command_mutates_state ... ok
test command::tests::every_wire_name_round_trips_through_parsing_and_serde ... ok
test command::tests::only_a_command_naming_a_revision_asserts_one ... ok
test command::tests::the_serde_tag_names_the_variant_in_kebab_case ... ok
test command::tests::the_sample_set_covers_every_command_kind ... ok
test command::tests::wire_names_are_the_versioned_names_the_specification_lists ... ok
test domain_event::tests::a_creation_event_pins_the_initial_revision_and_an_update_pins_the_one_it_produced ... ok
test command::tests::every_command_round_trips_through_json_under_its_own_tag ... ok
test domain_event::tests::a_malformed_event_type_is_rejected_with_the_reason_it_failed ... ok
test domain_event::tests::a_relation_event_has_no_subject_because_a_relation_is_not_an_entity ... ok
test domain_event::tests::a_revision_without_a_subject_is_rejected ... ok
test domain_event::tests::a_subject_without_a_revision_is_rejected ... ok
test domain_event::tests::a_custom_event_keeps_its_data ... ok
test domain_event::tests::a_type_matches_its_own_family_across_versions_but_not_another_event ... ok
test domain_event::tests::an_envelope_reports_every_broken_invariant_at_once ... ok
test domain_event::tests::an_envelope_whose_declared_type_contradicts_its_payload_is_rejected ... ok
test domain_event::tests::an_event_that_names_a_command_but_a_different_cause_is_rejected ... ok
test domain_event::tests::an_event_with_no_causing_command_needs_no_causation ... ok
test domain_event::tests::an_organisation_may_emit_an_event_type_this_crate_does_not_name ... ok
test domain_event::tests::correlation_and_causation_survive_a_serde_round_trip ... ok
test entity::tests::a_pinned_reference_stops_being_current_when_the_entity_moves_on ... ok
test domain_event::tests::every_payload_declares_a_type_this_crate_names ... ok
test entity::tests::a_revision_advances_and_records_who_changed_it ... ok
test entity::tests::a_versioned_reference_names_one_revision_and_an_unversioned_one_does_not ... ok
test domain_event::tests::from_command_records_the_command_as_the_direct_cause ... ok
test domain_event::tests::every_named_event_type_round_trips_through_its_wire_name ... ok
test entity::tests::actors_distinguish_who_authorised_from_what_ran ... ok
test entity::tests::an_entity_id_is_opaque_and_long_enough_not_to_be_a_key ... ok
test entity::tests::entity_types_round_trip_and_carry_a_version ... ok
test entity::tests::locators_are_addresses_with_four_segments ... ok
test entity::tests::revisions_start_at_one ... ok
test error::tests::errors_accumulate_and_report_every_problem ... ok
test error::tests::every_code_has_a_distinct_stable_string ... ok
test error::tests::the_serialised_form_matches_the_string_form ... ok
test event::tests::events_serialise_with_a_stable_tag ... ok
test evidence::tests::a_conformance_result_projects_a_fact_a_completion_condition_can_read ... ok
test evidence::tests::a_conformance_run_with_failures_does_not_pass_however_it_reports_its_status ... ok
test evidence::tests::a_deployment_records_whether_a_rollback_target_exists ... ok
test evidence::tests::a_conformance_record_that_cannot_name_its_specification_is_refused ... ok
test evidence::tests::a_property_run_hands_back_the_seed_that_reproduces_its_counterexample ... ok
test evidence::tests::a_property_run_without_a_seed_says_so_rather_than_looking_reproducible ... ok
test evidence::tests::a_review_projects_facts_for_the_subject_kind_hierarchy ... ok
test evidence::tests::a_run_against_yesterdays_revision_does_not_cover_the_specification_in_the_graph ... ok
test evidence::tests::a_test_run_projects_canonical_facts_and_documented_aliases ... ok
test evidence::tests::a_trace_check_projects_the_verdict_the_counts_and_both_digests ... ok
test evidence::tests::a_trace_record_does_not_claim_to_be_about_an_ess_revision ... ok
test evidence::tests::a_trace_check_reporting_a_pass_beside_a_gap_does_not_pass ... ok
test evidence::tests::a_transcript_digest_is_not_interchangeable_with_a_specification_digest ... ok
test domain_event::tests::every_payload_builds_an_envelope_that_validates ... ok
test evidence::tests::an_empty_test_run_is_inconclusive_not_green ... ok
test evidence::tests::conformance_evidence_for_one_specification_does_not_attest_another ... ok
test evidence::tests::an_undecided_run_is_not_a_failed_one_in_the_record ... ok
test evidence::tests::a_verification_record_projects_the_claim_verified_alias ... ok
test evidence::tests::every_evidence_kind_names_at_least_one_verifier ... ok
test evidence::tests::evidence_kind_aliases_parse ... ok
test evidence::tests::an_artifact_evidence_record_can_be_written_in_a_document_at_all ... ok
test evidence::tests::metrics_project_both_the_namespaced_and_bare_path ... ok
test evidence::tests::only_a_conformance_runner_establishes_conformance ... ok
test evidence::tests::only_a_trace_checker_establishes_transcript_conformance ... ok
test evidence::tests::deserializing_an_evidence_kind_accepts_exactly_what_parsing_it_accepts ... ok
test evidence::tests::only_evidence_that_was_produced_against_a_model_carries_a_digest ... ok
test evidence::tests::a_trace_record_round_trips_through_the_document_form_the_engine_reads ... ok
test evidence::tests::trace_conformance_is_spelled_the_way_the_checker_writes_it ... ok
test facts::tests::a_number_too_large_to_represent_stays_the_text_it_was_written_as ... ok
test facts::tests::an_infinity_is_refused_because_it_cannot_be_written_back_out ... ok
test facts::tests::a_document_cannot_deserialise_a_number_the_constructor_would_refuse ... ok
test facts::tests::parses_and_rejects_fact_paths ... ok
test facts::tests::parses_literals_by_shape ... ok
test facts::tests::patterns_match_segment_wise ... ok
test facts::tests::rejects_nan_numbers ... ok
test facts::tests::scales_order_non_numeric_values ... ok
test ids::tests::accepts_well_formed_identifiers ... ok
test ids::tests::rejects_malformed_identifiers ... ok
test ids::tests::rejects_numeric_tail_on_dotted_ids_to_keep_version_refs_unambiguous ... ok
test ids::tests::subject_refs_round_trip ... ok
test node::tests::a_number_stays_a_number_when_serde_json_arbitrary_precision_is_unified ... ok
test predicate::tests::conjunction_follows_the_kleene_table_in_all_nine_rows ... ok
test predicate::tests::disjunction_follows_the_kleene_table_in_all_nine_rows ... ok
test predicate::tests::negating_unknown_leaves_it_unknown ... ok
test predicate::tests::only_true_satisfies_a_transition ... ok
test predicate::tests::kleene_conjunction_keeps_false_ahead_of_unknown ... ok
test predicate::tests::explains_only_the_failing_children_of_a_conjunction ... ok
test predicate::tests::a_predicate_nested_past_the_limit_is_refused_rather_than_overflowing_the_stack ... ok
test predicate::tests::ordering_text_needs_a_declared_scale ... ok
test predicate::tests::parses_the_compact_forms ... ok
test predicate::tests::parses_the_structured_mapping_forms ... ok
test predicate::tests::reads_a_dotted_right_hand_side_as_a_fact_and_a_bare_word_as_text ... ok
test predicate::tests::a_predicate_as_deep_as_anybody_writes_is_still_accepted ... ok
test predicate::tests::rejects_unknown_operators_and_bad_paths ... ok
test predicate::tests::unobserved_facts_are_unknown_not_false ... ok
test predicate::tests::round_trips_through_document_form ... ok
test predicate::tests::the_two_predicate_forms_share_one_depth_budget ... ok
test principle::tests::a_principle_that_enforces_nothing_is_rejected ... ok
test principle::tests::a_timing_selector_without_requirements_is_rejected ... ok
test principle::tests::a_parameter_the_action_does_not_take_is_refused_rather_than_ignored ... ok
test principle::tests::every_invented_parameter_is_named_at_once ... ok
test principle::tests::an_unobservable_applicability_condition_keeps_the_principle_in_force ... ok
test principle::tests::every_action_in_the_register_is_known_to_the_parser_and_published_with_a_shape ... ok
test principle::tests::parses_the_artifact_requires_form_with_a_before_selector ... ok
test principle::tests::parses_failure_policies_in_both_forms ... ok
test principle::tests::colliding_generated_obligation_ids_are_made_unique ... ok
test principle::tests::requirements_with_no_timing_default_to_before_completion ... ok
test principle::tests::parses_the_phase_keyed_requires_form ... ok
test project::tests::a_git_protocol_source_is_a_repository_pinned_to_one_full_commit ... ok
test profile::tests::requires_a_workflow_and_a_completion_condition ... ok
test profile::tests::accepts_a_complete_profile ... ok
test profile::tests::a_profile_can_drop_an_inherited_principle ... ok
test project::tests::a_git_protocol_source_without_an_immutable_revision_is_refused ... ok
test project::tests::a_relative_protocol_source_is_still_accepted ... ok
test project::tests::a_minimal_project_file_names_only_what_it_must ... ok
test project::tests::a_complete_hybrid_carries_its_words_and_both_halves ... ok
test project::tests::an_absolute_protocol_source_is_refused_however_it_is_spelt ... ok
test profile::tests::rejects_a_principle_that_is_both_listed_and_dropped ... ok
test project::tests::an_absolute_path_is_refused ... ok
test project::tests::a_hybrid_missing_a_word_is_refused_naming_the_word ... ok
test profile::tests::extending_adds_principles_and_can_only_tighten_completion ... ok
test project::tests::an_unknown_key_is_refused_rather_than_ignored ... ok
test project::tests::a_sqlite_store_is_a_relative_path_resolved_against_engineering ... ok
test project::tests::an_absolute_schema_registry_is_refused ... ok
test project::tests::paths_resolve_against_the_engineering_directory ... ok
test project::tests::a_pinned_git_source_may_carry_an_absolute_path_inside_its_locator ... ok
test project::tests::a_hybrid_word_the_runtime_does_not_know_is_refused_naming_the_field_and_the_words ... ok
test project::tests::the_protocol_tree_may_live_outside_the_project_without_being_named_absolutely ... ok
test project::tests::a_project_that_names_no_store_keeps_its_plan_in_markdown ... ok
test protocol::tests::a_floor_wider_than_the_grant_is_not_answered_by_denying_a_slice_of_it ... ok
test project::tests::an_unknown_format_version_is_refused_rather_than_guessed ... ok
test protocol::tests::rejects_an_unsupported_major_version ... ok
test protocol::tests::a_broad_read_beside_the_denial_the_floor_asked_for_is_not_refused ... ok
test protocol::tests::accepts_a_self_contained_protocol ... ok
test protocol::tests::a_derived_protocol_may_add_to_the_floor_but_not_escape_it ... ok
test protocol::tests::extension_adds_to_the_base_vocabulary_without_removing_anything ... ok
test requirement::tests::a_horizon_that_cannot_be_checked_refuses_rather_than_passing ... ok
test requirement::tests::a_pinned_edge_and_an_unpinned_declaration_are_the_same_work ... ok
test requirement::tests::a_requirement_without_a_horizon_is_untouched_by_the_clock ... ok
test requirement::tests::a_relation_target_nothing_binds_to_is_refused_by_name ... ok
test protocol::tests::rejects_evidence_no_declared_verifier_can_establish ... ok
test requirement::tests::a_nested_advisory_row_is_owned_by_whoever_declared_it ... ok
test requirement::tests::a_rejected_artifact_is_false_rather_than_merely_unobserved ... ok
test requirement::tests::a_bare_list_is_read_as_predicates ... ok
test protocol::tests::the_spellings_adp_declares_for_transcript_conformance_are_the_ones_the_enum_holds ... ok
test requirement::tests::a_horizon_is_read_from_the_document_in_the_spellings_the_convention_uses ... ok
test requirement::tests::a_specification_bound_to_the_task_refuses_another_storys_approved_one ... ok
test requirement::tests::a_specification_with_no_recorded_digest_is_conformed_to_by_nothing ... ok
test requirement::tests::a_stale_revision_outranks_a_lapsed_observation_because_it_is_the_worse_news ... ok
test requirement::tests::a_bound_requirement_reads_unknown_and_names_what_the_task_is_about ... ok
test requirement::tests::a_task_that_declares_no_work_satisfies_no_bound_requirement ... ok
test requirement::tests::a_stale_approval_does_not_satisfy_a_fresh_review_requirement ... ok
test requirement::tests::an_advisory_requirement_is_checked_and_reported_and_does_not_block ... ok
test requirement::tests::an_observation_past_its_horizon_reads_unknown_and_names_the_horizon_and_the_date ... ok
test requirement::tests::an_observation_exactly_its_horizon_old_still_satisfies_the_requirement ... ok
test requirement::tests::an_unbound_relation_serialises_exactly_as_it_did ... ok
test requirement::tests::an_artifact_requirement_reads_the_kind_hierarchy_and_status_ladder ... ok
test requirement::tests::an_unrecognised_mapping_key_becomes_a_fact_predicate ... ok
test requirement::tests::conditional_requirements_apply_only_when_the_condition_holds ... ok
test requirement::tests::an_advisory_tier_cannot_be_declared_without_an_owner_or_an_exit_criterion ... ok
test requirement::tests::one_edge_has_to_satisfy_both_halves_of_a_bound_relation ... ok
test requirement::tests::missing_evidence_is_unknown_and_agent_evidence_does_not_count_as_independent ... ok
test requirement::tests::conformance_evidence_from_an_older_revision_does_not_satisfy_a_current_requirement ... ok
test review::tests::a_review_outcome_reads_and_writes_the_three_words_it_was_recorded_with ... ok
test requirement::tests::reports_list_every_unmet_requirement_with_a_reason ... ok
test requirement::tests::the_same_requirement_blocks_when_it_is_not_advisory ... ok
test requirement::tests::the_bound_declaration_survives_a_round_trip ... ok
test review::tests::a_revision_bound_artifact_needs_a_versioned_review ... ok
test review::tests::an_approval_of_version_three_does_not_cover_version_seven ... ok
test review::tests::blocking_findings_defeat_an_approval ... ok
test time::tests::a_date_maps_to_midnight_utc_on_that_day ... ok
test time::tests::a_date_that_is_today_at_utc_plus_fourteen_is_not_in_the_future_and_tomorrow_everywhere_is ... ok
test time::tests::a_date_the_calendar_does_not_have_is_refused_by_name ... ok
test task::tests::rejects_an_unparsable_artifact_reference ... ok
test time::tests::a_date_without_zero_padding_is_refused_rather_than_guessed ... ok
test time::tests::a_horizon_over_an_observation_that_is_not_at_midnight_is_exact_to_the_instant ... ok
test requirement::tests::the_revision_binding_leaves_alone_what_it_says_nothing_about ... ok
test time::tests::a_horizon_that_is_a_phrase_is_not_a_horizon ... ok
test task::tests::task_facts_include_the_kind_and_declared_constraints ... ok
test task::tests::parses_the_documented_task_shape ... ok
test time::tests::an_instant_is_spelled_as_iso_8601_in_utc ... ok
test time::tests::an_observation_in_the_future_is_recognised_rather_than_aged_backwards ... ok
test time::tests::an_observation_exactly_its_horizon_old_is_still_covered ... ok
test time::tests::the_corpus_reference_date_is_seven_days_after_its_boundary_observation ... ok
test time::tests::every_date_in_the_corpus_survives_the_round_trip_through_an_instant ... ok
test time::tests::the_epoch_spelling_keeps_the_exact_comparison_a_day_does_not_get ... ok
test time::tests::the_horizon_token_is_read_in_every_spelling_the_corpus_uses ... ok
test time::tests::the_parser_accepts_every_spelling_the_published_pattern_describes ... ok
test verification::tests::a_seed_is_taken_in_whatever_spelling_its_tool_uses ... ok
test verification::tests::a_seed_nobody_could_act_on_is_refused ... ok
test time::tests::the_two_spellings_name_one_instant_and_are_not_one_claim ... ok
test verification::tests::a_trace_checker_is_a_named_class_and_not_an_external_tool ... ok
test verification::tests::inconclusive_is_not_a_pass ... ok
test time::tests::the_schema_publishes_the_string_form_and_not_only_the_integer ... ok
test verification::tests::unknown_verifiers_become_external_tools ... ok
test version::tests::distinguishes_a_version_suffix_from_a_namespaced_id ... ok
test version::tests::parses_protocol_refs ... ok
test version::tests::protocol_refs_require_a_version ... ok
test version::tests::rejects_zero_versions ... ok
test version::tests::unpinned_refs_accept_any_version ... ok
test workflow::tests::rejects_a_non_terminal_state_with_no_way_out ... ok
test workflow::tests::rejects_a_transition_to_a_state_that_does_not_exist ... ok
test workflow::tests::rejects_a_rollback_policy_that_does_not_say_what_it_needs ... ok
test workspace::tests::a_git_member_has_no_store_path_until_something_materializes_it ... ok
test workflow::tests::rejects_an_unreachable_state_unless_it_is_declared_intentional ... ok
test workspace::tests::a_name_that_could_not_be_a_namespace_is_refused ... ok
test workflow::tests::rejects_duplicate_transitions_between_the_same_states ... ok
test workflow::tests::reports_every_problem_in_one_pass ... ok
test workspace::tests::a_member_takes_the_default_store_and_keeps_the_order_it_was_written_in ... ok
test workspace::tests::a_qualified_reference_is_never_ambiguous_however_many_members_hold_the_name ... ok
test workflow::tests::accepts_a_well_formed_workflow_and_indexes_it ... ok
test workspace::tests::a_reference_naming_two_members_is_refused ... ok
test workspace::tests::a_pinned_git_member_is_accepted_and_an_unpinned_one_is_not ... ok
test workspace::tests::a_reference_may_name_a_member_or_leave_it_to_where_it_was_written ... ok
test workflow::tests::rejects_rollback_on_an_irreversible_state ... ok
test workspace::tests::a_reference_into_a_member_that_does_not_hold_it_is_absent_rather_than_falling_back ... ok
test workspace::tests::a_slash_after_the_colon_is_part_of_the_name_and_not_a_member ... ok
test workspace::tests::a_relative_member_resolves_its_store_under_the_engineering_directory ... ok
test workspace::tests::an_unqualified_reference_held_by_two_members_is_ambiguous_rather_than_guessed ... ok
test workspace::tests::an_unqualified_reference_takes_the_member_it_was_written_in ... ok
test workspace::tests::a_workspace_with_no_members_is_refused_rather_than_read_as_empty ... ok
test workspace::tests::an_unreadable_format_version_is_refused_by_name ... ok
test workspace::tests::two_members_cannot_share_a_name ... ok
test workspace::tests::a_store_that_leaves_the_members_tree_is_refused ... ok

test result: ok. 289 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/determinism.rs (target/debug/deps/determinism-d812f6a3deddcc7e)

running 2 tests
test the_determinism_scan_sees_code_and_not_prose_and_not_operand ... ok
test the_domain_crate_reads_no_clock_no_randomness_and_no_unordered_map ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/ess_v2_sources.rs (target/debug/deps/ess_v2_sources-e36f6f415a946c42)

running 1 test
test raw_source_transport_has_no_admitted_facts_or_model ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ess_v2_time.rs (target/debug/deps/ess_v2_time-9fb55dd27c589f74)

running 2 tests
test actual_integer_observation_tokens_retain_full_u64 ... ok
test legacy_observed_at_scalar_and_calendar_controls ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/horizon_immutability.rs (target/debug/deps/horizon_immutability-742444bbcbc324a7)

running 4 tests
test the_extractors_see_the_constructs_they_exist_to_refuse ... ok
test an_evidence_record_has_no_horizon_field_for_anything_to_mutate ... ok
test no_shipped_code_assigns_to_a_horizon_after_it_has_been_read_from_a_document ... ok
test no_shipped_code_offers_an_operation_that_mutates_a_horizon_in_place ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/invariants.rs (target/debug/deps/invariants-3d230f08392703a7)

running 3 tests
test the_scan_reads_derives_and_impls_and_not_the_prose_about_them ... ok
test every_raw_document_type_is_checked_against_its_validated_counterpart ... ok
test a_validated_document_type_cannot_be_deserialised_straight_off_the_wire ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

     Running tests/safety_envelope.rs (target/debug/deps/safety_envelope-43c89003db8e72d4)

running 3 tests
test a_recorded_refusal_does_not_satisfy_the_approval_it_was_recorded_against ... ok
test a_refused_approval_projects_a_grant_fact_that_is_false ... ok
test a_floor_on_production_deployment_catches_a_profile_that_grants_every_environment ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/truth_laws.rs (target/debug/deps/truth_laws-5be8092b73893263)

running 7 tests
test negation_is_an_involution ... ok
test identities_annihilators_idempotence_and_absorption ... ok
test only_true_permits_and_composition_cannot_widen_it ... ok
test conjunction_and_disjunction_commute ... ok
test conjunction_and_disjunction_associate ... ok
test each_operation_distributes_over_the_other ... ok
test de_morgan_holds_in_both_directions ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (target/debug/deps/aep_engine-c1f1a901ac34d261)

running 55 tests
test clock::tests::a_fixed_clock_never_moves ... ok
test clock::tests::a_stepping_clock_moves_by_its_step ... ok
test clock::tests::the_system_clock_is_after_2020 ... ok
test engine::tests::once_a_task_says_what_it_is_about_evidence_may_not_stay_silent ... ok
test engine::tests::a_fact_observed_of_one_thing_does_not_move_another ... ok
test engine::tests::evidence_about_the_declared_subject_is_admitted ... ok
test engine::tests::rejects_evidence_the_protocol_does_not_declare ... ok
test execution::tests::a_refused_approval_is_not_counted_among_the_granted_ones ... ok
test engine::tests::a_refused_approval_enters_the_audit_trail_as_a_refusal ... ok
test engine::tests::a_task_that_declares_no_subject_is_unchanged ... ok
test engine::tests::explains_completion_as_a_checklist ... ok
test engine::tests::reports_other_permitted_transitions_rather_than_hiding_the_choice ... ok
test engine::tests::walks_a_workflow_from_start_to_completion ... ok
test policy::tests::a_denied_capability_cannot_be_unlocked_by_an_approval ... ok
test execution::tests::submission_order_is_observable_so_ordering_rules_are_checkable ... ok
test execution::tests::a_snapshot_round_trips_through_a_freshly_resolved_plan ... ok
test execution::tests::a_snapshot_from_another_task_is_refused ... ok
test execution::tests::starts_in_the_workflows_initial_state ... ok
test policy::tests::a_capability_nobody_granted_is_refused_with_what_is_missing ... ok
test engine::tests::a_refused_action_is_recorded_with_the_rule_that_refused_it ... ok
test engine::tests::a_blocked_transition_says_what_is_missing_and_records_it ... ok
test engine::tests::replays_identically_under_a_fixed_clock ... ok
test policy::tests::an_approval_may_also_be_matched_by_its_identifier ... ok
test policy::tests::a_state_grant_cannot_get_past_the_protocols_approval_floor ... ok
test resolve::tests::a_profile_that_forgot_the_private_denial_is_refused_by_the_floor ... ok
test policy::tests::an_approval_recorded_as_evidence_unlocks_the_capability ... ok
test policy::tests::an_allowed_action_is_allowed_and_says_which_document_granted_it ... ok
test policy::tests::deploying_to_an_environment_nobody_granted_is_not_granted ... ok
test policy::tests::production_write_is_refused_and_names_the_rule_that_refused_it ... ok
test resolve::tests::a_principle_can_take_a_capability_away_and_the_reason_is_recorded ... ok
test resolve::tests::a_resolved_plan_denies_the_private_read_the_profile_denied ... ok
test execution::tests::counts_unmet_evidence_requirements_without_consulting_predicates ... ok
test execution::tests::the_first_test_result_is_remembered_after_a_later_pass ... ok
test policy::tests::a_reviewer_who_refuses_a_production_change_has_not_thereby_permitted_it ... ok
test policy::tests::a_refused_approval_matched_by_its_identifier_unlocks_nothing_either ... ok
test resolve::tests::a_removal_that_does_nothing_is_an_error ... ok
test resolve::tests::a_task_may_name_the_base_protocol_its_profile_refines ... ok
test resolve::tests::rejects_a_completion_condition_reading_an_unobservable_fact ... ok
test resolve::tests::an_added_principle_joins_the_obligations ... ok
test resolve::tests::rejects_a_task_whose_protocol_the_profile_does_not_refine ... ok
test resolve::tests::rejects_an_obligation_timed_against_a_phase_no_state_declares ... ok
test resolve::tests::records_every_dropped_principle ... ok
test resolve::tests::a_principle_whose_condition_excludes_the_task_is_not_in_force ... ok
test resolve::tests::a_task_cannot_need_a_capability_the_policy_denies ... ok
test resolve::tests::refuses_to_grant_production_write_outright ... ok
test trail::tests::a_command_issued_during_an_execution_inherits_its_activity ... ok
test trail::tests::a_blocked_transition_is_audited_with_what_was_missing ... ok
test resolve::tests::a_profile_extending_another_inherits_its_workflow_and_tightens_completion ... ok
test resolve::tests::rejects_an_unknown_principle ... ok
test resolve::tests::resolves_a_task_into_a_plan ... ok
test trail::tests::evidence_and_transitions_are_recorded_with_the_execution_and_task ... ok
test trail::tests::evidence_stored_as_an_entity_is_pointed_at_rather_than_copied ... ok
test trail::tests::bookkeeping_events_do_not_become_audit_records ... ok
test trail::tests::a_refused_action_becomes_an_audit_record_naming_the_rule ... ok
test trail::tests::the_trail_is_identical_across_replays ... ok

test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/adopting_guide.rs (target/debug/deps/adopting_guide-df6f24a80fe52f49)

running 3 tests
test the_guides_owned_tree_example_loads_and_the_teams_own_ladder_is_in_force ... ok
test pointing_at_a_tree_merges_principles_and_profiles_and_nothing_else ... ok
test without_the_two_redirects_the_owned_tree_is_refused_exactly_as_the_guide_shows ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/contract_gate.rs (target/debug/deps/contract_gate-dec18e58608eaa0b)

running 2 tests
test a_run_that_never_heard_from_a_contract_runner_does_not_enter_review_by_saying_nothing ... ok
test a_breaking_record_does_not_reach_review_and_a_red_one_does ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/document_tree_order.rs (target/debug/deps/document_tree_order-f52e49e0fe742d7b)

running 1 test
test a_repository_with_no_drivers_directory_loads_exactly_as_before ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/document_tree_order_adversarial.rs (target/debug/deps/document_tree_order_adversarial-9ec595a047923d23)

running 6 tests
test drivers_is_the_last_row_of_the_loaders_table ... ok
test the_table_holds_one_row_for_every_kind_the_loader_accepts ... ok
test the_walk_reads_every_row_of_the_table_in_the_order_it_is_written ... ok
test a_dot_prefixed_file_under_drivers_is_not_a_step_map ... ok
test a_dot_prefixed_directory_under_drivers_is_not_descended_into ... ok
test a_drivers_directory_that_yields_no_documents_loads_like_one_that_is_absent ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/documents.rs (target/debug/deps/documents-ef0d7f8976b13056)

running 7 tests
test a_development_task_can_be_walked_to_completion_with_evidence ... ok
test a_task_that_declares_no_code_change_owes_no_contract_or_property_evidence ... ok
test the_approval_floor_is_in_force_for_every_shipped_profile ... ok
test a_profile_that_grants_production_outright_is_refused_under_every_protocol ... ok
test the_document_tree_loads_and_is_internally_consistent ... ok
test every_profile_resolves_for_a_task_of_its_kind ... ok
test a_task_that_declares_nothing_still_owes_contract_and_property_evidence ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/end_to_end.rs (target/debug/deps/end_to_end-e0a9da9fa78abaa0)

running 15 tests
test conformance_evidence_for_one_specification_does_not_satisfy_a_requirement_about_another ... ok
test an_approval_of_design_version_three_does_not_cover_version_seven ... ok
test a_task_without_a_specification_owes_no_conformance ... ok
test changing_production_is_refused_and_names_the_rule_that_refused_it ... ok
test a_specification_governed_task_is_not_finished_until_something_else_says_it_conforms ... ok
test completion_is_refused_with_the_missing_requirements_named ... ok
test a_conformance_run_against_an_older_revision_leaves_the_requirement_owed ... ok
test work_cannot_be_decomposed_before_a_specification_exists ... ok
test a_conditional_requirement_that_does_not_apply_is_not_counted_as_missing ... ok
test a_run_that_found_the_implementation_wrong_leaves_the_task_open_and_says_which_scenario ... ok
test failing_contracts_send_the_work_back_to_implementation ... ok
test a_passing_test_submitted_before_any_code_fails_red_before_green ... ok
test the_example_walks_to_completion_on_its_own_evidence ... ok
test the_example_documents_are_valid_and_resolve ... ok
test a_task_governed_by_a_specification_finishes_only_on_a_conformance_run_it_did_not_produce ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/evidence_horizons.rs (target/debug/deps/evidence_horizons-ed6720b22d65a00a)

running 8 tests
test an_observation_in_the_future_is_refused_and_never_stored ... ok
test a_day_that_has_begun_somewhere_is_admitted_and_one_that_has_begun_nowhere_is_still_refused ... ok
test past_its_horizon_the_requirement_reads_unknown_and_names_the_horizon_and_the_observation ... ok
test inside_its_horizon_a_test_run_satisfies_the_requirement_and_permits_the_transition ... ok
test the_transition_a_lapsed_record_used_to_permit_is_refused_rather_than_taken ... ok
test the_missing_count_and_the_requirement_outcome_agree_about_a_lapsed_record ... ok
test re_submitting_the_identical_record_restores_nothing_and_a_new_observation_does ... ok
test a_snapshot_restored_after_its_horizon_re_decays_from_the_same_bytes ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/evidence_scan.rs (target/debug/deps/evidence_scan-4c5ae27691340cc4)

running 2 tests
test the_scan_reads_constructions_and_not_patterns_or_prose ... ok
test the_engine_constructs_no_evidence_payload_outside_its_test_modules ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/lifecycle_fallback.rs (target/debug/deps/lifecycle_fallback-3894966d8128badb)

running 6 tests
test without_the_fallback_document_the_same_kind_is_governed_by_nothing ... ok
test the_fallback_makes_a_status_on_an_unregistered_kind_refusable ... ok
test a_second_kind_less_document_is_one_refusal_and_the_first_still_stands ... ok
test the_family_ladder_wins_over_the_fallback_and_the_fallback_answers_last ... ok
test a_kind_less_document_governs_every_kind_nothing_nearer_names ... ok
test a_family_of_custom_kinds_shares_the_ladder_its_last_segment_names ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/project_directory_env.rs (target/debug/deps/project_directory_env-899dec9e735060cc)

running 1 test
test the_environment_names_the_project_directory_and_nothing_else_is_a_project ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/task_scoped_artifacts.rs (target/debug/deps/task_scoped_artifacts-382fd247b4513cdc)

running 4 tests
test a_specification_naming_the_task_itself_counts_as_this_tasks ... ok
test another_storys_approved_specification_does_not_open_implementation ... ok
test a_task_that_declares_no_work_admits_no_storys_specification ... ok
test the_tasks_own_approved_specification_opens_implementation ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running unittests src/lib.rs (target/debug/deps/aep_ess_evidence-cc097569c125478a)

running 5 tests
test count_json::tests::unsigned_scalars_cover_full_u64_without_large_outcome_allocations ... ok
test tests::a_passing_report_with_a_failed_scenario_is_refused ... ok
test tests::a_report_field_added_without_a_format_change_is_refused ... ok
test tests::a_standalone_report_becomes_the_existing_aep_evidence_shape ... ok
test tests::an_unknown_report_version_is_refused_before_fields_are_interpreted ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/count_reader_pass1.rs (target/debug/deps/count_reader_pass1-b82779d19beaf69a)

running 4 tests
test decoded_duplicate_keys_refuse_in_report_and_arbitrary_nested_payload ... ok
test closed_scenario_values_and_shapes_do_not_close_literal_payload_keys ... ok
test structural_and_textual_predicate_depth_share_the_frozen_limit ... ok
test count_tokens_remain_strict_while_legacy_payload_numbers_keep_their_grammar ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/counts.rs (target/debug/deps/counts-cc1031955c97003d)

running 11 tests
test detailed_run_and_future_suite_refuse_before_interpreting_their_body ... ok
test source_readback_cannot_restore_or_retain_a_failed_admission ... ok
test exact_original_suite_bytes_and_selected_membership_are_required ... ok
test duplicate_and_unknown_closed_fields_refuse_even_inside_unused_metadata ... ok
test report_completion_tokens_are_exact_and_lexically_unsigned ... ok
test original_suite_pairs_admit_with_exact_time_and_unknown_coverage ... ok
test unsupported_format_profile_policy_and_coverage_reasons_are_distinct ... ok
test zero_and_mixed_terminal_selections_keep_each_profile_aggregate_truthful ... ok
test frozen_predicate_depth_and_quantified_metadata_are_admitted_without_rewriting_bytes ... ok
test category_arithmetic_lists_and_producer_aggregates_are_checked ... ok
test complete_frozen_step_shape_view_and_dependency_vocabulary_is_read_for_every_legacy_major ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/aep_schema-02c997dc6a13fcb2)

running 28 tests
test alias::tests::the_list_is_sorted_so_a_new_alias_lands_where_a_reader_looks_for_it ... ok
test alias::tests::no_alias_is_listed_twice_and_none_renames_a_spelling_to_itself ... ok
test parse::tests::document_kinds_map_to_repository_directories ... ok
test parse::tests::a_duplicated_key_is_refused_in_a_document_with_no_raw_stage_too ... ok
test parse::tests::a_profile_that_writes_one_key_twice_is_refused_with_the_key_named ... ok
test parse::tests::reads_json_as_well_as_yaml ... ok
test parse::tests::a_shape_error_still_says_which_line_it_is_on ... ok
test parse::tests::reads_an_evidence_list_and_keeps_a_payloads_own_subject ... ok
test parse::tests::separates_syntax_errors_from_semantic_errors ... ok
test schema::tests::every_schema_says_what_it_describes ... ok
test schema::tests::generates_a_schema_for_every_document_and_interchange_type ... ok
test schema::tests::the_workflow_schema_publishes_identifier_patterns ... ok
test schema::tests::the_objective_schema_accepts_the_one_line_form_every_task_is_written_with ... ok
test schema::tests::the_evidence_kind_schema_publishes_the_spellings_the_parser_accepts ... ok
test schema::tests::the_capability_policy_schema_accepts_the_spelling_every_principle_uses ... ok
test schema::tests::the_step_map_schema_refuses_the_unpinned_workflow_its_validator_refuses ... ok
test schema::tests::the_approval_requirement_schema_accepts_a_bare_id_and_refuses_a_mapping_naming_none ... ok
test schema::tests::the_artifact_requirement_schema_accepts_a_bare_kind_and_refuses_an_unknown_status ... ok
test schema::tests::the_relation_requirement_schema_accepts_a_bare_relation_and_refuses_an_invented_one ... ok
test schema::tests::the_relation_requirement_schema_carries_the_task_binding_and_refuses_an_invented_one ... ok
test schema::tests::the_review_requirement_schema_accepts_a_bare_kind_and_refuses_a_mapping_naming_no_subject ... ok
test schema::tests::no_two_schemas_claim_the_same_file ... ok
test schema::tests::the_evidence_requirement_schema_accepts_a_bare_kind_and_refuses_a_mapping_with_no_kind ... ok
test schema::tests::the_step_schema_gives_an_llm_step_nowhere_to_put_evidence ... ok
test schema::tests::the_requirement_set_schema_accepts_every_form_a_state_may_be_written_in ... ok
test schema::tests::the_conditional_requirement_schema_accepts_both_spellings_of_what_is_then_owed ... ok
test schema::tests::schemas_are_valid_json_with_a_title_and_a_trailing_newline ... ok
test schema::tests::generation_is_deterministic ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/published.rs (target/debug/deps/published-8bb4f5727b48fc24)

running 7 tests
test no_type_carries_an_alias_no_published_schema_would_ever_be_asked_about ... ok
test count_stage_evidence_schema_refuses_cache_claims_but_accepts_raw_source_strings ... ok
test the_alias_scan_finds_the_attributes_it_is_supposed_to_find ... ok
test no_published_alias_is_a_spelling_the_parsers_refuse ... ok
test every_published_schema_accepts_every_spelling_its_parser_does ... ok
test the_document_schemas_accept_every_document_this_repository_ships ... ok
test an_aliased_field_is_never_left_required_under_its_canonical_spelling ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

   Doc-tests aep_domain

running 2 tests
test crates/govern/aep-domain/src/raw.rs - raw (line 16) - compile ... ok
test crates/govern/aep-domain/src/ids.rs - ids::identifier_pattern (line 161) ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

   Doc-tests aep_engine

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests aep_ess_evidence

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests aep_schema

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

exit: 0
```

4. Findings

Nothing found in this bounded first pass.

| File:line | Verdict | Origin | What was measured | What reaches it |
|---|---|---|---|---|

There are no judgement findings or unresolved semantic counterexamples. The preliminary missing-trait compilation errors and incorrect CLI wording assertion are review-test construction mistakes, preserved above; neither is assigned a production origin or promoted to a finding.

5. What this pass tried

- Decoded duplicate keys, including Unicode spellings nested in unconstrained payload maps: refused with exact structured paths, while unique escaped keys remained valid.
- Six count fields with noncanonical numeric spellings and u64 overflow, alongside finite-number legacy payload controls: lexical count admission remained strict without narrowing payload grammar.
- Combined structural and textual predicate nesting at 32/33: the original frozen boundary held.
- Closed ScenarioValue and LeafShape metadata beside arbitrary literal payload keys and shape field names: typed metadata closure held without closing data fields.
- Original CRLF and Unicode escape bytes through JSON/YAML paired evidence batches: exact full-u64 time and bytes survived; either invalid batch position and an absent reader refused.
- Exact one-day horizon at u64::MAX and one millisecond beyond it, using an initialized execution with its independent task/model/suite expectation: the fresh boundary remained UnknownCoverage and the older observation became StaleObservation.
- Public direct-record mutation and restore of nonempty execution: changed original suite bytes and a one-millisecond future observation refused; snapshot bytes, facts and evaluation time remained unchanged.
- Actual aep/protocol inspect aliases: full-u64 completion output, future-observation refusal, source record identification and whole-batch malformed-input refusal held.

The accepted design and complete frozen source diff were read before these cases were written, together with the original brief/corrections, implementation tests and public callers. The frozen ESS suite vocabulary was compared read-only against ESS ba43fda29de637ad9323d96c4bb9aac10f48ae64. This pass did not add a writer, assume suite/5 or run/2 support, demand complete-coverage success, substitute a dishonest custom Rust reader, mutate production to probe a guard, or execute a second review pass. Existing package cases supply the inherited planning-store, driver, schema and legacy controls in the suite output above; no new planning command was introduced by this pass.

6. Paths and process closure

All three new source paths are tests or test support under the assigned package directories. The tracked diff is empty and the complete untracked inventory is printed above. No old test, production source, manifest, lockfile, generated document, planning store, Git index/ref/object, registry or lifecycle entry was intentionally written by this review.

Assigned scratch, logs, run script, focused report, output capture and fixture files remain beneath:
`~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/target/ess-conformance-v2-counts/adversary-pass-1`.

Local compilation used the existing target at:
`~/.local/state/worktree/trees/b10x/aep/aep-ess-conformance-v2-counts/target`.
Inherited package tests also used their established in-tree test fixture locations; TMPDIR was the assigned scratch for child processes. No new build directory outside this worktree was selected.

One observed pre-existing infrastructure metadata path outside the worktree was refreshed by Cargo during these runs:
`~/.cargo/.global-cache` (size 1,007,616 bytes before and after; mtime moved from 2026-09-06 09:59:05.661875980 +0200 to at least 2026-09-06 10:02:49.329071322 +0200). No authored file was written outside the worktree. The ordinary Cargo lock files `~/.cargo/.package-cache` and `~/.cargo/.package-cache-mutate` retained their earlier mtimes. The inherited sccache socket was absent; RUSTC_WRAPPER and SCCACHE_SERVER_UDS were unset, and this review launched no cache daemon or shared build.

Both focused runner sessions and the full package runner exited. The full suite shell/Cargo/test PIDs 3803506, 3803572 and 3815725 were absent at final process inspection. No service, background test or integration process was left running by this review. Free space remained above the 8 GiB floor throughout observed checks; the final check reported 35 GiB available.

The focused-section SHA256 and all three final source hashes were captured before the suite and verified unchanged afterward. The frozen subject still reads fc58d0fb365f04f2c92e0a7cc7a278f85b55ec8e, and the implementation handoff still hashes to edaa883c03ccf116fba536b00e9600e096f4d7723f02c16084b7d9a1d15a2571. This report is returned once and will not be revised by this reviewer. All test and scratch writes are relinquished to the coordinator upon handoff.

```findings
[]
```
