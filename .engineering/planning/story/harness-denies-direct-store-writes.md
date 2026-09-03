---
format: aep.planning-md/1
id: story:harness-denies-direct-store-writes
kind: story
status: draft
title: The harness refuses a direct write to the planning store, so no prose has to ask it not to
relations:
- informed_by: story:skill-text-cannot-instruct-a-direct-store-write
scope:
- confidence: inferred
  path: .claude
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/protocol-cli/tests
- confidence: cited
  path: crates/protocol-cli/tests/store_selection.rs
revision: 6
---
# Story: The harness refuses a direct write to the planning store, so no prose has to ask it not to

## Outcome

An agent working in this repository cannot edit a file under `.engineering/planning/` with `Edit`,
`Write` or `NotebookEdit`. It can still read them. The refusal is a path match, so it cannot
misjudge a sentence.

## Context

The planning store has one writer, `protocol artifact`. Three attempts have now been made to hold
that line, and only the mechanical ones work.

| attempt | instrument | outcome |
|---|---|---|
| prose in every shipped skill | English | regressed once; a skill *told* agents to patch bodies directly |
| a source scan over that prose | 2,153 lines of Rust classifying sentences | built, attacked twice, corrected three times, **deleted 2026-08-30** — went red on two correct sentences the first time somebody edited a skill normally |
| `protocol artifact validate` | the store's own event log | works, deterministically, but only at gate time and only for documents that have events |

This story is the fourth, and it is the only one that acts **before** the write rather than after it.

**It is not hypothetical.** During the wave of 2026-08-30 an adversary agent — one whose charter
says it never writes the planning store, dispatched with an instruction saying the same — ran
`protocol artifact new` without `--store`, wrote `story/title-check.md` into the worktree's real
store and appended a `journal.jsonl` line. It disclosed this and reverted by hand. The revert was
verified (623 = 623 journal lines, `git status` empty, `validate` `valid`), so nothing reached the
tree. **The rule was stated in two places and the agent broke it anyway**, which is the argument
for enforcement that is not a sentence.

**Where the configuration goes.** `AGENTS.md` states the placement rule: Claude-specific config in
the repository's own `.claude/`, harness-neutral definitions in `.agents/`. This repository has
**no `.claude/` directory at all** (checked 2026-08-30), so this is greenfield.

## Acceptance

- A `PreToolUse` deny on `Edit`, `Write` and `NotebookEdit` for `.engineering/planning/**`, in this
  repository's own `.claude/`.
- **Reads are unaffected.** A wave's implementors read the story body they are given; that is the
  whole input to a unit, and denying it would break every dispatch.
- The refusal names `protocol artifact` as the route, so an agent that hits it knows what to do
  next rather than retrying.
- The repository's own test suites still pass. This is the hard half — see below.

## Out of Scope

- **Codex and any other harness.** A `.claude/` rule binds Claude Code only. Whether the same
  refusal is expressible harness-neutrally under `.agents/` is a second question and a second
  story.
- **Denying reads.**

## Open Questions

**Does the deny cover the whole repository or only `.engineering/planning/`?** Decides: protocol
owner. Default if nobody answers: **only `.engineering/planning/`** — this repository's own store,
one path. A wider match breaks the test suites described below.

## Not established

**This repository is the hard case, and the reason is worth stating before anybody starts.** Here
the planning store is both the product's subject *and* its test fixture: `crates/protocol-cli/tests/`
creates and edits stores constantly, and several suites assert on what a hand edit does — the drift
tests exist precisely to check that an out-of-band edit is caught. A deny that matched every
`planning` path would break the tests that prove the rule.

The split that looks right, **inferred and not measured**: deny `.engineering/planning/**`, this
repository's own store, and leave scratch stores alone. Test fixtures already build under
`CARGO_TARGET_TMPDIR`, which `crates/protocol-cli/tests/store_selection.rs:77` asserts lies under
the repository root — so the two are distinguishable by path. **Nobody has run the suite under a
deny rule to confirm it**, and that measurement is the first task of this story, not an assumption
it may rest on.
