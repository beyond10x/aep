# Planning authority selection and migration commands v0.1

Status: proposed concrete command/config companion to eventlog-planning-authority-v0.1.md,
under the same migration owner. No command below is claimed to exist in the current executable.
The pure raw-capture contract and values are implemented at
715e52a88df9abcfcdd35752c09ab64b1e5a3fb3, with both source examinations and the full
repository gate complete. See planning-raw-capture-v0.1.md. Physical source acquisition,
semantic/history mapping, durable phases, command/result encodings and enforced legacy-writer
exclusion remain to be implemented under this same migration owner.

## Versioned selection

Parse project configuration through distinct closed raw v1/v2 readers, and retain the selected
version in the validated configuration. A missing version retains the existing v1 meaning.
Reject an unknown version before resolving/opening a planning backend. Do not extend v1's
untagged store enum so that a legacy project silently accepts the new authority.

V1 preserves every existing Markdown, SQLite, PostgreSQL and hybrid selection. V2 supports only
Eventlog file authority and Markdown projection. Its omitted `store` resolves to authority `state`
and projection `planning`, relative to the project directory; an explicit equivalent is:

```yaml
version: aep.project/2
store:
  eventlog:
    path: state
    projection: planning
planning_scope: '<existing ER logical scope>'
planning_tenant: '<existing Eventlog tenant identity>'
planning_identity: '<existing authority stream identity>'
```

Other required project fields retain their existing meanings. `planning_scope`, `planning_tenant`
and `planning_identity` are required, nonempty strings in v2 and are compared byte-for-byte to the
existing ER logical scope, Eventlog tenant and provider stream identity respectively. Together they
are exactly the public adapter `Authority`; none is inferred from a path, repository name or another
member of the tuple. `planning_identity` is not the
workflow `state: state.yaml` path, an Eventlog manifest field, a fresh random identity on open or
a value recovered from the projection. Initialization must provision and verify authority before
writing the selector. Migration uses the staged destination's verified identity. Omitted `store`
does not permit an omitted authority member or implicit empty-store initialization.

Explicit v2 paths must be relative, nonempty, contained in the project directory, and disjoint
from one another and the selector. Resolve existing ancestors without following a path out of
that boundary; refuse symlink aliases, nested authority/projection paths and foreign destination
content. Apply records the exact resolved identities and refuses substitution after inspection.
V2 has no Markdown authority, SQL runtime store, hybrid, fallback or automatic source import.

Every store-consuming command resolves this versioned selection, including driver paths,
validation, history, evidence and conformance. Vocabulary-only commands remain lazy and need not
open any store. The compatibility binary `protocol` uses the identical dispatcher and bytes.

## Explicit legacy paths

The existing `--store <directory>` remains a legacy Markdown locator; it never becomes an
Eventlog locator or a request to treat a projection as authority. In the new executable, before
opening an explicit path, inspect its nearest project selector and any migration ownership marker
through the same resolver. A v2 projection or retired source refuses with `ProjectionIsNotAuthority`
or `LegacySourceRetired`, including an empty projection and invocation from outside the project.
An unrelated explicit v1/standalone Markdown store retains its existing behavior.

This corrects the early explicit-store return in `crates/edge/aep-cli/src/planning.rs`; it does not
retroactively change old executables. Existing old builds were observed to write an explicit
projection path even when project discovery refused v2. Therefore a v2 selector, a marker or a
new advisory lock is not proof of old-writer exclusion. Cutover admission must establish actual
source-specific exclusion/quiescence, and report `WriterExclusionUnavailable` otherwise. No
`--force`, `--assume-quiescent` or assertion-only success path is added by this command contract.
Direct edits by an unfenced writer remain drift and cannot become authoritative commands.

## Command surface

All new operations accept `--project <project.yaml>` and `--format text|json`. Omitted project uses
ordinary discovery; an explicit project names the exact selector rather than a working directory.
No ambient legacy `--store` override applies to these operations. SQL connection information is
resolved from the selected legacy config and existing credential mechanism, never copied to output.

| Command | Required additional inputs | Result and effects |
| --- | --- | --- |
| `aep plan store inspect` | none | Read-only complete available source inventory, selection, history boundaries, readiness and drift. |
| `aep plan store migrate dry-run` | `--authority-scope <value>`, `--authority-tenant <value>` and exactly one of `--authority-new` or `--authority-identity <value>` | Read-only capture and proposed import/equivalence assessment for the exact destination request below; a new physical stream identity remains explicitly unassigned. Returns source snapshot/config identities or named unreadiness. |
| `aep plan store migrate apply` | `--snapshot <id>`, `--migration <id>` and the same destination request | Re-captures under admitted writer exclusion, checks the immutable request, creates or recovers its owned stage, binds the actual provider identity, stages/verifies, publishes and switches; returns the original matching migration receipt on retry. |
| `aep plan store verify` | none | Read-only verification of selected authority, its existing identity, complete recorded/imported boundary and projection equivalence. |
| `aep plan store rebuild` | `--authority-snapshot <sha256:...>` | Rebuilds the owned projection only if a fresh complete provider-owned capture produces the same opaque authority snapshot under its writer fence. Never re-executes imported or committed commands and never claims that the digest is a native Eventlog frontier. |

`--snapshot` is not a supplied capture payload. The command re-reads the selected source; it does
not trust caller JSON as storage evidence. `--migration` is a caller-supplied stable typed
idempotency identity. Same ID/same bound snapshot and config recovers the same phase/receipt;
same ID/different intent refuses. A source-only digest cannot authorize changed config or changed
hybrid policy. A destination is derived from the selected project and migration identity, not an
arbitrary path supplied at apply time. Its exact staging path is recorded in the durable intent.
Logical scope and tenant are authored destination coordinates, not inferred legacy facts. The
destination mode is also explicit and participates in dry-run comparison and immutable request
equality. Physical stream identity is provider-owned, as specified below; a fresh migration never
predicts it. The selected v2 authority always has all three concrete coordinates.

Inspect/dry-run/verify never initialize, recover, migrate, clean up or lock a store in a way that
creates durable lock files. Pending legacy batches and provider recovery intents are reported and
prevent an admitted capture. Reading a file twice can detect movement but does not prove writer
exclusion for apply. These commands use dedicated read-only provider entries and actual schemas.
No remote protocol fetch is needed to inventory raw storage; missing exact governing definitions
is reported distinctly when semantic validation/equivalence needs them.

## Results and failure boundaries

Each operation has a separate closed Rust-owned versioned result, generated schema and fixtures;
there is no arbitrary metadata map or partially populated success object. The operation's format
version identifies its exact result family. Machine output includes the selected backend kind,
config/source digests, exact source or authority snapshot, complete counts, explicit history
availability and structured refusal when applicable. Raw configuration bytes, credentials, SQL
connection strings and raw artifact/blob content never enter diagnostic output. Configuration
digests bind private source bytes without publishing those bytes into tracked migration records.

Exit0 requires the requested observation or mutation to complete. A read-only inspect may return
an observed unreadiness result with exit0 because it successfully diagnosed it; dry-run admission,
apply, verify and rebuild return exit1 on unmet requirements. CLI syntax remains Clap's exit2.
JSON is still emitted for typed exit1 refusals. Operational output failures retain the operation's
durable receipt/phase; they cannot relabel a committed apply as rollback or authorize a fresh ID.
Text and JSON report the same facts; protocol/aep aliases must have identical bytes and exits.

Apply's result distinguishes pre-switch refusal, switched-and-verified, and a recoverable uncertain
phase. An uncertain phase includes the migration identity needed for retry and never claims the
old source safe to reactivate. Committed projection failure carries the original command receipt.
Rebuild requires an exact opaque authority snapshot to prevent applying a stale projection after
another commit; the result names that snapshot and full projection inventory digest.

## Before implementation

Reuse the accepted pure raw-capture values and canonical raw-snapshot hashing; do not repeat
their implementation or reopen their two completed examinations. Remaining source work must
bind physical acquisition to those exact values, preserve semantic/history and relation mappings
(including hybrid-local evidence), and implement the durable migration phases and exact command
result schemas fixed below.
Writer control must provide source retirement and actual old-writer exclusion for every backend.

The ER adapter now has concrete candidate interfaces: EventlogRecordedStore::open validates an
already provisioned and attached authority; AsyncBindingProvisioner provisions it explicitly;
AsyncImportedAnchorWriter returns imported anchor outcomes; CompleteStoreSnapshot retains complete
recorded/imported boundaries; rebuild_indexes rebuilds and rechecks exact indexes; and
RecordedEventlogBridge supplies the existing synchronous edge with explicit shutdown outcomes.
Authority and PhysicalRef carry the destination identity and individual physical positions. Source preparation
can bind to these interfaces, but the candidate is not an accepted runtime pin. Provider
administration examination and adapter source acceptance remain required before qualification
and integration. No consumer check substitutes for those requirements.

