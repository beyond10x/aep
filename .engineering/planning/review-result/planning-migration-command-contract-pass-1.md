---
format: aep.planning-md/1
id: review-result:planning-migration-command-contract-pass-1
kind: review-result
status: active
title: 'Complete migration command contract examination: exact encodings and consumer composition'
relations:
- reviews: story:eventlog-planning-authority-migration
revision: 1
---
# Whole migration command/config contract examination — pass 1 of 2

Verdict: **needs change**.

The companion has a coherent safety direction: distinct v1/v2 selection, read-only acquisition,
eight monotone phases, source/config rechecks, post-switch Eventlog authority, original-receipt
retry, and fail-closed writer admission. It is not yet a complete implementable command contract.
Six finite blockers remain. They concern the command/config and public consumer seams; this review
does not reopen the accepted raw-capture transport or SQL capture decisions.

## Frozen review basis

- AEP managed tree: `~/.local/state/worktree/trees/b10x/aep/ess-evolution-aep-migration-20260915`
- AEP HEAD: `715e52a88df9abcfcdd35752c09ab64b1e5a3fb3`
- Accepted owner outcome: `story-body.md`, SHA-256
  `9b2707c1596c6f2f683297aaf240401861ff2fc250b00f6fed9adaabb989cf32`
- Accepted direction: `docs/design/eventlog-planning-authority-v0.1.md`, SHA-256
  `6ddf871c246a01fe8b6acb8754411e898206b16588c80062dbccbe19dc29a2d6`
- Proposed companion: `docs/design/planning-store-selection-and-commands-v0.1.md`, SHA-256
  `1420d2e69541cd45e0ba5932ff86c9502dd87a8f95ac1d6851412eea0ce7fe27`
- Diagnostic coordinate model: `specs/planning-migration/domains/migration.yaml`, SHA-256
  `aac2ed5844677602649b56a53b0d71bc1d9b6725dedf2e3ae219adafbe37ee34`

The three named proposal hashes equal the fixed brief. The proposal remained unchanged during this
examination.

## Findings

### A. Closed public encodings are still prose

#### F1 — The five result families have names, but no implementable closed schemas

**Blocker.** The companion promises separate closed Rust-owned results and strict readers
(`planning-store-selection-and-commands-v0.1.md:94-115,241-260`), but the only definition is a table
of outcome prose and “essential facts.” It does not bind the exact variant roster, required fields,
coordinate types, refusal-code enum, accumulated-refusal shape, JSON representation, or text
rendering for any of:

- `aep.planning-inspection/1`
- `aep.planning-migration-dry-run/1`
- `aep.planning-migration-apply/1`
- `aep.planning-verification/1`
- `aep.planning-projection-rebuild/1`

This also leaves inspect resolution-refusal exits ambiguous: inspect unreadiness may exit 0, while
the exit-1 sentence enumerates dry-run/apply/verify/rebuild but not inspect
(`planning-store-selection-and-commands-v0.1.md:104-109`). The accepted raw-capture types do not fill
this gap; they define acquisition outcomes and capture refusal codes, not command results
(`crates/plan/aep-contract/src/migration/capture.rs:480-560,1067-1104`).

**Affected acceptance:** closed versioned reports and named refusals, exact alias/output/exit checks,
and strict result fixtures (`story-body.md:29-31,58-61`).

**Required decision:** publish the five exact Rust-equivalent wire models, complete refusal taxonomy,
exit mapping, and canonical text/JSON renderings before dispatch implementation.

#### F2 — Durable migration identity, files, selector bytes, and ownership markers are unbound

**Blocker.** `MigrationId`, `SnapshotId`, and `BoundaryId` are unconstrained string newtypes in the
diagnostic model (`migration.yaml:7-15`). The companion derives a destination and an AEP-owned
migration directory from project/migration identity, but specifies neither the identifier grammar
nor the deterministic directory/stage/final/receipt/phase filenames
(`planning-store-selection-and-commands-v0.1.md:80-85,187-204`). It also requires an ownership marker
for explicit-path refusal without defining its name, bytes, placement, ownership scope, or
reconciliation with phase records (`planning-store-selection-and-commands-v0.1.md:48-63`).

