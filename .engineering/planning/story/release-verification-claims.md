---
format: aep.planning-md/3
id: story:release-verification-claims
kind: story
status: draft
title: A release run records its four verification claims from commands
relations:
- decomposes: epic:deterministic-delivery-workflows
- depends_on: story:release-step-map
- depends_on: story:release-cutting-commands
revision: 2
---
# Story: A release run records its four verification claims from commands

## Outcome

`cargo xtask release check <claim>` verifies one condition of a release and, when it holds,
records it through `aep plan artifact evidence epic:planning-on-entity-runtime --kind verification`.
The four claims are the ones `workflows/releases/source.yaml` guards on:

| claim | holds when |
|---|---|
| `release-on-default-branch` | the release branch was cut from the current `origin/main` |
| `release-inputs` | `--version` is a bare semantic version above the newest tag, and `--date` is a UTC date |
| `release-tag-pushed` | the annotated tag `<version>` exists on `origin` and points at the branch head |
| `source-release` | the tag's `Release` run succeeded, the GitHub Release carries the four archives and `SHA256SUMS`, and `cargo xtask release` prints 6/6 `ok` |

A check that does not hold records nothing, prints the condition that failed, and exits 1. The
evidence boundary is unchanged: a command step still does not mint `verification`
(`EvidenceMapping::MINTABLE`); the claim is written by this named command through the existing
evidence command. The release step map's "claim writer" steps run it.

## Context

`story:release-step-map` drives a release whose claims a fixture stub writes. Without a writer, a
real release cut by `aep drive run` cannot pass the guarded transitions, and the epic's first Done
When bullet cannot be met.

## Acceptance

- For each claim, a fixture where the condition holds: `cargo xtask release check <claim>` exits 0
  and writes exactly one evidence file of kind `verification` naming the claim; a second run writes
  a second record and changes nothing else.
- For each claim, a fixture where the condition fails: exit 1, the printed reason names the
  failing condition, and no file under `.engineering/evidence/` is written.
- An unknown claim is refused by clap with the four valid names, writing nothing.
- The release step map's claim-writer steps name this command, and the fixture from
  `story:release-step-map` passes with its stub replaced by this command over stubbed Git and
  forge reads.

## Scope

- `xtask/src/release.rs` (or a new `xtask/src/release/check.rs`) and its tests under `xtask/tests/`
- `drivers/releases/` (the claim-writer steps' `run`)
- `AGENTS.md` (Releases and commits), `CHANGELOG.md` (Unreleased)

Build tooling, outside the ESS specification (as `story:release-cutting-commands`).

## Depends on

- `story:release-step-map`
- `story:release-cutting-commands`
