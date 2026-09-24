---
format: aep.planning-md/2
id: story:migration-import-costs-one-capture-per-batch
kind: story
status: implemented
title: The migration import captures the destination once per batch, not once per subject
relations:
- serves: vision:O2
- informed_by: story:eventlog-store-hydrates-from-one-snapshot
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/edge/aep-cli/src/store_command.rs
- confidence: inferred
  path: crates/plan/aep-backend-eventlog/src/lib.rs
- confidence: cited
  path: crates/plan/aep-planning-migration/src/durable.rs
revision: 9
---
## Outcome

`aep plan store migrate apply` imports a store in time proportional to the store's size, not to its
square: one capture per batch of imported subjects, not one per subject. Measured on the migrated
real-store copies before use: minutes per store, and the cost per artifact does not grow with the
number already imported.

## Why

Measured on the real cutovers of 2026-09-21 (wave-validate-v2-20260920; receipts under
`waves/0005-aep-migration/<store>-activation-20260921/receipts/`, wall clock from
`r10-apply.hold.stdout` and `SUMMARY.txt`):

| store | artifacts | apply | seconds per artifact |
| --- | --- | --- | --- |
| Service SDK (M4c) | 33 | 2 min 20 s | 4.2 |
| Eventlog (M4a) | 75 | 7 min 33 s | 6.0 |
| Entity Runtime (M4b) | 104 | 15 min 04 s | 8.7 |
| ESS (M4d) | 448 | started 16:26:21; staged journal 18.6 MB of ~21.5 MB at 21:05, ~5.5 h projected | ~44 |

Cost per artifact grows with the number of artifacts already imported, so the total grows roughly
with the square of the store. The rehearsals (runs 1–3 on a 64-artifact copy) measured ~4 minutes
per apply and the coordinator recorded it as a flat cost; nobody projected it to the 448-, 352- and
314-artifact stores before running them. The decision "provider fix before any real cutover" was
applied to reads (units 3, 6, 7) and not to the import.

Mechanism, inferred from source and not yet profiled: the import appends each subject through the
recorded store's write path, and each append captures the destination authority in full before
writing (`entity-eventlog/src/adapter.rs:475-493` `capture` → `capture_tenant`; the destination is
the growing store), with every blob re-hashed per capture (story:file-capture-blob-digests-without-rehashing).
Unit 7 removed the same pattern from the read path at open (`EventlogPlanningStore` retains one
snapshot per command). The import loop in `aep-cli/src/store_command.rs` (phase 03 `imported`) is
where the batching belongs.

## Target (Timo, 2026-09-21 21:12)

`apply` on a store of the ESS size (448 artifacts, ~21.5 MB journal, blobs of the migrated copies)
completes in **under 10 seconds** wall clock on this workstation (i9-10900K, no SHA-NI), measured on
the rehearsal copy with the qualified binary. That is ~22 ms per artifact. The three costs that stand
between the current ~44 s per artifact and that number, each to be measured before it is removed:

1. one full capture of the destination per imported subject (N captures → 1 per import);
2. one transaction per subject, each re-hashing the committed prefix of the growing journal
   (`Journal::resume`, raw-bytes prefix hash; N × growing prefix ≈ N²/2 bytes) → one atomic append
   group per import, or the incremental digest of story:incremental-history-digest-for-a-resumed-handle
   (Eventlog store); if the group route needs a provider change, stop and report;
3. blob content hashed on every capture (story:file-capture-blob-digests-without-rehashing) → with 1
   capture per import this is paid once.

## Acceptance

- Measured on a copy of the ESS store (448 artifacts) with the qualified binary: `apply` wall clock
  under 10 s, three runs, each on a fresh copy; the number in the CHANGELOG. Connectors (352) and AEP
  (314) copies likewise, all under 10 s.
- Red first, a counting case through the same counting seam unit 7 added: importing N subjects into
  a fresh authority makes O(1) captures per batch, not N; fails at d10fcebc5 with N.
- Red first, a timing-free case: the number of captures during `apply` does not depend on the
  number of subjects already imported.
- Same receipts, same digests: the migration receipt, `verify`, history equality and the projection
  bytes on the rehearsal copy are identical to those produced by the unbatched import (run 3's
  SUMMARY values are the control).
- Measured before any further cutover: `apply` on a copy of the Connectors (352) and AEP (314)
  stores, wall time in the CHANGELOG entry, seconds per artifact flat across the run.
- 16-step gate green; lane counts otherwise unchanged; no provider or Entity Runtime change if the
  batching fits in AEP; if it needs one, stop and report with the reason.
- Recorded where the next operator meets it: an acceptance that says "before cutover" names the
  cutover's own cost at the real store's size, projected from the rehearsal copy's per-artifact
  cost, before the first real store is touched.
