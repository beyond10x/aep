---
format: aep.planning-md/1
id: story:conformance-verb-takes-a-backend
kind: story
status: draft
title: '`protocol conformance` runs against a backend the caller names'
summary: 'The verb is hard-coded to the in-memory reference backend, so a store that passes the suites in a Rust test cannot be shown to pass them from the command line.'
relations:
- decomposes: epic:planning-store-as-backend
revision: 1
---
# Story: `protocol conformance` runs against a backend the caller names

## Outcome

Somebody asking *does this store implement the contract?* gets the answer from the command line,
against the store they actually have.

## Context

Opened by a review on 2026-08-26, which found `story:journal-backed-store` ticking the acceptance
line *"`protocol conformance` runs all sixteen suites against the markdown store and passes"* while
`crates/protocol-cli/src/main.rs:901` reads `let backend = MemoryBackend::new();` — hard-coded, no
flag, and the subcommand's own help says *"Runs against the in-memory reference backend."*

The suites **do** run against the markdown store and the SQLite one, in
`crates/aep-backend-markdown/tests/conformance.rs` and `crates/aep-backend-sqlite/tests/conformance.rs`,
each with a faulty-backend guard. What is missing is the verb, and the line that claimed it has been
corrected rather than quietly dropped.

## Acceptance

`protocol conformance --backend <memory|markdown|sqlite>` runs the suites against the named backend;
`--store` says where a durable one lives; the default stays `memory` so no existing invocation
changes meaning; and the help text says which backend answered, because a report that does not name
what it ran against is a report somebody will attribute to the wrong thing.

## Out of Scope

Running the suites against a backend outside this workspace. That needs a plugin surface, and
`aep-conformance` is already a public crate anybody can call from their own tests.

## Open Questions

None outstanding.
