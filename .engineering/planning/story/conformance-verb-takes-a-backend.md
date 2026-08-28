---
format: aep.planning-md/1
id: story:conformance-verb-takes-a-backend
kind: story
status: implemented
title: '`protocol conformance` runs against a backend the caller names'
summary: The verb is hard-coded to the in-memory reference backend, so a store that passes the suites in a Rust test cannot be shown to pass them from the command line.
relations:
- decomposes: epic:planning-store-as-backend
- depends_on: story:one-adapter-over-any-store
revision: 6
---
# Story: `protocol conformance` runs against a backend the caller names

## Outcome

Somebody asking *does this store implement the contract?* gets the answer from the command line,
against the store they actually have.

## Context

Opened by a review on 2026-08-26, which found `story:journal-backed-store` ticking the acceptance
line *"`protocol conformance` runs all sixteen suites against the markdown store and passes"* while
`crates/protocol-cli/src/main.rs` read `let backend = MemoryBackend::new();` — hard-coded, no
flag, and the subcommand's own help said *"Runs against the in-memory reference backend."*

The suites **did** run against the markdown store and the SQLite one, in Rust tests, each with a
faulty-backend guard. What was missing was the verb, and the line that claimed it was corrected
rather than quietly dropped.

Wave F, story 5 (`docs/plan/store-waves-f-g-h.md`). After F2, because the SQLite backend the verb
opens is the adapter over `SqliteStore`.

## Acceptance

`protocol conformance --backend <memory|markdown|sqlite>` runs the suites against the named backend;
`--store` says where a durable one lives; the default stays `memory` so no existing invocation
changes meaning; and the help text says which backend answered, because a report that does not name
what it ran against is a report somebody will attribute to the wrong thing.

**Done, 2026-08-28.**

- `--backend` is a `clap` `ValueEnum` (`ConformanceBackend`), so `--backend postgres` is a usage
  error (exit 2) naming the three that exist.
- `--store <path>`: a file for `sqlite`, a directory for `markdown`. Without it `sqlite` gets an
  in-memory database and `markdown` a scratch directory under the temporary directory — **the suites
  write**, and the help says so, and says that pointing `--store` at a plan you keep appends the
  suites' commands to its journal. `--backend memory --store …` is refused (exit 1, *"would have no
  effect"*) rather than ignored.
- The markdown backend is opened with permissive ladders and no workspace members, the same choice
  `aep-backend-markdown`'s own conformance test makes: a real ladder would refuse moves the suites
  are entitled to make.
- **The report names what it ran against**, not only the help: `ConformanceReport` gained
  `ran_against: Option<String>` (absent when nobody said, never guessed), printed as the first line
  `ran against: sqlite (in-memory database)` in text and as a field in JSON and YAML.
- Tests: `conformance_runs_against_the_backend_the_caller_names_and_the_report_says_which` (sqlite,
  markdown at a scratch `--store`, the unchanged default, and the JSON field),
  `conformance_refuses_a_store_for_the_backend_that_keeps_nothing`; the four pre-existing
  conformance tests unchanged and green; `cargo xtask guards` 0 duplicated.

`website/docs/status/limitations.md`'s *"`protocol conformance` runs only against the in-memory
backend"* is corrected in the same change.

## Out of Scope

Running the suites against a backend outside this workspace. That needs a plugin surface, and
`aep-conformance` is already a public crate anybody can call from their own tests.

## Open Questions

None outstanding.
