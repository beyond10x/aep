---
format: aep.planning-md/1
id: story:shell-echo-harness
kind: story
status: implemented
title: A second harness with no model, no network and no credential
summary: A shell-echo LlmStepExecutor and a reader for its own transcript dialect, so harness-neutrality becomes a gate inside task check instead of a sentence.
owner: trace
tags:
- harness
- trace
relations:
- decomposes: epic:cross-harness-portability
- depends_on: story:driver-router
revision: 5
---
# Story: A second harness with no model, no network and no credential

## Outcome

Anyone can run `task check` and watch the neutrality claim be tested — and watch it go red when
somebody breaks the seam for a harness that is not Claude Code.

## Context

Today one adapter exists and *harness-neutral* is a property nothing has ever tested. A second
**real** harness would test it and would also need credentials, a network and a bill, which is why
the acceptance for the seam is a **fake** one: a shell script that reads a prompt on stdin, writes a
fixed set of files, and emits a transcript in a dialect of its own. It proves all three adapter
points at once — two executor implementations, one `tool_config` consumed by both, and a
`trace_conformance` record minted from a transcript no Claude Code wrote.

## Acceptance

- The shell-echo executor and its transcript reader run inside `task check`, with no model, no
  network and no credential.
- The same step map, the same workflow, the same `tool_config` function and the same checker drive
  both harnesses.
- The reader returns `TraceIr` with its own `AdapterRef`, and `check` plus `to_evidence` mint a
  record from it that `protocol evaluate --evidence` accepts.
- Breaking the executor seam for either harness fails the gate, naming which one.

## Shipped — read against the code, 2026-08-28

`crates/aep-driver/tests/shell_echo.rs` is the harness, the reader and the assertions, and it is in
the gate because `task test` is `cargo test --workspace`. Runs on 2026-08-28: `cargo test -p
aep-driver --test shell_echo` → 6 passed; `cargo test -p protocol-cli --test trace_cli` → 6 passed.

| line | where it holds |
|---|---|
| the executor and its reader run inside `task check` with no model, no network and no credential | the file drives `sh` as a **real subprocess** reading the prompt on stdin — `a_second_llm_executor_that_is_a_real_subprocess_walks_the_published_map_to_completion` (`shell_echo.rs:673`), over a real `drive` call, a `StepMap` parsed from YAML and a real `Registry` |
| the same step map, workflow, `tool_config` and checker drive both harnesses | `the_shared_tool_config_renders_into_this_harnesss_own_names_and_never_into_claude_codes` (`:718`), `a_shell_is_rendered_exactly_when_command_execute_is_admitted_and_never_otherwise` (`:735`), `no_subagent_spawner_is_rendered_however_much_the_policy_admits` (`:757`) |
| the reader returns `TraceIr` with its own `AdapterRef`, and `check` + `to_evidence` mint a record | `a_transcript_no_claude_code_wrote_is_checked_and_mints_a_trace_conformance_record` (`:810`): verdict `Ok`, 5 expectations, 0 gaps, **0 unknown**, and the minted record carries `EvidenceKind::TraceConformance`, the checker as producer, and `shell-echo/lines` as the adapter that judged the run |
| breaking the seam for either harness fails the gate, naming which | `the_claude_code_adapter_refuses_the_dialect_this_files_own_reader_understands` (`:781`) — the load-bearing direction: a dialect the first adapter happened to read would make the second reader decorative |

**One clause is covered by two tests rather than one, and it is recorded rather than glossed.** *…a
record `protocol evaluate --evidence` accepts* is asserted as a composition: `shell_echo.rs:810`
mints the record and asserts its kind, producer, status and adapter; `crates/protocol-cli/tests/trace_cli.rs`
asserts that a `trace_conformance` record written to disk is read and accepted by `protocol evaluate
--evidence`. No single test carries a shell-echo transcript all the way to the verb, because the
second reader is a free function inside the test file (§ 4.9 point 3's own decision) and the CLI
test cannot call it. Both halves are in the gate; the join is an argument, and this is it.

The live Codex `full` tier stays where it is: it costs money and needs a person at a keyboard.

## Out of Scope

Making the fake harness realistic. It is a seam test, not a simulator; anything it does beyond
exercising the three points is surface a real harness will contradict.

## Open Questions

None blocking. Whether the fake harness's dialect should be documented for outside use is decided
against for now: a dialect nobody outside this repository writes is not a format.