A missing operational writer-control provider must yield WriterExclusionUnavailable. It does not
prevent implementing read-only inventory, legacy acquisition, dry-run, exact mappings, command
parsing or the closed phase transition logic against an explicit control interface. Tests of that
interface do not qualify a real writer fence. Apply qualification and every real cutover still
require observed exclusion/revalidation for the exact source. These are existing requirements,
not a new fencing platform or another migration story.
Old/new reader fixtures must precede new parser behavior. Native command tests cover all v1
selections, v2 defaults/explicit paths, unknown fields, identity mismatch, alias equivalence,
external explicit-path resolution, empty projection refusal, stale snapshots, original-receipt
retry, committed projection failure and actual old-writer races. No real cutover is admitted by
this proposed page alone.

## Physical acquisition and mapping boundary

The IO edge produces the accepted `RawCaptureObservationV1`; it does not reconstruct a capture
from `QueryService` results. Ordinary Markdown opening can recover a pending batch, SQL opening
can create schema, and `aep-backend-entity::hydrate` omits removed relations. Those paths cannot
prove complete, read-only acquisition. The existing raw-capture contract fixes the acquisition
roster, catalog order, row order, retained partial evidence, canonical transcript and refusal
codes; this command layer must reuse them without a second hashing scheme.

The bounded acquisition implementations belong beside each legacy backend, with orchestration in
the project/CLI edge. Markdown enumerates raw descendants without following links and runs the
two required scans. SQLite opens an existing database read-only, never creates schema or runs
recovery, and reads catalog and all row families in one transaction. It must not use immutable
mode to pretend a live WAL is absent. A database that cannot be observed without creating or
changing sidecars is reported unreadable; before/after filesystem inventories test this boundary.
PostgreSQL captures the selected existing namespace in a read-only repeatable-read transaction;
it never calls the normal schema-preparing constructor. Hybrid performs the exact bracketed
local/divergence/replica roster for both replica kinds and every admitted policy. The actual
schema predicates and queries must be checked against the accepted raw schema inventory, including
foreign catalog objects and malformed cells; a successful connection is not catalog admission.

Semantic mapping starts only from a validated complete capture and exact governing definitions.
It emits a closed import plan with one entry per source subject, all auxiliary planning subjects,
raw evidence references, exact ordered relations and explicit history boundaries. The plan must
enumerate, rather than sample, all `aep.entity`, `aep.relation`, `aep.audit` and `aep.applied` facts.
Removed relations, refused audit entries and old idempotency facts remain evidence even where a
query view hides them. Hybrid equivalence compares both complete inventories; its policy never
selects a winner for contradictory facts. Local-only journal evidence remains retained and named
even when the replica is the declared command authority.

The candidate adapter's actual import API accepts `SubjectHistory` with an
`Imported(LegacyAnchor)` origin and an empty newly recorded suffix. Complete legacy envelopes go
through `ImportedRecordEvidence`, preserving their original record identities and only the known
subject or per-kind order. Bare decisions/events use the corresponding `LegacyEvidence` variants.
Raw legacy journal representations that are not those types remain exact source evidence, not
fabricated envelopes. The available completeness and ordering declarations must be justified from
the complete capture. Importing a current state never proves genesis. `CompleteStoreSnapshot` is
editable data; setting its coverage field is not provider provenance. Destination verification
must obtain a fresh complete snapshot through the adapter's actual provider port.

## Durable apply phases and restart rules

This is the finite command protocol required by the existing migration acceptance, not a second
planning engine. The diagnostic model names these phase values; Rust-owned closed structs and
their generated schemas remain the implementation authority. Each phase binds the same immutable
request: migration ID, inspected snapshot ID, selector/config digests, source coordinate,
resolved staging/destination/projection coordinates, destination request, mapping
version and exact governing-definition digests. Provider-owned physical identity and final selector
bytes are durably bound at provisioning, as specified below. Changing a request member for an existing ID refuses.
Captured configuration credentials are never stored in this tuple; only its digest is retained.

Before a destination exists, durable intent lives in an AEP-owned migration directory beside the
project selector. It does not alter the Eventlog manifest. The directory is bound to the exact
project and migration ID, refuses foreign contents and path substitutions, and retains wanted
recovery evidence. Phase records are closed, versioned values atomically replaced with the
required file and parent-directory durability. Every completed phase cites the exact observations
that justify it; a phase string alone cannot authorize the next effect. Once staged, the final
authority also retains the import boundary and receipt references. Neither copy may be silently
preferred when the copies disagree.

| Last established phase | Required effect and next phase | Restart rule |
| --- | --- | --- |
| No intent | Acquire actual source/selector writer control, re-capture and compare the inspected snapshot/config, persist `Prepared` | No durable mutation is allowed before source/control admission. A read-only dry-run never creates this intent. |
| `Prepared` | Create/reopen the owned stage, obtain its provider identity, provision that exact authority, and persist `DestinationProvisioned` with the actual binding and finalized selector | Recover only the same owned stage and binding through `recover_binding`. An uncertain provision stays uncertain; do not create another authority or ID. |
| `DestinationProvisioned` | Import every planned subject and evidence item; persist `Imported` | Recover exact subject anchors and compare their complete contents. Resume missing entries only; foreign or different anchors refuse. |
| `Imported` | Reopen the stage, verify all state/history/identity reservations and complete projection equivalence, preserve the exact legacy recovery copy; persist `Verified` | Repeat verification from actual stage bytes. Imported counts alone do not establish equivalence. |
| `Verified` | Recheck held source/selector exclusion and exact source/config snapshot; publish the verified authority to its reserved final path; persist `Published` | A renamed destination without its next phase is recognized only by exact intent/binding/content evidence. Never overwrite a foreign destination. |
| `Published` | Under the still-held exclusion, atomically replace the selector with the exact v2 bytes; establish `Selected` | Inspect the selector. Exact old bytes mean pre-switch; exact intended new bytes mean selected. Any other bytes or unresolved durability return uncertainty. |
| `Selected` | Materialize the verified owned Markdown projection and watermark; persist `ProjectionPublished` | The new Eventlog authority is authoritative. Repair from it, preserving foreign-file conflicts; do not reactivate the legacy source. |
| `ProjectionPublished` | Reopen through ordinary project discovery, verify exact authority, complete projection and retained history boundary; persist `Complete` and its immutable receipt | A failed reopen/verification remains post-switch recovery. Do not issue fresh import commands. |
| `Complete` | Return the original receipt for matching retry | Revalidate that the selected authority agrees. Subsequent legitimate commands do not make the original migration receipt disappear. |

Projection and legacy source can initially share a pathname. Therefore no pre-switch phase replaces
the live legacy tree with the new projection. `Verified` preserves an exact recovery copy and a
separate staged projection; projection replacement occurs only after the selector is demonstrably
selected. New-build store writers see the incomplete migration intent and refuse ordinary mutation
until recovery completes. Older writers still require the independent enforced exclusion: the
intent file is not their fence. After switch, projection failure is a committed migration outcome
with the same migration identity and selected authority, not a pre-switch refusal.

Every transition revalidates its input identities and writer-control capability. A crash releases
process locks, so restart reacquires actual control before effecting any transition. A persisted
claim that a fence was once held cannot replace that reacquisition. Interrupted writes, truncated
records, missing evidence, mixed stages and selector substitution refuse or report uncertainty;
none is repaired by timestamp ordering, choosing the longest file or minting a new migration ID.
Read-only inspect/verify reports unresolved phases without advancing them.

Writer control is an edge-owned capability, not caller JSON or a boolean flag. Admission returns
an opaque held guard bound to the complete source fleet and selector; effecting operations require
that guard and a fresh recheck. Its implementation must enforce exclusion or prove the admitted
supervised quiescence for all relevant legacy writers, including restarts. The operational facts
needed for that provider remain unavailable. Until established, real apply returns
`WriterExclusionUnavailable`; pure phase tests and disposable controlled fixtures do not qualify
any of the six real stores.

## Closed command result families

Rust implementation supplies these separate format identities, each with a closed payload and
strict unknown-field/version rejection. Shared typed facts may be reused, but there is no
operation-name switch plus arbitrary result map. Published schemas come only from Rust types.

