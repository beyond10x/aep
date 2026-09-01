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

| Layer | Crates | Responsibility |
|---|---|---|
| vocabulary | `aep-domain`, `adp-domain`, `aop-domain` | typed rules, tasks, evidence, predicates, and profile vocabulary |
| decision | `aep-engine` | resolution, evaluation, authorization, and transitions |
| storage contract | `aep-contract`, `aep-conformance` | provider-independent commands, queries, and black-box suites |
| providers | `aep-backend-*` | memory, markdown, SQLite, PostgreSQL, Entity Runtime, and hybrid edges |
| driving | `aep-driver-spec`, `aep-driver` | step maps and the reference workflow caller |
| observation | `trace-domain`, `trace-spec` | normalized transcript IR and typed expectations |
| shell | `protocol-cli` | canonical `aep` command and exact `protocol` alias |

The document tree is data. A new lifecycle, principle, profile, or workflow normally changes YAML,
not engine code.

## External boundaries

- Entity Runtime supplies the IO-free entity kernel and providers. AEP depends on one pinned Entity
  Runtime release; the reverse dependency does not exist.
- ESS is standalone and shares no modeling crate with AEP. Only `aep-ess-evidence` understands the
  standalone ESS report at the optional evidence boundary.
- Agent plugins live in the curated `beyond10x` marketplace. The driver and evaluation runner accept
  plugin directories from the operator and guess none.
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
