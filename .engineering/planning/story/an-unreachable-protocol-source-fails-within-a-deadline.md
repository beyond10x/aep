---
format: aep.planning-md/3
id: story:an-unreachable-protocol-source-fails-within-a-deadline
kind: story
status: implemented
title: An unreachable protocol source fails within a deadline, naming the pin and the cache
tags:
- confinement
- protocol-source
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-project/src/project.rs
- confidence: cited
  path: crates/edge/aep-project/tests/fetch_deadline.rs
- confidence: cited
  path: crates/edge/aep-project/tests/fetch_deadline_adversary.rs
- confidence: cited
  path: crates/edge/aep-project/tests/protocol_cache_security.rs
- confidence: cited
  path: crates/edge/aep-project/tests/support/mod.rs
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/project-file.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T16:38:55Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-30T16:38:55Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-30T17:37:05Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
# Story: An unreachable protocol source fails within a deadline, naming the pin and the cache

## Outcome

A project whose `protocols:` names a `git+https` pin and whose cache holds no snapshot of that pin
gets a bounded, clear refusal when the network cannot reach the source: within a deadline, naming the
pin, the cache directory in use and `AEP_CACHE_DIR`. A cache that already holds the sealed snapshot
resolves with no network at all.

## Context

A downstream run in a Kubernetes pod with DNS but no egress called `aep plan artifact list --format
json` in a store pinned to `git+https://github.com/beyond10x/aep#<sha>`. With no snapshot in the cache,
`materialize_git_source` (`crates/edge/aep-project/src/project.rs:401-434`) runs `git fetch --depth 1`
through `git_bare` (`project.rs:601-631`), which has no deadline. The connect hung until the caller
killed aep at its own 120 s limit ("aep timed out after 120.0 s and was killed"), and the message named
neither the pin nor the cache. With no DNS (`bwrap --unshare-net`) the same call fails at once with
"fetching the pinned protocol revision: fatal: unable to access 'https://github.com/beyond10x/aep/':
Could not resolve host: github.com", again without naming `AEP_CACHE_DIR`. With `AEP_CACHE_DIR`
pointing at a populated cache, writable or read-only, the same run took four steps with no network
(the reporter's probes `p-nocache`, `p-cache`, `p-cachero`, aep 0.77.1).

Verified by the reporter's runtime probes; the absence of a deadline is read from the source above.

## Acceptance

1. The fetch of a pinned revision runs under a deadline: 60 s by default, `AEP_FETCH_TIMEOUT_SECS`
   (a positive integer) overrides it, and any other value is refused naming the variable. A test
   points a pin at a loopback HTTP listener that accepts and never answers, sets the deadline to a
   few seconds, and asserts that resolution fails within deadline plus a small margin.
2. The refusal on expiry and on any other fetch failure names the pin (`<url>#<revision>`), the cache
   directory in use, and says that `AEP_CACHE_DIR` pointed at a cache holding the sealed snapshot
   resolves with no network. Asserted on the message text.
3. On expiry, Git and the transport helper it started are gone when aep returns: the loopback
   listener of acceptance 1 reads end-of-stream on its accepted connection shortly after aep
   returns.
4. A refused fetch leaves no snapshot directory and no temporary checkout under the cache
   (refusals change nothing).
5. A cache that already holds the sealed snapshot resolves with the source URL pointing at the
   never-answering listener, and the listener records no connection.
6. `CHANGELOG.md` gains an Unreleased `### Fixed` entry; `website/docs/reference/cli.md` lists
   `AEP_FETCH_TIMEOUT_SECS`.

## Out of scope

- Pre-filling a cache for a deployment (an image layer or a `prefetch` verb): the consumer's.
- The consumer pushing a branch before its store call: the consumer's.
- Group-writable snapshot modes: `story:a-group-writable-snapshot-is-admitted-explicitly`.