| Result format | Closed outcomes and essential facts |
| --- | --- |
| `aep.planning-inspection/1` | Observed source selection, raw completeness, explicit history availability, drift and migration phase; or resolution refusal. An observed unready store is still an inspection result. |
| `aep.planning-migration-dry-run/1` | Admitted snapshot/config/mapping identities, complete comparison and explicit destination requirements; or path-bearing accumulated refusals. No destination is provisioned. |
| `aep.planning-migration-apply/1` | Pre-switch refusal; selected but projection/recovery incomplete; complete original receipt; or unresolved outcome with exact migration identity and last proved phase. Only the complete receipt closes apply. |
| `aep.planning-verification/1` | Exact selected authority/snapshot, provider-backed complete coverage, each history assurance and projection inventory/watermark; or explicit mismatch/unreadiness. Imported suffix assurance remains distinguishable from genesis. |
| `aep.planning-projection-rebuild/1` | Rebuilt complete owned projection at the requested unchanged opaque authority snapshot; or stale-snapshot, ownership-conflict, incomplete-publication or other typed refusal. Never a new command receipt. |

Per-family outcomes retain structured codes and typed coordinates, not raw SQL errors, source
documents or connection strings. A read/parse failure is not silently converted to absence.
Internal retained evidence and public command summaries have separate representations. An apply
receipt names the original migration intent and selection, import comparison, destination identity
and final covered authority snapshot. If later commands advance that authority, retry returns the
same receipt with a separately typed current observation rather than rewriting history.

## Exact scalar, wire and rendering contract

The tables in this section are Rust-equivalent: every struct rejects unknown and missing fields;
every enum rejects unknown tags; every format scalar accepts only its one literal. JSON enum values
use an adjacent object with exactly `kind` and, for non-unit variants, `value`. A presence is either
`{"kind":"missing"}` or `{"kind":"present","value":...}`. There are no JSON nulls, flattened maps
or source-derived diagnostic strings. Lists preserve their declared order. Refusal lists are
nonempty and sorted by canonical coordinate bytes and then code.

`DigestV1` and exact bytes reuse the accepted raw-capture spellings: `sha256:` plus 64 lowercase
hexadecimal digits, and `hex:` plus lowercase byte pairs. `MigrationIdV1` is 1–128 ASCII bytes and
matches `[A-Za-z0-9][A-Za-z0-9._:-]{0,127}`. It is preserved byte-for-byte and never interpolated
into a path. `AuthorityValueV1` is a 1–255 byte UTF-8 string containing at least one non-whitespace
character and no NUL or ASCII control byte. It is otherwise opaque and is never trimmed or Unicode
normalized.

`AuthorityCoordinateV1` has, in this order, `logical_scope: AuthorityValueV1`,
`tenant: AuthorityValueV1`, and `stream_identity: AuthorityValueV1`. These are the exact three
members passed to and recovered from the adapter's public `Authority`. `AuthoritySnapshotIdV1`,
`SourceSnapshotIdV1`, `IntentDigestV1`, `PhaseDigestV1`, `ProjectionInventoryDigestV1` and
`ReceiptDigestV1` are distinct Rust newtypes over `DigestV1`; their shared wire spelling does not
make them interchangeable.

`AuthorityIdV1` is the distinct digest newtype produced by
`digest_parts_v1("aep.migration.authority/1", [logical_scope bytes, tenant bytes, stream_identity
bytes])`. The diagnostic `AuthorityCoordinate` entity uses that value as its identity.
`LegacyRecordCoordinate` identities and projection-watermark identities are likewise domain-separated
digests of all their non-identity fields under `aep.migration.legacy-record-coordinate/1` and
`aep.migration.projection-watermark/1`; changing a mapped fact mints a different identity and makes
the import comparison fail rather than overwriting evidence.

`BackendKindV1` is exactly `markdown | sqlite | postgres | hybrid | eventlog`. `ProjectVersionV1`
is exactly `aep.project/1 | aep.project/2`. `HistoryAvailabilityV1` is exactly
`complete_recorded | partial | unrecorded`. `MigrationPhaseV1` is exactly `prepared |
destination_provisioned | imported | verified | published | selected | projection_published |
complete`.

`DiagnosticCoordinateV1` is the following closed adjacent enum:

| tag | value fields |
| --- | --- |
| `selector` | `path: HostPathV1` |
| `config` | `field: ConfigFieldV1` |
| `source` | `coordinate: PhysicalCoordinateV1` from `aep.raw-capture/1` |
| `migration` | `migration_id: MigrationIdV1`, `component: MigrationComponentV1` |
| `destination` | `path: HostPathV1` |
| `projection` | `path: HostPathV1` |
| `authority` | `authority: AuthorityCoordinateV1`, `subject: PresenceV1<SubjectCoordinateV1>`, `record_id: PresenceV1<String>` |
| `output` | `stream: OutputStreamV1` (`stdout | stderr`) |

`ConfigFieldV1` is `version | store | planning_scope | planning_tenant | planning_identity |
protocol | profile | protocols | artifacts | task | state | principles | profiles | schemas |
providers`. `MigrationComponentV1` is `intent | ownership | phase_prepared |
phase_destination_provisioned | phase_imported | phase_verified | phase_published | phase_selected |
phase_projection_published | phase_complete | current | receipt | recovery_capture`.
`SubjectCoordinateV1` has `entity: String` and `id: String` and uses the adapter's byte-exact
subject components.

`CommandRefusalCodeV1` is exactly:

`unknown_project_version`, `invalid_project`, `unknown_field`, `path_outside_project`,
`path_alias`, `nested_store_paths`, `foreign_content`, `projection_is_not_authority`,
`legacy_source_retired`, `ownership_uncertain`, `source_unreadable`, `source_unstable`,
`pending_legacy_intent`, `unsupported_schema`, `incomplete_inventory`, `missing_definitions`,
`semantic_mismatch`, `divergent_hybrid`, `snapshot_changed`, `config_changed`, `selector_changed`,
`writer_exclusion_unavailable`, `intent_conflict`, `foreign_stage`, `destination_conflict`,
`provision_uncertain`, `import_conflict`, `import_uncertain`, `verification_mismatch`,
`publish_uncertain`, `selector_uncertain`, `projection_conflict`, `projection_drift`,
`authority_identity_mismatch`, `authority_snapshot_changed`, `incomplete_publication`,
`receipt_conflict`, `command_identity_conflict`, `committed_projection_failure`, and
`output_failure`.

`CommandRefusalV1` has `code: CommandRefusalCodeV1` and `at: DiagnosticCoordinateV1`. Public output
contains no free-form `detail`; the text renderer derives fixed English from the code and coordinate.
Private retained evidence may carry provider errors separately and is excluded from result schemas.

JSON output is the compact UTF-8 serialization of the declared field order followed by one LF.
Strings use JSON escaping; maps admitted inside typed payloads are `BTreeMap` and serialize in byte
lexicographic key order. Text output is a lossless deterministic flattening of the same value: one
`<path>\t<JSON scalar>` line per scalar, struct fields in declaration order, enum discriminants at
`<path>.kind`, variant values below `<path>.value`, lists with `<path>.count` followed by zero-based
`<path>[n]`, and one final LF. The root paths are `format` and `outcome`. This grammar, rather than
handwritten per-alias prose, establishes byte equality between `aep` and `protocol`.

## Exact five result families

Shared facts below are closed structs:

- `SelectionV1`: `project_version`, `backend`, `selector`, `selector_digest`, `config_digest`,
  `source`, `authority: PresenceV1<AuthorityCoordinateV1>`, and
  `projection: PresenceV1<HostPathV1>`.
- `InventoryCountsV1`: `subjects`, `entities`, `relations`, `audit_records`, `applied_commands`,
  `complete_envelopes`, `bare_decisions`, `bare_events`, `raw_evidence_items`, all `u64`.
- `HistorySummaryV1`: `complete_recorded`, `partial`, `unrecorded`, each `u64`.
- `MappingIdentityV1`: `mapping_version` (the literal `aep.planning-import/1`) and ordered
  `definition_digests: Vec<DigestV1>`.
- `AuthorityObservationV1`: `authority`, `snapshot_id`, `inventory`, `history`, and
  `capture_digest`.
- `ProjectionObservationV1`: `root`, `authority_snapshot`, `inventory_digest`, `watermark_digest`,
  and `drift: ProjectionDriftV1` (`current | missing | extra | stale | corrupt | foreign_conflict`).
