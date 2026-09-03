---
format: aep.planning-md/1
id: story:redact-covers-home-and-user
kind: story
status: draft
title: --redact covers the home directory and the user name
summary: Recorded streams carried /home/<user> and the user name 18–51 times each; redact them and digest after, so a stream can be committed to a public recorded/ directory.
owner: eval
tags:
- eval
relations:
- decomposes: epic:self-evaluation
- serves: vision:O3
revision: 1
---
# Story: `--redact` covers the home directory and the user name

## Outcome

A recorded stream can be committed to a public repository: no absolute home path and no local user name survive redaction, and the manifest's digest is taken after redaction so the file still ingests.

## Context

The eight P1 streams recorded on 2026-09-03 under `--redact` carried `/home/<user>` 18–49 times each and the user name 20–51 times, so none of them could go into the public `recorded/` directories the cases were written for; they stay on the operator's machine.

## Acceptance

- `--redact` replaces the operator's home directory with `~` (or `<home>`) and the user name with `<user>` in every event text and path field; the rule is documented beside the existing redaction rules.
- The transcript digest in the manifest is computed over the redacted bytes, so `aep eval run --stream` accepts the file.
- A test feeds a stream containing both and asserts neither survives; an ingestion test proves the redacted file replays.

## Out of Scope

Redacting repository names or commit ids; those are the run's subject.

## Ambiguities

- `inferable` — where redaction lives today: the `--redact` path in `crates/protocol-cli/src/eval.rs`.

## Open Questions

None.
