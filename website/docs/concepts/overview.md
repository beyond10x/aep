---
title: Architecture overview
sidebar_position: 1
description: AEP's deterministic core, provider edges, profiles, driver, trace checker, and external boundaries.
---

# Architecture overview

AEP is a library and a specification, not a service. The engine holds no credential and observes
nothing by itself. Inputs enter as validated documents and evidence; decisions leave as values.

```text
protocols + profiles + task + evidence
                    │
                    ▼
             deterministic engine
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
    decision    obligations   explanation
        │
        ▼
 named IO edges: backend, CLI, driver, harness
```

## Layers

Each layer is a directory under `crates/`, so the tree says which crate is which.

| Layer | Directory | Crates | Responsibility |
|---|---|---|---|
| vocabulary and decision | `crates/govern/` | `aep-domain`, `aep-engine` | typed rules, tasks, evidence and predicates; resolution, evaluation, authorization, and transitions |
| storage contract and providers | `crates/plan/` | `aep-contract`, `aep-conformance`, `aep-client`, `aep-backend-*` | provider-independent commands, queries and black-box suites; memory, markdown, SQLite, PostgreSQL, Entity Runtime, and hybrid edges |
| driving | `crates/drive/` | `aep-driver-spec`, `aep-driver`, `aep-render` | step maps, the reference workflow caller, and drawing a run |
| observation | `crates/observe/` | `trace-domain`, `trace-spec`, `aep-ess-evidence` | normalized transcript IR, typed expectations, and the optional ESS report adapter |
| profiles | `crates/profile/` | `aep-profile-development`, `aep-profile-operations` | development and operations vocabulary over the substrate |
| shell | `crates/edge/` | `aep-schema`, `aep-project`, `aep-cli` | published document schemas, the filesystem and Git acquisition edge, and the canonical `aep` command with its exact `protocol` alias |

A crate depends on its own directory and the ones under it — `edge` → `{profile, drive, observe}` →
`{govern, plan}` → `aep-domain`. The repository's `AGENTS.md` records the one compiled exception.

The document tree is data. A new lifecycle, principle, profile, or workflow normally changes YAML,
not engine code.

## External boundaries

- Entity Runtime supplies the IO-free entity kernel and providers. AEP depends on one pinned Entity
  Runtime release; the reverse dependency does not exist.
- ESS is standalone and shares no modeling crate with AEP. Only `aep-ess-evidence` understands the
  standalone ESS report at the optional evidence boundary.
- Agent plugins live in the sibling repository `beyond10x/agentplugins`, which publishes the curated
  `beyond10x` marketplace; this repository carries no plugin and no marketplace manifest. The driver
  and evaluation runner accept plugin directories from the operator and guess none.
- Metaharness owns vendor-specific transcript readers and paid execution. AEP owns the neutral trace
  vocabulary and deterministic checker.

## Properties

- Same validated state, evidence, and injected time produce the same decision and bytes.
- Raw documents deserialize; validated domain values are constructed only after semantic checks.
- Independent validation defects accumulate with stable codes and paths.
- Unknown evidence differs from false evidence.
- Capabilities default to deny.
- Refusals leave stores unchanged and remain auditable.
- Generated AEP schemas are committed and checked for changed or orphaned files.

See [Design principles](./design-principles.md) for the behavioral consequences and [CLI
reference](../reference/cli.md) for the executable surface.
