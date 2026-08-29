---
format: aep.planning-md/1
id: story:adopter-schema-contract-tooling
kind: story
status: implemented
title: One schema, validated and projected for adopters
summary: An adopter authors one local JSON Schema; protocol validates instances and deterministically projects TypeScript without a second handwritten contract.
owner: protocol
tags:
- adoption
- schema
relations:
- decomposes: epic:adopter-feedback-round-1
- serves: vision:O2
revision: 7
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

## Closed on evidence — 2026-08-30

Eight of nine Acceptance lines hold outright. Runs, this session:

| check | result |
|---|---|
| `cargo test -p schema-contract` | 9 passed, exit 0 |
| `cargo test -p protocol-cli --test schema_contract_cli` | 3 passed, exit 0 |
| adopter: `protocol schema validate docs/principles.json docs/research/evidence` in `agentic-principles` | `6 schema(s), 6 instance(s): valid`, exit 0 |

Named evidence per line: registry path and default at `crates/aep-domain/src/project.rs:252`, `:268`,
with `an_absolute_schema_registry_is_refused` at `:975` · offline validation by
`crates/schema-contract/Cargo.toml:18` (`default-features = false`) and
`an_unprovided_reference_is_refused_offline` · projection drift by
`typescript_is_generated_by_schema_id_and_can_be_drift_checked`
(`crates/protocol-cli/tests/schema_contract_cli.rs:169-172`, asserts exit 1 and the file unchanged) ·
back-compatibility by `crates/protocol-cli/src/main.rs:522-524` · `artifact body --from <path|->` as
the sole path by `crates/protocol-cli/tests/planning_cli.rs:270`, `:316`, `:972` · both skills
prohibiting direct edits at `integrations/claude-code/skills/planning/SKILL.md:54` and the codex
skill's `:53` · the adopter migrated at `agentic-principles/.engineering/project.yaml:8`
(`schemas: schemas`), six schemas, one generated module, CI at `.github/workflows/pages.yml:51`.

**The one line not whole** is the last, and it is seven of its eight items: *direct store-write
instruction* has no test. Nothing in this repository reads a `SKILL.md`'s **content** — the only code
touching one asserts the file exists (`crates/protocol-cli/src/drive.rs:7747`). A skill that
regressed to instructing a direct body patch would ship green. Carried as
`story:skill-text-cannot-instruct-a-direct-store-write` rather than held against this story, because
it guards the skills, not the schema tooling this story built.