- `MigrationReceiptV1`: `format` (literal `aep.planning-migration-receipt/1`), `migration_id`,
  `intent_digest`, `source_snapshot`, `source_config_digest`, `authority`, `mapping`,
  `import_comparison_digest`, `selected_selector_digest`, `covered_authority_snapshot`,
  `projection_inventory_digest`, and `receipt_digest`. `receipt_digest` is the digest of the same
  value with that field omitted, under `aep.migration.receipt/1`.

The five roots and outcomes are:

| format | closed outcome variants and exact value fields |
| --- | --- |
| `aep.planning-inspection/1` | `observed { selection, raw_complete: bool, inventory, history, migration_phase: PresenceV1<MigrationPhaseV1>, authority: PresenceV1<AuthorityObservationV1>, projection: PresenceV1<ProjectionObservationV1>, readiness: InspectionReadinessV1 }`; `refused { refusals }` |
| `aep.planning-migration-dry-run/1` | `admitted { selection, source_snapshot, inventory, history, target_authority, mapping, comparison_digest, destination_requirements: DestinationRequirementsV1 }`; `refused { selection: PresenceV1<SelectionV1>, refusals }` |
| `aep.planning-migration-apply/1` | `complete { receipt, current: AuthorityObservationV1 }`; `recoverable { migration_id, intent_digest, last_proved_phase, authority: PresenceV1<AuthorityCoordinateV1>, original_receipt: PresenceV1<MigrationReceiptV1>, refusals }`; `uncertain { migration_id, intent_digest, last_proved_phase: PresenceV1<MigrationPhaseV1>, refusals }`; `refused { migration_id, last_proved_phase: PresenceV1<MigrationPhaseV1>, refusals }` |
| `aep.planning-verification/1` | `verified { selection, authority, projection, import_comparison_digest }`; `mismatch { selection, authority: PresenceV1<AuthorityObservationV1>, projection: PresenceV1<ProjectionObservationV1>, refusals }`; `refused { selection: PresenceV1<SelectionV1>, refusals }` |
| `aep.planning-projection-rebuild/1` | `rebuilt { authority, projection, replaced_owned_paths: u64, preserved_foreign_paths: u64 }`; `refused { requested_snapshot, current_snapshot: PresenceV1<AuthoritySnapshotIdV1>, refusals }`; `uncertain { requested_snapshot, staged_inventory_digest, refusals }` |

`InspectionReadinessV1` is `ready | source_unready | migration_incomplete | projection_drifted`.
`DestinationRequirementsV1` has `migration_root`, `staging_path`, `destination_path`,
`projection_path`, `authority`, and `foreign_content: Vec<HostPathV1>`. The list is complete and
canonical, not a sample.

Exit 0 is limited to inspection `observed`, dry-run `admitted`, apply `complete`, verification
`verified`, and rebuild `rebuilt`. Inspection `observed` may carry an unready readiness value and
still exits 0 because the requested inventory completed. Inspection resolution `refused` exits 1.
Every other typed outcome exits 1 and is still written to stdout in the selected format. Clap
syntax/help/version retain Clap's exit 2 before a typed operation exists. A failure writing stdout
exits 1, writes only a fixed `output_failure` diagnostic to stderr when possible, and never changes
the durable phase or receipt; a matching retry returns the original result.

## Exact v2 selector and complete authority

`RawProjectConfigV2` is a distinct closed reader. It carries every existing project field with its
v1 meaning, the Eventlog store, and the three required authority strings. The source key order is:
`version`, `protocol`, `profile`, then present `summary`, `protocols`, `artifacts`, `task`, `state`,
`principles`, `profiles`, `schemas`, then `store`, `planning_scope`, `planning_tenant`,
`planning_identity`, then nonempty `providers`. V2 rejects v1 store variants and every unknown
field. The three authority members must reproduce the exact adapter `Authority` recovered at open.

The selector written by migration is deterministic JSON-as-YAML: one compact JSON object in that
field order, provider keys byte-lexicographically sorted, followed by LF. Absent optional v1 fields
remain absent; present values are parsed and re-emitted as JSON strings without semantic
normalization. Relative paths use `/`, are nonempty UTF-8, contain no empty, `.`, `..`, NUL or
platform-prefix component, and are checked through no-follow ancestor handles before encoding.
Comments and YAML presentation are not carried into v2; the exact original selector remains in the
recovery capture and its digest remains in the receipt.

Canonical default-store selector example (the other existing fields are illustrative values, but
the bytes and order shown are exact for those values):

```json
{"version":"aep.project/2","protocol":"adp/1","profile":"development/1","store":{"eventlog":{"path":"state","projection":"planning"}},"planning_scope":"aep.planning","planning_tenant":"planning-main","planning_identity":"stream-01"}
```

The trailing LF after the displayed object is part of the selector. The corresponding public
adapter authority is exactly `{ logical_scope: "aep.planning", tenant: "planning-main",
stream_identity: "stream-01" }`; no path participates in that tuple.

Dry-run and apply require authored scope/tenant and an explicit destination mode. A v2
inspect/verify/rebuild still takes the complete concrete authority only from the selected
configuration and refuses any provider mismatch.

## Provider-owned identity during fresh migration

This correction is required by the original M3 fresh-stage and restart outcome. The public file
provider creates its physical stream identity on initial open; `prepare_file` can return it only
after creation, and `provision_file` correctly refuses a different requested identity. The earlier
contract froze a predicted identity and final selector before that first effect, so it could not
provision a genuinely fresh stage. Pure capture, injected coordinator tests and a manually prepared
fixture do not establish this outcome. No provider or ER API change, extra preparation command or
administration review is required.

`--authority-new` selects a closed provider-assigned request containing exactly the authored
logical scope and tenant. `--authority-identity` selects an exact-existing-identity request; it is
usable only against an already owned migration stage/record with that identity, never as an order
to create or rename a provider stream. Missing, mixed or contradictory modes refuse. Retrying a
provider-assigned migration uses the same original mode and request, even after its physical
identity is known. It never replaces the original request with an exact-identity request.

The eight logical phases remain. Their identity ordering is:

1. Before effects, acquire actual source/selector writer control, re-capture/recheck source and
   configuration, and persist the immutable request and ownership marker. Bind migration ID,
   snapshot/config/selector digests, exact source and owned paths, mapping/governing definitions,
   destination mode and authored scope/tenant. The request also fixes the deterministic selector
   projection version. It contains no guessed physical identity or future selector digest.
2. Within `Prepared` to `DestinationProvisioned`, create or reopen only that request's owned stage.
   Obtain the actual provider identity there, provision the adapter using it, and durably record
   the complete authority, original binding receipt, and exact finalized v2 selector bytes/digest.
   The binding record commits to the immutable request digest. No import, publication or selector
   switch may precede this binding. All descendants use that recorded authority and selector.
3. Recovery after provider creation but before recording the binding reopens only the same owned
   stage and reconciles its actual provider/adapter outcome. Never generate another stream or
   recreate a missing or substituted store. Foreign contents, ambiguous ownership, a different
   present binding or an unresolved creation/provision outcome refuse or remain uncertain.
   Resume after binding requires exact coordinate/receipt/selector agreement. A later selector
   switch uses those exact stored bytes, not a fresh independently generated selector.

Dry-run remains read-only: its new-stream result reports the closed pending-identity request and
does not open/create the destination merely to discover a UUID. Readiness of the source and the
ability to create a new owned destination do not fabricate an existing complete authority.

This changes the intent, dry-run and durable ownership/phase encodings. Preserve the existing
version-1 types, canonical bytes and fixtures; introduce version-2 replacements for the intent,
dry-run result, ownership marker, phase record and phase pointer, with closed Rust types and
old-reader rejection tests. The current v1
layout/field catalog below is retained as the compatibility definition, not permission to write
an unknown identity into `AuthorityCoordinateV1`. No empty/sentinel identity, mutable intent,
generic JSON template or metadata bag is admitted. The author must document the exact new field
catalog beside the existing one before adding its readers. Unchanged capture, ordinary mutation,
receipt and `aep.project/2` selector meanings do not change solely because a new intent version
references them. Ownership resolution must recognize the new pending marker and fail closed until
its request/binding/phase chain can be reconciled.

Bounded deliverable and stopping condition: implement this fresh-stage identity path in the same
five-command assignment and pass (a) read-only dry-run with no destination, (b) actual empty-stage
apply through provision/import/select/reopen using the minted identity, (c) interruptions before
and after provider creation and binding persistence that resume the same identity/receipt,
(d) changed request, foreign/substituted stage, mismatched binding and conflicting selector refusal,
and (e) old-reader/byte compatibility. Existing writer-control, phase, complete equivalence and full
repository gates still apply. A disposable successful apply does not qualify the six real writer
fleets or clear provider acceptance. This is one correction to the existing M3 contract, not a new
task, product capability, third contract review or authorization request.

