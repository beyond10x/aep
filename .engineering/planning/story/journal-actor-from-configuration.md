---
format: aep.planning-md/1
id: story:journal-actor-from-configuration
kind: story
status: draft
title: The journal actor comes from configuration, and the default names nobody
revision: 1
---
# Story: the journal actor comes from configuration, and the default names nobody

## Goal
Every store write (`artifact new`, `move`, `relate`, `body`, `evidence`, `scope`) is journalled with an
actor. Today `command_actor()` (`crates/protocol-cli/src/planning.rs:1440`) takes `AEP_ACTOR` when it
is set and otherwise stamps `human:<$USER>` (`planning.rs:1486`). The environment override works and
is what a driven session uses; nothing else declares the actor, so an operator running the CLI by
hand in a public repository publishes their login name in `journal.jsonl` on every write. Measured
on 2026-09-04 on `origin/main`: aep 615 lines, ess 77, connectors 52, worktree 2 carry `human:<name>`.

The actor must be declarable in configuration, with a precedence a reader can predict, and the
undeclared default must not be a person's login.

## Shape
- Precedence: `AEP_ACTOR` (per process) → `actor:` in `.engineering/project.yaml` (per project) →
  `actor` in the user's own configuration (`$XDG_CONFIG_HOME/b10x/aep.toml`, per machine) →
  the default.
- The default is `human:operator`. A project that wants login attribution says so
  (`actor: human:$USER` is refused; a project declares a literal actor or nothing).
- `ActorRef::parse` validates every source the same way; a malformed value is refused naming the
  source it came from (env, project file, user file), never defaulted — the existing behaviour of
  `a_malformed_declared_actor_is_refused_naming_the_variable_and_never_defaulted` extended to the
  two new sources.
- `aep artifact validate` warns once when a journal line of the current store carries a
  `human:` actor that equals the current login, so a public repository learns about the pattern
  before the next push; it does not rewrite history.
- `docs/guide` gains the precedence table and the one-line project setting.

## Acceptance
- With no `AEP_ACTOR`, no project setting and no user setting, `artifact new` journals
  `human:operator`.
- `actor: agent:planning-bot` in `project.yaml` is what a write is journalled as; `AEP_ACTOR` set in
  the same shell wins over it; the user file loses to both.
- A malformed value in any of the three sources is refused by name.
- The existing driven-session behaviour (`AEP_ACTOR=agent:<run>` set by `drive`) is unchanged.
