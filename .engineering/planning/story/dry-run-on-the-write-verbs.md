---
format: aep.planning-md/1
id: story:dry-run-on-the-write-verbs
kind: story
status: draft
title: A write verb can say what it would do without doing it
summary: No verb has --dry-run, so the only way to find out what a bulk write does is to do it.
revision: 1
---
## Context

Every write verb commits. `new`, `move`, `relate`, `body` and `set` each take effect the moment they
exit, and a search of `crates/*/src` and `crates/*/tests` finds no `--dry-run` on any of them.

This is what makes a bulk write expensive to be wrong about. The `story-migration` skill in
`beyond10x/agentplugins` exists partly to work around it: its first two steps produce the migration
plan as a document and stop, so a person can read what twenty creates would do before any of them
runs. That is a dry run implemented in prose because the CLI has none.

## Acceptance

- `new`, `move`, `relate`, `body` and `set` accept `--dry-run`.
- With it, each prints exactly what it would print on success, writes no file, and appends nothing
  to `journal.jsonl`.
- A refusal is still a refusal under `--dry-run`: a `move` the ladder does not allow exits 1 and
  names the legal rungs, because the value of the flag is finding that out without a write.
- A test asserts the journal is byte-identical before and after a `--dry-run` sweep.

## Evidence for the gap

`crates/protocol-cli/src/planning.rs` — no verb declares the flag. The nearest existing precedent
for verify-then-commit is `reverse init` (`reverse.rs`), which verifies its protocol source before
writing `project.yaml` and cleans up only the directory it created.
