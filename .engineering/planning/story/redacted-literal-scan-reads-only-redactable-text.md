---
format: aep.planning-md/3
id: story:redacted-literal-scan-reads-only-redactable-text
kind: story
status: implemented
title: validate reports a redacted literal in transition actor ids, which redact never rewrites
relations:
- serves: vision:O2
scope:
- confidence: cited
  path: crates/edge/aep-cli/src/planning.rs
- confidence: cited
  path: crates/edge/aep-cli/tests/validate_packs.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T21:08:55Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T21:08:57Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-10T22:33:44Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Problem

The store-pack feature (commit 74d3c0400, check 7 of `pack_problems` in `crates/edge/aep-cli/src/planning.rs`) checks every copy of an artifact for a literal that one of its redaction records removed. The security fixes extended that check to loose copies. It searches the **whole file**: front matter included, so `transitions` actor ids are searched too.

`aep plan artifact redact` rewrites only the title, summary and body. It never rewrites transitions, and the redaction record itself carries the same actor id. So when the redacted literal is a person's name that also appears as `actor: "human:<name>"`, `validate` reports `pack_redacted_literal_exposed`. That is text `redact` cannot change and should not change. aep 0.79.0, which predates the check, says `valid` on the same store.

## Reproduction (observed 2026-11-10 on the ess store at 6c994162cb, by the ess session)

- `aep plan artifact validate` built from aep main ec48cb490: exit 1 with 6 `[pack_redacted_literal_exposed]` problems, all on loose approval-records that carry redaction records. One of them is `approval-record:d132-types-across-specifications-by-vendor`, evidence `20261028T105458Z-000-14687b80c9ba.json`.
- aep 0.79.0 on the same store: `valid`.
- In every case the literal's digest (case-folded) matches only a front-matter `transitions` actor id. It never matches the title, summary or body.

## Fix

- The exposed-literal scan covers the free text a person writes into an artifact: title, summary and body, which `redact` rewrites, and a review's `prose_only` reason, which `show` prints. Each is read from the copy as a parsed document.
- Identity and structure fields are not searched: transitions and their actor ids, owner, tags, relations, scope, refs. `redact` does not touch them, and a person's name there is an identity, not a disclosure.
- A copy that does not parse as a document is still searched whole, so nothing is skipped.
- `redact` does not yet rewrite `prose_only`. Until it does, a literal left there keeps the store invalid, as it did before this change; teaching `redact` that field changes what its records replay and is a separate story.
- Whether a pack should carry actor ids at all (pseudonymising at pack time) is a separate question, not a validity error.

## Acceptance

- A store whose redaction literal equals the name in a transition actor id (`human:<name>`) validates, loose and packed.
- A literal left in a title, summary, body or `prose_only` reason of any copy, loose or packed, still fails with `pack_redacted_literal_exposed`.
- On the ess store at 6c994162cb, `validate` built from the fixed main reports no `pack_redacted_literal_exposed`.
