---
format: aep.planning-md/3
id: approval-record:checks-map-command-surface
kind: approval-record
status: draft
title: 'Approved: the development/checks map declares a command surface per phase'
summary: 'Orchestrator decision 2481b483 (2026-10-23 14:01Z): specify and decompose admit only aep plan artifact, implement admits the repository''s task and cargo commands, no general shell.'
owner: project
tags:
- driver
relations:
- decides: story:checks-map-per-phase-command-surface
revision: 2
---
# Approval record: the development/checks map declares a command surface per phase

## Decision

Orchestrator decision 2481b483, taken 2026-10-23 14:01Z from the ess-ui-templates and metaharness
reports, relayed to this session by `b10x-chef-orchestrator` in these words:

> file and implement the AEP development/checks map decided 2026-10-23 14:01Z (orchestrator decision
> 2481b483, from ess-ui-templates and metaharness reports): in governed runs, the specify and
> decompose phases admit only the aep CLI (plan artifact commands); the implement phase admits the
> repository's task and cargo commands; no general shell. Record the decision in your store as an
> approval-record first, then story, wave, release through your own process. Tell ess-ui-templates
> and metaharness when it is released. Approved under the grant in […].

The grant named is the workspace operator's standing approval for class-C items relayed by the
orchestrator. Decider: operator.

## What this record authorizes

- The story it decides, as one wave of one unit, and the source release that ships it, each through
  this repository's own process (`AGENTS.md` § *Releases and commits*).
- Telling the ess-ui-templates and metaharness sessions once the release is verified.

## Decisions taken inside the item

- D1 The map declares the surface. Each `llm` step of `drivers/development/checks.yaml` carries a
  `commands:` list of argv prefixes, and `driven_surface` in `aep-cli` admits one simple invocation
  whose leading words match a declared prefix. A step that declares no `commands:` keeps the
  surface it has today (`aep plan artifact …`, `aep observe trace …`, the read-only programs) and
  the map digest it has today, so `drivers/development/default.yaml` and every map outside this
  repository drive and resume as before.
- D2 `receive`, `specify`, `decompose`, `establish_verifiers` and `adversarial_verify` declare
  `[aep, plan, artifact]` only. `implement` declares the gate and the generators the Taskfile
  names (`task check`, `task fmt`, `task schema`, `task ess`, `task status`) and the `cargo` verbs
  that act inside the working tree (`build`, `check`, `clippy`, `doc`, `fmt`, `test`). This is
  narrower than the decision's words "the repository's task and cargo commands", because four of
  those reach past the tree or around the surface and the decision's "no general shell" governs:
  `task install` overwrites the operator's `aep`; `cargo run -p aep-cli -- plan artifact …` is the
  store's write path through the build; `cargo xtask release tag` writes through the bot; `cargo
  install` and `cargo publish` reach the home directory or a registry. Found by the security review
  of the first cut, 2026-10-23.
- D3 The loader refuses a declared program that exists to run another program or a string: the
  shells and `eval`, `exec`, `source`, `.`, `command`, `builtin`; the wrappers (`env`, `xargs`,
  `sudo`, `doas`, `su`, `timeout`, `nohup`, `nice`, `setsid`, `find`, …); the script interpreters
  (`perl`, `python3`, `node`, `ruby`, `awk`, …); and `npx` (`CommandPrefix::PROGRAM_RUNNERS`). It
  also refuses a program written as a path, because the driver compares an invocation's program by
  its leaf name. The list is the runners the loader knows, not a proof that no declared program can
  run another; the docs say so.
- D4 `driven_surface` refuses, under every surface, a flag that makes a named program run an
  unnamed one: `cargo --config` (a `runner=` runs a string as the test runner) and `rg --pre`
  (`RUNNER_FLAGS`). Found by the adversary and security reviews of the first cut.
- D5 The map a run is held to is the one its cursor's digest pins, so `aep drive resume` refuses a
  changed map. Every `llm` step's `scope:` additionally denies writes under `drivers/**` (a run that
  edited its declaration could not resume under it) and to `Taskfile.yml`, `.cargo/**` and
  `rust-toolchain*`, the files that decide what the declared `task` and `cargo` run.
- D6 What the admitted programs do with the tree is outside the surface: `cargo test` runs the
  tree's tests and `cargo build` its build scripts, whoever wrote them. The declaration bounds which
  programs the session invokes; a sandbox for the build itself is the harness's, and the map's
  header and the reference page state the limit rather than claim a guarantee.
- D7 The native arm (`harness: b10x`) renders the programs it publishes from the same decision in
  metaharness; carrying the per-step declaration there is metaharness's change and is named to that
  session in the release message.
