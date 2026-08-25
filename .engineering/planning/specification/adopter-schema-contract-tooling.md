---
format: aep.planning-md/1
id: specification:adopter-schema-contract-tooling
kind: specification
status: approved
title: 'Local JSON contracts: validation and TypeScript projection'
summary: Required behavior for offline instance validation, stable schema identity, deterministic TypeScript projection, and drift checking from one adopter-owned JSON Schema.
owner: protocol
tags:
- adoption
- schema
relations:
- specifies: story:adopter-schema-contract-tooling
revision: 5
---
# Specification: project schema registries, validation, and TypeScript projection

`protocol schema` gains two adopter-facing operations around one authored JSON Schema registry:
offline instance validation and deterministic projection of structural TypeScript types. A project's
`.engineering/project.yaml` names the registry; each JSON Schema's `$id` supplies identity. JSON
Schema remains authoritative for runtime acceptance and generated TypeScript is a reviewable
projection. `protocol artifact` also gains the missing body mutation so every planning-store write
crosses one validated surface.

## Context

This specifies `story:adopter-schema-contract-tooling`, found while applying engineering-protocols to
`agentic-principles`. Existing compatibility forms remain:

```text
protocol schema
protocol schema workflow
```

They list and print engineering-protocols' Rust-generated schemas. Adopter-owned contracts are a
separate project registry discovered through `.engineering/project.yaml`; they do not become a
second protocol document vocabulary.

The existing planning plugin divided each markdown artifact into CLI-owned frontmatter and a
directly edited body. The ownership division is useful; the second writer is not. The CLI preserves
the division while becoming the only component that writes either half.

## Requirements

### Project schema registry

**R1.** `RawProjectConfig` accepts a relative `schemas` path. It defaults to `schemas`, resolves
against the project's engineering directory, and therefore defaults to `.engineering/schemas`.
Absolute values are refused by the same portability invariant as every other project-owned path.

**R2.** Every project-owned custom JSON Schema is placed in the configured registry directory and
is discovered recursively by files ending `.schema.json`. The configured path locates the registry;
it does not identify contracts. Every loaded schema declares a non-empty absolute `$id`, and `$id`
is the only schema identity used by validation or projection.

**R3.** `protocol schema validate <path>...` discovers the registry from the nearest project file.
`--schemas <directory>` is an explicit override for fixtures and non-project use. Each positional
path may be a JSON file or directory; directories are walked recursively for `.json` files while
`.schema.json` files are excluded from the instance set.

### Validation

**R4.** Every loaded schema must be a valid JSON Schema supported by the embedded `jsonschema`
engine. Duplicate `$id` values are refused before any instance is validated. No schema is selected
by filename.

**R5.** Every instance must be a JSON object with a non-empty string `schema` property. That value
selects the schema whose `$id` is byte-identical. A missing selector and an unknown identifier are
different named failures.

**R6.** Validation reports all instance failures it can observe, ordered by document and instance
location. It exits `0` only when every schema compiles and every instance validates, and `1` for
invalid contracts or instances. Text reports issues plus derived counts; JSON and YAML reports carry
stable issue codes. Read, parse, and I/O failures identify the responsible path.

**R7.** Validation resolves only schemas from the discovered or overridden registry and local
fragments inside them. The validator is built without HTTP or filesystem resolution features. An
unresolved or remote `$ref` is refused; validation never reaches a network.

### TypeScript projection

**R8.** `protocol schema typescript <schema-id> --root <TypeName>` discovers the same project
registry and selects exactly one schema by byte-identical `$id`. `--schemas` provides the same
non-project override. The command writes the projection to standard output; `--out <path>` writes
exactly those bytes; `--check --out <path>` compares without writing and exits `1` if the file is
absent or differs. `--check` without `--out` is refused.

**R9.** The projection supports the adopter's required structural vocabulary: objects,
`properties`, `required`, `additionalProperties: false`, arrays and `items`, string, integer,
number, boolean, null, `enum`, `const`, local `$ref`, and `$defs`. Property names that are not legal
TypeScript identifiers are quoted. Definitions are emitted in lexical name order.

**R10.** The projection is byte-deterministic: the same schema and root name produce the same UTF-8
bytes with one trailing newline, independent of map insertion order. It starts with a generated-file
notice naming the schema `$id` and stating that JSON Schema remains the runtime authority.

**R11.** Validation refinements with no TypeScript structural equivalent—including `format`,
`pattern`, lengths, numeric bounds, array uniqueness, and item counts—do not become a second
runtime contract. The projector may ignore only this enumerated refinement class and the generated
notice says runtime validation remains with JSON Schema.

