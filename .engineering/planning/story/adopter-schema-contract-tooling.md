---
format: aep.planning-md/1
id: story:adopter-schema-contract-tooling
kind: story
status: active
title: One schema, validated and projected for adopters
summary: An adopter authors one local JSON Schema; protocol validates instances and deterministically projects TypeScript without a second handwritten contract.
owner: protocol
tags:
- adoption
- schema
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
revision: 5
---
# Story: one project schema registry, validated and projected for adopters

## Outcome

An adopter declares one local JSON Schema registry in `.engineering/project.yaml`, authors each
contract once as JSON Schema, validates instances by stable schema identity, and deterministically
projects structural TypeScript types without maintaining a second contract by hand.

## Context

The `agentic-principles` adopter created versioned evidence documents and a registry, then added
Python validation scripts and two handwritten TypeScript type copies around its JSON Schemas.
Changing one contract required knowing about three languages and several unrelated paths. That is
integration burden engineering-protocols should remove from adopters.

The registry belongs to project configuration for the same reason artifacts, tasks, principles, and
profiles do: a command should discover the project's inputs from one manifest. Its default is
`.engineering/schemas`, while `$id` inside each schema remains its globally stable identity.

The same run exposed a missing planning operation. The installed planning skill explicitly told
agents to patch artifact bodies directly because the CLI owned only frontmatter. The stronger store
invariant is that every mutation crosses one command surface; prose is not an exception.

## Acceptance

- `project.yaml` accepts a relative `schemas` registry path, defaulting to
  `.engineering/schemas`; all project-owned custom schemas live there.
- `protocol schema validate <instances>...` discovers that registry, matches each instance's
  `schema` value to one `$id`, accumulates failures, and reaches no network. `--schemas` is an
  explicit fixture/non-project override.
- `protocol schema typescript <schema-id> --root <name>` selects from the same registry by `$id` and
  emits deterministic TypeScript declarations; `--out` writes and `--check` fails on drift without
  writing.
- Existing `protocol schema` and `protocol schema workflow` behavior remains byte-compatible.
- Unsupported projection semantics are refused by name rather than erased. Validation-only
  constraints remain authoritative in JSON Schema and are named in the generated header.
- `protocol artifact body <id> --from <path|->` is the only supported body-mutation path. It
  preserves CLI-owned frontmatter and exact supplied body bytes, increments one revision on change,
  and leaves the file unchanged on identical input or refusal.
- Codex and Claude planning skills prohibit direct edits anywhere in the planning store and use the
  body command. A schema-contracts skill teaches registry discovery, validation, projection, and
  drift checking.
- `agentic-principles` moves its contracts to `.engineering/schemas`, declares the registry in its
  project file, deletes Python schema tooling and handwritten TypeScript contract copies, generates
  one website type module, and validates all research instances through `protocol`.
- Known-good instances pass; a wrong identity, unknown field, missing field, duplicate `$id`, remote
  reference, stale projection, absolute registry path, and direct store-write instruction each fail
  in a test naming the defect.

## Out of Scope

- Generating JSON Schema from adopter-owned Rust types. Adopters choosing Rust already have their own
  source-to-schema toolchain; this story begins at checked-in JSON Schema.
- TypeScript runtime validators or replacement of JSON Schema with TypeScript.
- Resolving remote schemas, downloading `$ref` targets, or publishing a registry service.
- Projecting every JSON Schema vocabulary keyword into TypeScript. Unsupported shape semantics are
  refused; validation refinements remain in the source schema.
- Interpreting or formatting planning markdown. The body command transports bytes only.

## Open Questions

None. The project selects the registry path, schemas select their own identity, JSON Schema is the
single authored source, generated TypeScript is a projection, and the CLI is the sole planning-store
writer.
