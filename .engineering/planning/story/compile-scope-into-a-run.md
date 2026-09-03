---
format: aep.planning-md/1
id: story:compile-scope-into-a-run
kind: story
status: draft
title: The step map's scope and context reach a run without a person retyping them
summary: Compile aep.driver-steps/1 scope and context into the arm's own flags, so the declaration is the source of truth rather than documentation.
owner: eval
tags:
- eval
- harness
relations:
- decomposes: epic:self-evaluation
revision: 5
---
# Story: the declaration reaches the run

## Outcome

Somebody who changes `scope:` or `context:` in `drivers/development/default.yaml` changes what the
next run may do. Today they change a document, and a person has to notice and retype it as flags.

## Context

Four live runs on 2026-08-24 (see `docs/reviews/2026-08-24-scope-cache-and-the-native-arm.md`) used
the declared scope, and every one of them had it hand-translated: six `paths` entries became six
`--write-scope <glob>=<word>` arguments, and one `context:` entry became one `--context <file>`. The
translation is mechanical, which is the argument for doing it in code rather than the argument for
leaving it to a person — a mechanical step a person performs is a step that will one day be skipped.

`aep-driver-spec` already parses and validates both keys, and the b10x arm already takes both flags.
What is missing is the compile between them, and the place for it is the adapter, beside `rendering()`
— the same place that already maps neutral operations onto a harness's own names.

## Decision, 2026-08-29: the compile belongs to the driver

**`protocol drive` owns the compile, and there is no second one.** The driver is the one place that
assembles a run from a step map, so it is the one place the declaration can become an argv without a
second copy to keep in step with the map. An eval runner that wants the declaration **launches
through `protocol drive run`** rather than retyping flags — which is this story's own Outcome, and
the reason the question does not split between the two halves.

Verified against the tree at `52d529c`:

* **The driver already compiles both keys.** `b10x_argv` renders `scope:` as ordered
  `--write-scope <glob>=<word>` arguments and `context:` as `--context <file>`
  (`crates/edge/protocol-cli/src/drive.rs:3706-3717`), and its caller `argv_for` hands it the validated
  step's own fields (`crates/edge/protocol-cli/src/drive.rs:1490-1496`). Nobody names a flag.
* **The eval-runner half is already on that route and needs nothing of its own.**
  `metaharness/evals/engineering-protocols/run-driven.sh:291` launches `protocol drive run --map
  "$MAP"`; the only arm flags it adds are machine facts a pinned map cannot carry —
  `--b10x-endpoint`, `--b10x-model`, `--b10x-wire`, the subscription token file and pointer, and
  `--b10x-cgroup-root` (`run-driven.sh:270-277`). It assembles no `b10x-harness` run argv. The one
  `b10x-harness` invocation it does make is the catalogue pre-flight `b10x-harness tools`
  (`run-driven.sh:172-174`), which spawns no session and carries no scope.
* The question was raised again by
  `docs/design/native-arm-store-integrity-design-v0.1.md` § 7 item 4, which records the driver half
  as done and directs the decision here rather than taking it.

This closes the open question. It does not close the story: one acceptance bullet is still unmet
below.

## Acceptance

- **Met.** A run assembled from a step-map step carries that step's `scope` and `context`, with
  nobody naming a flag — `argv_for` (`crates/edge/protocol-cli/src/drive.rs:1490-1496`) passes the
  parsed `LlmStep`'s own fields to `b10x_argv`, which renders them at
  `crates/edge/protocol-cli/src/drive.rs:3706-3717`. Asserted by
  `the_b10x_argv_carries_the_scope_and_never_the_frame_that_loop_would_refuse`
  (`crates/edge/protocol-cli/src/drive.rs:5491`), which calls `b10x_argv` directly, and by
  `the_committed_step_map_compiles_into_the_exact_argv_a_native_run_is_launched_with`
  (`crates/edge/protocol-cli/src/drive.rs:6693`), which goes through `argv_for` and so closes the one
  seam nothing used to assert: that the caller hands the compile the step's *own* fields.
  Mutation-checked — `&step.scope` replaced by `&[]` turns it red.
