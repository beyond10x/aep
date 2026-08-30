---
name: schema-contracts
description: Manage project-owned JSON Schema contracts through engineering-protocols — configure the registry, validate JSON research or product records, generate TypeScript projections, and check schema/type drift. Use when a project has `.engineering/schemas`, a `schemas` entry in `.engineering/project.yaml`, JSON files with a `schema` selector, generated TypeScript contract types, or requests involving JSON Schema validation, schema identity, contract generation, or schema drift.
---

**Skill version 0.2.0** — the version in `.claude-plugin/plugin.json`.

# Project schema contracts

## Keep one authored source

Author runtime contracts only as JSON Schema. Put project schemas under the registry named by
`.engineering/project.yaml`:

```yaml
schemas: schemas
```

The path is relative to `.engineering/` and defaults to `schemas`, so this resolves to
`.engineering/schemas`. The path locates the registry; each schema's absolute `$id` is its identity.
Instances select a contract with a byte-identical top-level `schema` value.

Do not add a project-local validator or hand-maintained TypeScript copy. Validation and projection
come from `protocol`; generated code is disposable output from the JSON Schema source.

## Validate before interpreting

From anywhere inside the project:

```console
$ protocol schema validate docs/research evidence
```

Directories are recursive. Use `--format json` for a machine report. Use `--schemas <dir>` only for
a fixture or non-project invocation; ordinary project work must use the registry declared in
`project.yaml`. Validation is offline and fails closed on duplicate identities, unknown selectors,
invalid instances, and unresolved references.

Structural validity does not replace domain checks. Run experiment, graph, or semantic checks only
after contract validation succeeds, and keep those checks out of the schema source.

## Generate consumer types

Select the source by `$id`, never by filename:

```console
$ protocol schema typescript urn:example:registry:1 \
    --root PrincipleRegistry \
    --out website/src/generated/principle-registry.ts
```

Commit a generated module when a consumer must build without Rust. Import types only from that
module and delete handwritten equivalents. Prove the committed projection is current in the gate:

```console
$ protocol schema typescript urn:example:registry:1 \
    --root PrincipleRegistry \
    --out website/src/generated/principle-registry.ts \
    --check
```

If projection refuses an unsupported structural keyword, change the consumer design or extend the
projector under review. Never replace the refusal with `any`, `unknown`, or a handwritten type.
