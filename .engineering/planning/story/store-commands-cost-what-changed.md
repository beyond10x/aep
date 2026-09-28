---
format: aep.planning-md/3
id: story:store-commands-cost-what-changed
kind: story
status: draft
title: A planning-store command costs what changed since the last verified head, not the whole store
revision: 2
---
## Problem

Every command against a tree planning store costs time proportional to the whole store, not to
what changed. Measured on 2026-09-27 against a copy of a downstream store (`aep.project/4`,
`eventlog-tree/1`, 199 planning artifacts, 2,924 migration subjects, 5,209 events, 9,906 provider
blobs, 4,063 content blobs), CPU user time without `nice`:

| command | 0.61.1 | peak RSS |
|---|---|---|
| `plan artifact list` | 4.3–7.5 s | 785 MB |
| `plan artifact validate` | 17.8–25 s | 2.2 GB |
| `plan artifact new task` | 13–18.5 s | 1.1 GB |
| `plan artifact new review-result --from --findings` | 24.8 s | 1.27 GB |

At a machine load of 36–45 on 20 cores these became 53 s for `list` and 142 s for `validate`.

Per command, measured with `strace -e openat` and `perf record --call-graph lbr`:

1. The complete store snapshot is rebuilt and every content reference resolved: a write opened
   32,913 provider files and 4,101 content blobs.
2. Legacy-boundary validation (`validated_legacy_boundary_snapshot`) re-decodes and re-digests
   every imported evidence blob on open and on every recorded command.
3. `authority_snapshot_identity` canonicalises and digests the whole authority on every write.
4. `FileProjectionPublisher::publish_current` rewrites all 212 projected markdown files; the bytes
   are unchanged.

The Git protocol snapshot was verified about three times per command; that part is fixed
separately (verified once per process).

## Outcome

A command's cost is proportional to the events recorded since the last verified point, plus the
artifacts it touches.

- The derived cache records the authority head it last verified in full (legacy boundaries,
  authority identity) and the digest of that verification. A later command verifies only the
  events after that head and the joins they touch; a head the cache does not know, or a changed
  earlier event, falls back to full verification.
- The projection writes only the markdown files whose rendered bytes change.
- `validate` keeps a flag that forces full verification.

## Acceptance

- A write on the store above costs under 2 s of CPU, and its CPU time does not grow with the
  number of imported evidence blobs (scenario: the same write on a store with 10× the evidence).
- A corrupted event before the cached head is refused by the next command (the cache is keyed by
  the verified bytes, not trusted by position).
- A write that changes one artifact rewrites one projected markdown file plus any that render its
  relations.

## Outcome, 2026-09-28

Delivered by another route: 0.62.0 replaced the event-log store with the Git-native
`aep.project/5` store (docs/design/git-native-planning-store-v0.1.md) instead of making the
event-log store incremental. Measured with a release build on this repository after migration:
`list` 0.02 s, `validate` 0.05 s, `new task` 0.05 s CPU, peak RSS 35 MB. On a migrated copy of a
downstream 210-artifact store: `list` 0.43 s, `validate` 0.44 s, write 0.56 s wall, 57 MB. The
stamp work in 0.63.1 and 0.64.0 removed most of the remaining protocol-snapshot cost. This story is
left in draft for the operator to archive.