- **Met.** Rule order survives the compile, and the scope is an ordered list from the document to
  the argv. `LlmStep::scope` is a `Vec<ScopeRule>` documented *first match wins*
  (`crates/drive/aep-driver-spec/src/map.rs:645-652`); `validated_scope` keeps the written order, never
  sorts, and refuses a scope whose last rule does not name `**`
  (`crates/drive/aep-driver-spec/src/map.rs:315-359`); `b10x_argv` pushes one `--write-scope` per path in
  that order (`crates/edge/protocol-cli/src/drive.rs:3706-3712`). Asserted as an exact ordered vector at
  `crates/edge/protocol-cli/src/drive.rs:5539-5542`, with the word spelling held to the map's own by
  `the_write_scope_words_are_the_ones_the_step_map_is_written_in`
  (`crates/edge/protocol-cli/src/drive.rs:5750`), and the parse side by
  `a_step_may_declare_the_files_it_is_given_and_where_it_may_write`
  (`crates/drive/aep-driver-spec/src/map.rs:1613`) and
  `a_scope_whose_last_rule_is_not_a_catch_all_is_refused`
  (`crates/drive/aep-driver-spec/src/map.rs:1654`).
- **Met — by a test in the harness repository, and by a recorded decision that the refusal is
  the loop's to test.** `harness/crates/harness-cli/tests/context.rs:188`
  `a_declared_context_file_that_is_absent_refuses_the_run_before_any_session` runs `b10x-harness
  run --context <absent file>` against a fixture endpoint and asserts exit `1`,
  `{"kind":"refused"}` on `--json` stdout, an empty request record and an empty `--session-dir`:
  the refusal fires in `prepare` (`harness/crates/harness-cli/src/lib.rs:1159`, the error built at
  `:1636-1641`) before any request and before any write. `context.rs:199` pins the same for a
  declared-but-absent `--hooks` file (`harness/crates/harness-cli/src/hooks.rs:162-165`, propagated
  at `lib.rs:1185-1189`). Gate on that tree: 733 passed, 0 failed, 1 ignored. The test is in flight
  as an uncommitted patch on 2026-08-29 (`harness-ctx.patch`); this bullet closes when it lands.
  The other half — that *this* repository asserts only that the declared path travels, and the
  loop tests the refusal — is recorded in `docs/design/native-arm-store-integrity-design-v0.1.md`
  § 8.7 and needs no preflight here: `b10x_preflight` keeps testing the endpoint, the model, the
  binary and the adapter, never a declared file, because the loop refuses first and says which.
- **Met.** One test compiles the committed `drivers/development/default.yaml` step and asserts the
  exact argv: `the_committed_step_map_compiles_into_the_exact_argv_a_native_run_is_launched_with`
  (`crates/edge/protocol-cli/src/drive.rs:6693`). It reads the committed file, takes the `llm` step
  `receive` declares — `drivers/development/default.yaml:62-73`, one `context:` file and a
  three-rule `scope:` ending in the `**` catch-all — and asserts the whole vector `argv_for`
  returns for the native arm, in order: the six `--write-scope <glob>=<word>` arguments those three
  rules expand to, in the order the document writes them, then `--context
  integrations/claude-code/skills/planning/SKILL.md`, then `-p`, and no `--frame`, `--decisions`,
  `--plugin-dir` or `--substrate-embedded`. The tool config admits reading and not execution, so
  the argv is exactly assertable: with `CommandExecution` it would also carry `--allow-program`
  naming `std::env::current_exe()`, a different string in every checkout. The test asserts too that
  the declared context file is a file in this checkout, which catches a map naming a path a later
  commit moved — a check of the declaration, not the run-time refusal bullet three is about.
  Verified by mutation: `for rule in scope` iterated `.rev()`, and `argv_for` passing `&[]` instead
  of `&step.scope`, each turns it red with a message naming what went missing.

**Remaining, to close this story:** nothing in this repository. It moves along its lifecycle once
the harness test above is committed; until then bullet three rests on an uncommitted patch.

## Out of Scope

The vendor arms. Their scope travels as `Frame.subjects`, which is
`story:frame-subjects-from-the-step-map`. `context:` likewise reaches only the b10x arm today:
`metaharness_argv` (`crates/edge/protocol-cli/src/drive.rs:3534-3587`) renders no `--context` and no
`--write-scope`.