The `Published -> Selected` transition compares “exact intended v2 bytes,” but no deterministic v2
selector encoder says how existing required project fields are ordered and preserved, how comments
or equivalent YAML spellings are handled, or which bytes are placed in the immutable intent
(`planning-store-selection-and-commands-v0.1.md:31-36,191-204,212-216`). The current reader is one
closed `RawProjectConfig`, then `TryFrom`, and the current CLI has no lossless selector writer
(`crates/govern/aep-domain/src/project.rs:584-653`; `crates/edge/aep-schema/src/parse.rs:273-280`).

Without these bindings, two implementations can choose different paths/bytes for the same ID,
path-like IDs can collide with durable state, retry cannot identify the same intent, and a new build
cannot reliably recognize a retired explicit source or reconcile the two required phase copies.

**Affected acceptance:** stale/substituted input refusal, exact-identity retry, every interruption
boundary, atomic selector switching, retained recovery copies, and explicit projection/source
refusals (`story-body.md:37-50`).

**Required decision:** bind identifier grammars or opaque derivation, all durable relative paths and
file formats, exact selector construction, marker bytes, atomic durability rules, and disagreement
handling as closed versioned values.

### B. The proposed selector and mapping do not compose with the public adapter contract

#### F3 — `aep.project/2` does not identify the complete ER/Eventlog authority

**Blocker.** The v2 selector carries authority path, projection path, and one
`planning_identity` string (`planning-store-selection-and-commands-v0.1.md:18-36`). The candidate
public adapter opens and provisions an `Authority` containing three independent byte-preserved
coordinates: `logical_scope`, Eventlog `tenant`, and `stream_identity`
(`entity-eventlog/src/encoding.rs:21-45`; `entity-eventlog/src/adapter.rs:117-121,900-911`). The
companion does not state whether `planning_identity` is the stream identity, how logical scope and
tenant are derived or persisted, or how those derivations remain stable across move/retry/reopen.

The gap prevents construction and later byte-exact verification of the `Authority` the selector is
said to bind. A filesystem path and one provider identity cannot silently stand for the three public
adapter coordinates.

**Affected acceptance:** exact destination identity, ordinary project discovery, identity-mismatch
refusal, provision/recover retry, and v2 default/explicit equivalence (`story-body.md:19-31,40-46`).

**Required decision:** add or deterministically derive every `Authority` component, bind it into the
selector/intent/receipt, and define byte-exact comparison and path-move behavior.

#### F4 — The legacy ordering and evidence model cannot be mapped losslessly to `SubjectHistory`

**Blocker.** The diagnostic vocabulary admits `KnownOrder::{Store, Subject, PerKind, Unavailable}`
(`migration.yaml:22-24`), and the accepted outcome requires preserving available global order while
reporting unavailable global order as unavailable (`eventlog-planning-authority-v0.1.md:40-54`).
The candidate public import types offer only per-envelope `KnownLegacyOrder::{PerKind(u64),
Subject(u64)}` and anchor-wide `LegacyOrderDeclaration::{PerKindOnly, Subject}`; there is no
store-wide or unavailable variant (`entity-store/src/asynchronous/types.rs:457-478,525-556`).
Imports are one `SubjectHistory` at a time and its new suffix must be empty
(`entity-store/src/asynchronous/types.rs:569-577`; `entity-eventlog/src/adapter.rs:1224-1239`).

The companion says to emit one entry per source subject and “use the corresponding variants,” but
does not define a closed mapping for store-global order, absent order, Markdown relation ordinals,
auxiliary `aep.relation`/`aep.audit`/`aep.applied` subjects, source locators, complete envelope
eligibility, or governing-definition identities (`planning-store-selection-and-commands-v0.1.md:
168-185`). Those auxiliary families are distinct persisted entity kinds today, and logical hydrate
omits removed relations (`crates/plan/aep-backend-entity/src/lib.rs:38-53,119-145,873-909`).

Choosing `PerKindOnly` for unavailable order would invent an ordering guarantee; splitting
store-global evidence into subject imports would drop its cross-subject order unless a separate
typed mapping record is defined. Either violates the approved no-invention/full-preservation rule.

**Affected acceptance:** complete state/history equivalence, exact ordered relations, auxiliary
facts, original global IDs/order, explicit unavailable boundaries, and all backend/hybrid mappings
(`story-body.md:32-39,55-57`).

