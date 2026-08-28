---
format: aep.planning-md/1
id: story:tool-availability-expectation
kind: story
status: implemented
title: 'env.tool_available: the 50th expectation kind'
summary: A trace specification can assert which tools a session was offered, so the per-state allowlist has something that audits it.
owner: trace
tags:
- driver
- trace
relations:
- decomposes: epic:reference-driver
revision: 5
---
# Story: `env.tool_available` — the 50th expectation kind

## Outcome

Someone reading a run can tell which tools the session was offered, and a specification can require
that a step held only the tools its state permits — so the per-state allowlist is audited rather than
asserted.

## Context

The per-state tool set is the driver's primary enforcement mechanism, and the standard the design
sets for itself is that *an enforcement mechanism nobody audits is a claim*. `SessionStart.tools` is
already in the IR; what is missing is a kind that can read it. It ships **first** in the driver
sequence for exactly that reason: shipping the allowlist before the thing that audits it would meet
the letter of the design and not its standard.

## Acceptance

- A specification asserting a tool was offered passes against a transcript whose `SessionStart` lists
  it, and gaps against one that does not.
- A specification asserting a tool was **not** offered gaps when the session was given it.
- A transcript whose adapter could not read the offered tool list yields `unk`, not a pass.
- The existing drift test that asserts the raw and validated vocabularies agree catches a half-done
  job — a variant added without its name arm fails.

## Shipped — read against the code, 2026-08-28

All four acceptance lines hold, with named tests. Runs on 2026-08-28: `cargo test -p trace-spec
--lib` → 82 passed; `cargo test -p trace-domain --lib` → 54 passed; `cargo test -p aep-driver --test
shell_echo` → 6 passed.

| line | where it holds |
|---|---|
| a spec asserting a tool was offered passes, and gaps when it was not | `crates/trace-spec/src/check.rs:566-568`; `every_kind_holds_on_the_real_run_and_a_negative_case_beside_it_does_not`, `check.rs:2476` |
| asserting a tool was **not** offered gaps when the session was given it | `check.rs:569-571`, `env_withholds` `check.rs:611-636` |
| a transcript whose adapter could not read the list yields `unk`, not a pass | `check.rs:623`; `a_tool_expectation_over_a_run_that_recorded_no_tool_list_is_undecidable_not_a_gap`, `check.rs:2995` |
| the drift test catches a variant added without its name arm | `crates/trace-domain/src/raw.rs:2984`, `spec.rs:1085`, `env_tool_available_reads_its_three_claims_and_refuses_the_ways_they_can_be_written_wrong`, `raw.rs:2876` |

End to end as well as in the unit: `crates/aep-driver/tests/shell_echo.rs:192,198` runs an
`env.tool_available` specification against a real driven transcript.

**Two corrections to this story's own text.** The title says *the 50th* expectation kind; the
published vocabulary is now **51** (`spec.rs:1088`). And what the kind audits is narrower than the
Context claims: `SessionStart.tools` is the harness's tool **inventory**, not the session's allow
rules — the committed fixture was launched with nine allowed tools and lists thirty-two
(`docs/plan/gap-register.md:40`). The kind stays load-bearing because it rules out *the tool did not
exist* as an explanation for a refusal, which is what makes a refusal attributable to a layer that
chose to refuse. It does not audit the allowlist, and this story is closed on what it does.

## Out of Scope

Asserting that a tool was *used*; tool traffic already has kinds. This one is about what the session
was offered, which is the only thing that can audit an allowlist.

## Open Questions

None. The kind mirrors `env.skill_available` line for line, and the shape is settled.
