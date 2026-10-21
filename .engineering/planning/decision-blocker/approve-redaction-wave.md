---
format: aep.planning-md/3
id: decision-blocker:approve-redaction-wave
kind: decision-blocker
status: cleared
title: The redaction wave waits for approval
relations:
- blocks: story:immutable-record-text-is-redacted-by-record
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-21T00:53:03Z", actor: "human:timo", revision: 2}
---
# Decision: approve the redaction wave

A wave of one: `story:immutable-record-text-is-redacted-by-record`, plus the move to ess 0.66.0.

- `ess/ess-inputs.yaml` requires ess 0.66.0, CI installs it with the x86_64 Linux line of the 0.66.0
  release's `SHA256SUMS`, and `xtask/tests/ess_gate.rs` pins it; `generated/ess/` is regenerated under
  it. First commit on the integration branch `wave/redact`.
- `story:immutable-record-text-is-redacted-by-record`: the `redact` verb, its evidence record, and the
  `validate` rule that replays recorded redactions over a review-result's first committed version.
  Unit branch `impl/immutable-record-text-is-redacted-by-record`, forked from the integration branch
  after the ESS commit.

Width 1: the request names one story and one sub-agent at a time. The other stories `aep plan artifact
waves` lists as ready were not asked for.

Package gate of `aep-cli` per unit; one adversary pass on the unit; the full gate once on the
integrated head; one pull request; a minor release, since the CLI gains a verb and the evidence store a
record kind.

Commits this approval covers: the opening store commit, the ESS pin commit, the unit commit, its merge
into `wave/redact`, the closing store commit, and the merge into `main` through a pull request. The
release follows its own procedure in `AGENTS.md` § Releases and commits.

## Decided

Approved by the directing session's brief of 2026-10-21, under the standing approval of directed
waves and releases in this repository.