## Durable layout, encodings and reconciliation

Let `engineering` be the selector's parent and `migration_key` be the lowercase 64-hex payload of
`digest_parts_v1("aep.migration.directory/1", [migration_id.as_bytes()])`. The only migration-owned
layout is:

```text
<engineering>/migrations/<migration_key>/
  intent.json
  ownership.json
  current.json
  phases/01-prepared.json
  phases/02-destination-provisioned.json
  phases/03-imported.json
  phases/04-verified.json
  phases/05-published.json
  phases/06-selected.json
  phases/07-projection-published.json
  phases/08-complete.json
  recovery/raw-capture.json
  recovery/source-location.json
  receipt.json
  stage/authority/
  stage/projection/
```

The final authority path is the v2 store authority path; the final projection path is the v2
projection path. Stage paths are those literal descendants and never caller supplied. Before any
write, every ancestor is opened without following links; wrong type, alias, hard-link substitution,
nested source/destination/projection, or existing unowned content refuses.

`MigrationIntentV1` (`aep.planning-migration-intent/1`) has exactly: `migration_id`,
`source_snapshot`, `selector_digest`, `config_digest`, `source_coordinate`, `migration_root`,
`staging_path`, `destination_path`, `projection_path`, `authority`, `mapping`,
`intended_selector_bytes: HexBytesV1`, `intended_selector_digest`, and `intent_digest`.
`intent_digest` hashes the preceding fields under `aep.migration.intent/1`. `intent.json` is created
exclusively, fsynced, and never replaced. Same migration ID with any unequal field is
`intent_conflict`.

`DestinationRequestV2` is the closed adjacent enum `provider_assigned { logical_scope:
AuthorityValueV1, tenant: AuthorityValueV1 } | exact_existing { authority:
AuthorityCoordinateV1 }`. `MigrationIntentV2` (`aep.planning-migration-intent/2`) has exactly:
`migration_id`, `source_snapshot`, `selector_digest`, `config_digest`, `source_coordinate`,
`migration_root`, `staging_path`, `destination_path`, `projection_path`, `destination_request`,
`mapping`, `selector_version: ProjectVersionV1` fixed to `aep.project/2`, and `intent_digest`.
`intent_digest` hashes the preceding fields under `aep.migration.intent/2`. It contains no physical
identity or final selector bytes. `intent.json` remains exclusive, synced and immutable; a version-1
reader rejects this format literal and every added field.

`DestinationRequirementsV2` has the four owned paths, the exact `destination_request`, and ordered
`foreign_content: Vec<HostPathV1>`. `DryRunAdmittedV2` has `selection`, `source_snapshot`,
`inventory`, `history`, `destination_request`, `mapping`, `comparison_digest`, and
`destination_requirements`. `DryRunResultV2` (`aep.planning-migration-dry-run/2`) is the closed
`admitted(DryRunAdmittedV2) | refused(DryRunRefusedV1)` outcome. A provider-assigned admitted result
therefore carries scope and tenant twice only through equality with the nested destination
requirements; it carries no `AuthorityCoordinateV1`, empty identity or guessed selector.

`OwnershipMarkerV1` (`aep.planning-store-ownership/1`) has `migration_id`, `intent_digest`,
`legacy_source`, `projection_path`, `intended_selector_digest`, and `marker_digest`. It is written
and synced with `Prepared`. The explicit-path resolver walks from the supplied path to the nearest
project selector without following aliases and scans only validated `migrations/<64hex>/ownership.json`
entries. It refuses a matching projection when the selector equals `intended_selector_digest`; it
refuses a matching legacy source at `Selected` or later. A matching marker with missing, conflicting
or unreadable phase/selector evidence yields `ownership_uncertain`, never writable Markdown.
Content emptiness is irrelevant, so an empty projection is still refused.

`OwnershipMarkerV2` (`aep.planning-store-ownership/2`) has exactly `migration_id`,
`intent_digest`, `legacy_source`, `staging_path`, `destination_path`, `projection_path`,
`destination_request`, and `marker_digest`. The marker digest uses
`aep.migration.ownership-marker/2`. It proves the request owned these exact derived paths before
provider creation; it does not claim a future identity or selector digest. Ownership resolution
joins it to the version-2 phase chain and, from `DestinationProvisioned`, to the binding observation.

`PhaseRecordV1` (`aep.planning-migration-phase/1`) has `migration_id`, `intent_digest`, `phase`,
`predecessor: PresenceV1<PhaseDigestV1>`, ordered `observations: Vec<PhaseObservationV1>`, and
`phase_digest`. `PhaseObservationV1` is a closed enum over selector/config/source capture,
writer-control reacquisition, binding outcome, imported-subject comparison, provider-complete
snapshot, projection inventory, rename durability and selector durability, each carrying only typed
coordinates and digests. Each numbered phase file is created exclusively and fsynced; `current.json`
is an atomically replaced `{format,migration_id,intent_digest,phase,phase_digest}` pointer whose file
and parent are synced. A phase never overwrites evidence from an earlier phase.

`PhaseRecordV2` (`aep.planning-migration-phase/2`) and `CurrentPhaseV2`
(`aep.planning-migration-current/2`) retain the version-1 fields and ordering but carry the closed
`MigrationPhaseObservationV2` enum. Its unchanged variants reuse the exact version-1 values.
`destination_precondition` has `staging_path`, `destination_path`, `projection_path`, and three
booleans `staging_absent`, `destination_absent`, `projection_safe`; it is present in `Prepared` and
proves the no-foreign-content check preceded provider effects. Its `binding` value has
`intent_digest`, the complete provider-minted `authority`, exact public `physical` receipt,
`replayed`, `intended_selector_bytes`, and `intended_selector_digest`; it is the sole
`DestinationProvisioned` observation and fixes all later coordinates. Phase and pointer digests use
the `/2` format bytes and `aep.migration.phase/2`; version-1 readers reject both literals.

The exact `PhaseObservationV1` variants are:

| tag | value fields |
| --- | --- |
| `selector` | `digest: DigestV1` |
| `config` | `digest: DigestV1` |
| `source_capture` | `snapshot_id: SourceSnapshotIdV1`, `capture_digest: DigestV1` |
| `writer_control` | `source_digest: DigestV1`, `selector_digest: DigestV1` (the opaque held guard itself is never serialized) |
| `binding` | `authority: AuthorityCoordinateV1`, `physical: PhysicalRefV1`, `replayed: bool` |
| `imported_subject` | `subject: SubjectCoordinateV1`, `boundary_digest: DigestV1`, `replayed: bool` |
| `complete_snapshot` | `snapshot_id: AuthoritySnapshotIdV1`, `capture_digest: DigestV1` |
| `projection` | `authority_snapshot: AuthoritySnapshotIdV1`, `inventory_digest: ProjectionInventoryDigestV1` |
| `rename` | `from: HostPathV1`, `to: HostPathV1`, `source_absent: bool`, `destination_present: bool`, `parents_synced: bool` |
| `selector_durability` | `old_digest: DigestV1`, `new_digest: DigestV1`, `file_synced: bool`, `parent_synced: bool` |

`PhysicalRefV1` exactly mirrors the public adapter value: `event_id: String`, `global_seq: u64`,
`stream_id: String`, `stream_version: u64`. Physical references are evidence for the individual
binding/import effect that returned them; they are never promoted to a whole-authority frontier.

`recovery/raw-capture.json` is the exact accepted `RawCaptureObservationV1` used at `Verified`.
`recovery/source-location.json` is a closed credential-free source coordinate plus its config
digest. Markdown and SQLite recovery retain their separately owned source bytes; PostgreSQL retains
the exact raw capture and the still-existing source under enforced read-only retirement, never a
credential copy; hybrid retains both sides and divergence evidence. None is an enabled writer.

At `DestinationProvisioned` and later, the staged/final authority retains the same intent digest,
latest phase digest, import-boundary digest and receipt digest as typed AEP migration subjects.
Recovery compares the full digest chain. Exact equality permits resume; a missing side is restored
only from the side whose preceding durable rename/append observation proves it; two unequal present
copies yield `receipt_conflict` or an uncertain result. Timestamp, longest-file and highest-phase
selection are forbidden.

`receipt.json` is the exact compact JSON-plus-LF `MigrationReceiptV1`, created exclusively and
synced only after `Complete`. Its digest and bytes never change. Matching retry revalidates selector,
authority identity and current complete snapshot, then returns that receipt plus the separately
typed current observation.

## Complete legacy-to-import mapping

