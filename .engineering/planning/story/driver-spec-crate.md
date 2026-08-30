---
format: aep.planning-md/1
id: story:driver-spec-crate
kind: story
status: active
title: 'aep-driver-spec: the step map, validated before anything runs'
summary: A leaf crate over aep-domain holding RawStepMap, StepMap, PinnedWorkflowRef, the cursor types, ToolConfig and both cross-validation phases.
owner: driver
tags:
- driver
relations:
- decomposes: epic:reference-driver
- serves: vision:O3
- depends_on: story:workflow-id-pattern-numeric-tail
revision: 5
---
# Story: `aep-driver-spec` — the step map, validated before anything runs

## Outcome

An author writes a step map and finds out at load time that it is wrong, instead of finding out
mid-run when a model call has already been paid for and half a workflow has already happened.

## Context

The map is the document that says what happens in each state, and it pins the workflow it belongs to.
A workflow major bump must orphan a map pinned to the old one, loudly, at load. The crate is a leaf
on `aep-domain` only — the same shape `aep-backend-markdown` already has — because everything it
holds is a document type and a validation, and none of it touches the world.

## Acceptance

- A map whose `workflow` pin names a major the registry no longer has is refused at load, naming the
  pin and what is available.
- Validation runs in **two phases**: states and named verifiers at load; evidence kinds and the
  workflow pin at run start, because the protocol in force comes from the task, which no document
  loader has seen. `Verifier::ExternalTool` is exempt at load.
- `PinnedWorkflowRef` refuses a reference with no major version, and the schema it publishes makes
  the version group **required** — an editor cannot tell an author a map is fine that the loader will
  refuse.
- The manifest carries `[lints] workspace = true`, and the crate's row is added to invariant 9's list
  in `AGENTS.md` in the same change.

## Re-scoped on evidence — 2026-08-28

The crate exists and is in invariant 9's list (`AGENTS.md:288`); `cargo test -p aep-driver-spec` →
33 passed. Three acceptance lines are **written but not guarded**, and that is all that is left:

| line | state | what remains |
|---|---|---|
| an orphaned major pin is refused at load, naming the pin and what is available | code at `crates/aep-engine/src/registry.rs:375-398` | no test reaches the registry branch — the only test (`a_renamed_state_and_a_moved_major_accumulate_rather_than_short_circuit`, `map.rs:1523`) covers `cross_validate`. A test that loads a map pinned to a major the tree no longer has |
| two phases, `ExternalTool` exempt at load | both phases exist (`map.rs:793`, `map.rs:890`); the exemption is tested (`map.rs:1498`) | phase two has a **green path only** (`drive_cli.rs:545`). Nothing asserts `UndeclaredEvidenceKind` (`map.rs:904`) ever fires |
| `[lints] workspace = true`, and the invariant-9 row | both present (`Cargo.toml:34-35`, `AGENTS.md:288`) | nothing asserts the manifest line; it is caught only indirectly by the `clippy` gate step |
| `PinnedWorkflowRef` refuses an unversioned reference; the schema requires the group | **holds** | — |

Three red-path tests, no production code. The story stays `active` until they exist, because a
refusal nobody has watched fire is the shape this repository refuses to call done.

### Re-verified — 2026-08-30

Unchanged. `cargo test -p aep-driver-spec` → **35 passed**, exit 0 (33 on 2026-08-28); the two extra
tests are not these. Line numbers have drifted; the state has not.

- Orphan major pin: production code at `crates/aep-engine/src/registry.rs:377-398`. `registry.rs`
  still has **no `mod tests`** at all, so no test can reach the branch. The nearest assertion,
  `crates/aep-driver-spec/src/map.rs:1585`, calls `cross_validate` directly and never loads through
  the registry.
- `UndeclaredEvidenceKind`: appears once, at `crates/aep-driver-spec/src/map.rs:913`, in production
  code only. The sole `check_run` test asserts `refusals.is_empty()`
  (`crates/protocol-cli/tests/drive_cli.rs:921`) — a green path.
- `[lints] workspace = true` is present at `crates/aep-driver-spec/Cargo.toml:35`, and the
  invariant-9 row at `AGENTS.md:300`. Both are still unasserted; the clippy gate step catches the
  first only indirectly, as 2026-08-28 said.

Two red-path tests are what this story now owes.

## Out of Scope

Executing anything. This crate has no executor, no process spawn and no file write outside its own
tests.

## Open Questions

None blocking. Whether cursor types belong here or beside the run directory is settled here: they are
data the router reads, and the router is pure.
