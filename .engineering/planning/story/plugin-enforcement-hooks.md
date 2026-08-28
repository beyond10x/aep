---
format: aep.planning-md/1
id: story:plugin-enforcement-hooks
kind: story
status: archived
title: Hooks that deny, and a record of every denial
summary: PreToolUse denies from the per-state tool set, a write guard over the planning store, and the hook-decisions channel the driver folds into its run report.
owner: driver
tags:
- driver
- plugin
relations:
- decomposes: epic:reference-driver
- depends_on: story:tool-availability-expectation
- depends_on: story:protocol-drive-verb
revision: 5
---
# Story: Hooks that deny, and a record of every denial

## Outcome

A step that is not allowed to edit the plan cannot edit the plan, whatever the model decides to try —
and every refusal is in a channel the run report folds in, so *"nothing was denied"* and *"denials
are counted somewhere else"* stop looking identical.

## Context

`--allowedTools` is fixed at session launch, so the per-state tool set is enforced primarily by the
flag. The hook layer is the backstop over the same derived set, plus the one rule the flag cannot
express: a path check. `tool_input.file_path` is what makes the planning-store guard a path rule
rather than a tool ban — the model may write files, and it may not write these files.

## Acceptance

- A `PreToolUse` deny fires for a tool outside the state's derived set, and the reason reaches the
  model rather than only the log.
- A write to `.engineering/planning/**` from a step that is not permitted to move the plan is denied
  by path, with the tool itself still available for every other path.
- The driver's `claude -p` invocation carries `--settings` and never `--bare`, asserted on the
  constructed command line rather than by inspection.
- Denials land in `hook-decisions.jsonl` and appear in the run report, counted, with the rule that
  produced each.
- Hooks deny and never grant: a hook cannot widen the tool set the flag established.

## Superseded — 2026-08-28

**The outcome survives and the mechanism does not.** A step that may not edit the plan still cannot
edit the plan, and every refusal is still in a channel the run report folds in — but none of it is a
`PreToolUse` hook, because `epic:metaharness-migration` deleted `integrations/claude-code/hooks/` on
2026-08-22 and moved the decision in-process. `story:metaharness-executor` (implemented) is where
this story's outcome now lives.

| this story's line | what holds it today |
|---|---|
| a deny for a tool outside the state's set, with the reason reaching the model | `decide_tool` and `answer_events` in `crates/protocol-cli/src/drive.rs`, emitting `tool.decide {"decision":"deny","reason":…}` down the metaharness seam; `a_tool_outside_the_states_surface_is_denied_with_the_surface_named` |
| a write to `.engineering/planning/**` denied **by path**, the tool free elsewhere | `store_integrity` in `drive.rs`, matching `file_path`/`notebook_path`; `the_planning_stores_frontmatter_is_the_clis` |
| hooks deny and never grant | allow is the absence of a refusal; an engine deny wins; `a_call_the_engine_refuses_is_denied_and_the_refusal_is_in_the_executions_event_record` |
| `claude -p` carries `--settings` and never `--bare` | **gone with its subject.** The argv is now `metaharness run claude --hermetic --cwd … --frame … --decisions ask -p …`, pinned by `the_metaharness_argv_drives_the_seam_with_the_declared_directory_and_frame`; the bare vendor argv and the settings file both left `drive.rs` on 2026-08-22 |
| denials in `hook-decisions.jsonl`, counted in the run report | **gone with its subject.** Decisions are events in the transcript stream. The one `hook-decisions.jsonl` in this tree is a 2026-08-21 artefact of run `W4-1/1` and nothing writes the name any more |

Read against the code 2026-08-28: `cargo test -p protocol-cli --bin protocol drive::` → 24 passed.
The Open Question — *if plugin hooks need consent, does the driver ship its own settings file?* — is
answered by a third thing: the driver ships neither, and is itself the per-call decider through
`metaharness --decisions ask`.

## Out of Scope

Whether a plugin's hooks run without a per-invocation consent step. That is an assumption named in
the design and an unknown the wave could not close by reading; it is `story:driven-eval-acceptance`
that finds out.

## Open Questions

If plugin hooks turn out to need consent, does the driver ship its own settings file instead?
Decides: driver owner, on the evidence from the driven eval.
