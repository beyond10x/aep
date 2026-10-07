---
format: aep.planning-md/3
id: story:task-flag-keeps-project-protocols
kind: story
status: active
title: An explicit --task keeps the project's protocol source
refs:
- provider: github
  reference: beyond10x/aep#89
relations:
- serves: vision:O3
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/app.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/govern_resolve_task_in_project.rs
- confidence: cited
  path: docs/guide/README.md
- confidence: cited
  path: website/docs/guides/govern-a-task.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:06:12Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-07T08:06:12Z", actor: "human:timo", revision: 5}
---
## Outcome

`aep govern resolve --task <task>` (and every command that shares `ExecutionArgs`) run inside a
project resolves protocols from that project's configured `protocols` source when `--root` is not
given. An explicit `--task` chooses the task only; it no longer turns off project discovery.

## Acceptance

- In a project whose `.engineering/project.yaml` names an external pinned protocol source,
  `aep govern resolve --task <valid-task> --format json` exits 0 and resolves the protocol, without
  `--root`. Before the fix the same command refuses `unknown_protocol` (the failing test comes first).
- `--root <dir>` still takes precedence over the project's source (control test).
- With no project in the working directory or any parent, `--task` still resolves against `.`, as
  documented (control test).
- `--artifacts` still overrides the project's artifacts when given with `--task`.

## Origin

https://github.com/beyond10x/aep/issues/89, reproduced on aep 0.68.0: from a valid `aep.project/5`
working directory, `govern resolve --task` exits 1 with `unknown_protocol` and loads no protocol
documents; adding only `--root <the project's pinned protocol snapshot>` exits 0.

## Source

`crates/edge/aep-cli/src/app.rs:1607` `inputs`: `if let (Some(task), root)` returns before
`aep_project::project::discover`, and defaults the root to `.`.

## Specification

No ESS specification exists in this repository. This fix changes no noun, command, outcome or
configuration: it corrects the default of an existing flag to the behaviour the command's own help
documents.

## Scope

- cited: `crates/edge/aep-cli/src/app.rs`
- inferred: `crates/edge/aep-cli/tests/govern_resolve_task_in_project.rs`

## Found during the fix

Recorded by the implementor, 2026-10-07; not fixed in this story.

- Inside a project, `--root` given without `--task` is ignored: `aep govern resolve --root <empty
  dir>` exits 0 and reads the project (probed against the fixed build). The CLI reference says each
  flag defaults to the project's value, which implies the flag wins when given.
- With `--task` given inside a project, a broken project task file or artifact manifest still
  fails the project load, though neither is used. Tolerating it needs a separate loader in
  `aep-project`.
- `aep drive` (`crates/edge/aep-cli/src/drive.rs:441`) already discovered the project when
  `--task` was given; only `govern resolve`, `evaluate` and `explain` went through `inputs`.
