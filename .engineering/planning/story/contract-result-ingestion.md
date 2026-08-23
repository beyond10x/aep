---
format: aep.planning-md/1
id: story:contract-result-ingestion
kind: story
status: implemented
title: An adapter's conformance crosses the seam as a contract_result
summary: The record metaharness prints about its own adapters becomes evidence the engine reads, with the discipline the contract-testing principle states enforced where the record enters.
owner: protocol
tags:
- evidence
- harness
relations:
- decomposes: epic:metaharness-migration
- depends_on: story:metaharness-executor
revision: 1
---
# Story: An adapter's conformance crosses the seam as a `contract_result`

## Outcome

Somebody holding the output of `metaharness conformance <kind> --contract` can put it in front of
`protocol evaluate` and have the `contract-testing` principle decide on it, without knowing anything
about metaharness's internals — which is what its adapter-contract design said the shared vocabulary
was for, and what nothing here had ever done.

## Context

metaharness (private) contract-tests each `metaharness ⇄ vendor` adapter and emits the outcome in
the `contract_result` shape this repository defines
(`docs/design/adapter-contract-v0.1.md` § *Reuse of engineering-protocols' tooling*, milestone CT-1,
read 2026-08-23). Its own words:

> An EP-driven eval, or any consumer, reads an adapter's conformance as a `contract_result` without
> knowing anything about metaharness's internals.

Until this story that sentence described an intention. `grep contract_result crates/protocol-cli/src`
returned nothing: the vocabulary crossed the boundary and no bytes ever had. It is the same failure
the frame document had before `crates/protocol-cli/tests/metaharness_frame_contract.rs`, and the
mirror image of it — that seam is *what this repository mints, read by their rules*, and this one is
*what they mint, read by ours*. No Cargo dependency crosses in either direction, because this
repository is public and that one is not.

## What the ingestion is, and what it deliberately is not

The record parses **directly**. `Evidence` is tagged by `kind`, `ContractResult` is the payload, and
`{"kind":"contract_result","checked":20,…}` deserialises into `Evidence::ContractResult` with no
adapter, no second type and no new evidence kind. `protocols/aep/1.yaml` has declared
`contract_result`, the `contract-runner` verifier and `contracts.**` since the base protocol was
written, so no document changed either. That is most of the work, and the shared vocabulary did it
before this story started.

What a record on standard output does not carry is the **envelope** an evidence document needs:
`observed_at` and `producer`. So there is a verb, `protocol contract evidence`
(`crates/protocol-cli/src/contract.rs`), and it supplies exactly those two fields. It computes no
verdict, touches no count and adds no fact. It follows every decision `protocol trace evidence`
already took — the producer is a constant so no caller can name itself the verifier, the document is
a list of one in both renderings the engine reads, and a record reporting failures is written down
rather than exited on.

One decision is **stricter** than the trace verb's, and it is the only one: `--observed-at` is
required. That verb runs its check in its own process and may honestly stamp its own clock; this one
is handed a record produced elsewhere, possibly last week, and the record carries no time of its own.
A default of *now* would be this binary claiming a freshness it did not witness, which is what
evidence horizons exist to catch.

## Acceptance

- The provider's captured bytes are the payload this repository defines, asserted on the committed
  fixtures rather than on a value built here. **Met**:
  `the_providers_own_bytes_are_the_payload_this_repository_defines`, over
  `crates/protocol-cli/fixtures/metaharness-contract-result-{claude,codex}.json` — 20 and 10 vectors,
  both green, captured 2026-08-23 from the live build. They hold two provider strings, one consumer
  string and four counts, and nothing account-level.
- The loop closes through the binary, for both adapters and both renderings: bytes → verb → document
  → `protocol evaluate --evidence`. **Met**: `both_captured_records_become_evidence_the_engine_reads`
  (`crates/protocol-cli/tests/metaharness_contract_result.rs`), which reads back
  `✓ evidence contract_result from contract-runner (independent)` and all three of the principle's
  predicates.
- `checked > 0` or the record asserts nothing, refused with the reason named. **Met**:
  `a_record_that_checked_nothing_is_refused_before_a_document_exists`, plus
  `a_record_that_checked_nothing_is_refused_with_the_reason_named` beside the rule. Both verified by
  mutation.
- `breaking_changes ≤ failed`, refused when violated. **Met**:
  `a_record_whose_breaking_changes_exceed_its_failures_is_refused_before_a_document_exists`, plus
  `a_record_claiming_more_breaking_changes_than_failures_is_refused` beside the rule — and
  `a_record_whose_failures_are_all_breaking_is_accepted` for the boundary, without which the rule
  could have been written `>=` and every other test would still pass.
- `breaking_changes` is the number a consumer gates on, shown end to end. **Met**:
  `a_breaking_change_is_the_number_the_evaluation_turns_on` evaluates three records that differ only
  in two counts — `0/0`, `failed: 1 / breaking: 0`, `failed: 1 / breaking: 1` — and the middle row is
  the control: rows two and three agree that `contracts.failed == 0` fails and disagree on exactly
  one line. That is the provider's own decision 2 (`failed` is *the contract is red*,
  `breaking_changes` is *and it is the vendor's fault*) asserted from this side of the seam.

## Why the two refusals are at the boundary and not left to the engine

`principles/development/contract-testing.yaml` already states the discipline as
`contracts.checked > 0`, so leaving a zero-checked record to the engine looks defensible. Measured,
it is not. A `contract_result` with `checked: 0` submitted against `examples/billing-conformance`
reads:

```text
✗ contracts.checked > 0                                     [principle contract-testing]
✓ contracts.failed == 0                                     [principle contract-testing]
✓ contracts.breaking_changes == 0                           [principle contract-testing]
✓ evidence contract_result from contract-runner (independent)
✓ contract-runner must run
```

Two predicates pass vacuously and — the part that matters — the **evidence obligation is
discharged**: the task now has its independent contract record and `contract-runner` has run, on a
run that checked nothing. So the record is refused where it enters.

## Out of Scope

- **Anything gating on the record.** No workflow, profile or step map requires a `contract_result`
  about a metaharness adapter; the verb makes the fact available and nothing yet asks for it. Stated
  rather than implied — it is the same remainder `trace_conformance` carries in
  `drivers/development/checks.yaml`.
- **Reading the record from a pipe.** `--record` takes a path, so the bytes the provenance digest
  names exist somewhere a later reader can go and check. Callers redirect.
- **Re-running the provider.** Nothing in `task check` reaches another repository, so the fixtures
  are a snapshot by construction. The metaharness-side wave pinning the same bytes as goldens is what
  makes a drift in the emitter fail over there rather than go unnoticed here.
- **Attestation.** The document is YAML by the time the engine reads it and a person can type one;
  the digest is over bytes this process was handed, not bytes it watched being produced. The same
  limit `trace-spec` and `ess-conformance` state about their own records.

## Open Questions

- Whether a driven run should mint this record itself — a `command` step running
  `protocol contract evidence` with `evidence.record:`, the way `drivers/development/checks.yaml`
  already mints `trace_conformance`. It would only make sense for a map driving metaharness's own
  work, which is a step map in the other repository. Decides: operator, if that map is ever written.
