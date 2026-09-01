# AGENTS.md — AEP

The contract for changing this repository. Read it before changing anything.

Organization-wide repository, language, history, provenance, and coordinated-migration rules live
in `atlas/AGENTS.md`. A change to bytes another repository verifies is a coordinated migration with
an Atlas ADR.

## Serves

This repository advances these objectives from `atlas/ROADMAP.md`:

- **O2 — decisions as data, with evidence.** Artifact state, lifecycle, legal moves, rules,
  evidence, and completion are data a deterministic engine decides.
- **O3 — any harness, observed and compared.** AEP supplies the governor, trace vocabulary,
  reference driver, and evaluation substrate; harness-specific readers and paid runs stay outside
  this repository.

A change that advances neither objective is a question for the operator, not an inferred task.

## What this repository is

A Rust library collection, a typed document tree, and one command with two names:

- `aep-domain`, `aep-engine`, `aep-contract`, `aep-conformance`, and the `aep-backend-*` crates own
  the generic protocol and planning substrate.
- `adp-domain` and `aop-domain` add development and operations vocabulary over that substrate.
- `aep-driver-spec` and `aep-driver` own reference step maps and deterministic driving.
- `trace-domain` and `trace-spec` normalize and check recorded harness activity.
- `aep-schema` publishes the document schemas.
- `aep-ess-evidence` is an optional report adapter at the AEP boundary.
- `protocol-cli` publishes canonical `aep` and the exact `protocol` compatibility alias.

It is not an LLM orchestration framework, hosted database, CI system, deployment platform,
marketplace, system-modeling toolchain, or credential holder. The engine decides from caller-supplied
documents and evidence; named edge crates own IO.

## Normative documents

| Subject | Authority |
|---|---|
| protocol semantics | `docs/design/consolidated-design-v0.2.md` and accepted reconciliation |
| artifact completion evidence | accepted portions of `docs/design/story-completion-evidence-design-v0.1.md` |
| transcript checking | `docs/design/transcript-conformance-design-v0.1.md` |
| driver and harness boundary | `docs/design/harness-planning-and-driver-design-v0.1.md` |
| open work and acceptance | `.engineering/planning/` plus accepted pages under `docs/plan/` |
| delivered releases | `docs/status.md`, generated from reachable annotated tags |

A design is proposed until a plan or planning artifact accepts it. Historical prose does not
override a later accepted decision.

## Repository boundaries

### Entity Runtime

The dependency arrow points from AEP to `entity-runtime`. Its IO-free kernel and providers are
pinned as one release. No Entity Runtime manifest names an AEP crate. Changing that direction or
changing verified bytes is a coordinated migration.

`cargo xtask deps` refuses more than one Entity Runtime version or pin.

### ESS

ESS is a standalone sibling repository. It owns executable system descriptions, compilation,
generation, synthesis, infrastructure import and projection, and its own conformance report. ESS
has no AEP dependency.

Core AEP must not compile against ESS modeling types. The optional `aep-ess-evidence` crate may read
the closed standalone report format and convert it into AEP `ess_conformance` evidence. That adapter
must refuse unknown fields and contradictory totals.

### Agent plugins

Harness-specific skills, agents, and marketplace manifests live in the sibling `agentplugins`
repository. This repository carries no plugin source and no marketplace manifest.

`aep eval run --arm plugin` requires an explicit `--plugin-dir`. `aep drive run` accepts repeatable
`--plugin-dir` values and the `AEP_DRIVE_PLUGIN_DIR` fallback. Neither command guesses a path under
this checkout.

### Metaharness

`metaharness` is an external tool, never a crate dependency. Paid or vendor-backed evaluation runs
live there. This repository retains language-neutral trace fixtures, evaluation case definitions,
and the runner that ingests or orchestrates explicitly authorized runs.

Nothing spawns without `METAHARNESS_LIVE=1` and `--budget-usd`. An absent tool exits with the
documented tool-missing status. `--stream` ingestion spends nothing.

### Public-source provenance

Public repositories carry product source and public technical history only. Credential minting,
delivery credentials, private provenance policy, and history-audit machinery belong in private
Atlas infrastructure. Do not add token scripts, private paths, private identities, or a public list
of forbidden identities here.

## Invariants

Each invariant names what enforces it. A rule without a check is not an invariant.

1. **Rust types are the source of truth.** Generated schemas under `schemas/generated/` are written
   only by `cargo xtask schema`; `schema-check` detects changed and orphaned files.
2. **Parse, then validate.** Raw document types may deserialize; validated domain types are created
   only through validation. Unknown fields on closed formats are refused.
3. **Validation accumulates.** Independent defects are reported together with stable codes and
   paths. Tests assert variants or codes, never only `is_err()`.
4. **Decisions are deterministic.** Domain and engine code use ordered collections and caller-
   supplied time. No ambient clock, random source, filesystem, environment, or network belongs in
   deterministic cores.
5. **Unknown differs from false.** A missing observation cannot satisfy a predicate and is not
   rewritten as a contradiction.
