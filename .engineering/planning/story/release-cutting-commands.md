---
format: aep.planning-md/3
id: story:release-cutting-commands
kind: story
status: draft
title: The release steps are commands
relations:
- decomposes: epic:deterministic-delivery-workflows
revision: 2
---
# Story: The release steps are commands

## Outcome

`cargo xtask release` gains subcommands for the steps of this repository's release in `AGENTS.md`
(Releases and commits) that an agent performs by hand today. Each subcommand is idempotent and
refuses with a named reason when its precondition does not hold.

| step | subcommand | today |
|---|---|---|
| set the version | `prepare <version> --date <date>` | edit `Cargo.toml` and `Cargo.lock` |
| rename the changelog heading | `prepare <version> --date <date>` | edit `CHANGELOG.md` |
| create the tag | `tag` | `b10x-gates bot -- tag -a` |
| fold the status output and move the tag | `pin` | `cargo xtask status`, then a commit and a re-tag |
| record the release in the planning store | `record` | `aep plan artifact evidence` |

`record` finds the `Release` workflow run for the tag itself, by a read-only forge query. It needs
no URL as input, because no URL exists when a driven run starts.

## Context

`cargo xtask release` only checks a release that was already cut (`xtask/src/main.rs:70-73`).

## Acceptance

`prepare`, over a fixture checkout:

- `prepare 9.9.9 --date 2026-01-02` sets the workspace version.
- It turns `## [Unreleased]` into `## [9.9.9] — 2026-01-02`.
- A second run changes nothing.
- It refuses a version that is not above the newest tag, naming both.

`tag`, over a fixture checkout:

- It creates the annotated tag `9.9.9` at HEAD with the message `aep 9.9.9`.
- A second run at the same HEAD changes nothing.
- It refuses when a tag `9.9.9` exists at another commit, naming that commit.

`pin`, over a fixture checkout:

- After `tag`, it commits the `cargo xtask status` output and moves the tag to that commit.
- A second run changes nothing.
- It refuses when the tree has changes other than the status output, naming the paths.

`record`, against a stubbed forge:

- It writes one `test_result` naming the tag's commit and the Release run's URL.
- It refuses when the tag is not on `origin/main`.
- It refuses when no Release run exists for the tag, naming the tag.

Over a real release:

- The existing 6/6 check (`task release-check`) passes on a release cut with these subcommands.

## Scope

Cited: `xtask/src/main.rs`.

Inferred: a new `xtask/src/release*.rs`.

## Out of Scope

Push, PR and merge. They stay the bot route's (`b10x-gates`), called from the step map.
