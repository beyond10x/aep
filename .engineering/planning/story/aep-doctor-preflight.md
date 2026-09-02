---
format: aep.planning-md/1
id: story:aep-doctor-preflight
kind: story
status: active
title: aep doctor says whether this checkout is ready to be planned in
relations:
- decomposes: epic:adopter-feedback-round-2
- serves: vision:O3
revision: 3
---
# Story: `aep doctor` says whether this checkout is ready to be planned in

## Outcome

An adopter runs one command and reads, line by line, whether the binary, the project file, the
protocol source, the store and the plugin directories are in a state the other verbs will accept —
before they type a prompt and wonder whether they used the tools properly.

## Context

The second adopter's report ended with "I am still unsure if I even used them properly". Nothing in
the CLI answers that; `artifact validate` checks the store, `reverse init` checks the source, and the
binary's version is only visible through `--version`. The third-party plugin has a `/setup` skill
that detects, plans and confirms. Derived from the epic this decomposes.

## Acceptance

- `aep doctor [--root <path>] [--plugin-dir <path>]... [--format text|json]` exists in
  `protocol-cli` (clap derive) with `protocol doctor` as an exact alias.
- It reports, one line each with `ok`/`warn`/`fail`: the binary version; whether
  `.engineering/project.yaml` exists and parses; whether its `protocols` source resolves (a path that
  exists, or a `git+…#<40-hex>` locator — resolution is offline: it does not fetch); whether the
  planning store exists and `validate` would pass (reusing the same code, not shelling out); for each
  plugin directory given or found through `AEP_DRIVE_PLUGIN_DIR`, whether it carries a manifest; and
  whether the newest reachable release tag agrees with the binary version when the root is a Git
  checkout.
- Exit status is 1 on any `fail`, 0 otherwise; output is deterministic for one tree; it reads no
  clock and opens no network connection.
- Tests name the failing condition per line; `website/docs/reference/cli.md` gains the verb;
  `CHANGELOG.md` gains an Unreleased line; `task check` passes.

## Out of Scope

Installing or fixing anything; checking a harness's plugin cache.

## Open Questions

None.