**R12.** A keyword that changes structural shape and is unsupported—including external `$ref`,
conditional schemas, unevaluated properties, or unhandled composition—is refused with the JSON
pointer where it occurred. It is never projected to `unknown`, `any`, or an incomplete interface.

### Compatibility and adopter integration

**R13.** `protocol schema` continues listing generated engineering-protocols schemas in the same
order and bytes; `protocol schema <name>` continues printing one. `validate` and `typescript` become
reserved operation names. Tests pin both legacy forms.

**R14.** `agentic-principles` declares `schemas: schemas` in `.engineering/project.yaml`, keeps only
its authored `.schema.json` contracts under `.engineering/schemas`, and contains no Python schema
validator, Python schema dependency, or handwritten TypeScript representation of those contracts.
Its website imports a generated module produced from the registry schema.

**R15.** The adopter exposes repeatable commands for validation, projection generation, and
projection drift checking. A normal website build may consume committed generated TypeScript
without requiring Rust, while its repository gate runs drift checking with pinned
engineering-protocols tooling.

### Planning-store mutation and plugin guidance

**R16.** `protocol artifact body <id> --from <path|->` reads a complete UTF-8 body from a file or
standard input, loads the store cleanly, replaces only the selected document's body, and writes via
`MarkdownStore::update`. A changed body increments `revision` once. Identical bytes print `nothing
to do` and change neither file nor revision. A missing artifact, unreadable source, invalid store, or
failed write leaves the document unchanged.

**R17.** The Codex and Claude planning skills state that no planning-store file is edited directly.
Creation, relations, status, and body each name their CLI verb. Worked examples use
`protocol artifact body`; decomposer instructions and instruction-surface evaluations enforce the
same rule.

**R18.** The engineering-protocols plugins include a concise `schema-contracts` skill triggered by
project JSON Schema, contract validation, generated TypeScript, schema-registry configuration, or
schema drift work. It teaches project registry discovery, one authored source, validation by `$id`,
deterministic projections, committed output when consumers need Rust-free builds, and `--check`.

## Constraints

- No command introduced here performs network I/O.
- The implementation uses the existing `jsonschema` dependency with default features disabled; it
  does not add a second schema engine.
- The projector is a deterministic pure transformation over parsed JSON and reads no clock or random
  source.
- Generated TypeScript is a projection, never a runtime validator and never an input to schema
  generation.
- Planning bodies remain opaque bytes; the body command does not parse or reformat markdown.
- Public CLI and project-format changes receive an Unreleased changelog entry and reference updates.

## Out of Scope

- YAML-instance validation.
- Remote schema registries and URL dereferencing.
- TypeScript-to-schema generation.
- Code generation for languages other than TypeScript.
- Semantic graph checks between records. A domain runner may perform those after structural schema
  validation.
- A general-purpose planning document editor or markdown formatter.

## Invariants

1. One authored contract: JSON Schema in an adopter; Rust in engineering-protocols for its own
   generated schemas. No handwritten projection is authoritative.
2. Every project custom schema is reachable through the registry declared by `project.yaml`.
3. Every planning-store mutation crosses the `protocol artifact` command surface.
4. A projection or body-update failure leaves existing output untouched. `--check` never writes.
5. Schema identity comes from `$id`; instance selection comes from `schema`; paths are locations.
6. Unsupported semantics fail closed.
7. Validation and projection are offline and deterministic.

## Acceptance Criteria

- Domain tests cover the configured `schemas` path, its default, resolution, and refusal of absolute
  values.
- CLI integration tests cover project discovery, explicit override, validation success and
  accumulated failure, `$id` selection, projection generation, and non-writing drift checks.
- Projector unit tests cover every supported structural form, permuted property order, refinements,
  and planted unsupported structural keywords.
- Backend tests establish that body replacement preserves frontmatter, exact body bytes, and no-op
  revision behavior.
- Skill validation and instruction-surface checks establish the sole planning writer and project
  schema-registry workflow.
- The `agentic-principles` corpus validates through the new command and its committed generated type
  passes `--check`.
- The known-bad mutations named by the story each turn their responsible test red before acceptance.
- `task check` passes in engineering-protocols and the Docusaurus typecheck/build pass in the adopter.

## Open Questions

None. Project configuration owns registry discovery, `$id` owns contract identity, JSON Schema owns
runtime truth, generated code is a projection, and the CLI owns every planning-store mutation.