Mapping version `aep.planning-import/1` begins only from a validated complete raw capture and exact
governing definitions. It first builds a canonical source fact ledger, sorted by accepted raw-capture
coordinate, and enforces global uniqueness of every available original record ID. Same ID with any
different envelope bytes, subject or source locator refuses before the destination exists.

Every current artifact becomes one `aep.entity` subject with the exact existing identity, namespace,
revision, status, body bytes, scope, references, metadata and archived fact. Every relation ever
present becomes one `aep.relation` subject; SQL preserves its stored ID, while Markdown uses the
already-authored migration ID derived from source document coordinate plus relation ordinal and
records that source mapping. Removed relations remain imported evidence and terminal removed facts.
Every accepted or refused audit item becomes one `aep.audit` subject. Every idempotency fact becomes
one `aep.applied` subject with its original key, command ID and replay result. Unknown instance kind,
unresolved reference, ambiguous Markdown relation match, or duplicate mapping refuses.

Backend rules are exact:

| source | current state and auxiliary mapping | history/order mapping |
| --- | --- | --- |
| Markdown | Parse each retained document into its artifact subject while retaining exact document bytes. Map frontmatter relations by document coordinate and ordinal. Preserve journal/pending-file presence independently. | Sealed complete envelopes use `ImportedRecordEvidence`; old `Change` and bare `DomainEvent` entries use their matching evidence kind only. Journal line order supplies store order. A document with no complete genesis is an imported anchor, never genesis. Contradictory document/journal state refuses. |
| SQLite | Map all physical `aep.entity`, `aep.relation`, `aep.audit`, `aep.applied` instances, including removed relations, plus events, history and legacy origins. Preserve original provider identities and raw TEXT/storage-class evidence. | Complete envelopes preserve record IDs. SQL sequence/order facts become store or subject order only where the captured schema proves them. Missing envelope/receipt/order remains explicit boundary evidence. |
| PostgreSQL | The SQLite mapping applies to the selected captured namespace, additionally binding provider sequences and credential-free endpoint identity. Foreign catalog objects remain evidence and do not become AEP subjects. | Repeatable-read row/sequence order is used only where the accepted schema defines it; no timestamp or physical-row inference. |
| Hybrid | Construct the full Markdown and replica ledgers independently, then compare every current and historical fact. Exact active-edge correspondence is one-to-one. | Equal facts retain both source locators. Local-only journal evidence is retained even for replica authority. Any contradictory fact or ambiguous duplicate match refuses; policy never chooses a winner. |

For each destination subject, the terminal state becomes `LegacyAnchor.instance`; completeness is
`CompleteSubject` only when the capture proves the complete subject evidence through that revision,
otherwise `AvailableEvidenceOnly`. Complete envelopes use `LegacyEvidence::Envelope` and reserve
their original global IDs. Bare decisions and events use only their corresponding variants. The
newly recorded suffix is empty. Store-global order implies a subject order by stable filtering, so
the adapter receives honest subject indices while the original global ordinal is retained in a
typed AEP boundary record. Per-kind order maps directly. Unavailable global order is never replaced
with a synthetic index.

Typed AEP boundary evidence supplements, but does not replace, the adapter anchor. One
`aep.migration.LegacyRecordCoordinate` subject is imported for every retained historical item with
source snapshot, source locator, destination subject, evidence kind, original record ID presence,
and `store | subject | per_kind | unavailable` order plus its present ordinal. The complete ordered
set is hashed into the import-comparison digest. This preserves cross-subject store order and raw
unavailable-order evidence while provider-complete verification enumerates the boundary subjects.

The current public import type needs no amendment for the supported sources. Every envelope eligible
for `ImportedRecordEvidence` in Markdown or SQL has a captured journal/row coordinate from which at
least per-kind or subject order is proved; store order is filtered to subject order and retained
separately. A complete envelope whose representation does not prove either order is not coerced into
that type. It has three joined ordinary provider subjects, all imported as immutable boundary values:

1. `aep.migration.LegacyRecordCoordinate` carries the coordinate fields above plus
   `evidence_blob_id`, `envelope_digest` and `reservation_roster_id`;
2. `aep.migration.LegacyEvidenceBlob` carries `format: aep.legacy-evidence-blob/1`, the source
   snapshot and coordinate IDs, the original record ID, exact envelope bytes as
   `hex:<two lowercase hexadecimal digits per byte>`, their byte length and
   `digest_parts_v1("aep.migration.legacy-evidence/1", [exact envelope bytes])`; and
3. one `aep.migration.LegacyIdReservationRoster` per import boundary carries
   `format: aep.legacy-id-reservations/1`, boundary and authority IDs, and the complete nonempty
   vector sorted by original record-ID UTF-8 bytes. Each entry is the closed tuple
   `{record_id, coordinate_subject_id, evidence_blob_subject_id, envelope_digest}`. Duplicate IDs,
   duplicate subject IDs, an unsorted vector or a tuple that does not join byte-for-byte refuses.
   `roster_digest` is `digest_parts_v1("aep.migration.legacy-id-reservations/1", [authority
   canonical parts, boundary ID, canonical entry parts])`.

The three subject kinds are reserved backend types. Their imported terminal values and relations
cannot be changed by an ordinary planning command; removal, replacement or a relation to a foreign
boundary is a provider-complete verification failure. The Eventlog planning backend loads and
validates every roster and referenced blob from the same fresh complete snapshot before accepting a
new command. It rejects a proposed record ID present either in adapter lookup or any validated
roster. The check is inside the same recorded-command guard and all writers for this authority use
it; B-WRITER still has to prove that operational premise.

The ordinary opened Eventlog backend exposes imported evidence through
`with_store` and `EventlogPlanningStore::legacy_evidence_for_subject` or
`legacy_evidence_by_original_id`. Each read takes a fresh provider-complete snapshot and validates
the complete boundary/coordinate/blob/roster join before returning separate typed evidence with
the exact envelope bytes, source and destination identities, original record ID, and explicit
order and ordinal. Subject lookup presents results deterministically by coordinate ID; that
presentation order does not assert a historical order. Unavailable-order evidence remains distinct
from newly recorded `HistoryProvider` and `QueryService` history, whose ordered suffix semantics do
not change. Destination verification regenerates the complete roster
and every blob from the source fact ledger, then compares the ordinary provider subjects and joins.
The external `recovery/raw-capture.json` is still required for incomplete apply recovery but is not a
history or collision side table: after `Complete`, deleting that recovery copy leaves history,
lookup and collision refusal byte-identical because their only inputs are the provider-complete
subjects. Missing or tampered coordinate, blob or roster data changes the complete transcript and
authority digest and fails verification. Thus original global identity collision protection and
complete available history remain inside the selected authority without inventing order or changing
ER public types. Any future source that cannot provide either adapter-eligible order or this closed
coordinate/blob/reservation join is `unsupported_schema`; no currently supported legacy backend is
narrowed.

## Opaque authority snapshots and projection watermarks

An authority snapshot is an AEP digest of the public provider-complete read, not an Eventlog native
position. `AepCompleteStoreTranscriptV1` contains format `aep.complete-store-transcript/1`, the exact
`AuthorityCoordinateV1`, the requested logical scope, coverage fixed to `complete_snapshot`, and all
subject snapshots sorted by `(entity bytes,id bytes)`. Each subject carries its origin, terminal
instance and stored suffix. Each stored record carries its complete entry, subject/store positions,
receipt, expectation, exact request bytes and exact record bytes. Imported anchors carry terminal
instance, completeness, declared order and every evidence item. Boundary coordinate, exact blob and
reservation-roster subjects are ordinary enumerated subjects and therefore their full values and
joins enter the same transcript.

The transcript uses the accepted `frame_v1` length/count grammar. Enum tags and UTF-8 strings are
parts; integers are unsigned big-endian; byte strings are exact; arrays contribute their count then
members; JSON values in entity fields use tags `null | false | true | number | string | array |
object`, arrays retain order, objects sort keys by UTF-8 bytes, and numbers use the in-memory
`serde_json::Number::to_string()` spelling. No provider file bytes or unordered map iteration enter
this semantic snapshot.

`AuthoritySnapshotIdV1` is
`digest_parts_v1("aep.migration.authority-snapshot/1", [authority canonical parts, complete
transcript bytes])`. It is computed only after `verify_complete_store` obtains and validates a fresh
provider-owned complete snapshot. Read-only verify captures twice and claims `verified` only when
the two IDs match around projection inspection. Rebuild captures under the actual authority writer
fence, compares the first ID to `--authority-snapshot`, stages the full projection, captures again
under the same fence, and publishes only if the ID is unchanged. A mismatch is
`authority_snapshot_changed`; it is not a stale native event-position comparison.