**Required decision:** provide a backend-by-backend closed source-to-subject/evidence mapping and
extend or supplement the public adapter vocabulary so every admitted ordering state and auxiliary
fact is representable and verifiable.

#### F5 — The rebuild/verification “authority position” is not available from the public read port

**Blocker.** Verify and rebuild require one exact selected authority position, and rebuild must
compare the caller's position before publishing a projection
(`planning-store-selection-and-commands-v0.1.md:72-78,111-115,247-260`). The public
`CompleteStoreSnapshot` contains scope, coverage, histories, and terminals but no authority or
provider position (`entity-store/src/asynchronous/types.rs:624-637`). Provider-complete assurance is
obtained through that read port (`entity-store/src/asynchronous/verify.rs:555-579`).

`PhysicalRef` is the coordinate of one Eventlog reference event, not a defined store frontier
(`entity-eventlog/src/encoding.rs:49-60`). Provisioning returns one for the binding, while imported
anchor outcomes expose only subject assurance and replay status
(`entity-eventlog/src/adapter.rs:928-959,1233-1253`). No public adapter operation named by the
companion yields a complete-snapshot position/watermark that `verify` can report and `rebuild` can
later compare.

**Affected acceptance:** proved-snapshot rebuild, stale concurrent-commit refusal, exact projection
watermark, complete verification, and deletion/drift recovery (`story-body.md:47-50`).

**Required decision:** define the authority-position algebra and add an admitted public snapshot
operation that atomically returns complete contents plus that position, then bind its CLI scalar and
comparison rules.

### C. The retained mutation surface lacks its committed-failure contract

#### F6 — Existing planning mutations have no defined receipt-bearing projection-failure result

**Blocker.** The accepted design applies the commit-then-project rule to **every command** and
requires a structured committed outcome containing the original durable receipt after projection
failure (`eventlog-planning-authority-v0.1.md:106-120`). The companion says every store-consuming
command resolves v2 (`planning-store-selection-and-commands-v0.1.md:44-46`) but defines result
families only for the five new `plan store` operations. It never defines how retained artifact,
driver, served mutation, evidence, or other write paths return a durable receipt plus projection
failure, how their retry identity reaches ER, or which exit/text/JSON shape distinguishes committed
from pre-commit failure.

The current CLI has a shared dispatcher, but its existing artifact commands return their existing
command-specific text/JSON and success codes (`crates/edge/aep-cli/src/app.rs:277-340,985-1004`;
`crates/edge/aep-cli/src/planning.rs:1568-1716`). The two binaries do share the same `aep_cli::main`
entry, so alias routing itself is sound (`crates/edge/aep-cli/src/aep.rs:1-5`;
`crates/edge/aep-cli/src/protocol.rs:1-5`). Shared dispatch does not supply the missing committed
outcome contract.

**Affected acceptance:** complete recorded execution, same-identity receipt recovery, post-commit
projection failure, drift refusal before ordinary mutation, and alias output/exit equivalence
(`story-body.md:19-22,47-50`).

**Required decision:** bind a common or per-command closed committed outcome, retry identity source,
receipt carrier, projection-failure carrier, and exit/rendering rules for every retained mutating
entry point, including served and driven writes.

## Complete surface disposition

