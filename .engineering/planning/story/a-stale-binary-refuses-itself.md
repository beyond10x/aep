---
format: aep.planning-md/1
id: story:a-stale-binary-refuses-itself
kind: story
status: draft
title: A stale `protocol` refuses the store it cannot read correctly
summary: Six sessions got a wrong validate/history answer from a protocol older than the store; 0.32.1 installed against 0.33.0 tagged today. The journal should carry the writer's version and an older binary refuse.
tags:
- cli
- store
relations:
- decomposes: epic:declared-configuration-invariants
- serves: vision:O2
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/aep-backend-markdown/src/journal.rs
- confidence: inferred
  path: crates/protocol-cli/src/planning.rs
revision: 6
---
# Story: A stale `protocol` refuses the store it cannot read correctly

## Outcome

A `protocol` older than the store it is pointed at says so and stops, instead of answering wrongly. The
journal records which build wrote each event, and the release procedure ends with the install that
makes the ambient binary the released one.

## Context

Six sessions, one failure shape (`SYNTHESIS.md` CL-1):

- `431986de#1566`: `protocol` 0.28.0 on `PATH` against a 0.31.0 store → "5 drifted stories" that had not drifted.
- `ed007513#2664-#2751`: 0.26.0 two releases after 0.27.3 shipped; `artifact validate` gave a false `undeclared_reference`.
- `9da4f51c#3300`: a build predating the journal — `history` reported nothing at revision 4, `validate` stayed green.
- `3130470e#97`: the stale install reported 3 live stories as deleted.
- `11727595#23267`, `9c286ad7#19`: 0.32.1 installed against a 0.33.0 tree; today `~/.cargo/bin/protocol`
  was built 00:14 and `0.33.0` tagged 00:30.

Landed so far: `--version` prints the workspace version (`ed007513#903`); H2 reads the store with the
tree's own build and refuses a mismatch (`8bb84a3`); adopters carry `scripts/check-protocol.py` with a
hard-coded floor that cannot see a fix in a later patch release (`ed007513#2748`).

## Landed 2026-08-30 (this story stays open for the rest)

- `task install` (`cargo install --path crates/protocol-cli --locked --force`) and `AGENTS.md`
  § Releases ends with it; `task release-check` follows.

## Acceptance

- Every journal event carries `protocol: <version>` of the writer (today `journal.jsonl` carries `actor` only).
- Every `artifact` verb compares its own version to the newest `protocol:` in the journal; an older
  binary refuses, naming both versions and `cargo install --path crates/protocol-cli --locked --force`.
  `--allow-older` overrides for one command and is journalled.
- A project whose `protocols:` pin is a Git source is compared against the pin's declared CLI version
  where the pinned tree states one.
- `AGENTS.md` § Releases ends with the install line, and `Taskfile.yml` has `task install`.

## Out of Scope

Auto-updating the binary; a network check against GitHub releases.