6. **Capability decisions default to deny.** A denial cannot be granted back by a later layer.
   Resolution and explain tests cover conflicts and provenance.
7. **Refusals change nothing.** Failed commands and transitions do not partially mutate stores;
   backend conformance suites cover rollback and idempotency.
8. **Audit is append-only.** Actor, executor, correlation, causation, and idempotency metadata cross
   the command boundary. Archive and supersede are the lifecycle vocabulary; deletion is not.
9. **Planning status is decided as data.** `entity-core` evaluates validated lifecycle definitions;
   no generic status setter exists in AEP.
10. **Command aliases are exact.** For retained operations, `aep` and `protocol` have identical
    output bytes and exit status. `command_equivalence.rs` holds accepted and refused paths.
11. **The ESS adapter is optional and narrow.** No AEP core manifest depends on an ESS crate;
    dependency scans and adapter tests hold this boundary.
12. **Plugin authority is explicit.** No repository-local fallback chooses a plugin. Launch records
    preserve the operator-supplied directories.
13. **Public APIs are documented and unsafe is forbidden.** Workspace lints are raised to errors by
    Clippy and rustdoc gate steps; every member opts into workspace lints.
14. **The gate is offline except by an opted-in name.** No check calls a model or spends money.
    `postgres-check` reaches only `ENTITY_POSTGRES_URL` when it is set and prints that it skipped
    otherwise. Cargo and the website package manager may populate their caches on a cold machine.
15. **A guard is mutation-tested before it is trusted.** Break the guarded condition, observe the
    named failure, restore it, and run the passing test.

## Gate

```console
task check
```

Read the command's own exit status. Do not pipe the authoritative run through a command whose exit
status replaces it.

<!-- generated:gate-steps:begin — do not edit; run `cargo xtask status` -->
`task check` runs **16 steps**, in this order: `fmt-check`, `status-check`, `plan-check`, `audit-check`, `version-check`, `dep-check`, `guard-check`, `claim-check`, `clippy`, `test`, `docs-check`, `postgres-check`, `doc-check`, `schema-check`, `msrv`, `website`.
<!-- generated:gate-steps:end -->

The list above is generated from `Taskfile.yml`; do not edit it by hand. Prose states no count of
test suites or test cases. The gate output is the only place that count belongs.

`task check` is authoritative. A change under `website/` must also be exercised through the same
website task the gate calls. CI delegates to the Taskfile rather than restating its steps.

## Planning artifacts

The planning store is `.engineering/planning/`. Its local skill at
`.agents/skills/planning/SKILL.md` is the complete model and must be read before any store write.

Before the first planning-store write in a session, run:

```console
protocol artifact list
```

Rules:

1. Never edit a planning artifact or journal directly.
2. Create with `protocol artifact new`, relate with `relate`, write prose with `body`, and change
   status only with `move`.
3. A status move is a claim about project state. Propose it unless the operator requested that
   exact move.
4. A refusal is an answer. Relay the legal moves the command prints; do not route around it.
5. After a batch, run `protocol artifact validate` and relay its output verbatim.
6. An already-satisfied or invalid request still gets an artifact recording the finding when the
   operator asked for planning work.

Do not improvise machine-owned frontmatter if the command is absent.

## Change conventions

- Use `rg` and `rg --files` for discovery.
- Preserve unrelated work in dirty worktrees.
- Use `apply_patch` for source edits. Bulk mechanical rewrites and formatter output may use their
  dedicated tools.
- Anything executable added or replaced here is Rust unless Atlas records an accepted exception.
  Existing shell and Python checkers are legacy; touching their behavior triggers replacement,
  not extension.
- Rust CLIs use `clap` derive.
- Tests are named for behavior and assert the reason for failure.
- Comments explain why; public docs explain what a type is for.
- Prefer no new dependency. Explain every necessary dependency beside its manifest entry.
- `CHANGELOG.md` gains an Unreleased entry for every user-visible change.

The direct dependency policy is recorded in the workspace manifests and enforced by the lockfile,
Clippy, tests, and dependency guard. The AEP domain must not acquire IO dependencies.

## Worktrees and concurrency

Use a dedicated worktree for coordinated work. Before editing, inspect `git status`, active
worktrees, and overlapping changes. A shared Cargo target directory is acceptable for builds but
never for generated source. Run generators only from the worktree whose files they own.

When another agent is integrating the same repository, stop at a clean handoff boundary. Do not
merge, rebase, publish, or rewrite shared state without explicit authority.

## Releases and commits

Bare semantic-version tags are the organization convention. The full gate passes before the
changelog is cut and the annotated tag is created. Tag, workspace version, and dated changelog
heading must agree.

This public repository contains no delivery credential machinery. Commit and publication are
performed through the organization delivery boundary maintained by Atlas. Do not copy that
machinery here for convenience.

Conventional commit prefixes are `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, and `chore:`.
Commit messages have a title, a blank line, and a body explaining what changed and why. Ticket
references belong in a trailing `Refs:` line, never the title.
