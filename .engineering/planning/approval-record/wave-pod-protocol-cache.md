---
format: aep.planning-md/3
id: approval-record:wave-pod-protocol-cache
kind: approval-record
status: draft
title: 'Approved: wave pod-protocol-cache (protocol source in a pod, ESS 0.77.0 pin)'
relations:
- decides: story:a-group-writable-snapshot-is-admitted-explicitly
- decides: story:an-unreachable-protocol-source-fails-within-a-deadline
- decides: story:evidence-admits-ess-0770-suites
- decides: story:a-transition-executor-reads-on-every-path
revision: 1
---
# Approval record: wave pod-protocol-cache

## Decision

The wave coordinator proposed one wave on the integration branch `wave/pod-protocol-cache`, base
`2b378ccaa`:

- Unit A, one implementor, two stories landing in `crates/edge/aep-project/src/project.rs`
  (`aep plan artifact waves` reports them colliding on that file, `CHANGELOG.md` and
  `website/docs/reference/cli.md`, all cited): `story:a-group-writable-snapshot-is-admitted-explicitly`,
  then `story:an-unreachable-protocol-source-fails-within-a-deadline`.
- Unit B: `story:evidence-admits-ess-0770-suites` (the ESS pin moves under the operator's rule that a
  repository touched anyway takes the newest ESS release in the same wave).
- `story:a-transition-executor-reads-on-every-path`: store-only, closed as rejected with no code change.

The orchestrating session approved it on the operator's grant, 2026-10-30:

> Approved as proposed (D284): units A and B, and R12 closed as rejected, store-only. The K2 design is
> accepted: group-write is admitted only with AEP_CACHE_GROUP_WRITABLE=1, and other-write is always
> refused. In the CHANGELOG and docs, name the Kubernetes fsGroup case as the reason for the variable,
> so pod users find it.

## Commits this approval authorises

The opening store commit, the unit commits, the merges of both units into the integration branch, the
coordinator's documentation and changelog commit, the review records and the closing store commit, one
pull request to `main` through the bot, and the release through this repository's own process.