| Surface | Disposition | Reason / closure condition |
| --- | --- | --- |
| v1 parser and four legacy selections | Direction accepted | Distinct closed readers and missing-version-v1 preserve compatibility. Implement with old/new fixtures. |
| v2 selection/defaults/path safety | Needs change | F2 and F3: exact selector bytes, durable ownership artifacts, and full adapter authority coordinates are missing. |
| explicit `--store` legacy behavior | Needs change | The early-return defect is correctly identified at `planning.rs:135-145`; F2 must bind the marker/resolver evidence that makes `ProjectionIsNotAuthority` and `LegacySourceRetired` decidable. |
| `inspect` | Needs change | F1 exact result/refusal/exit model; F2/F3 selection and phase identity. It remains read-only. |
| `migrate dry-run` | Needs change | F1 result schema and F3/F4 exact target/mapping assessment. Accepted raw snapshot hashing is reusable unchanged. |
| `migrate apply` | Needs change | F1-F4. The eight-phase ordering and fail-closed source recheck are sound, but durable bytes and exact import mapping are not bound. |
| `verify` | Needs change | F1, F3-F5. Complete provider-backed coverage is required; a caller-edited completeness marker is correctly rejected. |
| `rebuild` | Needs change | F1, F2, F5, F6. Post-switch authority and no re-execution are sound; the comparable authority position and ordinary committed outcome are missing. |
| Markdown physical acquisition | No new finding | The dedicated two-scan/raw-descendant direction matches the accepted capture boundary. Not executed or re-reviewed here. |
| SQLite physical acquisition | No new finding | Read-only existing-file transaction and no normal mutating opener are correctly required. Not executed or re-reviewed here. |
| PostgreSQL physical acquisition | No new finding | Read-only repeatable-read selected namespace and no schema-preparing constructor are correctly required. No SQL/server action was run. |
| Hybrid physical acquisition | No new finding | Bracketed local/divergence/replica roster and no policy winner are correctly required. F4 remains for semantic import. |
| `Prepared` | Needs change | F2/F3 must bind intent bytes, paths, selector and full destination authority. B-WRITER must admit actual control. |
| `DestinationProvisioned` | Needs change | F2/F3 must bind destination coordinates and recovery identity. B-ADMIN/provider qualification remains external. |
| `Imported` | Needs change | F2/F4 must bind exact per-subject/evidence entries and durable progress. |
| `Verified` | Needs change | F4/F5 must prove complete semantic/history equivalence at a defined provider position. |
| `Published` | Needs change | F2 must bind publish paths, exact rename evidence, and phase-copy reconciliation. |
| `Selected` | Needs change | F2 must bind exact v2 bytes and comparison. The post-switch authority rule itself is accepted. |
| `ProjectionPublished` | Needs change | F5/F6 must bind watermark/position and receipt-bearing projection failure. |
| `Complete` | Needs change | F1/F2/F5/F6 must bind immutable receipt bytes and separately typed current observation. |
| source/config rechecks | Direction accepted | Recheck under a reacquired actual guard and refusal on movement are explicit. B-WRITER still controls real admission. |
| writer control | Correctly external and fail-closed | Missing control must remain `WriterExclusionUnavailable`; no force/assumption path is admitted. This review supplies no fleet fact or qualification. |
| candidate ER/Eventlog adapter | External dependency | Public candidate interfaces were examined only for composition. AEP still pins `faadc04f...`; the candidate is not an accepted pin and B-ADMIN/provider qualification remains open. |
| six real cutovers | Outside this pass and unperformed | No rehearsal, migration, writer change, release, or deployment occurred. |

## External restrictions and non-claims

- B-ADMIN remains open. I did not inspect the denied administration reviewer tree, its three denied
  cases, or provider-administration implementation, and did not retry an equivalent review.
- B-WRITER remains open. No operational writer fleet, source denial, quiescence, or real apply was
  observed. `WriterExclusionUnavailable` remains the only honest real-apply outcome without it.
- M3 cannot clear M1 qualification, the four native provider stages, the adapter/bridge acceptance,
  or operational writer control. M4 still depends on a corrected, implemented executable.
- The accepted SQL/raw-capture implementation and its two reviews were not reopened. Their public
  types were read only to establish the command-layer seam.
- No implementation, configuration mutation, selector change, store open, capture, migration,
  cutover, publication, or release is claimed.

## Inspected versus executed

Inspected: the whole accepted owner outcome; all three frozen proposed inputs; current AEP config
parser/project loader/CLI dispatcher/store resolution and alias entry points; current legacy
identity projection declarations and hydrate behavior; the accepted public raw-capture result and
digest boundaries; AEP's current dependency pin; and the candidate Entity Runtime public adapter
exports, authority/position values, recorded read/import/provision interfaces, history/evidence
types, and provider-complete verification port. The exact paths and content hashes are in
`input-evidence-manifest.tsv`.

Executed: read-only filesystem/source searches, numbered source reads, SHA-256/byte counts, Git
HEAD/status inspection, and this reviewer's managed-worktree lease/heartbeat/release hooks. No
Cargo, test, formatter, browser, SQL client/server, AEP planning command, adapter operation, or
other operational command was executed.

## Pass-1 closure

Correct F1-F6 in the same companion, preserving the accepted safety rules and external blockers.
A second whole examination can then decide the corrected fixed contract. This pass does not approve
implementation from the current proposal.