`ProjectionWatermarkV1` (`aep.planning-projection-watermark/1`) has `authority`,
`authority_snapshot`, `projection_inventory_digest`, and `watermark_digest`. The inventory digest
covers the sorted relative owned paths, exact file bytes, modes admitted by the projection contract,
and explicit foreign conflicts. The watermark lives in the AEP-owned migration/authority record,
not as authority inferred from Markdown. A projection-local copy is an optimization and is accepted
only when byte-equal to the authority record. This supplies stale-rebuild and drift comparison
without changing the public Eventlog adapter or claiming a native provider frontier.

The watermark names the complete snapshot **before its own recorded creation**. It cannot name
a snapshot containing itself: that would require a cryptographic fixed point. Each publication
therefore creates a distinct immutable watermark subject with one creation record. Its identity
binds the authority, covered snapshot and projection inventory under the existing watermark
identity domain. The complete provider transcript continues to include every subject, including
all watermark subjects; no metadata filter is permitted in `AuthoritySnapshotIdV1`.

Under the writer fence, rebuild captures S, checks the requested snapshot, stages S's deterministic
projection, and captures again to prove S unchanged. It then records W naming S and publishes the
staged projection. Verify captures the current complete authority C twice around projection
inspection and requires equality. It validates W's identity, exact payload, sole creation record
and durable receipt, then reconstructs the covered transcript by removing only that exact new W
subject from C. All other subjects, previous watermarks, origins and stored suffix bytes remain.
The reconstructed snapshot hash must equal W's `authority_snapshot`; any intervening business,
invocation, receipt or other metadata write makes this comparison stale. This is one explicit
publication boundary, not permission to filter metadata or select arbitrary historical prefixes.

A failure after W commits is a committed projection failure. Recovery reuses the exact W and its
original receipt and republishes only after the same coverage check; it never repeats a business
command or silently creates another W. Same identity with changed payload refuses. A projection
local watermark must still be byte-equal to the recorded W. Required controls cover an unchanged
successful publication, crash after W/before publication, identical retry, extra business/control
write, modified or additional watermark record, and preserved complete-transcript inclusion.

## Every retained mutation and committed projection failure

Every v2 mutation reaches the same recorded execution path: CLI artifact writes, served board
transitions, driver/harness submissions and any internal `CommandService` caller. Vocabulary-only
and read-only queries do not. V1-only hybrid `catch-up` remains a legacy operation and is refused as
`legacy_source_retired` after selection.

`CommandIdentityV1` uses the `MigrationIdV1` scalar grammar. CLI mutation commands accept a common
optional `--command-identity`; when absent the edge mints one before execution. A served mutation
requires the client request identity, and a driver derives it from execution/action identity. The
identity belongs to the complete invocation, including ordered `--relate` values and the requested
`--via` destination; it is never reused as a child command identity. The canonical complete request
bytes and `request_digest = digest_parts_v1("aep.planning-invocation-request/1", [request bytes])`
are fixed before any command executes. A retry supplies the returned invocation identity. Same
identity/different request digest is `command_identity_conflict`; same bytes resumes the original
invocation and never repeats a committed command.

`InvocationReservationV1` is an immutable ordinary authority subject with format
`aep.planning-invocation-reservation/1`, invocation identity, request digest, ordered step count and
the digest of every planned child tuple. Its exclusive recorded creation is the admission point that
makes identity conflict durable even before the first business command commits. It is not a
business result and is not projected into Markdown. A child tuple is
`{step_index:u64, child_identity, child_request_digest}`. Steps are a contiguous zero-based vector.
`child_request_digest` binds the canonical bytes of exactly one existing independent business
command. `child_identity` is the lower-case value `child-<64 hex digits>` where the digits are
`digest_parts_v1("aep.planning-command-child/1", [invocation identity, request digest, unsigned
big-endian step index, child request digest])`. A changed order, inserted relation, changed hop or
different child bytes therefore changes both reservation and child identity.

`BatchKeyV1` is `single_record(String) | named(String)`. `RecordReceiptV1` has `record_id`,
`subject`, `kind: decision | observation`, `revision`, `position: {subject:u64,store:u64}`,
`batch_key`, and `member_index`. `CommitReceiptV1` is the adjacent enum
`single(RecordReceiptV1) | batch { key: BatchKeyV1, members: Vec<RecordReceiptV1> }`; batch members
are nonempty, ordered by `member_index`, reproduce their key, and have unique record IDs.

`PlanningMutationResultV1` is the closed adjacent enum:

| tag | value fields |
| --- | --- |
| `created` | `id`, `kind`, `status`, `revision`, `path` |
| `moved` | `id`, `from`, `to`, `revision` |
| `related` | `id`, `relation`, `target`, `revision` |
| `unrelated` | `id`, `relation`, `target`, `revision` |
| `body_updated` | `id`, `revision`, `body_digest` |
| `fields_set` | `id`, `revision`, ordered `fields: Vec<String>` |
| `scope_updated` | `id`, `revision`, `scope_digest` |
| `evidence_recorded` | `id`, `evidence_id`, `revision` |

Served transitions and driver-issued commands use the same variant as their underlying command;
one recorded atomic command has one result variant even when its receipt is a batch.

`DurableCommandReceiptV1` (`aep.planning-command-receipt/1`) has `command_identity`, `authority`,
`commit_receipt: CommitReceiptV1`, `result_digest`, `committed_authority_snapshot`, and
`receipt_digest`. `CommittedMutationStepV1` has `step_index`, `child_identity`,
`child_request_digest`, the original `DurableCommandReceiptV1`, the original
`PlanningMutationResultV1`, and that result's authority snapshot. Its fields are immutable once the
child commits.

`DurableInvocationReceiptV1` (`aep.planning-invocation-receipt/1`) has invocation identity,
authority, request digest, reservation receipt, the complete ordered planned child tuples, the
ordered committed prefix, and `invocation_receipt_digest`. A committed prefix must start at zero,
be contiguous, reproduce the reservation tuples, and carry each child's original receipt/result.
It is updated only by an authority command whose expected revision is the previous prefix length;
recovery instead reconstructs and verifies the same prefix from child idempotency receipts when an
update outcome was uncertain.

`MutationStopV1` is the adjacent enum `refused { step_index, child_identity,
child_request_digest, refusals } | uncertain { step_index, child_identity, child_request_digest,
refusals }`. `PlanningMutationOutcomeV1` has format `aep.planning-mutation/1` and the closed outcomes
`complete { invocation_receipt, committed, authority_snapshot, projection }`,
`partial { invocation_receipt, committed, stopped, current_authority_snapshot, projection }`,
`committed_projection_failure { invocation_receipt, committed, failed_step_index, projection,
current_authority_snapshot }`, `refused { command_identity, request_digest, step_index,
child_identity, child_request_digest, refusals }`, and `uncertain { command_identity,
request_digest, authority, step_index, child_identity, child_request_digest, committed,
refusals }`. `committed` is nonempty for `partial` and `committed_projection_failure`; `refused` is
only a step-zero pre-commit result. `ProjectionCompletionV1` is
`published { authority_snapshot, inventory_digest, watermark_digest }`. `ProjectionFailureV1` has `code` fixed to
`committed_projection_failure`, `at: DiagnosticCoordinateV1::projection`, and a closed reason
`io | ownership_conflict | foreign_path | inventory_mismatch | watermark_failure`.

All v2 retained mutations render `PlanningMutationOutcomeV1`; commands without a format flag use
text. One-command invocations use the same vector representation with one child. `artifact new
--relate` plans creation followed by one relation child per argument in caller order. `artifact move
--via` plans the ladder hops in their existing deterministic order. Source inspection finds no
other current multi-command ordinary mutation: relate, unrelate, body, set, scope, evidence, served
transition and driver-issued action each issue one business command and therefore plan one child.
Adding a second command to any route requires the same invocation representation rather than a new
ad-hoc retry rule.

Before the next child, retry validates the reservation and every committed prefix member against
provider receipts. It starts at the first missing child. A definite refusal or projection failure
after earlier commits returns `partial` or `committed_projection_failure` with every earlier original
receipt/result and the exact first stop; it never rolls them back and never executes them again.
Projection runs after each committed child, preserving the current observable per-command behavior.
A projection failure stops before later children. Retry first repairs that projection from the
selected authority; after repair it resumes at the next missing child. A failure of the first child
returns `refused`; an uncertain child returns `uncertain` with the verified prefix and is resolved
through that child's idempotency key before any later child. `complete` exits 0. `partial`,
`refused`, `uncertain`, and `committed_projection_failure` exit 1. V1 operations retain their
existing output. Direct edits or unresolved drift refuse before reserving a new invocation.

