---
format: aep.planning-md/1
id: epic:area-layout
kind: epic
status: implemented
title: Crates grouped by bounded context, profiles and CLI named after AEP
summary: Group the 22 crates into govern, plan, drive, observe, profile and edge; rename adp-domain, aop-domain and protocol-cli after AEP; keep crate names stable in the first step so no consumer re-pins.
relations:
- serves: vision:O2
revision: 4
---
# Epic: Crates grouped by bounded context, profiles and CLI named after AEP

## Outcome

`crates/` shows the five contexts the repository already has — govern, plan, drive, observe,
profile — plus an `edge/` directory for IO and document formats, and the two profile crates and the
CLI crate carry AEP's name rather than the acronyms ADP, AOP and the retired `protocol`. Crate
names inside the areas are unchanged in the first story, so no consumer re-pins; the renames are a
second story with zero external consumers (no sibling `Cargo.toml` names `adp-domain`, `aop-domain`
or `protocol-cli`).

## Why now

Three days after ADR 0017 split `engineering-protocols`, the operator read `aep` as "the planning
part" and `adp` as "the workflows". The cause is naming: the only AEP-branded plugin is
`aep-planning`, the execution plugin is branded `adp`, and 22 crates sit flat under `crates/` with
nothing saying which of them are the protocol, which are the store, and which are the driver.
Analysis: `~/.cache/beyond10x-notes/2026-09-03-aep-ess-structure.md`; plan: Atlas ADR to follow.

## Scope

Area directories under `crates/`; workspace member and dependency paths; the crate list in
`README.md`, `AGENTS.md` and `website/docs/concepts/overview.md`; the rename of `adp-domain`,
`aop-domain` and `protocol-cli`; the plugin-name references this repository holds for the sibling
`agentplugins` renames.

## Out of scope

Splitting `aep-domain` into protocol and planning halves (a coordinated migration, its own epic);
moving decision logic out of the CLI crate; moving the adopter-vendored data trees (`protocols/`,
`principles/`, `profiles/`, `workflows/`, `artifacts/`, `drivers/`, `conformance/`); changing the
YAML protocol ids `adp/1` and `aop/1`.
