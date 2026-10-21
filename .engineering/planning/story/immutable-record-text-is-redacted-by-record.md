---
format: aep.planning-md/3
id: story:immutable-record-text-is-redacted-by-record
kind: story
status: implemented
title: An immutable record's text changes only by a recorded redaction
summary: aep plan artifact redact replaces a literal in an artifact's title and body and records its digest; validate admits a review-result that differs from its first version only by recorded redactions.
owner: store
tags:
- review
- store
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: inferred
  path: crates/edge/aep-cli/tests/planning_cli.rs
- confidence: inferred
  path: crates/govern/aep-domain/src/evidence.rs
- confidence: inferred
  path: ess/domains/evidence.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:53:03Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-21T00:53:03Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-10-21T06:35:18Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: An immutable record's text changes only by a recorded redaction

## Outcome

A literal that must leave the current tree, such as a downstream project name, can be removed from a
`review-result` without breaking `aep plan artifact validate`. The removal is a write through the CLI
that leaves a record of itself, and `validate` checks the record instead of refusing the change.

## Context

`validate` compares a `review-result`'s title and body byte for byte with its first committed
Git-native version (`crates/edge/aep-cli/src/planning.rs:6305-6345`, `fn review_problems`, at
`33e0548ee`). A hand redaction therefore fails every CI that validates the store; two consumer
repositories run that validator in CI. The operator asked on 2026-10-21 that a downstream project name
leave the current tree of every repository, review records included. History is not rewritten: the
literal stays in Git history and leaves the current tree.

## Acceptance

1. `aep plan artifact redact <id> --literal <text> --with <replacement> --reason <text>` replaces every
   case-insensitive occurrence of the literal in the artifact's title and body, and writes one evidence
   file for the redaction holding the SHA-256 of the literal (never the literal), the replacement, the
   number of occurrences replaced, the reason and the instant. It changes one artifact file plus that
   one evidence file.
2. `redact` refuses, writing nothing, when the literal occurs nowhere in the title or body, when the
   literal is empty, and when the replacement itself contains the literal.
3. `validate` admits a `review-result` whose title and body differ from the first committed version only
   by recorded redactions: applying every redaction recorded for that artifact, in recorded order, to
   the first version, each literal matched by its digest, gives the current title and body byte for
   byte. Any other difference is refused with today's message.
4. Tests, in `crates/edge/aep-cli/tests/`: a redacted review-result validates; a redaction plus any other
   edit to the title or body is refused; a redaction record whose digest matches no text of the first
   version (or whose count differs) is refused; after `redact`, the literal appears in no file under
   `.engineering/` in any letter case.
5. `website/docs/reference/cli.md` documents the verb (`docs-check`), `CHANGELOG.md` gains an
   Unreleased entry, and `AGENTS.md` § Planning store names `redact` as the only way to change an
   immutable record's text.

## Out of scope

- Rewriting Git history. The literal stays in history.
- The planning skill's text, which lives in the `agentplugins` repository and changes there.