The server returns HTTP 409 for pre-commit conflict or a definite later-step refusal, HTTP 503 for uncertain provider outcome, and
HTTP 500 with the closed committed payload for committed projection failure; its body is the same
compact JSON result plus LF. The browser retains the command identity until it receives either the
invocation reservation plus all receipts needed to resume, or a definite step-zero refusal. Driver and bridge shutdown outcomes remain
separate operational results and cannot relabel a committed command.

## Concrete phase and result examples

The exact `Prepared` phase shape below omits real digests only by using syntactically valid repeated
fixture digits; every field is still required:

```json
{"format":"aep.planning-migration-phase/1","migration_id":"cutover-01","intent_digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111","phase":"prepared","predecessor":{"kind":"missing"},"observations":[{"kind":"selector","value":{"digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222"}},{"kind":"writer_control","value":{"source_digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333","selector_digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222"}}],"phase_digest":"sha256:4444444444444444444444444444444444444444444444444444444444444444"}
```

An exact typed inspect refusal is:

```json
{"format":"aep.planning-inspection/1","outcome":{"kind":"refused","value":{"refusals":[{"code":"unknown_project_version","at":{"kind":"config","value":{"field":"version"}}}]}}}
```

It exits 1. The corresponding text bytes are the shared flattening grammar, beginning
`format\t"aep.planning-inspection/1"\noutcome.kind\t"refused"\n`; no alias name appears.

An exact committed projection failure for a one-child invocation is:

```json
{"format":"aep.planning-mutation/1","outcome":{"kind":"committed_projection_failure","value":{"invocation_receipt":{"format":"aep.planning-invocation-receipt/1","command_identity":"move-story-42","authority":{"logical_scope":"aep.planning","tenant":"planning-main","stream_identity":"stream-01"},"request_digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111","reservation_receipt_digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222","planned":[{"step_index":0,"child_identity":"child-3333333333333333333333333333333333333333333333333333333333333333","child_request_digest":"sha256:4444444444444444444444444444444444444444444444444444444444444444"}],"committed_count":1,"invocation_receipt_digest":"sha256:5555555555555555555555555555555555555555555555555555555555555555"},"committed":[{"step_index":0,"child_identity":"child-3333333333333333333333333333333333333333333333333333333333333333","child_request_digest":"sha256:4444444444444444444444444444444444444444444444444444444444444444","receipt":{"format":"aep.planning-command-receipt/1","command_identity":"child-3333333333333333333333333333333333333333333333333333333333333333","authority":{"logical_scope":"aep.planning","tenant":"planning-main","stream_identity":"stream-01"},"commit_receipt":{"kind":"batch","value":{"key":{"kind":"named","value":"child-3333333333333333333333333333333333333333333333333333333333333333"},"members":[{"record_id":"record-42","subject":{"entity":"aep.entity","id":"story:demo"},"kind":"decision","revision":4,"position":{"subject":4,"store":19},"batch_key":{"kind":"named","value":"child-3333333333333333333333333333333333333333333333333333333333333333"},"member_index":0}]}},"result_digest":"sha256:6666666666666666666666666666666666666666666666666666666666666666","committed_authority_snapshot":"sha256:7777777777777777777777777777777777777777777777777777777777777777","receipt_digest":"sha256:8888888888888888888888888888888888888888888888888888888888888888"},"result":{"kind":"moved","value":{"id":"story:demo","from":"active","to":"done","revision":4}},"authority_snapshot":"sha256:7777777777777777777777777777777777777777777777777777777777777777"}],"failed_step_index":0,"projection":{"code":"committed_projection_failure","at":{"kind":"projection","value":{"path":{"kind":"unix","value":"hex:706c616e6e696e67"}}},"reason":"ownership_conflict"},"current_authority_snapshot":"sha256:7777777777777777777777777777777777777777777777777777777777777777"}}}
```

## Fixed implementation acceptance for this companion

The deliverable is the full new backend/configuration/command path under the one migration story.
The accepted raw-capture implementation is reused. Every command family has strict reader fixtures,
exact alias/output/exit checks, and native tests over all legacy backends and hybrid policies.
The phase test matrix interrupts before and after each durable effect in the table, including
destination rename, selector replacement and partial projection publication. Same-identity retry
must recover; altered intent, source/config/stage substitution and conflicting selectors must not.
Test exact auxiliary/history/global-ID retention and both source-side and destination-side
equivalence. Controls removing the final source recheck, selector comparison, original-receipt reuse
or post-switch authority rule must compile and fail the corresponding tests before restoration.

The complete acceptance map is:

| surface | required positive cases | required refusals / causal controls |
| --- | --- | --- |
| v1/v2 readers | missing-version v1 plus Markdown, SQLite, PostgreSQL and all hybrid policies; v2 omitted/explicit paths with all three authority members | old reader refuses v2 before backend open; new reader rejects unknown version/field, missing authority member, invalid/nested/aliased paths and v1 backend under v2 |
| selector bytes | byte fixture for canonical JSON-as-YAML and reopen through ordinary discovery | one-byte selector/config substitution, provider authority mismatch and directory-sync uncertainty |
| explicit legacy path | unrelated standalone v1 Markdown remains writable | empty/nonempty v2 projection, retired source, conflicting/malformed ownership marker and invocation with cwd outside the project |
| inspect | ready and each observed unready state at exit 0 in text/JSON | resolution refusal at exit 1; before/after filesystem inventory proves no durable mutation |
| dry-run | all four backend kinds and every hybrid policy with exact authority/mapping/destination requirements | incomplete capture, missing definitions, divergent hybrid, identity collision and path conflict; no destination/intent/lock files |
| apply | all eight phases and matching retry returning byte-identical receipt | interrupt before/after every effect; changed snapshot/config/selector/authority/intent/stage, foreign destination, uncertain provision/import/publish and missing writer control |
| legacy mapping | artifacts, ordered relations, removed relations, accepted/refused audit, applied commands, complete envelope, bare decision/event, raw representation, local-only hybrid evidence | ambiguous relation match, duplicate original ID, unsupported kind/schema, fabricated genesis/order/receipt; mutation dropping each auxiliary family fails equivalence |
| unavailable order | exact envelope bytes resolve through the coordinate/blob join after the external recovery copy is removed; the provider-complete roster still blocks original-ID reuse | replacing unavailable with per-kind/subject order, omitting/tampering a coordinate/blob/roster member or join, reordering the roster, or minting a committed receipt fails and changes the authority snapshot |
| verify | two equal provider-complete snapshots, all subject assurances, exact boundary ledger and current projection | unequal second snapshot, editable `CompleteSnapshot` marker, missing/extra/corrupt boundary or projection item, imported suffix claimed as genesis |
| rebuild | exact requested snapshot under held fence; deleted owned projection rebuilt with foreign paths preserved | stale first capture, movement during staging, foreign owned-path conflict, incomplete publication and watermark substitution; no command execution |
| ordinary mutations | CLI new/move/relate/unrelate/body/set/scope/evidence, served transition and driven command each return ordered invocation/child receipts/results/projection; `new --relate` and `move --via` retain every independently committed child | pre-commit drift; same identity/different request; changed child order; later refusal/uncertainty and projection fault after a committed prefix; same-identity retry repairs and resumes at the first missing child without appending any completed child |
| wire families | strict readers and generated schemas for five store results, mutation result, intent, marker, phase, receipt and watermark; all concrete examples round-trip | missing/unknown field, tag, format, null, empty refusal list, invalid scalar and noncanonical order |
| render/exit/aliases | exact compact JSON-plus-LF and text-flattening fixtures for every outcome; `aep` and `protocol` bytes/exits equal | syntax exit 2, typed refusal exit 1, output failure retains durable phase/receipt |
| operational boundary | controlled disposable writer-control interface can drive pure phase cases | no mock/interface test qualifies a real source; all six real applies remain `writer_exclusion_unavailable` until B-WRITER and provider qualification are proved |

Each causal control changes one fact only. The test must fail for the named reason rather than any
parse or setup failure. Exact PostgreSQL acquisition remains in its already-required disposable real
server lane during later implementation; this design correction runs no server.

This closes the missing phase/command boundary in the proposed implementation contract. It does
not assert implemented commands, accepted candidate dependencies, real writer exclusion or a real
store cutover. Source implementation can proceed independently of those operational blockers;
integration, actual apply qualification and the six cutovers retain their original gates.
