---
format: aep.planning-md/1
id: story:profile-and-cli-crates-named-after-aep
kind: story
status: implemented
title: The profile crates and the CLI crate are named after AEP
summary: Rename adp-domain to aep-profile-development, aop-domain to aep-profile-operations and protocol-cli to aep-cli; binaries and YAML protocol ids unchanged.
relations:
- decomposes: epic:area-layout
- serves: vision:O2
- depends_on: story:crates-under-area-directories
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/edge/aep-cli
- confidence: cited
  path: crates/profile/aep-profile-development
- confidence: cited
  path: crates/profile/aep-profile-operations
- confidence: cited
  path: website/docs/concepts/overview.md
- confidence: cited
  path: xtask/src/main.rs
revision: 6
---
# Story: The profile crates and the CLI crate are named after AEP

## Context

`adp-domain` and `aop-domain` are profiles of `aep/1` — 2,037 and 2,238 lines of vocabulary, no
engine, no store — yet each carries a three-letter acronym that reads as a sibling product.
`protocol-cli` builds the `aep` binary and carries the retired command's name. No repository outside
this one depends on any of the three (grep over every sibling `Cargo.toml`). Renames:

| old | new |
|---|---|
| `adp-domain` | `aep-profile-development` |
| `aop-domain` | `aep-profile-operations` |
| `protocol-cli` | `aep-cli` |

The YAML protocol ids `adp/1`, `aop/1` and the workflow id `adp/default` are wire ids and stay.
The binaries `aep` and `protocol` stay; `command_equivalence.rs` (invariant 10) must still pass.

## Acceptance

`cargo metadata` lists `aep-profile-development`, `aep-profile-operations` and `aep-cli` and none of
the old names; `task check` exits 0; `aep --version` and `protocol --version` print the same version
from the renamed crate.

## Notes

Depends on story:crates-under-area-directories landing first (the directories the renamed crates
live in). Atlas records components `aep/adp-domain`, `aep/aop-domain`, `aep/protocol-cli`; the
catalog update is the coordinator's step after this lands.
